use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use tokio::sync::mpsc;
use voxy_core::{AppEvent, CoreModel};

use crate::{
    app::behavior::surface::layer_shell::LayerShellBackend,
    diagnostics,
    ui::pages::voxy_window_page::Widgets,
    wiring::{self, command_bus::CommandBus},
};

use super::{bootstrap, settings_sync};

pub(super) fn start_event_loop(
    widgets: Widgets,
    model: Rc<RefCell<CoreModel>>,
    applying_text_update: Rc<Cell<bool>>,
    layer_shell_backend: Rc<LayerShellBackend>,
    event_rx: Rc<RefCell<mpsc::Receiver<AppEvent>>>,
    command_bus: CommandBus,
) {
    let model_for_render = model;
    let model_for_events = Rc::clone(&model_for_render);
    let command_bus_for_events = command_bus;

    let widgets_for_render = widgets;
    let applying_for_render = applying_text_update;
    let layer_shell_backend_for_render = layer_shell_backend;

    wiring::event_loop::start(
        event_rx,
        move |event| {
            diagnostics::pipeline_trace::log("event", format!("received={event:?}"));

            let before_settings = {
                let snapshot = model_for_events.borrow();
                settings_sync::SettingsSnapshot::from_model(&snapshot)
            };
            let commands = model_for_events.borrow_mut().reduce(event);
            diagnostics::pipeline_trace::log("event", format!("commands={commands:?}"));
            command_bus_for_events.execute(commands);
            let after_settings = {
                let snapshot = model_for_events.borrow();
                settings_sync::SettingsSnapshot::from_model(&snapshot)
            };
            settings_sync::persist_changed_settings(before_settings, after_settings);

            {
                let snapshot = model_for_events.borrow();
                diagnostics::pipeline_trace::log(
                    "state",
                    format!(
                        "app_state={:?} confirmed_len={} live_len={} full_len={} error={}",
                        snapshot.app_state,
                        snapshot.buffer.confirmed_text.len(),
                        snapshot.buffer.live_segment.len(),
                        snapshot.buffer.full_text().len(),
                        snapshot.runtime_error.is_some()
                    ),
                );
            }
        },
        move || {
            bootstrap::render_ui(
                &widgets_for_render,
                &model_for_render,
                &layer_shell_backend_for_render,
                &applying_for_render,
            );
        },
    );
}
