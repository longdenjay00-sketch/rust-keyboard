//! `rust-keyboard` — Tactical Rust Keyboard Engine (`aarch64`).
//!
//! Combines three operational surfaces in a single zero-dependency Rust crate:
//! 1. **Core Layout & Modal Engine (`layout`, `engine`, `spec`)**:
//!    - Supports Dvorak, Programmer Dvorak, and QWERTY profiles plus Symbol layer.
//!    - Neovim-style modal engine (`Insert`, `Normal`, `OperatorPending` with `d`/`c`/`y` + motions and counts).
//!    - Golden JSON specification generator (`layout.json`, `vectors.json`).
//! 2. **Terminal TUI & Interactive Simulator (`tui`)**:
//!    - Full ANSI keyboard grid visualizer, live text buffer editor, and key-sequence simulator.
//! 3. **Linux `evdev` / `uinput` Hardware Keymapper (`evdev`)**:
//!    - Raw `/dev/input/event*` listener and `/dev/uinput` virtual keyboard emitter with scancode translation table.
//! 4. **Android IME / JNI FFI Bridge (`jni_bridge`)**:
//!    - C-ABI and JNI (`Java_com_construct_rustkeyboard_RustKeyboardBridge_*`) exports for Android `InputMethodService`.

pub mod engine;
pub mod evdev;
pub mod jni_bridge;
pub mod layout;
pub mod spec;
pub mod tui;

pub use engine::{Action, Buffer, Engine, Mode, PendingOp, Shift};
pub use layout::{grid, profile_grid, shifted, Key, Layer, Profile};
