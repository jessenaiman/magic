//! Thin `--control` proxy from Magic's desktop bridge to system-installed ArtCraft windows.
//!
//! Magic drives the live desktop Studio UI views via `FilmCraft` (`filmcraft --control 9876` +
//! `FILMCRAFT_CONTROL_TOKEN`) and `EffectCraft` (`effectcraft --control 9877`) (plus `DesignCraft`,
//! `PhotoCraft`, and `VectorCraft`). Early-alpha `AniCraft` (`storytold/AniCraft`) is strictly
//! headless CLI-only (`anicraft-cli`) and is refused on live desktop `--control`.

use std::path::Path;

use magic::{CraftHub, CraftTarget};
use serde_json::{Value, json};

fn optional_port(params: &Value, method_name: &str) -> Result<Option<u16>, String> {
    let Some(raw) = params.get("port") else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let port = raw
        .as_u64()
        .and_then(|n| u16::try_from(n).ok())
        .filter(|&n| n > 0)
        .ok_or_else(|| format!("{method_name}: port must be an integer from 1 to 65535"))?;
    Ok(Some(port))
}

fn forward_target(target: CraftTarget, caller: &str, params: &Value, workspace_dir: Option<&Path>) -> Result<Value, String> {
    if !target.allows_live_ui() {
        return Err(format!(
            "{caller}: `{}` desktop UI is disabled for safety while AniCraft is in early alpha; run headless commands through `anicraft-cli` (`anicraft.exec`) and preview in EffectCraft or FilmCraft",
            target.name()
        ));
    }
    let port = optional_port(params, caller)?;
    if let Some(action) = params.get("action").and_then(Value::as_str) {
        return match action {
            "launch" | "open" => {
                let doc = params.get("path").and_then(Value::as_str).map(Path::new);
                CraftHub::launch_with_control(target, doc, port, workspace_dir).map_err(|e| format!("{}: {e}", target.name()))
            }
            "focus" => CraftHub::live_control(target, port, "ui.focus", &json!({})).map_err(|e| format!("{}: {e}", target.name())),
            "close" => CraftHub::close_opened_apps().map_err(|e| e.to_string()),
            "status" => {
                let addr = port.map_or_else(
                    || CraftHub::control_addr(target).map_err(|e| e.to_string()),
                    |p| Ok(std::net::SocketAddr::from(([127, 0, 0, 1], p))),
                )?;
                let listening = magic::craft_hub::control::is_listening(addr);
                Ok(json!({
                    "ok": true,
                    "app": target.name(),
                    "control_addr": addr.to_string(),
                    "port": addr.port(),
                    "listening": listening,
                    "cli": target.cli_control_hint(addr.port()),
                    "mcp": target.mcp_bridge_hint(addr.port()),
                }))
            }
            other => Err(format!("{caller}: unknown action `{other}` (expected status|launch|focus|close)")),
        };
    }
    let inner = params.get("params").cloned().unwrap_or_else(|| json!({}));
    if let Some(cmd) = params.get("command").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
        return CraftHub::live_execute(target, port, cmd, &inner).map_err(|e| format!("{}: {e}", target.name()));
    }
    let method = params["method"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{caller} requires `method`, `command`, or `action`"))?;
    CraftHub::live_control(target, port, method, &inner).map_err(|e| format!("{}: {e}", target.name()))
}

pub fn control(params: &Value, workspace_dir: Option<&Path>) -> Result<Value, String> {
    super::only_keys(params, "ui.filmcraft", &["action", "path", "command", "method", "params", "port"])?;
    forward_target(CraftTarget::Film, "ui.filmcraft", params, workspace_dir)
}

pub fn control_effect(params: &Value, workspace_dir: Option<&Path>) -> Result<Value, String> {
    super::only_keys(params, "ui.effectcraft", &["action", "path", "command", "method", "params", "port"])?;
    forward_target(CraftTarget::Effect, "ui.effectcraft", params, workspace_dir)
}

pub fn control_any(params: &Value, workspace_dir: Option<&Path>) -> Result<Value, String> {
    super::only_keys(params, "ui.craft", &["app", "action", "path", "command", "method", "params", "port"])?;
    let app = params["app"].as_str().ok_or("ui.craft needs an app name")?;
    let target = CraftTarget::from_name(app).ok_or_else(|| format!("ui.craft: unknown Craft app `{app}`"))?;
    forward_target(target, "ui.craft", params, workspace_dir)
}
