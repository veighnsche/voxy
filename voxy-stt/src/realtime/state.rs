#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Ready,
    Stopping,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}
