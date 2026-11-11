//! `cargo xtask ci`: build groups, fmt, architecture layers, and Clippy (`-D warnings`).
//!
//! Test execution is held off by default until visual design confirmation (`MAGIC_RUN_TESTS=1`
//! opts into `cargo test --workspace --no-run`) so premature UI assertions never block or distort
//! design iteration.
use std::process::Command;
use std::time::Instant;

use crate::Result;

pub(crate) const SETS: &[(&str, &[&str])] =
    &[("workspace", &["craft-util", "craft-scribbles", "magic-engine", "magic-mcp", "magic-ui-egui", "magic-cli", "magic", "magic-tui", "xtask"])];

pub(crate) fn packages(set: &[&str]) -> Vec<String> {
    set.iter().flat_map(|name| ["-p".to_owned(), (*name).to_owned()]).collect()
}

fn check_sets() -> Result<()> {
    let md = crate::policy::metadata()?;
    let grouped: Vec<&str> = SETS.iter().flat_map(|(_, set)| set.iter().copied()).collect();
    let mut errors = Vec::new();
    for package in md["packages"].as_array().into_iter().flatten() {
        let name = package["name"].as_str().unwrap_or_default();
        match grouped.iter().filter(|g| **g == name).count() {
            1 => {}
            0 => errors.push(format!("{name}: in no build group (add it to xtask `SETS`)")),
            _ => errors.push(format!("{name}: in more than one build group")),
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors.join("\n")) }
}

fn report_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/ci-report.jsonl")
}

fn run(cmd: &mut Command) -> Result<()> {
    eprintln!("$ {cmd:?}");
    let output = cmd.output().map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    print!("{text}");
    let tail = text.lines().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
    let path = report_path();
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write as _;
        let _ = writeln!(file, "{}", serde_json::json!({"command": format!("{cmd:?}"), "tail": tail}));
    }
    Err(format!("failed ({}): {cmd:?}", output.status))
}

fn stage(name: &str, gate: impl FnOnce() -> Result<()>) -> Result<()> {
    let started = Instant::now();
    let outcome = gate();
    let mark = if outcome.is_ok() { "ok" } else { "FAILED" };
    eprintln!("== {name}: {mark} ({:.1}s)", started.elapsed().as_secs_f32());
    outcome
}

fn clippy() -> Result<()> {
    let cargo = env!("CARGO");
    for (_, set) in SETS {
        run(Command::new(cargo).arg("clippy").args(packages(set)).args(["--lib", "--bins", "--", "-D", "warnings"]))?;
    }
    Ok(())
}

fn tests() -> Result<()> {
    let cargo = env!("CARGO");
    run(Command::new(cargo).args(["test", "--workspace", "--no-run"]))
}

pub(crate) fn ci() -> Result<()> {
    let cargo = env!("CARGO");
    let path = report_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::File::create(path).map_err(|e| e.to_string())?;
    stage("build groups", check_sets)?;
    stage("fmt", || run(Command::new(cargo).args(["fmt", "--check"])))?;
    stage("layers", crate::policy::layers)?;
    stage("clippy", clippy)?;
    if std::env::var("MAGIC_RUN_TESTS").as_deref() == Ok("1") {
        stage("tests", tests)?;
    } else {
        eprintln!("== tests: deferred until visual UI design confirmation (set MAGIC_RUN_TESTS=1 to enable)");
    }
    Ok(())
}
