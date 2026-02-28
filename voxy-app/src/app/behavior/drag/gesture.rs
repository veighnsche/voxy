use std::{cell::Cell, rc::Rc, sync::OnceLock, time::Instant};

use gtk4::{prelude::*, ApplicationWindow, GestureDrag};
use gtk4_layer_shell::{Edge, LayerShell};

use super::{
    hit_test,
    session::{DragBounds, DragSession},
};

pub fn connect_drag_surface(window: &ApplicationWindow, on_position: impl Fn(i32, i32) + 'static) {
    let drag_gesture = GestureDrag::new();
    drag_gesture.set_button(1);
    drag_gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);

    let window_for_pick = window.clone();
    let window_for_start = window.clone();
    let window_for_update = window.clone();
    let drag_session = Rc::new(DragSession::default());
    let trace_state = Rc::new(DragTraceState::default());

    {
        let drag_session = Rc::clone(&drag_session);
        let trace_state = Rc::clone(&trace_state);

        drag_gesture.connect_drag_begin(move |_gesture, start_x, start_y| {
            if !hit_test::should_start_drag(&window_for_pick, start_x, start_y) {
                trace_drag(format!(
                    "b! s={start_x:.1},{start_y:.1} reason=interactive"
                ));
                drag_session.cancel();
                return;
            }

            let base_left = window_for_start.margin(Edge::Left);
            let base_top = window_for_start.margin(Edge::Top);
            let scale_factor = current_scale_factor(&window_for_start);
            drag_session.begin();
            let start_abs_x = (base_left as f64) + start_x;
            let start_abs_y = (base_top as f64) + start_y;
            trace_state.begin(base_left, base_top, start_abs_x, start_abs_y);
            trace_drag(format!(
                "b s={start_x:.1},{start_y:.1} b={base_left},{base_top} sf={scale_factor} a0={start_abs_x:.1},{start_abs_y:.1}"
            ));
        });
    }

    {
        let drag_session = Rc::clone(&drag_session);
        let trace_state = Rc::clone(&trace_state);

        drag_gesture.connect_drag_update(move |_gesture, offset_x, offset_y| {
            if !drag_session.is_active() {
                return;
            }

            let scale_factor = current_scale_factor(&window_for_update);
            let bounds = current_drag_bounds(&window_for_update);
            let current_left = window_for_update.margin(Edge::Left);
            let current_top = window_for_update.margin(Edge::Top);
            if let Some((left, top)) =
                drag_session.position_for(current_left, current_top, offset_x, offset_y, bounds)
            {
                trace_update(
                    &trace_state,
                    &window_for_update,
                    offset_x,
                    offset_y,
                    scale_factor,
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
            trace_drag(format!(
                "e n={} dt={}ms",
                trace_state.sequence(),
                trace_state.elapsed_ms()
            ));
            trace_state.reset();
            drag_session.end();
        });
    }

    window.add_controller(drag_gesture);
}

fn current_scale_factor(window: &ApplicationWindow) -> i32 {
    window
        .surface()
        .map(|surface| surface.scale_factor())
        .unwrap_or_else(|| window.scale_factor())
        .max(1)
}

fn current_drag_bounds(window: &ApplicationWindow) -> DragBounds {
    let (monitor_width, monitor_height) = window
        .surface()
        .and_then(|surface| surface.display().monitor_at_surface(&surface))
        .map(|monitor| {
            let geometry = monitor.geometry();
            (geometry.width(), geometry.height())
        })
        .unwrap_or_else(|| fallback_monitor_size(window));

    let window_width = window.width().max(window.default_width()).max(1);
    let window_height = window.height().max(window.default_height()).max(1);
    let max_left = (monitor_width - window_width).max(0);
    let max_top = (monitor_height - window_height).max(0);

    DragBounds::from_extents(max_left, max_top)
}

fn fallback_monitor_size(window: &ApplicationWindow) -> (i32, i32) {
    let width = window.width().max(window.default_width()).max(1);
    let height = window.height().max(window.default_height()).max(1);
    (width, height)
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
}

fn trace_update(
    trace_state: &DragTraceState,
    window: &ApplicationWindow,
    offset_x: f64,
    offset_y: f64,
    scale_factor: i32,
    bounds: DragBounds,
    left: i32,
    top: i32,
) {
    if !drag_trace_enabled() {
        return;
    }

    let seq = trace_state.bump_sequence();
    let every = drag_trace_every().max(1);
    if seq % every != 0 {
        return;
    }

    let elapsed = trace_state.elapsed_ms();
    let (abs_x, abs_y, drift_x, drift_y) = match pointer_abs(window) {
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

    trace_drag(format!(
        "u#{seq} t={elapsed}ms o={offset_x:.1},{offset_y:.1} p={left},{top} sf={scale_factor} b={},{} m={},{} a={abs_x},{abs_y} d={drift_x},{drift_y}",
        bounds.max_left, bounds.max_top, window.width(), window.height()
    ));
}

fn pointer_abs(window: &ApplicationWindow) -> Option<(f64, f64)> {
    let surface = window.surface()?;
    let display = surface.display();
    let seat = display.default_seat()?;
    let pointer = seat.pointer()?;
    let (local_x, local_y, _) = surface.device_position(&pointer)?;
    let scale = surface.scale_factor().max(1) as f64;
    let local_x = local_x / scale;
    let local_y = local_y / scale;
    let margin_left = window.margin(Edge::Left) as f64;
    let margin_top = window.margin(Edge::Top) as f64;
    Some((margin_left + local_x, margin_top + local_y))
}

fn trace_drag(message: impl AsRef<str>) {
    if !drag_trace_enabled() {
        return;
    }

    eprintln!("[voxy:drag] {}", message.as_ref());
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
