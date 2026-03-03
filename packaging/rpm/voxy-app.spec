%global app_id com.vince.voxy

Name:           voxy-app
Version:        %{!?pkg_version:1.0.0~RC2}%{?pkg_version}
Release:        %{!?pkg_release:1}%{?pkg_release}%{?dist}
Summary:        Wayland-native GTK4 app for live transcription
License:        MIT
URL:            https://github.com/veighnsche/voxy
Source0:        %{name}-%{?voxy_upstream_version}%{!?voxy_upstream_version:%{version}}.tar.gz

BuildRequires:  appstream
BuildRequires:  cargo
BuildRequires:  desktop-file-utils
BuildRequires:  gcc-c++
BuildRequires:  pkgconfig(alsa)
BuildRequires:  pkgconfig(graphene-gobject-1.0)
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(gtk4-layer-shell-0)

%description
Voxy is a Wayland-native GTK4 Rust app for live streaming speech-to-text
into an editable text area.

%prep
%autosetup -n %{name}-%{?voxy_upstream_version}%{!?voxy_upstream_version:%{version}}

%build
cargo build --release --locked -p voxy-app

%install
install -Dpm0755 target/release/voxy-app %{buildroot}%{_bindir}/voxy-app
install -Dpm0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE
install -Dpm0644 packaging/linux/%{app_id}.desktop %{buildroot}%{_datadir}/applications/%{app_id}.desktop
install -Dpm0644 packaging/linux/%{app_id}.metainfo.xml %{buildroot}%{_datadir}/metainfo/%{app_id}.metainfo.xml
install -Dpm0644 assets/icons/hicolor/scalable/apps/%{app_id}.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg

%check
VOXY_PACKAGING_VALIDATE_STRICT=1 ./scripts/release/validate-packaging.sh

%files
%license %{_licensedir}/%{name}/LICENSE
%{_bindir}/voxy-app
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/metainfo/%{app_id}.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg

%changelog
* Tue Mar 03 2026 Vince Liem <vincepaul.liem@gmail.com> - 1.0.0~RC2-1
- Fix stop-path benign empty-buffer commit errors during silence-timeout flows.
- Split realtime STT client into focused modules for maintainability.
- Add release metadata/evidence automation and release preflight checks.

* Tue Mar 03 2026 Vince Liem <vincepaul.liem@gmail.com> - 1.0.0~RC1-1
- Initial COPR spec from source tarball/SRPM flow.
