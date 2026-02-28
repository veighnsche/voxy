# Voxy

[![CI](https://github.com/veighnsche/voxy/actions/workflows/ci.yml/badge.svg)](https://github.com/veighnsche/voxy/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

Voxy is a Wayland-native GTK4 Rust app scaffold for live streaming speech-to-text into an editable text area.

## Project Status

Pre-alpha scaffold.

Implemented:
- Workspace and module boundaries
- Core state and buffer model
- Stub audio/STT integrations
- GTK shell with explicit controls and event wiring

Intentionally not implemented yet:
- Production audio capture
- Production STT integration
- DBus/portal/global hotkeys/background services

## Workspace Layout

- `voxy-app/`: GTK4 application shell (controller + render layer)
  - Internal split: `app/` (orchestration), `wiring/` (runtime/channels), `ui/` (render), `diagnostics/` (smoke hooks)
- `voxy-core/`: buffer model, event model, state machine, reducer
- `voxy-stt/`: streaming transcriber abstraction + dummy implementation
- `voxy-audio/`: audio input abstraction + no-op implementation
- `docs/`: architecture and planning docs

## Requirements

- Rust stable
- GTK4 + gtk4-layer-shell development packages visible to `pkg-config`

Example Linux dependencies:

```bash
# Ubuntu/Debian
sudo apt-get install -y pkg-config libgtk-4-dev libgraphene-1.0-dev libgtk4-layer-shell-dev

# Fedora
sudo dnf install -y pkgconf-pkg-config gtk4-devel graphene-devel gtk4-layer-shell-devel
```

## Reproducible Setup

```bash
just deps    # install system GTK dependencies for your distro
just doctor  # verify toolchain + pkg-config modules
```

## Local Development

```bash
cargo fmt --all
cargo check -p voxy-core -p voxy-audio -p voxy-stt
cargo test -p voxy-core
just dev
```

`just dev` is the canonical UI entry point:
- with `watchexec`/`cargo-watch`, it auto-restarts on file changes
- without a watcher, it runs a normal single-session GUI

For a direct single-session run:

```bash
just gui
```

GUI smoke test via `xtask` (launch, signal, verify shutdown):

```bash
just validate
# or directly:
cargo run -p xtask -- gui smoke
cargo run -p xtask -- gui lifecycle
cargo run -p xtask -- gui reset-flow
cargo run -p xtask -- gui pin-flow
cargo run -p xtask -- gui pin-unsupported
```

## Architecture Docs

- [Architecture](docs/ARCHITECTURE.md)
- [State Machine](docs/STATE_MACHINE.md)
- [Buffer Model](docs/BUFFER_MODEL.md)
- [UX Rules](docs/UX_RULES.md)
- [Roadmap](docs/ROADMAP.md)
- [Development Environment](docs/DEV_ENV.md)
- [App Wiring](docs/APP_WIRING.md)

## Contributing and Governance

- [Contributing Guide](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security Policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## License

MIT. See [LICENSE](LICENSE).
