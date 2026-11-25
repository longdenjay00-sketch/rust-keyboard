package com.construct.rustkeyboard

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
