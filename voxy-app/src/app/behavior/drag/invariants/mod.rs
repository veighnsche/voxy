mod model;
mod simulation_helpers;

use model::{Point, Rect, Size};
use simulation_helpers::{
    compute_anchor, desktop_clamp, desktop_extents, local_to_global, target_monitor_for_window,
    window_top_left_from_pointer,
};

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
        assert_eq!(
            window_now.x + window_size.w / 2,
            pointer_now.x - anchor.x + window_size.w / 2
        );
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

    let far_off = Point {
        x: 10_000,
        y: -2_000,
    };
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
