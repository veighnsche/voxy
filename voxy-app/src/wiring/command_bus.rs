use std::sync::Arc;

use gtk4::{prelude::*, Application, ApplicationWindow};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_audio::{AudioInput, NoopAudioInput};
use voxy_core::{AppEvent, CoreCommand};
use voxy_stt::{DummyStreamingTranscriber, StreamingTranscriber};

use crate::{app::behavior, tray};

#[derive(Clone)]
pub struct CommandBus {
    event_tx: mpsc::Sender<AppEvent>,
    transcriber: Arc<DummyStreamingTranscriber>,
    audio_input: Arc<NoopAudioInput>,
    runtime: Arc<Runtime>,
    window: ApplicationWindow,
    app: Application,
}

impl CommandBus {
    pub fn new(
        event_tx: mpsc::Sender<AppEvent>,
        transcriber: Arc<DummyStreamingTranscriber>,
        audio_input: Arc<NoopAudioInput>,
        runtime: Arc<Runtime>,
        window: ApplicationWindow,
        app: Application,
    ) -> Self {
        Self {
            event_tx,
            transcriber,
            audio_input,
            runtime,
            window,
            app,
        }
    }

    pub fn execute(&self, commands: Vec<CoreCommand>) {
        for command in commands {
            self.execute_one(command);
        }
    }

    fn execute_one(&self, command: CoreCommand) {
        match command {
            CoreCommand::StartAudioInput => self.audio_input.start(),
            CoreCommand::StopAudioInput => self.audio_input.stop(),
            CoreCommand::StartTranscriber => {
                let transcriber = Arc::clone(&self.transcriber);
                self.runtime.spawn(async move {
                    transcriber.start().await;
                });
            }
            CoreCommand::StopTranscriber => {
                let transcriber = Arc::clone(&self.transcriber);
                self.runtime.spawn(async move {
                    transcriber.stop().await;
                });
            }
            CoreCommand::StopTranscriberThenEmit(event) => {
                let transcriber = Arc::clone(&self.transcriber);
                let event_tx = self.event_tx.clone();
                self.runtime.spawn(async move {
                    transcriber.stop().await;
                    let _ = event_tx.send(event).await;
                });
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
                behavior::system::clipboard::copy_text_to_clipboard(&self.window, &text)
            }
            CoreCommand::QuitApplication => {
                tray::shutdown();
                self.app.quit();
            }
        }
    }
}
