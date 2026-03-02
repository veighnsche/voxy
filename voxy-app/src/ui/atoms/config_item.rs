use gtk4::{prelude::*, Align, Box as GtkBox, Label, Orientation, SpinButton};

#[derive(Clone)]
pub struct ConfigItem {
    pub container: GtkBox,
    pub input: SpinButton,
}

pub fn build_spin(title: &str, hint: &str, input: SpinButton) -> ConfigItem {
    let container = GtkBox::new(Orientation::Vertical, 4);
    container.set_halign(Align::Fill);
    container.set_hexpand(true);
    container.set_valign(Align::Start);

    let title_label = Label::new(Some(title));
    title_label.set_xalign(0.0);

    input.set_halign(Align::Start);

    let hint_label = Label::new(Some(hint));
    hint_label.set_xalign(0.0);
    hint_label.set_wrap(true);
    hint_label.set_max_width_chars(28);
    hint_label.add_css_class("dim-label");

    container.append(&title_label);
    container.append(&input);
    container.append(&hint_label);

    ConfigItem { container, input }
}
