use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("edit-copy-symbolic");
    button.set_tooltip_text(Some("Copy transcript to clipboard"));
    button
}
