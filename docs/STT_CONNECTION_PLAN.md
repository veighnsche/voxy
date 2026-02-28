# STT Connection Plan (OpenAI Realtime Transcription)

## Goal
Replace `DummyStreamingTranscriber` with a real OpenAI streaming STT client while preserving current module boundaries:
- `voxy-core`: state/events/commands only
- `voxy-stt`: network protocol + stream lifecycle
- `voxy-audio`: PCM frame source
- `voxy-app`: orchestration + rendering

## Current Decisions
- Transport: **WebSocket** (best fit for Rust desktop app runtime).
- API family: **Realtime transcription sessions**.
- Default model: `gpt-4o-mini-transcribe` (selected in app wiring, owned by `voxy-stt`).
- Alternative model: `gpt-4o-transcribe`.

## Relevant OpenAI Docs
- Speech-to-text guide: <https://developers.openai.com/api/docs/guides/speech-to-text>
- Realtime transcription guide: <https://platform.openai.com/docs/guides/realtime-transcription>
- Realtime input audio buffer events (`append`, `commit`, `clear`): <https://platform.openai.com/docs/api-reference/realtime-client-events/input_audio_buffer/append>
- Realtime transcription session API (`transcription_session.update`): <https://platform.openai.com/docs/api-reference/realtime-transcription-sessions/create>

## Protocol Mapping (Planned)
1. Connect WebSocket to Realtime API.
2. Send `transcription_session.update` with:
   - `input_audio_format: "pcm16"`
   - `input_audio_transcription.model` from selected `voxy-stt::TranscriptionModel`
   - turn detection strategy (start with server VAD)
3. While recording:
   - send `input_audio_buffer.append` with base64 PCM chunks
   - send `input_audio_buffer.commit` at explicit boundaries/stop
4. Consume server events:
   - `conversation.item.input_audio_transcription.delta` -> `AppEvent::LiveText(...)`
   - `conversation.item.input_audio_transcription.completed` -> `AppEvent::CommitRequested`
   - `conversation.item.input_audio_transcription.failed`/`error` -> `AppEvent::RuntimeError(...)`

## Module-Level Implementation Steps
### Phase A: `voxy-stt` transport skeleton
- Add websocket client task lifecycle (connect/reconnect/stop).
- Keep transcriber trait boundary small and async-safe.
- Add message parsing enums for only required event subset.

### Phase B: audio-to-wire framing
- Define a PCM frame input contract from `voxy-audio`.
- Base64 encode PCM16 frames for `input_audio_buffer.append`.
- Add deterministic flush/commit on stop.

### Phase C: event translation into core
- Map delta/completed/failed events into existing `AppEvent`.
- Keep buffer rules unchanged:
  - stream only mutates `live_segment`
  - commit finalizes via `CommitRequested`

### Phase D: resilience and observability
- Structured retry policy with explicit backoff caps.
- Explicit surfaced errors (no silent failure).
- Optional debug logging guard for protocol-level traces.

## Non-Goals (for this phase)
- No diarization UI.
- No background daemon.
- No global shortcuts.
- No hidden automatic destructive behavior.

## Open Questions Before Production Rollout
- Final server VAD settings (latency vs stability tradeoff).
- Whether to include confidence/logprobs in future UI.
- Whether to support non-realtime `/audio/transcriptions` fallback path.
