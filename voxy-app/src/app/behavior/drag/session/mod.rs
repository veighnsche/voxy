use std::cell::Cell;

mod helpers;
mod transitions;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragBounds {
    pub max_left: i32,
    pub max_top: i32,
}

impl DragBounds {
    pub fn from_extents(width: i32, height: i32) -> Self {
        Self {
            max_left: width.max(0),
            max_top: height.max(0),
        }
    }
}

#[derive(Default)]
pub struct DragSession {
    active: Cell<bool>,
    base_left: Cell<i32>,
    base_top: Cell<i32>,
    last_position: Cell<Option<(i32, i32)>>,
}

impl DragSession {
    pub fn position_for_offset(
        &self,
        offset_x: f64,
        offset_y: f64,
        bounds: DragBounds,
    ) -> Option<(i32, i32)> {
        let (raw_left, raw_top) = helpers::raw_from_base_offset(
            self.base_left.get(),
            self.base_top.get(),
            offset_x,
            offset_y,
        );
        self.position_for_raw(raw_left, raw_top, bounds)
    }

    pub fn position_for_incremental(
        &self,
        current_left: i32,
        current_top: i32,
        offset_x: f64,
        offset_y: f64,
        bounds: DragBounds,
    ) -> Option<(i32, i32)> {
        let (raw_left, raw_top) =
            helpers::raw_from_current_offset(current_left, current_top, offset_x, offset_y);
        self.position_for_raw(raw_left, raw_top, bounds)
    }

    pub fn position_for_raw(
        &self,
        raw_left: f64,
        raw_top: f64,
        bounds: DragBounds,
    ) -> Option<(i32, i32)> {
        let candidate = helpers::clamp_raw_position(raw_left, raw_top, bounds);

        if self.last_position.get() == Some(candidate) {
            return None;
        }

        self.last_position.set(Some(candidate));
        Some(candidate)
    }
}
