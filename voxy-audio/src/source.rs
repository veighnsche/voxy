use crate::PcmFrame;

pub trait AudioInput: Send + Sync {
    fn start(&self);
    fn stop(&self);
}

pub trait AudioFrameSource: Send + Sync {
    fn sample_rate_hz(&self) -> u32;
    fn channels(&self) -> u16;
    fn read_frame(&self) -> Option<PcmFrame>;
}
