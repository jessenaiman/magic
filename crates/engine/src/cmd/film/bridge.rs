//! Cross-app `FilmCraft` × `EffectCraft` integration bridge (`T04`–`T09`, `T26`):
//! - `bridge.status` (`T07`): Live `filmcraft --control 127.0.0.1:9876` (`FILMCRAFT_CONTROL_TOKEN`)
//!   and `effectcraft --control 9877` (`effectcraft-cli mcp --autosave`) status & token readiness.
//! - `bridge.control` (`T07`): Headless/CLI/MCP/UI bridge dispatcher (`launch`, `focus`, `close`,
//!   `execute`, `control`) onto `FilmCraft` (`:9876`), `EffectCraft` (`:9877`), or any ArtCraft window.
//! - `bridge.preflight` (`T08`): Offline media (`media.status` + `media.findMissing` + `footage.check`)
//!   and render-source overwrite guard (`file.checkOutput`).
//! - `bridge.history` (`T09`): Branching undo history (`edit.history.list` / `edit.history.goto`) &
//!   crash-journal recovery (`file.recover` on both `FilmCraft` and `EffectCraft`).
//! - `effect.lottie` (`T04`): Two-way Lottie / dotLottie (`file.exportLottie` / `file.importLottie`).
//! - `effect.import_aep` (`T05`): Clean-room After Effects `.aep` / `.aepx` import (`file.importAep`).
//! - `bridge.templates` (`T06`): Clean-room `.ectemplate` (including `effectcraft-cli render --template --values`)
//!   ↔ `.fcgt` dual-bridge (refusing `.mogrt`).
//! - `words.captions_sync` (`T26`): Bidirectional `V3` timed lyrics ↔ `FilmCraft` `C1` caption tracks
//!   (`Subtitle`, `CEA-608`, `CEA-708`, `Teletext` in `SRT`, `WebVTT`, `SCC`, `MCC`, `STL`, `TTML`)
//!   + `graphics.upgradeCaption`.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::craft_hub::control::{extract_doc_path, is_listening, live_control};
use crate::craft_hub::discovery::{
    control_addr, ensure_control_token, read_control_token, resolve_executable_path,
};
use crate::time::TICKS_PER_SECOND;
use crate::{CraftHub, CraftTarget, Result, Session};

use super::{bad, film};

fn resolve_ecproj_for_session(s: &Session, p: &Value) -> Result<PathBuf> {
    if let Some(raw) = p["ecproj"]
        .as_str()
        .or_else(|| p["project"].as_str())
        .filter(|v| !v.trim().is_empty())
    {
        return Ok(crate::workspace::resolve_workspace_path(s, raw.trim()));
    }
    let dir = crate::workshop::project_dir(s)?;
    let motion = dir.join("motion/actions.ecproj");
    if motion.is_file() {
        return Ok(motion);
    }
    Ok(dir.join("lettering/lyrics.ecproj"))
}

fn probe_open_doc(target: CraftTarget, port: Option<u16>) -> Option<PathBuf> {
    let (method, params) = target.live_inspect_envelope();
    let reply = live_control(target, port, &method, &params).ok()?;
    let body = reply.get("result").unwrap_or(&reply);
    extract_doc_path(target, body)
}

/// `bridge.status` (`T07`): inspect live `FilmCraft` (`:9876` + `FILMCRAFT_CONTROL_TOKEN`) and
/// `EffectCraft` (`:9877` + `mcp --autosave`) loopback control endpoints, CLI binaries, and integer ticks.
pub fn cmd_bridge_status(s: &mut Session, p: &Value) -> Result<Value> {
    let fc_addr = control_addr(CraftTarget::Film).ok();
    let ec_addr = control_addr(CraftTarget::Effect).ok();
    if p["ensure_tokens"].as_bool() == Some(true) {
        if let Some(a) = fc_addr {
            let _ = ensure_control_token(CraftTarget::Film, a.port());
        }
        if let Some(a) = ec_addr {
            let _ = ensure_control_token(CraftTarget::Effect, a.port());
        }
    }
    let fc_port = fc_addr.map_or(CraftTarget::Film.default_port(), |a| a.port());
    let ec_port = ec_addr.map_or(CraftTarget::Effect.default_port(), |a| a.port());
    let fc_live = fc_addr.is_some_and(is_listening);
    let ec_live = ec_addr.is_some_and(is_listening);
    let fc_has_token = read_control_token(CraftTarget::Film, fc_port).is_some();
    let stored = crate::project_settings::stored_settings();
    let mcp_autosave = stored
        .pointer("/craft/effectcraft_mcp_autosave")
        .or_else(|| stored.get("craft.effectcraft_mcp_autosave"))
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let fc_open_doc = fc_live.then(|| probe_open_doc(CraftTarget::Film, Some(fc_port))).flatten();
    let ec_open_doc = ec_live.then(|| probe_open_doc(CraftTarget::Effect, Some(ec_port))).flatten();

    let proj_dir = crate::workshop::project_dir(s).ok();
    Ok(json!({
        "ok": true,
        "ticks_per_second": TICKS_PER_SECOND,
        "filmcraft": {
            "cli_installed": resolve_executable_path("filmcraft-cli").is_some(),
            "app_installed": resolve_executable_path("filmcraft").is_some(),
            "control_addr": format!("127.0.0.1:{fc_port}"),
            "port": fc_port,
            "live_connected": fc_live,
            "tcp_listening": fc_live,
            "token_configured": fc_has_token,
            "token_ready": fc_has_token,
            "cli_bridge_hint": CraftTarget::Film.cli_control_hint(fc_port),
            "mcp_bridge_hint": CraftTarget::Film.mcp_bridge_hint(fc_port),
            "open_document": fc_open_doc,
        },
        "effectcraft": {
            "cli_installed": resolve_executable_path("effectcraft-cli").is_some(),
            "app_installed": resolve_executable_path("effectcraft").is_some(),
            "control_addr": format!("127.0.0.1:{ec_port}"),
            "port": ec_port,
            "live_connected": ec_live,
            "tcp_listening": ec_live,
            "token_ready": true,
            "autosave_flag": if mcp_autosave { "--autosave" } else { "" },
            "mcp_autosave": mcp_autosave,
            "cli_bridge_hint": CraftTarget::Effect.cli_control_hint(ec_port),
            "mcp_bridge_hint": CraftTarget::Effect.mcp_bridge_hint(ec_port),
            "mcp_headless_hint": CraftTarget::Effect.mcp_headless_hint(),
            "open_document": ec_open_doc,
        },
        "project_dir": proj_dir,
    }))
}

/// `bridge.control` (`T07`): launch, focus, close, or forward a `--control` method / engine command
/// to `FilmCraft` (`:9876`), `EffectCraft` (`:9877`), or another ArtCraft app from CLI, MCP, or UI.
pub fn cmd_bridge_control(s: &mut Session, p: &Value) -> Result<Value> {
    let app = p["app"].as_str().unwrap_or("filmcraft");
    let target = CraftTarget::from_name(app).ok_or_else(|| bad(format!("unknown Craft app `{app}`")))?;
    let port = p["port"].as_u64().and_then(|n| u16::try_from(n).ok()).filter(|&n| n > 0);
    let action = p["action"].as_str().unwrap_or("status");
    let ws_dir = s.project_path.as_deref().and_then(std::path::Path::parent);

    match action {
        "status" => cmd_bridge_status(s, p),
        "launch" | "open" => {
            let file = p["path"]
                .as_str()
                .filter(|v| !v.trim().is_empty())
                .map(|raw| crate::workspace::resolve_workspace_path(s, raw));
            CraftHub::launch_with_control(target, file.as_deref(), port, ws_dir)
        }
        "focus" => CraftHub::live_control(target, port, "ui.focus", &json!({})),
        "close" => CraftHub::close_opened_apps(),
        "execute" => {
            let cmd_id = p["command"]
                .as_str()
                .or_else(|| p["id"].as_str())
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| bad("bridge.control action=execute requires `command`"))?;
            let inner = p.get("params").cloned().unwrap_or_else(|| json!({}));
            CraftHub::live_execute(target, port, cmd_id, &inner)
        }
        "control" => {
            let method = p["method"]
                .as_str()
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| bad("bridge.control action=control requires `method`"))?;
            let inner = p.get("params").cloned().unwrap_or_else(|| json!({}));
            CraftHub::live_control(target, port, method, &inner)
        }
        other => Err(bad(format!("unknown bridge.control action `{other}` (expected status|launch|focus|close|execute|control)"))),
    }
}

/// `bridge.preflight` (`T08`): check offline media across `.fcproj` & linked `.ecproj` files and
/// validate render-source overwrite protection (`file.checkOutput`) prior to master renders.
pub fn cmd_bridge_preflight(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let action = p["action"].as_str().unwrap_or("check");
    let mut fc_media = Value::Null;
    let mut fc_missing = Value::Null;
    if f.project.is_file() {
        fc_media = CraftHub::execute(CraftTarget::Film, Some(&f.project), "media.status", &json!({})).unwrap_or(Value::Null);
        if action == "relink" {
            fc_missing = CraftHub::execute_and_save(
                CraftTarget::Film,
                &f.project,
                "media.findMissing",
                &json!({"searchDirs": [f.dir]}),
            )
            .unwrap_or(Value::Null);
        } else {
            fc_missing = CraftHub::execute(CraftTarget::Film, Some(&f.project), "media.findMissing", &json!({})).unwrap_or(Value::Null);
        }
    }

    let ecproj = resolve_ecproj_for_session(s, p)?;
    let mut ec_footage = Value::Null;
    let mut source_guard = Value::Null;
    if ecproj.is_file() {
        let out_candidate = p["output"]
            .as_str()
            .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
            .unwrap_or_else(|| f.export());
        let steps = vec![
            ("footage.check".to_string(), json!({"wait": true})),
            ("file.checkOutput".to_string(), json!({"path": out_candidate})),
        ];
        if let Ok(res) = CraftHub::run_effect_steps(Some(&ecproj), false, &steps) {
            ec_footage = res.first().cloned().unwrap_or(Value::Null);
            source_guard = res.get(1).cloned().unwrap_or(Value::Null);
        }
    }

    let linked = super::linked_comp::inspect_linked_comps(&f.dir);
    let offline_links: Vec<&super::linked_comp::LinkedCompRecord> = linked.iter().filter(|r| r.status == "offline").collect();
    let ec_missing_count = ec_footage["missing"]
        .as_array()
        .map_or(offline_links.len(), Vec::len);
    let fc_offline_count = fc_media
        .as_array()
        .map_or(0, |arr| arr.iter().filter(|m| m["status"].as_str() == Some("offline") || m["status"].as_str() == Some("missing")).count());
    let overwrite_conflict = source_guard["blocked"].as_bool() == Some(true)
        || source_guard["overwritesSource"].as_bool() == Some(true)
        || source_guard["conflict"].as_bool() == Some(true);

    Ok(json!({
        "ok": true,
        "all_online": ec_missing_count == 0 && fc_offline_count == 0,
        "fc_offline_count": fc_offline_count,
        "ecproj_missing_count": ec_missing_count,
        "source_overwrite_blocked": !overwrite_conflict,
        "filmcraft_media": fc_media,
        "filmcraft_missing": fc_missing,
        "effectcraft_footage": ec_footage,
        "render_source_protection": source_guard,
        "offline_linked_comps": offline_links,
    }))
}

/// `bridge.history` (`T09`): inspect or jump EffectCraft/FilmCraft branching undo states
/// (`edit.history.list` / `edit.history.goto`) or trigger crash-journal recovery (`file.recover`).
pub fn cmd_bridge_history(s: &mut Session, p: &Value) -> Result<Value> {
    let op = p["op"]
        .as_str()
        .or_else(|| p["action"].as_str())
        .unwrap_or("list");
    let target_app = p["app"].as_str().unwrap_or("effectcraft");

    if op == "recover_filmcraft" || (op == "recover" && target_app == "filmcraft") {
        let f = film(s)?;
        let project = f.require()?.to_path_buf();
        let res = CraftHub::execute_and_save(CraftTarget::Film, &project, "file.recover", &json!({}))?;
        return Ok(json!({"ok": true, "target": "filmcraft", "recovered": res}));
    }
    let ecproj = resolve_ecproj_for_session(s, p)?;
    if op == "recover_effectcraft" || (op == "recover" && target_app == "effectcraft") {
        if !ecproj.is_file() {
            return Err(bad(format!("No EffectCraft project at {}", ecproj.display())));
        }
        let res = CraftHub::run_effect_steps(Some(&ecproj), true, &[("file.recover".into(), json!({}))])?;
        return Ok(json!({"ok": true, "target": "effectcraft", "ecproj": ecproj, "recovered": res.first()}));
    }

    let mut journals = Vec::new();
    if let Ok(dir) = crate::workshop::project_dir(s) {
        let hist_dir = dir.join(".history");
        if hist_dir.is_dir() {
            for entry in std::fs::read_dir(&hist_dir).into_iter().flatten().flatten() {
                if entry.path().is_file() {
                    journals.push(entry.path());
                }
            }
        }
    }

    if op == "goto" {
        let mut goto_p = json!({});
        if let Some(idx) = p["index"].as_u64() {
            goto_p["index"] = json!(idx);
        } else if let Some(id) = p["state_id"].as_str() {
            goto_p["id"] = json!(id);
        }
        if target_app == "filmcraft" {
            let f = film(s)?;
            let project = f.require()?.to_path_buf();
            let res = CraftHub::execute_and_save(CraftTarget::Film, &project, "edit.history.goto", &goto_p)?;
            return Ok(json!({"ok": true, "target": "filmcraft", "jumped": res}));
        }
        if !ecproj.is_file() {
            return Err(bad(format!("No EffectCraft project at {}", ecproj.display())));
        }
        let res = CraftHub::run_effect_steps(Some(&ecproj), true, &[("edit.history.goto".into(), goto_p)])?;
        return Ok(json!({"ok": true, "target": "effectcraft", "ecproj": ecproj, "jumped": res.first()}));
    }

    let fc_hist = film(s)
        .ok()
        .filter(|f| f.project.is_file())
        .and_then(|f| CraftHub::execute(CraftTarget::Film, Some(&f.project), "edit.history.list", &json!({})).ok())
        .unwrap_or_else(|| json!({"entries": []}));
    let ec_states = if ecproj.is_file() {
        CraftHub::run_effect_steps(Some(&ecproj), false, &[("edit.history.list".into(), json!({}))])
            .ok()
            .and_then(|r| r.into_iter().next())
            .unwrap_or_else(|| json!([]))
    } else {
        json!([])
    };

    Ok(json!({
        "ok": true,
        "ecproj": ecproj,
        "states": ec_states,
        "filmcraft_history": fc_hist,
        "effectcraft_history": {"states": ec_states},
        "recovery_journals": journals,
    }))
}

/// `effect.lottie` (`T04`): two-way Lottie (`.json`) and dotLottie (`.lottie`) export & import,
/// surfacing EffectCraft's incompatibility `warnings` list.
pub fn cmd_effect_lottie(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let mode = p["mode"]
        .as_str()
        .or_else(|| p["direction"].as_str())
        .unwrap_or("export");
    let ecproj = resolve_ecproj_for_session(s, p)?;

    if mode == "import" {
        let raw_in = p["path"]
            .as_str()
            .or_else(|| p["source"].as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| bad("path=<.json or .lottie> is required for Lottie import"))?;
        let lottie_path = crate::workspace::resolve_workspace_path(s, raw_in);
        if !lottie_path.is_file() {
            return Err(bad(format!("Lottie file not found: {}", lottie_path.display())));
        }
        if let Some(parent) = ecproj.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let res = CraftHub::run_effect_steps(
            ecproj.is_file().then_some(ecproj.as_path()),
            true,
            &[("file.importLottie".into(), json!({"path": lottie_path}))],
        )?;
        let first = res.first().cloned().unwrap_or_else(|| json!({}));
        return Ok(json!({
            "ok": true,
            "mode": "import",
            "ecproj": ecproj,
            "path": lottie_path,
            "output": ecproj,
            "comp": first.get("comp"),
            "warnings": first.get("warnings").cloned().unwrap_or_else(|| json!([])),
            "result": first,
        }));
    }

    if !ecproj.is_file() {
        return Err(bad(format!("EffectCraft file not found: {}", ecproj.display())));
    }
    let want_dotlottie = p["format"].as_str().map_or_else(|| p["dotlottie"].as_bool().unwrap_or(true), |f| f != "json");
    let ext = if want_dotlottie { "lottie" } else { "json" };
    let out_path = p["path"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
        .unwrap_or_else(|| dir.join("exports").join(format!("motion-lottie.{ext}")));
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut exp_p = json!({
        "path": out_path,
        "includeExpressions": p["include_expressions"].as_bool().unwrap_or(true),
    });
    if let Some(comp) = p["comp"].as_str().filter(|v| !v.trim().is_empty()) {
        exp_p["comp"] = json!(comp);
    }
    let res = CraftHub::run_effect_steps(Some(&ecproj), false, &[("file.exportLottie".into(), exp_p)])?;
    let first = res.first().cloned().unwrap_or_else(|| json!({}));
    Ok(json!({
        "ok": true,
        "mode": "export",
        "ecproj": ecproj,
        "path": out_path,
        "output": out_path,
        "bytes": first.get("bytes"),
        "warnings": first.get("warnings").cloned().unwrap_or_else(|| json!([])),
        "result": first,
    }))
}

/// `effect.import_aep` (`T05`): clean-room After Effects `.aep` / `.aepx` import (`file.importAep`)
/// with `dryRun` conversion/loss report before committing `allowLosses`.
pub fn cmd_effect_import_aep(s: &mut Session, p: &Value) -> Result<Value> {
    let raw = p["path"]
        .as_str()
        .or_else(|| p["source"].as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("path=<.aep or .aepx> is required"))?;
    let aep_path = crate::workspace::resolve_workspace_path(s, raw);
    if !aep_path.is_file() {
        return Err(bad(format!("After Effects file not found: {}", aep_path.display())));
    }
    let dry_run = p["dry_run"].as_bool().or_else(|| p["dryRun"].as_bool()).unwrap_or(true);
    let allow_losses = p["allow_losses"].as_bool().or_else(|| p["allowLosses"].as_bool()).unwrap_or(!dry_run);
    let dest_ecproj = resolve_ecproj_for_session(s, p)?;
    if let Some(parent) = dest_ecproj.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let import_params = json!({
        "path": aep_path,
        "dryRun": dry_run,
        "allowLosses": allow_losses,
        "applySettings": p["apply_settings"].as_bool().unwrap_or(true),
    });
    let res = CraftHub::run_effect_steps(
        dest_ecproj.is_file().then_some(dest_ecproj.as_path()),
        !dry_run,
        &[("file.importAep".into(), import_params)],
    )?;
    let first = res.first().cloned().unwrap_or_else(|| json!({}));
    let report = first.get("report").cloned().unwrap_or_else(|| first.clone());
    let loss_count = report["losses"]
        .as_array()
        .or_else(|| report["warnings"].as_array())
        .map_or(0, Vec::len);
    Ok(json!({
        "ok": true,
        "dry_run": dry_run,
        "allow_losses": allow_losses,
        "source": aep_path,
        "dest_ecproj": dest_ecproj,
        "loss_count": loss_count,
        "report": report,
    }))
}

/// `bridge.templates` (`T06`): clean-room dual template bridge for EffectCraft `.ectemplate`
/// (`essential.exportTemplate`, `essential.set`, `effectcraft-cli render --template --values`)
/// and FilmCraft `.fcgt` (`graphics.template.*`).
pub fn cmd_bridge_templates(s: &mut Session, p: &Value) -> Result<Value> {
    let raw_check = p["path"]
        .as_str()
        .or_else(|| p["source"].as_str())
        .or_else(|| p["template_path"].as_str())
        .unwrap_or_default();
    if raw_check.trim().to_ascii_lowercase().ends_with(".mogrt") {
        return Err(bad(
            "Clean-room policy: Adobe .mogrt packages are refused by FilmCraft & EffectCraft; use .fcgt or .ectemplate",
        ));
    }
    let action = p["action"].as_str().unwrap_or("list");
    let inferred_ec = matches!(action, "export_ectemplate" | "set_essential" | "render_template")
        || raw_check.to_ascii_lowercase().ends_with(".ectemplate")
        || p["domain"].as_str() == Some("effectcraft");

    if inferred_ec {
        let dir = crate::workshop::project_dir(s)?;
        if action == "render" || action == "render_template" {
            let tpl = crate::workspace::resolve_workspace_path(s, raw_check);
            if !tpl.is_file() {
                return Err(bad(format!("EffectCraft template not found: {}", tpl.display())));
            }
            let out = p["output"]
                .as_str()
                .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
                .unwrap_or_else(|| dir.join("exports/template-render.mov"));
            let res = CraftHub::render_effect_template(&tpl, p.get("values"), &out, p["time"].as_f64())?;
            return Ok(json!({"ok": true, "domain": "effectcraft", "template": tpl, "output": out, "result": res}));
        }
        let ecproj = resolve_ecproj_for_session(s, p)?;
        if !ecproj.is_file() {
            return Err(bad(format!("No EffectCraft project at {}", ecproj.display())));
        }
        return match action {
            "export" | "export_ectemplate" => {
                let slug = p["name"].as_str().map(crate::workspace::sanitize_asset_id).filter(|v| !v.is_empty());
                let out = p["path"]
                    .as_str()
                    .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
                    .unwrap_or_else(|| dir.join("exports").join(format!("{}.ectemplate", slug.as_deref().unwrap_or("lyrics-rig"))));
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let comp = p["comp"].as_str().unwrap_or("Lyrics");
                let res = CraftHub::run_effect_steps(
                    Some(&ecproj),
                    false,
                    &[("essential.exportTemplate".into(), json!({"comp": comp, "path": out}))],
                )?;
                Ok(json!({"ok": true, "domain": "effectcraft", "path": out, "result": res.first()}))
            }
            "set" | "set_essential" => {
                let ctrl = p["control"].as_str().unwrap_or("Title");
                let val = p.get("value").cloned().unwrap_or_else(|| json!(""));
                let layer = p["layer"].as_str().unwrap_or("#1");
                let res = CraftHub::run_effect_steps(
                    Some(&ecproj),
                    true,
                    &[("essential.set".into(), json!({"layer": layer, "control": ctrl, "value": val}))],
                )?;
                Ok(json!({"ok": true, "domain": "effectcraft", "control": ctrl, "result": res.first()}))
            }
            "install" | "instantiate" => {
                let tpl = crate::workspace::resolve_workspace_path(s, raw_check);
                let res = CraftHub::run_effect_steps(
                    Some(&ecproj),
                    true,
                    &[("file.import".into(), json!({"paths": [tpl]}))],
                )?;
                Ok(json!({"ok": true, "domain": "effectcraft", "installed": tpl, "result": res.first()}))
            }
            _ => Ok(json!({"ok": true, "domain": "effectcraft", "extension": ".ectemplate", "ecproj": ecproj})),
        };
    }

    let f = film(s)?;
    let project = f.require()?.to_path_buf();
    match action {
        "apply" => {
            let tpl = p["template"].as_str().unwrap_or("Slab Lower Third");
            let values = p.get("values").cloned().unwrap_or_else(|| json!({}));
            let res = CraftHub::execute_and_save(
                CraftTarget::Film,
                &project,
                "graphics.template.apply",
                &json!({"template": tpl, "values": values}),
            )?;
            Ok(json!({"ok": true, "domain": "filmcraft", "template": tpl, "result": res}))
        }
        "set" => {
            let key = p["control"].as_str().or_else(|| p["key"].as_str()).unwrap_or("Title");
            let val = p.get("value").cloned().unwrap_or_else(|| json!(""));
            let res = CraftHub::execute_and_save(
                CraftTarget::Film,
                &project,
                "graphics.template.set",
                &json!({"key": key, "value": val}),
            )?;
            Ok(json!({"ok": true, "domain": "filmcraft", "control": key, "result": res}))
        }
        "export" | "export_fcgt" => {
            let out = p["path"]
                .as_str()
                .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
                .unwrap_or_else(|| f.dir.join("exports").join(format!("{}.fcgt", f.name)));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let name = p["name"].as_str().unwrap_or(&f.name);
            let res = CraftHub::execute(
                CraftTarget::Film,
                Some(&project),
                "graphics.template.export",
                &json!({"name": name, "path": out, "embedFonts": true}),
            )?;
            Ok(json!({"ok": true, "domain": "filmcraft", "path": out, "result": res}))
        }
        "install" | "instantiate" => {
            if raw_check.is_empty() {
                return Err(bad("path=<.fcgt> is required"));
            }
            let resolved = crate::workspace::resolve_workspace_path(s, raw_check);
            let res = CraftHub::execute_and_save(
                CraftTarget::Film,
                &project,
                "graphics.template.install",
                &json!({"path": resolved}),
            )?;
            Ok(json!({"ok": true, "domain": "filmcraft", "installed": resolved, "result": res}))
        }
        _ => {
            let res = CraftHub::execute(CraftTarget::Film, Some(&project), "graphics.template.list", &json!({}))?;
            Ok(json!({"ok": true, "domain": "filmcraft", "templates": res}))
        }
    }
}

/// `words.captions_sync` (`T26`): bidirectional `V3` timed lyrics ↔ FilmCraft `C1` caption track
/// (`Subtitle`, `CEA-608`, `CEA-708`, `Teletext`, `SRT`, `WebVTT`, `SCC`, `MCC`, `STL`, `TTML`) + `graphics.upgradeCaption`.
pub fn cmd_words_captions_sync(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let direction = p["direction"].as_str().unwrap_or("v3_to_c1");
    let raw_std = p["track_format"].as_str().or_else(|| p["standard"].as_str()).unwrap_or("Subtitle");
    let format_std = match raw_std.trim().to_ascii_lowercase().as_str() {
        "cea608" | "cea-608" | "608" => "CEA-608",
        "cea708" | "cea-708" | "708" => "CEA-708",
        "teletext" | "ttx" => "Teletext",
        _ => "Subtitle",
    };
    let file_ext = p["format"].as_str().unwrap_or("srt");
    let language = p["language"].as_str().unwrap_or("en");
    let project = f.require()?.to_path_buf();

    if direction == "c1_to_v3" {
        let pulled = f.dir.join("lettering").join("c1-pulled.srt");
        if let Some(parent) = pulled.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let ex = CraftHub::execute(
            CraftTarget::Film,
            Some(&project),
            "captions.export",
            &json!({"path": pulled, "format": "srt", "track": "C1"}),
        )?;
        let imported = if pulled.is_file() {
            crate::workshop::cmd_import_transcript(s, &json!({"path": pulled, "replace": true})).ok()
        } else {
            None
        };
        return Ok(json!({
            "ok": true,
            "direction": "c1_to_v3",
            "timeline": project,
            "pulled_srt": pulled,
            "exported": ex,
            "transcript": imported,
        }));
    }

    let srt_out = f.dir.join("lettering").join(format!("lyrics.{file_ext}"));
    let _ = crate::cmd::words::cmd_words_captions(
        s,
        &json!({
            "by": p["by"].as_str().unwrap_or("line"),
            "out": srt_out,
        }),
    );

    let mut results = Vec::new();
    if let Ok(nt) = CraftHub::execute_and_save(
        CraftTarget::Film,
        &project,
        "captions.newTrack",
        &json!({"format": format_std, "name": "C1 Lyrics", "language": language}),
    ) {
        results.push(nt);
    }
    if srt_out.is_file()
        && let Ok(imp) = CraftHub::execute_and_save(
            CraftTarget::Film,
            &project,
            "captions.import",
            &json!({"path": srt_out, "language": language}),
        )
    {
        results.push(imp);
    }
    let _ = CraftHub::execute_and_save(
        CraftTarget::Film,
        &project,
        "captions.setTrack",
        &json!({"track": "C1", "enabled": true, "syncLock": true}),
    );

    let mut upgraded = Value::Null;
    if p["upgrade_to_graphic"].as_bool() == Some(true)
        && let Ok(up) = CraftHub::execute_and_save(CraftTarget::Film, &project, "graphics.upgradeCaption", &json!({"track": "C1"}))
    {
        upgraded = up;
    }

    let mut sidecar = Value::Null;
    if p["export_sidecar"].as_bool() == Some(true) {
        let exp_path = f.dir.join("exports").join(format!("{}.{file_ext}", f.name));
        if let Some(parent) = exp_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Ok(ex) = CraftHub::execute(
            CraftTarget::Film,
            Some(&project),
            "captions.export",
            &json!({"path": exp_path, "format": file_ext, "track": "C1"}),
        ) {
            sidecar = ex;
        }
    }

    Ok(json!({
        "ok": true,
        "direction": "v3_to_c1",
        "timeline": project,
        "caption_source": srt_out,
        "track_format": format_std,
        "upgraded_graphic": upgraded,
        "sidecar": sidecar,
        "steps": results,
    }))
}
