use std::sync::{Arc, Mutex};

use gtk4::{prelude::*, Application, ApplicationWindow};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_audio::{AudioInput, AudioRoute, InputEngine};
use voxy_core::{AppEvent, CoreCommand};
use voxy_stt::{StreamingTranscriber, TranscriberSessionConfig, TranscriptionModel};

use crate::{
    app::behavior::{self, surface::layer_shell::MonitorCycleOutcome},
    diagnostics::pipeline_trace,
    tray,
    wiring::transcriber::AppTranscriber,
};

#[derive(Clone)]
pub struct CommandBus {
    event_tx: mpsc::Sender<AppEvent>,
    transcriber: Arc<AppTranscriber>,
    audio_input: Arc<InputEngine>,
    runtime: Arc<Runtime>,
    window: ApplicationWindow,
    app: Application,
    layer_shell_backend: behavior::surface::layer_shell::LayerShellBackend,
    selected_model: Arc<Mutex<TranscriptionModel>>,
    vad_silence_duration_ms: Arc<Mutex<u32>>,
}

impl CommandBus {
    pub fn new(
        event_tx: mpsc::Sender<AppEvent>,
        transcriber: Arc<AppTranscriber>,
        audio_input: Arc<InputEngine>,
        runtime: Arc<Runtime>,
        window: ApplicationWindow,
        app: Application,
        layer_shell_backend: behavior::surface::layer_shell::LayerShellBackend,
        selected_model: Arc<Mutex<TranscriptionModel>>,
        vad_silence_duration_ms: Arc<Mutex<u32>>,
    ) -> Self {
        Self {
            event_tx,
            transcriber,
            audio_input,
            runtime,
            window,
            app,
            layer_shell_backend,
            selected_model,
            vad_silence_duration_ms,
        }
    }

    pub fn set_transcription_model(&self, model: TranscriptionModel) -> bool {
        if !self.transcriber.supports_model(model) {
            let backend = self.transcriber.backend_name();
            self.emit_runtime_error(format!(
                "Selected model '{}' is not supported by active backend '{}'.",
                model.as_api_id(),
                backend
            ));
            self.emit_log_message(format!(
                "Model selection blocked (backend '{}' does not support model '{}')",
                backend,
                model.as_api_id()
            ));
            pipeline_trace::log(
                "command",
                format!(
                    "SetTranscriptionModel blocked model={} backend={}",
                    model.as_api_id(),
                    backend
                ),
            );
            return false;
        }

        let mut selected_model = self
            .selected_model
            .lock()
            .expect("selected transcription model mutex poisoned");
        *selected_model = model;
        pipeline_trace::log(
            "command",
            format!("SetTranscriptionModel model={}", model.as_api_id()),
        );
        true
    }

    pub fn execute(&self, commands: Vec<CoreCommand>) {
        for command in commands {
            self.execute_one(command);
        }
    }

    pub fn set_vad_silence_duration_ms(&self, vad_silence_duration_ms: u32) {
        let mut current = self
            .vad_silence_duration_ms
            .lock()
            .expect("vad silence duration mutex poisoned");
        *current = vad_silence_duration_ms.clamp(100, 5_000);
        pipeline_trace::log(
            "command",
            format!("SetVadSilenceDurationMs value={}", *current),
        );
    }

    fn execute_one(&self, command: CoreCommand) {
        pipeline_trace::log("command", format!("execute {command:?}"));
        match command {
            CoreCommand::StartAudioInput => {
                if let Err(error) = self.audio_input.start_checked() {
                    self.emit_runtime_error(error.to_string());
                    pipeline_trace::log("command", format!("StartAudioInput error={error}"));
                } else {
                    self.emit_log_message("Audio input started");
                    pipeline_trace::log("command", "StartAudioInput ok");
                }
            }
            CoreCommand::StopAudioInput => {
                if let Err(error) = self.audio_input.stop_checked() {
                    self.emit_runtime_error(error.to_string());
                    pipeline_trace::log("command", format!("StopAudioInput error={error}"));
                } else {
                    self.emit_log_message("Audio input stopped");
                    pipeline_trace::log("command", "StopAudioInput ok");
                }
            }
            CoreCommand::RouteMicrophoneAudio => {
                if let Err(error) = self.audio_input.set_route(AudioRoute::Microphone) {
                    self.emit_runtime_error(error.to_string());
                    pipeline_trace::log("command", format!("RouteMicrophoneAudio error={error}"));
                } else {
                    self.emit_log_message("Audio route set to microphone");
                    pipeline_trace::log("command", "RouteMicrophoneAudio ok");
                }
            }
            CoreCommand::StartTranscriber => {
                let transcriber = Arc::clone(&self.transcriber);
                let audio_input = Arc::clone(&self.audio_input);
                let model = *self
                    .selected_model
                    .lock()
                    .expect("selected transcription model mutex poisoned");
                let vad_silence_duration_ms = *self
                    .vad_silence_duration_ms
                    .lock()
                    .expect("vad silence duration mutex poisoned");
                if !transcriber.supports_model(model) {
                    let backend = transcriber.backend_name();
                    self.emit_runtime_error(format!(
                        "Selected model '{}' is not supported by active backend '{}'.",
                        model.as_api_id(),
                        backend
                    ));
                    self.emit_log_message(format!(
                        "Transcriber start blocked (backend '{}' does not support model '{}')",
                        backend,
                        model.as_api_id()
                    ));
                    pipeline_trace::log(
                        "command",
                        format!(
                            "StartTranscriber blocked model={} backend={}",
                            model.as_api_id(),
                            backend
                        ),
                    );
                    self.rollback_recording_start();
                    return;
                }
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
                    let mut config = TranscriberSessionConfig::from_model(model);
                    config.vad_silence_duration_ms = vad_silence_duration_ms;
                    pipeline_trace::log(
                        "command",
                        format!(
                            "StartTranscriber async start model={} vad_silence_ms={}",
                            model.as_api_id(),
                            config.vad_silence_duration_ms
                        ),
                    );
                    if let Err(error) = transcriber.start(config).await {
                        if let Err(stop_error) = audio_input.stop_checked() {
                            let _ = event_tx
                                .send(AppEvent::RuntimeError(format!(
                                    "failed to stop audio input after transcriber start error: {stop_error}"
                                )))
                                .await;
                            pipeline_trace::log(
                                "command",
                                format!("StartTranscriber rollback stop_audio error={stop_error}"),
                            );
                        } else {
                            pipeline_trace::log(
                                "command",
                                "StartTranscriber rollback stop_audio ok",
                            );
                        }
                        let _ = event_tx.send(AppEvent::RecordingStartRejected).await;
                        let _ = event_tx
                            .send(AppEvent::RuntimeError(format!(
                                "failed to start transcriber: {error}"
                            )))
                            .await;
                        pipeline_trace::log("command", format!("StartTranscriber error={error}"));
                    } else {
                        pipeline_trace::log("command", "StartTranscriber started");
                    }
                });
                self.emit_log_message(format!("Transcriber started ({})", model.as_api_id()));
            }
            CoreCommand::StopTranscriber => {
                let transcriber = Arc::clone(&self.transcriber);
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
                    if let Err(error) = transcriber.stop().await {
                        let _ = event_tx
                            .send(AppEvent::RuntimeError(format!(
                                "failed to stop transcriber: {error}"
                            )))
                            .await;
                        pipeline_trace::log("command", format!("StopTranscriber error={error}"));
                    } else {
                        pipeline_trace::log("command", "StopTranscriber stopped");
                    }
                });
                self.emit_log_message("Transcriber stop requested");
            }
            CoreCommand::StopTranscriberThenEmit(event) => {
                let transcriber = Arc::clone(&self.transcriber);
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
                    if let Err(error) = transcriber.stop().await {
                        let _ = event_tx
                            .send(AppEvent::RuntimeError(format!(
                                "failed to stop transcriber before emit: {error}"
                            )))
                            .await;
                        pipeline_trace::log(
                            "command",
                            format!("StopTranscriberThenEmit stop error={error}"),
                        );
                    }
                    pipeline_trace::log(
                        "command",
                        format!("StopTranscriberThenEmit emit {event:?}"),
                    );
                    let _ = event_tx.send(event).await;
                });
                self.emit_log_message("Transcriber stopping; commit scheduled");
            }
            CoreCommand::EmitEvent(event) => {
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
                    pipeline_trace::log("command", format!("EmitEvent send {event:?}"));
                    let _ = event_tx.send(event).await;
                });
            }
            CoreCommand::ShowWindow => {
                behavior::visibility::window_visibility::show_window(&self.window)
            }
            CoreCommand::HideWindow => {
                behavior::visibility::window_visibility::hide_window(&self.window)
            }
            CoreCommand::ResizeWindow { width, height } => {
                self.window.set_default_size(width, height);
            }
            CoreCommand::MoveWindowToNextScreen => {
                match self
                    .layer_shell_backend
                    .move_window_to_next_monitor(&self.window)
                {
                    Ok(MonitorCycleOutcome::Moved {
                        from_index,
                        to_index,
                        monitor_count,
                    }) => {
                        self.emit_log_message(format!(
                            "Moved to screen {}/{}",
                            to_index + 1,
                            monitor_count
                        ));
                        pipeline_trace::log(
                            "command",
                            format!(
                                "MoveWindowToNextScreen moved from={} to={} total={}",
                                from_index + 1,
                                to_index + 1,
                                monitor_count
                            ),
                        );
                    }
                    Ok(MonitorCycleOutcome::SingleMonitor) => {
                        self.emit_log_message("Only one screen detected; nothing to move");
                        pipeline_trace::log("command", "MoveWindowToNextScreen single_monitor");
                    }
                    Err(error) => {
                        self.emit_runtime_error(error.clone());
                        pipeline_trace::log(
                            "command",
                            format!("MoveWindowToNextScreen error={error}"),
                        );
                    }
                }
            }
            CoreCommand::CopyTextToClipboard(text) => {
                behavior::system::clipboard::copy_text_to_clipboard(&self.window, &text);
                self.emit_log_message("Transcript copied to clipboard");
            }
            CoreCommand::InjectFixtureAudio(fixture_id) => {
                if let Err(error) = self.audio_input.inject_fixture_checked(fixture_id) {
                    self.emit_runtime_error(error.to_string());
                    pipeline_trace::log("command", format!("InjectFixtureAudio error={error}"));
                } else {
                    self.emit_log_message(format!(
                        "Fixture test_{fixture_id}.mp3 injected into microphone stream"
                    ));
                    pipeline_trace::log(
                        "command",
                        format!("InjectFixtureAudio ok id={fixture_id}"),
                    );
                }
            }
            CoreCommand::QuitApplication => {
                pipeline_trace::log("command", "QuitApplication");
                tray::shutdown();
                self.app.quit();
            }
        }
    }

    fn emit_runtime_error(&self, message: String) {
        let _ = self.event_tx.try_send(AppEvent::RuntimeError(message));
    }

    fn rollback_recording_start(&self) {
        if let Err(error) = self.audio_input.stop_checked() {
            self.emit_runtime_error(format!(
                "failed to stop audio input after start rejection: {error}"
            ));
            pipeline_trace::log(
                "command",
                format!("rollback_recording_start.stop_audio error={error}"),
            );
        } else {
            pipeline_trace::log("command", "rollback_recording_start.stop_audio ok");
        }

        let _ = self.event_tx.try_send(AppEvent::RecordingStartRejected);
    }

    fn emit_log_message(&self, message: impl Into<String>) {
        let _ = self.event_tx.try_send(AppEvent::LogMessage(message.into()));
    }
}
