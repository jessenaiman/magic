//! Single-responsibility `<product>-cli` subprocess runner for headless and `--bridge` operations.
//!
//! Uses exact upstream `<product>-cli` syntax verified from `storytold/*` repositories:
//! - `filmcraft-cli`: `--project <p.fcproj> [--save] exec|run|probe|export`, `--bridge 127.0.0.1:<port>`
//! - `effectcraft-cli`: `exec <cmd> '<json>' [<p.ecproj>] [--save|--empty --save-as <out>] --json`,
//!   `--bridge <port> --json exec <cmd> '<json>'`,
//!   `run [<p.ecproj>] --commands - [--save|--empty --save-as <out>] --json`,
//!   `render-frame <p.ecproj|--template tpl.ectemplate> [--values v.json] --time <sec> --max-side <px> --out <f.png> --json`,
//!   `render <p.ecproj|--template tpl.ectemplate> [--values v.json] [--comp <comp>] --out <out> [--format ...] --json`
//! - `pdfcraft-cli`: `info`, `text`, `tools`, `ui --control ~/.pdfcraft-control.json`, `run --script <s.json> --root <dir> --report`
//! - `designcraft-cli` / `deckcraft-cli`: `script - --in <doc> [--save <doc>]`, `run --in <doc>`
//! - `photocraft-cli`: `run <doc|--new '{}'> [--cmd <id> --params '<json>'] [--out <doc> --in-place]`,
//!   `batch --actions <grade.json> --in <in_dir> --out <out_dir>`
//! - `vectorcraft-cli`: `run [--in <doc>] [--commands -] [--export <doc>]`
//! - `anicraft-cli` (headless only; pre-alpha): `info`, `commands`, `exec`, `serve`, `render`,
//!   `render-seq`, `gif`, `mcp` (<https://github.com/storytold/AniCraft>)
//! - `texturecraft-cli`: `run [<doc>|builtin:material_ball] <cmd> '<json>' [-o <doc>]`

use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use super::discovery::{require_executable, resolve_executable_path, scrubbed_command};
use super::target::CraftTarget;
use crate::time::{TICKS_PER_SECOND, Tick};
use crate::{EngineError, Result};

pub(crate) struct ScopedTempDir(PathBuf);

impl ScopedTempDir {
    pub(crate) fn new(prefix: &str) -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("{prefix}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScopedTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cli_command(target: CraftTarget, port: Option<u16>) -> Result<Command> {
    let bin = require_executable(target.cli_name())?;
    Ok(scrubbed_command(&bin, target, port))
}

fn parse_cli_value(target: CraftTarget, command: &str, out: &Output) -> Result<Value> {
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(EngineError::CommandFailed(command.to_string(), format!("{}: {detail}", target.cli_name())));
    }
    let text = std::str::from_utf8(&out.stdout).map_err(|e| EngineError::InvalidParams(format!("invalid UTF-8 from {}: {e}", target.cli_name())))?.trim();
    if text.is_empty() || text == "null" {
        return Ok(Value::Null);
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(text) {
        if let Some(err) = parsed["error"].as_str().filter(|_| parsed["ok"] == json!(false)) {
            return Err(EngineError::CommandFailed(command.to_string(), err.to_string()));
        }
        return Ok(parsed.get("result").cloned().unwrap_or(parsed));
    }
    if let Some(parsed) = text.lines().rev().find_map(|line| serde_json::from_str::<Value>(line.trim()).ok()) {
        return Ok(parsed.get("result").cloned().unwrap_or(parsed));
    }
    Ok(json!({"raw": text}))
}

pub fn exec_film_bridge(addr: SocketAddr, command: &str, params: &Value) -> Result<Value> {
    let _slot = crate::workspace::acquire_tool_slot();
    let params_str = serde_json::to_string(params).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    let mut cmd = cli_command(CraftTarget::Film, Some(addr.port()))?;
    cmd.arg("--bridge").arg(addr.to_string()).arg("exec").arg(command).arg(&params_str);
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("filmcraft-cli".into(), e.to_string()))?;
    parse_cli_value(CraftTarget::Film, command, &out)
}

pub fn exec_effect_bridge(addr: SocketAddr, command: &str, params: &Value) -> Result<Value> {
    let _slot = crate::workspace::acquire_tool_slot();
    let params_str = serde_json::to_string(params).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    let mut cmd = cli_command(CraftTarget::Effect, Some(addr.port()))?;
    cmd.arg("--bridge").arg(addr.port().to_string()).arg("--json").arg("exec").arg(command).arg(&params_str);
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("effectcraft-cli".into(), e.to_string()))?;
    parse_cli_value(CraftTarget::Effect, command, &out)
}

pub fn exec_single(target: CraftTarget, doc: Option<&Path>, command: &str, params: &Value, save: bool) -> Result<Value> {
    let _slot = crate::workspace::acquire_tool_slot();
    let _lock = doc.filter(|_| save).map(crate::project_lock::for_project);
    let params_str = serde_json::to_string(params).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    let mut cmd = cli_command(target, None)?;

    match target {
        CraftTarget::Film => {
            if let Some(path) = doc.filter(|p| p.is_file()) {
                cmd.arg("--project").arg(path);
                if save {
                    cmd.arg("--save");
                }
            }
            cmd.arg("exec").arg(command).arg(&params_str);
        }
        CraftTarget::Effect => {
            cmd.arg("exec").arg(command).arg(&params_str);
            if let Some(path) = doc.filter(|p| p.is_file()) {
                cmd.arg(path);
                if save {
                    cmd.arg("--save");
                }
            } else {
                cmd.arg("--empty");
                if save && let Some(path) = doc {
                    cmd.arg("--save-as").arg(path);
                }
            }
            cmd.arg("--json");
        }
        CraftTarget::Vector => {
            let steps = [(command, params.clone())];
            let out_doc = if save { doc } else { None };
            drop(_slot);
            let mut list = exec_batch(CraftTarget::Vector, doc, out_doc, &steps)?;
            return Ok(list.pop().unwrap_or_else(|| json!({"ok": true})));
        }
        CraftTarget::Photo => {
            if command == "info"
                && let Some(path) = doc
            {
                cmd.arg("info").arg(path);
            } else {
                cmd.arg("run");
                if let Some(path) = doc.filter(|p| craft_util::media::is_craft_doc_path(p)) {
                    cmd.arg(path);
                    if save {
                        cmd.arg("--out").arg(path).arg("--in-place");
                    }
                } else {
                    cmd.arg("--new").arg("{}");
                    if save && let Some(path) = doc {
                        cmd.arg("--out").arg(path);
                    }
                }
                cmd.arg("--cmd").arg(command).arg("--params").arg(&params_str);
            }
        }
        CraftTarget::Pdf => {
            if command == "info"
                && let Some(path) = doc
            {
                cmd.arg("info").arg(path);
            } else if command == "text"
                && let Some(path) = doc
            {
                cmd.arg("text").arg(path);
                if let Some(pg) = params["page"].as_u64() {
                    cmd.arg("--page").arg(pg.to_string());
                }
            } else if command == "tools" {
                cmd.arg("tools");
            } else if let Some(ui_sub) = command.strip_prefix("ui.") {
                let ctrl_file = craft_util::discovery::pdfcraft_control_file()
                    .ok_or_else(|| EngineError::InvalidParams("HOME required for ~/.pdfcraft-control.json".into()))?;
                cmd.arg("ui").arg("--control").arg(ctrl_file).arg(ui_sub);
                if let Some(obj) = params.as_object() {
                    for (k, v) in obj {
                        let val_str = v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string());
                        cmd.arg(format!("{k}={val_str}"));
                    }
                }
            } else {
                let steps = [(command, params.clone())];
                let out_doc = if save { doc } else { None };
                drop(_slot);
                let mut list = exec_batch(CraftTarget::Pdf, doc, out_doc, &steps)?;
                return Ok(list.pop().unwrap_or_else(|| json!({"ok": true})));
            }
        }
        CraftTarget::Design | CraftTarget::Deck => {
            if save && let Some(path) = doc {
                cmd.arg("script").arg("--in").arg(path).arg("--save").arg(path);
                cmd.arg("--cmd").arg(format!("{command}={params_str}"));
            } else {
                cmd.arg("run");
                if let Some(path) = doc {
                    cmd.arg("--in").arg(path);
                }
                cmd.arg("--cmd").arg(format!("{command}={params_str}"));
            }
        }
        CraftTarget::Texture => {
            if command == "info" {
                let input = doc.and_then(Path::to_str).or_else(|| params["path"].as_str()).unwrap_or("builtin:material_ball");
                cmd.arg("info").arg(input);
            } else {
                cmd.arg("run");
                if let Some(path) = doc.filter(|p| p.is_file()) {
                    cmd.arg(path);
                } else {
                    cmd.arg("builtin:material_ball");
                }
                cmd.arg(command).arg(&params_str);
                if save && let Some(path) = doc {
                    cmd.arg("-o").arg(path);
                }
            }
        }
        CraftTarget::Ani => {
            if command == "info"
                && let Some(path) = doc
            {
                cmd.arg("info").arg(path);
            } else if command == "commands" {
                cmd.arg("commands");
            } else {
                let steps = [(command, params.clone())];
                let out_doc = if save { doc } else { None };
                drop(_slot);
                let mut list = exec_batch(CraftTarget::Ani, doc, out_doc, &steps)?;
                return Ok(list.pop().unwrap_or_else(|| json!({"ok": true})));
            }
        }
    }

    let out = cmd.output().map_err(|e| EngineError::ExternalTool(target.cli_name().to_string(), e.to_string()))?;
    parse_cli_value(target, command, &out)
}

pub fn exec_batch(target: CraftTarget, input: Option<&Path>, output: Option<&Path>, steps: &[(&str, Value)]) -> Result<Vec<Value>> {
    if steps.is_empty() {
        return Ok(Vec::new());
    }
    let _slot = crate::workspace::acquire_tool_slot();
    let _lock = output.or(input).map(crate::project_lock::for_project);
    if let Some(out_parent) = output.and_then(Path::parent) {
        std::fs::create_dir_all(out_parent)?;
    }

    let mut cmd = cli_command(target, None)?;
    let mut stdin_bytes: Option<Vec<u8>> = None;

    match target {
        CraftTarget::Film => {
            let existing = input.filter(|p| p.is_file());
            if let Some(proj) = existing {
                cmd.arg("--project").arg(proj);
                if output.is_some_and(|out| out == proj) {
                    cmd.arg("--save");
                }
            }
            cmd.arg("run").arg("-");
            let mut lines = Vec::new();
            for (id, params) in steps {
                if *id == "file.save" && params.as_object().is_some_and(serde_json::Map::is_empty) {
                    if let Some(dest) = output.or(input) {
                        lines.push(json!({"id": "file.save", "params": {"path": dest}}).to_string());
                    }
                } else {
                    lines.push(json!({"id": id, "params": params}).to_string());
                }
            }
            if let Some(dest) = output
                && existing.is_none_or(|proj| proj != dest)
            {
                lines.push(json!({"id": "file.save", "params": {"path": dest}}).to_string());
            }
            stdin_bytes = Some(lines.join("\n").into_bytes());
        }
        CraftTarget::Effect => {
            cmd.arg("run");
            if let Some(proj) = input.filter(|p| p.is_file()) {
                cmd.arg(proj);
                if let Some(dest) = output {
                    if dest == proj {
                        cmd.arg("--save");
                    } else {
                        cmd.arg("--save-as").arg(dest);
                    }
                }
            } else {
                cmd.arg("--empty");
                if let Some(dest) = output {
                    cmd.arg("--save-as").arg(dest);
                }
            }
            cmd.arg("--commands").arg("-").arg("--json");
            let payload: Vec<Value> = steps.iter().map(|(id, params)| json!({"command": id, "params": params})).collect();
            stdin_bytes = Some(serde_json::to_vec(&payload).map_err(|e| EngineError::InvalidParams(e.to_string()))?);
        }
        CraftTarget::Pdf => {
            let tmp = ScopedTempDir::new("magic-pdfcraft")?;
            let script_path = tmp.path().join("steps.json");
            let root_dir = input.or(output).and_then(Path::parent).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
            let mut script_steps: Vec<Value> = Vec::with_capacity(steps.len() + 2);
            if let Some(src) = input.filter(|p| p.is_file()) {
                script_steps.push(json!({"tool": "doc_open", "params": {"path": src}}));
            }
            for (id, params) in steps {
                script_steps.push(json!({"tool": id, "params": params}));
            }
            if let Some(dest) = output {
                script_steps.push(json!({"tool": "doc_save", "params": {"path": dest}}));
            }
            let bytes = serde_json::to_vec_pretty(&script_steps).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
            craft_atomic::atomic_write(&script_path, &bytes)?;
            cmd.arg("run").arg("--script").arg(&script_path).arg("--root").arg(root_dir).arg("--report");
            let out = cmd.output().map_err(|e| EngineError::ExternalTool("pdfcraft-cli".into(), e.to_string()))?;
            let val = parse_cli_value(CraftTarget::Pdf, "run", &out)?;
            if let Some(arr) = val.get("steps").or_else(|| val.get("results")).and_then(Value::as_array) {
                return Ok(arr.clone());
            }
            return Ok(vec![val]);
        }
        CraftTarget::Design | CraftTarget::Deck => {
            let src = input.ok_or_else(|| EngineError::InvalidParams(format!("{} batch requires an input document", target.cli_name())))?;
            cmd.arg("script").arg("-").arg("--in").arg(src);
            if let Some(dest) = output {
                cmd.arg("--save").arg(dest);
            }
            let payload: Vec<Value> = steps.iter().map(|(id, params)| json!({"command": id, "params": params})).collect();
            stdin_bytes = Some(serde_json::to_vec(&payload).map_err(|e| EngineError::InvalidParams(e.to_string()))?);
        }
        CraftTarget::Photo => {
            cmd.arg("run");
            if let Some(src) = input.filter(|p| craft_util::media::is_craft_doc_path(p)) {
                cmd.arg(src);
            } else {
                cmd.arg("--new").arg("{}");
            }
            for (id, params) in steps {
                cmd.arg("--cmd").arg(id).arg("--params").arg(params.to_string());
            }
            if let Some(dest) = output {
                cmd.arg("--out").arg(dest);
                if input.is_some_and(|src| src == dest) {
                    cmd.arg("--in-place");
                }
            }
        }
        CraftTarget::Vector => {
            cmd.arg("run");
            if let Some(src) = input.filter(|p| p.is_file()) {
                cmd.arg("--in").arg(src);
            }
            cmd.arg("--commands").arg("-");
            if let Some(dest) = output {
                cmd.arg("--export").arg(dest);
            }
            let payload: Vec<Value> = steps.iter().map(|(id, params)| json!({"command": id, "params": params})).collect();
            stdin_bytes = Some(serde_json::to_vec(&payload).map_err(|e| EngineError::InvalidParams(e.to_string()))?);
        }
        CraftTarget::Ani => {
            cmd.arg("serve");
            if let Some(src) = input.filter(|p| p.is_file()) {
                cmd.arg(src);
            }
            let mut lines = Vec::with_capacity(steps.len() + 2);
            for (idx, (id, params)) in steps.iter().enumerate() {
                lines.push(json!({"id": idx + 1, "cmd": id, "params": params}).to_string());
            }
            if let Some(dest) = output {
                lines.push(json!({"id": steps.len() + 1, "cmd": "file.save", "params": {"path": dest}}).to_string());
            }
            lines.push(json!({"cmd": "quit"}).to_string());
            stdin_bytes = Some(lines.join("\n").into_bytes());
        }
        CraftTarget::Texture => {
            drop(_slot);
            let mut results = Vec::with_capacity(steps.len());
            let last_idx = steps.len().saturating_sub(1);
            for (idx, (id, params)) in steps.iter().enumerate() {
                let save_step = output.is_some() && idx == last_idx;
                let doc_arg = if idx == 0 { input } else { output.or(input) };
                results.push(exec_single(target, doc_arg, id, params, save_step)?);
            }
            return Ok(results);
        }
    }

    run_piped_batch(target, cmd, stdin_bytes.as_deref())
}

/// Flatten any PhotoCraft-supported layered or HDR document (`.pcraft`, `.psd`, `.psb`, `.ora`,
/// `.xcf`, `.kra`, `.pdn`, `.afphoto`, `.exr`, `.hdr`, `.fits`, `.heic`) to a flat PNG via
/// `photocraft-cli run <doc> --out <out.png>`.
pub fn export_photo_flat(doc: &Path, out_png: &Path) -> Result<()> {
    let _slot = crate::workspace::acquire_tool_slot();
    if let Some(parent) = out_png.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut cmd = cli_command(CraftTarget::Photo, None)?;
    cmd.arg("run").arg(doc).arg("--out").arg(out_png);
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("photocraft-cli".into(), e.to_string()))?;
    if !out.status.success() {
        return Err(EngineError::ExternalTool("photocraft-cli".into(), String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(())
}

/// Apply a PhotoCraft action list across an entire folder of images via native
/// `photocraft-cli batch --actions <grade.json> --in <in_dir> --out <out_dir>`.
pub fn run_photo_batch_actions(actions_file: &Path, in_dir: &Path, out_dir: &Path) -> Result<Value> {
    let _slot = crate::workspace::acquire_tool_slot();
    std::fs::create_dir_all(out_dir)?;
    let mut cmd = cli_command(CraftTarget::Photo, None)?;
    cmd.arg("batch").arg("--actions").arg(actions_file).arg("--in").arg(in_dir).arg("--out").arg(out_dir);
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("photocraft-cli".into(), e.to_string()))?;
    parse_cli_value(CraftTarget::Photo, "batch", &out)
}

/// Render/export any VectorCraft-supported vector or Affinity document (`.vectorcraft`, `.ai`,
/// `.eps`, `.svg`, `.svgz`, `.dxf`, `.emf`, `.wmf`, `.af`, `.afdesign`, `.afpub`) via native
/// `vectorcraft-cli run --in <doc> --export <out>`.
pub fn export_vector_flat(doc: &Path, out_file: &Path) -> Result<()> {
    let _slot = crate::workspace::acquire_tool_slot();
    if let Some(parent) = out_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut cmd = cli_command(CraftTarget::Vector, None)?;
    cmd.arg("run").arg("--in").arg(doc).arg("--export").arg(out_file);
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("vectorcraft-cli".into(), e.to_string()))?;
    if !out.status.success() {
        return Err(EngineError::ExternalTool("vectorcraft-cli".into(), String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(())
}

fn run_piped_batch(target: CraftTarget, mut cmd: Command, stdin_payload: Option<&[u8]>) -> Result<Vec<Value>> {
    cmd.stdin(if stdin_payload.is_some() { Stdio::piped() } else { Stdio::null() });
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let tool = |e: std::io::Error| EngineError::ExternalTool(target.cli_name().into(), e.to_string());
    let mut child = cmd.spawn().map_err(tool)?;
    if let (Some(bytes), Some(mut stdin)) = (stdin_payload, child.stdin.take()) {
        stdin.write_all(bytes).map_err(tool)?;
        let _ = stdin.write_all(b"\n");
    }
    let out = child.wait_with_output().map_err(tool)?;
    parse_batch_output(target, &out)
}

fn parse_batch_output(target: CraftTarget, out: &Output) -> Result<Vec<Value>> {
    let report = serde_json::from_slice::<Value>(&out.stdout).ok();
    if let Some(err) = report.as_ref().and_then(|r| r["failed"]["error"].as_str().or_else(|| r["error"].as_str())) {
        return Err(EngineError::CommandFailed(target.cli_name().into(), err.to_string()));
    }
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(EngineError::ExternalTool(target.cli_name().into(), detail));
    }
    if let Some(report) = report {
        if let Some(list) = report.as_array().or_else(|| report["result"].as_array()).or_else(|| report["results"].as_array()) {
            return Ok(list.iter().map(|entry| entry.get("result").cloned().unwrap_or_else(|| entry.clone())).collect());
        }
        return Ok(vec![report.get("result").cloned().unwrap_or(report)]);
    }
    let mut lines = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            if v["ok"] == json!(false)
                && let Some(err) = v["error"].as_str()
            {
                return Err(EngineError::CommandFailed(target.cli_name().into(), err.to_string()));
            }
            lines.push(v.get("result").cloned().unwrap_or(v));
        }
    }
    Ok(lines)
}

pub fn render_pdf_pages(doc: &Path, page: Option<usize>, scale: f64, destination: &Path) -> Result<()> {
    let step = match page {
        Some(index) => ("page_render", json!({"page": index + 1, "scale": scale, "out": destination})),
        None => ("page_render", json!({"scale": scale, "out_dir": destination})),
    };
    let _ = exec_batch(CraftTarget::Pdf, Some(doc), None, &[step])?;
    Ok(())
}

pub fn render_design_pages(doc: &Path, page: Option<usize>, scale: f64, destination: &Path) -> Result<()> {
    let _slot = crate::workspace::acquire_tool_slot();
    let mut cmd = cli_command(CraftTarget::Design, None)?;
    cmd.arg("run").arg("--in").arg(doc).arg("--scale").arg(scale.to_string());
    if let Some(index) = page {
        cmd.arg("--page").arg(index.to_string()).arg("--export").arg(destination);
    } else {
        cmd.arg("--all-pages").arg(destination);
    }
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("designcraft-cli".into(), e.to_string()))?;
    if !out.status.success() {
        return Err(EngineError::ExternalTool("designcraft-cli".into(), String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(())
}

/// Render a single EffectCraft composition frame via native `effectcraft-cli render-frame`.
pub fn render_effect_frame(project: &Path, comp: Option<&str>, seconds: f64, max_side: u32, output: &Path) -> Result<()> {
    let _slot = crate::workspace::acquire_tool_slot();
    let mut cmd = cli_command(CraftTarget::Effect, None)?;
    cmd.arg("render-frame")
        .arg(project)
        .arg("--time")
        .arg(seconds.max(0.0).to_string())
        .arg("--max-side")
        .arg(max_side.clamp(64, 4096).to_string())
        .arg("--out")
        .arg(output)
        .arg("--json");
    if let Some(c) = comp.filter(|s| !s.is_empty()) {
        cmd.arg("--comp").arg(c);
    }
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("effectcraft-cli".into(), e.to_string()))?;
    if !out.status.success() {
        return Err(EngineError::ExternalTool("effectcraft-cli".into(), String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(())
}

/// Render an EffectCraft composition clip or sequence via native `effectcraft-cli render`.
pub fn render_effect_clip(project: &Path, params: &Value) -> Result<Value> {
    let _slot = crate::workspace::acquire_tool_slot();
    let output_str = params["output"].as_str().ok_or_else(|| EngineError::InvalidParams("output path is required".into()))?;
    let mut cmd = cli_command(CraftTarget::Effect, None)?;
    cmd.arg("render").arg(project).arg("--out").arg(output_str).arg("--json");
    if let Some(comp) = params["comp"].as_str().filter(|s| !s.is_empty()) {
        cmd.arg("--comp").arg(comp);
    }
    if let Some(fmt) = params["format"].as_str().filter(|s| !s.is_empty()) {
        cmd.arg("--format").arg(fmt);
    }
    if let Some(st) = params["start"].as_f64() {
        cmd.arg("--start").arg(st.to_string());
    }
    if let Some(en) = params["end"].as_f64() {
        cmd.arg("--end").arg(en.to_string());
    }
    if let Some(fps) = params["frameRate"].as_u64() {
        cmd.arg("--fps").arg(fps.to_string());
    }
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("effectcraft-cli".into(), e.to_string()))?;
    parse_cli_value(CraftTarget::Effect, "render", &out)
}

/// Render an `.ectemplate` directly via `effectcraft-cli render|render-frame --template <tpl> [--values <v.json>] --out <out> --json` (`T06`).
pub fn render_effect_template(template: &Path, values: Option<&Value>, output: &Path, frame_time: Option<f64>) -> Result<Value> {
    let _slot = crate::workspace::acquire_tool_slot();
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = ScopedTempDir::new("magic-ectpl")?;
    let mut cmd = cli_command(CraftTarget::Effect, None)?;
    let subcmd = if frame_time.is_some() { "render-frame" } else { "render" };
    cmd.arg(subcmd).arg("--template").arg(template);
    if let Some(vals) = values.filter(|v| v.is_object() && !v.as_object().is_none_or(serde_json::Map::is_empty)) {
        let vals_path = tmp.path().join("values.json");
        let raw = serde_json::to_vec_pretty(vals).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
        craft_atomic::atomic_write(&vals_path, &raw)?;
        cmd.arg("--values").arg(&vals_path);
    }
    if let Some(sec) = frame_time {
        cmd.arg("--time").arg(sec.max(0.0).to_string());
    }
    cmd.arg("--out").arg(output).arg("--json");
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("effectcraft-cli".into(), e.to_string()))?;
    parse_cli_value(CraftTarget::Effect, subcmd, &out)
}

pub fn render_film_frame(project: &Path, tick: Tick, scale: f32, output: &Path) -> Result<()> {
    let params = json!({
        "path": output,
        "format": "png",
        "target": "program",
        "time": tick.0,
        "scale": scale,
    });
    let _ = exec_single(CraftTarget::Film, Some(project), "file.exportFrame", &params, false)?;
    Ok(())
}

/// Render a single frame of an `.anicraft` document via headless `anicraft-cli render`.
pub fn render_anicraft_frame(doc: &Path, frame: u32, output: &Path) -> Result<()> {
    let _slot = crate::workspace::acquire_tool_slot();
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut cmd = cli_command(CraftTarget::Ani, None)?;
    cmd.arg("render").arg(doc).arg("--frame").arg(frame.to_string()).arg("--out").arg(output);
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("anicraft-cli".into(), e.to_string()))?;
    if !out.status.success() {
        return Err(EngineError::ExternalTool("anicraft-cli".into(), String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(())
}

/// Render a PNG sequence from an `.anicraft` document via headless `anicraft-cli render-seq`.
pub fn render_anicraft_sequence(doc: &Path, start: u32, end: Option<u32>, out_dir: &Path) -> Result<Vec<PathBuf>> {
    let _slot = crate::workspace::acquire_tool_slot();
    std::fs::create_dir_all(out_dir)?;
    let mut cmd = cli_command(CraftTarget::Ani, None)?;
    cmd.arg("render-seq").arg(doc).arg("--out-dir").arg(out_dir).arg("--start").arg(start.to_string());
    if let Some(end_frame) = end {
        cmd.arg("--end").arg(end_frame.to_string());
    }
    let out = cmd.output().map_err(|e| EngineError::ExternalTool("anicraft-cli".into(), e.to_string()))?;
    if !out.status.success() {
        return Err(EngineError::ExternalTool("anicraft-cli".into(), String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    let mut pngs: Vec<PathBuf> =
        std::fs::read_dir(out_dir)?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"))).collect();
    pngs.sort();
    Ok(pngs)
}

/// Probe media duration in `Tick`s: zero-subprocess `craft-audio` fast path for audio files, else
/// `filmcraft-cli probe <file>`, else `ffprobe`.
pub fn probe_media(filename: &Path) -> Result<Value> {
    if !filename.is_file() {
        return Err(EngineError::InvalidParams(format!("cannot read {}: file does not exist", filename.display())));
    }
    if craft_util::media::is_pure_audio_file(filename)
        && let Ok(audio_info) = craft_util::media::probe_audio_in_process(filename)
    {
        return Ok(audio_info);
    }
    if craft_util::media::is_isobmff_file(filename)
        && let Ok(container_info) = craft_util::media::probe_container_in_process(filename)
    {
        return Ok(container_info);
    }
    if let Ok(bin) = require_executable(CraftTarget::Film.cli_name()) {
        let _slot = crate::workspace::acquire_tool_slot();
        let mut cmd = scrubbed_command(&bin, CraftTarget::Film, None);
        cmd.arg("probe").arg(filename);
        if let Ok(out) = cmd.output()
            && out.status.success()
            && let Ok(raw) = serde_json::from_slice::<Value>(&out.stdout)
        {
            return Ok(normalize_probe_json(raw));
        }
    }
    if let Some(ffprobe_info) = probe_with_ffprobe(filename) {
        return Ok(ffprobe_info);
    }
    craft_util::media::probe_audio_in_process(filename).map_err(|e| EngineError::InvalidParams(e.to_string()))
}

fn normalize_probe_json(mut info: Value) -> Value {
    let body = info.get("result").cloned().unwrap_or_else(|| info.clone());
    if body != info {
        info = body;
    }
    let has_tick = info["duration"].as_i64().is_some() || info.pointer("/duration/ticks").and_then(Value::as_i64).is_some();
    if !has_tick {
        let secs = info["duration_seconds"].as_f64().or_else(|| info["durationSeconds"].as_f64()).or_else(|| info["duration"].as_f64());
        if let Some(s) = secs {
            info["duration"] = json!(Tick::from_seconds_f64(s).0);
        }
    }
    if info.get("audio_streams").is_none()
        && let Some(audio) = info.get("audioStreams").or_else(|| info.get("audio")).cloned()
    {
        info["audio_streams"] = if audio.is_array() { audio } else { json!([audio]) };
    }
    if info.get("kind").is_none() {
        let is_video = info.get("video").is_some_and(|v| !v.is_null());
        info["kind"] = json!(if is_video { "Movie" } else { "Audio" });
    }
    info
}

fn probe_with_ffprobe(filename: &Path) -> Option<Value> {
    let bin = resolve_executable_path("ffprobe")?;
    let mut cmd = Command::new(bin);
    craft_util::discovery::scrub_credentials(&mut cmd);
    let out = cmd.args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams"]).arg(filename).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let parsed: Value = serde_json::from_slice(&out.stdout).ok()?;
    let streams = parsed["streams"].as_array()?;
    let video = streams.iter().find(|s| s["codec_type"].as_str() == Some("video")).cloned();
    let audio_streams: Vec<Value> = streams.iter().filter(|s| s["codec_type"].as_str() == Some("audio")).cloned().collect();
    let secs = parsed
        .pointer("/format/duration")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| streams.iter().find_map(|s| s["duration"].as_str()?.parse::<f64>().ok()))?;
    let ticks = (secs * (TICKS_PER_SECOND as f64)).round() as i64;
    Some(json!({
        "kind": if video.is_some() { "Movie" } else { "Audio" },
        "duration": ticks,
        "video": video,
        "audio_streams": audio_streams,
    }))
}
