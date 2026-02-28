use std::sync::{Arc, Mutex};

use gtk4::{prelude::*, Application, ApplicationWindow};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_audio::{AudioInput, AudioRoute, InputEngine};
use voxy_core::{AppEvent, CoreCommand};
use voxy_stt::{DummyStreamingTranscriber, StreamingTranscriber, TranscriptionModel};

use crate::{app::behavior, tray};

#[derive(Clone)]
pub struct CommandBus {
    event_tx: mpsc::Sender<AppEvent>,
    transcriber: Arc<DummyStreamingTranscriber>,
    audio_input: Arc<InputEngine>,
    runtime: Arc<Runtime>,
    window: ApplicationWindow,
    app: Application,
    selected_model: Arc<Mutex<TranscriptionModel>>,
}

impl CommandBus {
    pub fn new(
        event_tx: mpsc::Sender<AppEvent>,
        transcriber: Arc<DummyStreamingTranscriber>,
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
        match command {
            CoreCommand::StartAudioInput => {
                if let Err(error) = self.audio_input.start_checked() {
                    self.emit_runtime_error(error.to_string());
                } else {
                    self.emit_log_message("Audio input started");
                }
            }
            CoreCommand::StopAudioInput => {
                if let Err(error) = self.audio_input.stop_checked() {
                    self.emit_runtime_error(error.to_string());
                } else {
                    self.emit_log_message("Audio input stopped");
                }
            }
            CoreCommand::RouteMicrophoneAudio => {
                if let Err(error) = self.audio_input.set_route(AudioRoute::Microphone) {
                    self.emit_runtime_error(error.to_string());
                } else {
                    self.emit_log_message("Audio route set to microphone");
                }
            }
            CoreCommand::StartTranscriber => {
                let transcriber = Arc::clone(&self.transcriber);
                let model = *self
                    .selected_model
                    .lock()
                    .expect("selected transcription model mutex poisoned");
                self.runtime.spawn(async move {
                    transcriber.start(model).await;
                });
                self.emit_log_message(format!("Transcriber started ({})", model.as_api_id()));
            }
            CoreCommand::StopTranscriber => {
                let transcriber = Arc::clone(&self.transcriber);
                self.runtime.spawn(async move {
                    transcriber.stop().await;
                });
                self.emit_log_message("Transcriber stop requested");
            }
            CoreCommand::StopTranscriberThenEmit(event) => {
                let transcriber = Arc::clone(&self.transcriber);
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
                    transcriber.stop().await;
                    let _ = event_tx.send(event).await;
                });
                self.emit_log_message("Transcriber stopping; commit scheduled");
            }
            CoreCommand::EmitEvent(event) => {
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
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
            CoreCommand::RouteFixtureAudio(fixture_id) => {
                if let Err(error) = self
                    .audio_input
                    .set_route(AudioRoute::fixture_test(fixture_id))
                {
                    self.emit_runtime_error(error.to_string());
                } else {
                    self.emit_log_message(format!("Audio route set to fixture test_{fixture_id}"));
                    let event_tx = self.event_tx.clone();
                    self.runtime.spawn(async move {
                        let playback_task = tokio::task::spawn_blocking(move || {
                            behavior::system::audio_preview::play_fixture_audio(fixture_id)
                        });

                        match playback_task.await {
                            Ok(Ok(())) => {
                                let _ = event_tx
                                    .send(AppEvent::LogMessage(format!(
                                        "Fixture playback started: test_{fixture_id}"
                                    )))
                                    .await;
                            }
                            Ok(Err(message)) => {
                                let _ = event_tx
                                    .send(AppEvent::RuntimeError(format!(
                                        "fixture playback unavailable: {message}"
                                    )))
                                    .await;
                            }
                            Err(error) => {
                                let _ = event_tx
                                    .send(AppEvent::RuntimeError(format!(
                                        "fixture playback task failed: {error}"
                                    )))
                                    .await;
                            }
                        }
                    });
                }
            }
            CoreCommand::QuitApplication => {
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
