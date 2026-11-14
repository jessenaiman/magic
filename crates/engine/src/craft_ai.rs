//! AI status and configuration shell backed by `craft-ai-core` (clean rewrite in Step 2).

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub fn craft_ai_status() -> Value {
    json!({"ok": true, "app_id": "magic", "enabled": false, "connections": [], "roles": []})
}

pub fn cmd_ai_status(s: &mut Session, _p: &Value) -> Result<Value> {
    let enabled = s.settings.prefs().ai.enabled;
    Ok(json!({"ok": true, "app_id": "magic", "enabled": enabled, "connections": [], "roles": []}))
}

pub fn cmd_ai_configure(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(flag) = p.get("enabled").and_then(Value::as_bool) {
        let _ = s.execute("prefs.set", json!({"key": "ai.enabled", "value": flag}))?;
    }
    cmd_ai_status(s, p)
}

pub fn cmd_ai_consent(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(EngineError::InvalidParams("craft-ai-core desktop consent store is not initialized yet".into()))
}

pub fn cmd_image_generate(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(EngineError::InvalidParams("craft-ai-core image execution requires a desktop client adapter".into()))
}
