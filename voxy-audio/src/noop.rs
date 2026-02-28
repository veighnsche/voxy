use crate::source::AudioInput;

#[derive(Debug, Default)]
pub struct NoopAudioInput;

impl AudioInput for NoopAudioInput {
    fn start(&self) {}

    fn stop(&self) {}
}
