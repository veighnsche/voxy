use super::model::{MonitorTarget, Point, Rect, Size};

pub(super) fn compute_anchor(pointer_start: Point, window_start: Point) -> Point {
    Point {
        x: pointer_start.x - window_start.x,
        y: pointer_start.y - window_start.y,
    }
}

pub(super) fn window_top_left_from_pointer(pointer_now: Point, anchor: Point) -> Point {
    Point {
        x: pointer_now.x - anchor.x,
        y: pointer_now.y - anchor.y,
    }
}

pub(super) fn desktop_clamp(window_top_left: Point, window_size: Size, monitors: &[Rect]) -> Point {
    let desktop = desktop_extents(monitors);
    Point {
        x: window_top_left
            .x
            .clamp(desktop.x, desktop.x + desktop.w - window_size.w),
        y: window_top_left
            .y
            .clamp(desktop.y, desktop.y + desktop.h - window_size.h),
    }
}

pub(super) fn target_monitor_for_window(
    monitors: &[Rect],
    top_left: Point,
    window_size: Size,
) -> MonitorTarget {
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

pub(super) fn local_to_global(monitor: Rect, local_top_left: Point) -> Point {
    Point {
        x: monitor.x + local_top_left.x,
        y: monitor.y + local_top_left.y,
    }
}

pub(super) fn desktop_extents(monitors: &[Rect]) -> Rect {
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
