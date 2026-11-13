//! Technicolor Workshop 24-bit ANSI renderer for `magic-tui` (`WORKSHOP F1`..`SETTINGS F5`).
//!
//! Renders static on-disk logs (`magic.log`, `activity.jsonl`) and past CI/audit reports with
//! zero external TUI dependencies outside `Cargo.lock` and zero background polling.

use crate::snapshot::BackstageSnapshot;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const BG_HEADER: &str = "\x1b[48;2;20;24;38m";
const FG_CREAM: &str = "\x1b[38;2;242;238;228m";
const FG_AMBER: &str = "\x1b[38;2;245;179;66m";
const FG_TEAL: &str = "\x1b[38;2;56;189;178m";
const FG_EMERALD: &str = "\x1b[38;2;74;222;128m";
const FG_CORAL: &str = "\x1b[38;2;248;113;113m";
const FG_VIOLET: &str = "\x1b[38;2;167;139;250m";
const FG_MUTED: &str = "\x1b[38;2;148;163;184m";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabId {
    Workshop,
    Booth,
    Mops,
    Mayhem,
    Settings,
}

impl TabId {
    pub const ALL: [Self; 5] = [Self::Workshop, Self::Booth, Self::Mops, Self::Mayhem, Self::Settings];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Workshop => "1:WORKSHOP F1",
            Self::Booth => "2:BOOTH F2",
            Self::Mops => "3:MOPS F3",
            Self::Mayhem => "4:MAYHEM F4",
            Self::Settings => "5:SETTINGS F5",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Workshop => Self::Booth,
            Self::Booth => Self::Mops,
            Self::Mops => Self::Mayhem,
            Self::Mayhem => Self::Settings,
            Self::Settings => Self::Workshop,
        }
    }
}

pub struct UiCursor {
    pub tab: TabId,
    pub row_idx: usize,
}

fn render_header(out: &mut String, snap: &BackstageSnapshot, active: TabId) {
    out.push_str("\x1b[2J\x1b[H");
    out.push_str(&format!(
        "{BG_HEADER}{BOLD}{FG_AMBER} * MAGIC BACKSTAGE / TECHNICOLOR WORKSHOP {RESET}{BG_HEADER}{FG_MUTED} (static viewer, rereads={}) {RESET}\n",
        snap.reread_count
    ));
    out.push(' ');
    for tab in TabId::ALL {
        if tab == active {
            out.push_str(&format!("{BOLD}{FG_TEAL}[ {} ]{RESET} ", tab.label()));
        } else {
            out.push_str(&format!("{FG_MUTED}  {}   {RESET}", tab.label()));
        }
    }
    out.push_str("\n──────────────────────────────────────────────────────────────────────────────────────────\n");
}

fn render_workshop(out: &mut String, snap: &BackstageSnapshot) {
    let p = &snap.project;
    out.push_str(&format!(" {BOLD}{FG_VIOLET}OPEN PROJECT SNAPSHOT{RESET}  {FG_CREAM}{}{RESET}\n", p.name));
    out.push_str(&format!(
        "   Path       : {}\n",
        p.path.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "(none — pass --project <slug>.magic)".to_string())
    ));
    out.push_str(&format!("   Audio      : {}\n", p.audio));
    out.push_str(&format!("   Storyboard : {}   |   Film: {}\n", p.storyboard, p.video_project));
    out.push_str(&format!("   Assets     : {} owned inside project directory\n\n", p.asset_count));
    out.push_str(&format!(" {BOLD}{FG_TEAL}RECENT ACTIVITY JOURNAL (activity.jsonl){RESET}\n"));
    if snap.activity_recent.is_empty() {
        out.push_str(&format!("   {DIM}(no recorded activity entries on disk){RESET}\n"));
    } else {
        for line in snap.activity_recent.iter().take(12) {
            out.push_str(&format!("   • {line}\n"));
        }
    }
}

fn render_booth(out: &mut String, snap: &BackstageSnapshot, offset: usize) {
    out.push_str(&format!(" {BOLD}{FG_AMBER}STATIC LOG TAIL (magic.log){RESET}  ({} lines)\n", snap.log_tail.len()));
    if snap.log_tail.is_empty() {
        out.push_str(&format!("   {DIM}(no magic.log entries found yet — pass --log-file or press r after running magic){RESET}\n"));
        return;
    }
    for line in snap.log_tail.iter().skip(offset.min(snap.log_tail.len().saturating_sub(1))).take(16) {
        let color = if line.contains("ERROR") {
            FG_CORAL
        } else if line.contains("WARN") {
            FG_AMBER
        } else {
            FG_CREAM
        };
        out.push_str(&format!("   {color}{line}{RESET}\n"));
    }
}

fn render_mops(out: &mut String, snap: &BackstageSnapshot, selected: usize) {
    out.push_str(&format!(" {BOLD}{FG_EMERALD}MOPS TASK HANDOFF SNAPSHOTS{RESET}  ({} rows)\n", snap.mops_tasks.len()));
    if snap.mops_tasks.is_empty() {
        out.push_str(&format!("   {DIM}(no Mops handoff JSON loaded — pass --mops-json <report.json>){RESET}\n"));
        return;
    }
    for (idx, row) in snap.mops_tasks.iter().enumerate().take(14) {
        let marker = if idx == selected % snap.mops_tasks.len() { "▸" } else { " " };
        out.push_str(&format!(
            " {marker} {FG_TEAL}{:<12}{RESET} {FG_CREAM}{:<34}{RESET} {FG_AMBER}{:<12}{RESET} {FG_MUTED}{}{RESET}\n",
            row.department, row.task, row.review, row.issue_pr
        ));
    }
}

fn render_mayhem(out: &mut String, snap: &BackstageSnapshot) {
    let m = &snap.mayhem;
    out.push_str(&format!(" {BOLD}{FG_CORAL}PAST CI & STATIC GATE RECEIPTS{RESET}  baseline={}  candidate={}\n", m.baseline_sha, m.candidate_sha));
    for tr in &m.candidate_tools {
        let col = if tr.error_count > 0 { FG_CORAL } else { FG_EMERALD };
        out.push_str(&format!("   {col}[{:^6}]{RESET} {:<20} issues={} errors={} ({})\n", tr.status, tr.tool_id, tr.issue_count, tr.error_count, tr.subtitle));
    }
    if !m.findings.is_empty() {
        out.push_str(&format!("\n {BOLD}Findings ({}){RESET}:\n", m.findings.len()));
        for f in m.findings.iter().take(10) {
            out.push_str(&format!("   • {}:{} [{}] {}\n", f.file_path, f.line, f.severity, f.message));
        }
    }
}

fn render_settings(out: &mut String, snap: &BackstageSnapshot) {
    out.push_str(&format!(" {BOLD}{FG_VIOLET}ON-DISK LOG & REPORT SOURCES{RESET}\n"));
    for src in &snap.sources {
        let badge = if src.exists { format!("{FG_EMERALD}FOUND{RESET}") } else { format!("{FG_MUTED}ABSENT{RESET}") };
        out.push_str(&format!("   [{badge}] {:<22} {} ({} B)\n", src.label, src.path.display(), src.bytes));
    }
}

pub fn render_screen(snap: &BackstageSnapshot, cursor: &UiCursor) -> String {
    let mut out = String::with_capacity(4096);
    render_header(&mut out, snap, cursor.tab);
    match cursor.tab {
        TabId::Workshop => render_workshop(&mut out, snap),
        TabId::Booth => render_booth(&mut out, snap, cursor.row_idx),
        TabId::Mops => render_mops(&mut out, snap, cursor.row_idx),
        TabId::Mayhem => render_mayhem(&mut out, snap),
        TabId::Settings => render_settings(&mut out, snap),
    }
    out.push_str("──────────────────────────────────────────────────────────────────────────────────────────\n");
    out.push_str(&format!(" {FG_MUTED}Commands: [1..5 / n] switch tab · [j / k] scroll · [r] reread logs from disk · [q] quit{RESET}\n"));
    out
}
