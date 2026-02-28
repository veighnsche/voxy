use crate::{transition, AppEvent, AppState, BufferState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreCommand {
    StartAudioInput,
    StopAudioInput,
    StartTranscriber,
    StopTranscriber,
    StopTranscriberThenEmit(AppEvent),
    EmitEvent(AppEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreModel {
    pub app_state: AppState,
    pub buffer: BufferState,
}

impl Default for CoreModel {
    fn default() -> Self {
        Self {
            app_state: AppState::Idle,
            buffer: BufferState::default(),
        }
    }
}

impl CoreModel {
    pub fn reduce(&mut self, event: AppEvent) -> Vec<CoreCommand> {
        match event {
            AppEvent::MicToggled => self.reduce_mic_toggle(),
            AppEvent::ResetRequested => {
                self.app_state = transition(&self.app_state, &AppEvent::ResetRequested);
                self.buffer.reset_all();
                vec![CoreCommand::StopAudioInput, CoreCommand::StopTranscriber]
            }
            AppEvent::LiveText(text) => {
                self.buffer.append_live(&text);
                self.app_state = transition(&self.app_state, &AppEvent::LiveText(text));
                Vec::new()
            }
            AppEvent::CommitRequested => {
                self.buffer.commit_live();
                self.app_state = transition(&self.app_state, &AppEvent::CommitRequested);
                Vec::new()
            }
        }
    }

    pub fn apply_user_edit(&mut self, text: String) {
        self.buffer.replace_confirmed(text);
        self.buffer.clear_live();
    }

    fn reduce_mic_toggle(&mut self) -> Vec<CoreCommand> {
        let previous = self.app_state.clone();
        self.app_state = transition(&self.app_state, &AppEvent::MicToggled);

        match (previous, self.app_state.clone()) {
            (AppState::Idle, AppState::Recording) => {
                vec![CoreCommand::StartAudioInput, CoreCommand::StartTranscriber]
            }
            (AppState::Recording, AppState::Processing) => vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriberThenEmit(AppEvent::CommitRequested),
            ],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreCommand, CoreModel};
    use crate::{AppEvent, AppState};

    #[test]
    fn mic_toggle_from_idle_starts_recording_pipeline() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::MicToggled);

        assert_eq!(model.app_state, AppState::Recording);
        assert_eq!(
            commands,
            vec![CoreCommand::StartAudioInput, CoreCommand::StartTranscriber]
        );
    }

    #[test]
    fn mic_toggle_from_recording_stops_pipeline_and_requests_commit() {
        let mut model = CoreModel {
            app_state: AppState::Recording,
            ..CoreModel::default()
        };

        let commands = model.reduce(AppEvent::MicToggled);

        assert_eq!(model.app_state, AppState::Processing);
        assert_eq!(
            commands,
            vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriberThenEmit(AppEvent::CommitRequested),
            ]
        );
    }

    #[test]
    fn user_edit_replaces_confirmed_and_clears_live() {
        let mut model = CoreModel::default();
        model.buffer.append_live("tail");

        model.apply_user_edit("hello".to_owned());

        assert_eq!(model.buffer.confirmed_text, "hello");
        assert_eq!(model.buffer.live_segment, "");
    }
}
