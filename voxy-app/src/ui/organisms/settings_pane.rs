use gtk4::{
    prelude::*, Adjustment, Align, Box as GtkBox, Label, Orientation, ScrolledWindow, SpinButton,
};

const MIN_TIMEOUT_SECONDS: f64 = 0.0;
const MAX_TIMEOUT_SECONDS: f64 = 600.0;

#[derive(Clone)]
pub struct SettingsPane {
    pub container: ScrolledWindow,
    pub silence_timeout_seconds: SpinButton,
}

pub fn build() -> SettingsPane {
    let content = GtkBox::new(Orientation::Vertical, 10);
    content.set_halign(Align::Fill);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_margin_top(4);
    content.set_margin_bottom(4);
    content.set_margin_start(4);
    content.set_margin_end(4);

    let title = Label::new(Some("Settings"));
    title.set_xalign(0.0);
    title.add_css_class("title-4");

    let description = Label::new(Some(
        "This panel replaces the transcript area temporarily.\
\nUse the gear button again to return to transcript view.",
    ));
    description.set_xalign(0.0);
    description.set_wrap(true);

    let section = Label::new(Some(
        "Recording Controls\
\n- Click the IN meter to set the silence threshold\
\n- Countdown appears while under threshold\
\n- Auto-stop triggers when countdown reaches zero",
    ));
    section.set_xalign(0.0);
    section.set_wrap(true);

    let timeout_row = GtkBox::new(Orientation::Horizontal, 8);
    timeout_row.set_halign(Align::Start);

    let timeout_label = Label::new(Some("Silence Timeout (s)"));
    timeout_label.set_xalign(0.0);

    let adjustment = Adjustment::new(
        10.0,
        MIN_TIMEOUT_SECONDS,
        MAX_TIMEOUT_SECONDS,
        1.0,
        5.0,
        0.0,
    );
    let silence_timeout_seconds = SpinButton::new(Some(&adjustment), 1.0, 0);
    silence_timeout_seconds.set_numeric(true);
    silence_timeout_seconds.set_wrap(false);
    silence_timeout_seconds.set_width_chars(4);
    silence_timeout_seconds.set_tooltip_text(Some("0 disables silence auto-stop"));

    let timeout_hint = Label::new(Some("0 = off"));
    timeout_hint.set_xalign(0.0);
    timeout_hint.add_css_class("dim-label");

    timeout_row.append(&timeout_label);
    timeout_row.append(&silence_timeout_seconds);
    timeout_row.append(&timeout_hint);

    content.append(&title);
    content.append(&description);
    content.append(&section);
    content.append(&timeout_row);

    let container = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&content)
        .build();

    SettingsPane {
        container,
        silence_timeout_seconds,
    }
}

pub fn render(pane: &SettingsPane, silence_timeout_seconds: u64) {
    let bounded = silence_timeout_seconds.min(MAX_TIMEOUT_SECONDS as u64) as f64;
    let current = pane.silence_timeout_seconds.value();
    if (current - bounded).abs() > 0.49 {
        pane.silence_timeout_seconds.set_value(bounded);
    }
}
