//! Terminal TUI Keyboard Visualizer & Interactive Simulator.
//!
//! Renders an ANSI-colored key grid for Dvorak, Programmer Dvorak, or QWERTY,
//! shows active Mode (`INSERT` / `NORMAL`), Layer (`ALPHA` / `SYM`), Shift state,
//! highlights the last pressed key, and shows the live simulated text buffer with cursor.

use crate::engine::{Action, Buffer, Engine, Mode, Shift};
use crate::layout::{profile_grid, Key, Layer, Profile};
use std::io::{self, BufRead, Write};

const RED: &str = "\x1b[38;5;196m";
const GOLD: &str = "\x1b[38;5;220m";
const GREEN: &str = "\x1b[38;5;46m";
const CYAN: &str = "\x1b[38;5;45m";
const WHITE: &str = "\x1b[38;5;255m";
const GRAY: &str = "\x1b[38;5;240m";
const BG_ACTIVE: &str = "\x1b[48;5;196;38;5;255;1m";
const BG_KEY: &str = "\x1b[48;5;236;38;5;255m";
const RST: &str = "\x1b[0m";

/// Render the current keyboard layout and status deck to a string.
pub fn render_keyboard(
    engine: &Engine,
    buffer: &Buffer,
    last_key: Option<Key>,
    last_actions: &[Action],
) -> String {
    let mut out = String::new();
    let shift_active = matches!(engine.shift, Shift::Once | Shift::Locked);
    let rows = profile_grid(engine.profile, engine.layer);

    let mode_badge = match engine.mode {
        Mode::Insert => format!("{GREEN}● INSERT{RST}"),
        Mode::Normal => format!("{GOLD}◆ NORMAL (VIM){RST}"),
    };
    let layer_badge = match engine.layer {
        Layer::Alpha => format!("{CYAN}ALPHA{RST}"),
        Layer::Sym => format!("{GOLD}SYMBOL{RST}"),
    };
    let shift_badge = match engine.shift {
        Shift::Off => format!("{GRAY}OFF{RST}"),
        Shift::Once => format!("{GOLD}ONCE (⇧){RST}"),
        Shift::Locked => format!("{RED}CAPS LOCK (⇪){RST}"),
    };

    out.push_str(&format!(
        "{GRAY}┌────────────────────────────────────────────────────────────────────────────┐{RST}\n"
    ));
    out.push_str(&format!(
        "{GRAY}│{RST} {RED}★ RUST-KEYBOARD{RST} {GRAY}│{RST} Profile: {WHITE}{:<11}{RST} {GRAY}│{RST} Mode: {:<18} {GRAY}│{RST} Layer: {:<15} {GRAY}│{RST}\n",
        engine.profile.name().to_ascii_uppercase(),
        mode_badge,
        layer_badge
    ));
    out.push_str(&format!(
        "{GRAY}│{RST} Shift: {:<19} {GRAY}│{RST} Pending: {GOLD}{:<10}{RST} {GRAY}│{RST} Cursor: {CYAN}{:<4}{RST} ({}B)          {GRAY}│{RST}\n",
        shift_badge,
        engine
            .pending
            .map(|p| format!("{p:?}"))
            .unwrap_or_else(|| "-".into()),
        buffer.cursor,
        buffer.text.len()
    ));
    out.push_str(&format!(
        "{GRAY}├────────────────────────────────────────────────────────────────────────────┤{RST}\n"
    ));

    // Render live buffer with visible cursor block
    let before = &buffer.text[..buffer.cursor];
    let after = &buffer.text[buffer.cursor..];
    let display_line = format!("{before}{RED}▌{RST}{after}").replace('\n', " ↵ ");
    out.push_str(&format!(
        "{GRAY}│{RST} {GOLD}BUFFER ▸{RST} {WHITE}{display_line}{RST}\n"
    ));

    let act_str: Vec<String> = last_actions.iter().map(|a| a.token()).collect();
    let act_summary = if act_str.is_empty() {
        "-".to_string()
    } else {
        act_str.join(", ")
    };
    out.push_str(&format!(
        "{GRAY}│{RST} {CYAN}ACTION ▸{RST} {GREEN}{act_summary}{RST}\n"
    ));
    out.push_str(&format!(
        "{GRAY}├────────────────────────────────────────────────────────────────────────────┤{RST}\n"
    ));

    // Render 4 keyboard rows
    for (r_idx, row) in rows.iter().enumerate() {
        out.push_str(&format!("{GRAY}│{RST}  "));
        if r_idx == 1 {
            out.push_str(" ");
        }
        for key in row {
            let is_active = last_key == Some(*key)
                || (*key == Key::Shift && shift_active)
                || (*key == Key::Nav && engine.mode == Mode::Normal)
                || (*key == Key::Sym && engine.layer == Layer::Sym);
            let lbl = key.label(shift_active);
            let cell = match key {
                Key::Space => format!("     {lbl}     "),
                Key::Shift | Key::Backspace | Key::Enter | Key::Sym | Key::Abc | Key::Nav | Key::Esc | Key::Tab => {
                    format!(" {lbl:^5} ")
                }
                Key::Char(_) => format!("  {lbl}  "),
            };
            if is_active {
                out.push_str(&format!("{BG_ACTIVE}{cell}{RST} "));
            } else {
                out.push_str(&format!("{BG_KEY}{cell}{RST} "));
            }
        }
        out.push('\n');
        if r_idx + 1 < rows.len() {
            out.push_str(&format!("{GRAY}│{RST}\n"));
        }
    }

    out.push_str(&format!(
        "{GRAY}└────────────────────────────────────────────────────────────────────────────┘{RST}\n"
    ));
    out
}

/// Run the interactive TUI REPL (or one-shot sequence if tokens are provided).
pub fn run_tui(profile: Profile, initial_tokens: &[String], interactive: bool) -> io::Result<()> {
    let mut engine = Engine::with_profile(profile);
    let mut buffer = Buffer::new();
    let mut last_key = None;
    let mut last_actions = Vec::new();

    for t in initial_tokens {
        if let Some(k) = Key::from_token(t) {
            let acts = engine.press(k);
            for a in &acts {
                buffer.apply(a);
            }
            last_key = Some(k);
            last_actions = acts;
        }
    }

    println!(
        "{}",
        render_keyboard(&engine, &buffer, last_key, &last_actions)
    );

    if !interactive {
        return Ok(());
    }

    println!(
        "{GOLD}Interactive TUI ready.{RST} Type tokens (e.g. {CYAN}SHIFT h e l l o NAV h x{RST}), {CYAN}:profile <dvorak|prog-dvorak|qwerty>{RST}, {CYAN}:clear{RST}, or {CYAN}:q{RST} to quit."
    );
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("{RED}rust-kb>{RST} ");
        stdout.flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if matches!(trimmed, ":q" | ":quit" | "quit" | "exit") {
            break;
        }
        if trimmed == ":clear" {
            buffer = Buffer::new();
            engine = Engine::with_profile(engine.profile);
            last_key = None;
            last_actions.clear();
            println!(
                "{}",
                render_keyboard(&engine, &buffer, last_key, &last_actions)
            );
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(":profile ") {
            if let Some(p) = Profile::from_name(rest.trim()) {
                engine.profile = p;
                println!(
                    "{}",
                    render_keyboard(&engine, &buffer, last_key, &last_actions)
                );
            } else {
                eprintln!("Unknown profile: {rest} (use dvorak, prog-dvorak, or qwerty)");
            }
            continue;
        }

        last_actions.clear();
        for tok in trimmed.split_whitespace() {
            match Key::from_token(tok) {
                Some(k) => {
                    let acts = engine.press(k);
                    for a in &acts {
                        buffer.apply(a);
                    }
                    last_key = Some(k);
                    last_actions.extend(acts);
                }
                None => {
                    // Treat multi-character word as individual characters in Insert/Normal mode
                    for ch in tok.chars() {
                        let k = Key::Char(ch);
                        let acts = engine.press(k);
                        for a in &acts {
                            buffer.apply(a);
                        }
                        last_key = Some(k);
                        last_actions.extend(acts);
                    }
                }
            }
        }
        println!(
            "{}",
            render_keyboard(&engine, &buffer, last_key, &last_actions)
        );
    }
    Ok(())
}
