use gtk4::Label;

pub fn build() -> Label {
    let badge = Label::new(Some("Idle"));
    badge.set_xalign(0.0);
    badge
}

pub fn render(badge: &Label, text: &str) {
    badge.set_text(text);
}
