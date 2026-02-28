use crate::{transition, AppEvent, AppState, BufferState, UiPrefs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreCommand {
    StartAudioInput,
    StopAudioInput,
    RouteMicrophoneAudio,
    StartTranscriber,
    StopTranscriber,
    StopTranscriberThenEmit(AppEvent),
    EmitEvent(AppEvent),
    ShowWindow,
    HideWindow,
    CopyTextToClipboard(String),
    InjectFixtureAudio(u8),
    QuitApplication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreModel {
    pub app_state: AppState,
    pub buffer: BufferState,
    pub ui_prefs: UiPrefs,
    pub log_line: String,
    pub runtime_error: Option<String>,
}

impl Default for CoreModel {
    fn default() -> Self {
        Self {
            app_state: AppState::Idle,
            buffer: BufferState::default(),
            ui_prefs: UiPrefs::default(),
            log_line: "Ready".to_owned(),
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
                self.log_line = "Reset completed".to_owned();
                self.runtime_error = None;
                vec![
                    CoreCommand::StopAudioInput,
                    CoreCommand::StopTranscriber,
                    CoreCommand::RouteMicrophoneAudio,
                ]
            }
            AppEvent::LogMessage(message) => {
                self.log_line = message;
                Vec::new()
            }
            AppEvent::LiveText(text) => {
                self.buffer.append_live(&text);
                self.app_state = transition(&self.app_state, &AppEvent::LiveText(text));
                Vec::new()
            }
            AppEvent::CommitRequested => {
                self.buffer.commit_live();
                self.log_line = "Commit completed".to_owned();
                self.app_state = transition(&self.app_state, &AppEvent::CommitRequested);
                Vec::new()
            }
            AppEvent::VisibilityToggled => self.reduce_visibility_toggle(),
            AppEvent::ShowRequested => self.reduce_show_requested(),
            AppEvent::HideRequested => self.reduce_hide_requested(),
            AppEvent::CopyRequested => {
                self.log_line = "Copy requested".to_owned();
                vec![CoreCommand::CopyTextToClipboard(self.buffer.full_text())]
            }
            AppEvent::FixtureInjectRequested(fixture_id) => {
                self.log_line = format!("Fixture injection requested: test_{fixture_id}");
                vec![CoreCommand::InjectFixtureAudio(fixture_id)]
            }
            AppEvent::QuitRequested => vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriber,
                CoreCommand::QuitApplication,
            ],
            AppEvent::RuntimeError(message) => {
                self.log_line = format!("Error: {message}");
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

    pub fn set_window_position(&mut self, left: i32, top: i32) {
        self.ui_prefs.window_left = left.max(0);
        self.ui_prefs.window_top = top.max(0);
    }

    fn reduce_mic_toggle(&mut self) -> Vec<CoreCommand> {
        let previous = self.app_state.clone();
        self.app_state = transition(&self.app_state, &AppEvent::MicToggled);

        match (previous, self.app_state.clone()) {
            (AppState::Idle, AppState::Recording) => {
                self.log_line = "Recording started".to_owned();
                vec![CoreCommand::StartAudioInput, CoreCommand::StartTranscriber]
            }
            (AppState::Recording, AppState::Processing) => {
                self.log_line = "Recording stopped; processing".to_owned();
                vec![
                    CoreCommand::StopAudioInput,
                    CoreCommand::StopTranscriberThenEmit(AppEvent::CommitRequested),
                ]
            }
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
    use super::{CoreCommand, CoreModel};
    use crate::{AppEvent, AppState};

    #[test]
    fn mic_toggle_from_idle_starts_recording_pipeline() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::MicToggled);

        assert_eq!(model.app_state, AppState::Recording);
        assert_eq!(
            commands,
            vec![CoreCommand::StartAudioInput, CoreCommand::StartTranscriber,]
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
    fn fixture_inject_requested_emits_audio_command() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::FixtureInjectRequested(3));

        assert_eq!(commands, vec![CoreCommand::InjectFixtureAudio(3)]);
        assert_eq!(model.log_line, "Fixture injection requested: test_3");
    }

    #[test]
    fn log_message_event_updates_footer_log_line() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::LogMessage("audio started".to_owned()));

        assert!(commands.is_empty());
        assert_eq!(model.log_line, "audio started");
    }

    #[test]
    fn reset_keeps_visibility_preference() {
        let mut model = CoreModel::default();
        model.ui_prefs.visible = false;

        let commands = model.reduce(AppEvent::ResetRequested);

        assert!(!model.ui_prefs.visible);
        assert_eq!(
            commands,
            vec![
                CoreCommand::StopAudioInput,
                CoreCommand::StopTranscriber,
                CoreCommand::RouteMicrophoneAudio,
            ]
        );
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

    #[test]
    fn window_position_updates_are_clamped_and_persisted() {
        let mut model = CoreModel::default();

        model.set_window_position(-10, 42);

        assert_eq!(model.ui_prefs.window_left, 0);
        assert_eq!(model.ui_prefs.window_top, 42);
    }
}
