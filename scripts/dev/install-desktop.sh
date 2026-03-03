#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
  echo "Run this command as a normal user (no sudo)."
  exit 1
fi

app_name="Voxy"
app_id="com.vince.voxy"
binary_name="voxy-app"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

bin_dir="${HOME}/.local/bin"
apps_dir="${HOME}/.local/share/applications"
icons_dir="${HOME}/.local/share/icons/hicolor/scalable/apps"
metainfo_dir="${HOME}/.local/share/metainfo"
installed_bin="${bin_dir}/${binary_name}"
desktop_file="${apps_dir}/${app_id}.desktop"
icon_file="${icons_dir}/${app_id}.svg"
metainfo_file="${metainfo_dir}/${app_id}.metainfo.xml"
desktop_template="${repo_root}/packaging/linux/${app_id}.desktop"
metainfo_src="${repo_root}/packaging/linux/${app_id}.metainfo.xml"

echo "Building release binary..."
cargo build --release -p voxy-app

if [[ ! -x "target/release/${binary_name}" ]]; then
  echo "Built binary not found at target/release/${binary_name}"
  exit 1
fi

mkdir -p "${bin_dir}" "${apps_dir}" "${icons_dir}" "${metainfo_dir}"
install -m 0755 "target/release/${binary_name}" "${installed_bin}"
install -m 0644 "${repo_root}/assets/icons/hicolor/scalable/apps/${app_id}.svg" "${icon_file}"
install -m 0644 "${metainfo_src}" "${metainfo_file}"

if [[ ! -f "${desktop_template}" ]]; then
  echo "Desktop entry template not found at ${desktop_template}"
  exit 1
fi

escaped_exec_for_sed="$(printf '%s' "${installed_bin}" | sed 's/[&|]/\\&/g')"
sed "s|^Exec=.*|Exec=${escaped_exec_for_sed}|" "${desktop_template}" > "${desktop_file}"

desktop_dir=""
if command -v xdg-user-dir >/dev/null 2>&1; then
  desktop_dir="$(xdg-user-dir DESKTOP 2>/dev/null || true)"
fi

if [[ -z "${desktop_dir}" || "${desktop_dir}" == "${HOME}" ]]; then
  desktop_dir="${HOME}/Desktop"
fi

mkdir -p "${desktop_dir}"
desktop_shortcut="${desktop_dir}/${app_name}.desktop"
cp "${desktop_file}" "${desktop_shortcut}"
chmod +x "${desktop_shortcut}"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${apps_dir}" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed binary: ${installed_bin}"
echo "Installed launcher: ${desktop_file}"
echo "Installed icon: ${icon_file}"
echo "Installed metainfo: ${metainfo_file}"
echo "Desktop shortcut: ${desktop_shortcut}"
