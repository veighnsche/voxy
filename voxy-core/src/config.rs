pub const DEFAULT_MAX_RECORDING_SECONDS: u64 = 30 * 60;
pub const DEFAULT_SILENCE_AUTO_STOP_SECONDS: u64 = 10;
pub const MAX_SILENCE_AUTO_STOP_SECONDS: u64 = 600;
pub const DEFAULT_VAD_SILENCE_DURATION_MS: u32 = 1_600;
pub const MIN_VAD_SILENCE_DURATION_MS: u32 = 100;
pub const MAX_VAD_SILENCE_DURATION_MS: u32 = 5_000;
pub const DEFAULT_SILENCE_GATE_THRESHOLD: f32 = 0.30;

pub fn clamp_silence_auto_stop_seconds(seconds: u64) -> u64 {
    seconds.min(MAX_SILENCE_AUTO_STOP_SECONDS)
}

pub fn clamp_vad_silence_duration_ms(ms: u32) -> u32 {
    ms.clamp(MIN_VAD_SILENCE_DURATION_MS, MAX_VAD_SILENCE_DURATION_MS)
}

pub fn clamp_silence_gate_threshold(threshold: f32) -> f32 {
    if !threshold.is_finite() {
        return DEFAULT_SILENCE_GATE_THRESHOLD;
    }
    threshold.clamp(0.0, 1.0)
}

pub fn parse_max_recording_seconds(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_RECORDING_SECONDS)
}

pub fn parse_silence_auto_stop_seconds(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(clamp_silence_auto_stop_seconds)
        .unwrap_or(DEFAULT_SILENCE_AUTO_STOP_SECONDS)
}

pub fn parse_vad_silence_ms(value: Option<&str>) -> u32 {
    value
        .and_then(|value| value.trim().parse::<u32>().ok())
        .map(clamp_vad_silence_duration_ms)
        .unwrap_or(DEFAULT_VAD_SILENCE_DURATION_MS)
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_silence_auto_stop_seconds, clamp_silence_gate_threshold,
        clamp_vad_silence_duration_ms, parse_max_recording_seconds,
        parse_silence_auto_stop_seconds, parse_vad_silence_ms, DEFAULT_MAX_RECORDING_SECONDS,
        DEFAULT_SILENCE_AUTO_STOP_SECONDS, DEFAULT_SILENCE_GATE_THRESHOLD,
        DEFAULT_VAD_SILENCE_DURATION_MS,
    };

    #[test]
    fn parse_max_recording_seconds_falls_back_to_default() {
        assert_eq!(
            parse_max_recording_seconds(None),
            DEFAULT_MAX_RECORDING_SECONDS
        );
        assert_eq!(
            parse_max_recording_seconds(Some("not-a-number")),
            DEFAULT_MAX_RECORDING_SECONDS
        );
    }

    #[test]
    fn parse_max_recording_seconds_accepts_zero_as_disable_value() {
        assert_eq!(parse_max_recording_seconds(Some("0")), 0);
        assert_eq!(parse_max_recording_seconds(Some("30")), 30);
    }

    #[test]
    fn parse_silence_auto_stop_seconds_falls_back_to_default() {
        assert_eq!(
            parse_silence_auto_stop_seconds(None),
            DEFAULT_SILENCE_AUTO_STOP_SECONDS
        );
        assert_eq!(
            parse_silence_auto_stop_seconds(Some("bad")),
            DEFAULT_SILENCE_AUTO_STOP_SECONDS
        );
    }

    #[test]
    fn parse_silence_auto_stop_seconds_clamps_to_supported_bounds() {
        assert_eq!(parse_silence_auto_stop_seconds(Some("7")), 7);
        assert_eq!(parse_silence_auto_stop_seconds(Some("700")), 600);
    }

    #[test]
    fn parse_vad_silence_ms_falls_back_to_default() {
        assert_eq!(parse_vad_silence_ms(None), DEFAULT_VAD_SILENCE_DURATION_MS);
        assert_eq!(
            parse_vad_silence_ms(Some("invalid")),
            DEFAULT_VAD_SILENCE_DURATION_MS
        );
    }

    #[test]
    fn parse_vad_silence_ms_clamps_to_supported_bounds() {
        assert_eq!(parse_vad_silence_ms(Some("50")), 100);
        assert_eq!(parse_vad_silence_ms(Some("1200")), 1_200);
        assert_eq!(parse_vad_silence_ms(Some("6000")), 5_000);
    }

    #[test]
    fn clamp_helpers_bound_values() {
        assert_eq!(clamp_silence_auto_stop_seconds(900), 600);
        assert_eq!(clamp_vad_silence_duration_ms(5), 100);
        assert_eq!(clamp_vad_silence_duration_ms(9_000), 5_000);
        assert_eq!(clamp_silence_gate_threshold(-2.0), 0.0);
        assert_eq!(clamp_silence_gate_threshold(2.0), 1.0);
        assert_eq!(
            clamp_silence_gate_threshold(f32::NAN),
            DEFAULT_SILENCE_GATE_THRESHOLD
        );
        assert_eq!(
            clamp_silence_gate_threshold(f32::INFINITY),
            DEFAULT_SILENCE_GATE_THRESHOLD
        );
    }
}
