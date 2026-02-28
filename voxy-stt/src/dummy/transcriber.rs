use std::sync::{Arc, Mutex};

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::{self, Duration},
};
use voxy_audio::AudioFrameSource;
use voxy_core::AppEvent;

use crate::{traits::StreamingTranscriber, TranscriptionModel};

pub struct DummyStreamingTranscriber {
    tx: mpsc::Sender<AppEvent>,
    audio_source: Option<Arc<dyn AudioFrameSource>>,
    tick_interval: Duration,
    worker: Mutex<WorkerState>,
}

#[derive(Debug, Default)]
struct WorkerState {
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
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
        Self {
            tx,
            audio_source,
            tick_interval,
            worker: Mutex::new(WorkerState::default()),
        }
    }
}

impl StreamingTranscriber for DummyStreamingTranscriber {
    async fn start(&self, _model: TranscriptionModel) {
        let mut worker = self
            .worker
            .lock()
            .expect("dummy transcriber mutex poisoned in start");

        if worker.task.is_some() {
            return;
        }

        let (stop_tx, mut stop_rx) = oneshot::channel();
        let tx = self.tx.clone();
        let audio_source = self.audio_source.clone();
        let tick_interval = self.tick_interval;

        let task = tokio::spawn(async move {
            let fake_chunks = ["listening... ", "partial phrase... ", "continuing... "];
            let mut chunk_index = 0usize;
            let mut ticker = time::interval(tick_interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Some(source) = audio_source.as_ref() {
                            if source.read_frame().is_none() {
                                continue;
                            }
                        }

                        let chunk = fake_chunks[chunk_index % fake_chunks.len()];
                        chunk_index += 1;

                        let payload = chunk.to_owned();

                        if tx.send(AppEvent::LiveText(payload)).await.is_err() {
                            break;
                        }
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });

        worker.stop_tx = Some(stop_tx);
        worker.task = Some(task);
    }

    async fn stop(&self) {
        let (stop_tx, task) = {
            let mut worker = self
                .worker
                .lock()
                .expect("dummy transcriber mutex poisoned in stop");
            (worker.stop_tx.take(), worker.task.take())
        };

        if let Some(stop_tx) = stop_tx {
            let _ = stop_tx.send(());
        }

        if let Some(task) = task {
            let _ = task.await;
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
    use crate::{traits::StreamingTranscriber, TranscriptionModel};

    struct TestAudioSource {
        frames: Mutex<VecDeque<PcmFrame>>,
    }

    impl TestAudioSource {
        fn with_frame_count(frame_count: usize) -> Self {
            let frame = PcmFrame::new(16_000, 1, vec![1, 2, 3, 4]);
            let frames = (0..frame_count)
                .map(|_| frame.clone())
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

        transcriber.start(TranscriptionModel::default()).await;

        let event = time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("timed out while waiting for live text");

        transcriber.stop().await;

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

        transcriber.start(TranscriptionModel::default()).await;

        let received = time::timeout(Duration::from_millis(80), rx.recv()).await;

        transcriber.stop().await;

        assert!(received.is_err(), "unexpected live text received");
    }
}
