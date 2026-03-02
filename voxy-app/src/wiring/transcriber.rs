use std::{env, sync::Arc};

use tokio::sync::{broadcast, mpsc};
use voxy_audio::AudioFrameSource;
use voxy_core::AppEvent;
use voxy_stt::{
    DummyStreamingTranscriber, OpenAiRealtimeTranscriber, StreamingTranscriber,
    TranscriberContractError, TranscriberInput, TranscriberOutput, TranscriberSessionConfig,
    TranscriberStreamState, TranscriptionModel,
};

use crate::diagnostics::pipeline_trace;

const STT_BACKEND_ENV: &str = "VOXY_STT_BACKEND";
const STT_BACKEND_AUTO: &str = "auto";
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
        let backend_raw = env::var(STT_BACKEND_ENV).ok();
        let backend = normalize_backend_value(backend_raw.as_deref());
        pipeline_trace::log(
            "transcriber",
            format!("VOXY_STT_BACKEND resolved to '{backend}'"),
        );

        match backend.as_str() {
            STT_BACKEND_DUMMY => {
                pipeline_trace::log("transcriber", "using dummy backend");
                Self::Dummy(DummyStreamingTranscriber::new(event_tx, audio_source))
            }
            STT_BACKEND_OPENAI_API | STT_BACKEND_OPENAI_API_ALIAS | STT_BACKEND_AUTO => {
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

    pub fn supports_model(&self, _model: TranscriptionModel) -> bool {
        true
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

fn normalize_backend_value(value: Option<&str>) -> String {
    value
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| STT_BACKEND_OPENAI_API.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{normalize_backend_value, STT_BACKEND_OPENAI_API};

    #[test]
    fn normalize_backend_defaults_to_openai_api_when_missing_or_blank() {
        assert_eq!(normalize_backend_value(None), STT_BACKEND_OPENAI_API);
        assert_eq!(normalize_backend_value(Some("   ")), STT_BACKEND_OPENAI_API);
    }

    #[test]
    fn normalize_backend_trims_and_lowercases_values() {
        assert_eq!(normalize_backend_value(Some("  DUMMY  ")), "dummy");
        assert_eq!(normalize_backend_value(Some(" OpenAI ")), "openai");
    }
}
