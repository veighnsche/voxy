use std::{cell::RefCell, env};

use tokio::sync::mpsc;
use voxy_core::AppEvent;

mod status_notifier;

thread_local! {
    static TRAY_RUNTIME: RefCell<Option<status_notifier::TrayRuntime>> = const { RefCell::new(None) };
}

pub fn start(event_tx: mpsc::Sender<AppEvent>) -> Result<(), String> {
    if env_flag_enabled("VOXY_TRAY_DISABLED") {
        return Err("tray startup disabled by VOXY_TRAY_DISABLED".to_owned());
    }

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

fn env_flag_enabled(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}
