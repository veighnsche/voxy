use std::{
    cell::{Cell, RefCell},
    env,
    rc::Rc,
    sync::OnceLock,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use gtk4::{gdk, prelude::*, Application, EventControllerKey, PropagationPhase};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_core::{AppEvent, AppState, CoreModel};
use voxy_stt::TranscriptionModel;

use crate::{
    app::{
        behavior::{
            drag, resize, surface::layer_shell::LayerShellBackend, visibility::close_request,
        },
        lifecycle, settings_store, view_sync,
    },
    diagnostics, tray,
    ui::{self, Widgets},
    wiring::{self, command_bus::CommandBus, transcriber::AppTranscriber},
};

const RUNTIME_ERROR_CLEAR_DELAY: Duration = Duration::from_secs(2);
const INPUT_LEVEL_POLL_INTERVAL: Duration = Duration::from_millis(50);
const MAX_RECORDING_SECONDS_ENV: &str = "VOXY_MAX_RECORDING_SECONDS";
const DEFAULT_MAX_RECORDING_SECONDS: u64 = 30 * 60;
const SILENCE_AUTO_STOP_SECONDS_ENV: &str = "VOXY_SILENCE_AUTO_STOP_SECONDS";
const DEFAULT_SILENCE_AUTO_STOP_SECONDS: u64 = 10;
const SILENCE_GATE_RELEASE_HYSTERESIS: f32 = 0.05;
const SILENCE_RESET_DEBOUNCE: Duration = Duration::from_millis(300);
const GATE_THRESHOLD_SAVE_EPSILON: f32 = 0.0005;
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
    let initial_silence_timeout = initial_silence_auto_stop_seconds();
    let persisted_silence_timeout = settings_store::load_silence_auto_stop_seconds();
    let silence_timeout = match persisted_silence_timeout {
        Ok(Some(seconds)) => {
            diagnostics::pipeline_trace::log(
                "settings",
                format!("loaded silence_timeout_seconds={seconds} from persisted settings"),
            );
            seconds
        }
        Ok(None) => initial_silence_timeout,
        Err(error) => {
            diagnostics::pipeline_trace::log("settings", format!("load settings failed: {error}"));
            initial_silence_timeout
        }
    };
    model.borrow_mut().ui_prefs.silence_auto_stop_seconds = silence_timeout;
    let default_gate_threshold =
        crate::ui::atoms::input_level_meter::gate_threshold(&widgets.input_level_meter);
    let gate_threshold = match settings_store::load_silence_gate_threshold() {
        Ok(Some(value)) => {
            let clamped = value.clamp(0.0, 1.0);
            diagnostics::pipeline_trace::log(
                "settings",
                format!("loaded silence_gate_threshold={clamped:.3} from persisted settings"),
            );
            clamped
        }
        Ok(None) => default_gate_threshold,
        Err(error) => {
            diagnostics::pipeline_trace::log(
                "settings",
                format!("load silence_gate_threshold failed: {error}"),
            );
            default_gate_threshold
        }
    };
    crate::ui::atoms::input_level_meter::set_gate_threshold(
        &widgets.input_level_meter,
        gate_threshold,
    );
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

    let selected_model = Arc::new(Mutex::new(TranscriptionModel::default()));

    let command_bus = CommandBus::new(
        event_tx.clone(),
        Arc::clone(&transcriber),
        Arc::clone(&audio_input),
        Arc::clone(&runtime),
        widgets.window.clone(),
        app.clone(),
        layer_shell_backend.as_ref().clone(),
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

    start_input_level_meter_loop(
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
        widgets.settings_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                "settings_button.clicked -> AppEvent::SettingsToggled",
            );
            let _ = event_tx.try_send(AppEvent::SettingsToggled);
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets
            .settings_pane
            .silence_timeout_seconds
            .connect_value_changed(move |spin| {
                let seconds = spin.value().round().clamp(0.0, 600.0) as u64;
                diagnostics::pipeline_trace::log(
                    "ui",
                    format!(
                        "settings.timeout.changed -> AppEvent::SilenceAutoStopSecondsChanged({seconds})"
                    ),
                );
                let _ = event_tx.try_send(AppEvent::SilenceAutoStopSecondsChanged(seconds));
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
        let event_tx = event_tx.clone();
        let key_controller = EventControllerKey::new();
        key_controller.set_propagation_phase(PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, state| {
            let ctrl_down = state.contains(gdk::ModifierType::CONTROL_MASK);
            if ctrl_down && key == gdk::Key::space {
                diagnostics::pipeline_trace::log(
                    "ui",
                    "shortcut Ctrl+Space -> AppEvent::MicToggled",
                );
                let _ = event_tx.try_send(AppEvent::MicToggled);
                return gtk4::glib::Propagation::Stop;
            }

            gtk4::glib::Propagation::Proceed
        });
        widgets.window.add_controller(key_controller);
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

            let before_silence_timeout =
                model_for_events.borrow().ui_prefs.silence_auto_stop_seconds;
            let commands = model_for_events.borrow_mut().reduce(event);
            diagnostics::pipeline_trace::log("event", format!("commands={commands:?}"));
            command_bus_for_events.execute(commands);

            let after_silence_timeout =
                model_for_events.borrow().ui_prefs.silence_auto_stop_seconds;
            if after_silence_timeout != before_silence_timeout {
                match settings_store::save_silence_auto_stop_seconds(after_silence_timeout) {
                    Ok(()) => diagnostics::pipeline_trace::log(
                        "settings",
                        format!("saved silence_timeout_seconds={after_silence_timeout}"),
                    ),
                    Err(error) => diagnostics::pipeline_trace::log(
                        "settings",
                        format!(
                            "failed to save silence_timeout_seconds={after_silence_timeout}: {error}"
                        ),
                    ),
                }
            }

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

fn start_input_level_meter_loop(
    widgets: Widgets,
    model: Rc<RefCell<CoreModel>>,
    audio_input: Arc<voxy_audio::InputEngine>,
    event_tx: mpsc::Sender<AppEvent>,
) {
    let max_recording_duration = max_recording_duration();
    if let Some(duration) = max_recording_duration {
        diagnostics::pipeline_trace::log(
            "guard",
            format!("max_recording_seconds={}", duration.as_secs()),
        );
    } else {
        diagnostics::pipeline_trace::log("guard", "max_recording_seconds=disabled");
    }
    let initial_silence_seconds = model.borrow().ui_prefs.silence_auto_stop_seconds;
    diagnostics::pipeline_trace::log(
        "guard",
        format!("silence_auto_stop_seconds={initial_silence_seconds}"),
    );

    let mut recording_started_at: Option<Instant> = None;
    let mut max_duration_triggered = false;
    let mut below_gate_started_at: Option<Instant> = None;
    let mut above_gate_started_at: Option<Instant> = None;
    let mut silence_duration_triggered = false;
    let mut last_persisted_gate_threshold =
        crate::ui::atoms::input_level_meter::gate_threshold(&widgets.input_level_meter);

    gtk4::glib::timeout_add_local(INPUT_LEVEL_POLL_INTERVAL, move || {
        let (active, silence_timeout_seconds) = {
            let snapshot = model.borrow();
            (
                matches!(snapshot.app_state, AppState::Recording),
                snapshot.ui_prefs.silence_auto_stop_seconds,
            )
        };
        let level = audio_input.latest_input_level();
        let gate_threshold =
            crate::ui::atoms::input_level_meter::gate_threshold(&widgets.input_level_meter);
        if (gate_threshold - last_persisted_gate_threshold).abs() > GATE_THRESHOLD_SAVE_EPSILON {
            match settings_store::save_silence_gate_threshold(gate_threshold) {
                Ok(()) => diagnostics::pipeline_trace::log(
                    "settings",
                    format!("saved silence_gate_threshold={gate_threshold:.3}"),
                ),
                Err(error) => diagnostics::pipeline_trace::log(
                    "settings",
                    format!("failed to save silence_gate_threshold={gate_threshold:.3}: {error}"),
                ),
            }
            last_persisted_gate_threshold = gate_threshold;
        }
        let mut silence_seconds_remaining: Option<u64> = None;

        if active {
            if silence_timeout_seconds > 0 {
                let duration = Duration::from_secs(silence_timeout_seconds);
                let visual_level = crate::ui::atoms::input_level_meter::visual_level(level);
                let gate_release_threshold =
                    (gate_threshold + SILENCE_GATE_RELEASE_HYSTERESIS).clamp(0.0, 1.0);

                if below_gate_started_at.is_none() {
                    if visual_level < gate_threshold {
                        below_gate_started_at = Some(Instant::now());
                        above_gate_started_at = None;
                        silence_duration_triggered = false;
                    }
                } else if visual_level >= gate_release_threshold {
                    if let Some(above_started_at) = above_gate_started_at {
                        if above_started_at.elapsed() >= SILENCE_RESET_DEBOUNCE {
                            below_gate_started_at = None;
                            above_gate_started_at = None;
                            silence_duration_triggered = false;
                        }
                    } else {
                        above_gate_started_at = Some(Instant::now());
                    }
                } else {
                    above_gate_started_at = None;
                }

                if let Some(started_at) = below_gate_started_at {
                    let elapsed = started_at.elapsed();
                    if !silence_duration_triggered && elapsed >= duration {
                        diagnostics::pipeline_trace::log(
                            "guard",
                            format!(
                                "silence_auto_stop_reached={}s -> AppEvent::MicToggled",
                                duration.as_secs()
                            ),
                        );
                        match event_tx.try_send(AppEvent::MicToggled) {
                            Ok(()) => {
                                silence_duration_triggered = true;
                            }
                            Err(error) => {
                                diagnostics::pipeline_trace::log(
                                    "guard",
                                    format!("silence_auto_stop_send_failed={error}"),
                                );
                                silence_seconds_remaining = Some(0);
                            }
                        }
                    } else if !silence_duration_triggered {
                        let remaining = duration.saturating_sub(elapsed);
                        silence_seconds_remaining = Some(remaining.as_secs().max(1));
                    }
                }
            } else {
                below_gate_started_at = None;
                above_gate_started_at = None;
                silence_duration_triggered = false;
            }
        } else {
            below_gate_started_at = None;
            above_gate_started_at = None;
            silence_duration_triggered = false;
        }

        crate::ui::atoms::input_level_meter::render(
            &widgets.input_level_meter,
            level,
            active,
            silence_seconds_remaining,
        );

        if active {
            if recording_started_at.is_none() {
                recording_started_at = Some(Instant::now());
                max_duration_triggered = false;
            }

            if let (Some(duration), Some(started_at)) =
                (max_recording_duration, recording_started_at)
            {
                if !max_duration_triggered && started_at.elapsed() >= duration {
                    diagnostics::pipeline_trace::log(
                        "guard",
                        format!(
                            "max_recording_duration_reached={}s -> AppEvent::MicToggled",
                            duration.as_secs()
                        ),
                    );
                    match event_tx.try_send(AppEvent::MicToggled) {
                        Ok(()) => {
                            max_duration_triggered = true;
                        }
                        Err(error) => {
                            diagnostics::pipeline_trace::log(
                                "guard",
                                format!("max_recording_duration_send_failed={error}"),
                            );
                        }
                    }
                }
            }
        } else {
            recording_started_at = None;
            max_duration_triggered = false;
        }

        gtk4::glib::ControlFlow::Continue
    });
}

fn max_recording_duration() -> Option<Duration> {
    static MAX_RECORDING_SECONDS: OnceLock<u64> = OnceLock::new();
    let seconds = *MAX_RECORDING_SECONDS.get_or_init(|| {
        env::var(MAX_RECORDING_SECONDS_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_RECORDING_SECONDS)
    });

    if seconds == 0 {
        None
    } else {
        Some(Duration::from_secs(seconds))
    }
}

fn initial_silence_auto_stop_seconds() -> u64 {
    static SILENCE_AUTO_STOP_SECONDS: OnceLock<u64> = OnceLock::new();
    *SILENCE_AUTO_STOP_SECONDS.get_or_init(|| {
        env::var(SILENCE_AUTO_STOP_SECONDS_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_SILENCE_AUTO_STOP_SECONDS)
    })
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
