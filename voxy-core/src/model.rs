use crate::{transition, AppEvent, AppState, BufferState, UiPrefs};

const RECORD_SEED_TEXT: &str = "lorum ipso";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreCommand {
    StartAudioInput,
    StopAudioInput,
    StartTranscriber,
    StopTranscriber,
    StopTranscriberThenEmit(AppEvent),
    EmitEvent(AppEvent),
    ShowWindow,
    HideWindow,
    CopyTextToClipboard(String),
    QuitApplication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreModel {
    pub app_state: AppState,
    pub buffer: BufferState,
    pub ui_prefs: UiPrefs,
    pub runtime_error: Option<String>,
}

impl Default for CoreModel {
    fn default() -> Self {
        Self {
            app_state: AppState::Idle,
            buffer: BufferState::default(),
            ui_prefs: UiPrefs::default(),
            runtime_error: None,
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
                self.runtime_error = None;
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
            AppEvent::VisibilityToggled => self.reduce_visibility_toggle(),
            AppEvent::ShowRequested => self.reduce_show_requested(),
            AppEvent::HideRequested => self.reduce_hide_requested(),
            AppEvent::CopyRequested => {
                vec![CoreCommand::CopyTextToClipboard(self.buffer.full_text())]
            }
            AppEvent::QuitRequested => vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriber,
                CoreCommand::QuitApplication,
            ],
            AppEvent::RuntimeError(message) => {
                self.runtime_error = Some(message);
                Vec::new()
            }
            AppEvent::ErrorCleared => {
                self.runtime_error = None;
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
            (AppState::Idle, AppState::Recording) => vec![
                CoreCommand::StartAudioInput,
                CoreCommand::StartTranscriber,
                CoreCommand::EmitEvent(AppEvent::LiveText(RECORD_SEED_TEXT.to_owned())),
            ],
            (AppState::Recording, AppState::Processing) => vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriberThenEmit(AppEvent::CommitRequested),
            ],
            _ => Vec::new(),
        }
    }

    fn reduce_visibility_toggle(&mut self) -> Vec<CoreCommand> {
        self.ui_prefs.visible = !self.ui_prefs.visible;

        if self.ui_prefs.visible {
            vec![CoreCommand::ShowWindow]
        } else {
            vec![CoreCommand::HideWindow]
        }
    }

    fn reduce_show_requested(&mut self) -> Vec<CoreCommand> {
        self.ui_prefs.visible = true;
        vec![CoreCommand::ShowWindow]
    }

    fn reduce_hide_requested(&mut self) -> Vec<CoreCommand> {
        if !self.ui_prefs.visible {
            return Vec::new();
        }

        self.ui_prefs.visible = false;
        vec![CoreCommand::HideWindow]
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreCommand, CoreModel, RECORD_SEED_TEXT};
    use crate::{AppEvent, AppState};

    #[test]
    fn mic_toggle_from_idle_starts_recording_pipeline() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::MicToggled);

        assert_eq!(model.app_state, AppState::Recording);
        assert_eq!(
            commands,
            vec![
                CoreCommand::StartAudioInput,
                CoreCommand::StartTranscriber,
                CoreCommand::EmitEvent(AppEvent::LiveText(RECORD_SEED_TEXT.to_owned())),
            ]
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

    #[test]
    fn visibility_toggle_is_core_owned_and_does_not_change_app_state() {
        let mut model = CoreModel {
            app_state: AppState::Recording,
            ..CoreModel::default()
        };

        let commands = model.reduce(AppEvent::VisibilityToggled);

        assert_eq!(commands, vec![CoreCommand::HideWindow]);
        assert_eq!(model.app_state, AppState::Recording);
        assert!(!model.ui_prefs.visible);
    }

    #[test]
    fn hide_requested_is_idempotent() {
        let mut model = CoreModel::default();
        model.ui_prefs.visible = false;

        let commands = model.reduce(AppEvent::HideRequested);

        assert!(commands.is_empty());
        assert!(!model.ui_prefs.visible);
    }

    #[test]
    fn copy_requested_emits_clipboard_command() {
        let mut model = CoreModel::default();
        model.buffer.replace_confirmed("hello".to_owned());
        model.buffer.append_live(" world");

        let commands = model.reduce(AppEvent::CopyRequested);

        assert_eq!(
            commands,
            vec![CoreCommand::CopyTextToClipboard("hello world".to_owned())]
        );
    }

    #[test]
    fn reset_keeps_visibility_preference() {
        let mut model = CoreModel::default();
        model.ui_prefs.visible = false;

        let _ = model.reduce(AppEvent::ResetRequested);

        assert!(!model.ui_prefs.visible);
    }

    #[test]
    fn runtime_error_is_stored_and_cleared() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::RuntimeError("tray unavailable".to_owned()));
        assert!(commands.is_empty());
        assert_eq!(model.runtime_error.as_deref(), Some("tray unavailable"));

        let commands = model.reduce(AppEvent::ErrorCleared);
        assert!(commands.is_empty());
        assert_eq!(model.runtime_error, None);
    }

    #[test]
    fn runtime_error_does_not_change_recording_state() {
        let mut model = CoreModel {
            app_state: AppState::Recording,
            ..CoreModel::default()
        };

        let _ = model.reduce(AppEvent::RuntimeError("test".to_owned()));

        assert_eq!(model.app_state, AppState::Recording);
    }

    #[test]
    fn quit_requested_stops_pipeline_and_quits_application() {
        let mut model = CoreModel {
            app_state: AppState::Recording,
            ..CoreModel::default()
        };

        let commands = model.reduce(AppEvent::QuitRequested);

        assert_eq!(
            commands,
            vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriber,
                CoreCommand::QuitApplication,
            ]
        );
    }
}
