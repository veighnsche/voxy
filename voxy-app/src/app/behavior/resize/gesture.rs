use std::{cell::Cell, rc::Rc};

use gtk4::{prelude::*, ApplicationWindow, GestureDrag, Widget};

pub fn connect_resize_handle(
    window: &ApplicationWindow,
    handle: &Widget,
    on_resize: impl Fn(i32, i32) + 'static,
) {
    let drag_gesture = GestureDrag::new();
    drag_gesture.set_button(1);
    drag_gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);

    let active = Rc::new(Cell::new(false));
    let window_for_update = window.clone();

    {
        let active = Rc::clone(&active);

        drag_gesture.connect_drag_begin(move |_gesture, _start_x, _start_y| {
            active.set(true);
        });
    }

    {
        let active = Rc::clone(&active);

        drag_gesture.connect_drag_update(move |_gesture, offset_x, offset_y| {
            if !active.get() {
                return;
            }

            let current_width = window_for_update
                .width()
                .max(window_for_update.default_width())
                .max(1);
            let current_height = window_for_update
                .height()
                .max(window_for_update.default_height())
                .max(1);
            let next_width = current_width.saturating_add(offset_x.round() as i32).max(1);
            let next_height = current_height
                .saturating_add(offset_y.round() as i32)
                .max(1);
            on_resize(next_width, next_height);
        });
    }

    {
        let active = Rc::clone(&active);
        drag_gesture.connect_drag_end(move |_gesture, _offset_x, _offset_y| {
            active.set(false);
        });
    }

    handle.add_controller(drag_gesture);
}
