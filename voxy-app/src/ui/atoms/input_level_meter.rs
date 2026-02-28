use gtk4::{prelude::*, Align, Box as GtkBox, Label, LevelBar, LevelBarMode, Orientation};

const MIN_LEVEL: f32 = 0.0;
const MAX_LEVEL: f32 = 1.0;
const LOW_THRESHOLD: f32 = 0.75;
const HIGH_THRESHOLD: f32 = 0.90;

#[derive(Clone)]
pub struct InputLevelMeter {
    pub container: GtkBox,
    bar: LevelBar,
}

pub fn build() -> InputLevelMeter {
    let container = GtkBox::new(Orientation::Horizontal, 4);
    container.set_halign(Align::Start);
    container.set_hexpand(false);

    let title = Label::new(Some("IN"));
    title.set_xalign(0.0);
    title.set_width_chars(2);

    let bar = LevelBar::new();
    // Keep the meter as a single widget instead of many discrete blocks.
    bar.set_mode(LevelBarMode::Continuous);
    bar.set_min_value(MIN_LEVEL as f64);
    bar.set_max_value(MAX_LEVEL as f64);
    bar.set_value(MIN_LEVEL as f64);
    bar.set_size_request(56, 8);
    bar.set_hexpand(false);
    bar.set_halign(Align::Start);
    bar.set_sensitive(true);
    bar.set_inverted(false);
    bar.add_offset_value("low", LOW_THRESHOLD as f64);
    bar.add_offset_value("high", HIGH_THRESHOLD as f64);
    bar.set_tooltip_text(Some("Input level (green/yellow/red)"));

    container.append(&title);
    container.append(&bar);

    InputLevelMeter { container, bar }
}

pub fn render(meter: &InputLevelMeter, normalized_level: f32, active: bool) {
    if !active {
        meter.bar.set_value(0.0);
        return;
    }

    let visual_level = visual_level_from_raw(normalized_level);
    meter.bar.set_value(visual_level as f64);
}

fn visual_level_from_raw(raw_level: f32) -> f32 {
    let level = raw_level.clamp(MIN_LEVEL, MAX_LEVEL);
    if level <= 0.000_01 {
        return 0.0;
    }

    // Map linear PCM peak into a dB range so normal speech isn't stuck on block 1.
    let db = 20.0 * level.log10();
    let normalized_db = ((db + 54.0) / 54.0).clamp(0.0, 1.0);
    normalized_db.powf(0.8)
}
