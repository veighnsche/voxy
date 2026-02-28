#[derive(Debug, Clone, Default)]
pub struct ViewModel {
    pub text: String,
    pub mic_on: bool,
    pub state_badge_text: String,
    pub log_text: String,
    pub error_message: Option<String>,
}
