//! Key definitions, hardware/soft profiles (Dvorak, Programmer Dvorak, QWERTY), and layers.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Shift,
    Backspace,
    Enter,
    Space,
    Tab,
    /// Sends a real Escape to the application. Never hijacks keyboard mode.
    Esc,
    /// Toggles Insert <-> Normal (Neovim navigation) mode.
    Nav,
    /// Switch to the symbol layer.
    Sym,
    /// Switch back to the alpha layer.
    Abc,
}

impl Key {
    pub fn token(self) -> String {
        match self {
            Key::Char(c) => c.to_string(),
            Key::Shift => "SHIFT".into(),
            Key::Backspace => "BKSP".into(),
            Key::Enter => "ENTER".into(),
            Key::Space => "SPACE".into(),
            Key::Tab => "TAB".into(),
            Key::Esc => "ESC".into(),
            Key::Nav => "NAV".into(),
            Key::Sym => "SYM".into(),
            Key::Abc => "ABC".into(),
        }
    }

    pub fn label(self, shift_active: bool) -> String {
        match self {
            Key::Char(c) => {
                let d = if shift_active { shifted(c) } else { c };
                d.to_string()
            }
            Key::Shift => "⇧".into(),
            Key::Backspace => "⌫".into(),
            Key::Enter => "⏎".into(),
            Key::Space => "SPACE".into(),
            Key::Tab => "TAB".into(),
            Key::Esc => "ESC".into(),
            Key::Nav => "NAV".into(),
            Key::Sym => "SYM".into(),
            Key::Abc => "ABC".into(),
        }
    }

    pub fn from_token(t: &str) -> Option<Key> {
        let mut it = t.chars();
        if let (Some(c), None) = (it.next(), it.next()) {
            return Some(Key::Char(c));
        }
        Some(match t {
            "SHIFT" => Key::Shift,
            "BKSP" | "BACKSPACE" => Key::Backspace,
            "ENTER" | "RET" => Key::Enter,
            "SPACE" | "SPC" => Key::Space,
            "TAB" => Key::Tab,
            "ESC" | "ESCAPE" => Key::Esc,
            "NAV" => Key::Nav,
            "SYM" => Key::Sym,
            "ABC" => Key::Abc,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Alpha,
    Sym,
}

impl Layer {
    pub const ALL: [Layer; 2] = [Layer::Alpha, Layer::Sym];

    pub fn name(self) -> &'static str {
        match self {
            Layer::Alpha => "alpha",
            Layer::Sym => "sym",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Dvorak,
    ProgDvorak,
    Qwerty,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Dvorak, Profile::ProgDvorak, Profile::Qwerty];

    pub fn name(self) -> &'static str {
        match self {
            Profile::Dvorak => "dvorak",
            Profile::ProgDvorak => "prog-dvorak",
            Profile::Qwerty => "qwerty",
        }
    }

    pub fn from_name(s: &str) -> Option<Profile> {
        match s.to_ascii_lowercase().as_str() {
            "dvorak" | "dv" | "hackvorak" => Some(Profile::Dvorak),
            "prog-dvorak" | "progdvorak" | "pdv" | "programmer" => Some(Profile::ProgDvorak),
            "qwerty" | "qw" => Some(Profile::Qwerty),
            _ => None,
        }
    }
}

/// Standard Dvorak alpha grid.
pub const ALPHA_ROWS: [&str; 4] = [
    "' , . p y f g c r l",
    "a o e u i d h t n s",
    "SHIFT q j k x b m w v z BKSP",
    "SYM ESC ; SPACE - NAV ENTER",
];

/// Programmer Dvorak alpha grid (brackets/punctuation prioritized).
pub const PROG_DVORAK_ROWS: [&str; 4] = [
    "; , . p y f g c r l",
    "a o e u i d h t n s",
    "SHIFT ' q j k x b m w v z BKSP",
    "SYM ESC / SPACE - NAV ENTER",
];

/// Standard QWERTY alpha grid (for comparison & remapping).
pub const QWERTY_ROWS: [&str; 4] = [
    "q w e r t y u i o p",
    "a s d f g h j k l ;",
    "SHIFT z x c v b n m , . BKSP",
    "SYM ESC ' SPACE - NAV ENTER",
];

/// Programmer symbol layer.
pub const SYM_ROWS: [&str; 4] = [
    "1 2 3 4 5 6 7 8 9 0",
    "! @ # $ % ^ & * ( )",
    "TAB ` ~ | \\ / ? = + _ BKSP",
    "ABC ESC { } SPACE [ ] < > ENTER",
];

pub fn grid(layer: Layer) -> Vec<Vec<Key>> {
    profile_grid(Profile::Dvorak, layer)
}

pub fn profile_grid(profile: Profile, layer: Layer) -> Vec<Vec<Key>> {
    let rows = match (profile, layer) {
        (_, Layer::Sym) => &SYM_ROWS,
        (Profile::Dvorak, Layer::Alpha) => &ALPHA_ROWS,
        (Profile::ProgDvorak, Layer::Alpha) => &PROG_DVORAK_ROWS,
        (Profile::Qwerty, Layer::Alpha) => &QWERTY_ROWS,
    };
    rows.iter()
        .map(|r| {
            r.split_whitespace()
                .map(|t| Key::from_token(t).unwrap_or_else(|| panic!("bad token {t:?} in layout")))
                .collect()
        })
        .collect()
}

/// Remap a physical QWERTY character to the target profile character.
pub fn qwerty_to_profile(c: char, profile: Profile) -> char {
    if profile == Profile::Qwerty {
        return c;
    }
    let lower = c.to_ascii_lowercase();
    let is_upper = c.is_ascii_uppercase();
    let mapped = match lower {
        'q' => match profile {
            Profile::ProgDvorak => ';',
            _ => '\'',
        },
        'w' => ',',
        'e' => '.',
        'r' => 'p',
        't' => 'y',
        'y' => 'f',
        'u' => 'g',
        'i' => 'c',
        'o' => 'r',
        'p' => 'l',
        '[' => '/',
        ']' => '=',
        'a' => 'a',
        's' => 'o',
        'd' => 'e',
        'f' => 'u',
        'g' => 'i',
        'h' => 'd',
        'j' => 'h',
        'k' => 't',
        'l' => 'n',
        ';' => 's',
        '\'' => '-',
        'z' => match profile {
            Profile::ProgDvorak => '\'',
            _ => ';',
        },
        'x' => 'q',
        'c' => 'j',
        'v' => 'k',
        'b' => 'x',
        'n' => 'b',
        'm' => 'm',
        ',' => 'w',
        '.' => 'v',
        '/' => 'z',
        '-' => '[',
        '=' => ']',
        other => other,
    };
    if is_upper {
        shifted(mapped)
    } else {
        mapped
    }
}

/// US-layout shift pairs for non-letters.
pub const SHIFT_PAIRS: &[(char, char)] = &[
    ('\'', '"'),
    (',', '<'),
    ('.', '>'),
    (';', ':'),
    ('-', '_'),
    ('/', '?'),
    ('=', '+'),
    ('`', '~'),
    ('[', '{'),
    (']', '}'),
    ('\\', '|'),
    ('1', '!'),
    ('2', '@'),
    ('3', '#'),
    ('4', '$'),
    ('5', '%'),
    ('6', '^'),
    ('7', '&'),
    ('8', '*'),
    ('9', '('),
    ('0', ')'),
];

/// What a character becomes with Shift held.
pub fn shifted(c: char) -> char {
    if c.is_ascii_lowercase() {
        return c.to_ascii_uppercase();
    }
    SHIFT_PAIRS
        .iter()
        .find(|(base, _)| *base == c)
        .map(|(_, s)| *s)
        .unwrap_or(c)
}
