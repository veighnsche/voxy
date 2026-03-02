use gtk4::{prelude::*, Align, Box as GtkBox, Button, Label, Orientation, Revealer};

#[derive(Clone)]
pub struct ErrorBanner {
    pub revealer: Revealer,
    pub message: Label,
    pub copy_button: Button,
    pub dismiss_button: Button,
}

pub fn build() -> ErrorBanner {
    let revealer = Revealer::new();
    revealer.set_reveal_child(false);

    let message = Label::new(Some(""));
    message.set_xalign(0.0);
    message.set_wrap(true);
    message.set_hexpand(true);
    message.set_halign(Align::Fill);

    let actions = GtkBox::new(Orientation::Horizontal, 6);
    actions.set_halign(Align::End);
    actions.set_valign(Align::Start);

    let copy_button = Button::with_label("Copy Error");
    copy_button.set_tooltip_text(Some("Copy full error diagnostics to clipboard"));
    let dismiss_button = Button::with_label("Dismiss");
    dismiss_button.set_tooltip_text(Some("Hide this error"));

    actions.append(&copy_button);
    actions.append(&dismiss_button);

    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_halign(Align::Fill);
    row.set_hexpand(true);
    row.append(&message);
    row.append(&actions);

    revealer.set_child(Some(&row));

    ErrorBanner {
        revealer,
        message,
        copy_button,
        dismiss_button,
    }
}

pub fn render(banner: &ErrorBanner, maybe_message: Option<&str>) {
    match maybe_message {
        Some(message) => {
            banner.message.set_text(message);
            banner.message.set_tooltip_text(Some(message));
            banner.revealer.set_reveal_child(true);
        }
        None => {
            banner.message.set_text("");
            banner.message.set_tooltip_text(None);
            banner.revealer.set_reveal_child(false);
        }
    }
}
