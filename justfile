set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

deps:
    ./scripts/dev/install-system-deps.sh

doctor:
    ./scripts/dev/doctor.sh

install: doctor
    ./scripts/dev/install-desktop.sh

validate: doctor
    cargo fmt --all -- --check
    cargo check
    cargo test -p voxy-core
    cargo test -p voxy-audio
    cargo test -p voxy-stt
    cargo run -p xtask -- gui smoke
    cargo run -p xtask -- gui lifecycle
    cargo run -p xtask -- gui reset-flow
    cargo run -p xtask -- gui visibility-toggle-flow
    cargo run -p xtask -- gui visibility-smoke
    cargo run -p xtask -- gui visibility-window-guard

gui: doctor
    cargo run -p voxy-app

gui-trace every="10": doctor
    VOXY_TRACE_PIPELINE=1 VOXY_TRACE_PIPELINE_EVERY={{every}} cargo run -p voxy-app

# Canonical UI dev entry point:
# - with watcher: restarts app on source/config changes
# - without watcher: runs a normal single app session
dev: doctor
    #!/usr/bin/env bash
    set -euo pipefail

    if command -v watchexec >/dev/null 2>&1; then
      exec watchexec \
        --restart \
        --clear \
        --watch voxy-app/src \
        --watch voxy-core/src \
        --watch voxy-stt/src \
        --watch voxy-audio/src \
        --watch Cargo.toml \
        --watch Cargo.lock \
        --exts rs,toml \
        -- cargo run -p voxy-app
    elif command -v cargo-watch >/dev/null 2>&1; then
      exec cargo watch \
        --watch voxy-app/src \
        --watch voxy-core/src \
        --watch voxy-stt/src \
        --watch voxy-audio/src \
        --watch Cargo.toml \
        --watch Cargo.lock \
        -x 'run -p voxy-app'
    else
      echo "No file watcher found; running single-session GUI."
      echo "Install one for auto-restart:"
      echo "  cargo install watchexec-cli"
      echo "or"
      echo "  cargo install cargo-watch"
      exec cargo run -p voxy-app
    fi
