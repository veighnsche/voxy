use std::sync::Arc;

use voxy_core::{AppEvent, TranscriptionModelId};
use voxy_stt::{StreamingTranscriber, TranscriberSessionConfig};

use crate::diagnostics::pipeline_trace;

use super::CommandBus;

impl CommandBus {
    pub(super) fn handle_start_transcriber(
        &self,
        model: TranscriptionModelId,
        vad_silence_duration_ms: u32,
    ) {
        let transcriber = Arc::clone(&self.transcriber);
        let audio_input = Arc::clone(&self.audio_input);
        if !transcriber.supports_model(model) {
            let backend = transcriber.backend_name();
            self.emit_runtime_error(format!(
                "Selected model '{}' is not supported by active backend '{}'.",
                model.as_api_id(),
                backend
            ));
            self.emit_log_message(format!(
                "Transcriber start blocked (backend '{}' does not support model '{}')",
                backend,
                model.as_api_id()
            ));
            pipeline_trace::log(
                "command",
                format!(
                    "StartTranscriber blocked model={} backend={}",
                    model.as_api_id(),
                    backend
                ),
            );
            self.rollback_recording_start();
            return;
        }
        let event_tx = self.event_tx.clone();
        self.runtime.spawn(async move {
            let mut config = TranscriberSessionConfig::from_model(model);
            config.vad_silence_duration_ms = vad_silence_duration_ms;
            pipeline_trace::log(
                "command",
                format!(
                    "StartTranscriber async start model={} vad_silence_ms={}",
                    model.as_api_id(),
                    config.vad_silence_duration_ms
                ),
            );
            if let Err(error) = transcriber.start(config).await {
                if let Err(stop_error) = audio_input.stop_checked() {
                    let _ = event_tx
                        .send(AppEvent::RuntimeError(format!(
                            "failed to stop audio input after transcriber start error: {stop_error}"
                        )))
                        .await;
                    pipeline_trace::log(
                        "command",
                        format!("StartTranscriber rollback stop_audio error={stop_error}"),
                    );
                } else {
                    pipeline_trace::log("command", "StartTranscriber rollback stop_audio ok");
                }
                let _ = event_tx.send(AppEvent::RecordingStartRejected).await;
                let _ = event_tx
                    .send(AppEvent::RuntimeError(format!(
                        "failed to start transcriber: {error}"
                    )))
                    .await;
                pipeline_trace::log("command", format!("StartTranscriber error={error}"));
            } else {
                pipeline_trace::log("command", "StartTranscriber started");
            }
        });
        self.emit_log_message(format!("Transcriber started ({})", model.as_api_id()));
    }

    pub(super) fn handle_stop_transcriber(&self) {
        let transcriber = Arc::clone(&self.transcriber);
        let event_tx = self.event_tx.clone();
        self.runtime.spawn(async move {
            if let Err(error) = transcriber.stop().await {
                let _ = event_tx
                    .send(AppEvent::RuntimeError(format!(
                        "failed to stop transcriber: {error}"
                    )))
                    .await;
                pipeline_trace::log("command", format!("StopTranscriber error={error}"));
            } else {
                pipeline_trace::log("command", "StopTranscriber stopped");
            }
        });
        self.emit_log_message("Transcriber stop requested");
    }

    pub(super) fn handle_stop_transcriber_then_emit(&self, event: AppEvent) {
        let transcriber = Arc::clone(&self.transcriber);
        let event_tx = self.event_tx.clone();
        self.runtime.spawn(async move {
            if let Err(error) = transcriber.stop().await {
                let _ = event_tx
                    .send(AppEvent::RuntimeError(format!(
                        "failed to stop transcriber before emit: {error}"
                    )))
                    .await;
                pipeline_trace::log(
                    "command",
                    format!("StopTranscriberThenEmit stop error={error}"),
                );
            }
            pipeline_trace::log("command", format!("StopTranscriberThenEmit emit {event:?}"));
            let _ = event_tx.send(event).await;
        });
        self.emit_log_message("Transcriber stopping; commit scheduled");
    }

    pub(super) fn handle_emit_event(&self, event: AppEvent) {
        let event_tx = self.event_tx.clone();
        self.runtime.spawn(async move {
            pipeline_trace::log("command", format!("EmitEvent send {event:?}"));
            let _ = event_tx.send(event).await;
        });
    }
}
