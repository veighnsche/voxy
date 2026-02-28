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
        http::{header::HeaderValue, Request},
        Message,
    },
};
use voxy_audio::AudioFrameSource;
use voxy_core::AppEvent;

use crate::{
    config::load_api_key,
    realtime::{
        audio_uplink,
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
const DEFAULT_SOURCE_POLL_MS: u64 = 20;
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
    uplink_tx: Option<mpsc::UnboundedSender<TranscriberInput>>,
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
            source_poll_interval: Duration::from_millis(DEFAULT_SOURCE_POLL_MS),
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
}

impl StreamingTranscriber for OpenAiRealtimeTranscriber {
    async fn start(
        &self,
        config: TranscriberSessionConfig,
    ) -> Result<(), TranscriberContractError> {
        let mut worker = self
            .worker
            .lock()
            .expect("realtime transcriber mutex poisoned in start");

        if worker.task.is_some() {
            return Err(TranscriberContractError::AlreadyRunning);
        }

        let api_key = load_api_key().map_err(|error| {
            TranscriberContractError::Internal(format!("failed to load API key: {error}"))
        })?;
        ensure_rustls_provider()
            .map_err(|error| TranscriberContractError::Internal(error.to_owned()))?;
        let ws_url = env::var(REALTIME_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_REALTIME_URL.to_owned());
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
        let (uplink_tx, uplink_rx) = mpsc::unbounded_channel();
        let tx = self.tx.clone();
        let downlink_tx = self.downlink_tx.clone();
        let audio_source = self.audio_source.clone();
        let source_poll_interval = self.source_poll_interval;

        let task = tokio::spawn(async move {
            run_session(
                tx,
                downlink_tx,
                audio_source,
                uplink_rx,
                stop_rx,
                ws_url,
                api_key.api_key,
                config,
                source_poll_interval,
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
            let worker = self
                .worker
                .lock()
                .expect("realtime transcriber mutex poisoned in push_input");
            worker.uplink_tx.clone()
        };

        let Some(uplink_tx) = uplink_tx else {
            return Err(TranscriberContractError::NotRunning);
        };

        uplink_tx
            .send(input)
            .map_err(|_| TranscriberContractError::UplinkClosed)
    }

    async fn stop(&self) -> Result<(), TranscriberContractError> {
        let (stop_tx, task) = {
            let mut worker = self
                .worker
                .lock()
                .expect("realtime transcriber mutex poisoned in stop");
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

        let mut worker = self
            .worker
            .lock()
            .expect("realtime transcriber mutex poisoned after stop");
        worker.connection_state = ConnectionState::Disconnected;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<TranscriberOutput> {
        self.downlink_tx.subscribe()
    }

    fn state(&self) -> TranscriberStreamState {
        let worker = self
            .worker
            .lock()
            .expect("realtime transcriber mutex poisoned in state");
        if worker.task.is_some() {
            TranscriberStreamState::Streaming
        } else {
            TranscriberStreamState::Idle
        }
    }
}

async fn run_session(
    tx: mpsc::Sender<AppEvent>,
    downlink_tx: broadcast::Sender<TranscriberOutput>,
    audio_source: Option<Arc<dyn AudioFrameSource>>,
    mut uplink_rx: mpsc::UnboundedReceiver<TranscriberInput>,
    mut stop_rx: oneshot::Receiver<()>,
    ws_url: String,
    api_key: String,
    config: TranscriberSessionConfig,
    source_poll_interval: Duration,
) {
    let request = match build_request(&ws_url, &api_key) {
        Ok(request) => request,
        Err(error) => {
            emit_runtime_error(
                &tx,
                &downlink_tx,
                format!("failed to build realtime websocket request: {error}"),
            )
            .await;
            let _ = downlink_tx.send(TranscriberOutput::SessionStopped);
            return;
        }
    };

    let (ws_stream, _) = match connect_async(request).await {
        Ok(parts) => parts,
        Err(error) => {
            emit_runtime_error(
                &tx,
                &downlink_tx,
                format!("failed to connect realtime websocket: {error}"),
            )
            .await;
            let _ = downlink_tx.send(TranscriberOutput::SessionStopped);
            return;
        }
    };
    trace::log("session", "websocket connected");

    let (mut writer, mut reader) = ws_stream.split();

    let session = SessionConfig::for_model(config.model);
    let session_update = ClientEvent::SessionUpdate {
        model: session.model.as_api_id().to_owned(),
        input_audio_format: session.input_audio_format.to_owned(),
        turn_detection: session.turn_detection.to_owned(),
    };

    if let Err(error) = send_client_event(&mut writer, session_update).await {
        emit_runtime_error(
            &tx,
            &downlink_tx,
            format!("failed to send realtime session.update: {error}"),
        )
        .await;
        let _ = downlink_tx.send(TranscriberOutput::SessionStopped);
        return;
    }
    trace::log("session", "sent transcription_session.update");

    let _ = downlink_tx.send(TranscriberOutput::SessionStarted(config));

    let mut source_poll = time::interval(source_poll_interval);
    source_poll.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                trace::log("session", "stop requested -> commit + close");
                let _ = send_client_event(&mut writer, ClientEvent::InputAudioBufferCommit).await;
                let _ = writer.send(Message::Close(None)).await;
                break;
            }
            incoming = reader.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let seq = SERVER_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
                        if trace::should_log(seq) {
                            trace::log("server", format!("recv text_message#{} bytes={}", seq, text.len()));
                        }
                        handle_server_payload(&tx, &downlink_tx, &text).await;
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
                        handle_server_payload(&tx, &downlink_tx, &payload).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        trace::log("server", "socket closed by peer");
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        trace::log("server", format!("socket read error={error}"));
                        emit_runtime_error(
                            &tx,
                            &downlink_tx,
                            format!("realtime websocket read failed: {error}"),
                        ).await;
                        break;
                    }
                }
            }
            uplink = uplink_rx.recv() => {
                match uplink {
                    Some(input) => {
                        trace::log("uplink", format!("received manual input {:?}", input_kind(&input)));
                        if let Err(error) = handle_uplink_input(&mut writer, input).await {
                            emit_runtime_error(
                                &tx,
                                &downlink_tx,
                                format!("failed to send realtime uplink event: {error}"),
                            ).await;
                            break;
                        }
                    }
                    None => break,
                }
            }
            _ = source_poll.tick(), if audio_source.is_some() => {
                if let Some(source) = audio_source.as_ref() {
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
                            emit_runtime_error(
                                &tx,
                                &downlink_tx,
                                format!("failed to stream source frame: {error}"),
                            ).await;
                            break;
                        }
                    }
                }
            }
        }
    }

    let _ = downlink_tx.send(TranscriberOutput::SessionStopped);
    trace::log("session", "session stopped");
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
    if let Some(app_event) = map_server_event(event.clone()) {
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
        } => format!(
            "event=transcription_session.update model={} format={} turn_detection={}",
            model, input_audio_format, turn_detection
        ),
        ClientEvent::InputAudioBufferAppend { audio } => format!(
            "event=input_audio_buffer.append audio_base64_len={}",
            audio.len()
        ),
        ClientEvent::InputAudioBufferCommit => "event=input_audio_buffer.commit".to_owned(),
        ClientEvent::InputAudioBufferClear => "event=input_audio_buffer.clear".to_owned(),
    }
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
