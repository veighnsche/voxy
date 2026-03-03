#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <owner/project> [git-ref]" >&2
  echo "Example: $0 veighnsche/voxy HEAD" >&2
  exit 1
fi

project="$1"
ref="${2:-HEAD}"

if ! command -v copr-cli >/dev/null 2>&1; then
  echo "Missing required tool: copr-cli"
  echo "Install on Fedora: sudo dnf install -y copr-cli"
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

"${repo_root}/scripts/release/build-srpm.sh" "${ref}"

srpm_path="$(find "${repo_root}/target/rpm-srpm/SRPMS" -type f -name "*.src.rpm" | head -n 1)"
if [[ -z "${srpm_path}" ]]; then
  echo "No SRPM found in target/rpm-srpm/SRPMS after build."
  exit 1
fi

echo "Submitting SRPM to COPR project '${project}'..."
if [[ "${VOXY_COPR_NOWAIT:-0}" == "1" ]]; then
  copr-cli build --nowait "${project}" "${srpm_path}"
else
  copr-cli build "${project}" "${srpm_path}"
fi
