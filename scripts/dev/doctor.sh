#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

missing=0

print_ok() {
  echo -e "${GREEN}ok${NC}: $1"
}

print_warn() {
  echo -e "${YELLOW}warn${NC}: $1"
}

print_err() {
  echo -e "${RED}err${NC}: $1"
}

recommend_install_cmd() {
  local id=""
  local like=""

  if [[ -f /etc/os-release ]]; then
    # shellcheck disable=SC1091
    source /etc/os-release
    id="${ID:-}"
    like="${ID_LIKE:-}"
  fi

  if [[ "$id" =~ (ubuntu|debian|linuxmint|pop) || "$like" =~ debian ]]; then
    echo "sudo apt-get update && sudo apt-get install -y pkg-config libgtk-4-dev libgraphene-1.0-dev libgtk4-layer-shell-dev"
    return
  fi

  if [[ "$id" =~ (fedora|ultramarine|rhel|centos|rocky|almalinux) || "$like" =~ fedora ]]; then
    echo "sudo dnf install -y pkgconf-pkg-config gtk4-devel graphene-devel gtk4-layer-shell-devel"
    return
  fi

  if [[ "$id" =~ (arch|manjaro|endeavouros) || "$like" =~ arch ]]; then
    echo "sudo pacman -S --needed pkgconf gtk4 graphene gtk4-layer-shell"
    return
  fi

  if [[ "$id" =~ (opensuse|sles|sled) || "$like" =~ suse ]]; then
    echo "sudo zypper install -y pkgconf-pkg-config gtk4-devel graphene-devel gtk4-layer-shell-devel"
    return
  fi

  echo "Install packages that provide: pkg-config, gtk4.pc, graphene-gobject-1.0.pc, gtk4-layer-shell-0.pc"
}

recommend_audio_player_cmd() {
  local id=""
  local like=""

  if [[ -f /etc/os-release ]]; then
    # shellcheck disable=SC1091
    source /etc/os-release
    id="${ID:-}"
    like="${ID_LIKE:-}"
  fi

  if [[ "$id" =~ (ubuntu|debian|linuxmint|pop) || "$like" =~ debian ]]; then
    echo "sudo apt-get update && sudo apt-get install -y mpv ffmpeg vlc gstreamer1.0-tools"
    return
  fi

  if [[ "$id" =~ (fedora|ultramarine|rhel|centos|rocky|almalinux) || "$like" =~ fedora ]]; then
    echo "sudo dnf install -y mpv ffmpeg vlc gstreamer1-plugins-base-tools"
    return
  fi

  if [[ "$id" =~ (arch|manjaro|endeavouros) || "$like" =~ arch ]]; then
    echo "sudo pacman -S --needed mpv ffmpeg vlc gst-plugins-base"
    return
  fi

  if [[ "$id" =~ (opensuse|sles|sled) || "$like" =~ suse ]]; then
    echo "sudo zypper install -y mpv ffmpeg vlc gstreamer-plugins-base-tools"
    return
  fi

  echo "Install one of: mpv, ffplay (ffmpeg), vlc, gst-play-1.0"
}

check_command() {
  local cmd="$1"
  if command -v "$cmd" >/dev/null 2>&1; then
    print_ok "found command '$cmd'"
  else
    print_err "missing command '$cmd'"
    missing=1
  fi
}

check_pkg() {
  local module="$1"
  if pkg-config --exists "$module"; then
    local ver
    ver="$(pkg-config --modversion "$module")"
    print_ok "pkg-config module '$module' ($ver)"
  else
    print_err "missing pkg-config module '$module'"
    missing=1
  fi
}

check_audio_fixture() {
  local fixture_path="tests/fixtures/audio/test_3.mp3"
  if [[ -f "$fixture_path" ]]; then
    print_ok "audio fixture available: $fixture_path"
  else
    print_warn "audio fixture missing: $fixture_path"
    echo "      expected for fixture-input flow: record -> play(test_3)"
  fi
}

check_audio_preview_player() {
  local candidates=("mpv" "ffplay" "vlc" "gst-play-1.0")
  local found=()

  for candidate in "${candidates[@]}"; do
    if command -v "$candidate" >/dev/null 2>&1; then
      found+=("$candidate")
    fi
  done

  if [[ "${#found[@]}" -gt 0 ]]; then
    print_ok "audio playback tool available: ${found[*]}"
  else
    print_warn "no playback tool found (play button will not produce speaker output)"
    echo "      install one with: $(recommend_audio_player_cmd)"
  fi
}

echo "== Voxy Dev Environment Doctor =="

check_command cargo
check_command pkg-config

if command -v pkg-config >/dev/null 2>&1; then
  check_pkg gtk4
  check_pkg graphene-gobject-1.0
  check_pkg gtk4-layer-shell-0
fi

if command -v watchexec >/dev/null 2>&1; then
  print_ok "watcher available: watchexec"
elif command -v cargo-watch >/dev/null 2>&1; then
  print_ok "watcher available: cargo-watch"
else
  print_warn "no file watcher installed (optional for 'just dev')"
  echo "      install one with: cargo install watchexec-cli"
fi

check_audio_fixture
check_audio_preview_player

if [[ "$missing" -ne 0 ]]; then
  echo
  print_err "environment is not ready"
  echo "Suggested install command:"
  echo "  $(recommend_install_cmd)"
  exit 1
fi

echo
print_ok "environment looks ready"
