#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    MicToggled,
    ResetRequested,
    WindowLargerRequested,
    WindowSmallerRequested,
    WindowResizeRequested { width: i32, height: i32 },
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
