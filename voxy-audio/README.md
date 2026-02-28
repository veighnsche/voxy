# voxy-audio

Audio input abstraction and adapter engine for Voxy.

## Contains
- `AudioInput` trait
- `AudioFrameSource` trait
- `InputEngine` route/session orchestrator
- `AudioRoute` (microphone)
- real `CpalFrameSource` microphone capture path
- `NoopAudioInput` compatibility adapter
