use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use gtk4::{prelude::*, Application};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_core::{AppEvent, CoreModel};
use voxy_stt::TranscriptionModel;

use crate::{
    app::{
        behavior::{drag, surface::layer_shell::LayerShellBackend, visibility::close_request},
        lifecycle, view_sync,
    },
    diagnostics, tray,
    ui::{self, Widgets},
    wiring::{self, command_bus::CommandBus, transcriber::AppTranscriber},
};

const RUNTIME_ERROR_CLEAR_DELAY: Duration = Duration::from_secs(2);
const LAYER_SHELL_UNSUPPORTED_MESSAGE: &str = "Layer-shell unsupported on this compositor/session";

pub fn run() {
    let runtime = wiring::runtime::build();
    let app = lifecycle::build_application();

    app.connect_activate(move |app| activate(app, Arc::clone(&runtime)));
    app.run();
}

fn activate(app: &Application, runtime: Arc<Runtime>) {
    if let Some(existing_window) = app.windows().into_iter().next() {
        existing_window.present();
        return;
    }

    let widgets = ui::build(app);
    diagnostics::smoke_hooks::mark_window_created();

    let layer_shell_backend = Rc::new(LayerShellBackend::detect());
    layer_shell_backend.configure_window(&widgets.window);

    let model = Rc::new(RefCell::new(CoreModel::default()));
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
    drag::connect_drag_surface(&widgets.window, {
        let model = Rc::clone(&model);
        let layer_shell_backend = Rc::clone(&layer_shell_backend);
        let window = widgets.window.clone();

        move |left, top| {
            model.borrow_mut().set_window_position(left, top);
            layer_shell_backend.apply_position(&window, left, top);
        }
    });

    let selected_model = Arc::new(Mutex::new(TranscriptionModel::default()));

    let command_bus = CommandBus::new(
        event_tx.clone(),
        Arc::clone(&transcriber),
        Arc::clone(&audio_input),
        Arc::clone(&runtime),
        widgets.window.clone(),
        app.clone(),
        Arc::clone(&selected_model),
    );

    wire_ui_signals(
        widgets.clone(),
        Rc::clone(&model),
        Rc::clone(&applying_text_update),
        event_tx.clone(),
        command_bus.clone(),
    );

    start_event_loop(
        widgets.clone(),
        Rc::clone(&model),
        Rc::clone(&applying_text_update),
        Rc::clone(&layer_shell_backend),
        event_rx,
        event_tx.clone(),
        command_bus,
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

fn wire_ui_signals(
    widgets: Widgets,
    model: Rc<RefCell<CoreModel>>,
    applying_text_update: Rc<Cell<bool>>,
    event_tx: mpsc::Sender<AppEvent>,
    command_bus: CommandBus,
) {
    {
        let event_tx = event_tx.clone();
        widgets.mic_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log("ui", "mic_button.clicked -> AppEvent::MicToggled");
            let _ = event_tx.try_send(AppEvent::MicToggled);
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.reset_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                "reset_button.clicked -> AppEvent::ResetRequested",
            );
            let _ = event_tx.try_send(AppEvent::ResetRequested);
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.copy_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                "copy_button.clicked -> AppEvent::CopyRequested",
            );
            let _ = event_tx.try_send(AppEvent::CopyRequested);
        });
    }

    {
        let command_bus = command_bus.clone();
        widgets.model_dropdown.connect_changed(move |dropdown| {
            let Some(model_id) = dropdown.active_id() else {
                return;
            };
            let Some(model) = TranscriptionModel::from_api_id(model_id.as_str()) else {
                return;
            };
            diagnostics::pipeline_trace::log(
                "ui",
                format!("model_dropdown.changed -> {}", model.as_api_id()),
            );
            command_bus.set_transcription_model(model);
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.close_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                "close_button.clicked -> AppEvent::VisibilityToggled",
            );
            let _ = event_tx.try_send(AppEvent::VisibilityToggled);
        });
    }

    {
        let model = Rc::clone(&model);
        let applying_text_update = Rc::clone(&applying_text_update);

        widgets
            .text_buffer
            .connect_changed(move |buffer: &gtk4::TextBuffer| {
                if applying_text_update.get() {
                    return;
                }

                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                    .to_string();
                diagnostics::pipeline_trace::log(
                    "ui",
                    format!("text_buffer.changed user_edit_len={}", text.len()),
                );

                model.borrow_mut().apply_user_edit(text);
            });
    }
}

fn start_event_loop(
    widgets: Widgets,
    model: Rc<RefCell<CoreModel>>,
    applying_text_update: Rc<Cell<bool>>,
    layer_shell_backend: Rc<LayerShellBackend>,
    event_rx: Rc<RefCell<mpsc::Receiver<AppEvent>>>,
    event_tx: mpsc::Sender<AppEvent>,
    command_bus: CommandBus,
) {
    let model_for_events = Rc::clone(&model);
    let event_tx_for_errors = event_tx.clone();
    let command_bus_for_events = command_bus.clone();

    let widgets_for_render = widgets.clone();
    let model_for_render = Rc::clone(&model);
    let applying_for_render = Rc::clone(&applying_text_update);
    let layer_shell_backend_for_render = Rc::clone(&layer_shell_backend);

    wiring::event_loop::start(
        event_rx,
        move |event| {
            let should_clear_runtime_error = matches!(&event, AppEvent::RuntimeError(_));
            diagnostics::pipeline_trace::log("event", format!("received={event:?}"));

            let commands = model_for_events.borrow_mut().reduce(event);
            diagnostics::pipeline_trace::log("event", format!("commands={commands:?}"));
            command_bus_for_events.execute(commands);

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

            if should_clear_runtime_error {
                schedule_runtime_error_clear(event_tx_for_errors.clone());
            }
        },
        move || {
            render_ui(
                &widgets_for_render,
                &model_for_render,
                &layer_shell_backend_for_render,
                &applying_for_render,
            );
        },
    );
}

fn schedule_runtime_error_clear(event_tx: mpsc::Sender<AppEvent>) {
    gtk4::glib::timeout_add_local(RUNTIME_ERROR_CLEAR_DELAY, move || {
        let _ = event_tx.try_send(AppEvent::ErrorCleared);
        gtk4::glib::ControlFlow::Break
    });
}

fn render_ui(
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
