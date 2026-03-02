use std::{env, sync::OnceLock};

use voxy_core::{
    parse_silence_auto_stop_seconds, parse_vad_silence_ms, CoreModel,
    DEFAULT_SILENCE_GATE_THRESHOLD,
};

use crate::{app::settings_store, diagnostics};

const SILENCE_AUTO_STOP_SECONDS_ENV: &str = "VOXY_SILENCE_AUTO_STOP_SECONDS";
const VAD_SILENCE_MS_ENV: &str = "VOXY_STT_VAD_SILENCE_MS";

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
    let silence_timeout_changed =
        after.silence_auto_stop_seconds != before.silence_auto_stop_seconds;
    let vad_silence_changed = after.vad_silence_duration_ms != before.vad_silence_duration_ms;
    let silence_gate_changed =
        (after.silence_gate_threshold - before.silence_gate_threshold).abs() > f32::EPSILON;

    if !silence_timeout_changed && !vad_silence_changed && !silence_gate_changed {
        return;
    }

    let save_result = settings_store::save_recording_settings(
        after.silence_auto_stop_seconds,
        after.silence_gate_threshold,
        after.vad_silence_duration_ms,
    );

    if let Err(error) = save_result {
        if silence_timeout_changed {
            diagnostics::pipeline_trace::log(
                "settings",
                format!(
                    "failed to save silence_timeout_seconds={}: {error}",
                    after.silence_auto_stop_seconds
                ),
            );
        }
        if vad_silence_changed {
            diagnostics::pipeline_trace::log(
                "settings",
                format!(
                    "failed to save vad_silence_ms={}: {error}",
                    after.vad_silence_duration_ms
                ),
            );
        }
        if silence_gate_changed {
            diagnostics::pipeline_trace::log(
                "settings",
                format!(
                    "failed to save silence_gate_threshold={:.3}: {error}",
                    after.silence_gate_threshold
                ),
            );
        }
        return;
    }

    if silence_timeout_changed {
        diagnostics::pipeline_trace::log(
            "settings",
            format!(
                "saved silence_timeout_seconds={}",
                after.silence_auto_stop_seconds
            ),
        );
    }
    if vad_silence_changed {
        diagnostics::pipeline_trace::log(
            "settings",
            format!("saved vad_silence_ms={}", after.vad_silence_duration_ms),
        );
    }
    if silence_gate_changed {
        diagnostics::pipeline_trace::log(
            "settings",
            format!(
                "saved silence_gate_threshold={:.3}",
                after.silence_gate_threshold
            ),
        );
    }
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
