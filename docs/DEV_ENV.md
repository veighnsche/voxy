# Development Environment

Voxy's GTK app depends on system libraries discovered through `pkg-config`.

## Contract

A machine is considered dev-ready for GUI work when all checks pass:

```bash
just doctor
```

This validates:
- `cargo`
- `pkg-config`
- `gtk4` pkg-config module (`gtk4.pc`)
- `graphene-gobject-1.0` pkg-config module (`graphene-gobject-1.0.pc`)
- `gtk4-layer-shell-0` pkg-config module (`gtk4-layer-shell-0.pc`)
- `alsa` pkg-config module (`alsa.pc`) for CPAL microphone capture
- optional file watcher (`watchexec` or `cargo-watch`) for `just dev`

## Install Dependencies

Use the repository installer (distro-aware):

```bash
just deps
```

Supported distro families:
- Debian/Ubuntu (`apt`)
- Fedora/RHEL-like (`dnf`)
- Arch-like (`pacman`)
- openSUSE (`zypper`)

## Canonical GUI Entry Point

```bash
just dev
```

`just dev` is the canonical UI command:
- with `watchexec` or `cargo-watch`, it auto-restarts on file changes
- without a watcher, it falls back to a normal single-session GUI run

## Latency Tuning Knobs

You can tune live-text responsiveness with environment variables:

- `VOXY_STT_BACKEND` (default `openai_api`): STT backend selector.
  - Supported values: `openai_api`, `openai`, `dummy`
- `VOXY_UI_EVENT_POLL_MS` (default `16`): GTK event-loop drain cadence.
- `VOXY_STT_SOURCE_POLL_MS` (default `20`): realtime uplink source polling cadence.
- `VOXY_STT_VAD_SILENCE_MS` (default `1600`): server VAD silence window before auto-commit.
- `VOXY_STT_RECONNECT_ENABLED` (default `true`): enable reconnect loop after retryable websocket failures.
- `VOXY_STT_RECONNECT_MAX_RETRIES` (default `0` = unlimited): maximum reconnect retries before surfacing fatal failure.
- `VOXY_STT_RECONNECT_BASE_MS` (default `250`): initial reconnect backoff delay.
- `VOXY_STT_RECONNECT_MAX_MS` (default `5000`): reconnect backoff cap.
- `VOXY_AUDIO_FRAME_MS` (default `20`): CPAL frame duration per audio chunk.
- `VOXY_MAX_RECORDING_SECONDS` (default `1800`): hard stop to prevent runaway recording sessions (`0` disables).
- `VOXY_SILENCE_AUTO_STOP_SECONDS` (default `10`): initial silence auto-stop timeout shown in settings (`0` disables).

Settings persistence:
- Silence timeout is persisted to `$XDG_CONFIG_HOME/voxy/settings.json`.
- Silence gate threshold (IN meter click position) is persisted to `$XDG_CONFIG_HOME/voxy/settings.json`.
- VAD silence window (Settings -> Recording -> VAD pause) is persisted to `$XDG_CONFIG_HOME/voxy/settings.json`.
- If `XDG_CONFIG_HOME` is unset, fallback path is `~/.config/voxy/settings.json`.

Example:

```bash
VOXY_UI_EVENT_POLL_MS=16 VOXY_STT_SOURCE_POLL_MS=15 VOXY_AUDIO_FRAME_MS=15 just gui-trace
```

## One-Shot GUI Run

```bash
just gui
```

## GUI Validation Tasks

Run `xtask` GUI checks:

```bash
just validate
# or directly:
cargo run -p xtask -- gui smoke
cargo run -p xtask -- gui lifecycle
cargo run -p xtask -- gui reset-flow
cargo run -p xtask -- gui visibility-toggle-flow
cargo run -p xtask -- gui visibility-smoke
cargo run -p xtask -- gui visibility-window-guard
```

- `gui smoke`: launch, verify running, SIGTERM, verify shutdown.
- `gui lifecycle`: launch with auto-close hook and verify clean exit.
- `gui reset-flow`: inject reset event on startup, auto-close, verify clean exit.
- `gui visibility-toggle-flow`: inject visibility toggle and verify GUI remains healthy.
- `gui visibility-smoke`: extra visibility smoke coverage.
- `gui visibility-window-guard`: repeated visibility toggles, ensure single-window invariant.

`xtask` GUI runs force `GTK_THEME=Adwaita` for deterministic output and to avoid host-theme-specific GTK warnings.
