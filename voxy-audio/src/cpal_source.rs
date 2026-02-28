use crate::source::AudioInput;

#[derive(Debug, Default)]
pub struct CpalAudioInput;

impl AudioInput for CpalAudioInput {
    fn start(&self) {
        // Stub: real CPAL capture will be implemented in a later phase.
    }

    fn stop(&self) {
        // Stub: real CPAL capture will be implemented in a later phase.
    }
}
