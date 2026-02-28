use gtk4::ApplicationWindow;
use gtk4_layer_shell::{is_supported as layer_shell_supported, KeyboardMode, Layer, LayerShell};

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
    }
}
