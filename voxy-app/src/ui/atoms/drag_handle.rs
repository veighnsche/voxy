use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("transform-move-symbolic");
    button.set_tooltip_text(Some("Drag widget"));
    button
}
