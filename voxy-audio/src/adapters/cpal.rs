use crate::{AudioFrameSource, AudioInput, AudioRoute, PcmFrame};

#[derive(Debug, Default, Clone, Copy)]
pub struct CpalAudioInput;

impl AudioInput for CpalAudioInput {
    fn start(&self) {
        // Stub: real CPAL input will be connected in a later phase.
    }

    fn stop(&self) {
        // Stub: real CPAL input will be connected in a later phase.
    }

    fn current_route(&self) -> AudioRoute {
        AudioRoute::Microphone
    }
}

#[derive(Debug, Clone)]
pub struct CpalFrameSource {
    sample_rate_hz: u32,
    channels: u16,
}

impl Default for CpalFrameSource {
    fn default() -> Self {
        Self {
            sample_rate_hz: 16_000,
            channels: 1,
        }
    }
}

impl AudioFrameSource for CpalFrameSource {
    fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn read_frame(&self) -> Option<PcmFrame> {
        None
    }
}
