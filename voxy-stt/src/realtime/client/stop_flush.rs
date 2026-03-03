use crate::{realtime::protocol::server_event::ServerEvent, trace};

pub(super) fn observe_stop_flush_progress(
    event: &ServerEvent,
    stop_commit_pending: &mut bool,
    stop_commit_item_id: &mut Option<String>,
    stop_completion_received: &mut bool,
) {
    if !*stop_commit_pending {
        return;
    }

    match event {
        ServerEvent::InputAudioBufferCommitted {
            item_id,
            previous_item_id,
        } => {
            *stop_commit_item_id = item_id.clone();
            trace::log(
                "session",
                format!(
                    "stop flush commit ack item_id={} previous_item_id={}",
                    item_id.as_deref().unwrap_or("<none>"),
                    previous_item_id.as_deref().unwrap_or("<none>")
                ),
            );
        }
        ServerEvent::TranscriptionCompleted { item_id, .. } => {
            if completion_matches_expected(stop_commit_item_id.as_deref(), item_id.as_deref()) {
                *stop_completion_received = true;
            } else {
                trace::log(
                    "session",
                    format!(
                        "ignoring completion for non-matching item expected={} got={}",
                        stop_commit_item_id.as_deref().unwrap_or("<none>"),
                        item_id.as_deref().unwrap_or("<none>")
                    ),
                );
            }
        }
        ServerEvent::TranscriptionFailed { .. } | ServerEvent::Error { .. } => {
            *stop_completion_received = true;
        }
        ServerEvent::TranscriptionDelta { .. } | ServerEvent::Unknown { .. } => {}
    }
}

pub(super) fn completion_matches_expected(
    expected_item_id: Option<&str>,
    observed_item_id: Option<&str>,
) -> bool {
    match expected_item_id {
        Some(expected) => observed_item_id
            .map(|item_id| item_id == expected)
            .unwrap_or(false),
        None => observed_item_id.is_none(),
    }
}
