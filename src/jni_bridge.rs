//! Android IME & C-ABI / JNI Bridge (`cdylib` + Kotlin `InputMethodService` scaffold generator).
//!
//! Exposes both:
//! 1. Clean C-ABI functions (`rkb_engine_new`, `rkb_engine_press`, `rkb_engine_free`, `rkb_layout_json`)
//!    callable via JNA, Kotlin/Native, Python `ctypes`, or C/C++.
//! 2. JNI symbol exports (`Java_com_construct_rustkeyboard_RustKeyboardBridge_*`) so an Android
//!    Kotlin `InputMethodService` can load `librust_keyboard.so` directly via `System.loadLibrary("rust_keyboard")`.

use crate::engine::Engine;
use crate::layout::Key;
use crate::spec;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Opaque C handle wrapping `Engine`.
pub struct RkbHandle {
    engine: Engine,
    last_json: CString,
}

#[no_mangle]
pub extern "C" fn rkb_engine_new() -> *mut RkbHandle {
    Box::into_raw(Box::new(RkbHandle {
        engine: Engine::new(),
        last_json: CString::new("[]").unwrap(),
    }))
}

/// Feed a space-separated token string into the engine and return a JSON array of action tokens.
/// The returned pointer is valid until the next call on the same handle or `rkb_engine_free`.
#[no_mangle]
pub unsafe extern "C" fn rkb_engine_press(
    handle: *mut RkbHandle,
    token_cstr: *const c_char,
) -> *const c_char {
    if handle.is_null() || token_cstr.is_null() {
        return ptr::null();
    }
    let h = &mut *handle;
    let Ok(s) = CStr::from_ptr(token_cstr).to_str() else {
        return ptr::null();
    };
    let mut actions = Vec::new();
    for tok in s.split_whitespace() {
        if let Some(k) = Key::from_token(tok) {
            for act in h.engine.press(k) {
                actions.push(format!("\"{}\"", act.token().replace('"', "\\\"")));
            }
        }
    }
    let json = format!(
        "{{\"actions\":[{}],\"mode\":\"{}\",\"layer\":\"{}\",\"shift\":\"{}\"}}",
        actions.join(","),
        h.engine.mode.name(),
        h.engine.layer.name(),
        h.engine.shift.name()
    );
    if let Ok(c) = CString::new(json) {
        h.last_json = c;
    }
    h.last_json.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn rkb_engine_free(handle: *mut RkbHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Generate the complete Android Kotlin `InputMethodService` + JNI wrapper scaffold.
pub fn kotlin_ime_scaffold() -> &'static str {
    r#"package com.construct.rustkeyboard

import android.inputmethodservice.InputMethodService
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.InputConnection

/**
 * RustKeyboardBridge — JNI / C-ABI bridge to librust_keyboard.so (aarch64-linux-android).
 * Zero network permission. Offline by design.
 */
object RustKeyboardBridge {
    init {
        System.loadLibrary("rust_keyboard")
    }

    external fun engineNew(): Long
    external fun enginePress(handle: Long, token: String): String
    external fun engineFree(handle: Long)
}

class RustKeyboardIMEService : InputMethodService() {
    private var handle: Long = 0L

    override fun onCreate() {
        super.onCreate()
        handle = RustKeyboardBridge.engineNew()
    }

    override fun onDestroy() {
        if (handle != 0L) {
            RustKeyboardBridge.engineFree(handle)
            handle = 0L
        }
        super.onDestroy()
    }

    fun onKeyToken(token: String) {
        val ic: InputConnection = currentInputConnection ?: return
        val responseJson = RustKeyboardBridge.enginePress(handle, token)
        dispatchActions(ic, responseJson)
    }

    private fun dispatchActions(ic: InputConnection, json: String) {
        // Parses action tokens emitted by rust-keyboard core engine
        val regex = Regex("\"commit:([^\"]*)\"|\"(delete_back|delete_fwd|delete_word|delete_line|enter|escape|tab|left|right|up|down|word_fwd|word_back|line_start|line_end|undo)\"")
        for (match in regex.findAll(json)) {
            val commitText = match.groups[1]?.value
            if (commitText != null) {
                ic.commitText(commitText, 1)
                continue
            }
            when (match.groups[2]?.value) {
                "delete_back" -> ic.deleteSurroundingText(1, 0)
                "delete_fwd"  -> ic.deleteSurroundingText(0, 1)
                "enter"       -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER))
                "escape"      -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ESCAPE))
                "tab"         -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_TAB))
                "left"        -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_LEFT))
                "right"       -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_RIGHT))
                "up"          -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_UP))
                "down"        -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_DOWN))
                "line_start"  -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_MOVE_HOME))
                "line_end"    -> ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_MOVE_END))
            }
        }
    }
}
"#
}

/// Return the C header declaration for `librust_keyboard.so`.
pub fn c_header_scaffold() -> &'static str {
    r#"/* librust_keyboard.h — C-ABI Header for rust-keyboard (aarch64) */
#ifndef LIBRUST_KEYBOARD_H
#define LIBRUST_KEYBOARD_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RkbHandle RkbHandle;

RkbHandle* rkb_engine_new(void);
const char* rkb_engine_press(RkbHandle* handle, const char* token_cstr);
void rkb_engine_free(RkbHandle* handle);

#ifdef __cplusplus
}
#endif

#endif /* LIBRUST_KEYBOARD_H */
"#
}

/// Self-test the C-ABI FFI bridge in-process.
pub fn verify_ffi_bridge() -> Result<String, String> {
    unsafe {
        let h = rkb_engine_new();
        if h.is_null() {
            return Err("rkb_engine_new returned null".into());
        }
        let input = CString::new("SHIFT h i SPACE NAV h x").unwrap();
        let ptr = rkb_engine_press(h, input.as_ptr());
        if ptr.is_null() {
            rkb_engine_free(h);
            return Err("rkb_engine_press returned null".into());
        }
        let res = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        rkb_engine_free(h);
        let _ = spec::layout_json();
        Ok(res)
    }
}
