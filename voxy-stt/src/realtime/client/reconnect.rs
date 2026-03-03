use std::{
    env,
    sync::atomic::Ordering,
    sync::OnceLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio_tungstenite::tungstenite::{
    self,
    client::IntoClientRequest,
    http::{header::HeaderValue, Request, StatusCode},
};

use crate::{
    realtime::backoff::{delay_for_attempt_with_jitter, RetryPolicy},
    trace,
};

use super::{
    ReconnectConfig, RetryDecision, RetryPlan, DEFAULT_REALTIME_URL, DEFAULT_RECONNECT_BASE_MS,
    DEFAULT_RECONNECT_ENABLED, DEFAULT_RECONNECT_MAX_MS, DEFAULT_RECONNECT_MAX_RETRIES,
    DEFAULT_SOURCE_POLL_MS, DEFAULT_STOP_FLUSH_TIMEOUT_MS, REALTIME_URL_ENV, RECONNECT_BASE_MS_ENV,
    RECONNECT_ENABLED_ENV, RECONNECT_MAX_MS_ENV, RECONNECT_MAX_RETRIES_ENV, RETRY_JITTER_SEQ,
    RUSTLS_PROVIDER_INIT, SOURCE_POLL_MS_ENV, STOP_FLUSH_TIMEOUT_MS_ENV,
};

pub(super) fn should_retry_tungstenite_error(error: &tungstenite::Error) -> bool {
    match error {
        tungstenite::Error::ConnectionClosed
        | tungstenite::Error::AlreadyClosed
        | tungstenite::Error::Capacity(_)
        | tungstenite::Error::WriteBufferFull(_) => true,
        tungstenite::Error::Io(error) => is_retryable_io_error(error),
        tungstenite::Error::Tls(_) => false,
        tungstenite::Error::Http(response) => is_retryable_http_status(response.status()),
        tungstenite::Error::Protocol(_)
        | tungstenite::Error::Utf8
        | tungstenite::Error::AttackAttempt
        | tungstenite::Error::Url(_)
        | tungstenite::Error::HttpFormat(_) => false,
    }
}

fn is_retryable_io_error(error: &std::io::Error) -> bool {
    use std::io::ErrorKind;

    matches!(
        error.kind(),
        ErrorKind::TimedOut
            | ErrorKind::WouldBlock
            | ErrorKind::Interrupted
            | ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::NotConnected
            | ErrorKind::UnexpectedEof
    )
}

fn is_retryable_http_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

pub(super) fn reconnect_decision(
    reconnect_config: ReconnectConfig,
    retry_attempt: u32,
    message: &str,
) -> RetryDecision {
    if !reconnect_config.enabled {
        return RetryDecision::GiveUp(format!("{message}; reconnect is disabled"));
    }

    if let Some(max_retries) = reconnect_config.max_retries {
        if retry_attempt >= max_retries {
            return RetryDecision::GiveUp(format!(
                "{message}; reconnect attempts exhausted (max_retries={max_retries})"
            ));
        }
    }

    let delay = delay_for_attempt_with_jitter(
        reconnect_config.retry_policy,
        retry_attempt,
        next_retry_jitter_seed(retry_attempt),
    );
    let next_attempt = retry_attempt.saturating_add(1);
    let attempt_label = reconnect_config
        .max_retries
        .map(|limit| format!("{next_attempt}/{limit}"))
        .unwrap_or_else(|| format!("{next_attempt}/∞"));
    RetryDecision::Retry(RetryPlan {
        delay,
        retry_attempt: next_attempt,
        attempt_label,
    })
}

pub(super) fn reconnect_config_from_env() -> ReconnectConfig {
    let enabled = parse_bool_env(RECONNECT_ENABLED_ENV).unwrap_or(DEFAULT_RECONNECT_ENABLED);
    let max_retries = match parse_u32_env(RECONNECT_MAX_RETRIES_ENV) {
        Some(0) => None,
        Some(value) => Some(value),
        None => Some(DEFAULT_RECONNECT_MAX_RETRIES),
    };
    let base_ms = parse_u64_env(RECONNECT_BASE_MS_ENV)
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_RECONNECT_BASE_MS);
    let max_ms = parse_u64_env(RECONNECT_MAX_MS_ENV)
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_RECONNECT_MAX_MS)
        .max(base_ms);

    ReconnectConfig {
        enabled,
        max_retries,
        retry_policy: RetryPolicy {
            base_delay: Duration::from_millis(base_ms),
            max_delay: Duration::from_millis(max_ms),
        },
    }
}

fn next_retry_jitter_seed(retry_attempt: u32) -> u64 {
    let seq = RETRY_JITTER_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    seq ^ now_ns ^ ((retry_attempt as u64) << 32)
}

fn parse_bool_env(name: &str) -> Option<bool> {
    env::var(name).ok().and_then(|raw| {
        let value = raw.trim().to_ascii_lowercase();
        match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    })
}

fn parse_u32_env(name: &str) -> Option<u32> {
    env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())
}

fn parse_u64_env(name: &str) -> Option<u64> {
    env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
}

pub(super) fn stop_flush_timeout_from_env() -> Duration {
    static STOP_FLUSH_TIMEOUT_MS: OnceLock<u64> = OnceLock::new();
    let timeout_ms = *STOP_FLUSH_TIMEOUT_MS.get_or_init(|| {
        parse_u64_env(STOP_FLUSH_TIMEOUT_MS_ENV)
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_STOP_FLUSH_TIMEOUT_MS)
    });

    Duration::from_millis(timeout_ms)
}

pub(super) fn realtime_url_from_env() -> String {
    let raw = env::var(REALTIME_URL_ENV).ok();
    parse_realtime_url(raw.as_deref())
}

pub(super) fn parse_realtime_url(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| DEFAULT_REALTIME_URL.to_owned())
}

pub(super) fn redact_ws_url_for_trace(ws_url: &str) -> String {
    ws_url
        .split_once('?')
        .map(|(base, _)| format!("{base}?<redacted>"))
        .unwrap_or_else(|| ws_url.to_owned())
}

#[allow(clippy::result_large_err)]
pub(super) fn build_request(
    ws_url: &str,
    api_key: &str,
) -> Result<Request<()>, tungstenite::error::Error> {
    let mut request = ws_url.into_client_request()?;
    let auth = HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|error| {
        tungstenite::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid authorization header value: {error}"),
        ))
    })?;

    request.headers_mut().insert("Authorization", auth);
    request
        .headers_mut()
        .insert("OpenAI-Beta", HeaderValue::from_static("realtime=v1"));

    Ok(request)
}

pub(super) fn source_poll_interval_from_env() -> Duration {
    static SOURCE_POLL_MS: OnceLock<u64> = OnceLock::new();
    let poll_ms = *SOURCE_POLL_MS.get_or_init(|| {
        let raw = env::var(SOURCE_POLL_MS_ENV).ok();
        parse_source_poll_ms(raw.as_deref())
    });

    Duration::from_millis(poll_ms)
}

pub(super) fn parse_source_poll_ms(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_SOURCE_POLL_MS)
}

pub(super) fn ensure_rustls_provider() -> Result<(), &'static str> {
    let init = RUSTLS_PROVIDER_INIT.get_or_init(|| {
        if rustls::crypto::CryptoProvider::get_default().is_some() {
            return Ok(());
        }

        rustls::crypto::ring::default_provider()
            .install_default()
            .map_err(|_| "failed to install rustls ring crypto provider".to_owned())?;

        if rustls::crypto::CryptoProvider::get_default().is_some() {
            Ok(())
        } else {
            Err("rustls crypto provider still missing after install".to_owned())
        }
    });

    match init {
        Ok(()) => {
            trace::log("start", "rustls crypto provider ready");
            Ok(())
        }
        Err(_) => Err("failed to install rustls ring crypto provider"),
    }
}
