use gtk4::{prelude::*, Box as GtkBox, Button, Orientation};

use crate::ui::atoms;

#[derive(Clone)]
pub struct ControlActions {
    pub drag_handle: Button,
    pub mic_button: Button,
    pub reset_button: Button,
    pub copy_button: Button,
}

pub fn build() -> (GtkBox, ControlActions) {
    let row = GtkBox::new(Orientation::Horizontal, 8);

    let drag_handle = atoms::drag_handle::build();
    let mic_button = atoms::mic_button::build();
    let reset_button = atoms::reset_button::build();
    let copy_button = atoms::copy_button::build();

    row.append(&drag_handle);
    row.append(&mic_button);
    row.append(&reset_button);
    row.append(&copy_button);

    (
        row,
        ControlActions {
            drag_handle,
            mic_button,
            reset_button,
            copy_button,
        },
    )
}
