use rust_keyboard::{
    evdev::{self, InputEvent, EV_KEY},
    grid, jni_bridge, profile_grid, spec, Action,
    Action::*,
    Buffer, Engine, Key, Layer, Mode, Profile, Shift,
};
use std::collections::HashMap;

fn run(tokens: &str) -> (Vec<Action>, Engine, Buffer) {
    let mut e = Engine::new();
    let mut b = Buffer::new();
    let mut out = Vec::new();
    for t in tokens.split_whitespace() {
        let acts = e.press(Key::from_token(t).expect(t));
        for a in &acts {
            b.apply(a);
        }
        out.extend(acts);
    }
    (out, e, b)
}

#[test]
fn alpha_has_every_letter_exactly_once_across_all_profiles() {
    for profile in Profile::ALL {
        let mut seen: HashMap<char, usize> = HashMap::new();
        for row in profile_grid(profile, Layer::Alpha) {
            for k in row {
                if let Key::Char(ch) = k {
                    *seen.entry(ch).or_default() += 1;
                }
            }
        }
        for ch in 'a'..='z' {
            assert_eq!(
                seen.get(&ch),
                Some(&1),
                "profile {} missing letter {ch}",
                profile.name()
            );
        }
    }
}

#[test]
fn dvorak_home_row_matches_spec() {
    let home: String = grid(Layer::Alpha)[1]
        .iter()
        .map(|k| match k {
            Key::Char(c) => *c,
            _ => '?',
        })
        .collect();
    assert_eq!(home, "aoeuidhtns");
}

#[test]
fn modal_typing_and_buffer_editing_works() {
    let (acts, eng, buf) = run("SHIFT h e l l o SPACE w o r l d NAV b d w i SHIFT r u s t");
    assert_eq!(eng.mode, Mode::Insert);
    assert_eq!(eng.shift, Shift::Off);
    assert!(acts.contains(&DeleteWord));
    assert_eq!(buf.text, "Hello Rust");
}

#[test]
fn operator_pending_dd_and_undo_work() {
    let (_, _, buf) = run("a b c ENTER d e f NAV d d");
    assert_eq!(buf.text, "abc\n");
    let (_, _, buf_undone) = run("a b c ENTER d e f NAV d d u");
    assert_eq!(buf_undone.text, "abc\ndef");
}

#[test]
fn evdev_scancode_mapping_and_input_event_serialization() {
    // KEY_S (31 on physical QWERTY) -> 'o' on Dvorak
    let (name, key) = evdev::scancode_to_key(31, Profile::Dvorak).expect("scancode 31");
    assert_eq!(name, "KEY_S");
    assert_eq!(key, Key::Char('o'));

    // KEY_MINUS (12) -> '[', KEY_EQUAL (13) -> ']', KEY_LEFTBRACE (26) -> '/', KEY_RIGHTBRACE (27) -> '='
    assert_eq!(evdev::scancode_to_key(12, Profile::Dvorak).unwrap().1, Key::Char('['));
    assert_eq!(evdev::scancode_to_key(13, Profile::Dvorak).unwrap().1, Key::Char(']'));
    assert_eq!(evdev::scancode_to_key(26, Profile::Dvorak).unwrap().1, Key::Char('/'));
    assert_eq!(evdev::scancode_to_key(27, Profile::Dvorak).unwrap().1, Key::Char('='));

    // Programmer Dvorak swaps top-left ';' and bottom-left '\''
    assert_eq!(evdev::scancode_to_key(16, Profile::ProgDvorak).unwrap().1, Key::Char(';'));
    assert_eq!(evdev::scancode_to_key(44, Profile::ProgDvorak).unwrap().1, Key::Char('\''));

    // KEY_CAPSLOCK (58) -> Key::Nav
    let (_, cap_key) = evdev::scancode_to_key(58, Profile::Dvorak).expect("scancode 58");
    assert_eq!(cap_key, Key::Nav);

    // 24-byte InputEvent binary roundtrip
    let ev = InputEvent {
        tv_sec: 1700000000,
        tv_usec: 123456,
        event_type: EV_KEY,
        code: 31,
        value: 1,
    };
    let bytes = ev.to_bytes();
    assert_eq!(bytes.len(), 24);
    assert_eq!(InputEvent::from_bytes(&bytes), ev);
}

#[test]
fn jni_c_abi_ffi_bridge_roundtrip() {
    let json = jni_bridge::verify_ffi_bridge().expect("FFI bridge check");
    assert!(json.contains("\"commit:H\""));
    assert!(json.contains("\"commit:i\""));
    assert!(json.contains("\"mode:normal\""));
    assert!(json.contains("\"delete_fwd\""));
}

#[test]
fn spec_json_generators_are_valid() {
    let layout = spec::layout_json();
    let vectors = spec::vectors_json();
    assert!(layout.contains("\"dvorak\""));
    assert!(layout.contains("\"prog-dvorak\""));
    assert!(vectors.contains("\"nav_operator_pending_dd_dw_cw\""));
}
