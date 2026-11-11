//! Single-responsibility launcher and window handoff for system-installed ArtCraft desktop apps.
//!
//! Opens or reuses `<product> --control <port>` the exact same way a designer works alongside
//! `<product>-cli --bridge` / `--connect`:
//! - Never spawns embedded `eframe` windows inside Magic's process.
//! - Enforces `CraftTarget::allows_live_ui`: early-alpha `AniCraft` (`storytold/AniCraft`) is
//!   strictly headless CLI/tools-only (`anicraft-cli`), while all live Studio UI views and
//!   interactive windows run in `EffectCraft` and `FilmCraft` (plus `DesignCraft`, `PhotoCraft`,
//!   and `VectorCraft`).
//! - Tracks windows spawned by Magic so `workspace.close_opened` (`Close apps opened by Magic`)
//!   cleanly closes only non-dirty sibling windows Magic launched.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Value, json};

use super::control;
use super::discovery::{control_addr, ensure_control_token, require_executable, same_canonical_path, scrubbed_command};
use super::target::CraftTarget;
use crate::{EngineError, Result};

fn opened_apps_registry() -> &'static Mutex<BTreeMap<&'static str, u16>> {
    static OPENED: OnceLock<Mutex<BTreeMap<&'static str, u16>>> = OnceLock::new();
    OPENED.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn record_opened_app(target: CraftTarget, port: u16) {
    if let Ok(mut map) = opened_apps_registry().lock() {
        map.insert(target.name(), port);
    }
}

/// Gracefully close sibling ArtCraft windows spawned by Magic during this session unless dirty.
pub fn close_opened_apps() -> Result<Value> {
    let entries: Vec<(&'static str, u16)> = opened_apps_registry().lock().map_or_else(|_| Vec::new(), |map| map.iter().map(|(&k, &v)| (k, v)).collect());
    let mut closed = Vec::new();
    let mut skipped_dirty = Vec::new();

    for (name, port) in entries {
        let Some(target) = CraftTarget::from_name(name) else {
            continue;
        };
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        if !control::is_listening(addr) {
            if let Ok(mut map) = opened_apps_registry().lock() {
                map.remove(name);
            }
            continue;
        }
        let (inspect_method, inspect_params) = target.live_inspect_envelope();
        if let Ok(reply) = control::live_control(target, Some(port), &inspect_method, &inspect_params) {
            let body = reply.get("result").cloned().unwrap_or(reply);
            if control::is_doc_dirty(target, &body) {
                skipped_dirty.push(name);
                continue;
            }
        }
        if target == CraftTarget::Pdf {
            if let Some(ctrl_file) = craft_util::discovery::pdfcraft_control_file().filter(|p| p.is_file())
                && let Ok(cli_bin) = require_executable(target.cli_name())
            {
                let _ = scrubbed_command(&cli_bin, target, None).arg("ui").arg("--control").arg(ctrl_file).arg("quit").output();
            }
        } else {
            let _ = control::live_control(target, Some(port), "app.quit", &json!({"force": false}));
        }
        if let Ok(mut map) = opened_apps_registry().lock() {
            map.remove(name);
        }
        closed.push(name);
    }
    Ok(json!({
        "ok": true,
        "closed": closed,
        "skipped_dirty": skipped_dirty,
    }))
}

fn automation_root_for(file: Option<&Path>, workspace_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = workspace_dir.filter(|d| d.is_dir()) {
        return Some(dir.to_path_buf());
    }
    if let Some(doc) = file {
        for ancestor in doc.ancestors().skip(1) {
            if ancestor.join("project.magic").is_file() || ancestor.join("Cargo.toml").is_file() {
                return Some(ancestor.to_path_buf());
            }
        }
        if let Some(parent) = doc.parent().filter(|p| !p.as_os_str().is_empty()) {
            return Some(parent.to_path_buf());
        }
    }
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Launch `<product> --control <port>` detached if nothing is listening yet, or reuse the live
/// desktop window and open `file` (when given) safely.
pub fn open_or_focus(target: CraftTarget, file: Option<&Path>, port_override: Option<u16>, workspace_dir: Option<&Path>) -> Result<Value> {
    if !target.allows_live_ui() {
        return Err(EngineError::InvalidParams(format!(
            "{} desktop UI is disabled for safety while AniCraft is in early alpha; use `{}` headless commands (`anicraft.exec` / `character.sprite`) and inspect/composite inside EffectCraft or FilmCraft",
            target.name(),
            target.cli_name()
        )));
    }
    let addr = match port_override {
        Some(port) => SocketAddr::from(([127, 0, 0, 1], port)),
        None => control_addr(target)?,
    };
    let port = addr.port();
    let mut receipt = json!({
        "ok": true,
        "app": target.name(),
        "path": file,
        "file": file,
        "control": port,
        "inspect": target.cli_control_hint(port),
        "cli": target.cli_control_hint(port),
        "mcp": target.mcp_bridge_hint(port),
    });

    if control::is_listening(addr) {
        receipt["reused"] = json!(true);
        receipt["window"] = json!("focused");
        if let Some(doc) = file {
            reuse_live_window(target, port, doc, &mut receipt)?;
        } else {
            let _ = control::live_control(target, Some(port), "ui.focus", &json!({}));
        }
        return Ok(receipt);
    }

    spawn_controlled_app(target, port, file, workspace_dir)?;
    record_opened_app(target, port);
    let ready = wait_for_listener(addr, Duration::from_secs(6));
    receipt["reused"] = json!(false);
    receipt["listening"] = json!(ready);
    receipt["window"] = json!("opened");
    Ok(receipt)
}

fn spawn_controlled_app(target: CraftTarget, port: u16, file: Option<&Path>, workspace_dir: Option<&Path>) -> Result<()> {
    let app_bin = require_executable(target.name())?;
    let (token, token_file) = ensure_control_token(target, port)?;
    let mut cmd = scrubbed_command(&app_bin, target, Some(port));
    if let Some(tok) = token
        && target == CraftTarget::Film
    {
        cmd.env(target.token_env(), tok);
    }
    if target == CraftTarget::Pdf {
        let ctrl_file = craft_util::discovery::pdfcraft_control_file()
            .ok_or_else(|| EngineError::InvalidParams("HOME directory required for ~/.pdfcraft-control.json".into()))?;
        cmd.arg("--control").arg(ctrl_file).arg("--two-paged");
    } else {
        cmd.arg("--control").arg(port.to_string());
    }
    if target.uses_token_file_flag()
        && let Some(ref tf) = token_file
    {
        cmd.arg("--control-token-file").arg(tf);
    }
    if target.uses_automation_roots()
        && let Some(root) = automation_root_for(file, workspace_dir)
    {
        cmd.arg("--automation-read-root").arg(&root).arg("--automation-write-root").arg(&root);
    }
    if let Some(doc) = file {
        cmd.arg(doc);
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| EngineError::ExternalTool(target.name().to_owned(), e.to_string()))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn reuse_live_window(target: CraftTarget, port: u16, doc: &Path, receipt: &mut Value) -> Result<()> {
    let (inspect_method, inspect_params) = target.live_inspect_envelope();
    let info = match control::live_control(target, Some(port), &inspect_method, &inspect_params) {
        Ok(reply) => Some(reply.get("result").cloned().unwrap_or(reply)),
        Err(EngineError::CommandFailed(_, _)) => None,
        Err(err) => return Err(err),
    };
    let open_path = info.as_ref().and_then(|v| control::extract_doc_path(target, v));
    if open_path.as_deref().is_some_and(|prev| same_canonical_path(prev, doc)) {
        receipt["already_open"] = json!(true);
        let _ = control::live_control(target, Some(port), "ui.focus", &json!({}));
        return Ok(());
    }
    if info.as_ref().is_some_and(|v| control::is_doc_dirty(target, v)) {
        let shown = open_path.as_ref().map_or_else(|| "an unnamed document".to_owned(), |p| p.display().to_string());
        return Err(EngineError::InvalidParams(format!("{} has unsaved changes in {shown}; save or close it there first", target.name())));
    }
    let (open_method, open_params) = target.live_open_envelope(doc);
    let opened = control::live_control(target, Some(port), &open_method, &open_params)?;
    let _ = control::live_control(target, Some(port), "ui.focus", &json!({}));
    receipt["already_open"] = json!(false);
    if let Some(prev) = info {
        receipt["previous"] = prev;
    }
    receipt["opened"] = opened.get("result").cloned().unwrap_or(opened);
    Ok(())
}

fn wait_for_listener(addr: SocketAddr, timeout: Duration) -> bool {
    let step = Duration::from_millis(75);
    let attempts = (timeout.as_millis() / step.as_millis()).max(1);
    for _ in 0..attempts {
        if control::is_listening(addr) {
            return true;
        }
        std::thread::sleep(step);
    }
    false
}
