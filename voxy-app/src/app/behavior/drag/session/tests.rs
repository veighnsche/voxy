use super::{DragBounds, DragSession};

#[test]
fn applies_logical_delta_from_drag_start() {
    let session = DragSession::default();
    session.begin(100, 80);

    let position = session.position_for_offset(10.0, -5.0, DragBounds::from_extents(2_000, 2_000));

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

    let position = session.position_for_offset(-100.0, -100.0, DragBounds::from_extents(100, 100));

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

    let position =
        session.position_for_incremental(10, 10, 100.0, 50.0, DragBounds::from_extents(500, 500));

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
