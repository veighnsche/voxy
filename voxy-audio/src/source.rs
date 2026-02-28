use crate::{AudioError, AudioRoute, PcmFrame};

pub trait AudioInput: Send + Sync {
    fn start(&self);
    fn stop(&self);

    fn set_route(&self, _route: AudioRoute) -> Result<(), AudioError> {
        Ok(())
    }

    fn current_route(&self) -> AudioRoute {
        AudioRoute::default()
    }
}

pub trait AudioFrameSource: Send + Sync {
    fn sample_rate_hz(&self) -> u32;
    fn channels(&self) -> u16;
    fn read_frame(&self) -> Option<PcmFrame>;
}
