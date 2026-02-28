use crate::TranscriptionModel;

#[allow(async_fn_in_trait)]
pub trait StreamingTranscriber: Send + Sync {
    async fn start(&self, model: TranscriptionModel);
    async fn stop(&self);
}
