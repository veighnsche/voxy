#!/usr/bin/env bash
set -euo pipefail

if pkg-config --exists gtk4-layer-shell-0; then
  echo "gtk4-layer-shell already available: $(pkg-config --modversion gtk4-layer-shell-0)"
  exit 0
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "apt-get not found; this helper currently supports Debian/Ubuntu only." >&2
  exit 1
fi

sudo_cmd=()
if [[ "${EUID}" -ne 0 ]]; then
  sudo_cmd=(sudo)
fi

apt_install() {
  "${sudo_cmd[@]}" apt-get install -y "$@"
}

"${sudo_cmd[@]}" apt-get update

if apt-cache show libgtk4-layer-shell-dev >/dev/null 2>&1; then
  apt_install libgtk4-layer-shell-dev
else
  echo "libgtk4-layer-shell-dev is unavailable in apt; building gtk4-layer-shell from source."
  apt_install \
    ca-certificates \
    git \
    meson \
    ninja-build \
    libwayland-dev \
    wayland-protocols \
    gobject-introspection \
    libgirepository1.0-dev \
    python3

  ref="${GTK4_LAYER_SHELL_REF:-v1.0.4}"
  workdir="$(mktemp -d)"
  trap 'rm -rf "${workdir}"' EXIT

  git clone --depth 1 --branch "${ref}" \
    https://github.com/wmww/gtk4-layer-shell.git \
    "${workdir}/src"

  meson setup "${workdir}/build" "${workdir}/src" \
    --prefix=/usr \
    -Dexamples=false \
    -Ddocs=false \
    -Dtests=false \
    -Dvapi=false \
    -Dintrospection=false

  ninja -C "${workdir}/build"
  "${sudo_cmd[@]}" ninja -C "${workdir}/build" install
  "${sudo_cmd[@]}" ldconfig
fi

echo "gtk4-layer-shell ready: $(pkg-config --modversion gtk4-layer-shell-0)"
