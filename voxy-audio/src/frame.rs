#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmFrame {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub samples_i16: Vec<i16>,
}

impl PcmFrame {
    pub fn new(sample_rate_hz: u32, channels: u16, samples_i16: Vec<i16>) -> Self {
        Self {
            sample_rate_hz,
            channels,
            samples_i16,
        }
    }

    pub fn len_samples(&self) -> usize {
        self.samples_i16.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples_i16.is_empty()
    }
}
