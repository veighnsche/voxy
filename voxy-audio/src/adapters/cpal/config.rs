pub const DEFAULT_FRAME_MS: usize = 20;
pub const DEFAULT_MAX_BUFFER_FRAMES: usize = 200;

pub fn frame_samples(sample_rate_hz: u32, channels: u16, frame_ms: usize) -> usize {
    let per_channel = ((sample_rate_hz as usize * frame_ms) / 1000).max(1);
    (per_channel * channels as usize).max(channels as usize)
}
