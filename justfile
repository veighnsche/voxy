set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

deps:
    ./scripts/dev/install-system-deps.sh

doctor:
    ./scripts/dev/doctor.sh

make target action:
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ "{{target}}" == "rpm" && "{{action}}" == "package" ]]; then
      exec ./scripts/release/build-rpm.sh
    fi

    echo "Unsupported make target: {{target}} {{action}}" >&2
    echo "Usage: just make rpm package" >&2
    exit 1

validate: doctor
    cargo fmt --all -- --check
    cargo check --workspace
    cargo test -p voxy-core
    cargo test -p voxy-audio
    cargo test -p voxy-stt
    cargo test -p voxy-app
    cargo clippy --workspace --all-targets -- -D warnings
    just validate-packaging
    cargo run -p xtask -- gui smoke
    cargo run -p xtask -- gui lifecycle
    cargo run -p xtask -- gui quit-fallback
    cargo run -p xtask -- gui reset-flow
    cargo run -p xtask -- gui stt-fixture-smoke
    cargo run -p xtask -- gui visibility-toggle-flow
    cargo run -p xtask -- gui visibility-smoke
    cargo run -p xtask -- gui visibility-window-guard

act-ci event="push" runner="-self-hosted": doctor
    #!/usr/bin/env bash
    set -euo pipefail

    if ! command -v act >/dev/null 2>&1; then
      echo "act is required. Install it first: https://github.com/nektos/act" >&2
      exit 1
    fi

    cache_root="${ACT_CACHE_ROOT:-$HOME/.cache/voxy-act}"
    mkdir -p \
      "${cache_root}/actions" \
      "${cache_root}/cache" \
      "${cache_root}/artifacts"

    exec act "{{event}}" \
      -W .github/workflows/ci.yml \
      -P "ubuntu-latest={{runner}}" \
      --reuse \
      --pull=false \
      --action-cache-path "${cache_root}/actions" \
      --cache-server-path "${cache_root}/cache" \
      --artifact-server-path "${cache_root}/artifacts"

act-checks event="push" runner="-self-hosted": doctor
    #!/usr/bin/env bash
    set -euo pipefail

    if ! command -v act >/dev/null 2>&1; then
      echo "act is required. Install it first: https://github.com/nektos/act" >&2
      exit 1
    fi

    cache_root="${ACT_CACHE_ROOT:-$HOME/.cache/voxy-act}"
    mkdir -p \
      "${cache_root}/actions" \
      "${cache_root}/cache" \
      "${cache_root}/artifacts"

    exec act "{{event}}" \
      -j checks \
      -W .github/workflows/ci.yml \
      -P "ubuntu-latest={{runner}}" \
      --reuse \
      --pull=false \
      --action-cache-path "${cache_root}/actions" \
      --cache-server-path "${cache_root}/cache" \
      --artifact-server-path "${cache_root}/artifacts"

act-security event="push" runner="-self-hosted": doctor
    #!/usr/bin/env bash
    set -euo pipefail

    if ! command -v act >/dev/null 2>&1; then
      echo "act is required. Install it first: https://github.com/nektos/act" >&2
      exit 1
    fi

    cache_root="${ACT_CACHE_ROOT:-$HOME/.cache/voxy-act}"
    mkdir -p \
      "${cache_root}/actions" \
      "${cache_root}/cache" \
      "${cache_root}/artifacts"

    exec act "{{event}}" \
      -j security \
      -W .github/workflows/ci.yml \
      -P "ubuntu-latest={{runner}}" \
      --reuse \
      --pull=false \
      --action-cache-path "${cache_root}/actions" \
      --cache-server-path "${cache_root}/cache" \
      --artifact-server-path "${cache_root}/artifacts"

act-cache-clear:
    #!/usr/bin/env bash
    set -euo pipefail
    cache_root="${ACT_CACHE_ROOT:-$HOME/.cache/voxy-act}"
    rm -rf "${cache_root}"
    echo "Removed act cache at ${cache_root}"

validate-packaging:
    ./scripts/release/validate-packaging.sh

validate-packaging-strict:
    VOXY_PACKAGING_VALIDATE_STRICT=1 ./scripts/release/validate-packaging.sh

gui: doctor
    cargo run -p voxy-app

gui-trace every="10": doctor
    VOXY_TRACE_PIPELINE=1 \
    VOXY_TRACE_PIPELINE_EVERY={{every}} \
    cargo run -p voxy-app

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
