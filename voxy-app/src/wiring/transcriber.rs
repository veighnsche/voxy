use std::{env, sync::Arc};

use tokio::sync::{broadcast, mpsc};
use voxy_audio::AudioFrameSource;
use voxy_core::AppEvent;
use voxy_stt::{
    DummyStreamingTranscriber, OpenAiRealtimeTranscriber, StreamingTranscriber,
    TranscriberContractError, TranscriberInput, TranscriberOutput, TranscriberSessionConfig,
    TranscriberStreamState,
};

use crate::diagnostics::pipeline_trace;

const STT_BACKEND_ENV: &str = "VOXY_STT_BACKEND";
const STT_BACKEND_DUMMY: &str = "dummy";
const STT_BACKEND_OPENAI_API: &str = "openai_api";
const STT_BACKEND_OPENAI_API_ALIAS: &str = "openai";

pub enum AppTranscriber {
    Dummy(DummyStreamingTranscriber),
    OpenAiApi(OpenAiRealtimeTranscriber),
}

impl AppTranscriber {
    pub fn from_env(
        event_tx: mpsc::Sender<AppEvent>,
        audio_source: Option<Arc<dyn AudioFrameSource>>,
    ) -> Self {
        let backend = env::var(STT_BACKEND_ENV)
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| STT_BACKEND_OPENAI_API.to_owned());
        pipeline_trace::log(
            "transcriber",
            format!("VOXY_STT_BACKEND resolved to '{backend}'"),
        );

        match backend.as_str() {
            STT_BACKEND_DUMMY => {
                pipeline_trace::log("transcriber", "using dummy backend");
                Self::Dummy(DummyStreamingTranscriber::new(event_tx, audio_source))
            }
            STT_BACKEND_OPENAI_API | STT_BACKEND_OPENAI_API_ALIAS => {
                pipeline_trace::log("transcriber", "using openai_api backend");
                Self::OpenAiApi(OpenAiRealtimeTranscriber::new(event_tx, audio_source))
            }
            _ => {
                pipeline_trace::log(
                    "transcriber",
                    "unknown backend requested, falling back to openai_api",
                );
                Self::OpenAiApi(OpenAiRealtimeTranscriber::new(event_tx, audio_source))
            }
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match self {
            Self::Dummy(_) => STT_BACKEND_DUMMY,
            Self::OpenAiApi(_) => STT_BACKEND_OPENAI_API,
        }
    }
}

impl StreamingTranscriber for AppTranscriber {
    async fn start(
        &self,
        config: TranscriberSessionConfig,
    ) -> Result<(), TranscriberContractError> {
        match self {
            Self::Dummy(transcriber) => transcriber.start(config).await,
            Self::OpenAiApi(transcriber) => transcriber.start(config).await,
        }
    }

    async fn push_input(&self, input: TranscriberInput) -> Result<(), TranscriberContractError> {
        match self {
            Self::Dummy(transcriber) => transcriber.push_input(input).await,
            Self::OpenAiApi(transcriber) => transcriber.push_input(input).await,
        }
    }

    async fn stop(&self) -> Result<(), TranscriberContractError> {
        match self {
            Self::Dummy(transcriber) => transcriber.stop().await,
            Self::OpenAiApi(transcriber) => transcriber.stop().await,
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<TranscriberOutput> {
        match self {
            Self::Dummy(transcriber) => transcriber.subscribe(),
            Self::OpenAiApi(transcriber) => transcriber.subscribe(),
        }
    }

    fn state(&self) -> TranscriberStreamState {
        match self {
            Self::Dummy(transcriber) => transcriber.state(),
            Self::OpenAiApi(transcriber) => transcriber.state(),
        }
    }
}
