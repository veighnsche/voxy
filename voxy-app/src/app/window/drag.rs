use gtk4::{gdk, prelude::*, ApplicationWindow, GestureClick, Widget};

pub fn connect_drag_handle(window: &ApplicationWindow, drag_handle: &impl IsA<Widget>) {
    let drag_gesture = GestureClick::new();
    drag_gesture.set_button(1);

    let window = window.clone();
    drag_gesture.connect_pressed(move |gesture, _n_press, x, y| {
        begin_move(&window, gesture, x, y);
    });

    drag_handle.as_ref().add_controller(drag_gesture);
}

fn begin_move(window: &ApplicationWindow, gesture: &GestureClick, x: f64, y: f64) {
    let Some(event) = gesture.current_event() else {
        return;
    };

    let Some(device) = event.device() else {
        return;
    };

    let Some(surface) = window.surface() else {
        return;
    };

    let Ok(toplevel) = surface.downcast::<gdk::Toplevel>() else {
        return;
    };

    toplevel.begin_move(&device, 1, x, y, event.time());
}
