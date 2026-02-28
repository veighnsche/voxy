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
