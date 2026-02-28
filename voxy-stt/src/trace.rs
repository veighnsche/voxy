use std::{env, sync::OnceLock};

const TRACE_ENV: &str = "VOXY_TRACE_PIPELINE";
const TRACE_NOISY_EVERY_ENV: &str = "VOXY_TRACE_PIPELINE_NOISY_EVERY";
const MAX_TRACE_CHARS: usize = 180;

fn trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        env::var(TRACE_ENV)
            .ok()
            .map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false)
    })
}

pub fn every() -> u64 {
    static EVERY: OnceLock<u64> = OnceLock::new();
    *EVERY.get_or_init(|| {
        env::var("VOXY_TRACE_PIPELINE_EVERY")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(20)
    })
}

pub fn noisy_every() -> u64 {
    static NOISY_EVERY: OnceLock<u64> = OnceLock::new();
    *NOISY_EVERY.get_or_init(|| {
        env::var(TRACE_NOISY_EVERY_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or_else(|| every().saturating_mul(25).max(25))
    })
}

pub fn should_log(seq: u64) -> bool {
    seq % every() == 0
}

pub fn should_log_noisy(seq: u64) -> bool {
    seq <= 3 || seq % noisy_every() == 0
}

pub fn log(stage: &str, message: impl AsRef<str>) {
    if !trace_enabled() {
        return;
    }

    eprintln!("[voxy:pipe][stt][{stage}] {}", compact(message.as_ref()));
}

fn compact(message: &str) -> String {
    let total = message.chars().count();
    if total <= MAX_TRACE_CHARS {
        return message.to_owned();
    }

    let head: String = message.chars().take(MAX_TRACE_CHARS).collect();
    format!("{head}... [truncated {} chars]", total - MAX_TRACE_CHARS)
}
