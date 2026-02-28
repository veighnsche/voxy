pub trait AudioInput: Send + Sync {
    fn start(&self);
    fn stop(&self);
}

#[derive(Debug, Default)]
pub struct NoopAudioInput;

impl AudioInput for NoopAudioInput {
    fn start(&self) {}

    fn stop(&self) {}
}
