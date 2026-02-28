#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    MicToggled,
    ResetRequested,
    LiveText(String),
    CommitRequested,
    VisibilityToggled,
    ShowRequested,
    HideRequested,
    WindowPositionUpdated { left: i32, top: i32 },
    CopyRequested,
    QuitRequested,
    RuntimeError(String),
    ErrorCleared,
}
