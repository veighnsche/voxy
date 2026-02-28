use std::sync::Mutex;

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::{self, Duration},
};
use voxy_core::AppEvent;

#[allow(async_fn_in_trait)]
pub trait StreamingTranscriber: Send + Sync {
    async fn start(&self);
    async fn stop(&self);
}

#[derive(Debug)]
pub struct DummyStreamingTranscriber {
    tx: mpsc::Sender<AppEvent>,
    worker: Mutex<WorkerState>,
}

#[derive(Debug, Default)]
struct WorkerState {
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl DummyStreamingTranscriber {
    pub fn new(tx: mpsc::Sender<AppEvent>) -> Self {
        Self {
            tx,
            worker: Mutex::new(WorkerState::default()),
        }
    }
}

impl StreamingTranscriber for DummyStreamingTranscriber {
    async fn start(&self) {
        let mut worker = self
            .worker
            .lock()
            .expect("dummy transcriber mutex poisoned in start");

        if worker.task.is_some() {
            return;
        }

        let (stop_tx, mut stop_rx) = oneshot::channel();
        let tx = self.tx.clone();

        let task = tokio::spawn(async move {
            let fake_chunks = [
                "[stub] listening... ",
                "[stub] partial phrase... ",
                "[stub] continuing... ",
            ];
            let mut chunk_index = 0usize;
            let mut ticker = time::interval(Duration::from_secs(2));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let chunk = fake_chunks[chunk_index % fake_chunks.len()];
                        chunk_index += 1;

                        if tx.send(AppEvent::LiveText(chunk.to_owned())).await.is_err() {
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
