use std::{
    env,
    sync::{mpsc as std_mpsc, OnceLock},
    thread,
    time::Duration,
};

use tokio::sync::mpsc as tokio_mpsc;
use voxy_core::{
    parse_silence_auto_stop_seconds, parse_vad_silence_ms, AppEvent, CoreModel,
    DEFAULT_SILENCE_GATE_THRESHOLD,
};

use crate::{app::settings_store, diagnostics};

const SILENCE_AUTO_STOP_SECONDS_ENV: &str = "VOXY_SILENCE_AUTO_STOP_SECONDS";
const VAD_SILENCE_MS_ENV: &str = "VOXY_STT_VAD_SILENCE_MS";
const SETTINGS_PERSIST_DEBOUNCE: Duration = Duration::from_millis(150);
static SETTINGS_PERSIST_TX: OnceLock<std_mpsc::Sender<SettingsSnapshot>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub(super) struct SettingsSnapshot {
    silence_auto_stop_seconds: u64,
    vad_silence_duration_ms: u32,
    silence_gate_threshold: f32,
}

impl SettingsSnapshot {
    pub(super) fn from_model(model: &CoreModel) -> Self {
        Self {
            silence_auto_stop_seconds: model.ui_prefs.silence_auto_stop_seconds,
            vad_silence_duration_ms: model.ui_prefs.vad_silence_duration_ms,
            silence_gate_threshold: model.ui_prefs.silence_gate_threshold,
        }
    }

    pub(super) fn apply_to_model(self, model: &mut CoreModel) {
        model.ui_prefs.silence_auto_stop_seconds = self.silence_auto_stop_seconds;
        model.ui_prefs.vad_silence_duration_ms = self.vad_silence_duration_ms;
        model.ui_prefs.silence_gate_threshold = self.silence_gate_threshold;
    }
}

pub(super) fn initialize_persist_worker(event_tx: tokio_mpsc::Sender<AppEvent>) {
    SETTINGS_PERSIST_TX.get_or_init(|| spawn_persist_worker(event_tx));
}

pub(super) fn load_startup_settings() -> SettingsSnapshot {
    let fallback_silence_timeout = initial_silence_auto_stop_seconds();
    let silence_auto_stop_seconds = match settings_store::load_silence_auto_stop_seconds() {
        Ok(Some(seconds)) => {
            diagnostics::pipeline_trace::log(
                "settings",
                format!("loaded silence_timeout_seconds={seconds} from persisted settings"),
            );
            seconds
        }
        Ok(None) => fallback_silence_timeout,
        Err(error) => {
            diagnostics::pipeline_trace::log(
                "settings",
                format!("load silence_timeout_seconds failed: {error}"),
            );
            fallback_silence_timeout
        }
    };

    let fallback_vad_silence_duration_ms = initial_vad_silence_ms();
    let vad_silence_duration_ms = match settings_store::load_vad_silence_ms() {
        Ok(Some(value)) => {
            diagnostics::pipeline_trace::log("settings", format!("loaded vad_silence_ms={value}"));
            value
        }
        Ok(None) => fallback_vad_silence_duration_ms,
        Err(error) => {
            diagnostics::pipeline_trace::log(
                "settings",
                format!("load vad_silence_ms failed: {error}"),
            );
            fallback_vad_silence_duration_ms
        }
    };

    let silence_gate_threshold = match settings_store::load_silence_gate_threshold() {
        Ok(Some(value)) => {
            diagnostics::pipeline_trace::log(
                "settings",
                format!("loaded silence_gate_threshold={value:.3} from persisted settings"),
            );
            value
        }
        Ok(None) => DEFAULT_SILENCE_GATE_THRESHOLD,
        Err(error) => {
            diagnostics::pipeline_trace::log(
                "settings",
                format!("load silence_gate_threshold failed: {error}"),
            );
            DEFAULT_SILENCE_GATE_THRESHOLD
        }
    };

    SettingsSnapshot {
        silence_auto_stop_seconds,
        vad_silence_duration_ms,
        silence_gate_threshold,
    }
}

pub(super) fn persist_changed_settings(before: SettingsSnapshot, after: SettingsSnapshot) {
    if !did_settings_change(before, after) {
        return;
    }

    if let Some(persist_tx) = SETTINGS_PERSIST_TX.get() {
        if persist_tx.send(after).is_ok() {
            diagnostics::pipeline_trace::log("settings", "persist queued");
            return;
        }
        diagnostics::pipeline_trace::log(
            "settings",
            "persist queue send failed; falling back to synchronous save",
        );
    } else {
        diagnostics::pipeline_trace::log(
            "settings",
            "persist worker not initialized; falling back to synchronous save",
        );
    }

    if let Err(error) = persist_now(after) {
        diagnostics::pipeline_trace::log(
            "settings",
            format!("synchronous persist failed: {error}"),
        );
    }
}

fn spawn_persist_worker(
    event_tx: tokio_mpsc::Sender<AppEvent>,
) -> std_mpsc::Sender<SettingsSnapshot> {
    let (persist_tx, persist_rx) = std_mpsc::channel::<SettingsSnapshot>();
    let spawn_result = thread::Builder::new()
        .name("voxy-settings-persist".to_owned())
        .spawn(move || run_persist_worker(persist_rx, event_tx));

    if let Err(error) = spawn_result {
        diagnostics::pipeline_trace::log(
            "settings",
            format!("failed to spawn persist worker thread: {error}"),
        );
    }

    persist_tx
}

fn run_persist_worker(
    persist_rx: std_mpsc::Receiver<SettingsSnapshot>,
    event_tx: tokio_mpsc::Sender<AppEvent>,
) {
    while let Ok(snapshot) = persist_rx.recv() {
        let mut latest = snapshot;

        loop {
            match persist_rx.recv_timeout(SETTINGS_PERSIST_DEBOUNCE) {
                Ok(snapshot) => latest = snapshot,
                Err(std_mpsc::RecvTimeoutError::Timeout) => break,
                Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = persist_now(latest);
                    return;
                }
            }
        }

        if let Err(error) = persist_now(latest) {
            diagnostics::pipeline_trace::log("settings", format!("persist failed: {error}"));
            let _ = event_tx.blocking_send(AppEvent::RuntimeError(format!(
                "Failed to persist settings: {error}"
            )));
        }
    }
}

fn persist_now(snapshot: SettingsSnapshot) -> Result<(), String> {
    settings_store::save_recording_settings(
        snapshot.silence_auto_stop_seconds,
        snapshot.silence_gate_threshold,
        snapshot.vad_silence_duration_ms,
    )?;

    diagnostics::pipeline_trace::log(
        "settings",
        format!(
            "saved settings silence_timeout_seconds={} vad_silence_ms={} silence_gate_threshold={:.3}",
            snapshot.silence_auto_stop_seconds,
            snapshot.vad_silence_duration_ms,
            snapshot.silence_gate_threshold
        ),
    );
    Ok(())
}

fn did_settings_change(before: SettingsSnapshot, after: SettingsSnapshot) -> bool {
    let silence_timeout_changed =
        after.silence_auto_stop_seconds != before.silence_auto_stop_seconds;
    let vad_silence_changed = after.vad_silence_duration_ms != before.vad_silence_duration_ms;
    let silence_gate_changed =
        (after.silence_gate_threshold - before.silence_gate_threshold).abs() > f32::EPSILON;
    silence_timeout_changed || vad_silence_changed || silence_gate_changed
}

fn initial_silence_auto_stop_seconds() -> u64 {
    static SILENCE_AUTO_STOP_SECONDS: OnceLock<u64> = OnceLock::new();
    *SILENCE_AUTO_STOP_SECONDS.get_or_init(|| {
        let raw = env::var(SILENCE_AUTO_STOP_SECONDS_ENV).ok();
        parse_silence_auto_stop_seconds(raw.as_deref())
    })
}

fn initial_vad_silence_ms() -> u32 {
    static VAD_SILENCE_MS: OnceLock<u32> = OnceLock::new();
    *VAD_SILENCE_MS.get_or_init(|| {
        let raw = env::var(VAD_SILENCE_MS_ENV).ok();
        parse_vad_silence_ms(raw.as_deref())
    })
}
