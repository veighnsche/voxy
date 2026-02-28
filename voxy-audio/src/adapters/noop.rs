use crate::{AudioInput, AudioRoute};

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAudioInput;

impl AudioInput for NoopAudioInput {
    fn start(&self) {}

    fn stop(&self) {}

    fn current_route(&self) -> AudioRoute {
        AudioRoute::Microphone
    }
}
