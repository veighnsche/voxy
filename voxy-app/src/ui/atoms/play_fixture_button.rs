use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("media-playback-start-symbolic");
    button.set_tooltip_text(Some("Route test_3 fixture into audio input"));
    button
}
