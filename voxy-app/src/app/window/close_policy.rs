use gtk4::{glib::Propagation, prelude::*, ApplicationWindow};
use tokio::sync::mpsc;
use voxy_core::AppEvent;

pub fn install_hide_on_close(window: &ApplicationWindow, event_tx: mpsc::Sender<AppEvent>) {
    window.connect_close_request(move |_| {
        let _ = event_tx.try_send(AppEvent::HideRequested);
        Propagation::Stop
    });
}
