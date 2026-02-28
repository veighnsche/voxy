use gtk4::{pango::EllipsizeMode, prelude::*, Label};

pub fn build() -> Label {
    let label = Label::new(Some("Ready"));
    label.set_xalign(1.0);
    label.set_hexpand(true);
    label.set_ellipsize(EllipsizeMode::End);
    label
}

pub fn render(label: &Label, text: &str) {
    label.set_text(text);
}
