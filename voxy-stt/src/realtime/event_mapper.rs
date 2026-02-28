use voxy_core::AppEvent;

use crate::realtime::protocol::server_event::ServerEvent;

pub fn map_server_event(event: ServerEvent) -> Option<AppEvent> {
    match event {
        ServerEvent::TranscriptionDelta { text } if !text.is_empty() => {
            Some(AppEvent::LiveText(text))
        }
        ServerEvent::TranscriptionCompleted { .. } => Some(AppEvent::CommitRequested),
        ServerEvent::TranscriptionFailed { message } => Some(AppEvent::RuntimeError(message)),
        ServerEvent::Error { message } => Some(AppEvent::RuntimeError(message)),
        ServerEvent::Unknown { .. } => None,
        ServerEvent::TranscriptionDelta { .. } => None,
    }
}
