use gtk4::{prelude::*, Button};

pub fn build() -> Button {
    let button = Button::from_icon_name("microphone-sensitivity-muted-symbolic");
    button.set_tooltip_text(Some("Toggle microphone (currently off)"));
    button
}

pub fn render(button: &Button, mic_on: bool) {
    let (icon_name, tooltip) = if mic_on {
        (
            "audio-input-microphone-symbolic",
            "Toggle microphone (currently on)",
        )
    } else {
        (
            "microphone-sensitivity-muted-symbolic",
            "Toggle microphone (currently off)",
        )
    };

    button.set_icon_name(icon_name);
    button.set_tooltip_text(Some(tooltip));
}
