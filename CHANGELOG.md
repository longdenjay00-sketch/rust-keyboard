# Changelog — `rust-keyboard`

All notable changes to `rust-keyboard` are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

---

## [0.2.0] - 2026-11-25

### Added
- **Terminal TUI Visualizer & Simulator (`rust-keyboard tui` / `play`)**:
  - Live ANSI keyboard renderer displaying active Profile (`DVORAK`, `PROG-DVORAK`, `QWERTY`), Mode (`INSERT`, `NORMAL`), Layer (`ALPHA`, `SYMBOL`), Shift (`OFF`, `ONCE`, `CAPS LOCK`), and live text buffer with cursor & undo stack.
  - Full Neovim-style Normal mode motions (`h`/`j`/`k`/`l`, `w`/`e`/`b`, `0`/`^`/`$`), edits (`x`, `X`, `u`, `i`/`a`/`I`/`A`/`o`/`O`), and operator-pending commands (`dd`, `dw`, `cw`, `cc`).
- **Linux `evdev` / `uinput` Hardware Keymapper (`rust-keyboard evdev`)**:
  - Physical Linux scancode translation table (`--table`), PRoot-compatible event sequence simulator (`--sim KEY_...`), and hardware `/dev/input/eventX` listener (`--device`).
- **Android IME JNI & C-ABI FFI Bridge (`rust-keyboard jni` & `librust_keyboard.so`)**:
  - Dual `rlib` + `cdylib` compilation target exporting `rkb_engine_new`, `rkb_engine_press`, and `rkb_engine_free`.
  - In-process FFI verification (`--verify`) and code generators for `RustKeyboardIMEService.kt` and `librust_keyboard.h` (`--emit-dir`).
- **Golden JSON Specifications (`rust-keyboard spec`)**:
  - Deterministic `spec/layout.json` and `spec/vectors.json` export and integration test suite.
- **Debian Packaging & Release Automation (`build-deb.sh` & `bump-version.sh`)**:
  - Automated `arm64` `.deb` package builder (`dist/rust-keyboard_<version>_arm64.deb`) and version bump/tag helper.

---

## [0.1.0] - 2026-11-24

### Added
- Initial Dvorak modal key engine prototype (`Layout`, `Key`, `Mode`, `Layer`, `ShiftState`, and basic token playback).
