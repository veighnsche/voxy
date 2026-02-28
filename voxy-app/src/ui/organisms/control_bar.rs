use gtk4::Box as GtkBox;

use crate::ui::molecules::{self, control_actions::ControlActions};

#[derive(Clone)]
pub struct ControlBar {
    pub container: GtkBox,
    pub actions: ControlActions,
}

pub fn build() -> ControlBar {
    let (container, actions) = molecules::control_actions::build();
    ControlBar { container, actions }
}
