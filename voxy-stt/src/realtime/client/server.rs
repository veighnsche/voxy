use std::sync::atomic::Ordering;

use tokio::sync::{broadcast, mpsc};
use voxy_core::AppEvent;

use crate::{
    realtime::{
        event_mapper::map_server_event,
        protocol::server_event::{parse_server_event, ServerEvent},
    },
    trace,
    traits::TranscriberOutput,
};

use super::stop_flush::completion_matches_expected;
use super::SERVER_PAYLOAD_SEQ;

pub(super) async fn handle_server_payload(
    tx: &mpsc::Sender<AppEvent>,
    downlink_tx: &broadcast::Sender<TranscriberOutput>,
    payload: &str,
    stop_commit_pending: bool,
    stop_commit_item_id: Option<&str>,
) -> Option<ServerEvent> {
    let value = match serde_json::from_str::<serde_json::Value>(payload) {
        Ok(value) => value,
        Err(error) => {
            trace::log(
                "server",
                format!("discarding malformed server payload: {error}"),
            );
            let _ = tx
                .send(AppEvent::LogMessage(format!(
                    "Ignored malformed realtime payload: {error}"
                )))
                .await;
            return None;
        }
    };

    let event = parse_server_event(&value);
    let seq = SERVER_PAYLOAD_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    if trace::should_log(seq) {
        trace::log("server", format!("parsed_event={event:?}"));
    }
    let suppress_benign_stop_commit_error =
        stop_commit_pending && is_benign_stop_commit_empty_buffer_error(&event);
    let should_forward_to_app = !suppress_benign_stop_commit_error
        && should_forward_server_event_to_app(&event, stop_commit_pending, stop_commit_item_id);
    if suppress_benign_stop_commit_error {
        trace::log(
            "session",
            "ignored benign stop-commit empty-buffer error from server",
        );
    }
    if let Some(app_event) = map_server_event(&event) {
        if should_forward_to_app {
            if trace::should_log(seq) {
                trace::log("server", format!("mapped_app_event={app_event:?}"));
            }
            let _ = tx.send(app_event).await;
        } else if trace::should_log(seq) {
            trace::log(
                "server",
                format!("ignored stale mapped_app_event={app_event:?}"),
            );
        }
    }

    match &event {
        crate::realtime::protocol::server_event::ServerEvent::InputAudioBufferCommitted {
            ..
        } => {}
        crate::realtime::protocol::server_event::ServerEvent::TranscriptionDelta { text } => {
            if !text.is_empty() {
                let _ = downlink_tx.send(TranscriberOutput::LiveText(text.clone()));
            }
        }
        crate::realtime::protocol::server_event::ServerEvent::TranscriptionCompleted { .. } => {
            if should_forward_to_app {
                let _ = downlink_tx.send(TranscriberOutput::SegmentCommitted);
            }
        }
        crate::realtime::protocol::server_event::ServerEvent::TranscriptionFailed { message }
        | crate::realtime::protocol::server_event::ServerEvent::Error { message } => {
            if !suppress_benign_stop_commit_error {
                let _ = downlink_tx.send(TranscriberOutput::Error(message.clone()));
            }
        }
        crate::realtime::protocol::server_event::ServerEvent::Unknown { event_type } => {
            if let Some(event_type) = event_type {
                let _ = tx
                    .send(AppEvent::LogMessage(format!(
                        "Ignored unsupported realtime event type: {event_type}"
                    )))
                    .await;
            }
        }
    }

    Some(event)
}

pub(super) fn should_forward_server_event_to_app(
    event: &ServerEvent,
    stop_commit_pending: bool,
    stop_commit_item_id: Option<&str>,
) -> bool {
    if !stop_commit_pending {
        return true;
    }

    match event {
        ServerEvent::TranscriptionCompleted { item_id, .. } => {
            completion_matches_expected(stop_commit_item_id, item_id.as_deref())
        }
        _ => true,
    }
}

fn is_benign_stop_commit_empty_buffer_error(event: &ServerEvent) -> bool {
    let message = match event {
        ServerEvent::TranscriptionFailed { message } | ServerEvent::Error { message } => message,
        _ => return false,
    };

    // Realtime may reject stop-time commit when VAD/silence has already drained the buffer.
    // This specific error is expected and should not surface as a user-facing runtime error.
    let normalized = message.trim().to_ascii_lowercase();
    normalized.contains("error committing input audio buffer")
        && normalized.contains("buffer too small")
        && normalized.contains("expected at least")
        && normalized.contains("buffer only has")
}
