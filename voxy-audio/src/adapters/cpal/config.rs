use std::{env, sync::OnceLock};

pub const DEFAULT_FRAME_MS: usize = 20;
pub const MIN_FRAME_MS: usize = 5;
pub const MAX_FRAME_MS: usize = 200;
pub const DEFAULT_MAX_BUFFER_FRAMES: usize = 200;
const FRAME_MS_ENV: &str = "VOXY_AUDIO_FRAME_MS";

pub fn frame_ms() -> usize {
    static FRAME_MS: OnceLock<usize> = OnceLock::new();
    *FRAME_MS.get_or_init(|| {
        let raw = env::var(FRAME_MS_ENV).ok();
        parse_frame_ms(raw.as_deref())
    })
}

pub fn frame_samples(sample_rate_hz: u32, channels: u16, frame_ms: usize) -> Option<usize> {
    let sample_rate_hz = sample_rate_hz as usize;
    let channels = channels as usize;
    if channels == 0 {
        return None;
    }

    let per_channel = sample_rate_hz
        .checked_mul(frame_ms)?
        .checked_div(1000)?
        .max(1);
    per_channel
        .checked_mul(channels)
        .map(|total| total.max(channels))
}

fn parse_frame_ms(value: Option<&str>) -> usize {
    value
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| (*value >= MIN_FRAME_MS) && (*value <= MAX_FRAME_MS))
        .unwrap_or(DEFAULT_FRAME_MS)
}

#[cfg(test)]
mod tests {
    use super::{frame_samples, parse_frame_ms, DEFAULT_FRAME_MS, MAX_FRAME_MS, MIN_FRAME_MS};

    #[test]
    fn parse_frame_ms_falls_back_to_default() {
        assert_eq!(parse_frame_ms(None), DEFAULT_FRAME_MS);
        assert_eq!(parse_frame_ms(Some("abc")), DEFAULT_FRAME_MS);
        assert_eq!(parse_frame_ms(Some("0")), DEFAULT_FRAME_MS);
        assert_eq!(parse_frame_ms(Some("4")), DEFAULT_FRAME_MS);
        assert_eq!(parse_frame_ms(Some("500")), DEFAULT_FRAME_MS);
    }

    #[test]
    fn parse_frame_ms_accepts_in_range_integer() {
        assert_eq!(parse_frame_ms(Some("15")), 15);
        assert_eq!(parse_frame_ms(Some(" 30 ")), 30);
        assert_eq!(
            parse_frame_ms(Some(&MIN_FRAME_MS.to_string())),
            MIN_FRAME_MS
        );
        assert_eq!(
            parse_frame_ms(Some(&MAX_FRAME_MS.to_string())),
            MAX_FRAME_MS
        );
    }

    #[test]
    fn frame_samples_uses_checked_math() {
        assert_eq!(frame_samples(16_000, 1, 20), Some(320));
        assert_eq!(frame_samples(16_000, 2, 20), Some(640));
        assert_eq!(frame_samples(16_000, 0, 20), None);
    }
}
