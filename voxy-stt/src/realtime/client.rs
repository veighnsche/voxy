use crate::{
    realtime::{session::SessionConfig, state::ConnectionState},
    TranscriptionModel,
};

#[derive(Debug, Default)]
pub struct RealtimeTranscriberClient {
    pub state: ConnectionState,
}

impl RealtimeTranscriberClient {
    pub fn connect(&mut self, model: TranscriptionModel) -> SessionConfig {
        self.state = ConnectionState::Connecting;
        SessionConfig::for_model(model)
    }

    pub fn mark_ready(&mut self) {
        self.state = ConnectionState::Ready;
    }

    pub fn disconnect(&mut self) {
        self.state = ConnectionState::Disconnected;
    }
}
