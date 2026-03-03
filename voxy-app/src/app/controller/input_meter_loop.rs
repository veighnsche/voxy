use std::{
    cell::RefCell,
    env,
    rc::Rc,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use tokio::sync::mpsc;
use voxy_core::{parse_max_recording_seconds, AppEvent, AppState, CoreModel, RecordingStopReason};

use crate::{diagnostics, ui::pages::voxy_window_page::Widgets, wiring::event_emit};

const INPUT_LEVEL_POLL_INTERVAL: Duration = Duration::from_millis(50);
const MAX_RECORDING_SECONDS_ENV: &str = "VOXY_MAX_RECORDING_SECONDS";

pub(super) fn start_input_level_meter_loop(
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

    gtk4::glib::timeout_add_local(INPUT_LEVEL_POLL_INTERVAL, move || {
        let level = audio_input.latest_input_level();
        let (active, silence_timeout_seconds, gate_threshold, decision) = {
            let mut snapshot = model.borrow_mut();
            let active = matches!(snapshot.app_state, AppState::Recording);
            let silence_timeout_seconds = snapshot.ui_prefs.silence_auto_stop_seconds;
            let gate_threshold = snapshot.ui_prefs.silence_gate_threshold;
            let decision = snapshot.evaluate_recording_stop_policy(
                Instant::now(),
                level,
                max_recording_duration,
            );
            (active, silence_timeout_seconds, gate_threshold, decision)
        };

        crate::ui::atoms::input_level_meter::render(
            &widgets.input_level_meter,
            level,
            active,
            decision.silence_seconds_remaining,
            gate_threshold,
        );

        if let Some(reason) = decision.stop_reason {
            match reason {
                RecordingStopReason::SilenceAutoStop => diagnostics::pipeline_trace::log(
                    "guard",
                    format!(
                        "silence_auto_stop_reached={}s -> AppEvent::MicToggled",
                        silence_timeout_seconds
                    ),
                ),
                RecordingStopReason::MaxRecordingDuration => {
                    if let Some(duration) = max_recording_duration {
                        diagnostics::pipeline_trace::log(
                            "guard",
                            format!(
                                "max_recording_duration_reached={}s -> AppEvent::MicToggled",
                                duration.as_secs()
                            ),
                        );
                    }
                }
            }

            event_emit::emit_critical(&event_tx, AppEvent::MicToggled, "input_meter.auto_stop");
        }

        gtk4::glib::ControlFlow::Continue
    });
}

fn max_recording_duration() -> Option<Duration> {
    static MAX_RECORDING_SECONDS: OnceLock<u64> = OnceLock::new();
    let seconds = *MAX_RECORDING_SECONDS.get_or_init(|| {
        let raw = env::var(MAX_RECORDING_SECONDS_ENV).ok();
        parse_max_recording_seconds(raw.as_deref())
    });

    if seconds == 0 {
        None
    } else {
        Some(Duration::from_secs(seconds))
    }
}
