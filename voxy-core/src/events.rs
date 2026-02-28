#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    MicToggled,
    ResetRequested,
    FixtureInjectRequested(u8),
    LogMessage(String),
    LiveText(String),
    CommitRequested,
    VisibilityToggled,
    ShowRequested,
    HideRequested,
    CopyRequested,
    QuitRequested,
    RuntimeError(String),
    ErrorCleared,
}
