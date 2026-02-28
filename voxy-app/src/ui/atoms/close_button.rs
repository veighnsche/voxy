use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("window-close-symbolic");
    button.set_tooltip_text(Some("Hide window"));
    button
}
