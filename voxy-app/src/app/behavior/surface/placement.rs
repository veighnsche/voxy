use gtk4::ApplicationWindow;
use gtk4_layer_shell::{Edge, LayerShell};

pub fn apply_anchored_position(window: &ApplicationWindow, left: i32, top: i32) {
    window.set_margin(Edge::Left, left.max(0));
    window.set_margin(Edge::Top, top.max(0));
}
