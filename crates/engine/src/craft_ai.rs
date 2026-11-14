//! AI status, configuration, consent, and generation backed by `craft-ai-integrations`
//! (`craft-ai-core`, `craft-ai-native`, `craft-ai-client`, and `craft-ai-features`).
//!
//! Provides the coordinator's AI integration layer for `AppId("magic")`:
//! - Shared store (`craft_ai_native::shared::SharedStore` / `craft_ai_core::StoreFile`)
//! - Native consent management (`craft_ai_native::ConsentFile` / `craft_ai_core::ConsentLedger`)
//! - Official connection management (`craft_ai_client::ConnectionManager`)
//! - Client and tool status evaluation (`craft_ai_client::AiClient` and `craft_ai_client::AppTools`)
//! - In-memory isolation fallback when `resolved_config_dir().is_none()` (`MAGIC_CONFIG_DIR=""` or `MAGIC_NO_PREFS=1`)

use std::sync::{Arc, Mutex};

use craft_ai_client::ConnectionManager;
use craft_ai_core::capability::CapabilityId;
use craft_ai_core::connection::Billing;
use craft_ai_core::consent::{ConsentLedger, Disclosure, DisclosureItem};
use craft_ai_core::draft::ConnectionDraft;
use craft_ai_core::endpoint::Endpoint;
use craft_ai_core::history::{HistoryLog, merge as merge_history};
use craft_ai_core::ids::{AppId, ConnectionId, ModelId, RoleId};
use craft_ai_core::model::{InputRole, ModelInfo};
use craft_ai_core::payload::{Limits, Payload, PayloadKind};
use craft_ai_core::request::Request;
use craft_ai_core::role::{Binding, Role};
use craft_ai_core::store::StoreFile;
pub use craft_ai_core::{PRESETS, Preset};
use craft_ai_native::secrets::standard_store;
use craft_ai_native::shared::SharedStore;
use craft_ai_native::{ConsentFile, HistoryFile};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub const STANDARD_PROVIDERS: &[(&str, &str)] =
    &[("local", "Local Loopback Bridge"), ("openai", "OpenAI"), ("openrouter", "OpenRouter"), ("bfl", "Black Forest Labs"), ("seedance", "Seedance")];

/// The stable `AppId` for Magic Studio.
pub fn magic_app_id() -> Result<AppId> {
    AppId::new("magic").map_err(|e| EngineError::InvalidParams(format!("invalid Magic Studio app id: {e}")))
}

/// Resolved path to the shared connections file when storage is enabled.
pub fn current_shared_store() -> Option<SharedStore> {
    if std::env::var_os("MAGIC_CONFIG_DIR").is_some_and(|v| v.is_empty()) {
        return None;
    }
    let cfg_dir = crate::settings::prefs::resolved_config_dir()?;
    let is_explicit = std::env::var_os("MAGIC_CONFIG_DIR").is_some_and(|v| !v.is_empty());
    if is_explicit { Some(SharedStore::new(cfg_dir.join("ai/connections.json"))) } else { SharedStore::at_default_location().ok() }
}

/// Resolved path to the app's consent file when storage is enabled.
pub fn current_consent_file() -> Option<ConsentFile> {
    if std::env::var_os("MAGIC_CONFIG_DIR").is_some_and(|v| v.is_empty()) {
        return None;
    }
    let cfg_dir = crate::settings::prefs::resolved_config_dir()?;
    let is_explicit = std::env::var_os("MAGIC_CONFIG_DIR").is_some_and(|v| !v.is_empty());
    if is_explicit {
        Some(ConsentFile::new(cfg_dir.join("ai/consent/magic.json")))
    } else {
        magic_app_id().ok().and_then(|id| ConsentFile::at_default_location(&id))
    }
}

/// Resolved path to the shared request log (`jobs.jsonl`) when storage is enabled.
pub fn current_history_file() -> Option<HistoryFile> {
    if std::env::var_os("MAGIC_CONFIG_DIR").is_some_and(|v| v.is_empty()) {
        return None;
    }
    let cfg_dir = crate::settings::prefs::resolved_config_dir()?;
    let is_explicit = std::env::var_os("MAGIC_CONFIG_DIR").is_some_and(|v| !v.is_empty());
    if is_explicit { Some(HistoryFile::new(cfg_dir.join("ai/jobs.jsonl"))) } else { HistoryFile::at_default_location() }
}

/// Load the shared AI request log (`HistoryLog`), newest first.
pub fn load_history_log() -> HistoryLog {
    let Some(file) = current_history_file() else {
        return HistoryLog::default();
    };
    match file.read() {
        Ok(read) => merge_history(read.records, read.skipped),
        Err(_) => HistoryLog::default(),
    }
}

/// Active connection manager backed by the active shared store and platform secrets.
pub fn current_connection_manager() -> Option<ConnectionManager> {
    let store = current_shared_store()?;
    let secrets = standard_store(None);
    Some(ConnectionManager::new(store, Arc::new(secrets)))
}

struct InMemoryAiState {
    store: StoreFile,
    consent: ConsentLedger,
}

static IN_MEMORY_STATE: Mutex<InMemoryAiState> = Mutex::new(InMemoryAiState {
    store: StoreFile {
        schema_version: craft_ai_core::store::SCHEMA_VERSION,
        connections: Vec::new(),
        defaults: Vec::new(),
        apps: std::collections::BTreeMap::new(),
        custom_tools: Vec::new(),
        model_folders: Vec::new(),
    },
    consent: ConsentLedger { records: Vec::new() },
});

/// Load the current `StoreFile` from disk or memory fallback.
pub fn load_store() -> StoreFile {
    if let Some(store) = current_shared_store()
        && let Ok(file) = store.load()
    {
        return file;
    }
    match IN_MEMORY_STATE.lock() {
        Ok(g) => g.store.clone(),
        Err(p) => p.into_inner().store.clone(),
    }
}

/// Load the current `ConsentLedger` from disk or memory fallback.
pub fn load_consent() -> ConsentLedger {
    if let Some(file) = current_consent_file() {
        return file.load_or_empty();
    }
    match IN_MEMORY_STATE.lock() {
        Ok(g) => g.consent.clone(),
        Err(p) => p.into_inner().consent.clone(),
    }
}

/// Persist `consent` using `craft_ai_native::ConsentFile` or memory state.
pub fn save_consent(consent: &ConsentLedger) -> Result<()> {
    if let Some(file) = current_consent_file() {
        file.save(consent).map_err(|e| EngineError::InvalidParams(format!("cannot save consent: {e}")))?;
    }
    let mut guard = match IN_MEMORY_STATE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.consent = consent.clone();
    Ok(())
}

/// The standard creative image generation role for Magic Studio.
pub fn image_generate_role() -> Result<Role> {
    let role_id = RoleId::new("image.generate").map_err(|e| EngineError::InvalidParams(format!("invalid role id: {e}")))?;
    let cap_id = CapabilityId::parse("image.generate").map_err(|e| EngineError::InvalidParams(format!("invalid capability id: {e}")))?;
    Ok(Role { id: role_id, label: "Image Generation".to_owned(), capability: cap_id, inputs: vec![InputRole::Prompt] })
}

/// Global AI status report used by `magic-cli app doctor` and health probes.
pub fn craft_ai_status() -> Value {
    let enabled = crate::settings::prefs::open_store().get().ai.enabled;
    let store = load_store();
    let Ok(app_id) = magic_app_id() else {
        return json!({"ok": false, "error": "invalid Magic Studio app id"});
    };
    let Ok(role) = image_generate_role() else {
        return json!({"ok": false, "error": "invalid image.generate role"});
    };
    let resolved_info = store.resolve(&app_id, &role).ok().map(|res| {
        json!({
            "connection_id": res.connection.id,
            "provider": res.connection.provider,
            "model_id": res.model.id,
            "locality": format!("{:?}", res.connection.locality),
        })
    });
    let storage_path = current_shared_store().map(|s| s.path().display().to_string());
    json!({
        "ok": true,
        "app_id": app_id,
        "enabled": enabled,
        "connections": store.connections,
        "defaults": store.defaults,
        "roles": [
            {
                "id": role.id,
                "label": role.label,
                "capability": role.capability,
                "resolved": resolved_info,
            }
        ],
        "storage": {
            "persistent": storage_path.is_some(),
            "path": storage_path,
        }
    })
}

/// `ai.status`: return current connection, role resolution, and consent status for this session.
pub fn cmd_ai_status(s: &mut Session, _p: &Value) -> Result<Value> {
    let enabled = s.settings.prefs().ai.enabled;
    let store = load_store();
    let app_id = magic_app_id()?;
    let role = image_generate_role()?;
    let resolved_info = store.resolve(&app_id, &role).ok().map(|res| {
        json!({
            "connection_id": res.connection.id,
            "provider": res.connection.provider,
            "model_id": res.model.id,
            "locality": format!("{:?}", res.connection.locality),
        })
    });
    let ledger = load_consent();
    let storage_path = current_shared_store().map(|s| s.path().display().to_string());
    Ok(json!({
        "ok": true,
        "app_id": app_id,
        "enabled": enabled,
        "connections": store.connections,
        "defaults": store.defaults,
        "roles": [
            {
                "id": role.id,
                "label": role.label,
                "capability": role.capability,
                "resolved": resolved_info,
            }
        ],
        "consent_records": ledger.records,
        "storage": {
            "persistent": storage_path.is_some(),
            "path": storage_path,
        }
    }))
}

/// `ai.history`: read the shared Craft AI job log (`jobs.jsonl`) newest first.
pub fn cmd_ai_history(_s: &mut Session, p: &Value) -> Result<Value> {
    let log = load_history_log();
    let app_filter = p.get("app").and_then(Value::as_str).filter(|s| !s.trim().is_empty());
    let limit = p.get("limit").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(200);
    let entries: Vec<Value> = log
        .entries
        .iter()
        .filter(|e| app_filter.is_none_or(|a| e.start.app == a))
        .take(limit)
        .map(|e| {
            json!({
                "run": e.start.run,
                "at_ms": e.start.at_ms,
                "app": e.start.app,
                "role_id": e.start.role_id,
                "role_label": e.start.role_label,
                "capability": e.start.capability,
                "connection": e.start.connection,
                "connection_label": e.start.connection_label,
                "model": e.start.model,
                "locality": e.start.locality,
                "destination": e.start.destination,
                "provider_job": e.start.provider_job,
                "estimate": e.start.estimate,
                "prompt": e.start.prompt,
                "duration_ms": e.duration_ms(),
                "outcome": e.outcome(),
                "end": e.end,
            })
        })
        .collect();
    Ok(json!({
        "ok": true,
        "skipped": log.skipped,
        "count": entries.len(),
        "entries": entries,
    }))
}

fn apply_draft_fields(draft: &mut ConnectionDraft, reg: &Value) {
    if let Some(label) = reg.get("label").and_then(Value::as_str) {
        draft.label = label.to_owned();
    }
    if let Some(prov) = reg.get("provider").and_then(Value::as_str) {
        draft.provider = prov.to_owned();
    }
    if let Some(ep) = reg.get("endpoint").and_then(Value::as_str) {
        draft.endpoint = ep.to_owned();
    }
    if let Some(k) = reg.get("key").and_then(Value::as_str) {
        draft.key = k.to_owned();
    }
    if let Some(kh) = reg.get("key_header").and_then(Value::as_str) {
        draft.key_header = kh.to_owned();
    }
    if let Some(uk) = reg.get("uses_key").and_then(Value::as_bool) {
        draft.uses_key = uk;
    }
    if let Some(en) = reg.get("enabled").and_then(Value::as_bool) {
        draft.enabled = en;
    }
    if let Some(b) = reg.get("billing").and_then(Value::as_str) {
        draft.billing = match b {
            "free" => Billing::Free,
            "metered" => Billing::Metered,
            "subscription_allowance" => Billing::SubscriptionAllowance,
            _ => Billing::Unknown,
        };
    }
    if let Some(models_val) = reg.get("models").and_then(Value::as_array) {
        let mut parsed_models = Vec::new();
        for m in models_val {
            if let Ok(model_info) = serde_json::from_value::<ModelInfo>(m.clone()) {
                parsed_models.push(model_info);
            }
        }
        if !parsed_models.is_empty() {
            draft.models = parsed_models;
        }
    }
}

fn handle_register_connection(reg: &Value) -> Result<()> {
    let store = load_store();
    let mut draft = if let Some(preset_name) = reg.get("preset").and_then(Value::as_str) {
        if let Some(preset) = PRESETS.iter().find(|pr| pr.key == preset_name) {
            ConnectionDraft::from_preset(preset)
        } else {
            return Err(EngineError::InvalidParams(format!("unknown AI connection preset '{preset_name}'")));
        }
    } else if let Some(edit_id_str) = reg.get("editing").or_else(|| reg.get("id")).and_then(Value::as_str) {
        let edit_id = ConnectionId::new(edit_id_str).map_err(|e| EngineError::InvalidParams(format!("invalid connection id: {e}")))?;
        let existing = store
            .connections
            .iter()
            .find(|c| c.id == edit_id)
            .ok_or_else(|| EngineError::InvalidParams(format!("connection '{edit_id}' not found to edit")))?;
        ConnectionDraft::from_connection(existing)
    } else {
        ConnectionDraft::default()
    };

    apply_draft_fields(&mut draft, reg);

    if let Some(mgr) = current_connection_manager() {
        mgr.save(&draft).map_err(|e| EngineError::InvalidParams(format!("failed to save connection: {e}")))?;
    } else {
        let prepared = draft.prepare(&store.connections).map_err(|e| EngineError::InvalidParams(format!("draft validation failed: {e}")))?;
        let mut guard = match IN_MEMORY_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(idx) = guard.store.connections.iter().position(|c| c.id == prepared.connection.id) {
            guard.store.connections[idx] = prepared.connection;
        } else {
            guard.store.connections.push(prepared.connection);
        }
    }
    Ok(())
}

fn handle_remove_connection(conn_id_str: &str) -> Result<()> {
    let conn_id = ConnectionId::new(conn_id_str).map_err(|e| EngineError::InvalidParams(format!("invalid connection id: {e}")))?;
    if let Some(mgr) = current_connection_manager() {
        mgr.delete(&conn_id).map_err(|e| EngineError::InvalidParams(format!("failed to delete connection: {e}")))?;
    } else {
        let mut guard = match IN_MEMORY_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let before_len = guard.store.connections.len();
        guard.store.connections.retain(|c| c.id != conn_id);
        if guard.store.connections.len() == before_len {
            return Err(EngineError::InvalidParams(format!("connection '{conn_id}' not found")));
        }
        guard.store.defaults.retain(|b| b.connection != conn_id);
        for app_settings in guard.store.apps.values_mut() {
            app_settings.overrides.retain(|b| b.connection != conn_id);
        }
    }

    let mut ledger = load_consent();
    let app_id = magic_app_id()?;
    ledger.revoke(&app_id, &conn_id);
    save_consent(&ledger)
}

fn apply_role_binding(
    store: &mut StoreFile,
    app_id: &AppId,
    role_id: &RoleId,
    conn_id: &ConnectionId,
    model_id: &ModelId,
    binding: &Binding,
) -> std::result::Result<(), craft_ai_core::store::StoreError> {
    let conn = store
        .connections
        .iter()
        .find(|c| c.id == *conn_id)
        .ok_or_else(|| craft_ai_core::store::StoreError::Syntax(format!("connection '{conn_id}' does not exist")))?;
    if !conn.models.iter().any(|m| m.id == *model_id) {
        return Err(craft_ai_core::store::StoreError::Syntax(format!("connection '{conn_id}' does not offer model '{model_id}'")));
    }
    let app_settings = store.apps.entry(app_id.clone()).or_default();
    if let Some(idx) = app_settings.overrides.iter().position(|b| b.role == *role_id) {
        app_settings.overrides[idx] = binding.clone();
    } else {
        app_settings.overrides.push(binding.clone());
    }
    Ok(())
}

fn handle_bind_role(bind_val: &Value) -> Result<()> {
    let role_str = bind_val.get("role").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("bind_role requires 'role'".into()))?;
    let conn_str = bind_val.get("connection").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("bind_role requires 'connection'".into()))?;
    let model_str = bind_val.get("model").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("bind_role requires 'model'".into()))?;

    let role_id = RoleId::new(role_str).map_err(|e| EngineError::InvalidParams(format!("invalid role id: {e}")))?;
    let conn_id = ConnectionId::new(conn_str).map_err(|e| EngineError::InvalidParams(format!("invalid connection id: {e}")))?;
    let model_id = ModelId::new(model_str).map_err(|e| EngineError::InvalidParams(format!("invalid model id: {e}")))?;

    let app_id = magic_app_id()?;
    let binding = Binding { role: role_id.clone(), connection: conn_id.clone(), model: model_id.clone() };

    if let Some(shared) = current_shared_store() {
        let inner = shared
            .try_update(|store| apply_role_binding(store, &app_id, &role_id, &conn_id, &model_id, &binding))
            .map_err(|e| EngineError::InvalidParams(e.to_string()))?;
        inner.map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    } else {
        let mut guard = match IN_MEMORY_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        apply_role_binding(&mut guard.store, &app_id, &role_id, &conn_id, &model_id, &binding).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    }
    Ok(())
}

/// `ai.configure`: master switch toggle, connection registration, deletion, and role binding.
pub fn cmd_ai_configure(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(flag) = p.get("enabled").and_then(Value::as_bool) {
        let _ = s.execute("prefs.set", json!({"key": "ai.enabled", "value": flag}))?;
    }
    if let Some(reg) = p.get("register_connection") {
        handle_register_connection(reg)?;
    }
    if let Some(rem) = p.get("remove_connection").and_then(Value::as_str) {
        handle_remove_connection(rem)?;
    }
    if let Some(bind_val) = p.get("bind_role") {
        handle_bind_role(bind_val)?;
    }
    cmd_ai_status(s, p)
}

/// `ai.consent`: grant or revoke consent for an outbound connection.
pub fn cmd_ai_consent(_s: &mut Session, p: &Value) -> Result<Value> {
    let app_id = magic_app_id()?;
    let mut ledger = load_consent();

    if p.get("revoke").and_then(Value::as_bool) == Some(true) {
        if let Some(conn_str) = p.get("connection").and_then(Value::as_str) {
            let conn_id = ConnectionId::new(conn_str).map_err(|e| EngineError::InvalidParams(format!("invalid connection id: {e}")))?;
            ledger.revoke(&app_id, &conn_id);
        } else {
            ledger.records.retain(|r| r.app != app_id);
        }
        save_consent(&ledger)?;
        return Ok(json!({"ok": true, "revoked": true, "records": ledger.records}));
    }

    if p.get("agree").and_then(Value::as_bool) == Some(true) {
        let store = load_store();
        let conn_id = if let Some(c) = p.get("connection").and_then(Value::as_str) {
            ConnectionId::new(c).map_err(|e| EngineError::InvalidParams(format!("invalid connection id: {e}")))?
        } else {
            let role = image_generate_role()?;
            let resolved = store.resolve(&app_id, &role).map_err(|e| EngineError::InvalidParams(format!("cannot resolve connection for consent: {e}")))?;
            resolved.connection.id.clone()
        };

        let conn = store.connections.iter().find(|c| c.id == conn_id).ok_or_else(|| EngineError::InvalidParams(format!("connection '{conn_id}' not found")))?;
        let cap_str = p.get("capability").and_then(Value::as_str).unwrap_or("image.generate");
        let cap = CapabilityId::parse(cap_str).map_err(|e| EngineError::InvalidParams(format!("invalid capability: {e}")))?;

        let dest = conn.endpoint.as_deref().and_then(|e| Endpoint::parse(e).ok()).map(|e| e.fingerprint()).unwrap_or_else(|| conn.provider.clone());

        let disclosure = Disclosure {
            app: app_id.clone(),
            connection: conn.id.clone(),
            capability: cap,
            locality: conn.locality,
            destination: dest,
            items: vec![DisclosureItem { role: InputRole::Prompt, kind: PayloadKind::Text, bytes: None }],
            param_names: Vec::new(),
        };

        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let term = p.get("term").and_then(Value::as_str).unwrap_or("until_revoked");
        match term {
            "one_hour" => ledger.grant_until(&disclosure, now.saturating_add(3600)),
            "one_day" => ledger.grant_until(&disclosure, now.saturating_add(86400)),
            _ => ledger.grant(&disclosure),
        }
        save_consent(&ledger)?;
        return Ok(json!({"ok": true, "granted": true, "records": ledger.records}));
    }

    Ok(json!({
        "ok": true,
        "records": ledger.records,
    }))
}

/// `image.generate`: validate role binding, model specification, and outbound consent before execution.
pub fn cmd_image_generate(s: &mut Session, p: &Value) -> Result<Value> {
    if !s.settings.prefs().ai.enabled {
        return Err(EngineError::InvalidParams("AI features are disabled; enable ai.enabled in Settings (prefs.set or ai.configure)".into()));
    }
    let prompt = p.get("prompt").and_then(Value::as_str).map(str::trim).unwrap_or_default();
    if prompt.is_empty() {
        return Err(EngineError::InvalidParams("need 'prompt' description for image generation".into()));
    }

    let store = load_store();
    let app_id = magic_app_id()?;
    let role = image_generate_role()?;
    let resolved = store
        .resolve(&app_id, &role)
        .map_err(|e| EngineError::InvalidParams(format!("cannot resolve AI role 'image.generate': {e}; configure connection and binding via ai.configure")))?;

    let req = Request::new(role.capability.clone()).with_input(InputRole::Prompt, Payload::text(prompt));
    req.validate(resolved.spec, &Limits::default())
        .map_err(|e| EngineError::InvalidParams(format!("invalid request for model '{}': {e}", resolved.model.id)))?;

    let disclosure = Disclosure::for_request(&app_id, resolved.connection, &req);
    let ledger = load_consent();
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    if ledger.needs_consent(&disclosure, now) {
        return Err(EngineError::InvalidParams(format!(
            "AI consent required for remote destination '{}' (capability '{}') before sending prompt; grant consent via ai.consent",
            disclosure.destination, disclosure.capability
        )));
    }

    Err(EngineError::InvalidParams(format!(
        "AI provider '{}' (model '{}') is validated and consent verified, but no live HTTP provider adapter is attached to dispatch the request",
        resolved.connection.provider, resolved.model.id
    )))
}
