use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use gtk4::{gdk, prelude::*, EventControllerKey, PropagationPhase};
use tokio::sync::mpsc;
use voxy_core::{
    clamp_silence_auto_stop_seconds, clamp_vad_silence_duration_ms, AppEvent, AppState, CoreModel,
    TranscriptionModelId,
};

use crate::{diagnostics, ui::pages::voxy_window_page::Widgets, wiring::event_emit};

pub(super) fn wire_ui_signals(
    widgets: Widgets,
    model: Rc<RefCell<CoreModel>>,
    applying_text_update: Rc<Cell<bool>>,
    event_tx: mpsc::Sender<AppEvent>,
    tray_available: bool,
) {
    {
        let event_tx = event_tx.clone();
        widgets.mic_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log("ui", "mic_button.clicked -> AppEvent::MicToggled");
            event_emit::emit_critical(&event_tx, AppEvent::MicToggled, "ui.mic_button");
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.reset_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                "reset_button.clicked -> AppEvent::ResetRequested",
            );
            event_emit::emit_lossy(&event_tx, AppEvent::ResetRequested, "ui.reset_button");
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.copy_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                "copy_button.clicked -> AppEvent::CopyRequested",
            );
            event_emit::emit_lossy(&event_tx, AppEvent::CopyRequested, "ui.copy_button");
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.model_dropdown.connect_changed(move |dropdown| {
            let Some(model_id) = dropdown.active_id() else {
                return;
            };
            let Some(model) = TranscriptionModelId::from_api_id(model_id.as_str()) else {
                return;
            };
            diagnostics::pipeline_trace::log(
                "ui",
                format!("model_dropdown.changed -> {}", model.as_api_id()),
            );
            event_emit::emit_lossy(
                &event_tx,
                AppEvent::TranscriptionModelChanged(model),
                "ui.model_changed",
            );
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.settings_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                "settings_button.clicked -> AppEvent::SettingsToggled",
            );
            event_emit::emit_lossy(&event_tx, AppEvent::SettingsToggled, "ui.settings_button");
        });
    }

    {
        let event_tx = event_tx.clone();
        let model = Rc::clone(&model);
        let window = widgets.window.clone();
        widgets.error_copy_button.connect_clicked(move |_| {
            let report = {
                let snapshot = model.borrow();
                let message = active_error_message(&snapshot).unwrap_or("unknown");
                build_error_report(&snapshot, message)
            };
            crate::app::behavior::system::clipboard::copy_text_to_clipboard(&window, &report);
            event_emit::emit_lossy(
                &event_tx,
                AppEvent::LogMessage("Error report copied to clipboard".to_owned()),
                "ui.error_copy_button",
            );
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.error_dismiss_button.connect_clicked(move |_| {
            event_emit::emit_lossy(&event_tx, AppEvent::ErrorCleared, "ui.error_dismiss_button");
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets
            .settings_pane
            .silence_timeout_seconds
            .connect_value_changed(move |spin| {
                let seconds = clamp_silence_auto_stop_seconds(spin.value().round().max(0.0) as u64);
                diagnostics::pipeline_trace::log(
                    "ui",
                    format!(
                        "settings.timeout.changed -> AppEvent::SilenceAutoStopSecondsChanged({seconds})"
                    ),
                );
                event_emit::emit_lossy(
                    &event_tx,
                    AppEvent::SilenceAutoStopSecondsChanged(seconds),
                    "ui.silence_timeout_changed",
                );
            });
    }

    {
        let event_tx = event_tx.clone();
        widgets
            .settings_pane
            .vad_silence_ms
            .connect_value_changed(move |spin| {
                let vad_silence_ms =
                    clamp_vad_silence_duration_ms(spin.value().round().max(0.0) as u32);
                diagnostics::pipeline_trace::log(
                    "ui",
                    format!(
                        "settings.vad_silence.changed -> AppEvent::VadSilenceDurationMsChanged({vad_silence_ms})"
                    ),
                );
                event_emit::emit_lossy(
                    &event_tx,
                    AppEvent::VadSilenceDurationMsChanged(vad_silence_ms),
                    "ui.vad_silence_changed",
                );
            });
    }

    {
        let event_tx = event_tx.clone();
        crate::ui::atoms::input_level_meter::connect_gate_threshold_changed(
            &widgets.input_level_meter,
            move |threshold| {
                diagnostics::pipeline_trace::log(
                    "ui",
                    format!(
                        "input_level_meter.gate_threshold.changed -> AppEvent::SilenceGateThresholdChanged({threshold:.3})"
                    ),
                );
                event_emit::emit_lossy(
                    &event_tx,
                    AppEvent::SilenceGateThresholdChanged(threshold),
                    "ui.silence_gate_threshold_changed",
                );
            },
        );
    }

    {
        let event_tx = event_tx.clone();
        let close_action = if tray_available {
            AppEvent::VisibilityToggled
        } else {
            AppEvent::QuitRequested
        };
        let close_action_label = if tray_available {
            "AppEvent::VisibilityToggled"
        } else {
            "AppEvent::QuitRequested (tray unavailable)"
        };
        widgets.close_button.connect_clicked(move |_| {
            diagnostics::pipeline_trace::log(
                "ui",
                format!("close_button.clicked -> {close_action_label}"),
            );
            event_emit::emit_critical(&event_tx, close_action.clone(), "ui.close_button");
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
                event_emit::emit_critical(&event_tx, AppEvent::MicToggled, "ui.ctrl_space");
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

fn active_error_message(model: &CoreModel) -> Option<&str> {
    model.runtime_error.as_deref().or({
        if let AppState::Error(message) = &model.app_state {
            Some(message.as_str())
        } else {
            None
        }
    })
}

fn build_error_report(model: &CoreModel, error_message: &str) -> String {
    let unix_timestamp_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();
    format!(
        "VOXY ERROR REPORT\n\
timestamp_unix_utc={unix_timestamp_s}\n\
platform_os={}\n\
platform_arch={}\n\
app_state={:?}\n\
settings_open={}\n\
silence_timeout_seconds={}\n\
vad_silence_ms={}\n\
silence_gate_threshold={:.3}\n\
window_pos=({}, {})\n\
window_size=({} x {})\n\
confirmed_text_len={}\n\
live_segment_len={}\n\
full_text_len={}\n\
log_line={}\n\
\n\
error_message:\n{}\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        model.app_state,
        model.ui_prefs.settings_open,
        model.ui_prefs.silence_auto_stop_seconds,
        model.ui_prefs.vad_silence_duration_ms,
        model.ui_prefs.silence_gate_threshold,
        model.ui_prefs.window_left,
        model.ui_prefs.window_top,
        model.ui_prefs.window_width,
        model.ui_prefs.window_height,
        model.buffer.confirmed_text.len(),
        model.buffer.live_segment.len(),
        model.buffer.full_text().len(),
        model.log_line,
        error_message
    )
}
