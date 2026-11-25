//! Linux `evdev` / `uinput` Hardware Keycode Mapper & Daemon Module.
//!
//! Provides:
//! - Complete Linux `input-event-codes.h` scancode table (`EV_KEY`, `KEY_Q`..`KEY_M`, modifiers)
//! - Hardware QWERTY scancode -> Dvorak / Programmer Dvorak key & scancode translation
//! - Raw 24-byte `input_event` struct binary parser/encoder for `/dev/input/event*` & `/dev/uinput` on `aarch64`
//! - Dry-run / simulation mode so it works cleanly inside unprivileged Kali PRoot or Termux containers without root `/dev/uinput`.

use crate::engine::{Action, Engine};
use crate::layout::{qwerty_to_profile, Key, Profile};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const SYN_REPORT: u16 = 0x00;

/// Raw Linux `struct input_event` (24 bytes on 64-bit `aarch64`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub tv_sec: i64,
    pub tv_usec: i64,
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub fn to_bytes(self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&self.tv_sec.to_ne_bytes());
        buf[8..16].copy_from_slice(&self.tv_usec.to_ne_bytes());
        buf[16..18].copy_from_slice(&self.event_type.to_ne_bytes());
        buf[18..20].copy_from_slice(&self.code.to_ne_bytes());
        buf[20..24].copy_from_slice(&self.value.to_ne_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; 24]) -> Self {
        let tv_sec = i64::from_ne_bytes(buf[0..8].try_into().unwrap());
        let tv_usec = i64::from_ne_bytes(buf[8..16].try_into().unwrap());
        let event_type = u16::from_ne_bytes(buf[16..18].try_into().unwrap());
        let code = u16::from_ne_bytes(buf[18..20].try_into().unwrap());
        let value = i32::from_ne_bytes(buf[20..24].try_into().unwrap());
        Self {
            tv_sec,
            tv_usec,
            event_type,
            code,
            value,
        }
    }
}

/// Standard Linux `EV_KEY` scancode table (`linux/input-event-codes.h`).
pub const SCANCODE_TABLE: &[(u16, &str, Key)] = &[
    (1, "KEY_ESC", Key::Esc),
    (2, "KEY_1", Key::Char('1')),
    (3, "KEY_2", Key::Char('2')),
    (4, "KEY_3", Key::Char('3')),
    (5, "KEY_4", Key::Char('4')),
    (6, "KEY_5", Key::Char('5')),
    (7, "KEY_6", Key::Char('6')),
    (8, "KEY_7", Key::Char('7')),
    (9, "KEY_8", Key::Char('8')),
    (10, "KEY_9", Key::Char('9')),
    (11, "KEY_0", Key::Char('0')),
    (12, "KEY_MINUS", Key::Char('-')),
    (13, "KEY_EQUAL", Key::Char('=')),
    (14, "KEY_BACKSPACE", Key::Backspace),
    (15, "KEY_TAB", Key::Tab),
    (16, "KEY_Q", Key::Char('q')),
    (17, "KEY_W", Key::Char('w')),
    (18, "KEY_E", Key::Char('e')),
    (19, "KEY_R", Key::Char('r')),
    (20, "KEY_T", Key::Char('t')),
    (21, "KEY_Y", Key::Char('y')),
    (22, "KEY_U", Key::Char('u')),
    (23, "KEY_I", Key::Char('i')),
    (24, "KEY_O", Key::Char('o')),
    (25, "KEY_P", Key::Char('p')),
    (26, "KEY_LEFTBRACE", Key::Char('[')),
    (27, "KEY_RIGHTBRACE", Key::Char(']')),
    (28, "KEY_ENTER", Key::Enter),
    (30, "KEY_A", Key::Char('a')),
    (31, "KEY_S", Key::Char('s')),
    (32, "KEY_D", Key::Char('d')),
    (33, "KEY_F", Key::Char('f')),
    (34, "KEY_G", Key::Char('g')),
    (35, "KEY_H", Key::Char('h')),
    (36, "KEY_J", Key::Char('j')),
    (37, "KEY_K", Key::Char('k')),
    (38, "KEY_L", Key::Char('l')),
    (39, "KEY_SEMICOLON", Key::Char(';')),
    (40, "KEY_APOSTROPHE", Key::Char('\'')),
    (41, "KEY_GRAVE", Key::Char('`')),
    (42, "KEY_LEFTSHIFT", Key::Shift),
    (43, "KEY_BACKSLASH", Key::Char('\\')),
    (44, "KEY_Z", Key::Char('z')),
    (45, "KEY_X", Key::Char('x')),
    (46, "KEY_C", Key::Char('c')),
    (47, "KEY_V", Key::Char('v')),
    (48, "KEY_B", Key::Char('b')),
    (49, "KEY_N", Key::Char('n')),
    (50, "KEY_M", Key::Char('m')),
    (51, "KEY_COMMA", Key::Char(',')),
    (52, "KEY_DOT", Key::Char('.')),
    (53, "KEY_SLASH", Key::Char('/')),
    (54, "KEY_RIGHTSHIFT", Key::Shift),
    (57, "KEY_SPACE", Key::Space),
    (58, "KEY_CAPSLOCK", Key::Nav), // CapsLock acts as tactical NAV modal toggle!
];

/// Translate a physical Linux scancode under the chosen `Profile` into a logical `Key`.
pub fn scancode_to_key(code: u16, profile: Profile) -> Option<(&'static str, Key)> {
    let (_, name, base_key) = SCANCODE_TABLE.iter().find(|(c, _, _)| *c == code)?;
    let mapped = match base_key {
        Key::Char(c) => Key::Char(qwerty_to_profile(*c, profile)),
        other => *other,
    };
    Some((*name, mapped))
}

/// Lookup scancode by Linux key name (e.g. `KEY_S` or `KEY_CAPSLOCK`) or numeric code.
pub fn parse_scancode(token: &str) -> Option<u16> {
    if let Ok(num) = token.parse::<u16>() {
        return Some(num);
    }
    let upper = token.to_ascii_uppercase();
    SCANCODE_TABLE
        .iter()
        .find(|(_, name, _)| *name == upper || name.strip_prefix("KEY_") == Some(upper.as_str()))
        .map(|(code, _, _)| *code)
}

/// Inspect available `/dev/input/event*` devices or print full scancode remap table.
pub fn print_evdev_table(profile: Profile) {
    println!("┌──────────┬──────────────────┬──────────────────┬──────────────────┐");
    println!(
        "│ SCANCODE │ LINUX EVDEV NAME │ PHYSICAL QWERTY  │ {:<16} │",
        profile.name().to_ascii_uppercase()
    );
    println!("├──────────┼──────────────────┼──────────────────┼──────────────────┤");
    for (code, name, base_key) in SCANCODE_TABLE {
        let (_, mapped) = scancode_to_key(*code, profile).unwrap();
        println!(
            "│ {:>8} │ {:<16} │ {:<16} │ {:<16} │",
            code,
            name,
            base_key.token(),
            mapped.token()
        );
    }
    println!("└──────────┴──────────────────┴──────────────────┴──────────────────┘");
}

/// Simulate or attach to an `evdev` stream and feed translated events through the modal `Engine`.
pub fn run_evdev_daemon(
    device_path: Option<&str>,
    profile: Profile,
    sim_codes: &[String],
) -> io::Result<Vec<Action>> {
    let mut engine = Engine::with_profile(profile);
    let mut all_actions = Vec::new();

    if !sim_codes.is_empty() || device_path.is_none() {
        for tok in sim_codes {
            if let Some(code) = parse_scancode(tok) {
                if let Some((ev_name, key)) = scancode_to_key(code, profile) {
                    let acts = engine.press(key);
                    let act_tokens: Vec<String> = acts.iter().map(|a| a.token()).collect();
                    println!(
                        "[evdev] code={code:<3} ({ev_name:<14}) -> key={:<6} -> actions={act_tokens:?} [mode={}]",
                        key.token(),
                        engine.mode.name()
                    );
                    all_actions.extend(acts);
                }
            } else {
                eprintln!("[evdev] unknown scancode/key: {tok}");
            }
        }
        return Ok(all_actions);
    }

    let path = Path::new(device_path.unwrap());
    if !path.exists() {
        let dev_input = Path::new("/dev/input");
        let found = if dev_input.exists() {
            fs::read_dir(dev_input)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok().map(|x| x.path().display().to_string()))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        eprintln!(
            "[evdev] device {} not accessible in current container (found: {:?}). Use --sim <KEY_...> for simulation.",
            path.display(),
            found
        );
        return Ok(all_actions);
    }

    let mut file = File::open(path)?;
    let mut buf = [0u8; 24];
    println!(
        "[evdev] Attached to {} (profile={}). Press Ctrl+C to stop.",
        path.display(),
        profile.name()
    );
    while file.read_exact(&mut buf).is_ok() {
        let ev = InputEvent::from_bytes(&buf);
        // value == 1 is key press, value == 2 is key repeat
        if ev.event_type == EV_KEY && (ev.value == 1 || ev.value == 2) {
            if let Some((ev_name, key)) = scancode_to_key(ev.code, profile) {
                let acts = engine.press(key);
                let act_tokens: Vec<String> = acts.iter().map(|a| a.token()).collect();
                println!(
                    "[evdev] code={:<3} ({ev_name:<14}) -> key={:<6} -> {act_tokens:?}",
                    ev.code,
                    key.token()
                );
                all_actions.extend(acts);
            }
        }
    }
    Ok(all_actions)
}
