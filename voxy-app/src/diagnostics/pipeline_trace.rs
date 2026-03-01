use std::{
    env,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

const TRACE_ENV: &str = "VOXY_TRACE_PIPELINE";
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

pub fn log(stage: &str, message: impl AsRef<str>) {
    if !trace_enabled() {
        return;
    }

    eprintln!(
        "[voxy:pipe][app][{stage}][t_ms={}] {}",
        unix_time_ms(),
        compact(message.as_ref())
    );
}

fn compact(message: &str) -> String {
    let total = message.chars().count();
    if total <= MAX_TRACE_CHARS {
        return message.to_owned();
    }

    let head: String = message.chars().take(MAX_TRACE_CHARS).collect();
    format!("{head}... [truncated {} chars]", total - MAX_TRACE_CHARS)
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
