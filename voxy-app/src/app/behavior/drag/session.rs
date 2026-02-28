use std::cell::Cell;

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
    pub fn begin(&self, base_left: i32, base_top: i32) {
        self.active.set(true);
        self.base_left.set(base_left);
        self.base_top.set(base_top);
        self.last_position.set(None);
    }

    pub fn cancel(&self) {
        self.end();
    }

    pub fn end(&self) {
        self.active.set(false);
        self.last_position.set(None);
    }

    pub fn is_active(&self) -> bool {
        self.active.get()
    }

    pub fn position_for_offset(
        &self,
        offset_x: f64,
        offset_y: f64,
        bounds: DragBounds,
    ) -> Option<(i32, i32)> {
        let raw_left = (self.base_left.get() as f64) + offset_x;
        let raw_top = (self.base_top.get() as f64) + offset_y;
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
        let raw_left = (current_left as f64) + offset_x;
        let raw_top = (current_top as f64) + offset_y;
        self.position_for_raw(raw_left, raw_top, bounds)
    }

    pub fn position_for_raw(
        &self,
        raw_left: f64,
        raw_top: f64,
        bounds: DragBounds,
    ) -> Option<(i32, i32)> {
        let left = (raw_left.round() as i32).clamp(0, bounds.max_left);
        let top = (raw_top.round() as i32).clamp(0, bounds.max_top);
        let candidate = (left, top);

        if self.last_position.get() == Some(candidate) {
            return None;
        }

        self.last_position.set(Some(candidate));
        Some(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::{DragBounds, DragSession};

    #[test]
    fn applies_logical_delta_from_drag_start() {
        let session = DragSession::default();
        session.begin(100, 80);

        let position =
            session.position_for_offset(10.0, -5.0, DragBounds::from_extents(2_000, 2_000));

        assert_eq!(position, Some((110, 75)));
    }

    #[test]
    fn clamps_to_bounds() {
        let session = DragSession::default();
        session.begin(90, 95);

        let position = session.position_for_offset(20.0, 20.0, DragBounds::from_extents(100, 100));

        assert_eq!(position, Some((100, 100)));
    }

    #[test]
    fn clamps_negative_values_to_zero() {
        let session = DragSession::default();
        session.begin(5, 5);

        let position =
            session.position_for_offset(-100.0, -100.0, DragBounds::from_extents(100, 100));

        assert_eq!(position, Some((0, 0)));
    }

    #[test]
    fn suppresses_duplicate_position_updates() {
        let session = DragSession::default();
        session.begin(10, 10);

        let first = session.position_for_offset(0.49, 0.49, DragBounds::from_extents(100, 100));
        let second = session.position_for_offset(0.49, 0.49, DragBounds::from_extents(100, 100));

        assert_eq!(first, Some((10, 10)));
        assert_eq!(second, None);
    }

    #[test]
    fn applies_raw_pointer_target() {
        let session = DragSession::default();
        session.begin(10, 10);

        let position = session.position_for_raw(123.6, 49.2, DragBounds::from_extents(500, 500));

        assert_eq!(position, Some((124, 49)));
    }

    #[test]
    fn applies_incremental_offsets_from_current_position() {
        let session = DragSession::default();
        session.begin(10, 10);

        let position = session.position_for_incremental(
            10,
            10,
            100.0,
            50.0,
            DragBounds::from_extents(500, 500),
        );

        assert_eq!(position, Some((110, 60)));
    }

    #[test]
    fn snapshots_current_drag_sequence_behavior() {
        let session = DragSession::default();
        session.begin(24, 24);

        let bounds = DragBounds::from_extents(100, 100);
        let mut observed = Vec::new();

        for (offset_x, offset_y) in [
            (10.0, 0.0),
            (20.0, 0.0),
            (30.0, 0.0),
            (30.0, 0.0),
            (-200.0, -50.0),
            (-200.0, -50.0),
            (15.6, 15.4),
        ] {
            let next = session.position_for_offset(offset_x, offset_y, bounds);
            observed.push(next);
        }

        assert_eq!(
            observed,
            vec![
                Some((34, 24)),
                Some((44, 24)),
                Some((54, 24)),
                None,
                Some((0, 0)),
                None,
                Some((40, 39)),
            ]
        );
    }
}
