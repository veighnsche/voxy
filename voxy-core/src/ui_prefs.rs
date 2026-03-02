use crate::{
    TranscriptionModelId, DEFAULT_SILENCE_AUTO_STOP_SECONDS, DEFAULT_SILENCE_GATE_THRESHOLD,
    DEFAULT_VAD_SILENCE_DURATION_MS,
};

#[derive(Debug, Clone, PartialEq)]
pub struct UiPrefs {
    pub visible: bool,
    pub settings_open: bool,
    pub transcription_model: TranscriptionModelId,
    pub silence_auto_stop_seconds: u64,
    pub vad_silence_duration_ms: u32,
    pub silence_gate_threshold: f32,
    pub window_left: i32,
    pub window_top: i32,
    pub window_width: i32,
    pub window_height: i32,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            visible: true,
            settings_open: false,
            transcription_model: TranscriptionModelId::default(),
            silence_auto_stop_seconds: DEFAULT_SILENCE_AUTO_STOP_SECONDS,
            vad_silence_duration_ms: DEFAULT_VAD_SILENCE_DURATION_MS,
            silence_gate_threshold: DEFAULT_SILENCE_GATE_THRESHOLD,
            window_left: 24,
            window_top: 24,
            window_width: 360,
            window_height: 420,
        }
    }
}
