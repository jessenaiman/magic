//! Backstage (`apps/magic-tui`): interactive terminal companion for Magic Studio, supporting both
//! headless `magic::Session` and live `magic --control 9875` (`--connect 127.0.0.1:9875`) sessions.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

use std::io::{self, BufRead, Read, Write};

use clap::Parser;
use magic::{Session, command_specs};
use magic_mcp::{Backend, Remote, normalize_control_addr};
use serde_json::{Value, json};

const MAX_REPL_LINE: usize = 1024 * 1024;

#[derive(Parser)]
#[command(name = "magic-tui", about = "Magic Studio Backstage terminal console (headless or live `--connect 127.0.0.1:9875`)")]
struct Cli {
    /// Open this `.magic` project at launch (headless mode)
    #[arg(long)]
    project: Option<String>,
    /// Attach to a running `magic --control <PORT>` window (`127.0.0.1:9875` or `9875`)
    #[arg(long, visible_aliases = ["bridge", "control"])]
    connect: Option<String>,
}

fn parse_repl_line(line: &str) -> (&str, Value) {
    let mut parts = line.split_whitespace();
    let Some(cmd) = parts.next() else {
        return ("", json!({}));
    };
    let rest = line[cmd.len()..].trim();
    if rest.is_empty() {
        return (cmd, json!({}));
    }
    if rest.starts_with('{')
        && let Ok(v) = serde_json::from_str::<Value>(rest)
    {
        return (cmd, v);
    }
    let mut map = serde_json::Map::new();
    for tok in rest.split_whitespace() {
        if let Some((k, v)) = tok.split_once('=') {
            let parsed = serde_json::from_str::<Value>(v).unwrap_or_else(|_| json!(v));
            map.insert(k.to_owned(), parsed);
        }
    }
    (cmd, Value::Object(map))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut remote = cli.connect.as_deref().map(|spec| Remote::new(normalize_control_addr(spec), magic::workspace::control_token()));
    let mut session = remote.is_none().then(Session::user_session);

    if let (Some(path), Some(sess)) = (cli.project.as_deref(), session.as_mut()) {
        println!("Opening project: {path}...");
        match sess.execute("file.open", json!({ "path": path })) {
            Ok(res) => println!("Project loaded: {res}"),
            Err(e) => eprintln!("Error loading project: {e}"),
        }
    }

    println!("==================================================");
    println!(" Magic Studio Backstage (`magic-tui` · :9875)");
    println!("==================================================");
    println!("Shortcuts: help [filter] | status | storyboard | doctor | sync | quit");
    println!("Sample commands:");
    for spec in command_specs().iter().take(8) {
        println!("  - {:<26} {}", spec.id, spec.label);
    }
    println!("==================================================");

    let stdin = io::stdin();
    let mut locked = stdin.lock();
    loop {
        print!("backstage> ");
        io::stdout().flush()?;
        let mut buf = Vec::new();
        let n = locked.by_ref().take((MAX_REPL_LINE + 1) as u64).read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        if buf.len() > MAX_REPL_LINE {
            eprintln!("Error: input line exceeded {MAX_REPL_LINE} bytes");
            continue;
        }
        let Ok(line) = std::str::from_utf8(&buf) else {
            eprintln!("Error: input must be valid UTF-8");
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if matches!(trimmed, "quit" | "exit" | "q") {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("help") {
            let filter = rest.trim().to_ascii_lowercase();
            for spec in command_specs() {
                if filter.is_empty() || spec.id.to_ascii_lowercase().contains(&filter) {
                    println!("  {:<28} {}", spec.id, spec.label);
                }
            }
            continue;
        }
        let (raw_cmd, params) = match trimmed {
            "status" => ("workshop.status", json!({})),
            "storyboard" => ("storyboard.check", json!({})),
            "doctor" => ("app.doctor", json!({})),
            "sync" => ("video.sync_links", json!({"render": false})),
            other => parse_repl_line(other),
        };
        let res = if let Some(r) = remote.as_mut() {
            r.execute(raw_cmd, params)
        } else if let Some(s) = session.as_mut() {
            s.execute(raw_cmd, params).map_err(|e| e.to_string())
        } else {
            Err("no active backend".into())
        };
        match res {
            Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default()),
            Err(err) => eprintln!("Error: {err}"),
        }
    }
    Ok(())
}
