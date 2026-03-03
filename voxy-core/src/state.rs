use crate::AppEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Idle,
    Recording,
    Processing,
    Error(String),
}

pub fn transition(current: &AppState, event: &AppEvent) -> AppState {
    match current {
        AppState::Idle => match event {
            AppEvent::MicToggled => AppState::Recording,
            _ => AppState::Idle,
        },
        AppState::Recording => match event {
            AppEvent::MicToggled => AppState::Processing,
            AppEvent::RecordingStartRejected => AppState::Idle,
            AppEvent::ResetRequested => AppState::Recording,
            _ => AppState::Recording,
        },
        AppState::Processing => match event {
            AppEvent::CommitRequested => AppState::Idle,
            AppEvent::ResetRequested => AppState::Processing,
            _ => AppState::Processing,
        },
        AppState::Error(message) => match event {
            AppEvent::ResetRequested => AppState::Error(message.clone()),
            _ => AppState::Error(message.clone()),
        },
    }
}

pub fn to_error_state(message: impl Into<String>) -> AppState {
    AppState::Error(message.into())
}

#[cfg(test)]
mod tests {
    use super::{transition, AppState};
    use crate::AppEvent;

    #[test]
    fn reset_does_not_change_recording_state() {
        assert_eq!(
            transition(&AppState::Recording, &AppEvent::ResetRequested),
            AppState::Recording
        );
    }

    #[test]
    fn reset_does_not_change_processing_state() {
        assert_eq!(
            transition(&AppState::Processing, &AppEvent::ResetRequested),
            AppState::Processing
        );
    }
}
