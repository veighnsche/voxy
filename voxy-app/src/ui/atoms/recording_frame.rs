use std::{cell::Cell, rc::Rc};

use gtk4::{prelude::*, DrawingArea};

#[derive(Clone)]
pub struct RecordingFrame {
    pub area: DrawingArea,
    recording: Rc<Cell<bool>>,
}

pub fn build() -> RecordingFrame {
    let area = DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);
    area.set_can_target(false);

    let recording = Rc::new(Cell::new(false));
    let recording_for_draw = Rc::clone(&recording);
    area.set_draw_func(move |_, cr, width, height| {
        if !recording_for_draw.get() {
            return;
        }

        let line_width = 3.0;
        let inset = line_width / 2.0;
        let draw_width = (f64::from(width) - line_width).max(0.0);
        let draw_height = (f64::from(height) - line_width).max(0.0);

        cr.set_source_rgba(0.86, 0.16, 0.16, 0.95);
        cr.set_line_width(line_width);
        cr.rectangle(inset, inset, draw_width, draw_height);
        let _ = cr.stroke();
    });

    RecordingFrame { area, recording }
}

pub fn render(frame: &RecordingFrame, recording: bool) {
    if frame.recording.get() == recording {
        return;
    }

    frame.recording.set(recording);
    frame.area.queue_draw();
}
