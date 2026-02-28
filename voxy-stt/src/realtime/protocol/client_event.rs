#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientEvent {
    SessionUpdate {
        model: String,
        input_audio_format: String,
        turn_detection: String,
    },
    InputAudioBufferAppend {
        audio: String,
    },
    InputAudioBufferCommit,
    InputAudioBufferClear,
}
