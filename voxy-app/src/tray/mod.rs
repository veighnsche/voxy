use std::cell::RefCell;

use tokio::sync::mpsc;
use voxy_core::AppEvent;

mod menu;
mod status_notifier;

thread_local! {
    static TRAY_RUNTIME: RefCell<Option<status_notifier::TrayRuntime>> = const { RefCell::new(None) };
}

pub fn start(event_tx: mpsc::Sender<AppEvent>) -> Result<(), String> {
    TRAY_RUNTIME.with(|slot| {
        if slot.borrow().is_some() {
            return Ok(());
        }

        let runtime = status_notifier::start(event_tx)?;
        *slot.borrow_mut() = Some(runtime);
        Ok(())
    })
}

pub fn shutdown() {
    TRAY_RUNTIME.with(|slot| {
        if let Some(runtime) = slot.borrow_mut().take() {
            runtime.shutdown();
        }
    });
}
