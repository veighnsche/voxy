use std::sync::Arc;

use gtk4::{Application, ApplicationWindow};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_audio::InputEngine;
use voxy_core::{AppEvent, CoreCommand};

use crate::{
    app::behavior::{self, surface::layer_shell::LayerShellBackend},
    diagnostics::pipeline_trace,
    wiring::transcriber::AppTranscriber,
};

mod app;
mod audio;
mod stt;
mod window;

#[derive(Clone)]
pub struct CommandBus {
    event_tx: mpsc::Sender<AppEvent>,
    transcriber: Arc<AppTranscriber>,
    audio_input: Arc<InputEngine>,
    runtime: Arc<Runtime>,
    window: ApplicationWindow,
    app: Application,
    layer_shell_backend: LayerShellBackend,
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
    ) -> Self {
        Self {
            event_tx,
            transcriber,
            audio_input,
            runtime,
            window,
            app,
            layer_shell_backend,
        }
    }

    pub fn execute(&self, commands: Vec<CoreCommand>) {
        for command in commands {
            self.execute_one(command);
        }
    }

    fn execute_one(&self, command: CoreCommand) {
        pipeline_trace::log("command", format!("execute {command:?}"));
        match command {
            CoreCommand::StartAudioInput => self.handle_start_audio_input(),
            CoreCommand::StopAudioInput => self.handle_stop_audio_input(),
            CoreCommand::RouteMicrophoneAudio => self.handle_route_microphone_audio(),
            CoreCommand::InjectFixtureAudio(fixture_id) => {
                self.handle_inject_fixture_audio(fixture_id)
            }

            CoreCommand::StartTranscriber {
                model,
                vad_silence_duration_ms,
            } => self.handle_start_transcriber(model, vad_silence_duration_ms),
            CoreCommand::StopTranscriber => self.handle_stop_transcriber(),
            CoreCommand::StopTranscriberThenEmit(event) => {
                self.handle_stop_transcriber_then_emit(event)
            }
            CoreCommand::EmitEvent(event) => self.handle_emit_event(event),

            CoreCommand::ShowWindow => self.handle_show_window(),
            CoreCommand::HideWindow => self.handle_hide_window(),
            CoreCommand::ResizeWindow { width, height } => self.handle_resize_window(width, height),
            CoreCommand::MoveWindowToNextScreen => self.handle_move_window_to_next_screen(),
            CoreCommand::CopyTextToClipboard(text) => self.handle_copy_text_to_clipboard(text),

            CoreCommand::QuitApplication => self.handle_quit_application(),
        }
    }

    pub(super) fn emit_runtime_error(&self, message: String) {
        let _ = self.event_tx.try_send(AppEvent::RuntimeError(message));
    }

    pub(super) fn emit_log_message(&self, message: impl Into<String>) {
        let _ = self.event_tx.try_send(AppEvent::LogMessage(message.into()));
    }

    pub(super) fn rollback_recording_start(&self) {
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
}
