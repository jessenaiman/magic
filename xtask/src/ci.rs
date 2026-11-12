//! `cargo xtask ci`: fast-fail ArtCraft gates (`fmt` -> `layers` -> `assets` -> `clippy` -> `tests` with watchdog).

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::Result;

pub(crate) const SETS: &[(&str, &[&str])] =
    &[("workspace", &["magic-util", "magic-scribbles", "magic-engine", "magic-mcp", "magic-ui-egui", "magic-cli", "magic", "magic-tui", "xtask"])];

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

fn record_failure(command_desc: &str, tail: &str) {
    let path = report_path();
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write as _;
        let _ = writeln!(file, "{}", serde_json::json!({"command": command_desc, "tail": tail}));
    }
}

fn run_bounded(cmd: &mut Command, timeout: Duration) -> Result<()> {
    let desc = format!("{cmd:?}");
    eprintln!("$ {desc}");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    // TODO(MAGIC-REVIEW): KEEP concurrent pipe draining; waiting before reading deadlocks verbose checks.
    let stdout = child.stdout.take().ok_or("missing child stdout")?;
    let stderr = child.stderr.take().ok_or("missing child stderr")?;
    let drain = |mut pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let stdout = drain(Box::new(stdout));
    let stderr = drain(Box::new(stderr));
    let started = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                let stdout = stdout.join().map_err(|_| "stdout reader panicked")?.map_err(|e| e.to_string())?;
                let stderr = stderr.join().map_err(|_| "stderr reader panicked")?.map_err(|e| e.to_string())?;
                if status.success() {
                    return Ok(());
                }
                let text = format!("{}{}", String::from_utf8_lossy(&stdout), String::from_utf8_lossy(&stderr));
                print!("{text}");
                let tail = text.lines().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                record_failure(&desc, &tail);
                return Err(format!("failed ({}): {desc}", status));
            }
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let msg = format!("timed out after {}s: {desc}", timeout.as_secs());
                record_failure(&desc, &msg);
                return Err(msg);
            }
            None => thread::sleep(Duration::from_millis(100)),
        }
    }
}

fn stage(name: &str, gate: impl FnOnce() -> Result<()>) -> Result<()> {
    let started = Instant::now();
    let outcome = gate();
    let mark = if outcome.is_ok() { "ok" } else { "FAILED" };
    eprintln!("== {name}: {mark} ({:.1}s)", started.elapsed().as_secs_f32());
    outcome
}

fn test_timeout() -> Duration {
    let mins = std::env::var("MAGIC_TEST_TIMEOUT_MINUTES").ok().and_then(|v| v.trim().parse::<u64>().ok()).filter(|&m| m > 0).unwrap_or(20);
    Duration::from_secs(mins * 60)
}

fn clippy() -> Result<()> {
    let cargo = env!("CARGO");
    for (_, set) in SETS {
        let mut cmd = Command::new(cargo);
        cmd.arg("clippy").args(packages(set)).args(["--all-targets", "--", "-D", "warnings"]);
        run_bounded(&mut cmd, test_timeout())?;
    }
    Ok(())
}

pub(crate) fn tests() -> Result<()> {
    let cargo = env!("CARGO");
    let threads = std::env::var("RUST_TEST_THREADS").ok().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "4".to_owned());
    let mut cmd = Command::new(cargo);
    cmd.args(["test", "--workspace", "--", "--test-threads", &threads]);
    run_bounded(&mut cmd, test_timeout())
}

pub(crate) fn ci() -> Result<()> {
    let cargo = env!("CARGO");
    let path = report_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::File::create(path).map_err(|e| e.to_string())?;
    stage("build groups", check_sets)?;
    stage("fmt", || {
        let mut cmd = Command::new(cargo);
        cmd.args(["fmt", "--all", "--check"]);
        run_bounded(&mut cmd, Duration::from_secs(120))
    })?;
    stage("layers", crate::policy::layers)?;
    stage("assets", crate::policy::assets)?;
    stage("clippy", clippy)?;
    stage("tests", tests)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod pipe_tests {
    use super::*;
    #[test]
    fn verbose_child_does_not_fill_output_pipes() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "head -c 262144 /dev/zero; head -c 262144 /dev/zero >&2"]);
        assert!(run_bounded(&mut cmd, Duration::from_secs(5)).is_ok());
    }
}
