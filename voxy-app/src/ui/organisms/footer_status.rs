use gtk4::{prelude::*, Align, Box as GtkBox, Label, Orientation};

use crate::ui::{atoms, molecules};

#[derive(Clone)]
pub struct FooterStatus {
    pub container: GtkBox,
    pub state_badge: Label,
    pub recording_indicator: molecules::recording_indicator::RecordingIndicator,
    pub input_level_meter: atoms::input_level_meter::InputLevelMeter,
    pub log_display: Label,
}

pub fn build() -> FooterStatus {
    let container = GtkBox::new(Orientation::Horizontal, 8);
    container.set_hexpand(true);

    let state_badge = atoms::state_badge::build();
    let recording_indicator = molecules::recording_indicator::build();
    let input_level_meter = atoms::input_level_meter::build();
    let log_display = atoms::log_display::build();

    let left_slot = GtkBox::new(Orientation::Horizontal, 8);
    left_slot.set_halign(Align::Start);
    left_slot.set_hexpand(false);
    left_slot.append(&state_badge);
    left_slot.append(&recording_indicator.label);
    left_slot.append(&input_level_meter.container);

    let right_slot = GtkBox::new(Orientation::Horizontal, 0);
    right_slot.set_halign(Align::End);
    right_slot.set_hexpand(true);
    right_slot.append(&log_display);

    container.append(&left_slot);
    container.append(&right_slot);

    FooterStatus {
        container,
        state_badge,
        recording_indicator,
        input_level_meter,
        log_display,
    }
}
