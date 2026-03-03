#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version|vX.Y.Z> [YYYY-MM-DD]" >&2
  exit 1
fi

version="${1#v}"
expected_date="${2:-}"

changelog_path="${repo_root}/CHANGELOG.md"
if [[ ! -f "${changelog_path}" ]]; then
  echo "Missing changelog: ${changelog_path}" >&2
  exit 1
fi

entry_pattern="^## \\[${version//./\\.}\\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$"
entry_line="$(grep -En "${entry_pattern}" "${changelog_path}" | head -n 1 || true)"

if [[ -z "${entry_line}" ]]; then
  echo "Missing changelog heading for version ${version}." >&2
  echo "Expected a line like: ## [${version}] - YYYY-MM-DD" >&2
  exit 1
fi

entry_text="${entry_line#*:}"
entry_date="$(sed -E 's/^## \[[^]]+\] - ([0-9]{4}-[0-9]{2}-[0-9]{2})$/\1/' <<< "${entry_text}")"
if [[ -z "${entry_date}" ]]; then
  echo "Could not parse changelog date from: ${entry_text}" >&2
  exit 1
fi

if [[ -n "${expected_date}" && "${entry_date}" != "${expected_date}" ]]; then
  echo "Changelog date mismatch for ${version}: expected ${expected_date}, found ${entry_date}" >&2
  exit 1
fi

echo "[release] Changelog entry OK: ${entry_text}"
