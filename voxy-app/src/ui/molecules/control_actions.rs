use gtk4::{prelude::*, Align, Box as GtkBox, Button, Orientation};

use crate::ui::atoms;

#[derive(Clone)]
pub struct ControlActions {
    pub mic_button: Button,
    pub reset_button: Button,
    pub copy_button: Button,
    pub close_button: Button,
}

pub fn build() -> (GtkBox, ControlActions) {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_homogeneous(true);

    let mic_button = atoms::mic_button::build();
    let reset_button = atoms::reset_button::build();
    let copy_button = atoms::copy_button::build();
    let close_button = atoms::close_button::build();
    let logo = atoms::voxy_logo::build();

    let left_slot = GtkBox::new(Orientation::Horizontal, 8);
    left_slot.set_halign(Align::Start);
    left_slot.append(&mic_button);
    left_slot.append(&reset_button);
    left_slot.append(&copy_button);

    let center_slot = GtkBox::new(Orientation::Horizontal, 0);
    center_slot.set_halign(Align::Center);
    center_slot.append(&logo);

    let right_slot = GtkBox::new(Orientation::Horizontal, 8);
    right_slot.set_halign(Align::End);
    right_slot.append(&close_button);

    row.append(&left_slot);
    row.append(&center_slot);
    row.append(&right_slot);

    (
        row,
        ControlActions {
            mic_button,
            reset_button,
            copy_button,
            close_button,
        },
    )
}
