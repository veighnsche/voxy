use voxy_audio::{AudioInput, AudioRoute};

use crate::diagnostics::pipeline_trace;

use super::CommandBus;

impl CommandBus {
    pub(super) fn handle_start_audio_input(&self) {
        if let Err(error) = self.audio_input.start_checked() {
            self.emit_runtime_error(error.to_string());
            pipeline_trace::log("command", format!("StartAudioInput error={error}"));
        } else {
            self.emit_log_message("Audio input started");
            pipeline_trace::log("command", "StartAudioInput ok");
        }
    }

    pub(super) fn handle_stop_audio_input(&self) {
        if let Err(error) = self.audio_input.stop_checked() {
            self.emit_runtime_error(error.to_string());
            pipeline_trace::log("command", format!("StopAudioInput error={error}"));
        } else {
            self.emit_log_message("Audio input stopped");
            pipeline_trace::log("command", "StopAudioInput ok");
        }
    }

    pub(super) fn handle_route_microphone_audio(&self) {
        if let Err(error) = self.audio_input.set_route(AudioRoute::Microphone) {
            self.emit_runtime_error(error.to_string());
            pipeline_trace::log("command", format!("RouteMicrophoneAudio error={error}"));
        } else {
            self.emit_log_message("Audio route set to microphone");
            pipeline_trace::log("command", "RouteMicrophoneAudio ok");
        }
    }

    pub(super) fn handle_inject_fixture_audio(&self, fixture_id: u8) {
        if let Err(error) = self.audio_input.inject_fixture_checked(fixture_id) {
            self.emit_runtime_error(error.to_string());
            pipeline_trace::log("command", format!("InjectFixtureAudio error={error}"));
        } else {
            self.emit_log_message(format!(
                "Fixture test_{fixture_id}.mp3 injected into microphone stream"
            ));
            pipeline_trace::log("command", format!("InjectFixtureAudio ok id={fixture_id}"));
        }
    }
}
