#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPrefs {
    pub visible: bool,
    pub settings_open: bool,
    pub silence_auto_stop_seconds: u64,
    pub vad_silence_duration_ms: u32,
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
            silence_auto_stop_seconds: 10,
            vad_silence_duration_ms: 1_600,
            window_left: 24,
            window_top: 24,
            window_width: 360,
            window_height: 420,
        }
    }
}
