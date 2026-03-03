#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

if [[ "${VOXY_SKIP_PRECOMMIT:-0}" == "1" ]]; then
  echo "[pre-commit] Skipping checks (VOXY_SKIP_PRECOMMIT=1)."
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "[pre-commit] Missing required tool: cargo" >&2
  exit 1
fi

force_run="${VOXY_PRECOMMIT_FORCE:-0}"
staged_files="$(git diff --cached --name-only --diff-filter=ACMR)"
if [[ -z "${staged_files}" && "${force_run}" != "1" ]]; then
  echo "[pre-commit] No staged files. Skipping."
  exit 0
fi

has_code_changes=0
if grep -Eq '(^|/)(Cargo\.toml|Cargo\.lock|rust-toolchain\.toml)$|^justfile$|\.rs$|^tests/|^voxy-|^xtask/' <<< "${staged_files}"; then
  has_code_changes=1
fi

if [[ "${force_run}" == "1" ]]; then
  has_code_changes=1
fi

if [[ "${has_code_changes}" != "1" ]]; then
  echo "[pre-commit] No Rust/workspace changes staged. Skipping heavy checks."
  exit 0
fi

run() {
  echo "[pre-commit] $*"
  "$@"
}

run cargo fmt --all -- --check
run cargo check --workspace --all-targets
run cargo clippy --workspace --all-targets -- -D warnings

if [[ "${VOXY_PRECOMMIT_SKIP_TESTS:-0}" != "1" ]]; then
  run cargo test --workspace --all-targets
else
  echo "[pre-commit] Skipping tests (VOXY_PRECOMMIT_SKIP_TESTS=1)."
fi

if grep -Eq '^Cargo\.toml$|^packaging/rpm/voxy-app\.spec$' <<< "${staged_files}"; then
  run ./scripts/release/verify-version-sync.sh
fi

if grep -Eq '^packaging/linux/|^assets/icons/|^scripts/release/validate-packaging\.sh$' <<< "${staged_files}"; then
  run ./scripts/release/validate-packaging.sh
fi

echo "[pre-commit] All checks passed."
