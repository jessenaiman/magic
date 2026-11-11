//! Suite-native AI integration layer for Magic Studio built strictly on
//! [`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations).
//!
//! Security & boundary guarantees (`docs/ADOPT-A-TOOL.md` & `docs/PLATFORM-CONTRACT.md`):
//! - Magic never opens raw HTTP/TLS sockets, never reads or stores API keys, and never spawns
//!   CLI coding agents or browser windows.
//! - All secret lookup (OS keychain), TLS certificate checks (`craft-ai-http`), outbound disclosure
//!   consent (`ConsentLedger`), and provider adapters (`ProviderFactory::standard`: `openai` for
//!   OpenAI-compatible cloud or local subscription bridges, `declarative` for JSON data-spec routes,
//!   `comfyui`, `gemini`, `vertex`, `bfl`, `artcraft`) are owned by `craft-ai-integrations`.
//! - Magic enforces a user-controlled master kill-switch (`ai.enabled` in `config.toml`, `false` by
//!   default) plus per-role switches (`fidget.enabled`, `fidget.seamless`, `scribbles.enabled`).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use craft_ai_client::{AiClient, RoleStatus, ToolDesk};
use craft_ai_core::{
    AppId, CapabilityId, CapabilitySpec, Connection, ConnectionId, ConsentLedger, ConsentTerm,
    Event, InputRole, JobState, ModelId, ModelInfo, Output, Payload, Request, Role, RoleId,
};
use craft_ai_native::shared::SharedStore;
use serde_json::{Value, json};

use crate::settings::MagicPrefs;
use crate::settings::prefs::open_store;
use crate::{CraftHub, EngineError, Result, Session};

const MAX_REFERENCES: u32 = 5;
const POLL: Duration = Duration::from_millis(500);
const LIMIT: Duration = Duration::from_secs(600);

/// Upstream `ProviderFactory::standard()` provider names shipped by `storytold/craft-ai-integrations`.
pub const STANDARD_PROVIDERS: &[(&str, &str)] = &[
    ("openai", "OpenAI-Compatible / Local Subscription Bridge"),
    ("declarative", "Declarative JSON Route Spec (specs/<id>.json)"),
    ("comfyui", "Local ComfyUI Workflow"),
    ("gemini", "Google Gemini API"),
    ("vertex", "Google Vertex Express"),
    ("bfl", "Black Forest Labs (FLUX)"),
    ("artcraft", "ArtCraft Omni"),
];

fn bad(message: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(message.into())
}

fn ai_err(e: impl std::fmt::Display) -> EngineError {
    EngineError::CommandFailed("craft-ai".into(), e.to_string())
}

fn current_prefs() -> MagicPrefs {
    open_store().get().clone()
}

fn require_ai_enabled(prefs: &MagicPrefs) -> Result<()> {
    if !prefs.ai.enabled {
        return Err(bad(
            "AI integrations are turned off in Settings (`ai.enabled = false`). Enable AI in Edit -> Preferences -> AI & LLM first.",
        ));
    }
    Ok(())
}

/// Prompt-only `text.complete` role so both `craft-ai-openai` and prompt-only `craft-ai-declarative`
/// models satisfy `CORE-SYNC.md`'s input-eligibility check.
pub fn fidget_role() -> Result<Role> {
    Ok(Role {
        id: RoleId::new("fidget").map_err(|e| bad(e.to_string()))?,
        label: "Fidget (Structured Prompt Stager)".into(),
        capability: CapabilityId::parse("text.complete").map_err(|e| bad(e.to_string()))?,
        inputs: vec![InputRole::Prompt],
    })
}

/// Prompt-only `image.generate` role compatible with both `craft-ai-declarative` local subscription
/// specs and hosted/local `image.generate` providers.
pub fn scribbles_role() -> Result<Role> {
    Ok(Role {
        id: RoleId::new("scribbles").map_err(|e| bad(e.to_string()))?,
        label: "Scribbles (Configured Image Generator)".into(),
        capability: CapabilityId::parse("image.generate").map_err(|e| bad(e.to_string()))?,
        inputs: vec![InputRole::Prompt],
    })
}

/// Reference-aware `image.generate` role used when character turnaround/parts redraws attach reference PNGs.
pub fn scribbles_reference_role() -> Result<Role> {
    Ok(Role {
        id: RoleId::new("scribbles-reference").map_err(|e| bad(e.to_string()))?,
        label: "Scribbles Reference Redraw".into(),
        capability: CapabilityId::parse("image.generate").map_err(|e| bad(e.to_string()))?,
        inputs: vec![InputRole::Prompt, InputRole::Reference],
    })
}

pub fn magic_ai_roles() -> Result<Vec<Role>> {
    Ok(vec![fidget_role()?, scribbles_role()?, scribbles_reference_role()?])
}

fn standard_client() -> Result<AiClient> {
    let app_id = AppId::new("magic").map_err(ai_err)?;
    AiClient::standard(app_id).map_err(ai_err)
}

fn validate_endpoint_locality(endpoint: &str) -> Result<&'static str> {
    let trimmed = endpoint.trim();
    if trimmed.starts_with("https://") {
        return Ok("remote");
    }
    if let Some(rest) = trimmed.strip_prefix("http://") {
        let host = rest.split(['/', ':']).next().unwrap_or_default();
        let bracket_host = rest
            .strip_prefix('[')
            .and_then(|s| s.split(']').next())
            .unwrap_or_default();
        if matches!(host, "127.0.0.1" | "localhost") || bracket_host == "::1" {
            return Ok("local");
        }
        return Err(bad(
            "Plain HTTP endpoints are accepted only for loopback (`http://127.0.0.1:...` or `http://localhost:...`); remote services must use `https://`.",
        ));
    }
    Err(bad("Endpoint URL must start with `https://` (remote) or `http://127.0.0.1:...` / `http://localhost:...` (local loopback)."))
}

fn text_spec() -> Result<CapabilitySpec> {
    let cap = CapabilityId::parse("text.complete").map_err(ai_err)?;
    Ok(CapabilitySpec::new(cap)
        .accepting(InputRole::Prompt, 1)
        .accepting(InputRole::System, 1)
        .requiring(InputRole::Prompt))
}

fn image_spec(with_references: bool) -> Result<CapabilitySpec> {
    let cap = CapabilityId::parse("image.generate").map_err(ai_err)?;
    let mut spec = CapabilitySpec::new(cap)
        .accepting(InputRole::Prompt, 1)
        .requiring(InputRole::Prompt);
    if with_references {
        spec = spec.accepting(InputRole::Reference, MAX_REFERENCES);
    }
    Ok(spec)
}

fn read_shared_connections() -> Vec<Value> {
    let Ok(shared) = SharedStore::at_default_location() else {
        return Vec::new();
    };
    let mut snapshot = Vec::new();
    let _ = shared.update(|store| {
        for conn in &store.connections {
            let raw = serde_json::to_value(conn).unwrap_or_else(|_| json!({}));
            let models: Vec<Value> = conn
                .models
                .iter()
                .map(|m| json!({ "id": m.id.as_str(), "label": m.label }))
                .collect();
            snapshot.push(json!({
                "id": conn.id.as_str(),
                "label": conn.label,
                "provider": conn.provider,
                "endpoint": raw.get("endpoint").cloned().unwrap_or(Value::Null),
                "locality": raw.get("locality").cloned().unwrap_or(Value::Null),
                "models": models,
            }));
        }
    });
    snapshot
}

/// Live `craft-ai-integrations` status (`AiClient::standard` + `SharedStore` + `MagicPrefs`).
pub fn craft_ai_status() -> Value {
    let prefs = current_prefs();
    let roles = match magic_ai_roles() {
        Ok(r) => r,
        Err(e) => return json!({"ok": false, "error": e.to_string()}),
    };
    let role_reports: Vec<Value> = match standard_client() {
        Ok(client) => roles
            .iter()
            .map(|role| {
                let status = if !prefs.ai.enabled {
                    json!({ "state": "disabled", "reason": "Disabled by master switch (ai.enabled = false)" })
                } else {
                    match client.status(role) {
                        RoleStatus::Ready(ready) => json!({
                            "state": "ready",
                            "connection_id": ready.connection.as_str(),
                            "connection_label": ready.connection_label,
                            "model": ready.model.as_str(),
                            "model_label": ready.model_label,
                            "locality": format!("{:?}", ready.locality),
                            "billing": format!("{:?}", ready.billing),
                        }),
                        RoleStatus::Unavailable { reason } => json!({
                            "state": "unavailable",
                            "reason": reason.to_string(),
                        }),
                    }
                };
                json!({
                    "id": role.id.as_str(),
                    "label": role.label,
                    "capability": role.capability.as_str(),
                    "status": status,
                })
            })
            .collect(),
        Err(e) => roles
            .iter()
            .map(|role| {
                json!({
                    "id": role.id.as_str(),
                    "label": role.label,
                    "capability": role.capability.as_str(),
                    "status": { "state": "unavailable", "reason": e.to_string() },
                })
            })
            .collect(),
    };
    let adapters: Vec<Value> = STANDARD_PROVIDERS
        .iter()
        .map(|(id, label)| json!({"id": id, "label": label}))
        .collect();
    json!({
        "ok": true,
        "app_id": "magic",
        "enabled": prefs.ai.enabled,
        "fidget": {
            "enabled": prefs.fidget.enabled,
            "seamless": prefs.fidget.seamless,
            "connection": prefs.fidget.connection,
            "model": prefs.fidget.model,
        },
        "scribbles": {
            "enabled": prefs.scribbles.enabled,
            "connection": prefs.scribbles.connection,
            "model": prefs.scribbles.model,
            "endpoint": prefs.scribbles.endpoint,
            "template": prefs.scribbles.template,
        },
        "standard_providers": adapters,
        "connections": read_shared_connections(),
        "roles": role_reports,
    })
}

pub fn cmd_ai_status(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(craft_ai_status())
}

fn upsert_user_connection(p: &Value) -> Result<Option<String>> {
    let Some(conn_obj) = p.get("register_connection").and_then(Value::as_object) else {
        return Ok(None);
    };
    let conn_id_raw = conn_obj
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad("register_connection.id is required (e.g. `local-sub` or `my-openai`)"))?;
    let provider = conn_obj
        .get("provider")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("openai");
    if !STANDARD_PROVIDERS.iter().any(|(known, _)| *known == provider) {
        return Err(bad(format!(
            "Unknown `craft-ai-integrations` adapter `{provider}`; supported standard adapters: openai, declarative, comfyui, gemini, vertex, bfl, artcraft"
        )));
    }
    let label = conn_obj
        .get("label")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(conn_id_raw);
    let endpoint = conn_obj
        .get("endpoint")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let locality = match endpoint {
        Some(url) => validate_endpoint_locality(url)?,
        None => conn_obj.get("locality").and_then(Value::as_str).unwrap_or("remote"),
    };
    let secret_name = conn_obj
        .get("secret_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let allow_refs = provider != "declarative";
    let mut models: Vec<ModelInfo> = Vec::new();
    if let Some(tm) = conn_obj.get("text_model").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        models.push(ModelInfo::new(ModelId::new(tm).map_err(ai_err)?, tm).with(text_spec()?));
    }
    if let Some(im) = conn_obj.get("image_model").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(existing) = models.iter_mut().find(|m| m.id.as_str() == im) {
            let merged = ModelInfo::new(existing.id.clone(), existing.label.clone())
                .with(text_spec()?)
                .with(image_spec(allow_refs)?);
            *existing = merged;
        } else {
            models.push(ModelInfo::new(ModelId::new(im).map_err(ai_err)?, im).with(image_spec(allow_refs)?));
        }
    }
    if models.is_empty() {
        return Err(bad(
            "Provide at least `text_model` (for Fidget `text.complete`) or `image_model` (for Scribbles `image.generate`) when registering a connection.",
        ));
    }

    let auth = match secret_name {
        Some(secret_ref) => json!({ "kind": "api_key", "secret": secret_ref }),
        None => json!({ "kind": "none" }),
    };
    let mut conn_json = json!({
        "id": conn_id_raw,
        "label": label,
        "provider": provider,
        "locality": locality,
        "auth": auth,
        "billing": if locality == "local" { "none" } else { "subscription_allowance" },
        "models": models,
    });
    if let Some(url) = endpoint {
        conn_json["endpoint"] = json!(url);
    }
    let conn: Connection = serde_json::from_value(conn_json).map_err(ai_err)?;
    let shared = SharedStore::at_default_location().map_err(ai_err)?;
    shared
        .update(|store| {
            store.connections.retain(|existing| existing.id != conn.id);
            store.connections.push(conn);
        })
        .map_err(ai_err)?;
    Ok(Some(conn_id_raw.to_owned()))
}

fn bind_role_in_desk(role: &Role, conn_raw: &str, model_raw: &str) -> Result<()> {
    let conn_id = ConnectionId::new(conn_raw.trim()).map_err(ai_err)?;
    let model_id = ModelId::new(model_raw.trim()).map_err(ai_err)?;
    let shared = SharedStore::at_default_location().map_err(ai_err)?;
    let client = standard_client()?;
    ToolDesk::new(client, shared, None)
        .choose(role, &conn_id, &model_id)
        .map_err(ai_err)
}

/// `ai.configure` (also wired for `scribbles.connect`): updates AI toggles, registers an optional
/// `SharedStore` connection (with zero secret values in JSON), and binds `fidget` / `scribbles` in `ToolDesk`.
pub fn cmd_ai_configure(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(en) = p.get("enabled").and_then(Value::as_bool) {
        let _ = s.execute("prefs.set", json!({"key": "ai.enabled", "value": en}))?;
    }
    if let Some(fe) = p.get("fidget_enabled").and_then(Value::as_bool) {
        let _ = s.execute("prefs.set", json!({"key": "fidget.enabled", "value": fe}))?;
    }
    if let Some(fs) = p.get("fidget_seamless").and_then(Value::as_bool) {
        let _ = s.execute("prefs.set", json!({"key": "fidget.seamless", "value": fs}))?;
    }
    if let Some(se) = p.get("scribbles_enabled").and_then(Value::as_bool) {
        let _ = s.execute("prefs.set", json!({"key": "scribbles.enabled", "value": se}))?;
    }
    if let Some(ep) = p.get("endpoint").and_then(Value::as_str) {
        if !ep.trim().is_empty() {
            let _ = validate_endpoint_locality(ep)?;
        }
        let _ = s.execute("prefs.set", json!({"key": "scribbles.endpoint", "value": ep}))?;
    }

    let registered = upsert_user_connection(p)?;

    if let (Some(fc), Some(fm)) = (
        p.get("fidget_connection").and_then(Value::as_str).filter(|v| !v.trim().is_empty()),
        p.get("fidget_model").and_then(Value::as_str).filter(|v| !v.trim().is_empty()),
    ) {
        bind_role_in_desk(&fidget_role()?, fc, fm)?;
        let _ = s.execute("prefs.set", json!({"key": "fidget.connection", "value": fc}))?;
        let _ = s.execute("prefs.set", json!({"key": "fidget.model", "value": fm}))?;
    }

    let scrib_conn = p
        .get("scribbles_connection")
        .or_else(|| p.get("connection"))
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .or(registered.as_deref());
    let scrib_model = p
        .get("scribbles_model")
        .or_else(|| p.get("model"))
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty());
    if let (Some(sc), Some(sm)) = (scrib_conn, scrib_model) {
        bind_role_in_desk(&scribbles_role()?, sc, sm)?;
        let _ = bind_role_in_desk(&scribbles_reference_role()?, sc, sm);
        let _ = s.execute("prefs.set", json!({"key": "scribbles.connection", "value": sc}))?;
        let _ = s.execute("prefs.set", json!({"key": "scribbles.model", "value": sm}))?;
    }

    Ok(craft_ai_status())
}

/// `ai.consent` (and `scribbles.consent`): grants or revokes outbound disclosure consent in `ConsentLedger`.
pub fn cmd_ai_consent(_: &mut Session, p: &Value) -> Result<Value> {
    let client = standard_client()?;
    let file = client
        .consent_file()
        .ok_or_else(|| ai_err("this system has no folder for the craft-ai consent file"))?;

    if p.get("revoke").and_then(Value::as_bool) == Some(true) {
        file.save(&ConsentLedger::default()).map_err(ai_err)?;
        return Ok(json!({"ok": true, "revoked": true}));
    }

    if p.get("agree").and_then(Value::as_bool) != Some(true)
        && p.get("agree").and_then(Value::as_str) != Some("true")
    {
        return Err(bad("Pass agree=true (or revoke=true) to update outbound AI consent."));
    }
    let wanted = p.get("term").and_then(Value::as_str).unwrap_or("until_revoked");
    let term = ConsentTerm::ALL.into_iter().find(|t| t.id() == wanted).ok_or_else(|| {
        let names: Vec<&str> = ConsentTerm::ALL.iter().map(|t| t.id()).collect();
        bad(format!("term must be one of {}", names.join(", ")))
    })?;

    let mut ledger: ConsentLedger = file.load_or_empty();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(ai_err)?.as_secs();
    let mut destinations = Vec::new();

    for role in [fidget_role()?, scribbles_role()?, scribbles_reference_role()?] {
        if matches!(client.status(&role), RoleStatus::Ready(_)) {
            let mut req = Request::new(role.capability.clone()).with_input(InputRole::Prompt, Payload::text("consent-check"));
            if role.inputs.contains(&InputRole::Reference)
                && let Ok(inline) = Payload::inline("image/png", vec![0x89, b'P', b'N', b'G'])
            {
                req = req.with_input(InputRole::Reference, inline);
            }
            if let Ok(plan) = client.plan(&role, req) {
                destinations.push(plan.disclosure().destination.clone());
                ledger.agree(plan.disclosure(), term, now);
            }
        }
    }

    if destinations.is_empty() {
        return Err(bad(
            "Bind a connection and model for Fidget or Scribbles in Preferences -> AI & LLM before granting consent.",
        ));
    }
    if term.remembers() {
        file.save(&ledger).map_err(ai_err)?;
    }
    Ok(json!({
        "ok": true,
        "term": term.id(),
        "remembered": term.remembers(),
        "destinations": destinations,
    }))
}

/// Rephrases a rough user prompt into a structured JSON brief via Fidget (`text.complete` in `craft-ai-integrations`).
pub fn rephrase_prompt_with_fidget(template_name: &str, base_prompt: &str) -> Result<Value> {
    let prefs = current_prefs();
    require_ai_enabled(&prefs)?;
    if !prefs.fidget.enabled {
        return Err(bad("Fidget prompt rephrasing is turned off (`fidget.enabled = false`)."));
    }
    let role = fidget_role()?;
    let client = standard_client()?;
    if let RoleStatus::Unavailable { reason } = client.status(&role) {
        return Err(bad(format!(
            "Fidget is not bound to a ready `text.complete` model: {reason}. Select a Fidget connection & model in Preferences -> AI & LLM."
        )));
    }
    let combined_prompt = format!(
        "Role: Fidget (Magic Studio structured prompt architect).\n\
         Template: `{template_name}`.\n\
         Return ONLY a valid JSON object (no markdown fences) with exact keys:\n\
         {{\"subject\": string, \"visual_style\": string, \"composition\": string, \"palette\": string, \"negative_constraints\": string, \"staged_prompt\": string}}.\n\
         Preserve every canvas dimension and stance constraint from the brief below inside `staged_prompt`.\n\n\
         Brief:\n{base_prompt}"
    );
    let req = Request::new(role.capability.clone()).with_input(InputRole::Prompt, Payload::text(combined_prompt));
    let plan = client.plan(&role, req).map_err(ai_err)?;
    let ledger = client.consent_file().map(|f| f.load_or_empty()).unwrap_or_default();
    if plan.needs_consent(&ledger) {
        return Err(bad(format!(
            "Sending prompts to `{}` requires consent: grant consent in Preferences -> AI & LLM (`ai.consent agree=true`).",
            plan.disclosure().destination
        )));
    }
    let finished = client.start(&plan, &ledger).map_err(ai_err)?.wait(POLL, LIMIT).map_err(ai_err)?;
    if let JobState::Failed { error } = &finished.state {
        return Err(ai_err(error));
    }
    let mut text_out = String::new();
    for event in &finished.events {
        if let Event::Output(Output { payload: Payload::Text { text }, .. }) = event {
            text_out.push_str(text);
        }
    }
    let trimmed = text_out
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed)
        && parsed.is_object()
    {
        return Ok(parsed);
    }
    Ok(json!({
        "subject": base_prompt,
        "staged_prompt": if trimmed.is_empty() { base_prompt } else { trimmed },
    }))
}

fn build_image_request(role: &Role, prompt: &str, references: &[PathBuf]) -> Result<Request> {
    let mut req = Request::new(role.capability.clone()).with_input(InputRole::Prompt, Payload::text(prompt));
    for path in references {
        let data = std::fs::read(path)?;
        let media = image::guess_format(&data).map_err(ai_err)?.to_mime_type();
        req = req.with_input(InputRole::Reference, Payload::inline(media, data).map_err(ai_err)?);
    }
    Ok(req)
}

pub fn draw_via_scribbles(prompt: &str, references: &[PathBuf], out: &Path) -> Result<()> {
    let prefs = current_prefs();
    require_ai_enabled(&prefs)?;
    if !prefs.scribbles.enabled {
        return Err(bad("Scribbles image generation is turned off (`scribbles.enabled = false`)."));
    }
    let role = if references.is_empty() {
        scribbles_role()?
    } else {
        scribbles_reference_role()?
    };
    let client = standard_client()?;
    if let RoleStatus::Unavailable { reason } = client.status(&role) {
        return Err(bad(format!(
            "Scribbles cannot run `{}`: {reason}. Configure a compatible model in Preferences -> AI & LLM.",
            role.id.as_str()
        )));
    }
    let plan = client
        .plan(&role, build_image_request(&role, prompt, references)?)
        .map_err(ai_err)?;
    let ledger = client.consent_file().map(|f| f.load_or_empty()).unwrap_or_default();
    if plan.needs_consent(&ledger) {
        return Err(bad(format!(
            "Sending prompts to `{}` requires consent: grant consent in Preferences -> AI & LLM (`ai.consent agree=true`).",
            plan.disclosure().destination
        )));
    }
    let finished = client.start(&plan, &ledger).map_err(ai_err)?.wait(POLL, LIMIT).map_err(ai_err)?;
    if let JobState::Failed { error } = &finished.state {
        return Err(ai_err(error));
    }
    let png = finished.events.iter().find_map(|ev| match ev {
        Event::Output(Output { payload: Payload::Inline { data, .. }, .. }) => Some(data.as_slice()),
        _ => None,
    });
    let bytes = png.ok_or_else(|| ai_err("the image.generate job finished without inline image bytes"))?;
    crate::write_file(out, bytes)?;
    Ok(())
}

/// `image.generate`: generates an image via `storytold/craft-ai-integrations` (`scribbles` role)
/// and runs optional `PhotoCraft` background isolation + `VectorCraft` silhouette tracing.
pub fn cmd_image_generate(s: &mut Session, p: &Value) -> Result<Value> {
    let prompt = p["prompt"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("image.generate requires a nonempty `prompt`"))?;

    let references: Vec<PathBuf> = match p.get("references").or_else(|| p.get("reference")) {
        None => Vec::new(),
        Some(Value::String(one)) if !one.trim().is_empty() => vec![crate::workspace::resolve_workspace_path(s, one)],
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
            .collect(),
        _ => Vec::new(),
    };
    if references.len() > MAX_REFERENCES as usize {
        return Err(bad("references may contain at most 5 images"));
    }

    let out_dir = match &s.project_path {
        Some(proj) => proj.parent().unwrap_or(&s.cwd).join("assets/generated"),
        None => s.cwd.join("generated"),
    };
    std::fs::create_dir_all(&out_dir)?;
    let base_name = p["name"]
        .as_str()
        .map(crate::workspace::sanitize_asset_id)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "scribbles-render".into());
    let raw_png = out_dir.join(format!("{base_name}.png"));
    draw_via_scribbles(prompt, &references, &raw_png)?;

    let mut final_png = raw_png.clone();
    if p["remove_background"].as_bool().unwrap_or(false) || p["transparent_background"].as_bool().unwrap_or(false) {
        let cut_png = out_dir.join(format!("{base_name}-cutout.png"));
        let chroma = p["chroma"].as_str();
        if CraftHub::remove_background_photo(&raw_png, &cut_png, chroma, 0.8).is_ok() && cut_png.is_file() {
            final_png = cut_png;
        }
    }

    let mut traced_svg = None;
    if p["vectorize"].as_bool().unwrap_or(false) {
        let svg_out = out_dir.join(format!("{base_name}.svg"));
        if CraftHub::trace_subject_vector(&final_png, &svg_out, None, true, false).is_ok() && svg_out.is_file() {
            traced_svg = Some(svg_out);
        }
    }

    let (width, height) = image::image_dimensions(&final_png).unwrap_or((1024, 1024));
    let mut attached_id = Value::Null;
    if s.project_path.is_some() && p["register_asset"].as_bool().unwrap_or(true) {
        if let Ok(res) = s.execute(
            "project.add_asset",
            json!({"path": final_png, "kind": "image", "name": base_name, "load": false}),
        ) {
            attached_id = res["asset_id"].clone();
        }
    }

    Ok(json!({
        "ok": true,
        "image": final_png,
        "raw_image": raw_png,
        "svg": traced_svg,
        "width": width,
        "height": height,
        "asset_id": attached_id,
    }))
}
