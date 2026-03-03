#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
desktop_file="${repo_root}/packaging/linux/com.vince.voxy.desktop"
metainfo_file="${repo_root}/packaging/linux/com.vince.voxy.metainfo.xml"
icon_file="${repo_root}/assets/icons/hicolor/scalable/apps/com.vince.voxy.svg"
strict="${VOXY_PACKAGING_VALIDATE_STRICT:-0}"

if [[ ! -f "${desktop_file}" ]]; then
  echo "Missing desktop entry: ${desktop_file}" >&2
  exit 1
fi

if [[ ! -f "${metainfo_file}" ]]; then
  echo "Missing metainfo file: ${metainfo_file}" >&2
  exit 1
fi

if [[ ! -f "${icon_file}" ]]; then
  echo "Missing app icon file: ${icon_file}" >&2
  exit 1
fi

run_or_skip() {
  local tool="$1"
  shift
  if command -v "${tool}" >/dev/null 2>&1; then
    "$@"
    return
  fi

  if [[ "${strict}" == "1" ]]; then
    echo "Missing required packaging validation tool: ${tool}" >&2
    exit 1
  fi

  echo "[packaging] Skipping ${tool} validation; tool not installed."
}

run_or_skip desktop-file-validate desktop-file-validate "${desktop_file}"
run_or_skip appstreamcli appstreamcli validate --no-net "${metainfo_file}"

stage_dir="$(mktemp -d)"
trap 'rm -rf "${stage_dir}"' EXIT
mkdir -p \
  "${stage_dir}/usr/bin" \
  "${stage_dir}/usr/share/applications" \
  "${stage_dir}/usr/share/icons/hicolor/scalable/apps" \
  "${stage_dir}/usr/share/metainfo"

cat > "${stage_dir}/usr/bin/voxy-app" <<'SMOKE_BIN'
#!/usr/bin/env bash
exit 0
SMOKE_BIN
chmod +x "${stage_dir}/usr/bin/voxy-app"
install -m 0644 "${desktop_file}" "${stage_dir}/usr/share/applications/com.vince.voxy.desktop"
install -m 0644 "${metainfo_file}" "${stage_dir}/usr/share/metainfo/com.vince.voxy.metainfo.xml"
install -m 0644 "${icon_file}" "${stage_dir}/usr/share/icons/hicolor/scalable/apps/com.vince.voxy.svg"

test -x "${stage_dir}/usr/bin/voxy-app"
test -f "${stage_dir}/usr/share/applications/com.vince.voxy.desktop"
test -f "${stage_dir}/usr/share/metainfo/com.vince.voxy.metainfo.xml"
test -f "${stage_dir}/usr/share/icons/hicolor/scalable/apps/com.vince.voxy.svg"

echo "[packaging] Packaging metadata + staged install smoke passed."
