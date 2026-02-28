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

## Run the GUI

```bash
just gui
```

## UI Iteration Loop

```bash
just dev
```

`just dev` is auto-restart on file changes, not live widget hot reload.

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
