use super::DragBounds;

pub(super) fn raw_from_base_offset(
    base_left: i32,
    base_top: i32,
    offset_x: f64,
    offset_y: f64,
) -> (f64, f64) {
    ((base_left as f64) + offset_x, (base_top as f64) + offset_y)
}

pub(super) fn raw_from_current_offset(
    current_left: i32,
    current_top: i32,
    offset_x: f64,
    offset_y: f64,
) -> (f64, f64) {
    (
        (current_left as f64) + offset_x,
        (current_top as f64) + offset_y,
    )
}

pub(super) fn clamp_raw_position(raw_left: f64, raw_top: f64, bounds: DragBounds) -> (i32, i32) {
    let left = (raw_left.round() as i32).clamp(0, bounds.max_left);
    let top = (raw_top.round() as i32).clamp(0, bounds.max_top);
    (left, top)
}
