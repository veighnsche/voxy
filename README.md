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
- `voxy-core/`: buffer model, event model, state machine, reducer
- `voxy-stt/`: streaming transcriber abstraction + dummy implementation
- `voxy-audio/`: audio input abstraction + no-op implementation
- `docs/`: architecture and planning docs

## Requirements

- Rust stable
- GTK4 development packages visible to `pkg-config`

Example Linux dependencies:

```bash
# Ubuntu/Debian
sudo apt-get install -y pkg-config libgtk-4-dev libgraphene-1.0-dev

# Fedora
sudo dnf install -y pkgconf-pkg-config gtk4-devel graphene-devel
```

## Local Development

```bash
cargo fmt --all
cargo check -p voxy-core -p voxy-audio -p voxy-stt
cargo test -p voxy-core
cargo run -p voxy-app
```

## Architecture Docs

- [Architecture](docs/ARCHITECTURE.md)
- [State Machine](docs/STATE_MACHINE.md)
- [Buffer Model](docs/BUFFER_MODEL.md)
- [UX Rules](docs/UX_RULES.md)
- [Roadmap](docs/ROADMAP.md)

## Contributing and Governance

- [Contributing Guide](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security Policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## License

MIT. See [LICENSE](LICENSE).
