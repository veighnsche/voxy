use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use gtk4::{prelude::*, Application};
use tokio::runtime::Runtime;
use voxy_core::{AppEvent, CoreModel};

use crate::{
    app::{
        behavior::{
            drag, resize, surface::layer_shell::LayerShellBackend, visibility::close_request,
        },
        view_sync,
    },
    diagnostics, tray,
    ui::{self, Widgets},
    wiring::{self, command_bus::CommandBus, transcriber::AppTranscriber},
};

use super::{event_processing, input_meter_loop, settings_sync, ui_signals};

const LAYER_SHELL_UNSUPPORTED_MESSAGE: &str = "Layer-shell unsupported on this compositor/session";

pub(super) fn activate(app: &Application, runtime: Arc<Runtime>) {
    if let Some(existing_window) = app.windows().into_iter().next() {
        existing_window.present();
        return;
    }

    let widgets = ui::build(app);
    diagnostics::smoke_hooks::mark_window_created();

    let layer_shell_backend = Rc::new(LayerShellBackend::detect());
    layer_shell_backend.configure_window(&widgets.window);

    let model = Rc::new(RefCell::new(CoreModel::default()));
    let startup_settings = settings_sync::load_startup_settings();
    {
        let mut snapshot = model.borrow_mut();
        startup_settings.apply_to_model(&mut snapshot);
    }
    let applying_text_update = Rc::new(Cell::new(false));
    let wiring::channels::AppChannels { event_tx, event_rx } =
        wiring::channels::build_event_channels();
    let event_rx = Rc::new(RefCell::new(event_rx));

    let audio_input = Arc::new(voxy_audio::InputEngine::new());
    let transcriber = Arc::new(AppTranscriber::from_env(
        event_tx.clone(),
        Some(audio_input.clone() as Arc<dyn voxy_audio::AudioFrameSource>),
    ));
    diagnostics::pipeline_trace::log(
        "activate",
        format!("selected_stt_backend={}", transcriber.backend_name()),
    );
    let _ = event_tx.try_send(AppEvent::LogMessage(format!(
        "STT backend: {}",
        transcriber.backend_name()
    )));

    diagnostics::smoke_hooks::install(&widgets.window, &event_tx);
    diagnostics::smoke_hooks::install_visibility_toggle_injector({
        let event_tx = event_tx.clone();
        move || {
            let _ = event_tx.try_send(AppEvent::VisibilityToggled);
        }
    });

    close_request::install_hide_on_close(&widgets.window, event_tx.clone());
    drag::connect_drag_surface(
        &widgets.window,
        {
            let model = Rc::clone(&model);
            let layer_shell_backend = Rc::clone(&layer_shell_backend);
            let window = widgets.window.clone();

            move |left, top| {
                model.borrow_mut().set_window_position(left, top);
                layer_shell_backend.apply_position(&window, left, top);
            }
        },
        {
            let event_tx = event_tx.clone();

            move || {
                diagnostics::pipeline_trace::log(
                    "ui",
                    "drag_surface.double_click -> AppEvent::MicToggled",
                );
                let _ = event_tx.try_send(AppEvent::MicToggled);
            }
        },
    );

    resize::connect_resize_handle(&widgets.window, widgets.resize_handle.upcast_ref(), {
        let event_tx = event_tx.clone();
        move |width, height| {
            diagnostics::pipeline_trace::log(
                "ui",
                format!("resize_handle.drag -> AppEvent::WindowResizeRequested {width}x{height}"),
            );
            let _ = event_tx.try_send(AppEvent::WindowResizeRequested { width, height });
        }
    });

    let command_bus = CommandBus::new(
        event_tx.clone(),
        Arc::clone(&transcriber),
        Arc::clone(&audio_input),
        Arc::clone(&runtime),
        widgets.window.clone(),
        app.clone(),
        layer_shell_backend.as_ref().clone(),
    );

    ui_signals::wire_ui_signals(
        widgets.clone(),
        Rc::clone(&model),
        Rc::clone(&applying_text_update),
        event_tx.clone(),
    );

    event_processing::start_event_loop(
        widgets.clone(),
        Rc::clone(&model),
        Rc::clone(&applying_text_update),
        Rc::clone(&layer_shell_backend),
        event_rx,
        command_bus,
    );

    input_meter_loop::start_input_level_meter_loop(
        widgets.clone(),
        Rc::clone(&model),
        Arc::clone(&audio_input),
        event_tx.clone(),
    );

    if let Err(message) = tray::start(event_tx.clone()) {
        let _ = event_tx.try_send(AppEvent::RuntimeError(message));
    }

    if !layer_shell_backend.is_supported() {
        let _ = event_tx.try_send(AppEvent::RuntimeError(
            LAYER_SHELL_UNSUPPORTED_MESSAGE.to_owned(),
        ));
    }

    render_ui(
        &widgets,
        &model,
        &layer_shell_backend,
        &applying_text_update,
    );
    widgets.window.present();
}

pub(super) fn render_ui(
    widgets: &Widgets,
    model: &Rc<RefCell<CoreModel>>,
    layer_shell_backend: &LayerShellBackend,
    applying_text_update: &Rc<Cell<bool>>,
) {
    view_sync::render(widgets, model, applying_text_update);

    let model = model.borrow();
    layer_shell_backend.apply_position(
        &widgets.window,
        model.ui_prefs.window_left,
        model.ui_prefs.window_top,
    );
}
