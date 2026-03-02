use std::time::{Duration, Instant};

use crate::{
    recording_stop::RecordingStopPolicyState, AppEvent, AppState, BufferState,
    RecordingStopDecision, TranscriptionModelId, UiPrefs,
};

mod clipboard;
mod error;
mod recording;
mod settings;
mod window;

#[derive(Debug, Clone, PartialEq)]
pub enum CoreCommand {
    StartAudioInput,
    StopAudioInput,
    RouteMicrophoneAudio,
    StartTranscriber {
        model: TranscriptionModelId,
        vad_silence_duration_ms: u32,
    },
    StopTranscriber,
    StopTranscriberThenEmit(AppEvent),
    EmitEvent(AppEvent),
    ShowWindow,
    HideWindow,
    ResizeWindow {
        width: i32,
        height: i32,
    },
    MoveWindowToNextScreen,
    CopyTextToClipboard(String),
    InjectFixtureAudio(u8),
    QuitApplication,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoreModel {
    pub app_state: AppState,
    pub buffer: BufferState,
    pub ui_prefs: UiPrefs,
    pub log_line: String,
    pub runtime_error: Option<String>,
    recording_stop_policy: RecordingStopPolicyState,
}

impl Default for CoreModel {
    fn default() -> Self {
        Self {
            app_state: AppState::Idle,
            buffer: BufferState::default(),
            ui_prefs: UiPrefs::default(),
            log_line: "Ready".to_owned(),
            runtime_error: None,
            recording_stop_policy: RecordingStopPolicyState::default(),
        }
    }
}

impl CoreModel {
    pub fn reduce(&mut self, event: AppEvent) -> Vec<CoreCommand> {
        match event {
            AppEvent::MicToggled => self.reduce_mic_toggle(),
            AppEvent::LiveText(text) => self.reduce_live_text(text),
            AppEvent::CommitRequested => self.reduce_commit_requested(),
            AppEvent::RecordingStartRejected => self.reduce_recording_start_rejected(),

            AppEvent::SettingsToggled => self.reduce_settings_toggled(),
            AppEvent::TranscriptionModelChanged(model) => {
                self.reduce_transcription_model_changed(model)
            }
            AppEvent::SilenceAutoStopSecondsChanged(seconds) => {
                self.reduce_silence_auto_stop_seconds_changed(seconds)
            }
            AppEvent::VadSilenceDurationMsChanged(ms) => {
                self.reduce_vad_silence_duration_ms_changed(ms)
            }
            AppEvent::SilenceGateThresholdChanged(threshold) => {
                self.reduce_silence_gate_threshold_changed(threshold)
            }

            AppEvent::WindowLargerRequested => self.reduce_window_larger_requested(),
            AppEvent::WindowSmallerRequested => self.reduce_window_smaller_requested(),
            AppEvent::WindowResizeRequested { width, height } => {
                self.reduce_window_resize_requested(width, height)
            }
            AppEvent::WindowMoveToNextScreenRequested => {
                self.reduce_window_move_to_next_screen_requested()
            }
            AppEvent::VisibilityToggled => self.reduce_visibility_toggled(),
            AppEvent::ShowRequested => self.reduce_show_requested(),
            AppEvent::HideRequested => self.reduce_hide_requested(),

            AppEvent::RuntimeError(message) => self.reduce_runtime_error(message),
            AppEvent::ErrorCleared => self.reduce_error_cleared(),
            AppEvent::CopyRequested => self.reduce_copy_requested(),

            AppEvent::ResetRequested => {
                self.buffer.reset_all();
                Vec::new()
            }
            AppEvent::LogMessage(message) => {
                self.log_line = message;
                Vec::new()
            }
            AppEvent::FixtureInjectRequested(fixture_id) => {
                self.reduce_fixture_inject_requested(fixture_id)
            }
            AppEvent::QuitRequested => self.reduce_quit_requested(),
        }
    }

    pub fn apply_user_edit(&mut self, text: String) {
        self.buffer.replace_confirmed(text);
        self.buffer.clear_live();
    }

    pub fn evaluate_recording_stop_policy(
        &mut self,
        now: Instant,
        raw_input_level: f32,
        max_recording_duration: Option<Duration>,
    ) -> RecordingStopDecision {
        self.recording_stop_policy.evaluate(
            now,
            matches!(self.app_state, AppState::Recording),
            raw_input_level,
            self.ui_prefs.silence_auto_stop_seconds,
            self.ui_prefs.silence_gate_threshold,
            max_recording_duration,
        )
    }

    pub fn set_window_position(&mut self, left: i32, top: i32) {
        self.ui_prefs.window_left = left.max(0);
        self.ui_prefs.window_top = top.max(0);
    }

    pub fn set_window_size(&mut self, width: i32, height: i32) {
        self.ui_prefs.window_width =
            width.clamp(window::WINDOW_MIN_WIDTH, window::WINDOW_MAX_WIDTH);
        self.ui_prefs.window_height =
            height.clamp(window::WINDOW_MIN_HEIGHT, window::WINDOW_MAX_HEIGHT);
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreCommand, CoreModel};
    use crate::{AppEvent, AppState, TranscriptionModelId};

    #[test]
    fn mic_toggle_from_idle_starts_recording_pipeline() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::MicToggled);

        assert_eq!(model.app_state, AppState::Recording);
        assert_eq!(
            commands,
            vec![
                CoreCommand::StartAudioInput,
                CoreCommand::StartTranscriber {
                    model: TranscriptionModelId::Gpt4oMiniTranscribe,
                    vad_silence_duration_ms: 1_600,
                },
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
    fn transcription_model_update_is_core_owned() {
        let mut model = CoreModel::default();
        assert_eq!(
            model.ui_prefs.transcription_model,
            TranscriptionModelId::Gpt4oMiniTranscribe
        );

        let commands = model.reduce(AppEvent::TranscriptionModelChanged(
            TranscriptionModelId::Gpt4oTranscribe,
        ));
        assert!(commands.is_empty());
        assert_eq!(
            model.ui_prefs.transcription_model,
            TranscriptionModelId::Gpt4oTranscribe
        );
    }

    #[test]
    fn vad_silence_duration_update_is_core_owned() {
        let mut model = CoreModel::default();
        assert_eq!(model.ui_prefs.vad_silence_duration_ms, 1_600);

        let commands = model.reduce(AppEvent::VadSilenceDurationMsChanged(1_800));
        assert!(commands.is_empty());
        assert_eq!(model.ui_prefs.vad_silence_duration_ms, 1_800);

        let commands = model.reduce(AppEvent::VadSilenceDurationMsChanged(20));
        assert!(commands.is_empty());
        assert_eq!(model.ui_prefs.vad_silence_duration_ms, 100);
    }

    #[test]
    fn silence_gate_threshold_update_is_core_owned() {
        let mut model = CoreModel::default();
        assert!((model.ui_prefs.silence_gate_threshold - 0.30).abs() <= f32::EPSILON);

        let commands = model.reduce(AppEvent::SilenceGateThresholdChanged(0.42));
        assert!(commands.is_empty());
        assert!((model.ui_prefs.silence_gate_threshold - 0.42).abs() <= f32::EPSILON);

        let commands = model.reduce(AppEvent::SilenceGateThresholdChanged(9.9));
        assert!(commands.is_empty());
        assert!((model.ui_prefs.silence_gate_threshold - 1.0).abs() <= f32::EPSILON);
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
    fn recording_start_rejected_rolls_back_recording_state() {
        let mut model = CoreModel {
            app_state: AppState::Recording,
            ..CoreModel::default()
        };

        let commands = model.reduce(AppEvent::RecordingStartRejected);

        assert!(commands.is_empty());
        assert_eq!(model.app_state, AppState::Idle);
        assert_eq!(model.log_line, "Recording start blocked");
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
    fn window_move_to_next_screen_requested_emits_move_command() {
        let mut model = CoreModel::default();

        let commands = model.reduce(AppEvent::WindowMoveToNextScreenRequested);

        assert_eq!(commands, vec![CoreCommand::MoveWindowToNextScreen]);
    }

    #[test]
    fn window_size_updates_are_clamped_and_persisted() {
        let mut model = CoreModel::default();

        model.set_window_size(5, 9000);

        assert_eq!(model.ui_prefs.window_width, 280);
        assert_eq!(model.ui_prefs.window_height, 1280);
    }
}
