#!/usr/bin/env bash
set -euo pipefail

# Install base packages
sudo apt-get update
sudo apt-get install -y curl git python3 python3-pip golang rustc cargo apt-transport-https gnupg wget

# Install Dart
if ! command -v dart >/dev/null 2>&1; then
  wget -qO- https://dl-ssl.google.com/linux/linux_signing_key.pub | sudo gpg --dearmor -o /usr/share/keyrings/dart.gpg
  echo "deb [signed-by=/usr/share/keyrings/dart.gpg] https://storage.googleapis.com/download.dartlang.org/linux/debian stable main" | sudo tee /etc/apt/sources.list.d/dart_stable.list
  sudo apt-get update
  sudo apt-get install -y dart
fi

# Install Flutter without cloning the full repository
if ! command -v flutter >/dev/null 2>&1; then
  FLUTTER_VERSION="3.35.2"
  TMP_DIR="$(mktemp -d)"
  wget -qO "$TMP_DIR/flutter.tar.xz" "https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_${FLUTTER_VERSION}-stable.tar.xz"
  sudo mkdir -p /opt
  sudo tar -xf "$TMP_DIR/flutter.tar.xz" -C /opt
  sudo rm -rf "$TMP_DIR"
  echo 'export PATH=/opt/flutter/bin:$PATH' | sudo tee /etc/profile.d/flutter.sh >/dev/null
fi

