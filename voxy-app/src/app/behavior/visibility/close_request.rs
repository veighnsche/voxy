use gtk4::{glib::Propagation, prelude::*, ApplicationWindow};
use tokio::sync::mpsc;
use voxy_core::AppEvent;

use crate::wiring::event_emit;

pub fn install_close_behavior(
    window: &ApplicationWindow,
    event_tx: mpsc::Sender<AppEvent>,
    tray_available: bool,
) {
    window.connect_close_request(move |window| {
        if tray_available {
            event_emit::emit_critical(&event_tx, AppEvent::HideRequested, "window.close.hide");
            Propagation::Stop
        } else {
            if let Some(app) = window.application() {
                app.quit();
            }
            event_emit::emit_critical(&event_tx, AppEvent::QuitRequested, "window.close.quit");
            Propagation::Proceed
        }
    });
}
