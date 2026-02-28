use std::{
    cell::Cell, env, fs, os::fd::AsRawFd, os::unix::net::UnixStream, path::PathBuf, rc::Rc,
    sync::OnceLock, time::Instant,
};

use gtk4::{prelude::*, ApplicationWindow, GestureClick, GestureDrag};
use gtk4_layer_shell::{Edge, LayerShell};

use super::{
    hit_test,
    session::{DragBounds, DragSession},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragMathMode {
    LegacyIncremental,
    PointerAnchor,
}

impl DragMathMode {
    fn label(self) -> &'static str {
        match self {
            Self::LegacyIncremental => "legacy",
            Self::PointerAnchor => "anchor",
        }
    }
}

pub fn connect_drag_surface(
    window: &ApplicationWindow,
    on_position: impl Fn(i32, i32) + 'static,
    on_double_click: impl Fn() + 'static,
) {
    let drag_math_mode = detect_drag_math_mode();
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

            let base_left = window_for_start.margin(Edge::Left);
            let base_top = window_for_start.margin(Edge::Top);
            drag_session.begin(base_left, base_top);
            let start_abs_x = (base_left as f64) + start_x;
            let start_abs_y = (base_top as f64) + start_y;
            trace_state.begin(base_left, base_top, start_abs_x, start_abs_y);
            trace_drag(|| {
                let scale_factor = current_scale_factor(&window_for_start);
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

            let bounds = current_drag_bounds(&window_for_update);
            let next_position = match drag_math_mode {
                DragMathMode::LegacyIncremental => {
                    let current_left = window_for_update.margin(Edge::Left);
                    let current_top = window_for_update.margin(Edge::Top);
                    drag_session.position_for_incremental(
                        current_left,
                        current_top,
                        offset_x,
                        offset_y,
                        bounds,
                    )
                }
                DragMathMode::PointerAnchor => pointer_abs(&window_for_update)
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

fn current_scale_factor(window: &ApplicationWindow) -> i32 {
    window
        .surface()
        .map(|surface| surface.scale_factor())
        .unwrap_or_else(|| window.scale_factor())
        .max(1)
}

fn detect_drag_math_mode() -> DragMathMode {
    if let Some(mode) = drag_math_mode_override() {
        return mode;
    }

    let compositor_name = detect_wayland_compositor_name();
    drag_math_mode_for_compositor_name(compositor_name.as_deref())
}

fn drag_math_mode_override() -> Option<DragMathMode> {
    let raw = env::var("VOXY_DRAG_MATH").ok()?;
    let value = raw.trim().to_ascii_lowercase();
    match value.as_str() {
        "legacy" | "kde" | "incremental" => Some(DragMathMode::LegacyIncremental),
        "anchor" | "niri" | "pointer" => Some(DragMathMode::PointerAnchor),
        _ => None,
    }
}

fn drag_math_mode_for_compositor_name(compositor_name: Option<&str>) -> DragMathMode {
    let Some(name) = compositor_name else {
        return DragMathMode::LegacyIncremental;
    };

    let normalized = name.to_ascii_lowercase();
    if normalized.contains("niri") {
        DragMathMode::PointerAnchor
    } else {
        DragMathMode::LegacyIncremental
    }
}

fn detect_wayland_compositor_name() -> Option<String> {
    let socket_path = detect_wayland_socket_path()?;
    let stream = UnixStream::connect(&socket_path).ok()?;
    let pid = peer_pid(stream.as_raw_fd())?;

    read_proc_comm(pid).or_else(|| read_proc_exe_name(pid))
}

fn detect_wayland_socket_path() -> Option<PathBuf> {
    let wayland_display = env::var("WAYLAND_DISPLAY").ok()?;
    let display_path = PathBuf::from(&wayland_display);
    if display_path.is_absolute() {
        return Some(display_path);
    }

    let runtime_dir = env::var("XDG_RUNTIME_DIR").ok()?;
    Some(PathBuf::from(runtime_dir).join(display_path))
}

fn peer_pid(fd: std::os::fd::RawFd) -> Option<u32> {
    let mut ucred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut ucred as *mut libc::ucred as *mut libc::c_void,
            &mut len,
        )
    };
    if rc != 0 || len as usize != std::mem::size_of::<libc::ucred>() || ucred.pid <= 0 {
        return None;
    }

    Some(ucred.pid as u32)
}

fn read_proc_comm(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/comm");
    let content = fs::read_to_string(path).ok()?;
    let name = content.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn read_proc_exe_name(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/exe");
    let target = fs::read_link(path).ok()?;
    target
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{drag_math_mode_for_compositor_name, DragMathMode};

    #[test]
    fn niri_uses_anchor_mode() {
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("niri")),
            DragMathMode::PointerAnchor
        );
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("Niri")),
            DragMathMode::PointerAnchor
        );
    }

    #[test]
    fn kwin_and_unknown_default_to_legacy_mode() {
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("kwin_wayland")),
            DragMathMode::LegacyIncremental
        );
        assert_eq!(
            drag_math_mode_for_compositor_name(Some("sway")),
            DragMathMode::LegacyIncremental
        );
        assert_eq!(
            drag_math_mode_for_compositor_name(None),
            DragMathMode::LegacyIncremental
        );
    }
}

fn current_drag_bounds(window: &ApplicationWindow) -> DragBounds {
    let (monitor_width, monitor_height) = window
        .surface()
        .and_then(|surface| surface.display().monitor_at_surface(&surface))
        .map(|monitor| {
            let geometry = monitor.geometry();
            (geometry.width(), geometry.height())
        })
        .unwrap_or_else(|| {
            let width = window.width().max(window.default_width()).max(1);
            let height = window.height().max(window.default_height()).max(1);
            (width, height)
        });

    let window_width = window.width().max(window.default_width()).max(1);
    let window_height = window.height().max(window.default_height()).max(1);
    let max_left = monitor_width - window_width;
    let max_top = monitor_height - window_height;

    DragBounds::from_extents(max_left, max_top)
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
        let scale_factor = current_scale_factor(window);
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

        Some(format!(
            "u#{seq} t={elapsed}ms o={offset_x:.1},{offset_y:.1} p={left},{top} sf={scale_factor} b={},{} m={},{} a={abs_x},{abs_y} d={drift_x},{drift_y}",
            bounds.max_left, bounds.max_top, window.width(), window.height()
        ))
    });
}

fn pointer_abs(window: &ApplicationWindow) -> Option<(f64, f64)> {
    let surface = window.surface()?;
    let display = surface.display();
    let seat = display.default_seat()?;
    let pointer = seat.pointer()?;
    let (local_x, local_y, _) = surface.device_position(&pointer)?;
    let margin_left = window.margin(Edge::Left) as f64;
    let margin_top = window.margin(Edge::Top) as f64;
    Some((margin_left + local_x, margin_top + local_y))
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
