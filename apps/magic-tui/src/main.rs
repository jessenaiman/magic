//! Backstage (`apps/magic-tui`): **Technicolor Workshop** static log & report viewer.
//!
//! Renders the 5-tab Backstage view (`WORKSHOP F1`, `BOOTH F2`, `MOPS F3`, `MAYHEM F4`,
//! `SETTINGS F5`) strictly from existing on-disk logs (`magic.log`, `activity.jsonl`) and past
//! JSON/JSONL reports. Performs **zero** background polling, **zero** socket connections, and
//! **zero** subprocess spawns — blocks on stdin and rereads disk only on `r`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod snapshot;
mod ui;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use clap::Parser;
use snapshot::{SnapshotInputs, load_backstage_snapshot};
use ui::{TabId, UiCursor, render_screen};

#[derive(Parser)]
#[command(
    name = "magic-tui",
    about = "Magic Studio Backstage (Technicolor Workshop — static read-only viewer over existing logs and past reports)"
)]
struct Cli {
    /// Optional `.magic` project file to inspect on disk (read-only)
    #[arg(long)]
    project: Option<PathBuf>,
    /// Optional imported Mops task handoff snapshot JSON
    #[arg(long)]
    mops_json: Option<PathBuf>,
    /// Optional Mayhem baseline toolResults JSON
    #[arg(long)]
    baseline_json: Option<PathBuf>,
    /// Optional Mayhem candidate toolResults JSON
    #[arg(long)]
    candidate_json: Option<PathBuf>,
    /// Optional explicit path to `magic.log`
    #[arg(long)]
    log_file: Option<PathBuf>,
    /// Initial tab (`workshop`, `booth`, `mops`, `mayhem`, or `settings`)
    #[arg(long, default_value = "workshop")]
    tab: String,
    /// Print a non-interactive JSON summary of discovered logs/reports and exit
    #[arg(long)]
    dump: bool,
}

fn parse_tab(raw: &str) -> TabId {
    match raw.trim().to_ascii_lowercase().as_str() {
        "2" | "f2" | "booth" => TabId::Booth,
        "3" | "f3" | "mops" => TabId::Mops,
        "4" | "f4" | "mayhem" => TabId::Mayhem,
        "5" | "f5" | "settings" => TabId::Settings,
        _ => TabId::Workshop,
    }
}

fn run_interactive(inputs: &SnapshotInputs, initial_tab: TabId) -> io::Result<()> {
    let mut snap = load_backstage_snapshot(inputs, 0);
    let mut cursor = UiCursor { tab: initial_tab, row_idx: 0 };
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

    loop {
        print!("{}", render_screen(&snap, &cursor));
        print!("backstage> ");
        io::stdout().flush()?;
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        match line.trim().to_ascii_lowercase().as_str() {
            "q" | "quit" | "exit" => break,
            "r" | "refresh" | "reload" => snap = load_backstage_snapshot(inputs, snap.reread_count.saturating_add(1)),
            "1" | "f1" | "workshop" => cursor.tab = TabId::Workshop,
            "2" | "f2" | "booth" => cursor.tab = TabId::Booth,
            "3" | "f3" | "mops" => cursor.tab = TabId::Mops,
            "4" | "f4" | "mayhem" => cursor.tab = TabId::Mayhem,
            "5" | "f5" | "settings" => cursor.tab = TabId::Settings,
            "n" | "tab" => cursor.tab = cursor.tab.next(),
            "j" | "down" => cursor.row_idx = cursor.row_idx.saturating_add(1),
            "k" | "up" => cursor.row_idx = cursor.row_idx.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let inputs = SnapshotInputs {
        project: cli.project,
        mops_json: cli.mops_json,
        baseline_json: cli.baseline_json,
        candidate_json: cli.candidate_json,
        log_file: cli.log_file,
    };

    if cli.dump {
        let snap = load_backstage_snapshot(&inputs, 0);
        let sources: Vec<serde_json::Value> = snap
            .sources
            .iter()
            .map(|s| serde_json::json!({"label": s.label, "path": s.path, "exists": s.exists, "bytes": s.bytes}))
            .collect();
        let out = serde_json::json!({
            "mode": "static_read_only",
            "polling": false,
            "project": snap.project.name,
            "activity_count": snap.activity_recent.len(),
            "log_lines": snap.log_tail.len(),
            "mops_tasks": snap.mops_tasks.len(),
            "mayhem_findings": snap.mayhem.findings.len(),
            "sources": sources,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    run_interactive(&inputs, parse_tab(&cli.tab))?;
    Ok(())
}
