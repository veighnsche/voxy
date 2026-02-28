use gtk4::{prelude::*, Box as GtkBox, Orientation};

use crate::ui::molecules::{self, control_actions::ControlActions};

#[derive(Clone)]
pub struct ControlBar {
    pub container: GtkBox,
    pub actions: ControlActions,
}

pub fn build() -> ControlBar {
    let container = GtkBox::new(Orientation::Vertical, 0);
    let drag_surface = GtkBox::new(Orientation::Horizontal, 0);
    drag_surface.set_height_request(4);
    drag_surface.set_hexpand(true);

    let (row, actions) = molecules::control_actions::build();
    container.append(&drag_surface);
    container.append(&row);

    ControlBar { container, actions }
}
