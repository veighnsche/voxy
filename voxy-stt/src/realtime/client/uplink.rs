use std::sync::atomic::Ordering;

use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::{self, Message};

use crate::{
    realtime::{audio_uplink, protocol::client_event::ClientEvent},
    trace,
    traits::TranscriberInput,
};

use super::UPLINK_SEQ;

pub(super) async fn send_client_event(
    writer: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    event: ClientEvent,
) -> Result<(), tungstenite::Error> {
    let summary = summarize_client_event(&event);
    let payload = event.to_json().to_string();
    let seq = UPLINK_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let is_append = matches!(event, ClientEvent::InputAudioBufferAppend { .. });
    let should_log = if is_append {
        trace::should_log_noisy(seq)
    } else {
        trace::should_log(seq)
    };
    if should_log {
        trace::log("uplink", format!("send#{} {}", seq, summary));
    }
    writer.send(Message::Text(payload)).await
}

pub(super) async fn handle_uplink_input(
    writer: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    input: TranscriberInput,
) -> Result<(), tungstenite::Error> {
    match input {
        TranscriberInput::AudioFrame(frame) => {
            let Some(chunk) = audio_uplink::encode_frame_to_base64(&frame) else {
                trace::log("uplink", "skip empty audio frame");
                return Ok(());
            };
            send_client_event(
                writer,
                ClientEvent::InputAudioBufferAppend {
                    audio: chunk.base64_pcm16,
                },
            )
            .await
        }
        TranscriberInput::Commit => {
            trace::log("uplink", "commit requested");
            send_client_event(writer, ClientEvent::InputAudioBufferCommit).await
        }
        TranscriberInput::Clear => {
            trace::log("uplink", "clear requested");
            send_client_event(writer, ClientEvent::InputAudioBufferClear).await
        }
    }
}

pub(super) fn input_kind(input: &TranscriberInput) -> &'static str {
    match input {
        TranscriberInput::AudioFrame(_) => "AudioFrame",
        TranscriberInput::Commit => "Commit",
        TranscriberInput::Clear => "Clear",
    }
}

fn summarize_client_event(event: &ClientEvent) -> String {
    match event {
        ClientEvent::SessionUpdate {
            model,
            input_audio_format,
            turn_detection,
            turn_detection_silence_duration_ms,
        } => format!(
            "event=transcription_session.update model={} format={} turn_detection={} silence_ms={}",
            model, input_audio_format, turn_detection, turn_detection_silence_duration_ms
        ),
        ClientEvent::InputAudioBufferAppend { audio } => format!(
            "event=input_audio_buffer.append audio_base64_len={}",
            audio.len()
        ),
        ClientEvent::InputAudioBufferCommit => "event=input_audio_buffer.commit".to_owned(),
        ClientEvent::InputAudioBufferClear => "event=input_audio_buffer.clear".to_owned(),
    }
}
