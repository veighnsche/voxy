#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

normalize_version() {
  local raw="$1"
  printf '%s' "${raw#v}"
}

to_rpm_version() {
  local version="$1"
  if [[ "${version}" != *-* ]]; then
    printf '%s' "${version}"
    return
  fi

  local base="${version%%-*}"
  local suffix="${version#*-}"
  suffix="${suffix//-/.}"
  printf '%s~%s' "${base}" "${suffix}"
}

expected_version_raw="${1:-}"
expected_version=""
if [[ -n "${expected_version_raw}" ]]; then
  expected_version="$(normalize_version "${expected_version_raw}")"
fi

workspace_version="$(
  awk '
    /^\[workspace\.package\]/ { in_workspace_package = 1; next }
    in_workspace_package && /^\[/ { in_workspace_package = 0 }
    in_workspace_package && $1 == "version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
)"

if [[ -z "${workspace_version}" ]]; then
  echo "Could not resolve workspace version from Cargo.toml" >&2
  exit 1
fi

if [[ -n "${expected_version}" && "${workspace_version}" != "${expected_version}" ]]; then
  echo "Workspace version mismatch: expected ${expected_version}, found ${workspace_version}" >&2
  exit 1
fi

dep_lines="$(
  awk '
    /^\[workspace\.dependencies\]/ { in_workspace_dependencies = 1; next }
    in_workspace_dependencies && /^\[/ { in_workspace_dependencies = 0 }
    in_workspace_dependencies && $1 ~ /^voxy-(audio|core|stt)$/ {
      dep = $1
      match($0, /version *= *"[^"]+"/)
      if (RSTART > 0) {
        value = substr($0, RSTART, RLENGTH)
        gsub(/version *= *"/, "", value)
        gsub(/"/, "", value)
        print dep "=" value
      }
    }
  ' Cargo.toml
)"

while IFS= read -r dep_line; do
  [[ -z "${dep_line}" ]] && continue
  dep_name="${dep_line%%=*}"
  dep_version="${dep_line#*=}"
  if [[ "${dep_version}" != "${workspace_version}" ]]; then
    echo "Workspace dependency version mismatch for ${dep_name}: expected ${workspace_version}, found ${dep_version}" >&2
    exit 1
  fi
done <<< "${dep_lines}"

spec_path="${repo_root}/packaging/rpm/voxy-app.spec"
if [[ ! -f "${spec_path}" ]]; then
  echo "Missing RPM spec file: ${spec_path}" >&2
  exit 1
fi

spec_version_raw="$(awk '/^Version:[[:space:]]+/ { print $2; exit }' "${spec_path}")"
if [[ -z "${spec_version_raw}" ]]; then
  echo "Could not resolve Version from ${spec_path}" >&2
  exit 1
fi

spec_version="${spec_version_raw}"
if [[ "${spec_version_raw}" =~ ^%\{!\?pkg_version:([^}]*)\}%\{\?pkg_version\}$ ]]; then
  spec_version="${BASH_REMATCH[1]}"
fi

expected_spec_version="$(to_rpm_version "${workspace_version}")"
if [[ "${spec_version}" != "${expected_spec_version}" ]]; then
  echo "RPM spec Version mismatch: expected ${expected_spec_version}, found ${spec_version_raw}" >&2
  exit 1
fi

echo "[release] Version sync OK:"
echo "  workspace version: ${workspace_version}"
echo "  rpm spec version: ${spec_version}"
