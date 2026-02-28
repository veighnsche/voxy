use gtk4::{Label, Revealer};

#[derive(Clone)]
pub struct ErrorBanner {
    pub revealer: Revealer,
    pub message: Label,
}

pub fn build() -> ErrorBanner {
    let revealer = Revealer::new();
    revealer.set_reveal_child(false);

    let message = Label::new(Some(""));
    message.set_xalign(0.0);
    revealer.set_child(Some(&message));

    ErrorBanner { revealer, message }
}

pub fn render(banner: &ErrorBanner, maybe_message: Option<&str>) {
    match maybe_message {
        Some(message) => {
            banner.message.set_text(message);
            banner.revealer.set_reveal_child(true);
        }
        None => {
            banner.message.set_text("");
            banner.revealer.set_reveal_child(false);
        }
    }
}
