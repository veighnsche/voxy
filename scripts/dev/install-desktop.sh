#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
  echo "Run this command as a normal user (no sudo)."
  exit 1
fi

app_name="Voxy"
app_id="com.vince.voxy"
binary_name="voxy-app"

bin_dir="${HOME}/.local/bin"
apps_dir="${HOME}/.local/share/applications"
installed_bin="${bin_dir}/${binary_name}"
desktop_file="${apps_dir}/${app_id}.desktop"

echo "Building release binary..."
cargo build --release -p voxy-app

if [[ ! -x "target/release/${binary_name}" ]]; then
  echo "Built binary not found at target/release/${binary_name}"
  exit 1
fi

mkdir -p "${bin_dir}" "${apps_dir}"
install -m 0755 "target/release/${binary_name}" "${installed_bin}"

escaped_exec="${installed_bin// /\\ }"

cat > "${desktop_file}" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=${app_name}
Comment=Wayland-native GTK4 app for live transcription
Exec=${escaped_exec}
Icon=audio-input-microphone
Terminal=false
Categories=AudioVideo;Utility;
StartupNotify=true
EOF

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

echo "Installed binary: ${installed_bin}"
echo "Installed launcher: ${desktop_file}"
echo "Desktop shortcut: ${desktop_shortcut}"
