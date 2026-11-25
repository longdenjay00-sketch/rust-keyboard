#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"
PKG_NAME="rust-keyboard"
PKG_VER="$(awk -F'"' '/^version *=/ {print $2; exit}' Cargo.toml)"
ARCH="$(dpkg --print-architecture 2>/dev/null || echo arm64)"
STAGE_DIR="dist/${PKG_NAME}_${PKG_VER}_${ARCH}"

rm -rf "$STAGE_DIR"
mkdir -p \
  "$STAGE_DIR/DEBIAN" \
  "$STAGE_DIR/usr/bin" \
  "$STAGE_DIR/usr/lib" \
  "$STAGE_DIR/usr/include" \
  "$STAGE_DIR/usr/share/rust-keyboard/spec" \
  "$STAGE_DIR/usr/share/rust-keyboard/android" \
  "$STAGE_DIR/usr/share/doc/rust-keyboard"

# Generate spec and Android JNI files
./target/release/rust-keyboard spec --write spec
./target/release/rust-keyboard jni --emit-dir android

# Copy binaries, shared library, header, specs, and docs
install -m 0755 target/release/rust-keyboard "$STAGE_DIR/usr/bin/rust-keyboard"
install -m 0644 target/release/librust_keyboard.so "$STAGE_DIR/usr/lib/librust_keyboard.so"
install -m 0644 android/librust_keyboard.h "$STAGE_DIR/usr/include/librust_keyboard.h"
install -m 0644 android/RustKeyboardIMEService.kt "$STAGE_DIR/usr/share/rust-keyboard/android/RustKeyboardIMEService.kt"
install -m 0644 spec/layout.json "$STAGE_DIR/usr/share/rust-keyboard/spec/layout.json"
install -m 0644 spec/vectors.json "$STAGE_DIR/usr/share/rust-keyboard/spec/vectors.json"
install -m 0644 README.md "$STAGE_DIR/usr/share/doc/rust-keyboard/README.md"

cat <<EOF > "$STAGE_DIR/DEBIAN/control"
Package: ${PKG_NAME}
Version: ${PKG_VER}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: Alex (Epic) <longdenjay00@gmail.com>
Description: Tactical Rust Keyboard Engine (TUI + Linux evdev + Android JNI)
 Offline, zero-dependency modal keyboard engine supporting Dvorak, Programmer
 Dvorak, and QWERTY layouts with Neovim Normal mode, terminal ANSI TUI
 simulator, Linux evdev/uinput scancode mapper, and Android IME JNI bridge.
EOF

chmod 0755 "$STAGE_DIR/DEBIAN"
chmod 0644 "$STAGE_DIR/DEBIAN/control"

DEB_OUT="dist/${PKG_NAME}_${PKG_VER}_${ARCH}.deb"
dpkg-deb --build --root-owner-group "$STAGE_DIR" "$DEB_OUT"
echo "[✓] Built package: $DEB_OUT"
