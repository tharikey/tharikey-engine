//! `tharikey` CLI — author/test schemes without the macOS app.
//!
//! ```text
//! tharikey transliterate --scheme <id|path> "<latin>"
//! tharikey reverse       --scheme <id|path> "<thaana>"
//! tharikey validate      --scheme <id|path>
//! tharikey list
//! ```
//!
//! `--scheme` accepts a bundled id (e.g. `male-latin`) or a path to a `.toml`.

use std::path::Path;
use std::process::ExitCode;
use tharikey_core::{load_seed, merge_base, reverse, transliterate, RawScheme, Scheme, Severity};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            eprintln!();
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "\
usage:
  tharikey transliterate --scheme <id|path> \"<latin>\"
  tharikey reverse       --scheme <id|path> \"<thaana>\"
  tharikey validate      --scheme <id|path>
  tharikey list";

fn run(args: &[String]) -> Result<(), String> {
    let cmd = args.first().ok_or("missing command")?.as_str();
    match cmd {
        "list" => {
            for id in tharikey_core::bundled_scheme_ids() {
                match load_seed(id) {
                    Ok(s) => println!("{id:<12} {:<6} {}", kind(&s), s.meta().name),
                    Err(e) => println!("{id:<12} (failed to load: {e})"),
                }
            }
            Ok(())
        }
        "transliterate" | "reverse" | "validate" => {
            let scheme = load_scheme(scheme_arg(args)?)?;
            match cmd {
                "validate" => {
                    let issues = scheme.validate();
                    if issues.is_empty() {
                        println!("ok: no issues");
                    } else {
                        for i in &issues {
                            let tag = match i.severity {
                                Severity::Error => "error",
                                Severity::Warn => "warn",
                            };
                            println!("{tag}: {}", i.message);
                        }
                    }
                    Ok(())
                }
                "transliterate" => {
                    let text = positional(args).ok_or("missing input text")?;
                    println!("{}", transliterate(&scheme, &text));
                    Ok(())
                }
                "reverse" => {
                    let text = positional(args).ok_or("missing input text")?;
                    match reverse(&scheme, &text) {
                        Some(out) => {
                            println!("{out}");
                            Ok(())
                        }
                        None => Err("scheme is not reversible".into()),
                    }
                }
                _ => unreachable!(),
            }
        }
        other => Err(format!("unknown command `{other}`")),
    }
}

fn kind(s: &Scheme) -> &'static str {
    match s {
        Scheme::Keys(_) => "keys",
        Scheme::Text(_) => "text",
    }
}

fn scheme_arg(args: &[String]) -> Result<&str, String> {
    let pos = args
        .iter()
        .position(|a| a == "--scheme")
        .ok_or("missing --scheme")?;
    args.get(pos + 1)
        .map(String::as_str)
        .ok_or("missing value for --scheme".into())
}

/// First positional arg after the command that isn't a flag or a flag value.
fn positional(args: &[String]) -> Option<String> {
    let mut i = 1; // skip command
    while i < args.len() {
        if args[i] == "--scheme" {
            i += 2;
            continue;
        }
        if args[i].starts_with("--") {
            i += 1;
            continue;
        }
        return Some(args[i].clone());
    }
    None
}

fn load_scheme(arg: &str) -> Result<Scheme, String> {
    if arg.ends_with(".toml") || arg.contains('/') {
        raw_from_path(Path::new(arg))?
            .resolve()
            .map_err(|e| e.to_string())
    } else {
        load_seed(arg).map_err(|e| e.to_string())
    }
}

/// Read a scheme `.toml`, resolving `[meta] base` inheritance (child overrides base) relative to the
/// file's directory. **CLI-only convenience** — filesystem I/O is the host's job, so this lives here
/// rather than in the engine core (which stays I/O-free); it's built on the public `from_toml_str` +
/// `merge_base`.
fn raw_from_path(path: &Path) -> Result<RawScheme, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let scheme = RawScheme::from_toml_str(&s).map_err(|e| e.to_string())?;
    match scheme.meta.base.clone() {
        Some(base_id) => {
            let dir = path.parent().unwrap_or_else(|| Path::new("."));
            let base = raw_from_path(&dir.join(format!("{base_id}.toml")))?;
            Ok(merge_base(base, scheme))
        }
        None => Ok(scheme),
    }
}
