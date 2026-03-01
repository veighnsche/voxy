use std::{cell::Cell, rc::Rc};

use gtk4::{
    prelude::*, Align, Box as GtkBox, DrawingArea, GestureClick, Label, LevelBar, LevelBarMode,
    Orientation, Overlay,
};

const MIN_LEVEL: f32 = 0.0;
const MAX_LEVEL: f32 = 1.0;
const DEFAULT_GATE_THRESHOLD: f32 = 0.30;
const GATE_HIGH_OFFSET_DELTA: f32 = 0.15;

#[derive(Clone)]
pub struct InputLevelMeter {
    pub container: GtkBox,
    bar: LevelBar,
    gate_threshold: Rc<Cell<f32>>,
    countdown_label: Label,
}

pub fn build() -> InputLevelMeter {
    let container = GtkBox::new(Orientation::Horizontal, 4);
    container.set_halign(Align::Start);
    container.set_hexpand(false);

    let title = Label::new(Some("IN"));
    title.set_xalign(0.0);
    title.set_width_chars(2);

    let gate_threshold = Rc::new(Cell::new(DEFAULT_GATE_THRESHOLD));

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
    set_gate_offsets(&bar, DEFAULT_GATE_THRESHOLD);
    bar.set_tooltip_text(Some(
        "Input level (green/yellow/red). Click the meter to set gate threshold.",
    ));

    let threshold_line = DrawingArea::new();
    threshold_line.set_hexpand(true);
    threshold_line.set_vexpand(true);
    threshold_line.set_can_target(false);
    {
        let gate_threshold = Rc::clone(&gate_threshold);
        threshold_line.set_draw_func(move |_, cr, width, height| {
            let width = f64::from(width).max(1.0);
            let line_x =
                f64::from(gate_threshold.get().clamp(MIN_LEVEL, MAX_LEVEL)) * (width - 1.0);
            cr.set_source_rgba(0.05, 0.05, 0.05, 0.95);
            cr.set_line_width(1.0);
            cr.move_to(line_x + 0.5, 0.0);
            cr.line_to(line_x + 0.5, f64::from(height));
            let _ = cr.stroke();
        });
    }

    let meter_surface = Overlay::new();
    meter_surface.set_hexpand(false);
    meter_surface.set_vexpand(false);
    meter_surface.set_size_request(56, 8);
    meter_surface.set_child(Some(&bar));
    meter_surface.add_overlay(&threshold_line);

    let click = GestureClick::new();
    {
        let bar = bar.clone();
        let gate_threshold = Rc::clone(&gate_threshold);
        let threshold_line = threshold_line.clone();
        let meter_surface = meter_surface.clone();
        click.connect_pressed(move |_, _, x, _| {
            let width = meter_surface.allocated_width().max(1) as f32;
            let next_threshold = (x as f32 / width).clamp(MIN_LEVEL, MAX_LEVEL);
            gate_threshold.set(next_threshold);
            set_gate_offsets(&bar, next_threshold);
            threshold_line.queue_draw();
        });
    }
    meter_surface.add_controller(click);

    let countdown_label = Label::new(None);
    countdown_label.set_xalign(0.0);
    countdown_label.set_width_chars(5);
    countdown_label.set_markup("<span foreground=\"#6b7280\">--</span>");
    countdown_label.set_tooltip_text(Some("Silence auto-stop countdown"));

    container.append(&title);
    container.append(&meter_surface);
    container.append(&countdown_label);

    InputLevelMeter {
        container,
        bar,
        gate_threshold,
        countdown_label,
    }
}

pub fn render(
    meter: &InputLevelMeter,
    normalized_level: f32,
    active: bool,
    silence_seconds_remaining: Option<u64>,
) {
    if !active {
        meter.bar.set_value(0.0);
        meter
            .countdown_label
            .set_markup("<span foreground=\"#6b7280\">--</span>");
        return;
    }

    let visual_level = visual_level(normalized_level);
    meter.bar.set_value(visual_level as f64);

    if let Some(seconds_remaining) = silence_seconds_remaining {
        meter.countdown_label.set_markup(&format!(
            "<span foreground=\"#b45309\" weight=\"bold\">{:>2}s</span>",
            seconds_remaining.min(99)
        ));
    } else {
        meter
            .countdown_label
            .set_markup("<span foreground=\"#6b7280\">--</span>");
    }
}

pub fn gate_threshold(meter: &InputLevelMeter) -> f32 {
    meter.gate_threshold.get().clamp(MIN_LEVEL, MAX_LEVEL)
}

pub fn visual_level(raw_level: f32) -> f32 {
    let level = raw_level.clamp(MIN_LEVEL, MAX_LEVEL);
    if level <= 0.000_01 {
        return 0.0;
    }

    // Map linear PCM peak into a dB range so normal speech isn't stuck on block 1.
    let db = 20.0 * level.log10();
    let normalized_db = ((db + 54.0) / 54.0).clamp(0.0, 1.0);
    normalized_db.powf(0.8)
}

fn set_gate_offsets(bar: &LevelBar, threshold: f32) {
    let low = threshold.clamp(MIN_LEVEL, MAX_LEVEL);
    let high = (low + GATE_HIGH_OFFSET_DELTA).clamp(MIN_LEVEL, MAX_LEVEL);
    bar.add_offset_value("low", low as f64);
    bar.add_offset_value("high", high as f64);
}
