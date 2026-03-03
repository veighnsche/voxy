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

    if [[ "{{target}}" == "rpm" && "{{action}}" == "srpm" ]]; then
      exec ./scripts/release/build-srpm.sh
    fi

    echo "Unsupported make target: {{target}} {{action}}" >&2
    echo "Usage: just make rpm package|srpm" >&2
    exit 1

rpm-package:
    ./scripts/release/build-rpm.sh

rpm-srpm ref="HEAD":
    #!/usr/bin/env bash
    set -euo pipefail
    ref_name="{{ref}}"
    ref_name="${ref_name#ref=}"
    exec ./scripts/release/build-srpm.sh "${ref_name}"

copr-build project ref="HEAD":
    #!/usr/bin/env bash
    set -euo pipefail
    project_name="{{project}}"
    project_name="${project_name#project=}"
    ref_name="{{ref}}"
    ref_name="${ref_name#ref=}"
    exec ./scripts/release/copr-build.sh "${project_name}" "${ref_name}"

release-preflight version date="":
    #!/usr/bin/env bash
    set -euo pipefail
    version_value="{{version}}"
    version_value="${version_value#version=}"
    if [[ -z "${version_value}" ]]; then
      echo "Usage: just release-preflight version=<X.Y.Z[-RCN]>" >&2
      exit 1
    fi
    date_value="{{date}}"
    date_value="${date_value#date=}"
    ./scripts/release/verify-version-sync.sh "${version_value}"
    if [[ -n "${date_value}" ]]; then
      ./scripts/release/verify-changelog-entry.sh "${version_value}" "${date_value}"
    else
      ./scripts/release/verify-changelog-entry.sh "${version_value}"
    fi

release-evidence version:
    #!/usr/bin/env bash
    set -euo pipefail
    version_value="{{version}}"
    version_value="${version_value#version=}"
    if [[ -z "${version_value}" ]]; then
      echo "Usage: just release-evidence version=<X.Y.Z[-RCN]>" >&2
      exit 1
    fi
    exec ./scripts/release/generate-evidence.sh "${version_value}"

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

act-ci event="push" runner="ghcr.io/catthehacker/ubuntu:full-latest": doctor
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

    event_name="{{event}}"
    event_name="${event_name#event=}"
    runner_image="{{runner}}"
    runner_image="${runner_image#runner=}"

    exec act "${event_name}" \
      -W .github/workflows/ci.yml \
      -P "ubuntu-latest=${runner_image}" \
      --reuse \
      --pull=false \
      --action-cache-path "${cache_root}/actions" \
      --cache-server-path "${cache_root}/cache" \
      --artifact-server-path "${cache_root}/artifacts"

act-checks event="push" runner="ghcr.io/catthehacker/ubuntu:full-latest": doctor
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

    event_name="{{event}}"
    event_name="${event_name#event=}"
    runner_image="{{runner}}"
    runner_image="${runner_image#runner=}"

    exec act "${event_name}" \
      -j checks \
      -W .github/workflows/ci.yml \
      -P "ubuntu-latest=${runner_image}" \
      --reuse \
      --pull=false \
      --action-cache-path "${cache_root}/actions" \
      --cache-server-path "${cache_root}/cache" \
      --artifact-server-path "${cache_root}/artifacts"

act-security event="push" runner="ghcr.io/catthehacker/ubuntu:full-latest": doctor
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

    event_name="{{event}}"
    event_name="${event_name#event=}"
    runner_image="{{runner}}"
    runner_image="${runner_image#runner=}"

    exec act "${event_name}" \
      -j security \
      -W .github/workflows/ci.yml \
      -P "ubuntu-latest=${runner_image}" \
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
