use gtk4::{prelude::*, Box as GtkBox, Label, Orientation};

use crate::ui::{atoms, molecules};

#[derive(Clone)]
pub struct FooterStatus {
    pub container: GtkBox,
    pub state_badge: Label,
    pub recording_indicator: molecules::recording_indicator::RecordingIndicator,
}

pub fn build() -> FooterStatus {
    let container = GtkBox::new(Orientation::Horizontal, 8);

    let state_badge = atoms::state_badge::build();
    let recording_indicator = molecules::recording_indicator::build();

    container.append(&state_badge);
    container.append(&recording_indicator.label);

    FooterStatus {
        container,
        state_badge,
        recording_indicator,
    }
}
