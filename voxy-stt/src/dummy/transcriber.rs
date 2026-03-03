use std::sync::{Arc, Mutex};

use tokio::{
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
    time::{self, Duration},
};
use voxy_audio::AudioFrameSource;
use voxy_core::AppEvent;

use crate::trace;
use crate::traits::{
    StreamingTranscriber, TranscriberContractError, TranscriberInput, TranscriberOutput,
    TranscriberSessionConfig, TranscriberStreamState,
};

pub struct DummyStreamingTranscriber {
    tx: mpsc::Sender<AppEvent>,
    audio_source: Option<Arc<dyn AudioFrameSource>>,
    tick_interval: Duration,
    downlink_tx: broadcast::Sender<TranscriberOutput>,
    worker: Mutex<WorkerState>,
}

const UPLINK_BUFFER_CAPACITY: usize = 256;

#[derive(Debug, Default)]
struct WorkerState {
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
    uplink_tx: Option<mpsc::Sender<TranscriberInput>>,
}

impl DummyStreamingTranscriber {
    pub fn new(
        tx: mpsc::Sender<AppEvent>,
        audio_source: Option<Arc<dyn AudioFrameSource>>,
    ) -> Self {
        Self::new_with_tick(tx, audio_source, Duration::from_secs(2))
    }

    pub fn new_with_tick(
        tx: mpsc::Sender<AppEvent>,
        audio_source: Option<Arc<dyn AudioFrameSource>>,
        tick_interval: Duration,
    ) -> Self {
        let (downlink_tx, _) = broadcast::channel(64);
        Self {
            tx,
            audio_source,
            tick_interval,
            downlink_tx,
            worker: Mutex::new(WorkerState::default()),
        }
    }

    fn lock_worker(
        &self,
        context: &'static str,
    ) -> Result<std::sync::MutexGuard<'_, WorkerState>, TranscriberContractError> {
        self.worker.lock().map_err(|_| {
            TranscriberContractError::Internal(format!(
                "dummy transcriber mutex poisoned in {context}"
            ))
        })
    }
}

impl StreamingTranscriber for DummyStreamingTranscriber {
    async fn start(
        &self,
        config: TranscriberSessionConfig,
    ) -> Result<(), TranscriberContractError> {
        let mut worker = self.lock_worker("start")?;

        if worker.task.is_some() {
            return Err(TranscriberContractError::AlreadyRunning);
        }
        trace::log(
            "dummy/start",
            format!(
                "model={} sample_rate={} channels={}",
                config.model.as_api_id(),
                config.sample_rate_hz,
                config.channels
            ),
        );

        let (stop_tx, mut stop_rx) = oneshot::channel();
        let (uplink_tx, mut uplink_rx) = mpsc::channel(UPLINK_BUFFER_CAPACITY);
        let tx = self.tx.clone();
        let downlink_tx = self.downlink_tx.clone();
        let audio_source = self.audio_source.clone();
        let tick_interval = self.tick_interval;

        let task = tokio::spawn(async move {
            let fake_chunks = ["listening... ", "partial phrase... ", "continuing... "];
            let mut chunk_index = 0usize;
            let mut ticker = time::interval(tick_interval);

            let _ = downlink_tx.send(TranscriberOutput::SessionStarted(config));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Some(source) = audio_source.as_ref() {
                            let Some(frame) = source.read_frame() else {
                                continue;
                            };
                            if frame.is_empty() {
                                continue;
                            }

                            let chunk = fake_chunks[chunk_index % fake_chunks.len()];
                            chunk_index += 1;
                            let payload = chunk.to_owned();

                            if tx.send(AppEvent::LiveText(payload.clone())).await.is_err() {
                                break;
                            }
                            trace::log("dummy/tick", format!("emit LiveText len={}", payload.len()));

                            let _ = downlink_tx.send(TranscriberOutput::LiveText(payload));
                        }
                    }
                    input = uplink_rx.recv() => {
                        match input {
                            Some(TranscriberInput::AudioFrame(frame)) => {
                                if frame.is_empty() {
                                    continue;
                                }

                                let chunk = fake_chunks[chunk_index % fake_chunks.len()];
                                chunk_index += 1;
                                let payload = chunk.to_owned();

                                if tx.send(AppEvent::LiveText(payload.clone())).await.is_err() {
                                    break;
                                }

                                let _ = downlink_tx.send(TranscriberOutput::LiveText(payload));
                            }
                            Some(TranscriberInput::Commit) => {
                                if tx.send(AppEvent::CommitRequested).await.is_err() {
                                    break;
                                }
                                trace::log("dummy/uplink", "emit CommitRequested");
                                let _ = downlink_tx.send(TranscriberOutput::SegmentCommitted);
                            }
                            Some(TranscriberInput::Clear) => {
                                let _ = downlink_tx.send(TranscriberOutput::SegmentCleared);
                            }
                            None => break,
                        }
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }

            let _ = downlink_tx.send(TranscriberOutput::SessionStopped);
        });

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
            worker.uplink_tx = None;
            (worker.stop_tx.take(), worker.task.take())
        };

        if let Some(stop_tx) = stop_tx {
            let _ = stop_tx.send(());
        }

        if let Some(task) = task {
            let _ = task.await;
        }
        trace::log("dummy/stop", "stopped");

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
                    "dummy/state",
                    "dummy transcriber mutex poisoned; reporting idle",
                );
                TranscriberStreamState::Idle
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex, time::Duration};

    use tokio::time;
    use voxy_audio::{AudioFrameSource, PcmFrame};
    use voxy_core::AppEvent;

    use super::DummyStreamingTranscriber;
    use crate::traits::{
        StreamingTranscriber, TranscriberInput, TranscriberOutput, TranscriberSessionConfig,
        TranscriberStreamState,
    };

    struct TestAudioSource {
        frames: Mutex<VecDeque<PcmFrame>>,
    }

    impl TestAudioSource {
        fn with_frame_count(frame_count: usize) -> Self {
            let frames = (0..frame_count)
                .map(|_| PcmFrame::new(16_000, 1, vec![1, 2, 3, 4]))
                .collect::<VecDeque<_>>();
            Self {
                frames: Mutex::new(frames),
            }
        }
    }

    impl AudioFrameSource for TestAudioSource {
        fn sample_rate_hz(&self) -> u32 {
            16_000
        }

        fn channels(&self) -> u16 {
            1
        }

        fn read_frame(&self) -> Option<PcmFrame> {
            self.frames
                .lock()
                .expect("test source lock poisoned")
                .pop_front()
        }
    }

    #[tokio::test]
    async fn emits_live_text_when_audio_frames_exist() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let source = std::sync::Arc::new(TestAudioSource::with_frame_count(1));
        let transcriber =
            DummyStreamingTranscriber::new_with_tick(tx, Some(source), Duration::from_millis(10));

        transcriber
            .start(TranscriberSessionConfig::from_model(Default::default()))
            .await
            .expect("dummy start should succeed");

        let event = time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timed out while waiting for live text");

        transcriber.stop().await.expect("dummy stop should succeed");

        match event {
            Some(AppEvent::LiveText(payload)) => {
                assert!(!payload.is_empty());
            }
            other => panic!("expected live text event, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn does_not_emit_live_text_without_audio_frames() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let source = std::sync::Arc::new(TestAudioSource::with_frame_count(0));
        let transcriber =
            DummyStreamingTranscriber::new_with_tick(tx, Some(source), Duration::from_millis(10));

        transcriber
            .start(TranscriberSessionConfig::from_model(Default::default()))
            .await
            .expect("dummy start should succeed");

        let received = time::timeout(Duration::from_millis(80), rx.recv()).await;

        transcriber.stop().await.expect("dummy stop should succeed");

        assert!(received.is_err(), "unexpected live text received");
    }

    #[tokio::test]
    async fn emits_live_text_from_uplink_audio_frame() {
        let (tx, mut app_rx) = tokio::sync::mpsc::channel(8);
        let transcriber =
            DummyStreamingTranscriber::new_with_tick(tx, None, Duration::from_secs(1));
        let mut downlink_rx = transcriber.subscribe();

        transcriber
            .start(TranscriberSessionConfig::from_model(Default::default()))
            .await
            .expect("dummy start should succeed");

        transcriber
            .push_input(TranscriberInput::AudioFrame(PcmFrame::new(
                16_000,
                1,
                vec![1, 2, 3, 4],
            )))
            .await
            .expect("uplink push should succeed");

        let app_event = time::timeout(Duration::from_millis(200), app_rx.recv())
            .await
            .expect("timed out waiting for app event")
            .expect("channel should emit");

        let downlink_event = time::timeout(Duration::from_millis(200), async {
            loop {
                match downlink_rx.recv().await {
                    Ok(event @ TranscriberOutput::LiveText(_)) => break event,
                    Ok(_) => continue,
                    Err(error) => panic!("downlink receive failed: {error}"),
                }
            }
        })
        .await
        .expect("timed out waiting for live-text downlink event");

        transcriber.stop().await.expect("dummy stop should succeed");

        assert!(matches!(app_event, AppEvent::LiveText(_)));
        assert!(matches!(downlink_event, TranscriberOutput::LiveText(_)));
        assert_eq!(transcriber.state(), TranscriberStreamState::Idle);
    }

    #[tokio::test]
    async fn emits_commit_event_from_uplink_commit() {
        let (tx, mut app_rx) = tokio::sync::mpsc::channel(8);
        let transcriber =
            DummyStreamingTranscriber::new_with_tick(tx, None, Duration::from_secs(1));

        transcriber
            .start(TranscriberSessionConfig::from_model(Default::default()))
            .await
            .expect("dummy start should succeed");

        transcriber
            .push_input(TranscriberInput::Commit)
            .await
            .expect("commit push should succeed");

        let app_event = time::timeout(Duration::from_millis(200), app_rx.recv())
            .await
            .expect("timed out waiting for commit")
            .expect("channel should emit");

        transcriber.stop().await.expect("dummy stop should succeed");

        assert_eq!(app_event, AppEvent::CommitRequested);
    }
}
