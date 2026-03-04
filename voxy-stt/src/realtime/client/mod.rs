use std::{
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
use tokio_tungstenite::{connect_async, tungstenite::Message};
use voxy_audio::AudioFrameSource;
use voxy_core::AppEvent;

use crate::{
    config::load_api_key,
    realtime::{
        backoff::RetryPolicy, protocol::client_event::ClientEvent, session::SessionConfig,
        state::ConnectionState,
    },
    trace,
    traits::{
        StreamingTranscriber, TranscriberContractError, TranscriberInput, TranscriberOutput,
        TranscriberSessionConfig, TranscriberStreamState,
    },
};

use reconnect::{
    build_request, ensure_rustls_provider, realtime_url_from_env, reconnect_config_from_env,
    reconnect_decision, redact_ws_url_for_trace, should_retry_tungstenite_error,
    source_poll_interval_from_env, stop_flush_timeout_from_env,
};
use server::handle_server_payload;
use stop_flush::observe_stop_flush_progress;
use uplink::{handle_uplink_input, input_kind, send_client_event};

#[cfg(test)]
use reconnect::{parse_realtime_url, parse_source_poll_ms};
#[cfg(test)]
use server::should_forward_server_event_to_app;
#[cfg(test)]
use stop_flush::completion_matches_expected;

const DEFAULT_REALTIME_URL: &str = "wss://api.openai.com/v1/realtime?intent=transcription";
const REALTIME_URL_ENV: &str = "VOXY_OPENAI_REALTIME_URL";
const SOURCE_POLL_MS_ENV: &str = "VOXY_STT_SOURCE_POLL_MS";
const DEFAULT_SOURCE_POLL_MS: u64 = 20;
const RECONNECT_ENABLED_ENV: &str = "VOXY_STT_RECONNECT_ENABLED";
const RECONNECT_MAX_RETRIES_ENV: &str = "VOXY_STT_RECONNECT_MAX_RETRIES";
const RECONNECT_BASE_MS_ENV: &str = "VOXY_STT_RECONNECT_BASE_MS";
const RECONNECT_MAX_MS_ENV: &str = "VOXY_STT_RECONNECT_MAX_MS";
const STOP_FLUSH_TIMEOUT_MS_ENV: &str = "VOXY_STT_STOP_FLUSH_TIMEOUT_MS";
const DEFAULT_RECONNECT_ENABLED: bool = true;
const DEFAULT_RECONNECT_MAX_RETRIES: u32 = 8;
const DEFAULT_RECONNECT_BASE_MS: u64 = 250;
const DEFAULT_RECONNECT_MAX_MS: u64 = 5_000;
const DEFAULT_STOP_FLUSH_TIMEOUT_MS: u64 = 3_000;
const UPLINK_BUFFER_CAPACITY: usize = 256;
static SOURCE_FRAME_SEQ: AtomicU64 = AtomicU64::new(0);
static UPLINK_SEQ: AtomicU64 = AtomicU64::new(0);
static SERVER_SEQ: AtomicU64 = AtomicU64::new(0);
static SERVER_PAYLOAD_SEQ: AtomicU64 = AtomicU64::new(0);
static RETRY_JITTER_SEQ: AtomicU64 = AtomicU64::new(0);
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

        if worker
            .task
            .as_ref()
            .map(|task| task.is_finished())
            .unwrap_or(false)
        {
            trace::log(
                "start",
                "clearing stale worker state from finished background task",
            );
            worker.task = None;
            worker.stop_tx = None;
            worker.uplink_tx = None;
            worker.connection_state = ConnectionState::Disconnected;
        }

        if worker.task.is_some() {
            return Err(TranscriberContractError::AlreadyRunning);
        }

        let api_key = load_api_key().map_err(|error| {
            TranscriberContractError::Internal(format!("failed to load API key: {error}"))
        })?;
        let api_key_source = api_key.source.redacted_description().to_owned();
        ensure_rustls_provider()
            .map_err(|error| TranscriberContractError::Internal(error.to_owned()))?;
        let ws_url = realtime_url_from_env();
        trace::log(
            "start",
            format!(
                "api_key_source={} ws_url={} model={} sample_rate={} channels={}",
                api_key_source,
                redact_ws_url_for_trace(&ws_url),
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
                api_key_source,
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
            Ok(mut worker) => {
                if worker
                    .task
                    .as_ref()
                    .map(|task| task.is_finished())
                    .unwrap_or(false)
                {
                    trace::log("state", "detected finished worker task; resetting to idle");
                    worker.task = None;
                    worker.stop_tx = None;
                    worker.uplink_tx = None;
                    worker.connection_state = ConnectionState::Disconnected;
                }
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

#[allow(clippy::too_many_arguments)]
async fn run_session_with_reconnect(
    tx: mpsc::Sender<AppEvent>,
    downlink_tx: broadcast::Sender<TranscriberOutput>,
    audio_source: Option<Arc<dyn AudioFrameSource>>,
    mut uplink_rx: mpsc::Receiver<TranscriberInput>,
    mut stop_rx: oneshot::Receiver<()>,
    ws_url: String,
    api_key: String,
    api_key_source: String,
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
    emit_log_message(
        &tx,
        format!("Sanity check: API key resolved ({api_key_source})"),
    )
    .await;

    let mut retry_attempt = 0u32;
    let mut api_key_applied_logged = false;
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
            &api_key_source,
            &mut api_key_applied_logged,
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

#[allow(clippy::too_many_arguments)]
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
    api_key_source: &str,
    api_key_applied_logged: &mut bool,
) -> SessionAttemptOutcome {
    let stop_flush_timeout = stop_flush_timeout_from_env();
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
    if !*api_key_applied_logged {
        emit_log_message(
            tx,
            format!("Sanity check: API key applied ({api_key_source})"),
        )
        .await;
        *api_key_applied_logged = true;
    }

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
    let mut stop_commit_pending = false;
    let mut stop_commit_item_id: Option<String> = None;
    let mut stop_completion_received = false;
    let mut stop_flush_deadline: Option<time::Instant> = None;

    loop {
        tokio::select! {
            _ = &mut *stop_rx, if !stop_commit_pending => {
                trace::log("session", "stop requested -> commit + wait for completion");
                if let Err(error) = send_client_event(&mut writer, ClientEvent::InputAudioBufferCommit).await {
                    return SessionAttemptOutcome::FatalFailure(format!(
                        "failed to send realtime commit while stopping: {error}"
                    ));
                }

                stop_commit_pending = true;
                stop_flush_deadline = Some(time::Instant::now() + stop_flush_timeout);
            }
            incoming = reader.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let seq = SERVER_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
                        if trace::should_log(seq) {
                            trace::log("server", format!("recv text_message#{} bytes={}", seq, text.len()));
                        }
                        if let Some(event) = handle_server_payload(
                            tx,
                            downlink_tx,
                            &text,
                            stop_commit_pending,
                            stop_commit_item_id.as_deref(),
                        )
                        .await
                        {
                            observe_stop_flush_progress(
                                &event,
                                &mut stop_commit_pending,
                                &mut stop_commit_item_id,
                                &mut stop_completion_received,
                            );
                        }
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
                        if let Some(event) = handle_server_payload(
                            tx,
                            downlink_tx,
                            &payload,
                            stop_commit_pending,
                            stop_commit_item_id.as_deref(),
                        )
                        .await
                        {
                            observe_stop_flush_progress(
                                &event,
                                &mut stop_commit_pending,
                                &mut stop_commit_item_id,
                                &mut stop_completion_received,
                            );
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        trace::log("server", "socket closed by peer");
                        if stop_commit_pending {
                            emit_runtime_error(
                                tx,
                                downlink_tx,
                                "Realtime socket closed before stop flush completed; final transcript may be incomplete.".to_owned(),
                            )
                            .await;
                            return SessionAttemptOutcome::StopRequested;
                        }
                        return SessionAttemptOutcome::RetryableFailure(
                            "realtime websocket closed by peer".to_owned(),
                        );
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        trace::log("server", format!("socket read error={error}"));
                        if stop_commit_pending {
                            emit_runtime_error(
                                tx,
                                downlink_tx,
                                format!(
                                    "Realtime read failed while waiting for final stop flush; final transcript may be incomplete: {error}"
                                ),
                            )
                            .await;
                            return SessionAttemptOutcome::StopRequested;
                        }
                        let message = format!("realtime websocket read failed: {error}");
                        if should_retry_tungstenite_error(&error) {
                            return SessionAttemptOutcome::RetryableFailure(message);
                        }
                        return SessionAttemptOutcome::FatalFailure(message);
                    }
                }

                if stop_commit_pending && stop_completion_received {
                    trace::log(
                        "session",
                        format!(
                            "stop flush completed item_id={}",
                            stop_commit_item_id
                                .as_deref()
                                .unwrap_or("<unspecified>")
                        ),
                    );
                    let _ = writer.send(Message::Close(None)).await;
                    return SessionAttemptOutcome::StopRequested;
                }
            }
            uplink = uplink_rx.recv(), if !stop_commit_pending => {
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
            _ = source_poll.tick(), if audio_source.is_some() && !stop_commit_pending => {
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
            _ = async {
                if let Some(deadline) = stop_flush_deadline {
                    time::sleep_until(deadline).await;
                }
            }, if stop_commit_pending && stop_flush_deadline.is_some() => {
                emit_runtime_error(
                    tx,
                    downlink_tx,
                    format!(
                        "Timed out waiting for final transcription flush after stop ({}ms); transcript may be incomplete.",
                        stop_flush_timeout.as_millis()
                    ),
                )
                .await;
                let _ = writer.send(Message::Close(None)).await;
                return SessionAttemptOutcome::StopRequested;
            }
        }
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

mod reconnect;
mod server;
mod stop_flush;
mod uplink;

#[cfg(test)]
mod tests;
