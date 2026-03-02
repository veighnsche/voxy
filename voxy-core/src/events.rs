use crate::TranscriptionModelId;

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    MicToggled,
    ResetRequested,
    WindowLargerRequested,
    WindowSmallerRequested,
    WindowResizeRequested { width: i32, height: i32 },
    WindowMoveToNextScreenRequested,
    FixtureInjectRequested(u8),
    LogMessage(String),
    LiveText(String),
    CommitRequested,
    RecordingStartRejected,
    SettingsToggled,
    TranscriptionModelChanged(TranscriptionModelId),
    SilenceAutoStopSecondsChanged(u64),
    VadSilenceDurationMsChanged(u32),
    SilenceGateThresholdChanged(f32),
    VisibilityToggled,
    ShowRequested,
    HideRequested,
    CopyRequested,
    QuitRequested,
    RuntimeError(String),
    ErrorCleared,
}
