use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("edit-clear-all-symbolic");
    button.set_tooltip_text(Some("Reset transcript"));
    button
}
