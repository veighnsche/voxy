use gtk4::{prelude::*, Label};

pub fn build() -> Label {
    let label = Label::new(Some("VOXY"));
    label.add_css_class("heading");
    label.set_tooltip_text(Some("Voxy"));
    label
}
