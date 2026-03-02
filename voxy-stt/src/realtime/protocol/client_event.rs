use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientEvent {
    SessionUpdate {
        model: String,
        input_audio_format: String,
        turn_detection: String,
        turn_detection_silence_duration_ms: u32,
    },
    InputAudioBufferAppend {
        audio: String,
    },
    InputAudioBufferCommit,
    InputAudioBufferClear,
}

impl ClientEvent {
    pub fn to_json(&self) -> Value {
        match self {
            Self::SessionUpdate {
                model,
                input_audio_format,
                turn_detection,
                turn_detection_silence_duration_ms,
            } => json!({
                "type": "transcription_session.update",
                "session": {
                    "input_audio_format": input_audio_format,
                    "input_audio_transcription": {
                        "model": model
                    },
                    "turn_detection": {
                        "type": turn_detection,
                        "silence_duration_ms": turn_detection_silence_duration_ms
                    }
                }
            }),
            Self::InputAudioBufferAppend { audio } => json!({
                "type": "input_audio_buffer.append",
                "audio": audio
            }),
            Self::InputAudioBufferCommit => json!({
                "type": "input_audio_buffer.commit"
            }),
            Self::InputAudioBufferClear => json!({
                "type": "input_audio_buffer.clear"
            }),
        }
    }
}
