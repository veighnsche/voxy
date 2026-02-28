# voxy-stt

Streaming transcription abstraction for Voxy.

## Contains
- `StreamingTranscriber` bidirectional contract:
  - uplink input: `AudioFrame`, `Commit`, `Clear`
  - downlink output stream: `LiveText`, lifecycle, commit/clear events
- `DummyStreamingTranscriber` scaffold implementation
- `OpenAiRealtimeTranscriber` realtime WebSocket implementation

Runtime selection is handled by the app orchestration layer.
