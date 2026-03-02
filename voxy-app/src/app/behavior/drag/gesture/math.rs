use gtk4::{prelude::*, ApplicationWindow};
use gtk4_layer_shell::{Edge, LayerShell};

use crate::app::behavior::drag::session::DragBounds;

pub(super) fn current_scale_factor(window: &ApplicationWindow) -> i32 {
    window
        .surface()
        .map(|surface| surface.scale_factor())
        .unwrap_or_else(|| window.scale_factor())
        .max(1)
}

pub(super) fn current_drag_bounds(window: &ApplicationWindow) -> DragBounds {
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

pub(super) fn pointer_abs(window: &ApplicationWindow) -> Option<(f64, f64)> {
    let surface = window.surface()?;
    let display = surface.display();
    let seat = display.default_seat()?;
    let pointer = seat.pointer()?;
    let (local_x, local_y, _) = surface.device_position(&pointer)?;
    let margin_left = window.margin(Edge::Left) as f64;
    let margin_top = window.margin(Edge::Top) as f64;
    Some((margin_left + local_x, margin_top + local_y))
}
