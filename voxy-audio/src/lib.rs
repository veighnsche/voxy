pub mod adapters;
pub mod engine;
pub mod error;
pub mod frame;
pub mod route;
pub mod source;
mod trace;

pub use adapters::{cpal::CpalAudioInput, noop::NoopAudioInput};
pub use engine::{InputEngine, SessionSnapshot};
pub use error::AudioError;
pub use frame::PcmFrame;
pub use route::AudioRoute;
pub use source::{AudioFrameSource, AudioInput};

pub mod cpal_source {
    pub use crate::adapters::cpal::CpalAudioInput;
}

pub mod noop {
    pub use crate::adapters::noop::NoopAudioInput;
}
