//! Static, read-only disk snapshot loader for `magic-tui` (**Backstage · Technicolor Workshop**).
//!
//! Reads strictly from existing on-disk logs (`magic.log`, `activity.jsonl`, `activity-history.jsonl`),
//! past CI receipts (`ci-last.json`, `target/ci-report.jsonl`), saved `.magic` project files, and
//! caller-supplied Mops / Mayhem JSON snapshots. Performs **zero** polling, **zero** socket calls,
//! **zero** subprocess spawns, and **zero** network requests.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct SourceProbe {
    pub label: &'static str,
    pub path: PathBuf,
    pub exists: bool,
    pub bytes: u64,
}

#[derive(Clone, Debug, Default)]
#[expect(dead_code)]
pub struct MopsTaskRow {
    pub department: String,
    pub task: String,
    pub agent_harness: String,
    pub cast_member: String,
    pub issue_pr: String,
    pub review: String,
    pub action: String,
    pub session_id: String,
    pub branch: String,
    pub model_provider: String,
    pub resume_cmd: String,
    pub role_path: String,
    pub worktree_path: String,
    pub feedback: Vec<String>,
}

#[derive(Clone, Debug, Default)]
#[expect(dead_code)]
pub struct ToolResultRow {
    pub tool_id: String,
    pub subtitle: String,
    pub issue_count: i64,
    pub error_count: i64,
    pub files_analyzed: i64,
    pub status: String,
}

#[derive(Clone, Debug, Default)]
#[expect(dead_code)]
pub struct NativeFinding {
    pub tool_id: String,
    pub pattern_id: String,
    pub file_path: String,
    pub line: u64,
    pub severity: String,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
#[expect(dead_code)]
pub struct MayhemComparison {
    pub baseline_sha: String,
    pub candidate_sha: String,
    pub baseline_tools: Vec<ToolResultRow>,
    pub candidate_tools: Vec<ToolResultRow>,
    pub findings: Vec<NativeFinding>,
    pub native_errors: Vec<String>,
    pub baseline_path: Option<PathBuf>,
    pub candidate_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Default)]
#[expect(dead_code)]
pub struct ProjectSummary {
    pub name: String,
    pub path: Option<PathBuf>,
    pub audio: String,
    pub storyboard: String,
    pub video_project: String,
    pub draft_approved: bool,
    pub asset_count: usize,
}

#[derive(Clone, Debug, Default)]
#[expect(dead_code)]
pub struct BackstageSnapshot {
    pub loaded_at_utc: String,
    pub reread_count: u64,
    pub project: ProjectSummary,
    pub activity_recent: Vec<String>,
    pub log_tail: Vec<String>,
    pub ci_entries: Vec<String>,
    pub mops_tasks: Vec<MopsTaskRow>,
    pub mops_provenance: String,
    pub mayhem: MayhemComparison,
    pub config_preview: Vec<String>,
    pub sources: Vec<SourceProbe>,
}

#[derive(Clone, Debug, Default)]
pub struct SnapshotInputs {
    pub project: Option<PathBuf>,
    pub mops_json: Option<PathBuf>,
    pub baseline_json: Option<PathBuf>,
    pub candidate_json: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
}

fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("magic")
}

fn data_dir() -> PathBuf {
    magic_engine::workspace::user_data_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("magic/config.toml")
}

fn probe(label: &'static str, path: PathBuf) -> SourceProbe {
    match fs::metadata(&path) {
        Ok(meta) if meta.is_file() => SourceProbe { label, path, exists: true, bytes: meta.len() },
        _ => SourceProbe { label, path, exists: false, bytes: 0 },
    }
}

fn read_tail_lines(path: &Path, max_lines: usize) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(max_lines);
    all.get(start..).unwrap_or(&[]).iter().map(|s| (*s).to_owned()).collect()
}

fn now_label() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("epoch:{secs}s")
}

fn str_or<'a>(v: &'a Value, key: &str, default: &'a str) -> String {
    v.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or(default).to_owned()
}

fn load_project_summary(explicit: Option<&Path>) -> ProjectSummary {
    let candidate = explicit.map(Path::to_path_buf);
    let Some(path) = candidate.filter(|p| p.is_file()) else {
        return ProjectSummary {
            name: "No .magic project specified (pass --project <file.magic>)".into(),
            path: None,
            audio: "—".into(),
            storyboard: "storyboard.designcraft (DesignCraft system template)".into(),
            video_project: "—".into(),
            draft_approved: false,
            asset_count: 0,
        };
    };
    let Ok(bytes) = fs::read(&path) else {
        return ProjectSummary { name: path.display().to_string(), path: Some(path), ..ProjectSummary::default() };
    };
    let doc = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
    ProjectSummary {
        name: str_or(&doc, "name", "Untitled"),
        audio: doc.pointer("/workshop/audio").and_then(Value::as_str).unwrap_or("—").to_owned(),
        storyboard: doc.pointer("/workshop/storyboard").and_then(Value::as_str).unwrap_or("storyboard.designcraft").to_owned(),
        video_project: doc.pointer("/workshop/video_project").and_then(Value::as_str).unwrap_or("—").to_owned(),
        draft_approved: doc.pointer("/workshop/draft_approved").and_then(Value::as_bool).unwrap_or(false),
        asset_count: doc.get("assets").and_then(Value::as_array).map_or(0, Vec::len),
        path: Some(path),
    }
}

fn parse_activity_jsonl(paths: &[PathBuf]) -> Vec<String> {
    let mut out = Vec::new();
    for p in paths {
        for line in read_tail_lines(p, 40) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                let cmd = str_or(&v, "command", "event");
                let stage = str_or(&v, "stage", "ok");
                let proj = str_or(&v, "project", "-");
                let detail = v.get("error").and_then(Value::as_str).unwrap_or("");
                if detail.is_empty() {
                    out.push(format!("[{stage}] {cmd}  ({proj})"));
                } else {
                    out.push(format!("[{stage}] {cmd}: {detail}"));
                }
            }
        }
    }
    out
}

fn parse_mops_snapshot(path: &Path) -> (Vec<MopsTaskRow>, String) {
    let Ok(raw) = fs::read_to_string(path) else {
        return (Vec::new(), format!("No snapshot at {} (pass --mops-json <file.json>)", path.display()));
    };
    let Ok(doc) = serde_json::from_str::<Value>(&raw) else {
        return (Vec::new(), format!("Malformed JSON in {}", path.display()));
    };
    let items = doc.get("tasks").or_else(|| doc.get("records")).and_then(Value::as_array).or_else(|| doc.as_array());
    let mut rows = Vec::new();
    for item in items.into_iter().flatten() {
        let feedback =
            item.get("feedback").and_then(Value::as_array).map(|arr| arr.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default();
        rows.push(MopsTaskRow {
            department: str_or(item, "department", "Unknown"),
            task: str_or(item, "task", &str_or(item, "title", "Recorded task")),
            agent_harness: str_or(item, "agent_harness", &str_or(item, "agent", "Unknown")),
            cast_member: str_or(item, "cast", "Scooter"),
            issue_pr: str_or(item, "issue_pr", &str_or(item, "pr", "Unknown")),
            review: str_or(item, "review", "Draft"),
            action: str_or(item, "action", "Needs decision"),
            session_id: str_or(item, "session_id", "Unknown"),
            branch: str_or(item, "branch", "Unknown"),
            model_provider: str_or(item, "model_provider", "Unknown"),
            resume_cmd: str_or(item, "resume_command", "Unknown"),
            role_path: str_or(item, "role_path", "AGENTS.md"),
            worktree_path: str_or(item, "worktree", "Unknown"),
            feedback,
        });
    }
    let prov = str_or(&doc, "provenance", &format!("Static snapshot from {} · explicit read-only", path.display()));
    (rows, prov)
}

fn parse_mayhem_side(path: Option<&Path>) -> (String, Vec<ToolResultRow>, Vec<NativeFinding>, Vec<String>) {
    let Some(p) = path.filter(|p| p.is_file()) else {
        return ("not supplied".into(), Vec::new(), Vec::new(), Vec::new());
    };
    let Ok(raw) = fs::read_to_string(p) else {
        return ("unreadable".into(), Vec::new(), Vec::new(), Vec::new());
    };
    let Ok(doc) = serde_json::from_str::<Value>(&raw) else {
        return ("invalid JSON".into(), Vec::new(), Vec::new(), Vec::new());
    };
    let sha = str_or(&doc, "commitSha", &str_or(&doc, "sha", "caller-supplied"));
    let mut tools = Vec::new();
    let mut findings = Vec::new();
    let mut errs = Vec::new();
    for tr in doc.get("toolResults").and_then(Value::as_array).into_iter().flatten() {
        let tid = str_or(tr, "toolId", "tool");
        let sub = str_or(tr, "toolName", &tid);
        let issues = tr.get("issueCount").and_then(Value::as_i64).unwrap_or_else(|| tr.get("issues").and_then(Value::as_array).map_or(0, |a| a.len() as i64));
        let ecount = tr.get("errorCount").and_then(Value::as_i64).unwrap_or(0);
        let files = tr.get("filesAnalyzed").and_then(Value::as_i64).unwrap_or(0);
        let status = str_or(tr, "status", "success");
        tools.push(ToolResultRow { tool_id: tid.clone(), subtitle: sub, issue_count: issues, error_count: ecount, files_analyzed: files, status });
        for iss in tr.get("issues").and_then(Value::as_array).into_iter().flatten() {
            findings.push(NativeFinding {
                tool_id: tid.clone(),
                pattern_id: str_or(iss, "patternId", &str_or(iss, "code", "rule")),
                file_path: str_or(iss, "filePath", &str_or(iss, "file", "-")),
                line: iss.get("line").and_then(Value::as_u64).unwrap_or(0),
                severity: str_or(iss, "severity", "Warning"),
                message: str_or(iss, "message", ""),
            });
        }
        for er in tr.get("errors").and_then(Value::as_array).into_iter().flatten() {
            if let Some(s) = er.as_str() {
                errs.push(format!("[{tid}] {s}"));
            } else {
                errs.push(format!("[{tid}] {}", str_or(er, "message", "error")));
            }
        }
    }
    (sha, tools, findings, errs)
}

pub fn load_backstage_snapshot(inputs: &SnapshotInputs, prev_count: u64) -> BackstageSnapshot {
    let st_dir = state_dir();
    let dt_dir = data_dir();
    let cfg_file = config_path();
    let act_file = st_dir.join("activity.jsonl");
    let act_hist = st_dir.join("activity-history.jsonl");
    let ci_last = st_dir.join("ci-last.json");
    let ci_jsonl = PathBuf::from("target/ci-report.jsonl");
    let log_file = inputs.log_file.clone().unwrap_or_else(|| dt_dir.join("logs/magic.log"));
    let mops_file = inputs.mops_json.clone().unwrap_or_else(|| st_dir.join("mops-snapshot.json"));
    let base_file = inputs.baseline_json.clone().unwrap_or_else(|| st_dir.join("mayhem-baseline.json"));
    let cand_file = inputs.candidate_json.clone().unwrap_or_else(|| st_dir.join("mayhem-candidate.json"));

    let sources = vec![
        probe("activity.jsonl", act_file.clone()),
        probe("activity-history.jsonl", act_hist.clone()),
        probe("magic.log", log_file.clone()),
        probe("ci-last.json", ci_last.clone()),
        probe("target/ci-report.jsonl", ci_jsonl.clone()),
        probe("mops-snapshot.json", mops_file.clone()),
        probe("mayhem-baseline.json", base_file.clone()),
        probe("mayhem-candidate.json", cand_file.clone()),
        probe("config.toml", cfg_file.clone()),
    ];

    let mut ci_entries = read_tail_lines(&ci_last, 25);
    ci_entries.extend(read_tail_lines(&ci_jsonl, 25));

    let (mops_tasks, mops_provenance) = parse_mops_snapshot(&mops_file);
    let (base_sha, base_tools, base_findings, mut native_errors) = parse_mayhem_side(Some(&base_file));
    let (cand_sha, cand_tools, cand_findings, cand_errs) = parse_mayhem_side(Some(&cand_file));
    native_errors.extend(cand_errs);
    let findings = if !cand_findings.is_empty() { cand_findings } else { base_findings };

    BackstageSnapshot {
        loaded_at_utc: now_label(),
        reread_count: prev_count + 1,
        project: load_project_summary(inputs.project.as_deref()),
        activity_recent: parse_activity_jsonl(&[act_file, act_hist]),
        log_tail: read_tail_lines(&log_file, 60),
        ci_entries,
        mops_tasks,
        mops_provenance,
        mayhem: MayhemComparison {
            baseline_sha: base_sha,
            candidate_sha: cand_sha,
            baseline_tools: base_tools,
            candidate_tools: cand_tools,
            findings,
            native_errors,
            baseline_path: base_file.is_file().then_some(base_file),
            candidate_path: cand_file.is_file().then_some(cand_file),
        },
        config_preview: read_tail_lines(&cfg_file, 45),
        sources,
    }
}
