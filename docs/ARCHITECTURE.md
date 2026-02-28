# Voxy Architecture

## Scope (Scaffold Phase)
This scaffold defines boundaries, data flow, and async integration points without implementing production audio or production STT.

## Workspace Modules
- `voxy-core`: Pure domain logic.
  - Buffer model (`BufferState`)
  - App state machine (`AppState` + pure transition function)
  - Shared event contract (`AppEvent`)
  - Core reducer (`CoreModel::reduce`) that returns executable commands
- `voxy-stt`: Streaming transcription abstraction.
  - `StreamingTranscriber` trait
  - `DummyStreamingTranscriber` that emits fake chunks on a timer
- `voxy-audio`: Audio input abstraction.
  - `AudioInput` trait
  - `NoopAudioInput` stub
- `voxy-app`: GTK4 UI shell.
  - Window layout and controls
  - Controller layer for event wiring and command execution
  - UI presenter layer for widget construction and render-only updates
  - Event loop bridging GTK main loop and Tokio channel

## Data Flow
1. UI control emits `AppEvent` (`MicToggled`, `ResetRequested`) into `tokio::mpsc`.
2. `voxy-app/controller` drains events on GTK main loop tick.
3. `voxy-core` reducer applies event and returns side-effect commands.
4. Controller executes commands (audio start/stop, transcriber start/stop, follow-up event emit).
5. `voxy-app/ui` renders a view model into widgets.
6. `voxy-stt` dummy transcriber emits `LiveText(String)` events into same channel.

## Anti-Drift Guard
- Domain behavior must be implemented in `voxy-core`, not GTK callbacks.
- GTK callbacks are limited to:
  - dispatching events
  - invoking render with prepared view model data
- New behavior should enter through `CoreModel::reduce` and command execution mapping.

## Thread Model
- GTK main thread owns widgets and rendering.
- Tokio runtime runs async tasks (dummy STT worker).
- Cross-component communication uses `tokio::mpsc` message passing.
- No background daemon, no global shortcuts, no DBus/portal integration in this phase.
