use std::rc::Rc;

use gtk4::{prelude::*, ApplicationWindow, GestureDrag};
use gtk4_layer_shell::{Edge, LayerShell};
use tokio::sync::mpsc;
use voxy_core::AppEvent;

use super::{hit_test, session::DragSession};

pub fn connect_drag_surface(window: &ApplicationWindow, event_tx: mpsc::Sender<AppEvent>) {
    let drag_gesture = GestureDrag::new();
    drag_gesture.set_button(1);
    drag_gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);

    let window_for_pick = window.clone();
    let window_for_start = window.clone();
    let drag_session = Rc::new(DragSession::default());

    {
        let drag_session = Rc::clone(&drag_session);

        drag_gesture.connect_drag_begin(move |_gesture, start_x, start_y| {
            if !hit_test::should_start_drag(&window_for_pick, start_x, start_y) {
                drag_session.cancel();
                return;
            }

            drag_session.begin(
                window_for_start.margin(Edge::Left),
                window_for_start.margin(Edge::Top),
            );
        });
    }

    {
        let drag_session = Rc::clone(&drag_session);
        let event_tx = event_tx.clone();

        drag_gesture.connect_drag_update(move |_gesture, offset_x, offset_y| {
            if !drag_session.is_active() {
                return;
            }

            let (left, top) = drag_session.position_for(offset_x, offset_y);
            let _ = event_tx.try_send(AppEvent::WindowPositionUpdated { left, top });
        });
    }

    {
        let drag_session = Rc::clone(&drag_session);

        drag_gesture.connect_drag_end(move |_gesture, _offset_x, _offset_y| {
            drag_session.end();
        });
    }

    window.add_controller(drag_gesture);
}
