//! `rust-keyboard` CLI Entrypoint.
//!
//! Subcommands:
//! - `tui [--profile <dvorak|prog-dvorak|qwerty>] [--once] [TOKENS...]`
//! - `play [TOKENS...]`
//! - `evdev [--table] [--device /dev/input/eventX] [--profile <name>] [--sim KEY_...]`
//! - `jni [--kotlin | --header | --verify | --emit-dir <dir>]`
//! - `spec [--layout | --vectors | --write <dir>]`

use rust_keyboard::{
    evdev, jni_bridge,
    layout::{Key, Profile},
    spec, tui, Buffer, Engine,
};
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "\x1b[38;5;196m★ rust-keyboard v{VERSION}\x1b[0m — Tactical Rust Keyboard Engine (TUI + Linux evdev + Android JNI)

\x1b[38;5;220mUSAGE:\x1b[0m
  rust-keyboard <SUBCOMMAND> [OPTIONS]

\x1b[38;5;220mSUBCOMMANDS:\x1b[0m
  \x1b[38;5;46mtui\x1b[0m     [--profile <dvorak|prog-dvorak|qwerty>] [--once] [TOKENS...]
          Launch the interactive ANSI keyboard visualizer & live modal buffer simulator.
          Pass `--once` with tokens to render a single frame non-interactively.

  \x1b[38;5;46mplay\x1b[0m    [--profile <name>] [TOKENS...]
          Feed space-separated key tokens through the modal engine and print emitted actions.

  \x1b[38;5;46mevdev\x1b[0m   [--table] [--profile <name>] [--device <path>] [--sim <SCANCODE_OR_KEY_NAME>...]
          Linux evdev/uinput hardware keymapper. View scancode translation table (`--table`),
          simulate hardware scancode sequences (`--sim KEY_CAPSLOCK KEY_H KEY_J`), or attach to `/dev/input/eventX`.

  \x1b[38;5;46mjni\x1b[0m     [--verify] [--kotlin] [--header] [--emit-dir <dir>]
          Android IME / C-ABI bridge tools: verify FFI handle in-process, print Kotlin InputMethodService,
          print C header (`librust_keyboard.h`), or emit Android scaffold files.

  \x1b[38;5;46mspec\x1b[0m    [--layout | --vectors | --write <dir>]
          Generate golden JSON layout and test vectors (`layout.json`, `vectors.json`).

  \x1b[38;5;46mhelp\x1b[0m    Show this help message (`--help`, `-h`, `--version`, `-V`)."
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        // Default: render one-shot TUI banner if non-interactive stdin, or interactive TUI if TTY
        let _ = tui::run_tui(Profile::Dvorak, &[], false);
        println!("Run `rust-keyboard --help` for all subcommands (`tui`, `play`, `evdev`, `jni`, `spec`).");
        return;
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => {
            print_help();
        }
        "-V" | "--version" | "version" => {
            println!("rust-keyboard {VERSION}");
        }
        "tui" => {
            let mut profile = Profile::Dvorak;
            let mut once = false;
            let mut tokens = Vec::new();
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "-p" | "--profile" => {
                        if i + 1 < args.len() {
                            profile = Profile::from_name(&args[i + 1]).unwrap_or_else(|| {
                                eprintln!("Unknown profile: {}", args[i + 1]);
                                std::process::exit(1);
                            });
                            i += 2;
                        } else {
                            eprintln!("Missing argument for --profile");
                            std::process::exit(1);
                        }
                    }
                    "--once" | "-1" => {
                        once = true;
                        i += 1;
                    }
                    other => {
                        tokens.push(other.to_string());
                        i += 1;
                    }
                }
            }
            let interactive = !once && tokens.is_empty();
            if let Err(e) = tui::run_tui(profile, &tokens, interactive) {
                eprintln!("tui error: {e}");
                std::process::exit(1);
            }
        }
        "play" => {
            let mut profile = Profile::Dvorak;
            let mut tokens = Vec::new();
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "-p" | "--profile" if i + 1 < args.len() => {
                        profile = Profile::from_name(&args[i + 1]).unwrap_or(Profile::Dvorak);
                        i += 2;
                    }
                    other => {
                        tokens.push(other.to_string());
                        i += 1;
                    }
                }
            }
            let mut engine = Engine::with_profile(profile);
            let mut buffer = Buffer::new();
            if !tokens.is_empty() {
                for t in &tokens {
                    match Key::from_token(t) {
                        Some(k) => {
                            let acts = engine.press(k);
                            for a in &acts {
                                buffer.apply(a);
                            }
                            let act_strs: Vec<String> = acts.iter().map(|a| a.token()).collect();
                            println!(
                                "{t:>6} -> {act_strs:?}   [mode={} layer={} shift={} buf={:?}]",
                                engine.mode.name(),
                                engine.layer.name(),
                                engine.shift.name(),
                                buffer.text
                            );
                        }
                        None => eprintln!("unknown token: {t}"),
                    }
                }
                return;
            }
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                let l = line.expect("stdin");
                for t in l.split_whitespace() {
                    if let Some(k) = Key::from_token(t) {
                        let acts = engine.press(k);
                        for a in &acts {
                            buffer.apply(a);
                        }
                        let act_strs: Vec<String> = acts.iter().map(|a| a.token()).collect();
                        println!(
                            "{t:>6} -> {act_strs:?}   [mode={} layer={} shift={} buf={:?}]",
                            engine.mode.name(),
                            engine.layer.name(),
                            engine.shift.name(),
                            buffer.text
                        );
                    } else {
                        eprintln!("unknown token: {t}");
                    }
                }
            }
        }
        "evdev" => {
            let mut profile = Profile::Dvorak;
            let mut show_table = false;
            let mut device: Option<String> = None;
            let mut sim_codes: Vec<String> = Vec::new();
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--table" | "-t" => {
                        show_table = true;
                        i += 1;
                    }
                    "-p" | "--profile" if i + 1 < args.len() => {
                        profile = Profile::from_name(&args[i + 1]).unwrap_or(Profile::Dvorak);
                        i += 2;
                    }
                    "-d" | "--device" if i + 1 < args.len() => {
                        device = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--sim" | "-s" => {
                        i += 1;
                        while i < args.len() && !args[i].starts_with('-') {
                            sim_codes.push(args[i].clone());
                            i += 1;
                        }
                    }
                    other => {
                        sim_codes.push(other.to_string());
                        i += 1;
                    }
                }
            }
            if show_table || (device.is_none() && sim_codes.is_empty()) {
                evdev::print_evdev_table(profile);
            }
            if device.is_some() || !sim_codes.is_empty() {
                if let Err(e) = evdev::run_evdev_daemon(device.as_deref(), profile, &sim_codes) {
                    eprintln!("evdev error: {e}");
                    std::process::exit(1);
                }
            }
        }
        "jni" => {
            let mut i = 1;
            if args.len() == 1 {
                match jni_bridge::verify_ffi_bridge() {
                    Ok(json) => println!("[✓] JNI / C-ABI FFI Bridge verified: {json}"),
                    Err(e) => {
                        eprintln!("[✗] JNI FFI verification failed: {e}");
                        std::process::exit(1);
                    }
                }
                return;
            }
            while i < args.len() {
                match args[i].as_str() {
                    "--verify" | "-v" => {
                        match jni_bridge::verify_ffi_bridge() {
                            Ok(json) => println!("[✓] JNI / C-ABI FFI Bridge verified: {json}"),
                            Err(e) => {
                                eprintln!("[✗] JNI FFI verification failed: {e}");
                                std::process::exit(1);
                            }
                        }
                        i += 1;
                    }
                    "--kotlin" | "-k" => {
                        println!("{}", jni_bridge::kotlin_ime_scaffold());
                        i += 1;
                    }
                    "--header" | "-c" => {
                        println!("{}", jni_bridge::c_header_scaffold());
                        i += 1;
                    }
                    "--emit-dir" if i + 1 < args.len() => {
                        let dir = PathBuf::from(&args[i + 1]);
                        fs::create_dir_all(&dir).expect("create jni emit dir");
                        let kt_path = dir.join("RustKeyboardIMEService.kt");
                        let h_path = dir.join("librust_keyboard.h");
                        fs::write(&kt_path, jni_bridge::kotlin_ime_scaffold()).expect("write kt");
                        fs::write(&h_path, jni_bridge::c_header_scaffold()).expect("write header");
                        println!("wrote {}", kt_path.display());
                        println!("wrote {}", h_path.display());
                        i += 2;
                    }
                    other => {
                        eprintln!("Unknown jni option: {other}");
                        std::process::exit(1);
                    }
                }
            }
        }
        "spec" => {
            let flag = args.get(1).map(|s| s.as_str()).unwrap_or("--layout");
            match flag {
                "--layout" => print!("{}", spec::layout_json()),
                "--vectors" => print!("{}", spec::vectors_json()),
                "--write" => {
                    let dir = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| "spec".into()));
                    fs::create_dir_all(&dir).expect("create spec dir");
                    let l_path = dir.join("layout.json");
                    let v_path = dir.join("vectors.json");
                    fs::write(&l_path, spec::layout_json()).expect("write layout.json");
                    fs::write(&v_path, spec::vectors_json()).expect("write vectors.json");
                    println!("wrote {}", l_path.display());
                    println!("wrote {}", v_path.display());
                }
                other => {
                    eprintln!("Unknown spec option: {other}");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("Unknown subcommand: {other}. Run `rust-keyboard --help`.");
            std::process::exit(1);
        }
    }
}
