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
            AppEvent::ResetRequested => AppState::Idle,
            AppEvent::LiveText(_) => AppState::Idle,
            AppEvent::CommitRequested => AppState::Idle,
        },
        AppState::Recording => match event {
            AppEvent::MicToggled => AppState::Processing,
            AppEvent::ResetRequested => AppState::Idle,
            AppEvent::LiveText(_) => AppState::Recording,
            AppEvent::CommitRequested => AppState::Recording,
        },
        AppState::Processing => match event {
            AppEvent::MicToggled => AppState::Processing,
            AppEvent::ResetRequested => AppState::Idle,
            AppEvent::LiveText(_) => AppState::Processing,
            AppEvent::CommitRequested => AppState::Idle,
        },
        AppState::Error(_) => match event {
            AppEvent::ResetRequested => AppState::Idle,
            AppEvent::MicToggled => current.clone(),
            AppEvent::LiveText(_) => current.clone(),
            AppEvent::CommitRequested => current.clone(),
        },
    }
}

pub fn to_error_state(message: impl Into<String>) -> AppState {
    AppState::Error(message.into())
}
