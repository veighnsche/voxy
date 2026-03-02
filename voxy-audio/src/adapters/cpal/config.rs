use std::{env, sync::OnceLock};

pub const DEFAULT_FRAME_MS: usize = 20;
pub const DEFAULT_MAX_BUFFER_FRAMES: usize = 200;
const FRAME_MS_ENV: &str = "VOXY_AUDIO_FRAME_MS";

pub fn frame_ms() -> usize {
    static FRAME_MS: OnceLock<usize> = OnceLock::new();
    *FRAME_MS.get_or_init(|| {
        let raw = env::var(FRAME_MS_ENV).ok();
        parse_frame_ms(raw.as_deref())
    })
}

pub fn frame_samples(sample_rate_hz: u32, channels: u16, frame_ms: usize) -> usize {
    let per_channel = ((sample_rate_hz as usize * frame_ms) / 1000).max(1);
    (per_channel * channels as usize).max(channels as usize)
}

fn parse_frame_ms(value: Option<&str>) -> usize {
    value
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_FRAME_MS)
}

#[cfg(test)]
mod tests {
    use super::{parse_frame_ms, DEFAULT_FRAME_MS};

    #[test]
    fn parse_frame_ms_falls_back_to_default() {
        assert_eq!(parse_frame_ms(None), DEFAULT_FRAME_MS);
        assert_eq!(parse_frame_ms(Some("abc")), DEFAULT_FRAME_MS);
        assert_eq!(parse_frame_ms(Some("0")), DEFAULT_FRAME_MS);
    }

    #[test]
    fn parse_frame_ms_accepts_positive_integer() {
        assert_eq!(parse_frame_ms(Some("15")), 15);
        assert_eq!(parse_frame_ms(Some(" 30 ")), 30);
    }
}
