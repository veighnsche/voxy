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
set +e
verify_output="$(rpm -Kv "${rpm_path}" 2>&1)"
verify_rc=$?
set -e
printf '%s\n' "${verify_output}"

if [[ ${verify_rc} -eq 0 ]]; then
  exit 0
fi

if grep -q "Header OpenPGP" <<< "${verify_output}" && grep -q "NOKEY" <<< "${verify_output}"; then
  echo ""
  echo "RPM is signed, but the public key is not imported into the RPM keyring (NOKEY)."
  if [[ -n "${gpg_key}" ]]; then
    echo "Import it with:"
    echo "  gpg --armor --export ${gpg_key} | sudo rpm --import -"
  else
    echo "Import the signing public key with:"
    echo "  gpg --armor --export <KEYID> | sudo rpm --import -"
  fi
  exit 0
fi

echo "RPM signature verification failed." >&2
exit "${verify_rc}"
