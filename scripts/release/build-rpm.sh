#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
  echo "Run this command as a normal user (no sudo)."
  exit 1
fi

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "Missing required tool: rpmbuild"
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

package_name="voxy-app"
binary_name="voxy-app"
app_id="com.vince.voxy"
summary="Wayland-native GTK4 app for live transcription"
description="Voxy is a Wayland-native GTK4 Rust app scaffold for live streaming speech-to-text into an editable text area."
release="${VOXY_RPM_RELEASE:-1}"
packager="${VOXY_RPM_PACKAGER:-Voxy Maintainers <noreply@voxy.local>}"

pkgid="$(cargo pkgid -p voxy-app)"
version="${pkgid##*#}"
arch="$(uname -m)"

rpm_topdir="${repo_root}/target/rpm"
stage_dir="${rpm_topdir}/STAGE/${package_name}-${version}"
buildroot="${rpm_topdir}/BUILDROOT/${package_name}-${version}-${release}.${arch}"
spec_path="${rpm_topdir}/SPECS/${package_name}.spec"

echo "Building release binary..."
cargo build --release -p voxy-app

binary_src="${repo_root}/target/release/${binary_name}"
if [[ ! -x "${binary_src}" ]]; then
  echo "Built binary not found at ${binary_src}"
  exit 1
fi

echo "Preparing RPM staging layout..."
rm -rf "${rpm_topdir}"
mkdir -p \
  "${rpm_topdir}/BUILD" \
  "${rpm_topdir}/BUILDROOT" \
  "${rpm_topdir}/RPMS" \
  "${rpm_topdir}/SOURCES" \
  "${rpm_topdir}/SPECS" \
  "${rpm_topdir}/SRPMS" \
  "${stage_dir}/usr/bin" \
  "${stage_dir}/usr/share/applications" \
  "${stage_dir}/usr/share/licenses/${package_name}"

install -m 0755 "${binary_src}" "${stage_dir}/usr/bin/${binary_name}"
install -m 0644 "${repo_root}/LICENSE" "${stage_dir}/usr/share/licenses/${package_name}/LICENSE"

cat > "${stage_dir}/usr/share/applications/${app_id}.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Voxy
Comment=Wayland-native GTK4 app for live transcription
Exec=${binary_name}
Icon=audio-input-microphone
Terminal=false
Categories=AudioVideo;Utility;
StartupNotify=true
EOF

cat > "${spec_path}" <<EOF
%global debug_package %{nil}

Name:           ${package_name}
Version:        ${version}
Release:        ${release}%{?dist}
Summary:        ${summary}
License:        MIT
URL:            https://github.com/veighnsche/voxy
BuildArch:      ${arch}
Packager:       ${packager}

%description
${description}

%prep

%build

%install
mkdir -p %{buildroot}
cp -a "${stage_dir}/." "%{buildroot}/"

%files
%license /usr/share/licenses/${package_name}/LICENSE
/usr/bin/${binary_name}
/usr/share/applications/${app_id}.desktop

%changelog
* $(LC_ALL=C date '+%a %b %d %Y') ${packager} - ${version}-${release}
- Automated local RPM build
EOF

echo "Building RPM package..."
rpmbuild -bb "${spec_path}" \
  --define "_topdir ${rpm_topdir}" \
  --buildroot "${buildroot}" \
  >/dev/null

rpm_path="$(find "${rpm_topdir}/RPMS" -type f -name "${package_name}-${version}-${release}*.${arch}.rpm" | head -n 1)"
if [[ -z "${rpm_path}" ]]; then
  echo "RPM build finished but package file was not found in ${rpm_topdir}/RPMS."
  exit 1
fi

echo "RPM package created: ${rpm_path}"
