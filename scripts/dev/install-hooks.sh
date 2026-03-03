#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

if ! command -v git >/dev/null 2>&1; then
  echo "Missing required tool: git" >&2
  exit 1
fi

hooks_dir="${repo_root}/.githooks"
if [[ ! -d "${hooks_dir}" ]]; then
  echo "Missing hooks directory: ${hooks_dir}" >&2
  exit 1
fi

chmod +x "${hooks_dir}/pre-commit" "${repo_root}/scripts/dev/pre-commit-checks.sh"

git config core.hooksPath .githooks
echo "Configured git hooks path:"
echo "  core.hooksPath=$(git config --get core.hooksPath)"
echo "Pre-commit hook is now active."
