#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptionModel {
    Gpt4oMiniTranscribe,
    Gpt4oTranscribe,
}

impl TranscriptionModel {
    pub const ALL: [Self; 2] = [Self::Gpt4oMiniTranscribe, Self::Gpt4oTranscribe];

    pub fn as_api_id(self) -> &'static str {
        match self {
            Self::Gpt4oMiniTranscribe => "gpt-4o-mini-transcribe",
            Self::Gpt4oTranscribe => "gpt-4o-transcribe",
        }
    }

    pub fn as_label(self) -> &'static str {
        match self {
            Self::Gpt4oMiniTranscribe => "4o Mini Transcribe",
            Self::Gpt4oTranscribe => "4o Transcribe",
        }
    }

    pub fn from_api_id(value: &str) -> Option<Self> {
        match value {
            "gpt-4o-mini-transcribe" => Some(Self::Gpt4oMiniTranscribe),
            "gpt-4o-transcribe" => Some(Self::Gpt4oTranscribe),
            _ => None,
        }
    }
}

impl Default for TranscriptionModel {
    fn default() -> Self {
        Self::Gpt4oMiniTranscribe
    }
}
