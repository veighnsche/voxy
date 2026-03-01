use crate::{transition, AppEvent, AppState, BufferState, UiPrefs};

const WINDOW_RESIZE_STEP: i32 = 40;
const WINDOW_MIN_WIDTH: i32 = 280;
const WINDOW_MIN_HEIGHT: i32 = 320;
const WINDOW_MAX_WIDTH: i32 = 960;
const WINDOW_MAX_HEIGHT: i32 = 1280;

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
    ResizeWindow { width: i32, height: i32 },
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
                self.buffer.reset_all();
                Vec::new()
            }
            AppEvent::WindowLargerRequested => {
                self.resize_window_by(WINDOW_RESIZE_STEP, WINDOW_RESIZE_STEP)
            }
            AppEvent::WindowSmallerRequested => {
                self.resize_window_by(-WINDOW_RESIZE_STEP, -WINDOW_RESIZE_STEP)
            }
            AppEvent::WindowResizeRequested { width, height } => {
                self.resize_window_to(width, height)
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
            AppEvent::SettingsToggled => {
                self.ui_prefs.settings_open = !self.ui_prefs.settings_open;
                self.log_line = if self.ui_prefs.settings_open {
                    "Settings opened".to_owned()
                } else {
                    "Settings closed".to_owned()
                };
                Vec::new()
            }
            AppEvent::SilenceAutoStopSecondsChanged(seconds) => {
                self.ui_prefs.silence_auto_stop_seconds = seconds.min(600);
                self.log_line = if self.ui_prefs.silence_auto_stop_seconds == 0 {
                    "Silence auto-stop disabled".to_owned()
                } else {
                    format!(
                        "Silence auto-stop set to {}s",
                        self.ui_prefs.silence_auto_stop_seconds
                    )
                };
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

    pub fn set_window_size(&mut self, width: i32, height: i32) {
        self.ui_prefs.window_width = width.clamp(WINDOW_MIN_WIDTH, WINDOW_MAX_WIDTH);
        self.ui_prefs.window_height = height.clamp(WINDOW_MIN_HEIGHT, WINDOW_MAX_HEIGHT);
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

    fn resize_window_by(&mut self, width_delta: i32, height_delta: i32) -> Vec<CoreCommand> {
        let next_width = self.ui_prefs.window_width.saturating_add(width_delta);
        let next_height = self.ui_prefs.window_height.saturating_add(height_delta);
        self.set_window_size(next_width, next_height);
        self.log_line = format!(
            "Window resized to {}x{}",
            self.ui_prefs.window_width, self.ui_prefs.window_height
        );
        vec![CoreCommand::ResizeWindow {
            width: self.ui_prefs.window_width,
            height: self.ui_prefs.window_height,
        }]
    }

    fn resize_window_to(&mut self, width: i32, height: i32) -> Vec<CoreCommand> {
        self.set_window_size(width, height);
        vec![CoreCommand::ResizeWindow {
            width: self.ui_prefs.window_width,
            height: self.ui_prefs.window_height,
        }]
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
    fn settings_toggle_flips_settings_panel_preference() {
        let mut model = CoreModel::default();
        assert!(!model.ui_prefs.settings_open);

        let commands = model.reduce(AppEvent::SettingsToggled);
        assert!(commands.is_empty());
        assert!(model.ui_prefs.settings_open);

        let commands = model.reduce(AppEvent::SettingsToggled);
        assert!(commands.is_empty());
        assert!(!model.ui_prefs.settings_open);
    }

    #[test]
    fn silence_auto_stop_timeout_update_is_core_owned() {
        let mut model = CoreModel::default();
        assert_eq!(model.ui_prefs.silence_auto_stop_seconds, 10);

        let commands = model.reduce(AppEvent::SilenceAutoStopSecondsChanged(7));
        assert!(commands.is_empty());
        assert_eq!(model.ui_prefs.silence_auto_stop_seconds, 7);

        let commands = model.reduce(AppEvent::SilenceAutoStopSecondsChanged(0));
        assert!(commands.is_empty());
        assert_eq!(model.ui_prefs.silence_auto_stop_seconds, 0);
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
        assert!(commands.is_empty());
    }

    #[test]
    fn reset_does_not_change_recording_state() {
        let mut model = CoreModel {
            app_state: AppState::Recording,
            ..CoreModel::default()
        };
        model.buffer.replace_confirmed("hello".to_owned());
        model.buffer.append_live(" world");

        let commands = model.reduce(AppEvent::ResetRequested);

        assert!(commands.is_empty());
        assert_eq!(model.app_state, AppState::Recording);
        assert!(model.buffer.confirmed_text.is_empty());
        assert!(model.buffer.live_segment.is_empty());
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

    #[test]
    fn window_larger_requested_updates_prefs_and_emits_resize_command() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::WindowLargerRequested);

        assert_eq!(model.ui_prefs.window_width, 400);
        assert_eq!(model.ui_prefs.window_height, 460);
        assert_eq!(
            commands,
            vec![CoreCommand::ResizeWindow {
                width: 400,
                height: 460
            }]
        );
    }

    #[test]
    fn window_smaller_requested_clamps_to_minimum_size() {
        let mut model = CoreModel::default();
        model.set_window_size(280, 320);

        let commands = model.reduce(AppEvent::WindowSmallerRequested);

        assert_eq!(model.ui_prefs.window_width, 280);
        assert_eq!(model.ui_prefs.window_height, 320);
        assert_eq!(
            commands,
            vec![CoreCommand::ResizeWindow {
                width: 280,
                height: 320
            }]
        );
    }

    #[test]
    fn window_resize_requested_is_clamped_and_emits_resize_command() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::WindowResizeRequested {
            width: 100,
            height: 2000,
        });

        assert_eq!(model.ui_prefs.window_width, 280);
        assert_eq!(model.ui_prefs.window_height, 1280);
        assert_eq!(
            commands,
            vec![CoreCommand::ResizeWindow {
                width: 280,
                height: 1280
            }]
        );
    }

    #[test]
    fn window_size_updates_are_clamped_and_persisted() {
        let mut model = CoreModel::default();

        model.set_window_size(5, 9000);

        assert_eq!(model.ui_prefs.window_width, 280);
        assert_eq!(model.ui_prefs.window_height, 1280);
    }
}
