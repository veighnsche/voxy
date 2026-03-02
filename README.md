# Voxy

[![CI](https://github.com/veighnsche/voxy/actions/workflows/ci.yml/badge.svg)](https://github.com/veighnsche/voxy/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

Voxy is a GTK4 Rust desktop app for live microphone transcription into an editable text area.

## Project Status

Pre-alpha. The app is usable for local live transcription workflows, but APIs and UX may still change quickly.

## Current Features

- Live microphone capture with CPAL (`voxy-audio`) and realtime input level meter.
- OpenAI Realtime WebSocket transcription (`voxy-stt`) with model selection:
  - `gpt-4o-mini-transcribe`
  - `gpt-4o-transcribe`
- Bidirectional streaming flow:
  - audio frames streamed continuously while recording
  - partial transcript deltas rendered live
  - segment commit on stop and on server completion
- Editable transcript buffer with:
  - live + committed text merge behavior
  - `Copy` to clipboard
  - `Reset` clear action
- Recording controls and guardrails:
  - start/stop recording button
  - `Ctrl+Space` toggle shortcut (window-focused)
  - silence auto-stop timeout
  - configurable VAD pause window (ms)
  - max recording duration guard
- Click-to-set silence gate threshold on the `IN` meter, with countdown display.
- Settings pane with recording controls plus API key setup instructions.
- Settings persistence to JSON in user config dir:
  - silence timeout
  - silence gate threshold
  - VAD pause
- Tray/status notifier integration:
  - Show/Hide
  - Move To Next Screen
  - Reset
  - Size + / Size -
  - Quit
- Window behavior:
  - close button and window close request hide instead of exit
  - drag surface at top
  - bottom-right resize handle
  - layer-shell placement on supported Wayland compositors

## Workspace Layout

- `voxy-app/`: GTK4 application shell (controller + render layer)
  - Internal split: `app/` (orchestration), `wiring/` (runtime/channels), `ui/` (render), `diagnostics/` (smoke hooks)
- `voxy-core/`: buffer model, event model, state machine, reducer, side-effect command planning
- `voxy-stt/`: streaming transcriber abstraction + OpenAI realtime + dummy backend
- `voxy-audio/`: audio capture engine + CPAL adapter + fixture injection path
- `xtask/`: GUI automation tasks used by CI/local validation
- `docs/`: architecture and planning docs

## Requirements

- Rust stable toolchain
- Linux desktop with GTK4 + gtk4-layer-shell dev packages visible to `pkg-config`
- ALSA dev package for CPAL microphone capture
- Optional: `watchexec` or `cargo-watch` for auto-restart `just dev`

Example Linux dependencies:

```bash
# Ubuntu/Debian
sudo apt-get install -y pkg-config libgtk-4-dev libgraphene-1.0-dev libgtk4-layer-shell-dev libasound2-dev

# Fedora
sudo dnf install -y pkgconf-pkg-config gtk4-devel graphene-devel gtk4-layer-shell-devel alsa-lib-devel
```

## Reproducible Setup

```bash
just deps    # install system GTK dependencies for your distro
just doctor  # verify toolchain + pkg-config modules
```

## Quick Start

```bash
just deps
just doctor

# choose one API key method:
export VOXY_OPENAI_API_KEY="sk-..."
# or:
# export VOXY_OPENAI_API_KEY_FILE="/absolute/path/to/key.txt"
# or:
# export OPENAI_API_KEY="sk-..."

just gui
```

If you want to run without a real API key, use the dummy backend:

```bash
VOXY_STT_BACKEND=dummy just gui
```

## API Key Policy

- UI does not store or display key material.
- Lookup order:
  - `VOXY_OPENAI_API_KEY`
  - `VOXY_OPENAI_API_KEY_FILE`
  - `OPENAI_API_KEY`
  - `.env`
  - `.env.local` (overrides `.env`)
- Missing key surfaces as a runtime error banner.

## Runtime Configuration

Environment variables:

- `VOXY_STT_BACKEND` (`openai_api` default): `openai_api`, `openai`, `dummy`.
- `VOXY_OPENAI_REALTIME_URL` (default `wss://api.openai.com/v1/realtime?intent=transcription`).
- `VOXY_UI_EVENT_POLL_MS` (default `16`): UI event loop poll cadence.
- `VOXY_STT_SOURCE_POLL_MS` (default `20`): audio source poll cadence for STT uplink.
- `VOXY_STT_RECONNECT_ENABLED` (default `true`): enable reconnect on retryable websocket failures.
- `VOXY_STT_RECONNECT_MAX_RETRIES` (default `0` = unlimited): reconnect retry cap.
- `VOXY_STT_RECONNECT_BASE_MS` (default `250`): initial reconnect backoff delay.
- `VOXY_STT_RECONNECT_MAX_MS` (default `5000`): reconnect backoff cap.
- `VOXY_AUDIO_FRAME_MS` (default `20`): capture frame size.
- `VOXY_STT_VAD_SILENCE_MS` (default `1600`): initial VAD pause.
- `VOXY_SILENCE_AUTO_STOP_SECONDS` (default `10`): initial silence timeout.
- `VOXY_MAX_RECORDING_SECONDS` (default `1800`, `0` disables hard stop).
- `VOXY_TRACE_PIPELINE` (`1/true/on` to enable pipeline tracing).
- `VOXY_TRACE_PIPELINE_EVERY` (default `20`) and `VOXY_TRACE_PIPELINE_NOISY_EVERY`.
- `VOXY_APP_ID` (default `com.vince.voxy`) and `VOXY_NON_UNIQUE`.
- `VOXY_DRAG_MATH` (`legacy` or `anchor`) for compositor-specific drag behavior.

Persisted settings file:

- `$XDG_CONFIG_HOME/voxy/settings.json`
- fallback: `~/.config/voxy/settings.json`

Persisted keys:

- `silence_auto_stop_seconds`
- `silence_gate_threshold`
- `vad_silence_ms`

## Development

```bash
cargo fmt --all
cargo check
just dev
```

`just dev` is the canonical UI entry point:

- with `watchexec` / `cargo-watch`: auto-restarts on source changes
- without a watcher: single-session GUI run

Trace-enabled run:

```bash
just gui-trace every=10
```

## Validation

```bash
just validate
```

Or run GUI tasks directly:

```bash
cargo run -p xtask -- gui smoke
cargo run -p xtask -- gui lifecycle
cargo run -p xtask -- gui reset-flow
cargo run -p xtask -- gui visibility-toggle-flow
cargo run -p xtask -- gui visibility-smoke
cargo run -p xtask -- gui visibility-window-guard
cargo run -p xtask -- gui drag-math-sim
```

## Docs

- [Architecture](docs/ARCHITECTURE.md)
- [State Machine](docs/STATE_MACHINE.md)
- [Buffer Model](docs/BUFFER_MODEL.md)
- [App Wiring](docs/APP_WIRING.md)
- [Development Environment](docs/DEV_ENV.md)
- [API Key Ingestion](docs/API_KEY_INGESTION.md)
- [Realtime Event Map](docs/REALTIME_EVENT_MAP.md)
- [Production Readiness Checklist](docs/PRODUCTION_READINESS_CHECKLIST.md)
- [Operations Guide](docs/OPERATIONS.md)
- [Roadmap](docs/ROADMAP.md)

## Contributing and Governance

- [Contributing Guide](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security Policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## License

MIT. See [LICENSE](LICENSE).
