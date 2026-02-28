use std::sync::{Arc, Mutex};

use gtk4::{prelude::*, Application, ApplicationWindow};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_audio::{AudioInput, AudioRoute, InputEngine};
use voxy_core::{AppEvent, CoreCommand};
use voxy_stt::{StreamingTranscriber, TranscriberSessionConfig, TranscriptionModel};

use crate::{
    app::behavior, diagnostics::pipeline_trace, tray, wiring::transcriber::AppTranscriber,
};

#[derive(Clone)]
pub struct CommandBus {
    event_tx: mpsc::Sender<AppEvent>,
    transcriber: Arc<AppTranscriber>,
    audio_input: Arc<InputEngine>,
    runtime: Arc<Runtime>,
    window: ApplicationWindow,
    app: Application,
    selected_model: Arc<Mutex<TranscriptionModel>>,
}

impl CommandBus {
    pub fn new(
        event_tx: mpsc::Sender<AppEvent>,
        transcriber: Arc<AppTranscriber>,
        audio_input: Arc<InputEngine>,
        runtime: Arc<Runtime>,
        window: ApplicationWindow,
        app: Application,
        selected_model: Arc<Mutex<TranscriptionModel>>,
    ) -> Self {
        Self {
            event_tx,
            transcriber,
            audio_input,
            runtime,
            window,
            app,
            selected_model,
        }
    }

    pub fn set_transcription_model(&self, model: TranscriptionModel) {
        let mut selected_model = self
            .selected_model
            .lock()
            .expect("selected transcription model mutex poisoned");
        *selected_model = model;
    }

    pub fn execute(&self, commands: Vec<CoreCommand>) {
        for command in commands {
            self.execute_one(command);
        }
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
                let model = *self
                    .selected_model
                    .lock()
                    .expect("selected transcription model mutex poisoned");
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
                    let config = TranscriberSessionConfig::from_model(model);
                    pipeline_trace::log(
                        "command",
                        format!("StartTranscriber async start model={}", model.as_api_id()),
                    );
                    if let Err(error) = transcriber.start(config).await {
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
            CoreCommand::CopyTextToClipboard(text) => {
                behavior::system::clipboard::copy_text_to_clipboard(&self.window, &text);
                self.emit_log_message("Transcript copied to clipboard");
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

    fn emit_log_message(&self, message: impl Into<String>) {
        let _ = self.event_tx.try_send(AppEvent::LogMessage(message.into()));
    }
}
