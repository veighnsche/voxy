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
    last_position: Cell<Option<(i32, i32)>>,
}

impl DragSession {
    pub fn begin(&self) {
        self.active.set(true);
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

    pub fn position_for(
        &self,
        current_left: i32,
        current_top: i32,
        offset_x: f64,
        offset_y: f64,
        bounds: DragBounds,
    ) -> Option<(i32, i32)> {
        let raw_left = (current_left as f64) + offset_x;
        let raw_top = (current_top as f64) + offset_y;

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
    fn applies_logical_delta_without_scale_multiplication() {
        let session = DragSession::default();
        session.begin();

        let position =
            session.position_for(100, 80, 10.0, -5.0, DragBounds::from_extents(2_000, 2_000));

        assert_eq!(position, Some((110, 75)));
    }

    #[test]
    fn clamps_to_bounds() {
        let session = DragSession::default();
        session.begin();

        let position = session.position_for(90, 95, 20.0, 20.0, DragBounds::from_extents(100, 100));

        assert_eq!(position, Some((100, 100)));
    }

    #[test]
    fn clamps_negative_values_to_zero() {
        let session = DragSession::default();
        session.begin();

        let position =
            session.position_for(5, 5, -100.0, -100.0, DragBounds::from_extents(100, 100));

        assert_eq!(position, Some((0, 0)));
    }

    #[test]
    fn suppresses_duplicate_position_updates() {
        let session = DragSession::default();
        session.begin();

        let first = session.position_for(10, 10, 0.49, 0.49, DragBounds::from_extents(100, 100));
        let second = session.position_for(10, 10, 0.49, 0.49, DragBounds::from_extents(100, 100));

        assert_eq!(first, Some((10, 10)));
        assert_eq!(second, None);
    }

    #[test]
    fn applies_incremental_offsets_from_current_position() {
        let session = DragSession::default();
        session.begin();

        let position =
            session.position_for(10, 10, 100.0, 50.0, DragBounds::from_extents(500, 500));

        assert_eq!(position, Some((110, 60)));
    }

    #[test]
    fn snapshots_current_drag_sequence_behavior() {
        let session = DragSession::default();
        session.begin();

        let bounds = DragBounds::from_extents(100, 100);
        let mut current = (24, 24);
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
            let next = session.position_for(current.0, current.1, offset_x, offset_y, bounds);
            if let Some(position) = next {
                current = position;
            }
            observed.push(next);
        }

        assert_eq!(
            observed,
            vec![
                Some((34, 24)),
                Some((54, 24)),
                Some((84, 24)),
                Some((100, 24)),
                Some((0, 0)),
                None,
                Some((16, 15)),
            ]
        );
    }
}
