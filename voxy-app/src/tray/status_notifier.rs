use ksni::{blocking::TrayMethods as _, MenuItem, Tray};
use tokio::sync::mpsc;
use voxy_core::AppEvent;

use crate::tray::menu;

#[derive(Clone)]
pub(super) struct VoxyTray {
    event_tx: mpsc::Sender<AppEvent>,
}

impl VoxyTray {
    pub(super) fn emit(&self, event: AppEvent) {
        let _ = self.event_tx.try_send(event);
    }
}

impl Tray for VoxyTray {
    fn id(&self) -> String {
        "voxy".to_owned()
    }

    fn title(&self) -> String {
        "Voxy".to_owned()
    }

    fn icon_name(&self) -> String {
        "audio-input-microphone-symbolic".to_owned()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.emit(AppEvent::VisibilityToggled);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        menu::build_menu_items()
    }
}

pub struct TrayRuntime {
    handle: ksni::blocking::Handle<VoxyTray>,
}

impl TrayRuntime {
    pub fn shutdown(self) {
        self.handle.shutdown().wait();
    }
}

pub(super) fn start(event_tx: mpsc::Sender<AppEvent>) -> Result<TrayRuntime, String> {
    let tray = VoxyTray { event_tx };
    let handle = tray
        .assume_sni_available(true)
        .spawn()
        .map_err(|error| format!("failed to initialize tray status notifier: {error}"))?;

    Ok(TrayRuntime { handle })
}
