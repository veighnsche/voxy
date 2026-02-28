#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
  echo "Run this script as a normal user; it will use sudo when needed."
  exit 1
fi

if [[ -f /etc/os-release ]]; then
  # shellcheck disable=SC1091
  source /etc/os-release
else
  echo "/etc/os-release not found; cannot detect distro"
  exit 1
fi

id="${ID:-}"
like="${ID_LIKE:-}"

if [[ "$id" =~ (ubuntu|debian|linuxmint|pop) || "$like" =~ debian ]]; then
  sudo apt-get update
  sudo apt-get install -y \
    pkg-config \
    libgtk-4-dev \
    libgraphene-1.0-dev \
    libgtk4-layer-shell-dev \
    libasound2-dev
  exit 0
fi

if [[ "$id" =~ (fedora|ultramarine|rhel|centos|rocky|almalinux) || "$like" =~ fedora ]]; then
  sudo dnf install -y \
    pkgconf-pkg-config \
    gtk4-devel \
    graphene-devel \
    gtk4-layer-shell-devel \
    alsa-lib-devel
  exit 0
fi

if [[ "$id" =~ (arch|manjaro|endeavouros) || "$like" =~ arch ]]; then
  sudo pacman -S --needed \
    pkgconf \
    gtk4 \
    graphene \
    gtk4-layer-shell \
    alsa-lib
  exit 0
fi

if [[ "$id" =~ (opensuse|sles|sled) || "$like" =~ suse ]]; then
  sudo zypper install -y \
    pkgconf-pkg-config \
    gtk4-devel \
    graphene-devel \
    gtk4-layer-shell-devel \
    alsa-devel
  exit 0
fi

echo "Unsupported distribution: ID='${id}' ID_LIKE='${like}'"
echo "Please install packages that provide: pkg-config, gtk4.pc, graphene-gobject-1.0.pc, gtk4-layer-shell-0.pc, alsa.pc"
exit 1
