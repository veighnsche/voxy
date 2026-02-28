pub mod config;
pub mod dummy;
pub mod error;
pub mod model;
pub mod realtime;
pub mod trace;
pub mod traits;

pub use dummy::transcriber::DummyStreamingTranscriber;
pub use model::TranscriptionModel;
pub use realtime::OpenAiRealtimeTranscriber;
pub use traits::{
    StreamingTranscriber, TranscriberContractError, TranscriberInput, TranscriberOutput,
    TranscriberSessionConfig, TranscriberStreamState,
};
