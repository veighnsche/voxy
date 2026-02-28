# Voxy Architecture

## Scope (Current Scaffold)
This scaffold defines boundaries, data flow, and async integration points without implementing production audio or production STT.

## Workspace Modules
- `voxy-core`: Pure domain logic.
  - Buffer model (`BufferState`)
  - App state machine (`AppState` + pure transition function)
  - Shared event contract (`AppEvent`)
  - UI preference model (`UiPrefs`) for visibility + window position
  - Core reducer (`CoreModel::reduce`) that returns executable commands
- `voxy-stt`: Streaming transcription abstraction.
  - `StreamingTranscriber` trait
  - `DummyStreamingTranscriber` that emits fake chunks on a timer
- `voxy-audio`: Audio input abstraction.
  - `AudioInput` trait
  - `NoopAudioInput` stub
- `voxy-app`: GTK4 UI shell.
  - `app/`: orchestration and lifecycle
  - `app/window/`: layer-shell + visibility + drag + close/clipboard behavior
  - `tray/`: system tray (StatusNotifier) integration
  - `wiring/`: runtime/channel/event-loop/command execution plumbing
  - `ui/`: render-only widgets and component composition
  - `diagnostics/`: opt-in smoke/test hooks

## Data Flow
1. UI and tray controls emit `AppEvent` into `tokio::mpsc`.
2. `voxy-app/wiring/event_loop` drains events on GTK main loop tick.
3. `voxy-core` reducer applies event and returns side-effect commands.
4. `voxy-app/wiring/command_bus` executes commands (audio/transcriber/window/clipboard/quit).
5. `voxy-app/app/view_sync` builds and renders a `ViewModel` from `CoreModel`.
6. `voxy-stt` dummy transcriber emits `LiveText(String)` events into the same channel.

## Anti-Drift Guard
- Domain behavior must be implemented in `voxy-core`, not GTK callbacks.
- GTK/tray callbacks are limited to dispatching events.
- New behavior should enter through `CoreModel::reduce` and command execution mapping.
- Window-only behavior should be implemented in `voxy-app/app/window/*`.
- Async plumbing should be implemented in `voxy-app/wiring/*`, not in UI modules.
- Visibility toggle stays in one window instance; no window recreation on visibility transitions.

## Thread Model
- GTK main thread owns widgets and rendering.
- Tokio runtime runs async tasks (dummy STT worker).
- Cross-component communication uses `tokio::mpsc` message passing.
- Tray integration uses StatusNotifier via DBus (Plasma-compatible path).
- No global shortcuts or background daemon in this phase.
