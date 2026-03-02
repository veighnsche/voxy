use crate::{transition, AppEvent, AppState};

use super::{CoreCommand, CoreModel};

impl CoreModel {
    pub(super) fn reduce_mic_toggle(&mut self) -> Vec<CoreCommand> {
        let was_idle = matches!(self.app_state, AppState::Idle);
        let was_recording = matches!(self.app_state, AppState::Recording);
        self.app_state = transition(&self.app_state, &AppEvent::MicToggled);

        if was_idle && matches!(self.app_state, AppState::Recording) {
            self.recording_stop_policy.reset();
            self.log_line = "Recording started".to_owned();
            return vec![
                CoreCommand::StartAudioInput,
                CoreCommand::StartTranscriber {
                    model: self.ui_prefs.transcription_model,
                    vad_silence_duration_ms: self.ui_prefs.vad_silence_duration_ms,
                },
            ];
        }

        if was_recording && matches!(self.app_state, AppState::Processing) {
            self.recording_stop_policy.reset();
            self.log_line = "Recording stopped; processing".to_owned();
            return vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriberThenEmit(AppEvent::CommitRequested),
            ];
        }

        Vec::new()
    }

    pub(super) fn reduce_live_text(&mut self, text: String) -> Vec<CoreCommand> {
        self.buffer.append_live(&text);
        let event = AppEvent::LiveText(text);
        self.app_state = transition(&self.app_state, &event);
        Vec::new()
    }

    pub(super) fn reduce_commit_requested(&mut self) -> Vec<CoreCommand> {
        self.buffer.commit_live();
        self.log_line = "Commit completed".to_owned();
        self.app_state = transition(&self.app_state, &AppEvent::CommitRequested);
        Vec::new()
    }

    pub(super) fn reduce_recording_start_rejected(&mut self) -> Vec<CoreCommand> {
        if matches!(self.app_state, AppState::Recording) {
            self.app_state = AppState::Idle;
            self.recording_stop_policy.reset();
            self.log_line = "Recording start blocked".to_owned();
        }
        Vec::new()
    }

    pub(super) fn reduce_fixture_inject_requested(&mut self, fixture_id: u8) -> Vec<CoreCommand> {
        self.log_line = format!("Fixture injection requested: test_{fixture_id}");
        vec![CoreCommand::InjectFixtureAudio(fixture_id)]
    }

    pub(super) fn reduce_quit_requested(&mut self) -> Vec<CoreCommand> {
        vec![
            CoreCommand::StopAudioInput,
            CoreCommand::StopTranscriber,
            CoreCommand::QuitApplication,
        ]
    }
}
