#[derive(Debug, Clone, Default)]
pub struct ViewModel {
    pub text: String,
    pub mic_on: bool,
    pub settings_open: bool,
    pub silence_timeout_seconds: u64,
    pub vad_silence_ms: u32,
    pub state_badge_text: String,
    pub log_text: String,
    pub error_message: Option<String>,
}
