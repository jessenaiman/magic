//! `cargo xtask ci`: workspace quality gates (`fmt` -> `clippy`, with opt-in `MAGIC_RUN_TESTS=1`),
//! writing complete human- and agent-readable reports to `target/ci-report.txt` and `target/ci-report.jsonl`.

use std::io::{Read, Write as _};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::Result;

fn target_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target"))
}

fn jsonl_report_path() -> PathBuf {
    target_dir().join("ci-report.jsonl")
}

fn text_report_path() -> PathBuf {
    target_dir().join("ci-report.txt")
}

fn init_reports() -> Result<()> {
    let dir = target_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::File::create(jsonl_report_path()).map_err(|e| e.to_string())?;
    let mut txt = std::fs::File::create(text_report_path()).map_err(|e| e.to_string())?;
    writeln!(txt, "# Magic Studio `cargo xtask ci` Report").map_err(|e| e.to_string())?;
    Ok(())
}

fn append_stage_report(stage_name: &str, ok: bool, secs: f32, detail: &str) {
    if let Ok(mut jf) = std::fs::OpenOptions::new().create(true).append(true).open(jsonl_report_path()) {
        let _ = writeln!(jf, "{}", json!({"stage": stage_name, "ok": ok, "seconds": secs, "output": detail}));
    }
    if let Ok(mut tf) = std::fs::OpenOptions::new().create(true).append(true).open(text_report_path()) {
        let status = if ok { "OK" } else { "FAILED" };
        let _ = writeln!(tf, "\n== {stage_name}: {status} ({secs:.1}s) ==");
        if !detail.trim().is_empty() {
            let _ = writeln!(tf, "{}", detail.trim());
        }
    }
}

fn run_bounded(cmd: &mut Command, timeout: Duration) -> Result<String> {
    let desc = format!("{cmd:?}");
    eprintln!("$ {desc}");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
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
                let combined = format!("{}{}", String::from_utf8_lossy(&stdout), String::from_utf8_lossy(&stderr));
                if status.success() {
                    return Ok(combined);
                }
                print!("{combined}");
                return Err(format!("failed ({status}): {desc}\n{combined}"));
            }
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {}s: {desc}", timeout.as_secs()));
            }
            None => thread::sleep(Duration::from_millis(100)),
        }
    }
}

fn stage(name: &str, gate: impl FnOnce() -> Result<String>) -> Result<()> {
    let started = Instant::now();
    let outcome = gate();
    let secs = started.elapsed().as_secs_f32();
    match &outcome {
        Ok(detail) => {
            eprintln!("== {name}: ok ({secs:.1}s)");
            append_stage_report(name, true, secs, detail);
        }
        Err(err) => {
            eprintln!("== {name}: FAILED ({secs:.1}s)");
            append_stage_report(name, false, secs, err);
        }
    }
    outcome.map(|_| ())
}

fn test_timeout() -> Duration {
    let mins = std::env::var("MAGIC_TEST_TIMEOUT_MINUTES").ok().and_then(|v| v.trim().parse::<u64>().ok()).filter(|&m| m > 0).unwrap_or(20);
    Duration::from_secs(mins * 60)
}

fn clippy() -> Result<String> {
    let cargo = env!("CARGO");
    let mut cmd = Command::new(cargo);
    cmd.args(["clippy", "--workspace", "--all-targets", "--message-format=short", "--", "-D", "warnings"]);
    run_bounded(&mut cmd, test_timeout())
}

pub(crate) fn tests() -> Result<()> {
    let cargo = env!("CARGO");
    let threads = std::env::var("RUST_TEST_THREADS").ok().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "4".to_owned());
    let mut cmd = Command::new(cargo);
    cmd.args(["test", "--workspace", "--", "--test-threads", &threads]);
    run_bounded(&mut cmd, test_timeout()).map(|_| ())
}

pub(crate) fn ci() -> Result<()> {
    let cargo = env!("CARGO");
    init_reports()?;
    stage("fmt", || {
        let mut cmd = Command::new(cargo);
        cmd.args(["fmt", "--all", "--check"]);
        run_bounded(&mut cmd, Duration::from_secs(120))
    })?;
    stage("clippy", clippy)?;
    if std::env::var("MAGIC_RUN_TESTS").ok().as_deref() == Some("1") {
        stage("tests", || tests().map(|()| "workspace tests passed".to_owned()))?;
    } else {
        eprintln!("== tests: skipped (set MAGIC_RUN_TESTS=1 to include unit/integration tests)");
        append_stage_report("tests", true, 0.0, "skipped (MAGIC_RUN_TESTS unset)");
    }
    eprintln!("== report written to {}", text_report_path().display());
    Ok(())
}
