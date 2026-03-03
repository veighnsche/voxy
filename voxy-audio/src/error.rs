use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("audio lock poisoned: {0}")]
    LockPoisoned(&'static str),
    #[error("no default input audio device available")]
    CpalNoInputDevice,
    #[error("failed to get default input config: {0}")]
    CpalDefaultInputConfig(#[source] cpal::DefaultStreamConfigError),
    #[error("unsupported cpal sample format: {0}")]
    CpalUnsupportedSampleFormat(String),
    #[error("failed to build cpal input stream: {0}")]
    CpalBuildStream(#[source] cpal::BuildStreamError),
    #[error("failed to start cpal input stream: {0}")]
    CpalPlayStream(#[source] cpal::PlayStreamError),
    #[error("failed to spawn cpal capture thread: {0}")]
    CpalThreadSpawn(#[source] io::Error),
    #[error("cpal capture thread ended before startup completed")]
    CpalThreadStartup,
    #[error("cpal capture thread startup timed out after {timeout_ms}ms")]
    CpalThreadStartupTimeout { timeout_ms: u64 },
    #[error(
        "invalid audio frame configuration: frame_ms={frame_ms}, sample_rate_hz={sample_rate_hz}, channels={channels}"
    )]
    InvalidFrameConfig {
        frame_ms: usize,
        sample_rate_hz: u32,
        channels: u16,
    },
    #[error("audio frame buffer sizing overflowed (frame_samples={frame_samples}, max_frames={max_frames})")]
    FrameBufferOverflow {
        frame_samples: usize,
        max_frames: usize,
    },
    #[error("audio input is not running; start recording before injecting fixture audio")]
    FixtureInjectWhileStopped,
    #[error("fixture audio file not found: {0}")]
    FixtureNotFound(String),
    #[error("failed to read fixture audio file '{path}': {source}")]
    FixtureRead {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("failed to decode fixture audio '{path}': {message}")]
    FixtureDecode { path: String, message: String },
}
