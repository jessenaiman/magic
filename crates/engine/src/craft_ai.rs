//! Opt-in AI contract shell for Magic Studio built strictly on `craft-ai-core` and `craft-atomic`.

use std::path::{Path, PathBuf};
use std::process::Command;

use craft_ai_core::{AppId, CapabilityId, ConnectionId, InputRole, ModelId, Role, RoleId};
use serde_json::{Value, json};

use crate::settings::MagicPrefs;
use crate::settings::prefs::open_store;
use crate::{EngineError, Result, Session};

pub const STANDARD_PROVIDERS: &[(&str, &str)] = &[
    ("openai", "OpenAI-Compatible / Local Subscription Bridge"),
    ("declarative", "Declarative JSON Route Spec (specs/<id>.json)"),
    ("comfyui", "Local ComfyUI Workflow"),
    ("gemini", "Google Gemini API"),
    ("vertex", "Google Vertex Express"),
    ("bfl", "Black Forest Labs (FLUX)"),
    ("artcraft", "ArtCraft Omni"),
];

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn store_path() -> Option<PathBuf> {
    crate::workspace::magic_data_dir().map(|d| d.join("ai-store.json"))
}

fn load_store() -> Value {
    store_path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| json!({}))
}

fn save_store(doc: &Value) -> Result<()> {
    if let Some(path) = store_path() {
        crate::doc_state::write_json(&path, doc)?;
    }
    Ok(())
}

fn role_entry(prefs: &MagicPrefs, id: &str, label: &str, cap: &str, conn: Option<&str>, model: Option<&str>) -> Value {
    let role = RoleId::new(id).ok().zip(CapabilityId::parse(cap).ok()).map(|(rid, cid)| Role {
        id: rid,
        label: label.into(),
        capability: cid,
        inputs: vec![InputRole::Prompt],
    });
    let status = if !prefs.ai.enabled {
        json!({"state": "disabled", "reason": "Disabled by master switch (ai.enabled = false)"})
    } else if let (Some(c), Some(m)) = (conn.filter(|s| !s.is_empty()), model.filter(|s| !s.is_empty())) {
        json!({"state": "ready", "connection_id": c, "connection_label": c, "model": m, "model_label": m})
    } else {
        json!({"state": "unavailable", "reason": "Bind connection & model in Preferences -> AI & LLM"})
    };
    let (rid, rcap) = (role.as_ref().map_or(id, |r| r.id.as_str()), role.as_ref().map_or(cap, |r| r.capability.as_str()));
    json!({"id": rid, "label": label, "capability": rcap, "status": status})
}

fn status_for(prefs: &MagicPrefs) -> Value {
    let app_id = AppId::new("magic").ok().and_then(|a| serde_json::to_value(a).ok()).unwrap_or_else(|| json!("magic"));
    let store = load_store();
    let sc = &prefs.scribbles;
    json!({
        "ok": true, "app_id": app_id, "enabled": prefs.ai.enabled,
        "fidget": {"enabled": prefs.fidget.enabled, "seamless": prefs.fidget.seamless, "connection": prefs.fidget.connection, "model": prefs.fidget.model},
        "scribbles": {"enabled": sc.enabled, "connection": sc.connection, "model": sc.model, "endpoint": sc.endpoint, "template": sc.template},
        "standard_providers": STANDARD_PROVIDERS.iter().map(|(id, label)| json!({"id": id, "label": label})).collect::<Vec<_>>(),
        "connections": store.get("connections").and_then(Value::as_array).cloned().unwrap_or_default(),
        "consent": store.get("consent").cloned().unwrap_or(Value::Null),
        "roles": [
            role_entry(prefs, "fidget", "Fidget (Prompt Stager)", "text.complete", prefs.fidget.connection.as_deref(), prefs.fidget.model.as_deref()),
            role_entry(prefs, "scribbles", "Scribbles (Image Shell)", "image.generate", sc.connection.as_deref(), sc.model.as_deref()),
        ],
    })
}

pub fn craft_ai_status() -> Value {
    status_for(open_store().get())
}

pub fn cmd_ai_status(s: &mut Session, _p: &Value) -> Result<Value> {
    Ok(status_for(&s.settings.prefs()))
}

fn check_endpoint(raw: &str) -> Result<()> {
    let u = raw.trim();
    if u.is_empty() || u.starts_with("https://") || u.starts_with("http://127.0.0.1") || u.starts_with("http://localhost") || u.starts_with("http://[::1]") {
        Ok(())
    } else {
        Err(bad("Endpoint URL must use `https://` or loopback `http://127.0.0.1` / `http://localhost`"))
    }
}

pub fn cmd_ai_configure(s: &mut Session, p: &Value) -> Result<Value> {
    let flags =
        [("enabled", "ai.enabled"), ("fidget_enabled", "fidget.enabled"), ("fidget_seamless", "fidget.seamless"), ("scribbles_enabled", "scribbles.enabled")];
    for (k, pref) in flags {
        if let Some(flag) = p.get(k).and_then(Value::as_bool) {
            let _ = s.execute("prefs.set", json!({"key": pref, "value": flag}))?;
        }
    }
    if let Some(ep) = p.get("endpoint").and_then(Value::as_str) {
        check_endpoint(ep)?;
        let _ = s.execute("prefs.set", json!({"key": "scribbles.endpoint", "value": ep}))?;
    }
    let mut reg_id = None;
    if let Some(conn) = p.get("register_connection").and_then(Value::as_object) {
        let cid = conn.get("id").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("register_connection.id is required"))?;
        let _ = ConnectionId::new(cid).map_err(|e| bad(e.to_string()))?;
        let prov = conn.get("provider").and_then(Value::as_str).unwrap_or("openai").trim();
        if !STANDARD_PROVIDERS.iter().any(|(k, _)| *k == prov) {
            return Err(bad(format!("unknown provider `{prov}`")));
        }
        check_endpoint(conn.get("endpoint").and_then(Value::as_str).unwrap_or_default())?;
        let models: Vec<Value> = ["text_model", "image_model"]
            .into_iter()
            .filter_map(|k| conn.get(k).and_then(Value::as_str).map(str::trim))
            .filter(|m| !m.is_empty())
            .map(|m| json!({"id": m, "label": m}))
            .collect();
        let mut store = load_store();
        let mut list = store.get("connections").and_then(Value::as_array).cloned().unwrap_or_default();
        list.retain(|item| item["id"].as_str() != Some(cid));
        let label = conn.get("label").and_then(Value::as_str).unwrap_or(cid);
        let entry = json!({
            "id": cid, "label": label, "provider": prov,
            "endpoint": conn.get("endpoint"), "secret_ref": conn.get("secret_name"), "models": models,
        });
        list.push(entry);
        store["connections"] = json!(list);
        save_store(&store)?;
        reg_id = Some(cid.to_owned());
    }
    let bindings = [
        ("fidget_connection", "fidget_model", "fidget.connection", "fidget.model", None),
        ("scribbles_connection", "scribbles_model", "scribbles.connection", "scribbles.model", reg_id.as_deref()),
    ];
    for (ck, mk, cpref, mpref, fallback) in bindings {
        let alt_c = (ck == "scribbles_connection").then(|| p.get("connection")).flatten();
        let alt_m = (mk == "scribbles_model").then(|| p.get("model")).flatten();
        let c = p.get(ck).or(alt_c).and_then(Value::as_str).filter(|v| !v.trim().is_empty()).or(fallback);
        let m = p.get(mk).or(alt_m).and_then(Value::as_str).filter(|v| !v.trim().is_empty());
        if let (Some(cv), Some(mv)) = (c, m) {
            let _ = (ConnectionId::new(cv.trim()).map_err(|e| bad(e.to_string()))?, ModelId::new(mv.trim()).map_err(|e| bad(e.to_string()))?);
            let _ = s.execute("prefs.set", json!({"key": cpref, "value": cv}))?;
            let _ = s.execute("prefs.set", json!({"key": mpref, "value": mv}))?;
        }
    }
    Ok(status_for(&s.settings.prefs()))
}

pub fn cmd_ai_consent(_s: &mut Session, p: &Value) -> Result<Value> {
    let mut store = load_store();
    if p.get("revoke").and_then(Value::as_bool) == Some(true) {
        store["consent"] = Value::Null;
        save_store(&store)?;
        return Ok(json!({"ok": true, "revoked": true}));
    }
    if p.get("agree").and_then(Value::as_bool) != Some(true) && p.get("agree").and_then(Value::as_str) != Some("true") {
        return Err(bad("Pass agree=true (or revoke=true) to update outbound AI consent."));
    }
    let wanted = p.get("term").and_then(Value::as_str).unwrap_or("until_revoked");
    let remembered = match wanted {
        "once" | "this_request" | "session" => false,
        "one_hour" | "one_day" | "thirty_days" | "until_revoked" => true,
        _ => return Err(bad(format!("unknown consent term `{wanted}`"))),
    };
    let entry = json!({"agreed": true, "term": wanted, "remembered": remembered});
    if remembered {
        store["consent"] = entry.clone();
        save_store(&store)?;
    }
    Ok(json!({"ok": true, "term": wanted, "remembered": remembered, "consent": entry}))
}

pub fn rephrase_prompt_with_fidget(kind: &str, base_prompt: &str) -> Result<Value> {
    let prefs = open_store().get().clone();
    if !prefs.ai.enabled || !prefs.fidget.enabled {
        return Err(bad("Enable `ai.enabled` and `fidget.enabled` in Preferences -> AI & LLM before staging prompts."));
    }
    Ok(json!({"kind": kind, "subject": base_prompt.trim(), "staged_prompt": base_prompt.trim()}))
}

pub fn cmd_image_generate(s: &mut Session, p: &Value) -> Result<Value> {
    let prompt = p["prompt"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("image.generate requires a nonempty `prompt`"))?;
    let prefs = s.settings.prefs();
    if !prefs.ai.enabled || !prefs.scribbles.enabled {
        return Err(bad("AI image generation is disabled (`ai.enabled` / `scribbles.enabled` = false in Settings)."));
    }
    let ext_cmd = prefs.tools.get("scribbles-provider").cloned().or_else(|| {
        let dir = crate::extensions::resolve_scribbles_extension_dir(Some(s))?;
        let manifest: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).ok()?).ok()?;
        manifest["provider_command"].as_str().map(str::to_owned)
    });
    let Some(cmd_name) = ext_cmd.map(|c| c.trim().to_owned()).filter(|c| !c.is_empty()) else {
        return Err(bad("No external image provider shell is configured yet; set `provider_command` in `extensions/scribbles/manifest.json`."));
    };
    let exe = crate::workspace::resolve_executable_path(&cmd_name).ok_or_else(|| bad(format!("provider `{cmd_name}` not found on PATH")))?;
    let out_dir = s.project_path.as_deref().and_then(Path::parent).unwrap_or(&s.cwd).join("assets/generated");
    std::fs::create_dir_all(&out_dir)?;
    let stem = p["name"].as_str().map(crate::workspace::sanitize_asset_id).filter(|v| !v.is_empty()).unwrap_or_else(|| "scribbles-render".into());
    let raw_png = out_dir.join(format!("{stem}.png"));
    let mut cmd = Command::new(exe);
    cmd.arg("--prompt").arg(prompt).arg("--output").arg(&raw_png);
    if let Some(slide) = p["slide"].as_str().filter(|v| !v.is_empty()) {
        cmd.arg("--slide").arg(slide);
    }
    let out = cmd.output().map_err(|e| EngineError::ExternalTool(cmd_name, e.to_string()))?;
    if !out.status.success() || !raw_png.is_file() {
        return Err(EngineError::CommandFailed("image.generate".into(), String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    let (w, h) = image::image_dimensions(&raw_png).unwrap_or((1024, 1024));
    Ok(json!({"ok": true, "image": raw_png, "raw_image": raw_png, "width": w, "height": h}))
}
