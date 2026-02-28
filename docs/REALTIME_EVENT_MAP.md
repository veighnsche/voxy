# Realtime Event Map (Planned)

## Client -> Server
- `transcription_session.update`
- `input_audio_buffer.append`
- `input_audio_buffer.commit`
- `input_audio_buffer.clear`

## Server -> Client
- `conversation.item.input_audio_transcription.delta`
- `conversation.item.input_audio_transcription.completed`
- `conversation.item.input_audio_transcription.failed`
- `error`

## Voxy Mapping
- `...delta` -> `AppEvent::LiveText(String)`
- `...completed` -> `AppEvent::CommitRequested`
- `...failed` / `error` -> `AppEvent::RuntimeError(String)`
