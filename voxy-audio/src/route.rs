#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioRoute {
    Microphone,
}

impl Default for AudioRoute {
    fn default() -> Self {
        Self::Microphone
    }
}
