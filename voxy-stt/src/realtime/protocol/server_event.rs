use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEvent {
    TranscriptionDelta {
        text: String,
    },
    InputAudioBufferCommitted {
        item_id: Option<String>,
        previous_item_id: Option<String>,
    },
    TranscriptionCompleted {
        item_id: Option<String>,
        text: Option<String>,
    },
    TranscriptionFailed {
        message: String,
    },
    Error {
        message: String,
    },
    Unknown {
        event_type: Option<String>,
    },
}

pub fn parse_server_event(value: &Value) -> ServerEvent {
    let event_type = value.get("type").and_then(Value::as_str).map(str::to_owned);

    match event_type.as_deref() {
        Some("conversation.item.input_audio_transcription.delta") => {
            let text = value
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            ServerEvent::TranscriptionDelta { text }
        }
        Some("input_audio_buffer.committed") => {
            let item_id = value
                .get("item_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let previous_item_id = value
                .get("previous_item_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            ServerEvent::InputAudioBufferCommitted {
                item_id,
                previous_item_id,
            }
        }
        Some("conversation.item.input_audio_transcription.completed") => {
            let item_id = value
                .get("item_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let text = value
                .get("transcript")
                .and_then(Value::as_str)
                .map(str::to_owned);
            ServerEvent::TranscriptionCompleted { item_id, text }
        }
        Some("conversation.item.input_audio_transcription.failed") => {
            let message = extract_message(value);
            ServerEvent::TranscriptionFailed { message }
        }
        Some("error") => {
            let message = extract_message(value);
            ServerEvent::Error { message }
        }
        _ => ServerEvent::Unknown { event_type },
    }
}

fn extract_message(value: &Value) -> String {
    value
        .get("error")
        .and_then(Value::as_object)
        .and_then(|error| error.get("message"))
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("unknown realtime server error")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{parse_server_event, ServerEvent};

    #[test]
    fn parses_transcription_delta() {
        let event = parse_server_event(&json!({
            "type": "conversation.item.input_audio_transcription.delta",
            "delta": "hello "
        }));
        assert_eq!(
            event,
            ServerEvent::TranscriptionDelta {
                text: "hello ".to_owned()
            }
        );
    }

    #[test]
    fn parses_transcription_completed_with_transcript() {
        let event = parse_server_event(&json!({
            "type": "conversation.item.input_audio_transcription.completed",
            "item_id": "item_123",
            "transcript": "hello world"
        }));
        assert_eq!(
            event,
            ServerEvent::TranscriptionCompleted {
                item_id: Some("item_123".to_owned()),
                text: Some("hello world".to_owned())
            }
        );
    }

    #[test]
    fn parses_input_audio_buffer_committed_event() {
        let event = parse_server_event(&json!({
            "type": "input_audio_buffer.committed",
            "item_id": "item_42",
            "previous_item_id": "item_41"
        }));
        assert_eq!(
            event,
            ServerEvent::InputAudioBufferCommitted {
                item_id: Some("item_42".to_owned()),
                previous_item_id: Some("item_41".to_owned())
            }
        );
    }

    #[test]
    fn parses_error_payload_message() {
        let event = parse_server_event(&json!({
            "type": "error",
            "error": {
                "message": "api key invalid"
            }
        }));
        assert_eq!(
            event,
            ServerEvent::Error {
                message: "api key invalid".to_owned()
            }
        );
    }
}
