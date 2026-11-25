//! Golden test-vector & layout JSON generator.
//!
//! Exports `layout_json()` and `vectors_json()` compatible with the `hackvorak` spec contract,
//! plus extended operator-pending cases (`dd`, `dw`, `cw`, `cc`).

use crate::engine::Engine;
use crate::layout::{grid, profile_grid, Key, Layer, Profile, SHIFT_PAIRS};

pub const CASES: &[(&str, &[&str])] = &[
    ("type_hello", &["h", "e", "l", "l", "o"]),
    ("shift_once_capitalises_one_char", &["SHIFT", "h", "i"]),
    ("caps_lock_then_off", &["SHIFT", "SHIFT", "h", "i", "SHIFT", "x"]),
    ("shift_punctuation", &["SHIFT", "'", "SHIFT", ";", "SHIFT", "-", "SHIFT", ","]),
    ("space_backspace_enter_tab", &["a", "SPACE", "b", "BKSP", "ENTER", "SYM", "TAB"]),
    ("symbol_layer_roundtrip", &["SYM", "1", "{", "|", "ABC", "x"]),
    ("shift_on_symbol_layer", &["SYM", "SHIFT", "1", "2"]),
    ("redundant_layer_switch_is_silent", &["ABC", "SYM", "SYM"]),
    ("esc_passes_through_and_keeps_mode", &["ESC", "a", "NAV", "ESC"]),
    ("nav_hjkl", &["NAV", "h", "j", "k", "l"]),
    ("nav_word_and_line", &["NAV", "w", "e", "b", "SYM", "0", "^", "$", "ABC"]),
    ("nav_edit", &["NAV", "x", "SHIFT", "x", "u", "BKSP"]),
    ("nav_insert_variants", &["NAV", "i", "NAV", "a", "NAV", "SHIFT", "a", "NAV", "SHIFT", "i"]),
    ("nav_open_lines", &["NAV", "o", "NAV", "SHIFT", "o"]),
    ("nav_space_and_enter", &["NAV", "SPACE", "ENTER"]),
    ("nav_unknown_keys_ignored", &["NAV", "q", "z", ";"]),
    ("nav_toggle_back_types_again", &["NAV", "NAV", "q"]),
    ("caps_lock_persists_into_nav", &["SHIFT", "SHIFT", "NAV", "a", "x"]),
    ("nav_operator_pending_dd_dw_cw", &["NAV", "d", "d", "d", "w", "c", "w", "x"]),
];

fn js(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn arr<I: IntoIterator<Item = String>>(items: I) -> String {
    let v: Vec<String> = items.into_iter().map(|s| js(&s)).collect();
    format!("[{}]", v.join(", "))
}

fn comma(i: usize, len: usize) -> &'static str {
    if i + 1 < len { "," } else { "" }
}

pub fn layout_json() -> String {
    let mut o = String::from("{\n  \"version\": 2,\n  \"profiles\": [\n");
    for (pi, p) in Profile::ALL.iter().enumerate() {
        o += &format!("    {}{}\n", js(p.name()), comma(pi, Profile::ALL.len()));
    }
    o += "  ],\n  \"layers\": {\n";
    for (i, layer) in Layer::ALL.iter().enumerate() {
        o += &format!("    {}: [\n", js(layer.name()));
        let g = grid(*layer);
        for (r, row) in g.iter().enumerate() {
            o += &format!(
                "      {}{}\n",
                arr(row.iter().map(|k| k.token())),
                comma(r, g.len())
            );
        }
        o += &format!("    ]{}\n", comma(i, Layer::ALL.len()));
    }
    o += "  },\n  \"profile_layers\": {\n";
    for (pi, prof) in Profile::ALL.iter().enumerate() {
        o += &format!("    {}: [\n", js(prof.name()));
        let g = profile_grid(*prof, Layer::Alpha);
        for (r, row) in g.iter().enumerate() {
            o += &format!(
                "      {}{}\n",
                arr(row.iter().map(|k| k.token())),
                comma(r, g.len())
            );
        }
        o += &format!("    ]{}\n", comma(pi, Profile::ALL.len()));
    }
    o += "  },\n  \"shift_map\": {\n";
    let pairs: Vec<(char, char)> = ('a'..='z')
        .map(|c| (c, c.to_ascii_uppercase()))
        .chain(SHIFT_PAIRS.iter().copied())
        .collect();
    let lines: Vec<String> = pairs
        .iter()
        .map(|(a, b)| format!("    {}: {}", js(&a.to_string()), js(&b.to_string())))
        .collect();
    o += &lines.join(",\n");
    o += "\n  }\n}\n";
    o
}

pub fn vectors_json() -> String {
    let mut o = String::from("{\n  \"version\": 2,\n  \"cases\": [\n");
    for (i, (name, inputs)) in CASES.iter().enumerate() {
        let mut e = Engine::new();
        let mut actions = Vec::new();
        for t in inputs.iter() {
            let k = Key::from_token(t).unwrap_or_else(|| panic!("bad token {t:?} in case {name}"));
            actions.extend(e.press(k));
        }
        o += &format!(
            "    {{\"name\": {}, \"inputs\": {}, \"actions\": {}, \"final\": {{\"mode\": {}, \"layer\": {}, \"shift\": {}}}}}{}\n",
            js(name),
            arr(inputs.iter().map(|s| s.to_string())),
            arr(actions.iter().map(|a| a.token())),
            js(e.mode.name()),
            js(e.layer.name()),
            js(e.shift.name()),
            comma(i, CASES.len()),
        );
    }
    o += "  ]\n}\n";
    o
}
