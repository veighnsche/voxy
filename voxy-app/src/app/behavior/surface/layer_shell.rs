use gtk4::{gdk, glib::object::Cast, prelude::*, ApplicationWindow};
use gtk4_layer_shell::{
    is_supported as layer_shell_supported, Edge, KeyboardMode, Layer, LayerShell,
};

use crate::app::behavior::surface::placement;

const LAYER_NAMESPACE: &str = "voxy";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorCycleOutcome {
    Moved {
        from_index: usize,
        to_index: usize,
        monitor_count: usize,
    },
    SingleMonitor,
}

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

    pub fn move_window_to_next_monitor(
        &self,
        window: &ApplicationWindow,
    ) -> Result<MonitorCycleOutcome, String> {
        if !self.supported {
            return Err("cannot move to next screen: layer-shell is unavailable".to_owned());
        }

        let display = gtk4::prelude::WidgetExt::display(window);
        let monitors = display_monitors(&display);
        if monitors.is_empty() {
            return Err("cannot move to next screen: no monitors reported".to_owned());
        }
        if monitors.len() == 1 {
            return Ok(MonitorCycleOutcome::SingleMonitor);
        }

        let current_monitor = window
            .surface()
            .and_then(|surface| display.monitor_at_surface(&surface))
            .or_else(|| window.monitor());
        let current_index = current_monitor
            .as_ref()
            .and_then(|monitor| monitors.iter().position(|candidate| candidate == monitor))
            .unwrap_or(0);
        let next_index = (current_index + 1) % monitors.len();

        window.set_monitor(&monitors[next_index]);
        Ok(MonitorCycleOutcome::Moved {
            from_index: current_index,
            to_index: next_index,
            monitor_count: monitors.len(),
        })
    }
}

fn display_monitors(display: &gdk::Display) -> Vec<gdk::Monitor> {
    let list = display.monitors();
    let mut monitors = Vec::with_capacity(list.n_items() as usize);
    for index in 0..list.n_items() {
        let Some(item) = list.item(index) else {
            continue;
        };
        let Ok(monitor) = item.downcast::<gdk::Monitor>() else {
            continue;
        };
        monitors.push(monitor);
    }

    monitors
}
