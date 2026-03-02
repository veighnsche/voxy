use std::{cell::Cell, rc::Rc, sync::OnceLock, time::Instant};

use gtk4::{prelude::*, ApplicationWindow, GestureClick, GestureDrag};
use gtk4_layer_shell::LayerShell;

use crate::app::behavior::drag::{
    hit_test,
    session::{DragBounds, DragSession},
};

use super::{
    guards::{self, DragMathMode},
    math,
};

pub(crate) fn connect_drag_surface(
    window: &ApplicationWindow,
    on_position: impl Fn(i32, i32) + 'static,
    on_double_click: impl Fn() + 'static,
) {
    let drag_math_mode = guards::detect_drag_math_mode();
    let drag_gesture = GestureDrag::new();
    drag_gesture.set_button(1);
    drag_gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let click_gesture = GestureClick::new();
    click_gesture.set_button(1);
    click_gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);

    let window_for_pick = window.clone();
    let window_for_start = window.clone();
    let window_for_update = window.clone();
    let window_for_click = window.clone();
    let drag_session = Rc::new(DragSession::default());
    let trace_state = Rc::new(DragTraceState::default());

    {
        let drag_session = Rc::clone(&drag_session);
        let trace_state = Rc::clone(&trace_state);
        let drag_math_mode = drag_math_mode;

        drag_gesture.connect_drag_begin(move |_gesture, start_x, start_y| {
            if !hit_test::should_start_drag(&window_for_pick, start_x, start_y) {
                trace_drag(|| Some(format!("b! s={start_x:.1},{start_y:.1} reason=interactive")));
                drag_session.cancel();
                return;
            }

            let base_left = window_for_start.margin(gtk4_layer_shell::Edge::Left);
            let base_top = window_for_start.margin(gtk4_layer_shell::Edge::Top);
            drag_session.begin(base_left, base_top);
            let start_abs_x = (base_left as f64) + start_x;
            let start_abs_y = (base_top as f64) + start_y;
            trace_state.begin(base_left, base_top, start_abs_x, start_abs_y);
            trace_drag(|| {
                let scale_factor = math::current_scale_factor(&window_for_start);
                Some(format!(
                    "b s={start_x:.1},{start_y:.1} b={base_left},{base_top} sf={scale_factor} mode={} a0={start_abs_x:.1},{start_abs_y:.1}",
                    drag_math_mode.label()
                ))
            });
        });
    }

    {
        let drag_session = Rc::clone(&drag_session);
        let trace_state = Rc::clone(&trace_state);
        let drag_math_mode = drag_math_mode;

        drag_gesture.connect_drag_update(move |_gesture, offset_x, offset_y| {
            if !drag_session.is_active() {
                return;
            }

            let bounds = math::current_drag_bounds(&window_for_update);
            let next_position = match drag_math_mode {
                DragMathMode::LegacyIncremental => {
                    let current_left = window_for_update.margin(gtk4_layer_shell::Edge::Left);
                    let current_top = window_for_update.margin(gtk4_layer_shell::Edge::Top);
                    drag_session.position_for_incremental(
                        current_left,
                        current_top,
                        offset_x,
                        offset_y,
                        bounds,
                    )
                }
                DragMathMode::PointerAnchor => math::pointer_abs(&window_for_update)
                    .and_then(|(pointer_abs_x, pointer_abs_y)| {
                        let (anchor_x, anchor_y) = trace_state.pointer_anchor();
                        let raw_left = pointer_abs_x - anchor_x;
                        let raw_top = pointer_abs_y - anchor_y;
                        drag_session.position_for_raw(raw_left, raw_top, bounds)
                    })
                    .or_else(|| drag_session.position_for_offset(offset_x, offset_y, bounds)),
            };

            if let Some((left, top)) = next_position {
                trace_update(
                    &trace_state,
                    &window_for_update,
                    offset_x,
                    offset_y,
                    bounds,
                    left,
                    top,
                );
                on_position(left, top);
            }
        });
    }

    {
        let drag_session = Rc::clone(&drag_session);
        let trace_state = Rc::clone(&trace_state);

        drag_gesture.connect_drag_end(move |_gesture, _offset_x, _offset_y| {
            trace_drag(|| {
                Some(format!(
                    "e n={} dt={}ms",
                    trace_state.sequence(),
                    trace_state.elapsed_ms()
                ))
            });
            trace_state.reset();
            drag_session.end();
        });
    }

    click_gesture.connect_pressed(move |_gesture, n_press, x, y| {
        if n_press != 2 {
            return;
        }
        if !hit_test::should_start_drag(&window_for_click, x, y) {
            return;
        }
        on_double_click();
    });

    window.add_controller(drag_gesture);
    window.add_controller(click_gesture);
}

#[derive(Default)]
struct DragTraceState {
    sequence: Cell<u32>,
    started_at: Cell<Option<Instant>>,
    base_left: Cell<i32>,
    base_top: Cell<i32>,
    start_abs_x: Cell<f64>,
    start_abs_y: Cell<f64>,
}

impl DragTraceState {
    fn begin(&self, base_left: i32, base_top: i32, start_abs_x: f64, start_abs_y: f64) {
        self.sequence.set(0);
        self.started_at.set(Some(Instant::now()));
        self.base_left.set(base_left);
        self.base_top.set(base_top);
        self.start_abs_x.set(start_abs_x);
        self.start_abs_y.set(start_abs_y);
    }

    fn reset(&self) {
        self.sequence.set(0);
        self.started_at.set(None);
    }

    fn bump_sequence(&self) -> u32 {
        let next = self.sequence.get().saturating_add(1);
        self.sequence.set(next);
        next
    }

    fn sequence(&self) -> u32 {
        self.sequence.get()
    }

    fn elapsed_ms(&self) -> u128 {
        self.started_at
            .get()
            .map(|start| start.elapsed().as_millis())
            .unwrap_or(0)
    }

    fn pointer_anchor(&self) -> (f64, f64) {
        (
            self.start_abs_x.get() - (self.base_left.get() as f64),
            self.start_abs_y.get() - (self.base_top.get() as f64),
        )
    }
}

fn trace_update(
    trace_state: &DragTraceState,
    window: &ApplicationWindow,
    offset_x: f64,
    offset_y: f64,
    bounds: DragBounds,
    left: i32,
    top: i32,
) {
    trace_drag(|| {
        let seq = trace_state.bump_sequence();
        let every = drag_trace_every().max(1);
        if seq % every != 0 {
            return None;
        }

        let elapsed = trace_state.elapsed_ms();
        let scale_factor = math::current_scale_factor(window);
        let (abs_x, abs_y, drift_x, drift_y) = match math::pointer_abs(window) {
            Some((pointer_abs_x, pointer_abs_y)) => {
                let pointer_dx = pointer_abs_x - trace_state.start_abs_x.get();
                let pointer_dy = pointer_abs_y - trace_state.start_abs_y.get();
                let window_dx = (left - trace_state.base_left.get()) as f64;
                let window_dy = (top - trace_state.base_top.get()) as f64;
                (
                    format!("{pointer_abs_x:.1}"),
                    format!("{pointer_abs_y:.1}"),
                    format!("{:.1}", pointer_dx - window_dx),
                    format!("{:.1}", pointer_dy - window_dy),
                )
            }
            None => (
                "na".to_owned(),
                "na".to_owned(),
                "na".to_owned(),
                "na".to_owned(),
            ),
        };

        Some(format!(
            "u#{seq} t={elapsed}ms o={offset_x:.1},{offset_y:.1} p={left},{top} sf={scale_factor} b={},{} m={},{} a={abs_x},{abs_y} d={drift_x},{drift_y}",
            bounds.max_left,
            bounds.max_top,
            window.width(),
            window.height()
        ))
    });
}

fn trace_drag(build_message: impl FnOnce() -> Option<String>) {
    if !drag_trace_enabled() {
        return;
    }

    if let Some(message) = build_message() {
        eprintln!("[voxy:drag] {message}");
    }
}

fn drag_trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();

    *ENABLED.get_or_init(|| {
        std::env::var("VOXY_DRAG_TRACE")
            .ok()
            .map(|value| {
                let value = value.trim().to_ascii_lowercase();
                matches!(value.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false)
    })
}

fn drag_trace_every() -> u32 {
    static EVERY: OnceLock<u32> = OnceLock::new();

    *EVERY.get_or_init(|| {
        std::env::var("VOXY_DRAG_TRACE_EVERY")
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(8)
    })
}
