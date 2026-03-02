pub mod config;
pub mod dummy;
pub mod error;
pub mod realtime;
pub mod trace;
pub mod traits;

pub use dummy::transcriber::DummyStreamingTranscriber;
pub use realtime::OpenAiRealtimeTranscriber;
pub use traits::{
    StreamingTranscriber, TranscriberContractError, TranscriberInput, TranscriberOutput,
    TranscriberSessionConfig, TranscriberStreamState,
};
