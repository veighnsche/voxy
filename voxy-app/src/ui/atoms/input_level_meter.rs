use std::cell::Cell;

use gtk4::{prelude::*, Align, Box as GtkBox, Label, LevelBar, LevelBarMode, Orientation};

const MIN_LEVEL: f32 = 0.0;
const MAX_LEVEL: f32 = 1.0;
const LOW_THRESHOLD: f32 = 0.60;
const HIGH_THRESHOLD: f32 = 0.85;
const DISCRETE_BLOCKS: u32 = 24;
const PEAK_DECAY_PER_RENDER: f32 = 0.02;

#[derive(Clone)]
pub struct InputLevelMeter {
    pub container: GtkBox,
    bar: LevelBar,
    level_db_label: Label,
    peak_db_label: Label,
    peak_hold: Cell<f32>,
}

pub fn build() -> InputLevelMeter {
    let container = GtkBox::new(Orientation::Horizontal, 8);
    container.set_halign(Align::Start);
    container.set_hexpand(false);

    let title = Label::new(Some("IN"));
    title.set_xalign(0.0);
    title.set_width_chars(2);

    let bar = LevelBar::new();
    bar.set_mode(LevelBarMode::Discrete);
    bar.set_min_value(MIN_LEVEL as f64);
    bar.set_max_value(MAX_LEVEL as f64);
    bar.set_value(MIN_LEVEL as f64);
    bar.set_size_request(144, 12);
    bar.set_hexpand(false);
    bar.set_sensitive(false);
    bar.set_inverted(false);
    bar.add_offset_value("low", LOW_THRESHOLD as f64);
    bar.add_offset_value("high", HIGH_THRESHOLD as f64);
    bar.set_tooltip_text(Some("Input level (green/yellow/red)"));

    // Force a stable number of discrete ticks.
    bar.set_min_value(0.0);
    bar.set_max_value(DISCRETE_BLOCKS as f64);
    bar.set_value(0.0);
    bar.add_offset_value("low", (DISCRETE_BLOCKS as f32 * LOW_THRESHOLD) as f64);
    bar.add_offset_value("high", (DISCRETE_BLOCKS as f32 * HIGH_THRESHOLD) as f64);

    let level_db_label = Label::new(Some("-inf dB"));
    level_db_label.set_xalign(1.0);
    level_db_label.set_width_chars(8);

    let peak_db_label = Label::new(Some("pk -inf"));
    peak_db_label.set_xalign(1.0);
    peak_db_label.set_width_chars(8);

    container.append(&title);
    container.append(&bar);
    container.append(&level_db_label);
    container.append(&peak_db_label);

    InputLevelMeter {
        container,
        bar,
        level_db_label,
        peak_db_label,
        peak_hold: Cell::new(MIN_LEVEL),
    }
}

pub fn render(meter: &InputLevelMeter, normalized_level: f32, active: bool) {
    if !active {
        meter.bar.set_value(0.0);
        meter.level_db_label.set_text("-inf dB");
        meter.peak_db_label.set_text("pk -inf");
        meter.peak_hold.set(MIN_LEVEL);
        return;
    }

    let level = normalized_level.clamp(MIN_LEVEL, MAX_LEVEL);
    let peak = meter.peak_hold.get();
    let next_peak = if level >= peak {
        level
    } else {
        (peak - PEAK_DECAY_PER_RENDER).max(level)
    };

    meter.peak_hold.set(next_peak);
    meter.bar.set_value((level * DISCRETE_BLOCKS as f32) as f64);
    meter
        .level_db_label
        .set_text(&format!("{} dB", db_string(level)));
    meter
        .peak_db_label
        .set_text(&format!("pk {}", db_string(next_peak)));
}

fn db_string(normalized_level: f32) -> String {
    if normalized_level <= 0.000_01 {
        return "-inf".to_owned();
    }
    format!("{:.1}", 20.0 * normalized_level.log10())
}
