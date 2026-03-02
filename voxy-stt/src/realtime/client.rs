use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        self,
        client::IntoClientRequest,
        http::{header::HeaderValue, Request, StatusCode},
        Message,
    },
};
use voxy_audio::AudioFrameSource;
use voxy_core::AppEvent;

use crate::{
    config::load_api_key,
    realtime::{
        audio_uplink,
        backoff::{delay_for_attempt, RetryPolicy},
        event_mapper::map_server_event,
        protocol::{client_event::ClientEvent, server_event::parse_server_event},
        session::SessionConfig,
        state::ConnectionState,
    },
    trace,
    traits::{
        StreamingTranscriber, TranscriberContractError, TranscriberInput, TranscriberOutput,
        TranscriberSessionConfig, TranscriberStreamState,
    },
};

const DEFAULT_REALTIME_URL: &str = "wss://api.openai.com/v1/realtime?intent=transcription";
const REALTIME_URL_ENV: &str = "VOXY_OPENAI_REALTIME_URL";
const SOURCE_POLL_MS_ENV: &str = "VOXY_STT_SOURCE_POLL_MS";
const DEFAULT_SOURCE_POLL_MS: u64 = 20;
const RECONNECT_ENABLED_ENV: &str = "VOXY_STT_RECONNECT_ENABLED";
const RECONNECT_MAX_RETRIES_ENV: &str = "VOXY_STT_RECONNECT_MAX_RETRIES";
const RECONNECT_BASE_MS_ENV: &str = "VOXY_STT_RECONNECT_BASE_MS";
const RECONNECT_MAX_MS_ENV: &str = "VOXY_STT_RECONNECT_MAX_MS";
const DEFAULT_RECONNECT_ENABLED: bool = true;
const DEFAULT_RECONNECT_BASE_MS: u64 = 250;
const DEFAULT_RECONNECT_MAX_MS: u64 = 5_000;
const UPLINK_BUFFER_CAPACITY: usize = 256;
static SOURCE_FRAME_SEQ: AtomicU64 = AtomicU64::new(0);
static UPLINK_SEQ: AtomicU64 = AtomicU64::new(0);
static SERVER_SEQ: AtomicU64 = AtomicU64::new(0);
static SERVER_PAYLOAD_SEQ: AtomicU64 = AtomicU64::new(0);
static RUSTLS_PROVIDER_INIT: OnceLock<Result<(), String>> = OnceLock::new();

pub struct OpenAiRealtimeTranscriber {
    tx: mpsc::Sender<AppEvent>,
    audio_source: Option<Arc<dyn AudioFrameSource>>,
    downlink_tx: broadcast::Sender<TranscriberOutput>,
    source_poll_interval: Duration,
    worker: Mutex<WorkerState>,
}

#[derive(Debug, Default)]
struct WorkerState {
    connection_state: ConnectionState,
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
    uplink_tx: Option<mpsc::Sender<TranscriberInput>>,
}

#[derive(Debug, Clone, Copy)]
struct ReconnectConfig {
    enabled: bool,
    max_retries: Option<u32>,
    retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionAttemptOutcome {
    StopRequested,
    RetryableFailure(String),
    FatalFailure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetryPlan {
    delay: Duration,
    retry_attempt: u32,
    attempt_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RetryDecision {
    Retry(RetryPlan),
    GiveUp(String),
}

impl OpenAiRealtimeTranscriber {
    pub fn new(
        tx: mpsc::Sender<AppEvent>,
        audio_source: Option<Arc<dyn AudioFrameSource>>,
    ) -> Self {
        let (downlink_tx, _) = broadcast::channel(256);
        Self {
            tx,
            audio_source,
            downlink_tx,
            source_poll_interval: source_poll_interval_from_env(),
            worker: Mutex::new(WorkerState::default()),
        }
    }

    pub fn with_source_poll_interval(
        tx: mpsc::Sender<AppEvent>,
        audio_source: Option<Arc<dyn AudioFrameSource>>,
        source_poll_interval: Duration,
    ) -> Self {
        let (downlink_tx, _) = broadcast::channel(256);
        Self {
            tx,
            audio_source,
            downlink_tx,
            source_poll_interval,
            worker: Mutex::new(WorkerState::default()),
        }
    }

    fn lock_worker(
        &self,
        context: &'static str,
    ) -> Result<std::sync::MutexGuard<'_, WorkerState>, TranscriberContractError> {
        self.worker.lock().map_err(|_| {
            TranscriberContractError::Internal(format!(
                "realtime transcriber mutex poisoned in {context}"
            ))
        })
    }
}

impl StreamingTranscriber for OpenAiRealtimeTranscriber {
    async fn start(
        &self,
        config: TranscriberSessionConfig,
    ) -> Result<(), TranscriberContractError> {
        let mut worker = self.lock_worker("start")?;

        if worker.task.is_some() {
            return Err(TranscriberContractError::AlreadyRunning);
        }

        let api_key = load_api_key().map_err(|error| {
            TranscriberContractError::Internal(format!("failed to load API key: {error}"))
        })?;
        ensure_rustls_provider()
            .map_err(|error| TranscriberContractError::Internal(error.to_owned()))?;
        let ws_url = realtime_url_from_env();
        trace::log(
            "start",
            format!(
                "api_key_source={} ws_url={} model={} sample_rate={} channels={}",
                api_key.source.description(),
                ws_url,
                config.model.as_api_id(),
                config.sample_rate_hz,
                config.channels
            ),
        );

        let (stop_tx, stop_rx) = oneshot::channel();
        let (uplink_tx, uplink_rx) = mpsc::channel(UPLINK_BUFFER_CAPACITY);
        let tx = self.tx.clone();
        let downlink_tx = self.downlink_tx.clone();
        let audio_source = self.audio_source.clone();
        let source_poll_interval = self.source_poll_interval;
        let reconnect_config = reconnect_config_from_env();
        trace::log(
            "start",
            format!(
                "reconnect enabled={} max_retries={} base_ms={} max_ms={}",
                reconnect_config.enabled,
                reconnect_config
                    .max_retries
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unlimited".to_owned()),
                reconnect_config.retry_policy.base_delay.as_millis(),
                reconnect_config.retry_policy.max_delay.as_millis()
            ),
        );

        let task = tokio::spawn(async move {
            run_session_with_reconnect(
                tx,
                downlink_tx,
                audio_source,
                uplink_rx,
                stop_rx,
                ws_url,
                api_key.api_key,
                config,
                source_poll_interval,
                reconnect_config,
            )
            .await;
        });

        worker.connection_state = ConnectionState::Connecting;
        worker.stop_tx = Some(stop_tx);
        worker.task = Some(task);
        worker.uplink_tx = Some(uplink_tx);
        Ok(())
    }

    async fn push_input(&self, input: TranscriberInput) -> Result<(), TranscriberContractError> {
        let uplink_tx = {
            let worker = self.lock_worker("push_input")?;
            worker.uplink_tx.clone()
        };

        let Some(uplink_tx) = uplink_tx else {
            return Err(TranscriberContractError::NotRunning);
        };

        uplink_tx
            .send(input)
            .await
            .map_err(|_| TranscriberContractError::UplinkClosed)
    }

    async fn stop(&self) -> Result<(), TranscriberContractError> {
        let (stop_tx, task) = {
            let mut worker = self.lock_worker("stop")?;
            worker.connection_state = ConnectionState::Stopping;
            worker.uplink_tx = None;
            (worker.stop_tx.take(), worker.task.take())
        };

        if let Some(stop_tx) = stop_tx {
            let _ = stop_tx.send(());
        }

        if let Some(task) = task {
            let _ = task.await;
        }

        let mut worker = self.lock_worker("stop_after_join")?;
        worker.connection_state = ConnectionState::Disconnected;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<TranscriberOutput> {
        self.downlink_tx.subscribe()
    }

    fn state(&self) -> TranscriberStreamState {
        match self.worker.lock() {
            Ok(worker) => {
                if worker.task.is_some() {
                    TranscriberStreamState::Streaming
                } else {
                    TranscriberStreamState::Idle
                }
            }
            Err(_) => {
                trace::log(
                    "state",
                    "realtime transcriber mutex poisoned; reporting idle",
                );
                TranscriberStreamState::Idle
            }
        }
    }
}

async fn run_session_with_reconnect(
    tx: mpsc::Sender<AppEvent>,
    downlink_tx: broadcast::Sender<TranscriberOutput>,
    audio_source: Option<Arc<dyn AudioFrameSource>>,
    mut uplink_rx: mpsc::Receiver<TranscriberInput>,
    mut stop_rx: oneshot::Receiver<()>,
    ws_url: String,
    api_key: String,
    config: TranscriberSessionConfig,
    source_poll_interval: Duration,
    reconnect_config: ReconnectConfig,
) {
    trace::log(
        "session",
        format!(
            "source_poll_interval_ms={} vad_silence_ms={} reconnect_enabled={} reconnect_max_retries={}",
            source_poll_interval.as_millis(),
            config.vad_silence_duration_ms,
            reconnect_config.enabled,
            reconnect_config
                .max_retries
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unlimited".to_owned())
        ),
    );

    let mut retry_attempt = 0u32;
    loop {
        let outcome = run_single_session_attempt(
            &tx,
            &downlink_tx,
            audio_source.as_ref(),
            &mut uplink_rx,
            &mut stop_rx,
            &ws_url,
            &api_key,
            &config,
            source_poll_interval,
        )
        .await;

        match outcome {
            SessionAttemptOutcome::StopRequested => {
                trace::log("session", "session stop requested");
                break;
            }
            SessionAttemptOutcome::FatalFailure(message) => {
                emit_runtime_error(&tx, &downlink_tx, message).await;
                break;
            }
            SessionAttemptOutcome::RetryableFailure(message) => {
                match reconnect_decision(reconnect_config, retry_attempt, &message) {
                    RetryDecision::GiveUp(final_message) => {
                        emit_runtime_error(&tx, &downlink_tx, final_message).await;
                        break;
                    }
                    RetryDecision::Retry(plan) => {
                        retry_attempt = plan.retry_attempt;
                        emit_log_message(
                            &tx,
                            format!(
                                "Transcriber connection lost; reconnecting in {}ms (attempt {})",
                                plan.delay.as_millis(),
                                plan.attempt_label
                            ),
                        )
                        .await;
                        trace::log(
                            "session",
                            format!(
                                "retryable failure: '{}' -> backoff={}ms attempt={}",
                                message,
                                plan.delay.as_millis(),
                                plan.attempt_label
                            ),
                        );

                        tokio::select! {
                            _ = &mut stop_rx => {
                                trace::log("session", "stop requested during reconnect backoff");
                                break;
                            }
                            _ = time::sleep(plan.delay) => {}
                        }
                    }
                }
            }
        }
    }

    let _ = downlink_tx.send(TranscriberOutput::SessionStopped);
    trace::log("session", "session stopped");
}

async fn run_single_session_attempt(
    tx: &mpsc::Sender<AppEvent>,
    downlink_tx: &broadcast::Sender<TranscriberOutput>,
    audio_source: Option<&Arc<dyn AudioFrameSource>>,
    uplink_rx: &mut mpsc::Receiver<TranscriberInput>,
    stop_rx: &mut oneshot::Receiver<()>,
    ws_url: &str,
    api_key: &str,
    config: &TranscriberSessionConfig,
    source_poll_interval: Duration,
) -> SessionAttemptOutcome {
    let request = match build_request(ws_url, api_key) {
        Ok(request) => request,
        Err(error) => {
            return SessionAttemptOutcome::FatalFailure(format!(
                "failed to build realtime websocket request: {error}"
            ));
        }
    };

    let connect = connect_async(request);
    tokio::pin!(connect);
    let (ws_stream, _) = tokio::select! {
        _ = &mut *stop_rx => return SessionAttemptOutcome::StopRequested,
        result = &mut connect => {
            match result {
                Ok(parts) => parts,
                Err(error) => {
                    let message = format!("failed to connect realtime websocket: {error}");
                    if should_retry_tungstenite_error(&error) {
                        return SessionAttemptOutcome::RetryableFailure(message);
                    }
                    return SessionAttemptOutcome::FatalFailure(message);
                }
            }
        }
    };
    trace::log("session", "websocket connected");

    let (mut writer, mut reader) = ws_stream.split();

    let session = SessionConfig::for_model(config.model);
    let session_update = ClientEvent::SessionUpdate {
        model: session.model.as_api_id().to_owned(),
        input_audio_format: session.input_audio_format.to_owned(),
        turn_detection: session.turn_detection.to_owned(),
        turn_detection_silence_duration_ms: config.vad_silence_duration_ms,
    };

    if let Err(error) = send_client_event(&mut writer, session_update).await {
        let message = format!("failed to send realtime session.update: {error}");
        if should_retry_tungstenite_error(&error) {
            return SessionAttemptOutcome::RetryableFailure(message);
        }
        return SessionAttemptOutcome::FatalFailure(message);
    }
    trace::log("session", "sent transcription_session.update");

    let _ = downlink_tx.send(TranscriberOutput::SessionStarted(config.clone()));

    let mut source_poll = time::interval(source_poll_interval);
    source_poll.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = &mut *stop_rx => {
                trace::log("session", "stop requested -> commit + close");
                let _ = send_client_event(&mut writer, ClientEvent::InputAudioBufferCommit).await;
                let _ = writer.send(Message::Close(None)).await;
                return SessionAttemptOutcome::StopRequested;
            }
            incoming = reader.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let seq = SERVER_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
                        if trace::should_log(seq) {
                            trace::log("server", format!("recv text_message#{} bytes={}", seq, text.len()));
                        }
                        handle_server_payload(tx, downlink_tx, &text).await;
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        let payload = String::from_utf8_lossy(&bytes);
                        let seq = SERVER_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
                        if trace::should_log(seq) {
                            trace::log(
                                "server",
                                format!("recv binary_message#{} bytes={}", seq, bytes.len()),
                            );
                        }
                        handle_server_payload(tx, downlink_tx, &payload).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        trace::log("server", "socket closed by peer");
                        return SessionAttemptOutcome::RetryableFailure(
                            "realtime websocket closed by peer".to_owned(),
                        );
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        trace::log("server", format!("socket read error={error}"));
                        let message = format!("realtime websocket read failed: {error}");
                        if should_retry_tungstenite_error(&error) {
                            return SessionAttemptOutcome::RetryableFailure(message);
                        }
                        return SessionAttemptOutcome::FatalFailure(message);
                    }
                }
            }
            uplink = uplink_rx.recv() => {
                match uplink {
                    Some(input) => {
                        trace::log("uplink", format!("received manual input {:?}", input_kind(&input)));
                        if let Err(error) = handle_uplink_input(&mut writer, input).await {
                            let message = format!("failed to send realtime uplink event: {error}");
                            if should_retry_tungstenite_error(&error) {
                                return SessionAttemptOutcome::RetryableFailure(message);
                            }
                            return SessionAttemptOutcome::FatalFailure(message);
                        }
                    }
                    None => {
                        trace::log("uplink", "uplink channel closed");
                        return SessionAttemptOutcome::StopRequested;
                    }
                }
            }
            _ = source_poll.tick(), if audio_source.is_some() => {
                if let Some(source) = audio_source {
                    if let Some(frame) = source.read_frame() {
                        let seq = SOURCE_FRAME_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
                        if trace::should_log_noisy(seq) {
                            trace::log(
                                "source",
                                format!(
                                    "frame#{} sample_rate={} channels={} samples={}",
                                    seq,
                                    frame.sample_rate_hz,
                                    frame.channels,
                                    frame.samples_i16.len()
                                ),
                            );
                        }
                        if let Err(error) = handle_uplink_input(&mut writer, TranscriberInput::AudioFrame(frame)).await {
                            let message = format!("failed to stream source frame: {error}");
                            if should_retry_tungstenite_error(&error) {
                                return SessionAttemptOutcome::RetryableFailure(message);
                            }
                            return SessionAttemptOutcome::FatalFailure(message);
                        }
                    }
                }
            }
        }
    }
}

fn should_retry_tungstenite_error(error: &tungstenite::Error) -> bool {
    match error {
        tungstenite::Error::ConnectionClosed
        | tungstenite::Error::AlreadyClosed
        | tungstenite::Error::Io(_)
        | tungstenite::Error::Tls(_)
        | tungstenite::Error::Capacity(_)
        | tungstenite::Error::WriteBufferFull(_) => true,
        tungstenite::Error::Http(response) => is_retryable_http_status(response.status()),
        tungstenite::Error::Protocol(_)
        | tungstenite::Error::Utf8
        | tungstenite::Error::AttackAttempt
        | tungstenite::Error::Url(_)
        | tungstenite::Error::HttpFormat(_) => false,
    }
}

fn is_retryable_http_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn reconnect_decision(
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

    let delay = delay_for_attempt(reconnect_config.retry_policy, retry_attempt);
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

fn reconnect_config_from_env() -> ReconnectConfig {
    let enabled = parse_bool_env(RECONNECT_ENABLED_ENV).unwrap_or(DEFAULT_RECONNECT_ENABLED);
    let max_retries = parse_u32_env(RECONNECT_MAX_RETRIES_ENV).and_then(|value| {
        if value == 0 {
            None
        } else {
            Some(value)
        }
    });
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

fn realtime_url_from_env() -> String {
    let raw = env::var(REALTIME_URL_ENV).ok();
    parse_realtime_url(raw.as_deref())
}

fn parse_realtime_url(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| DEFAULT_REALTIME_URL.to_owned())
}

fn build_request(ws_url: &str, api_key: &str) -> Result<Request<()>, tungstenite::error::Error> {
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

async fn send_client_event(
    writer: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    event: ClientEvent,
) -> Result<(), tungstenite::Error> {
    let summary = summarize_client_event(&event);
    let payload = event.to_json().to_string();
    let seq = UPLINK_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let is_append = matches!(event, ClientEvent::InputAudioBufferAppend { .. });
    let should_log = if is_append {
        trace::should_log_noisy(seq)
    } else {
        trace::should_log(seq)
    };
    if should_log {
        trace::log("uplink", format!("send#{} {}", seq, summary));
    }
    writer.send(Message::Text(payload.into())).await
}

async fn handle_uplink_input(
    writer: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    input: TranscriberInput,
) -> Result<(), tungstenite::Error> {
    match input {
        TranscriberInput::AudioFrame(frame) => {
            let Some(chunk) = audio_uplink::encode_frame_to_base64(&frame) else {
                trace::log("uplink", "skip empty audio frame");
                return Ok(());
            };
            send_client_event(
                writer,
                ClientEvent::InputAudioBufferAppend {
                    audio: chunk.base64_pcm16,
                },
            )
            .await
        }
        TranscriberInput::Commit => {
            trace::log("uplink", "commit requested");
            send_client_event(writer, ClientEvent::InputAudioBufferCommit).await
        }
        TranscriberInput::Clear => {
            trace::log("uplink", "clear requested");
            send_client_event(writer, ClientEvent::InputAudioBufferClear).await
        }
    }
}

async fn handle_server_payload(
    tx: &mpsc::Sender<AppEvent>,
    downlink_tx: &broadcast::Sender<TranscriberOutput>,
    payload: &str,
) {
    let value = match serde_json::from_str::<serde_json::Value>(payload) {
        Ok(value) => value,
        Err(_) => return,
    };

    let event = parse_server_event(&value);
    let seq = SERVER_PAYLOAD_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    if trace::should_log(seq) {
        trace::log("server", format!("parsed_event={event:?}"));
    }
    if let Some(app_event) = map_server_event(&event) {
        if trace::should_log(seq) {
            trace::log("server", format!("mapped_app_event={app_event:?}"));
        }
        let _ = tx.send(app_event).await;
    }

    match event {
        crate::realtime::protocol::server_event::ServerEvent::TranscriptionDelta { text } => {
            if !text.is_empty() {
                let _ = downlink_tx.send(TranscriberOutput::LiveText(text));
            }
        }
        crate::realtime::protocol::server_event::ServerEvent::TranscriptionCompleted { .. } => {
            let _ = downlink_tx.send(TranscriberOutput::SegmentCommitted);
        }
        crate::realtime::protocol::server_event::ServerEvent::TranscriptionFailed { message }
        | crate::realtime::protocol::server_event::ServerEvent::Error { message } => {
            let _ = downlink_tx.send(TranscriberOutput::Error(message));
        }
        crate::realtime::protocol::server_event::ServerEvent::Unknown { .. } => {}
    }
}

async fn emit_runtime_error(
    tx: &mpsc::Sender<AppEvent>,
    downlink_tx: &broadcast::Sender<TranscriberOutput>,
    message: String,
) {
    let _ = tx.send(AppEvent::RuntimeError(message.clone())).await;
    let _ = downlink_tx.send(TranscriberOutput::Error(message));
}

async fn emit_log_message(tx: &mpsc::Sender<AppEvent>, message: String) {
    let _ = tx.send(AppEvent::LogMessage(message)).await;
}

fn input_kind(input: &TranscriberInput) -> &'static str {
    match input {
        TranscriberInput::AudioFrame(_) => "AudioFrame",
        TranscriberInput::Commit => "Commit",
        TranscriberInput::Clear => "Clear",
    }
}

fn summarize_client_event(event: &ClientEvent) -> String {
    match event {
        ClientEvent::SessionUpdate {
            model,
            input_audio_format,
            turn_detection,
            turn_detection_silence_duration_ms,
        } => format!(
            "event=transcription_session.update model={} format={} turn_detection={} silence_ms={}",
            model, input_audio_format, turn_detection, turn_detection_silence_duration_ms
        ),
        ClientEvent::InputAudioBufferAppend { audio } => format!(
            "event=input_audio_buffer.append audio_base64_len={}",
            audio.len()
        ),
        ClientEvent::InputAudioBufferCommit => "event=input_audio_buffer.commit".to_owned(),
        ClientEvent::InputAudioBufferClear => "event=input_audio_buffer.clear".to_owned(),
    }
}

fn source_poll_interval_from_env() -> Duration {
    static SOURCE_POLL_MS: OnceLock<u64> = OnceLock::new();
    let poll_ms = *SOURCE_POLL_MS.get_or_init(|| {
        let raw = env::var(SOURCE_POLL_MS_ENV).ok();
        parse_source_poll_ms(raw.as_deref())
    });

    Duration::from_millis(poll_ms)
}

fn parse_source_poll_ms(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_SOURCE_POLL_MS)
}

fn ensure_rustls_provider() -> Result<(), &'static str> {
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

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, time::Duration};

    use tokio_tungstenite::tungstenite::{self, http::Response};

    use super::{
        parse_realtime_url, parse_source_poll_ms, reconnect_config_from_env, reconnect_decision,
        should_retry_tungstenite_error, ReconnectConfig, RetryDecision, DEFAULT_REALTIME_URL,
        DEFAULT_RECONNECT_BASE_MS, DEFAULT_RECONNECT_ENABLED, DEFAULT_RECONNECT_MAX_MS,
        DEFAULT_SOURCE_POLL_MS, RECONNECT_BASE_MS_ENV, RECONNECT_ENABLED_ENV, RECONNECT_MAX_MS_ENV,
        RECONNECT_MAX_RETRIES_ENV,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
    fn reconnect_config_uses_defaults_when_env_is_unset() {
        let config = reconnect_defaults();
        assert_eq!(config.enabled, DEFAULT_RECONNECT_ENABLED);
        assert_eq!(config.max_retries, None);
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
                assert_eq!(plan.delay, Duration::from_millis(200));
                assert_eq!(plan.retry_attempt, 2);
                assert_eq!(plan.attempt_label, "2/3");
            }
            RetryDecision::GiveUp(message) => {
                panic!("expected retry decision, got give up: {message}");
            }
        }
    }
}
