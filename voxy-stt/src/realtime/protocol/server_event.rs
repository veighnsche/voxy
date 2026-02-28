#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEvent {
    TranscriptionDelta { text: String },
    TranscriptionCompleted,
    TranscriptionFailed { message: String },
    Error { message: String },
    Unknown,
}
