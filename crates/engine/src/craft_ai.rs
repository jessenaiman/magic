//! Standard Craft AI integration backed by `storytold/craft-ai-integrations`.
//!
//! Uses `AiClient::standard`, `ConnectionManager::standard`, `craft_ai_features::builtin`,
//! `SharedStore`, and `craft_ai_agents` without app-specific stores or consent forks.

use craft_ai_client::{AiClient, ConnectionManager, RoleStatus};
use craft_ai_core::capability::CapabilityId;
use craft_ai_core::history::HistoryLog;
use craft_ai_core::ids::{AppId, ConnectionId, ModelId, RoleId};
use craft_ai_core::model::InputRole;
use craft_ai_core::role::Role;
use craft_ai_native::shared::SharedStore;
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

/// The stable `AppId` for Magic Studio.
pub fn magic_app_id() -> Result<AppId> {
    AppId::new("magic").map_err(|e| EngineError::InvalidParams(format!("invalid Magic Studio app id: {e}")))
}

/// Standard `AiClient` configured for `magic`.
pub fn standard_ai_client() -> Result<AiClient> {
    let app = magic_app_id()?;
    AiClient::standard(app).map_err(|e| EngineError::CommandFailed("ai".into(), e.to_string()))
}

/// Standard `ConnectionManager` sharing connections and secrets across ArtCraft apps.
pub fn standard_connection_manager() -> Result<ConnectionManager> {
    ConnectionManager::standard().map_err(|e| EngineError::CommandFailed("ai".into(), e.to_string()))
}

/// Built-in roles advertised by `craft-ai-features`, falling back to `image.generate` if empty.
pub fn builtin_roles() -> Vec<Role> {
    let mut roles: Vec<Role> = craft_ai_features::builtin().iter().filter_map(|f| f.role()).collect();
    if roles.is_empty()
        && let (Ok(id), Ok(capability)) = (RoleId::new("image.generate"), CapabilityId::parse("image.generate"))
    {
        roles.push(Role { id, label: "Image Generation".to_owned(), capability, inputs: vec![InputRole::Prompt] });
    }
    roles
}

/// Returns true when `magic` overrides the launcher default for `role_id`.
pub fn has_role_override(role_id: &RoleId) -> bool {
    let Ok(app_id) = magic_app_id() else {
        return false;
    };
    let Ok(shared) = SharedStore::at_default_location() else {
        return false;
    };
    let Ok(store) = shared.load() else {
        return false;
    };
    store.apps.get(&app_id).is_some_and(|a| a.overrides.iter().any(|b| &b.role == role_id))
}

fn role_status_json(client: Option<&AiClient>, role: &Role) -> Value {
    let resolved_info = match client.map(|c| c.status(role)) {
        Some(RoleStatus::Ready(ready)) => json!({
            "ready": true,
            "connection_id": ready.connection,
            "connection_label": ready.connection_label,
            "model_id": ready.model,
            "model_label": ready.model_label,
            "locality": format!("{:?}", ready.locality),
        }),
        Some(RoleStatus::Unavailable { reason }) => json!({ "ready": false, "reason": reason }),
        None => json!({ "ready": false, "reason": "AiClient unavailable" }),
    };
    json!({
        "id": role.id,
        "label": role.label,
        "capability": role.capability,
        "has_override": has_role_override(&role.id),
        "resolved": resolved_info,
    })
}

fn status_payload(enabled: bool) -> Value {
    let app_id = magic_app_id().ok();
    let client = standard_ai_client().ok();
    let shared = SharedStore::at_default_location().ok();
    let storage_path = shared.as_ref().map(|s| s.path().display().to_string());
    let store = shared.and_then(|s| s.load().ok());
    let connections = store.as_ref().map_or_else(Vec::new, |s| s.connections.clone());
    let defaults = store.as_ref().map_or_else(Vec::new, |s| s.defaults.clone());
    let roles: Vec<Value> = builtin_roles().iter().map(|role| role_status_json(client.as_ref(), role)).collect();
    json!({
        "ok": true,
        "app_id": app_id,
        "enabled": enabled,
        "agents": {
            "switched_on": craft_ai_agents::switched_on(),
            "magic_allowed": craft_ai_agents::allowed("magic"),
        },
        "connections": connections,
        "defaults": defaults,
        "roles": roles,
        "storage": {
            "persistent": storage_path.is_some(),
            "path": storage_path,
        },
    })
}

/// Global AI status report used by `magic-cli app doctor` and health probes.
pub fn craft_ai_status() -> Value {
    let enabled = crate::settings::prefs::open_store().get().ai.enabled;
    status_payload(enabled)
}

/// `ai.status`: return current connection, role resolution, and agent toggle status for this session.
pub fn cmd_ai_status(s: &mut Session, _p: &Value) -> Result<Value> {
    Ok(status_payload(s.settings.prefs().ai.enabled))
}

/// `ai.history`: read (or clear) the shared Craft AI job log (`jobs.jsonl`).
pub fn cmd_ai_history(_s: &mut Session, p: &Value) -> Result<Value> {
    let client = standard_ai_client()?;
    if p.get("clear").and_then(Value::as_bool) == Some(true) {
        client.clear_history().map_err(|e| EngineError::CommandFailed("ai.history".into(), e.to_string()))?;
    }
    let log = client.history().unwrap_or_else(|_| HistoryLog::default());
    let app_filter = p.get("app").and_then(Value::as_str).filter(|s| !s.trim().is_empty());
    let limit = p.get("limit").and_then(Value::as_u64).map_or(200, |n| n as usize);
    let entries: Vec<Value> = log
        .entries
        .iter()
        .filter(|e| app_filter.is_none_or(|a| e.start.app == a))
        .take(limit)
        .map(|e| {
            json!({
                "run": e.start.run,
                "capability": e.start.capability,
                "connection": e.start.connection,
                "connection_label": e.start.connection_label,
                "model": e.start.model,
                "locality": e.start.locality,
                "destination": e.start.destination,
                "duration_ms": e.duration_ms(),
                "outcome": e.outcome(),
            })
        })
        .collect();
    Ok(json!({ "ok": true, "skipped": log.skipped, "count": entries.len(), "entries": entries }))
}

/// `ai.configure`: master switch toggle and `AiClient` role binding/clearing.
pub fn cmd_ai_configure(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(flag) = p.get("enabled").and_then(Value::as_bool) {
        let _ = s.execute("prefs.set", json!({"key": "ai.enabled", "value": flag}))?;
    }
    if let Some(bind) = p.get("bind_role") {
        let client = standard_ai_client()?;
        let role_str = bind.get("role").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("bind_role requires 'role'".into()))?;
        let conn_str = bind.get("connection").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("bind_role requires 'connection'".into()))?;
        let model_str = bind.get("model").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("bind_role requires 'model'".into()))?;
        let role_id = RoleId::new(role_str).map_err(|e| EngineError::InvalidParams(format!("invalid role id: {e}")))?;
        let conn_id = ConnectionId::new(conn_str).map_err(|e| EngineError::InvalidParams(format!("invalid connection id: {e}")))?;
        let model_id = ModelId::new(model_str).map_err(|e| EngineError::InvalidParams(format!("invalid model id: {e}")))?;
        let role =
            builtin_roles().into_iter().find(|r| r.id == role_id).ok_or_else(|| EngineError::InvalidParams(format!("unknown Craft AI role '{role_id}'")))?;
        client.bind(&role, &conn_id, &model_id).map_err(|e| EngineError::CommandFailed("ai.configure".into(), e.to_string()))?;
    }
    if let Some(role_str) = p.get("clear_role").and_then(Value::as_str) {
        let client = standard_ai_client()?;
        let role_id = RoleId::new(role_str).map_err(|e| EngineError::InvalidParams(format!("invalid role id: {e}")))?;
        client.clear_binding(&role_id).map_err(|e| EngineError::CommandFailed("ai.configure".into(), e.to_string()))?;
    }
    cmd_ai_status(s, p)
}
