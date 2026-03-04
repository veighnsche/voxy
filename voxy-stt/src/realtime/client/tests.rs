use std::{sync::Mutex, time::Duration};

use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::mpsc, time};
use tokio_tungstenite::{
    accept_async, accept_hdr_async,
    tungstenite::{self, http::Response},
    WebSocketStream,
};
use voxy_core::{AppEvent, TranscriptionModelId};

use super::{
    completion_matches_expected, handle_server_payload, observe_stop_flush_progress,
    parse_realtime_url, parse_source_poll_ms, reconnect_config_from_env, reconnect_decision,
    redact_ws_url_for_trace, should_forward_server_event_to_app, should_retry_tungstenite_error,
    OpenAiRealtimeTranscriber, ReconnectConfig, RetryDecision, DEFAULT_REALTIME_URL,
    DEFAULT_RECONNECT_BASE_MS, DEFAULT_RECONNECT_ENABLED, DEFAULT_RECONNECT_MAX_MS,
    DEFAULT_RECONNECT_MAX_RETRIES, DEFAULT_SOURCE_POLL_MS, REALTIME_URL_ENV, RECONNECT_BASE_MS_ENV,
    RECONNECT_ENABLED_ENV, RECONNECT_MAX_MS_ENV, RECONNECT_MAX_RETRIES_ENV,
    STOP_FLUSH_TIMEOUT_MS_ENV,
};
use crate::config::VOXY_OPENAI_API_KEY_ENV;
use crate::realtime::protocol::server_event::ServerEvent;
use crate::traits::{
    StreamingTranscriber, TranscriberInput, TranscriberOutput, TranscriberSessionConfig,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());
type MockSocket = WebSocketStream<tokio::net::TcpStream>;

struct EnvVarGuard {
    key: String,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self {
            key: key.to_owned(),
            previous,
        }
    }

    fn unset(key: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        Self {
            key: key.to_owned(),
            previous,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(&self.key, value) },
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
}

fn reconnect_defaults() -> ReconnectConfig {
    let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
    let _enabled = EnvVarGuard::unset(RECONNECT_ENABLED_ENV);
    let _max_retries = EnvVarGuard::unset(RECONNECT_MAX_RETRIES_ENV);
    let _base = EnvVarGuard::unset(RECONNECT_BASE_MS_ENV);
    let _max = EnvVarGuard::unset(RECONNECT_MAX_MS_ENV);
    reconnect_config_from_env()
}

fn message_to_json(message: tungstenite::Message) -> Option<Value> {
    match message {
        tungstenite::Message::Text(text) => serde_json::from_str(text.as_ref()).ok(),
        tungstenite::Message::Binary(bytes) => serde_json::from_slice(bytes.as_ref()).ok(),
        tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => None,
        tungstenite::Message::Close(_) => None,
        tungstenite::Message::Frame(_) => None,
    }
}

async fn receive_client_event(socket: &mut MockSocket, expected_type: &str) -> Value {
    loop {
        let message = match time::timeout(Duration::from_secs(3), socket.next()).await {
            Ok(Some(Ok(message))) => message,
            Ok(Some(Err(error))) => {
                panic!("mock websocket read failed while waiting for {expected_type}: {error}")
            }
            Ok(None) => panic!("mock websocket closed before receiving {expected_type}"),
            Err(_) => panic!("timed out waiting for client event {expected_type}"),
        };

        if matches!(message, tungstenite::Message::Close(_)) {
            panic!("client closed socket before sending {expected_type}");
        }

        if let Some(value) = message_to_json(message) {
            let event_type = value.get("type").and_then(Value::as_str);
            if event_type == Some(expected_type) {
                return value;
            }
        }
    }
}

async fn try_receive_client_event(
    socket: &mut MockSocket,
    expected_type: &str,
    timeout: Duration,
) -> Option<Value> {
    let deadline = time::Instant::now() + timeout;
    while time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        let next = match time::timeout(remaining, socket.next()).await {
            Ok(next) => next,
            Err(_) => return None,
        };

        let message = match next {
            Some(Ok(message)) => message,
            Some(Err(_)) | None => return None,
        };

        if matches!(message, tungstenite::Message::Close(_)) {
            return None;
        }

        if let Some(value) = message_to_json(message) {
            let event_type = value.get("type").and_then(Value::as_str);
            if event_type == Some(expected_type) {
                return Some(value);
            }
        }
    }

    None
}

async fn send_server_event(socket: &mut MockSocket, payload: Value) {
    socket
        .send(tungstenite::Message::Text(payload.to_string()))
        .await
        .expect("mock websocket should send server payload");
}

#[allow(clippy::result_large_err)]
fn reject_handshake_with_retryable_error(
    _request: &tungstenite::handshake::server::Request,
    _response: tungstenite::handshake::server::Response,
) -> Result<tungstenite::handshake::server::Response, tungstenite::handshake::server::ErrorResponse>
{
    let rejection = Response::builder()
        .status(500)
        .body(Some("retryable mock failure".to_owned()))
        .expect("mock rejection response should build");
    Err(rejection)
}

async fn run_reconnect_stop_flush_mock_server(listener: TcpListener, expected_model: &str) {
    let (first_stream, _) = time::timeout(Duration::from_secs(3), listener.accept())
        .await
        .expect("timed out waiting for first websocket client connection")
        .expect("first websocket client should connect");

    let first_handshake =
        accept_hdr_async(first_stream, reject_handshake_with_retryable_error).await;
    match first_handshake {
        Err(tungstenite::Error::Http(response)) => {
            assert_eq!(
                response.status().as_u16(),
                500,
                "first connect should fail with retryable HTTP status"
            );
        }
        Err(error) => panic!("expected HTTP handshake rejection, got: {error}"),
        Ok(_) => panic!("first connect unexpectedly succeeded"),
    }

    let (stream, _) = time::timeout(Duration::from_secs(10), listener.accept())
        .await
        .expect("timed out waiting for reconnect websocket client connection")
        .expect("reconnect websocket client should connect");
    let mut socket = accept_async(stream)
        .await
        .expect("reconnect websocket handshake should succeed");
    let first_update = receive_client_event(&mut socket, "transcription_session.update").await;
    assert_eq!(
        first_update
            .get("session")
            .and_then(|session| session.get("input_audio_transcription"))
            .and_then(|input| input.get("model"))
            .and_then(Value::as_str),
        Some(expected_model),
        "reconnect should preserve negotiated model",
    );

    if try_receive_client_event(
        &mut socket,
        "input_audio_buffer.commit",
        Duration::from_secs(10),
    )
    .await
    .is_none()
    {
        panic!("did not observe commit after reconnect");
    }

    send_server_event(
        &mut socket,
        json!({
            "type": "conversation.item.input_audio_transcription.completed",
            "item_id": "item-stale",
            "transcript": "stale"
        }),
    )
    .await;

    send_server_event(
        &mut socket,
        json!({
            "type": "input_audio_buffer.committed",
            "item_id": "item-fresh",
            "previous_item_id": "item-stale"
        }),
    )
    .await;
    send_server_event(
        &mut socket,
        json!({
            "type": "conversation.item.input_audio_transcription.completed",
            "item_id": "item-fresh",
            "transcript": "final"
        }),
    )
    .await;

    let _ = time::timeout(Duration::from_secs(10), socket.next()).await;
}

async fn run_sanity_check_mock_server(listener: TcpListener, expected_model: &str) {
    let (stream, _) = time::timeout(Duration::from_secs(3), listener.accept())
        .await
        .expect("timed out waiting for websocket client connection")
        .expect("websocket client should connect");
    let mut socket = accept_async(stream)
        .await
        .expect("websocket handshake should succeed");

    let session_update = receive_client_event(&mut socket, "transcription_session.update").await;
    assert_eq!(
        session_update
            .get("session")
            .and_then(|session| session.get("input_audio_transcription"))
            .and_then(|input| input.get("model"))
            .and_then(Value::as_str),
        Some(expected_model),
        "start should configure the expected model",
    );

    if try_receive_client_event(
        &mut socket,
        "input_audio_buffer.commit",
        Duration::from_secs(10),
    )
    .await
    .is_some()
    {
        send_server_event(
            &mut socket,
            json!({
                "type": "input_audio_buffer.committed",
                "item_id": "item-stop",
                "previous_item_id": null
            }),
        )
        .await;
        send_server_event(
            &mut socket,
            json!({
                "type": "conversation.item.input_audio_transcription.completed",
                "item_id": "item-stop",
                "transcript": ""
            }),
        )
        .await;
    }

    let _ = time::timeout(Duration::from_secs(10), socket.next()).await;
}

#[test]
fn retry_classifier_marks_server_and_rate_limit_http_as_retryable() {
    let response_500 = Response::builder()
        .status(500)
        .body(None)
        .expect("response should build");
    let response_429 = Response::builder()
        .status(429)
        .body(None)
        .expect("response should build");
    let response_401 = Response::builder()
        .status(401)
        .body(None)
        .expect("response should build");

    assert!(should_retry_tungstenite_error(&tungstenite::Error::Http(
        response_500
    )));
    assert!(should_retry_tungstenite_error(&tungstenite::Error::Http(
        response_429
    )));
    assert!(!should_retry_tungstenite_error(&tungstenite::Error::Http(
        response_401
    )));
}

#[test]
fn retry_classifier_marks_io_errors_as_retryable() {
    let io_error = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset");
    assert!(should_retry_tungstenite_error(&tungstenite::Error::Io(
        io_error
    )));
}

#[test]
fn retry_classifier_marks_permanent_io_errors_as_non_retryable() {
    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    assert!(!should_retry_tungstenite_error(&tungstenite::Error::Io(
        io_error
    )));
}

#[test]
fn retry_classifier_marks_connection_closed_as_retryable() {
    assert!(should_retry_tungstenite_error(
        &tungstenite::Error::ConnectionClosed
    ));
}

#[test]
fn retry_classifier_marks_protocol_errors_as_non_retryable() {
    assert!(!should_retry_tungstenite_error(
        &tungstenite::Error::Protocol(
            tungstenite::error::ProtocolError::ResetWithoutClosingHandshake
        ),
    ));
}

#[test]
fn retry_classifier_marks_tls_errors_as_non_retryable() {
    let tls_error = rustls::Error::General("handshake failed".to_owned());
    assert!(!should_retry_tungstenite_error(&tungstenite::Error::Tls(
        tls_error.into(),
    )));
}

#[test]
fn reconnect_config_uses_defaults_when_env_is_unset() {
    let config = reconnect_defaults();
    assert_eq!(config.enabled, DEFAULT_RECONNECT_ENABLED);
    assert_eq!(config.max_retries, Some(DEFAULT_RECONNECT_MAX_RETRIES));
    assert_eq!(
        config.retry_policy.base_delay,
        Duration::from_millis(DEFAULT_RECONNECT_BASE_MS)
    );
    assert_eq!(
        config.retry_policy.max_delay,
        Duration::from_millis(DEFAULT_RECONNECT_MAX_MS)
    );
}

#[test]
fn reconnect_config_applies_env_overrides() {
    let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
    let _enabled = EnvVarGuard::set(RECONNECT_ENABLED_ENV, "false");
    let _max_retries = EnvVarGuard::set(RECONNECT_MAX_RETRIES_ENV, "3");
    let _base = EnvVarGuard::set(RECONNECT_BASE_MS_ENV, "700");
    let _max = EnvVarGuard::set(RECONNECT_MAX_MS_ENV, "2000");

    let config = reconnect_config_from_env();

    assert!(!config.enabled);
    assert_eq!(config.max_retries, Some(3));
    assert_eq!(config.retry_policy.base_delay, Duration::from_millis(700));
    assert_eq!(config.retry_policy.max_delay, Duration::from_millis(2000));
}

#[test]
fn reconnect_config_treats_zero_retry_limit_as_unlimited() {
    let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
    let _max_retries = EnvVarGuard::set(RECONNECT_MAX_RETRIES_ENV, "0");

    let config = reconnect_config_from_env();
    assert_eq!(config.max_retries, None);
}

#[test]
fn parse_realtime_url_uses_default_when_value_is_missing_or_blank() {
    assert_eq!(parse_realtime_url(None), DEFAULT_REALTIME_URL);
    assert_eq!(parse_realtime_url(Some("   ")), DEFAULT_REALTIME_URL);
}

#[test]
fn parse_realtime_url_trims_and_preserves_valid_value() {
    let url = "wss://example.test/realtime";
    assert_eq!(parse_realtime_url(Some(url)), url);
    assert_eq!(
        parse_realtime_url(Some("  wss://example.test/x  ")),
        "wss://example.test/x"
    );
}

#[test]
fn redact_ws_url_for_trace_strips_query_string() {
    assert_eq!(
        redact_ws_url_for_trace("wss://example.test/realtime?intent=transcription&token=abc"),
        "wss://example.test/realtime?<redacted>"
    );
    assert_eq!(
        redact_ws_url_for_trace("wss://example.test/realtime"),
        "wss://example.test/realtime"
    );
}

#[test]
fn parse_source_poll_ms_falls_back_to_default_for_invalid_values() {
    assert_eq!(parse_source_poll_ms(None), DEFAULT_SOURCE_POLL_MS);
    assert_eq!(parse_source_poll_ms(Some("bad")), DEFAULT_SOURCE_POLL_MS);
    assert_eq!(parse_source_poll_ms(Some("0")), DEFAULT_SOURCE_POLL_MS);
}

#[test]
fn parse_source_poll_ms_accepts_positive_value() {
    assert_eq!(parse_source_poll_ms(Some("15")), 15);
    assert_eq!(parse_source_poll_ms(Some(" 30 ")), 30);
}

#[test]
fn reconnect_decision_gives_up_when_disabled() {
    let config = ReconnectConfig {
        enabled: false,
        max_retries: Some(5),
        retry_policy: Default::default(),
    };

    assert_eq!(
        reconnect_decision(config, 0, "socket closed"),
        RetryDecision::GiveUp("socket closed; reconnect is disabled".to_owned())
    );
}

#[test]
fn reconnect_decision_gives_up_when_retry_limit_is_exhausted() {
    let config = ReconnectConfig {
        enabled: true,
        max_retries: Some(2),
        retry_policy: Default::default(),
    };

    assert_eq!(
        reconnect_decision(config, 2, "socket closed"),
        RetryDecision::GiveUp(
            "socket closed; reconnect attempts exhausted (max_retries=2)".to_owned()
        )
    );
}

#[test]
fn reconnect_decision_returns_retry_plan_when_allowed() {
    let config = ReconnectConfig {
        enabled: true,
        max_retries: Some(3),
        retry_policy: super::RetryPolicy {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1_000),
        },
    };

    let decision = reconnect_decision(config, 1, "socket closed");
    match decision {
        RetryDecision::Retry(plan) => {
            assert!(plan.delay >= Duration::from_millis(100));
            assert!(plan.delay <= Duration::from_millis(200));
            assert_eq!(plan.retry_attempt, 2);
            assert_eq!(plan.attempt_label, "2/3");
        }
        RetryDecision::GiveUp(message) => {
            panic!("expected retry decision, got give up: {message}");
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn completion_payload_maps_commit_when_item_id_matches() {
    let (event_tx, mut event_rx) = mpsc::channel(8);
    let (downlink_tx, mut downlink_rx) = tokio::sync::broadcast::channel(8);

    let payload = json!({
        "type": "conversation.item.input_audio_transcription.completed",
        "item_id": "item-a",
        "transcript": "final"
    })
    .to_string();

    let parsed =
        handle_server_payload(&event_tx, &downlink_tx, &payload, true, Some("item-a")).await;
    assert!(matches!(
        parsed,
        Some(ServerEvent::TranscriptionCompleted { .. })
    ));

    let app_event = time::timeout(Duration::from_secs(1), event_rx.recv())
        .await
        .expect("app event should be emitted")
        .expect("event channel should stay open");
    assert_eq!(app_event, AppEvent::CommitRequested);

    let downlink_event = time::timeout(Duration::from_secs(1), downlink_rx.recv())
        .await
        .expect("downlink event should be emitted")
        .expect("downlink receiver should stay open");
    assert_eq!(downlink_event, TranscriberOutput::SegmentCommitted);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_payload_is_ignored_when_item_id_is_mismatched() {
    let (event_tx, mut event_rx) = mpsc::channel(8);
    let (downlink_tx, mut downlink_rx) = tokio::sync::broadcast::channel(8);

    let payload = json!({
        "type": "conversation.item.input_audio_transcription.completed",
        "item_id": "item-b",
        "transcript": "stale"
    })
    .to_string();

    let parsed =
        handle_server_payload(&event_tx, &downlink_tx, &payload, true, Some("item-a")).await;
    assert!(matches!(
        parsed,
        Some(ServerEvent::TranscriptionCompleted { .. })
    ));

    assert!(
        time::timeout(Duration::from_millis(120), event_rx.recv())
            .await
            .is_err(),
        "mismatched completion should not emit AppEvent::CommitRequested"
    );
    assert!(
        time::timeout(Duration::from_millis(120), downlink_rx.recv())
            .await
            .is_err(),
        "mismatched completion should not emit SegmentCommitted"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn stop_pending_ignores_benign_empty_buffer_commit_error() {
    let (event_tx, mut event_rx) = mpsc::channel(8);
    let (downlink_tx, mut downlink_rx) = tokio::sync::broadcast::channel(8);

    let payload = json!({
        "type": "error",
        "error": {
            "message": "Error committing input audio buffer: buffer too small. Expected at least 100ms of audio, but buffer only has 0.00ms of audio."
        }
    })
    .to_string();

    let parsed = handle_server_payload(&event_tx, &downlink_tx, &payload, true, None).await;
    assert!(matches!(parsed, Some(ServerEvent::Error { .. })));
    assert!(
        time::timeout(Duration::from_millis(120), event_rx.recv())
            .await
            .is_err(),
        "benign stop-commit empty-buffer errors should not emit AppEvent::RuntimeError"
    );
    assert!(
        time::timeout(Duration::from_millis(120), downlink_rx.recv())
            .await
            .is_err(),
        "benign stop-commit empty-buffer errors should not emit downlink errors"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn stop_pending_ignores_benign_empty_buffer_commit_error_variant() {
    let (event_tx, mut event_rx) = mpsc::channel(8);
    let (downlink_tx, mut downlink_rx) = tokio::sync::broadcast::channel(8);

    let payload = json!({
        "type": "error",
        "error": {
            "message": "Error committing input audio buffer: buffer too small. Expected at least 80ms of audio, but buffer only has no audio."
        }
    })
    .to_string();

    let parsed = handle_server_payload(&event_tx, &downlink_tx, &payload, true, None).await;
    assert!(matches!(parsed, Some(ServerEvent::Error { .. })));
    assert!(
        time::timeout(Duration::from_millis(120), event_rx.recv())
            .await
            .is_err(),
        "benign stop-commit empty-buffer variants should not emit AppEvent::RuntimeError"
    );
    assert!(
        time::timeout(Duration::from_millis(120), downlink_rx.recv())
            .await
            .is_err(),
        "benign stop-commit empty-buffer variants should not emit downlink errors"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn non_stop_error_payload_still_surfaces_runtime_error() {
    let (event_tx, mut event_rx) = mpsc::channel(8);
    let (downlink_tx, mut downlink_rx) = tokio::sync::broadcast::channel(8);

    let payload = json!({
        "type": "error",
        "error": {
            "message": "Error committing input audio buffer: buffer too small. Expected at least 100ms of audio, but buffer only has 0.00ms of audio."
        }
    })
    .to_string();

    let parsed = handle_server_payload(&event_tx, &downlink_tx, &payload, false, None).await;
    assert!(matches!(parsed, Some(ServerEvent::Error { .. })));

    let app_event = time::timeout(Duration::from_secs(1), event_rx.recv())
        .await
        .expect("app event should be emitted")
        .expect("event channel should stay open");
    assert_eq!(
        app_event,
        AppEvent::RuntimeError(
            "Error committing input audio buffer: buffer too small. Expected at least 100ms of audio, but buffer only has 0.00ms of audio.".to_owned()
        )
    );

    let downlink_event = time::timeout(Duration::from_secs(1), downlink_rx.recv())
        .await
        .expect("downlink error should be emitted")
        .expect("downlink receiver should stay open");
    assert_eq!(
        downlink_event,
        TranscriberOutput::Error(
            "Error committing input audio buffer: buffer too small. Expected at least 100ms of audio, but buffer only has 0.00ms of audio.".to_owned()
        )
    );
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "current_thread")]
async fn stop_flush_survives_reconnect_and_ignores_stale_completion_integration() {
    let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock websocket listener should bind");
    let ws_url = format!(
        "ws://{}",
        listener
            .local_addr()
            .expect("listener should have local address")
    );
    let _api_key = EnvVarGuard::set(VOXY_OPENAI_API_KEY_ENV, "sk-test");
    let _realtime_url = EnvVarGuard::set(REALTIME_URL_ENV, &ws_url);
    let _reconnect_enabled = EnvVarGuard::set(RECONNECT_ENABLED_ENV, "true");
    let _reconnect_max_retries = EnvVarGuard::set(RECONNECT_MAX_RETRIES_ENV, "3");
    let _reconnect_base_ms = EnvVarGuard::set(RECONNECT_BASE_MS_ENV, "10");
    let _reconnect_max_ms = EnvVarGuard::set(RECONNECT_MAX_MS_ENV, "10");
    let _stop_flush_timeout = EnvVarGuard::set(STOP_FLUSH_TIMEOUT_MS_ENV, "2000");

    let (event_tx, mut event_rx) = mpsc::channel(64);
    let transcriber = OpenAiRealtimeTranscriber::with_source_poll_interval(
        event_tx,
        None,
        Duration::from_millis(10),
    );
    let mut downlink_rx = transcriber.subscribe();
    let config = TranscriberSessionConfig::from_model(TranscriptionModelId::Gpt4oMiniTranscribe);

    let server = tokio::spawn(run_reconnect_stop_flush_mock_server(
        listener,
        config.model.as_api_id(),
    ));

    time::timeout(Duration::from_secs(3), transcriber.start(config.clone()))
        .await
        .expect("transcriber start should not time out")
        .expect("transcriber start should succeed");

    let mut session_started_count = 0usize;
    while session_started_count < 1 {
        let downlink_event = time::timeout(Duration::from_secs(3), downlink_rx.recv())
            .await
            .expect("expected session started event before timeout")
            .expect("downlink channel should stay open");
        match downlink_event {
            TranscriberOutput::SessionStarted(session_config) => {
                assert_eq!(
                    session_config, config,
                    "reconnect should keep session config unchanged"
                );
                session_started_count += 1;
            }
            TranscriberOutput::Error(message) => {
                panic!("unexpected downlink error before commit: {message}");
            }
            _ => {}
        }
    }

    time::timeout(
        Duration::from_secs(3),
        transcriber.push_input(TranscriberInput::Commit),
    )
    .await
    .expect("push_input(commit) should not time out")
    .expect("push_input(commit) should succeed");

    let mut segment_committed_count = 0usize;
    let mut downlink_errors = Vec::new();
    let collect_commit_deadline = time::Instant::now() + Duration::from_secs(3);
    while time::Instant::now() < collect_commit_deadline {
        let remaining = collect_commit_deadline.saturating_duration_since(time::Instant::now());
        match time::timeout(remaining, downlink_rx.recv()).await {
            Ok(Ok(TranscriberOutput::SegmentCommitted)) => {
                segment_committed_count += 1;
                break;
            }
            Ok(Ok(TranscriberOutput::Error(message))) => {
                downlink_errors.push(message);
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) => break,
            Err(_) => break,
        }
    }
    assert_eq!(
        segment_committed_count, 1,
        "only the fresh completion should commit a segment"
    );
    assert!(
        downlink_errors.is_empty(),
        "unexpected downlink errors before stop: {downlink_errors:?}"
    );

    let mut commit_requested_before_stop = 0usize;
    let mut runtime_errors_before_stop = Vec::new();
    let collect_pre_stop_app_deadline = time::Instant::now() + Duration::from_millis(600);
    while time::Instant::now() < collect_pre_stop_app_deadline {
        match time::timeout(Duration::from_millis(75), event_rx.recv()).await {
            Ok(Some(AppEvent::CommitRequested)) => {
                commit_requested_before_stop += 1;
            }
            Ok(Some(AppEvent::RuntimeError(message))) => {
                runtime_errors_before_stop.push(message);
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => break,
        }
    }
    assert!(
        commit_requested_before_stop >= 1,
        "expected at least one CommitRequested after reconnect commit flow"
    );
    assert!(
        runtime_errors_before_stop.is_empty(),
        "unexpected runtime errors before stop: {runtime_errors_before_stop:?}"
    );

    time::timeout(Duration::from_secs(3), transcriber.stop())
        .await
        .expect("transcriber stop should not time out")
        .expect("transcriber stop should succeed");
    server
        .await
        .expect("mock websocket server task should complete");

    let mut saw_session_stopped = false;
    let collect_downlink_deadline = time::Instant::now() + Duration::from_secs(2);
    while time::Instant::now() < collect_downlink_deadline {
        let remaining = collect_downlink_deadline.saturating_duration_since(time::Instant::now());
        match time::timeout(remaining, downlink_rx.recv()).await {
            Ok(Ok(TranscriberOutput::SessionStopped)) => {
                saw_session_stopped = true;
                break;
            }
            Ok(Ok(TranscriberOutput::Error(_))) => {}
            Ok(Ok(_)) => {}
            Ok(Err(_)) => break,
            Err(_) => break,
        }
    }
    assert!(
        saw_session_stopped,
        "session should emit SessionStopped after stop"
    );
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "current_thread")]
async fn start_emits_api_key_sanity_messages() {
    let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock websocket listener should bind");
    let ws_url = format!(
        "ws://{}",
        listener
            .local_addr()
            .expect("listener should have local address")
    );
    let _api_key = EnvVarGuard::set(VOXY_OPENAI_API_KEY_ENV, "sk-test");
    let _realtime_url = EnvVarGuard::set(REALTIME_URL_ENV, &ws_url);

    let (event_tx, mut event_rx) = mpsc::channel(64);
    let transcriber = OpenAiRealtimeTranscriber::with_source_poll_interval(
        event_tx,
        None,
        Duration::from_millis(10),
    );
    let config = TranscriberSessionConfig::from_model(TranscriptionModelId::Gpt4oMiniTranscribe);

    let server = tokio::spawn(run_sanity_check_mock_server(
        listener,
        config.model.as_api_id(),
    ));

    time::timeout(Duration::from_secs(3), transcriber.start(config))
        .await
        .expect("transcriber start should not time out")
        .expect("transcriber start should succeed");

    let mut saw_resolved = false;
    let mut saw_applied = false;
    let deadline = time::Instant::now() + Duration::from_secs(3);
    while time::Instant::now() < deadline && !(saw_resolved && saw_applied) {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        match time::timeout(remaining, event_rx.recv()).await {
            Ok(Some(AppEvent::LogMessage(message))) => {
                if message.starts_with("Sanity check: API key resolved (") {
                    saw_resolved = true;
                }
                if message.starts_with("Sanity check: API key applied (") {
                    saw_applied = true;
                }
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => break,
        }
    }

    assert!(
        saw_resolved,
        "expected resolved sanity message after start; got none"
    );
    assert!(
        saw_applied,
        "expected applied sanity message after websocket connect; got none"
    );

    time::timeout(Duration::from_secs(3), transcriber.stop())
        .await
        .expect("transcriber stop should not time out")
        .expect("transcriber stop should succeed");
    server
        .await
        .expect("mock websocket server task should complete");
}

#[test]
fn completion_matching_respects_expected_item_id() {
    assert!(completion_matches_expected(None, None));
    assert!(!completion_matches_expected(None, Some("item-a")));
    assert!(completion_matches_expected(Some("item-a"), Some("item-a")));
    assert!(!completion_matches_expected(Some("item-a"), Some("item-b")));
    assert!(!completion_matches_expected(Some("item-a"), None));
}

#[test]
fn stop_flush_progress_ignores_stale_completion_until_new_commit_ack() {
    let mut stop_commit_pending = true;
    let mut stop_commit_item_id = None;
    let mut stop_completion_received = false;

    observe_stop_flush_progress(
        &ServerEvent::TranscriptionCompleted {
            item_id: Some("item-stale".to_owned()),
            text: Some("stale".to_owned()),
        },
        &mut stop_commit_pending,
        &mut stop_commit_item_id,
        &mut stop_completion_received,
    );
    assert!(!stop_completion_received);
    assert!(stop_commit_item_id.is_none());

    observe_stop_flush_progress(
        &ServerEvent::InputAudioBufferCommitted {
            item_id: Some("item-fresh".to_owned()),
            previous_item_id: Some("item-stale".to_owned()),
        },
        &mut stop_commit_pending,
        &mut stop_commit_item_id,
        &mut stop_completion_received,
    );
    assert_eq!(stop_commit_item_id.as_deref(), Some("item-fresh"));
    assert!(!stop_completion_received);

    observe_stop_flush_progress(
        &ServerEvent::TranscriptionCompleted {
            item_id: Some("item-fresh".to_owned()),
            text: Some("final".to_owned()),
        },
        &mut stop_commit_pending,
        &mut stop_commit_item_id,
        &mut stop_completion_received,
    );
    assert!(stop_completion_received);
}

#[test]
fn stop_pending_forwarding_filters_non_matching_completion() {
    let mismatched = ServerEvent::TranscriptionCompleted {
        item_id: Some("item-b".to_owned()),
        text: Some("stale".to_owned()),
    };
    let matched = ServerEvent::TranscriptionCompleted {
        item_id: Some("item-a".to_owned()),
        text: Some("final".to_owned()),
    };

    assert!(!should_forward_server_event_to_app(
        &mismatched,
        true,
        Some("item-a")
    ));
    assert!(should_forward_server_event_to_app(
        &matched,
        true,
        Some("item-a")
    ));
}
