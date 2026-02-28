use tokio::sync::mpsc;
use voxy_core::AppEvent;

const EVENT_CHANNEL_CAPACITY: usize = 64;

pub struct AppChannels {
    pub event_tx: mpsc::Sender<AppEvent>,
    pub event_rx: mpsc::Receiver<AppEvent>,
}

pub fn build_event_channels() -> AppChannels {
    let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
    AppChannels { event_tx, event_rx }
}
