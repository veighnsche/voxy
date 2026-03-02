use std::{cell::RefCell, env, rc::Rc, sync::OnceLock, time::Duration};

use tokio::sync::mpsc;
use voxy_core::AppEvent;

const EVENT_POLL_MS_ENV: &str = "VOXY_UI_EVENT_POLL_MS";
const DEFAULT_EVENT_POLL_MS: u64 = 16;

pub fn start(
    event_rx: Rc<RefCell<mpsc::Receiver<AppEvent>>>,
    mut on_event: impl FnMut(AppEvent) + 'static,
    mut after_drain: impl FnMut() + 'static,
) {
    let poll_interval = event_poll_interval();
    crate::diagnostics::pipeline_trace::log(
        "event-loop",
        format!("poll_interval_ms={}", poll_interval.as_millis()),
    );
    gtk4::glib::timeout_add_local(poll_interval, move || {
        let mut drained_any = false;

        loop {
            let event = match event_rx.borrow_mut().try_recv() {
                Ok(event) => event,
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return gtk4::glib::ControlFlow::Break;
                }
            };

            crate::diagnostics::pipeline_trace::log("event-loop", format!("dispatch {event:?}"));
            on_event(event);
            drained_any = true;
        }

        if drained_any {
            crate::diagnostics::pipeline_trace::log("event-loop", "after_drain render");
            after_drain();
        }
        gtk4::glib::ControlFlow::Continue
    });
}

fn event_poll_interval() -> Duration {
    static POLL_MS: OnceLock<u64> = OnceLock::new();
    let poll_ms = *POLL_MS.get_or_init(|| {
        let raw = env::var(EVENT_POLL_MS_ENV).ok();
        parse_event_poll_ms(raw.as_deref())
    });

    Duration::from_millis(poll_ms)
}

fn parse_event_poll_ms(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_EVENT_POLL_MS)
}

#[cfg(test)]
mod tests {
    use super::{parse_event_poll_ms, DEFAULT_EVENT_POLL_MS};

    #[test]
    fn parse_event_poll_ms_falls_back_to_default() {
        assert_eq!(parse_event_poll_ms(None), DEFAULT_EVENT_POLL_MS);
        assert_eq!(parse_event_poll_ms(Some("abc")), DEFAULT_EVENT_POLL_MS);
        assert_eq!(parse_event_poll_ms(Some("0")), DEFAULT_EVENT_POLL_MS);
    }

    #[test]
    fn parse_event_poll_ms_accepts_positive_integer() {
        assert_eq!(parse_event_poll_ms(Some("25")), 25);
        assert_eq!(parse_event_poll_ms(Some(" 40 ")), 40);
    }
}
