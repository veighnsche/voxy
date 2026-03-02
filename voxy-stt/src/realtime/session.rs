use voxy_core::TranscriptionModelId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    pub model: TranscriptionModelId,
    pub input_audio_format: &'static str,
    pub turn_detection: &'static str,
}

impl SessionConfig {
    pub fn for_model(model: TranscriptionModelId) -> Self {
        Self {
            model,
            input_audio_format: "pcm16",
            turn_detection: "server_vad",
        }
    }
}
