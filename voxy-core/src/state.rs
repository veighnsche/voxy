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
            AppEvent::ResetRequested => AppState::Idle,
            _ => AppState::Recording,
        },
        AppState::Processing => match event {
            AppEvent::CommitRequested | AppEvent::ResetRequested => AppState::Idle,
            _ => AppState::Processing,
        },
        AppState::Error(_) => match event {
            AppEvent::ResetRequested => AppState::Idle,
            _ => current.clone(),
        },
    }
}

pub fn to_error_state(message: impl Into<String>) -> AppState {
    AppState::Error(message.into())
}
