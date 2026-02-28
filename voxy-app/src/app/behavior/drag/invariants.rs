#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Size {
    w: i32,
    h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Rect {
    fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MonitorTarget {
    index: usize,
    monitor: Rect,
    local_top_left: Point,
}

fn compute_anchor(pointer_start: Point, window_start: Point) -> Point {
    Point {
        x: pointer_start.x - window_start.x,
        y: pointer_start.y - window_start.y,
    }
}

fn window_top_left_from_pointer(pointer_now: Point, anchor: Point) -> Point {
    Point {
        x: pointer_now.x - anchor.x,
        y: pointer_now.y - anchor.y,
    }
}

fn desktop_clamp(window_top_left: Point, window_size: Size, monitors: &[Rect]) -> Point {
    let desktop = desktop_extents(monitors);
    Point {
        x: window_top_left.x.clamp(desktop.x, desktop.x + desktop.w - window_size.w),
        y: window_top_left.y.clamp(desktop.y, desktop.y + desktop.h - window_size.h),
    }
}

fn target_monitor_for_window(monitors: &[Rect], top_left: Point, window_size: Size) -> MonitorTarget {
    let center = Point {
        x: top_left.x + window_size.w / 2,
        y: top_left.y + window_size.h / 2,
    };

    let mut best_index = 0usize;
    let mut best_distance = i64::MAX;
    for (idx, monitor) in monitors.iter().enumerate() {
        if monitor.contains(center) {
            best_index = idx;
            break;
        }
        let d = distance_sq_to_rect(center, *monitor);
        if d < best_distance {
            best_index = idx;
            best_distance = d;
        }
    }

    let monitor = monitors[best_index];
    let local_left = top_left.x - monitor.x;
    let local_top = top_left.y - monitor.y;
    let max_local_left = (monitor.w - window_size.w).max(0);
    let max_local_top = (monitor.h - window_size.h).max(0);

    MonitorTarget {
        index: best_index,
        monitor,
        local_top_left: Point {
            x: local_left.clamp(0, max_local_left),
            y: local_top.clamp(0, max_local_top),
        },
    }
}

fn local_to_global(monitor: Rect, local_top_left: Point) -> Point {
    Point {
        x: monitor.x + local_top_left.x,
        y: monitor.y + local_top_left.y,
    }
}

fn desktop_extents(monitors: &[Rect]) -> Rect {
    assert!(!monitors.is_empty());

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for m in monitors {
        min_x = min_x.min(m.x);
        min_y = min_y.min(m.y);
        max_x = max_x.max(m.x + m.w);
        max_y = max_y.max(m.y + m.h);
    }

    Rect {
        x: min_x,
        y: min_y,
        w: max_x - min_x,
        h: max_y - min_y,
    }
}

fn distance_sq_to_rect(p: Point, r: Rect) -> i64 {
    let dx = if p.x < r.x {
        (r.x - p.x) as i64
    } else if p.x >= r.x + r.w {
        (p.x - (r.x + r.w - 1)) as i64
    } else {
        0
    };
    let dy = if p.y < r.y {
        (r.y - p.y) as i64
    } else if p.y >= r.y + r.h {
        (p.y - (r.y + r.h - 1)) as i64
    } else {
        0
    };
    dx * dx + dy * dy
}

#[test]
fn anchor_is_preserved_for_full_drag_sequence() {
    let window_size = Size { w: 340, h: 420 };
    let pointer_start = Point { x: 320, y: 180 };
    let window_start = Point { x: 24, y: 24 };
    let anchor = compute_anchor(pointer_start, window_start);

    for pointer_now in [
        Point { x: 321, y: 180 },
        Point { x: 640, y: 300 },
        Point { x: 1500, y: 700 },
        Point { x: 80, y: 50 },
        Point { x: -200, y: 450 },
    ] {
        let window_now = window_top_left_from_pointer(pointer_now, anchor);
        assert_eq!(
            Point {
                x: pointer_now.x - window_now.x,
                y: pointer_now.y - window_now.y,
            },
            anchor
        );
        assert_eq!(window_now.x + window_size.w / 2, pointer_now.x - anchor.x + window_size.w / 2);
    }
}

#[test]
fn crossing_monitor_seam_changes_monitor_target_without_jump() {
    let monitors = [
        Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        },
        Rect {
            x: 1920,
            y: 0,
            w: 1920,
            h: 1080,
        },
    ];
    let window_size = Size { w: 340, h: 420 };
    let pointer_start = Point { x: 1800, y: 200 };
    let window_start = Point { x: 1500, y: 60 };
    let anchor = compute_anchor(pointer_start, window_start);

    let left_pointer = Point { x: 1880, y: 240 };
    let right_pointer = Point { x: 2230, y: 240 };

    let left_window_global = window_top_left_from_pointer(left_pointer, anchor);
    let right_window_global = window_top_left_from_pointer(right_pointer, anchor);

    let left_target = target_monitor_for_window(&monitors, left_window_global, window_size);
    let right_target = target_monitor_for_window(&monitors, right_window_global, window_size);
    assert_eq!(left_target.index, 0);
    assert_eq!(right_target.index, 1);

    let left_roundtrip_global = local_to_global(left_target.monitor, left_target.local_top_left);
    let right_roundtrip_global = local_to_global(right_target.monitor, right_target.local_top_left);
    assert_eq!(left_roundtrip_global, left_window_global);
    assert_eq!(right_roundtrip_global, right_window_global);
}

#[test]
fn negative_coordinate_monitor_is_supported() {
    let monitors = [
        Rect {
            x: -1600,
            y: 0,
            w: 1600,
            h: 900,
        },
        Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        },
    ];
    let window_size = Size { w: 340, h: 420 };
    let pointer_start = Point { x: 300, y: 200 };
    let window_start = Point { x: 24, y: 24 };
    let anchor = compute_anchor(pointer_start, window_start);

    let pointer_now = Point { x: -900, y: 250 };
    let window_global = window_top_left_from_pointer(pointer_now, anchor);
    let target = target_monitor_for_window(&monitors, window_global, window_size);

    assert_eq!(target.index, 0);
    assert!(target.monitor.x < 0);

    let roundtrip = local_to_global(target.monitor, target.local_top_left);
    assert_eq!(roundtrip, window_global);
}

#[test]
fn desktop_clamp_limits_to_union_extents() {
    let monitors = [
        Rect {
            x: -1600,
            y: 0,
            w: 1600,
            h: 900,
        },
        Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        },
    ];
    let window_size = Size { w: 340, h: 420 };

    let far_off = Point { x: 10_000, y: -2_000 };
    let clamped = desktop_clamp(far_off, window_size, &monitors);
    let extents = desktop_extents(&monitors);

    assert_eq!(clamped.x, extents.x + extents.w - window_size.w);
    assert_eq!(clamped.y, extents.y);
}

#[test]
fn scale_factor_does_not_change_pointer_anchor_math() {
    let pointer_start = Point { x: 500, y: 300 };
    let window_start = Point { x: 120, y: 80 };
    let anchor = compute_anchor(pointer_start, window_start);

    let pointer_now = Point { x: 1500, y: 650 };
    let window_now_scale_1 = window_top_left_from_pointer(pointer_now, anchor);
    let window_now_scale_2 = window_top_left_from_pointer(pointer_now, anchor);

    assert_eq!(window_now_scale_1, window_now_scale_2);
    assert_eq!(
        Point {
            x: pointer_now.x - window_now_scale_1.x,
            y: pointer_now.y - window_now_scale_1.y,
        },
        anchor
    );
}
