#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <path-to-rpm>" >&2
  exit 1
fi

rpm_path="$1"
if [[ ! -f "${rpm_path}" ]]; then
  echo "RPM file not found: ${rpm_path}" >&2
  exit 1
fi

if ! command -v rpmsign >/dev/null 2>&1; then
  echo "Missing required tool: rpmsign" >&2
  echo "Install on Fedora: sudo dnf install -y rpm-sign" >&2
  exit 1
fi

if ! command -v gpg >/dev/null 2>&1; then
  echo "Missing required tool: gpg" >&2
  exit 1
fi

gpg_key="${VOXY_RPM_GPG_KEY:-}"
if [[ -n "${gpg_key}" ]]; then
  rpmsign --addsign \
    --define "_signature gpg" \
    --define "_gpg_name ${gpg_key}" \
    --define "__gpg /usr/bin/gpg" \
    "${rpm_path}"
else
  rpmsign --addsign "${rpm_path}"
fi

echo "Signature check:"
rpm -Kv "${rpm_path}"
