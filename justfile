set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

deps:
    ./scripts/dev/install-system-deps.sh

doctor:
    ./scripts/dev/doctor.sh

validate: doctor
    cargo fmt --all -- --check
    cargo check
    cargo test -p voxy-core
    cargo run -p xtask -- gui smoke
    cargo run -p xtask -- gui lifecycle
    cargo run -p xtask -- gui reset-flow
    cargo run -p xtask -- gui visibility-toggle-flow
    cargo run -p xtask -- gui visibility-smoke
    cargo run -p xtask -- gui visibility-window-guard

gui: doctor
    cargo run -p voxy-app

fixtures-fetch:
    cargo run -p xtask -- fixtures fetch-audio

fixtures-verify:
    cargo run -p xtask -- fixtures verify-audio

e2e-stt-live: doctor
    cargo run -p xtask -- gui stt-e2e

# Fast UI loop: restarts app on source/config changes.
# Requires one of: watchexec, cargo-watch.
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
      echo "Install a watcher first:"
      echo "  cargo install watchexec-cli"
      echo "or"
      echo "  cargo install cargo-watch"
      exit 1
    fi
