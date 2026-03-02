use gtk4::{
    prelude::*, Adjustment, Align, Box as GtkBox, Label, Orientation, ScrolledWindow, SpinButton,
};

use crate::ui::atoms::config_item;

const MIN_TIMEOUT_SECONDS: f64 = 0.0;
const MAX_TIMEOUT_SECONDS: f64 = 600.0;
const MIN_VAD_SILENCE_MS: f64 = 100.0;
const MAX_VAD_SILENCE_MS: f64 = 5_000.0;

#[derive(Clone)]
pub struct SettingsPane {
    pub container: ScrolledWindow,
    pub silence_timeout_seconds: SpinButton,
    pub vad_silence_ms: SpinButton,
}

pub fn build() -> SettingsPane {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_halign(Align::Fill);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_margin_top(2);
    content.set_margin_bottom(2);
    content.set_margin_start(2);
    content.set_margin_end(2);

    let (recording_section, silence_timeout_seconds, vad_silence_ms) = build_recording_section();
    let api_key_section = build_api_key_section();

    let columns = GtkBox::new(Orientation::Horizontal, 16);
    columns.set_halign(Align::Fill);
    columns.set_hexpand(true);
    columns.set_valign(Align::Start);
    columns.set_homogeneous(true);
    columns.append(&recording_section);
    columns.append(&api_key_section);

    content.append(&columns);

    let container = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&content)
        .build();

    SettingsPane {
        container,
        silence_timeout_seconds,
        vad_silence_ms,
    }
}

pub fn render(pane: &SettingsPane, silence_timeout_seconds: u64, vad_silence_ms: u32) {
    let bounded = silence_timeout_seconds.min(MAX_TIMEOUT_SECONDS as u64) as f64;
    let current = pane.silence_timeout_seconds.value();
    if (current - bounded).abs() > 0.49 {
        pane.silence_timeout_seconds.set_value(bounded);
    }

    let bounded_vad = vad_silence_ms.clamp(100, 5_000) as f64;
    let current_vad = pane.vad_silence_ms.value();
    if (current_vad - bounded_vad).abs() > 0.49 {
        pane.vad_silence_ms.set_value(bounded_vad);
    }
}

fn build_recording_section() -> (GtkBox, SpinButton, SpinButton) {
    let section = GtkBox::new(Orientation::Vertical, 10);
    section.set_halign(Align::Fill);
    section.set_hexpand(true);
    section.set_valign(Align::Start);

    let title = Label::new(Some("Recording"));
    title.set_xalign(0.0);
    title.add_css_class("dim-label");

    let timeout_adjustment = Adjustment::new(
        10.0,
        MIN_TIMEOUT_SECONDS,
        MAX_TIMEOUT_SECONDS,
        1.0,
        5.0,
        0.0,
    );
    let silence_timeout_seconds = SpinButton::new(Some(&timeout_adjustment), 1.0, 0);
    silence_timeout_seconds.set_numeric(true);
    silence_timeout_seconds.set_wrap(false);
    silence_timeout_seconds.set_width_chars(4);
    silence_timeout_seconds.set_tooltip_text(Some("0 disables silence auto-stop"));
    let timeout_item = config_item::build_spin(
        "Silence timeout (s)",
        "0 = off",
        silence_timeout_seconds.clone(),
    );

    let vad_adjustment = Adjustment::new(
        1600.0,
        MIN_VAD_SILENCE_MS,
        MAX_VAD_SILENCE_MS,
        50.0,
        250.0,
        0.0,
    );
    let vad_silence_ms = SpinButton::new(Some(&vad_adjustment), 1.0, 0);
    vad_silence_ms.set_numeric(true);
    vad_silence_ms.set_wrap(false);
    vad_silence_ms.set_width_chars(5);
    vad_silence_ms.set_tooltip_text(Some("How long silence must last before auto-commit"));
    let vad_item = config_item::build_spin(
        "VAD pause (ms)",
        "Higher = fewer split sentences",
        vad_silence_ms.clone(),
    );

    let hint = Label::new(Some("Set silence threshold by clicking the IN meter."));
    hint.set_xalign(0.0);
    hint.set_wrap(true);
    hint.add_css_class("dim-label");

    section.append(&title);
    section.append(&timeout_item.container);
    section.append(&vad_item.container);
    section.append(&hint);

    (section, timeout_item.input, vad_item.input)
}

fn build_api_key_section() -> GtkBox {
    let section = GtkBox::new(Orientation::Vertical, 6);
    section.set_halign(Align::Fill);
    section.set_hexpand(true);
    section.set_valign(Align::Start);

    let title = Label::new(Some("OpenAI API Key"));
    title.set_xalign(0.0);
    title.add_css_class("dim-label");

    let body = Label::new(Some(
        "Use environment variables (recommended):\n\
VOXY_OPENAI_API_KEY=...\n\
or VOXY_OPENAI_API_KEY_FILE=/path/to/key.txt\n\
or OPENAI_API_KEY=...\n\
\n\
To persist across restarts, add one of these to your shell profile.\n\
You can also put the same keys in .env or .env.local.",
    ));
    body.set_xalign(0.0);
    body.set_wrap(true);
    body.set_selectable(true);
    body.add_css_class("dim-label");

    section.append(&title);
    section.append(&body);
    section
}
