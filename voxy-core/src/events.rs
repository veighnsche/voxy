#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    MicToggled,
    ResetRequested,
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
