use gtk4::{pango::EllipsizeMode, prelude::*, Label};

pub fn build() -> Label {
    let badge = Label::new(Some("Idle"));
    badge.set_xalign(0.0);
    badge.set_hexpand(false);
    badge.set_width_chars(10);
    badge.set_max_width_chars(10);
    badge.set_size_request(90, -1);
    badge.set_single_line_mode(true);
    badge.set_ellipsize(EllipsizeMode::End);
    badge
}

pub fn render(badge: &Label, text: &str) {
    badge.set_text(text);
}
