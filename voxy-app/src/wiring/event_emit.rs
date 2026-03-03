use tokio::sync::mpsc;
use voxy_core::AppEvent;

use crate::diagnostics::pipeline_trace;

pub fn emit_lossy(event_tx: &mpsc::Sender<AppEvent>, event: AppEvent, context: &str) {
    if let Err(error) = event_tx.try_send(event) {
        pipeline_trace::log("event-send", format!("{context} dropped event: {error}"));
    }
}

pub fn emit_critical(event_tx: &mpsc::Sender<AppEvent>, event: AppEvent, context: &'static str) {
    match event_tx.try_send(event) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Closed(_)) => {
            pipeline_trace::log("event-send", format!("{context} failed: receiver closed"));
        }
        Err(mpsc::error::TrySendError::Full(event)) => {
            pipeline_trace::log(
                "event-send",
                format!("{context} queue full; retrying with blocking send"),
            );

            let event_tx = event_tx.clone();
            if let Err(spawn_error) = std::thread::Builder::new()
                .name("voxy-critical-event-send".to_owned())
                .spawn(move || {
                    if let Err(error) = event_tx.blocking_send(event) {
                        pipeline_trace::log(
                            "event-send",
                            format!("{context} blocking retry failed: {error}"),
                        );
                    }
                })
            {
                pipeline_trace::log(
                    "event-send",
                    format!("{context} failed to spawn blocking retry thread: {spawn_error}"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time;
    use voxy_core::AppEvent;

    use super::emit_critical;

    #[tokio::test]
    async fn critical_emit_retries_when_channel_is_full() {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1);
        event_tx
            .try_send(AppEvent::LogMessage("queued".to_owned()))
            .expect("first event should fit");

        emit_critical(
            &event_tx,
            AppEvent::QuitRequested,
            "tests.critical_emit_retries_when_channel_is_full",
        );

        let first = time::timeout(Duration::from_secs(1), event_rx.recv())
            .await
            .expect("first receive should not time out")
            .expect("channel should still be open");
        assert_eq!(first, AppEvent::LogMessage("queued".to_owned()));

        let second = time::timeout(Duration::from_secs(1), event_rx.recv())
            .await
            .expect("second receive should not time out")
            .expect("channel should still be open");
        assert_eq!(second, AppEvent::QuitRequested);
    }
}
