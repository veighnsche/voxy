use ksni::{blocking::TrayMethods as _, MenuItem, Tray};
use tokio::sync::mpsc;
use voxy_core::AppEvent;

use crate::wiring::event_emit;

#[derive(Clone)]
struct VoxyTray {
    event_tx: mpsc::Sender<AppEvent>,
}

impl VoxyTray {
    fn emit(&self, event: AppEvent) {
        event_emit::emit_critical(&self.event_tx, event, "tray.emit");
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
        build_menu_items()
    }
}

pub struct TrayRuntime {
    handle: ksni::blocking::Handle<VoxyTray>,
}

impl TrayRuntime {
    pub fn shutdown(self) {
        // Request shutdown without blocking the GTK thread.
        // Waiting here can hang app quit if DBus/tray teardown stalls.
        let _ = self.handle.shutdown();
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

fn build_menu_items() -> Vec<MenuItem<VoxyTray>> {
    use ksni::menu::StandardItem;

    vec![
        StandardItem {
            label: "Show/Hide".to_owned(),
            activate: Box::new(|tray: &mut VoxyTray| tray.emit(AppEvent::VisibilityToggled)),
            ..Default::default()
        }
        .into(),
        StandardItem {
            label: "Move To Next Screen".to_owned(),
            activate: Box::new(|tray: &mut VoxyTray| {
                tray.emit(AppEvent::WindowMoveToNextScreenRequested)
            }),
            ..Default::default()
        }
        .into(),
        StandardItem {
            label: "Reset".to_owned(),
            activate: Box::new(|tray: &mut VoxyTray| tray.emit(AppEvent::ResetRequested)),
            ..Default::default()
        }
        .into(),
        StandardItem {
            label: "Size +".to_owned(),
            activate: Box::new(|tray: &mut VoxyTray| tray.emit(AppEvent::WindowLargerRequested)),
            ..Default::default()
        }
        .into(),
        StandardItem {
            label: "Size -".to_owned(),
            activate: Box::new(|tray: &mut VoxyTray| tray.emit(AppEvent::WindowSmallerRequested)),
            ..Default::default()
        }
        .into(),
        StandardItem {
            label: "Quit".to_owned(),
            activate: Box::new(|tray: &mut VoxyTray| tray.emit(AppEvent::QuitRequested)),
            ..Default::default()
        }
        .into(),
    ]
}
