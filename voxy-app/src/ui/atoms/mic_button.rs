use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("media-record-symbolic");
    button.set_tooltip_text(Some("Start recording"));
    button
}

pub fn render(button: &Button, mic_on: bool) {
    let (icon_name, tooltip) = if mic_on {
        ("media-playback-stop-symbolic", "Stop recording")
    } else {
        ("media-record-symbolic", "Start recording")
    };

    button.set_icon_name(icon_name);
    button.set_tooltip_text(Some(tooltip));
    if mic_on {
        button.remove_css_class("destructive-action");
    } else {
        button.add_css_class("destructive-action");
    }
}
