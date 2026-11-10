//! Thin `--control` proxy from Magic's desktop bridge to system-installed ArtCraft windows.
//!
//! Magic drives the live desktop Studio UI views via `FilmCraft` (`filmcraft --control`) and
//! `EffectCraft` (`effectcraft --control`) (plus `DesignCraft`, `PhotoCraft`, and `VectorCraft`
//! when editing documents directly). Early-alpha `AniCraft` (`storytold/AniCraft`) is strictly
//! headless CLI-only (`anicraft-cli`) and is refused on live desktop `--control`.

use magic::{CraftHub, CraftTarget};
use serde_json::{Value, json};

fn optional_port(params: &Value, method_name: &str) -> Result<Option<u16>, String> {
    let Some(raw) = params.get("port") else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let port =
        raw.as_u64().and_then(|n| u16::try_from(n).ok()).filter(|&n| n > 0).ok_or_else(|| format!("{method_name}: port must be an integer from 1 to 65535"))?;
    Ok(Some(port))
}

fn forward_target(target: CraftTarget, caller: &str, params: &Value) -> Result<Value, String> {
    if !target.allows_live_ui() {
        return Err(format!(
            "{caller}: `{}` desktop UI is disabled for safety while AniCraft is in early alpha; run headless commands through `anicraft-cli` (`anicraft.exec`) and preview in EffectCraft or FilmCraft",
            target.name()
        ));
    }
    let method = params["method"].as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| format!("{caller} needs a method"))?;
    let port = optional_port(params, caller)?;
    let inner = params.get("params").cloned().unwrap_or_else(|| json!({}));
    CraftHub::live_control(target, port, method, &inner).map_err(|e| format!("{}: {e}", target.name()))
}

/// `ui.filmcraft {method, params?, port?}`: send any FilmCraft `--control` method to the open
/// system `filmcraft --control` window.
pub(super) fn control(params: &Value) -> Result<Value, String> {
    super::only_keys(params, "ui.filmcraft", &["method", "params", "port"])?;
    forward_target(CraftTarget::Film, "ui.filmcraft", params)
}

/// `ui.effectcraft {method, params?, port?}`: send any EffectCraft `--control` method to the open
/// system `effectcraft --control` window.
pub(super) fn control_effect(params: &Value) -> Result<Value, String> {
    super::only_keys(params, "ui.effectcraft", &["method", "params", "port"])?;
    forward_target(CraftTarget::Effect, "ui.effectcraft", params)
}

/// `ui.craft {app, method, params?, port?}`: send any `--control` method to an open system
/// ArtCraft desktop window that supports live UI (`filmcraft`, `effectcraft`, `designcraft`,
/// `photocraft`, `vectorcraft`, `deckcraft`, `texturecraft`).
pub(super) fn control_any(params: &Value) -> Result<Value, String> {
    super::only_keys(params, "ui.craft", &["app", "method", "params", "port"])?;
    let app = params["app"].as_str().ok_or("ui.craft needs an app name")?;
    let target = CraftTarget::from_name(app).ok_or_else(|| format!("ui.craft: unknown Craft app `{app}`"))?;
    forward_target(target, "ui.craft", params)
}
