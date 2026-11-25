//! Modal Keyboard Engine & Live Text Buffer Simulator.
//!
//! Translates `Key` presses into platform-independent `Action`s, supporting:
//! - `Insert` mode (standard typing + Shift / Caps-Lock + Layer switching)
//! - `Normal` mode (Neovim navigation `hjkl`, word motions `w`/`e`/`b`, line motions `0`/`^`/`$`,
//!   edits `x`/`X`/`u`, insert transitions `i`/`a`/`I`/`A`/`o`/`O`, plus operator-pending `dd`, `dw`, `cw`, `cc`)
//! - `Buffer` state machine to apply `Action`s directly to a multi-line text buffer with cursor tracking & undo stack.

use crate::layout::{shifted, Key, Layer, Profile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Insert,
    Normal,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Insert => "insert",
            Mode::Normal => "normal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shift {
    Off,
    Once,
    Locked,
}

impl Shift {
    pub fn name(self) -> &'static str {
        match self {
            Shift::Off => "off",
            Shift::Once => "once",
            Shift::Locked => "locked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOp {
    Delete, // 'd'
    Change, // 'c'
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Commit(String),
    DeleteBackward,
    DeleteForward,
    DeleteWord,
    DeleteLine,
    Enter,
    Escape,
    Tab,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    WordForward,
    WordBackward,
    LineStart,
    LineEnd,
    Undo,
    ModeChanged(Mode),
    LayerChanged(Layer),
}

impl Action {
    pub fn token(&self) -> String {
        use Action::*;
        match self {
            Commit(s) => format!("commit:{s}"),
            DeleteBackward => "delete_back".into(),
            DeleteForward => "delete_fwd".into(),
            DeleteWord => "delete_word".into(),
            DeleteLine => "delete_line".into(),
            Enter => "enter".into(),
            Escape => "escape".into(),
            Tab => "tab".into(),
            CursorLeft => "left".into(),
            CursorRight => "right".into(),
            CursorUp => "up".into(),
            CursorDown => "down".into(),
            WordForward => "word_fwd".into(),
            WordBackward => "word_back".into(),
            LineStart => "line_start".into(),
            LineEnd => "line_end".into(),
            Undo => "undo".into(),
            ModeChanged(m) => format!("mode:{}", m.name()),
            LayerChanged(l) => format!("layer:{}", l.name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Engine {
    pub mode: Mode,
    pub layer: Layer,
    pub shift: Shift,
    pub profile: Profile,
    pub pending: Option<PendingOp>,
}

impl Default for Engine {
    fn default() -> Self {
        Engine {
            mode: Mode::Insert,
            layer: Layer::Alpha,
            shift: Shift::Off,
            profile: Profile::Dvorak,
            pending: None,
        }
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_profile(profile: Profile) -> Self {
        Self {
            profile,
            ..Self::default()
        }
    }

    pub fn press(&mut self, key: Key) -> Vec<Action> {
        match key {
            Key::Shift => {
                self.shift = match self.shift {
                    Shift::Off => Shift::Once,
                    Shift::Once => Shift::Locked,
                    Shift::Locked => Shift::Off,
                };
                vec![]
            }
            Key::Sym => {
                self.pending = None;
                self.set_layer(Layer::Sym)
            }
            Key::Abc => {
                self.pending = None;
                self.set_layer(Layer::Alpha)
            }
            Key::Nav => {
                self.pending = None;
                let next = match self.mode {
                    Mode::Insert => Mode::Normal,
                    Mode::Normal => Mode::Insert,
                };
                self.set_mode(next)
            }
            Key::Esc => {
                self.pending = None;
                vec![Action::Escape]
            }
            Key::Backspace => {
                self.pending = None;
                vec![Action::DeleteBackward]
            }
            Key::Enter => {
                self.pending = None;
                vec![Action::Enter]
            }
            Key::Tab => {
                self.pending = None;
                vec![Action::Tab]
            }
            Key::Space => {
                self.pending = None;
                match self.mode {
                    Mode::Insert => vec![Action::Commit(" ".into())],
                    Mode::Normal => vec![Action::CursorRight],
                }
            }
            Key::Char(c) => {
                let c = self.resolve_shift(c);
                match self.mode {
                    Mode::Insert => vec![Action::Commit(c.to_string())],
                    Mode::Normal => self.normal_command(c),
                }
            }
        }
    }

    fn resolve_shift(&mut self, c: char) -> char {
        let out = match self.shift {
            Shift::Off => c,
            Shift::Once | Shift::Locked => shifted(c),
        };
        if self.shift == Shift::Once {
            self.shift = Shift::Off;
        }
        out
    }

    fn set_layer(&mut self, layer: Layer) -> Vec<Action> {
        if self.layer == layer {
            return vec![];
        }
        self.layer = layer;
        vec![Action::LayerChanged(layer)]
    }

    fn set_mode(&mut self, mode: Mode) -> Vec<Action> {
        if self.mode == mode {
            return vec![];
        }
        self.mode = mode;
        vec![Action::ModeChanged(mode)]
    }

    fn normal_command(&mut self, c: char) -> Vec<Action> {
        use Action::*;
        if let Some(op) = self.pending.take() {
            return match (op, c) {
                (PendingOp::Delete, 'd') => vec![DeleteLine],
                (PendingOp::Delete, 'w') => vec![DeleteWord],
                (PendingOp::Delete, '$') => vec![DeleteWord],
                (PendingOp::Change, 'c') => {
                    let mut v = vec![DeleteLine];
                    v.extend(self.set_mode(Mode::Insert));
                    v
                }
                (PendingOp::Change, 'w') => {
                    let mut v = vec![DeleteWord];
                    v.extend(self.set_mode(Mode::Insert));
                    v
                }
                _ => vec![],
            };
        }

        match c {
            'd' => {
                self.pending = Some(PendingOp::Delete);
                return vec![];
            }
            'c' => {
                self.pending = Some(PendingOp::Change);
                return vec![];
            }
            _ => {}
        }

        let (mut acts, enter_insert) = match c {
            'h' => (vec![CursorLeft], false),
            'j' => (vec![CursorDown], false),
            'k' => (vec![CursorUp], false),
            'l' => (vec![CursorRight], false),
            'w' | 'e' => (vec![WordForward], false),
            'b' => (vec![WordBackward], false),
            '0' | '^' => (vec![LineStart], false),
            '$' => (vec![LineEnd], false),
            'x' => (vec![DeleteForward], false),
            'X' => (vec![DeleteBackward], false),
            'u' => (vec![Undo], false),
            'i' => (vec![], true),
            'a' => (vec![CursorRight], true),
            'I' => (vec![LineStart], true),
            'A' => (vec![LineEnd], true),
            'o' => (vec![LineEnd, Enter], true),
            'O' => (vec![LineStart, Enter, CursorUp], true),
            _ => (vec![], false),
        };
        if enter_insert {
            acts.extend(self.set_mode(Mode::Insert));
        }
        acts
    }
}

/// Interactive text buffer that executes `Action`s for the TUI simulator and tests.
#[derive(Debug, Clone)]
pub struct Buffer {
    pub text: String,
    pub cursor: usize,
    history: Vec<(String, usize)>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            history: Vec::new(),
        }
    }

    fn save_undo(&mut self) {
        if self.history.last().map(|(t, _)| t) != Some(&self.text) {
            self.history.push((self.text.clone(), self.cursor));
            if self.history.len() > 64 {
                self.history.remove(0);
            }
        }
    }

    pub fn apply(&mut self, action: &Action) {
        match action {
            Action::Commit(s) => {
                self.save_undo();
                self.text.insert_str(self.cursor, s);
                self.cursor += s.len();
            }
            Action::DeleteBackward => {
                if self.cursor > 0 {
                    self.save_undo();
                    let prev = self.text[..self.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.text.drain(prev..self.cursor);
                    self.cursor = prev;
                }
            }
            Action::DeleteForward => {
                if self.cursor < self.text.len() {
                    self.save_undo();
                    let next = self.text[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| self.cursor + c.len_utf8())
                        .unwrap_or(self.text.len());
                    self.text.drain(self.cursor..next);
                }
            }
            Action::DeleteWord => {
                if self.cursor < self.text.len() {
                    self.save_undo();
                    let rest = &self.text[self.cursor..];
                    let mut end = self.cursor;
                    for c in rest.chars() {
                        if c == '\n' {
                            break;
                        }
                        end += c.len_utf8();
                        if c.is_whitespace() {
                            break;
                        }
                    }
                    self.text.drain(self.cursor..end);
                }
            }
            Action::DeleteLine => {
                self.save_undo();
                let start = self.line_start_idx();
                let end = self.line_end_idx();
                let drain_end = if end < self.text.len() && self.text.as_bytes()[end] == b'\n' {
                    end + 1
                } else {
                    end
                };
                self.text.drain(start..drain_end);
                self.cursor = start.min(self.text.len());
            }
            Action::Enter => {
                self.save_undo();
                self.text.insert(self.cursor, '\n');
                self.cursor += 1;
            }
            Action::Tab => {
                self.save_undo();
                self.text.insert_str(self.cursor, "    ");
                self.cursor += 4;
            }
            Action::CursorLeft => {
                if self.cursor > 0 {
                    let prev = self.text[..self.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.cursor = prev;
                }
            }
            Action::CursorRight => {
                if self.cursor < self.text.len() {
                    let next = self.text[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| self.cursor + c.len_utf8())
                        .unwrap_or(self.text.len());
                    self.cursor = next;
                }
            }
            Action::LineStart => {
                self.cursor = self.line_start_idx();
            }
            Action::LineEnd => {
                self.cursor = self.line_end_idx();
            }
            Action::WordForward => {
                let rest = &self.text[self.cursor..];
                let mut offset = 0;
                let mut skipped_word = false;
                for c in rest.chars() {
                    if !c.is_whitespace() && !skipped_word {
                        offset += c.len_utf8();
                    } else if c.is_whitespace() {
                        skipped_word = true;
                        offset += c.len_utf8();
                    } else if skipped_word {
                        break;
                    }
                }
                self.cursor = (self.cursor + offset).min(self.text.len());
            }
            Action::WordBackward => {
                if self.cursor > 0 {
                    let before = &self.text[..self.cursor];
                    let mut idx = self.cursor;
                    let mut in_word = false;
                    for (i, c) in before.char_indices().rev() {
                        if !c.is_whitespace() {
                            in_word = true;
                            idx = i;
                        } else if in_word {
                            break;
                        } else {
                            idx = i;
                        }
                    }
                    self.cursor = idx;
                }
            }
            Action::CursorUp => {
                let start = self.line_start_idx();
                let col = self.cursor - start;
                if start > 0 {
                    let prev_end = start - 1;
                    let prev_start = self.text[..prev_end]
                        .rfind('\n')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    self.cursor = (prev_start + col).min(prev_end);
                }
            }
            Action::CursorDown => {
                let start = self.line_start_idx();
                let col = self.cursor - start;
                let end = self.line_end_idx();
                if end < self.text.len() {
                    let next_start = end + 1;
                    let next_end = self.text[next_start..]
                        .find('\n')
                        .map(|i| next_start + i)
                        .unwrap_or(self.text.len());
                    self.cursor = (next_start + col).min(next_end);
                }
            }
            Action::Undo => {
                if let Some((prev_text, prev_cur)) = self.history.pop() {
                    self.text = prev_text;
                    self.cursor = prev_cur.min(self.text.len());
                }
            }
            Action::Escape | Action::ModeChanged(_) | Action::LayerChanged(_) => {}
        }
    }

    fn line_start_idx(&self) -> usize {
        self.text[..self.cursor]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    fn line_end_idx(&self) -> usize {
        self.text[self.cursor..]
            .find('\n')
            .map(|i| self.cursor + i)
            .unwrap_or(self.text.len())
    }
}
