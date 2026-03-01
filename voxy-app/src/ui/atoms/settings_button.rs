use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("preferences-system-symbolic");
    button.set_tooltip_text(Some("Toggle settings"));
    button
}

pub fn render(button: &Button, active: bool) {
    if active {
        button.add_css_class("suggested-action");
    } else {
        button.remove_css_class("suggested-action");
    }
}
