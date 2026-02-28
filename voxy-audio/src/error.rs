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
}
