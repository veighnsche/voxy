use std::fmt;

use tokio::sync::broadcast;
use voxy_audio::PcmFrame;
use voxy_core::TranscriptionModelId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriberSessionConfig {
    pub model: TranscriptionModelId,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub vad_silence_duration_ms: u32,
}

impl TranscriberSessionConfig {
    pub fn from_model(model: TranscriptionModelId) -> Self {
        Self {
            model,
            sample_rate_hz: 16_000,
            channels: 1,
            vad_silence_duration_ms: 1_600,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriberInput {
    AudioFrame(PcmFrame),
    Commit,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriberOutput {
    SessionStarted(TranscriberSessionConfig),
    LiveText(String),
    SegmentCommitted,
    SegmentCleared,
    SessionStopped,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriberStreamState {
    Idle,
    Streaming,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriberContractError {
    AlreadyRunning,
    NotRunning,
    UplinkClosed,
    Internal(String),
}

impl fmt::Display for TranscriberContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "transcriber session is already running"),
            Self::NotRunning => write!(f, "transcriber session is not running"),
            Self::UplinkClosed => write!(f, "transcriber uplink channel is closed"),
            Self::Internal(message) => write!(f, "transcriber internal error: {message}"),
        }
    }
}

impl std::error::Error for TranscriberContractError {}

#[allow(async_fn_in_trait)]
pub trait StreamingTranscriber: Send + Sync {
    async fn start(&self, config: TranscriberSessionConfig)
        -> Result<(), TranscriberContractError>;
    async fn push_input(&self, input: TranscriberInput) -> Result<(), TranscriberContractError>;
    async fn stop(&self) -> Result<(), TranscriberContractError>;
    fn subscribe(&self) -> broadcast::Receiver<TranscriberOutput>;
    fn state(&self) -> TranscriberStreamState;
}
