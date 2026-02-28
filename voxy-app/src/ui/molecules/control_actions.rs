use gtk4::{prelude::*, Align, Box as GtkBox, Button, ComboBoxText, Orientation, Overlay};

use crate::ui::atoms;

#[derive(Clone)]
pub struct ControlActions {
    pub mic_button: Button,
    pub reset_button: Button,
    pub copy_button: Button,
    pub model_dropdown: ComboBoxText,
    pub close_button: Button,
}

pub fn build() -> (GtkBox, ControlActions) {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_hexpand(true);

    let mic_button = atoms::mic_button::build();
    let reset_button = atoms::reset_button::build();
    let copy_button = atoms::copy_button::build();
    let model_dropdown = atoms::model_dropdown::build();
    let close_button = atoms::close_button::build();
    let logo = atoms::voxy_logo::build();
    logo.set_halign(Align::Center);
    logo.set_valign(Align::Center);
    logo.set_can_target(false);

    let left_slot = GtkBox::new(Orientation::Horizontal, 8);
    left_slot.set_halign(Align::Start);
    left_slot.append(&mic_button);
    left_slot.append(&copy_button);
    left_slot.append(&model_dropdown);

    let right_slot = GtkBox::new(Orientation::Horizontal, 8);
    right_slot.set_halign(Align::End);
    right_slot.append(&reset_button);
    right_slot.append(&close_button);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    let content = GtkBox::new(Orientation::Horizontal, 8);
    content.set_hexpand(true);
    content.append(&left_slot);
    content.append(&spacer);
    content.append(&right_slot);

    let overlay = Overlay::new();
    overlay.set_hexpand(true);
    overlay.set_child(Some(&content));
    overlay.add_overlay(&logo);

    row.append(&overlay);

    (
        row,
        ControlActions {
            mic_button,
            reset_button,
            copy_button,
            model_dropdown,
            close_button,
        },
    )
}
