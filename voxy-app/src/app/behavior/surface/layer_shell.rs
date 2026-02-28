use gtk4::ApplicationWindow;
use gtk4_layer_shell::{
    is_supported as layer_shell_supported, Edge, KeyboardMode, Layer, LayerShell,
};

use crate::app::behavior::surface::placement;

const LAYER_NAMESPACE: &str = "voxy";

#[derive(Debug, Clone)]
pub struct LayerShellBackend {
    supported: bool,
}

impl LayerShellBackend {
    pub fn detect() -> Self {
        Self {
            supported: layer_shell_supported(),
        }
    }

    pub fn is_supported(&self) -> bool {
        self.supported
    }

    pub fn configure_window(&self, window: &ApplicationWindow) {
        if !self.supported {
            return;
        }

        window.init_layer_shell();
        window.set_namespace(LAYER_NAMESPACE);
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Right, false);
        window.set_exclusive_zone(-1);
    }

    pub fn apply_position(&self, window: &ApplicationWindow, left: i32, top: i32) {
        if !self.supported {
            return;
        }

        placement::apply_anchored_position(window, left, top);
    }
}
