use ksni::{menu::StandardItem, MenuItem};
use voxy_core::AppEvent;

use crate::tray::status_notifier::VoxyTray;

pub(super) fn build_menu_items() -> Vec<MenuItem<VoxyTray>> {
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
