use std::{cell::RefCell, rc::Rc, time::Duration};

use tokio::sync::mpsc;
use voxy_core::AppEvent;

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub fn start(
    event_rx: Rc<RefCell<mpsc::Receiver<AppEvent>>>,
    mut on_event: impl FnMut(AppEvent) + 'static,
    mut after_drain: impl FnMut() + 'static,
) {
    gtk4::glib::timeout_add_local(EVENT_POLL_INTERVAL, move || {
        loop {
            let event = match event_rx.borrow_mut().try_recv() {
                Ok(event) => event,
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return gtk4::glib::ControlFlow::Break;
                }
            };

            on_event(event);
        }

        after_drain();
        gtk4::glib::ControlFlow::Continue
    });
}
