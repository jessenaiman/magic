//! Settings commands (`prefs.schema`, `prefs.get`, `prefs.set`) backed by the session's
//! in-memory [`crate::settings::prefs::SessionSettings`] (`config.toml` via `craft-settings`).

use serde_json::{Value, json};

use crate::{CommandSpec, EngineError, Result, Session};

fn spec(id: &'static str, label: &'static str, params: &'static str, run: fn(&mut Session, &Value) -> Result<Value>, journal: bool) -> CommandSpec {
    CommandSpec { id, label, menu: &[], shortcut: None, params, help: "", example: "", long_running: false, enabled: crate::commands::always, run, journal }
}

fn schema(s: &mut Session, p: &Value) -> Result<Value> {
    let want = p.get("category").and_then(Value::as_str);
    if let Some(w) = want
        && !super::CATEGORIES.iter().any(|c| c.id == w)
    {
        return Err(EngineError::InvalidParams(format!("unknown category `{w}`")));
    }
    Ok(super::schema_json(s, want))
}

fn get(s: &mut Session, p: &Value) -> Result<Value> {
    let Some(key) = p.get("key").and_then(Value::as_str).filter(|k| !k.trim().is_empty()) else {
        return Ok(json!({
            "prefs": s.settings.prefs(),
            "path": s.settings.path(),
            "report": s.settings.report_json(),
        }));
    };
    let known = super::fields_with_cat().iter().any(|(_, f)| f.key == key);
    if !known {
        return Err(EngineError::InvalidParams(format!("unknown setting `{key}`; see prefs.schema")));
    }
    Ok(json!({"key": key, "value": s.settings.get_value(key)}))
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    let key = p.get("key").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("need `key`".into()))?;
    let value = p.get("value").cloned().ok_or_else(|| EngineError::InvalidParams("need `value`".into()))?;
    // Flush immediately for headless CLI invocations; live desktop windows debounce via `SessionSettings::tick`.
    let flush_now = s.control_address.is_none();
    let saved = s.settings.set_value(key, &value, flush_now)?;
    Ok(json!({"key": key, "value": saved}))
}

pub fn commands() -> Vec<CommandSpec> {
    vec![
        spec("prefs.schema", "Settings Schema", r#"{"category":str?}"#, schema, false),
        spec("prefs.get", "Get Setting", r#"{"key":str?}"#, get, false),
        spec("prefs.set", "Set Setting", r#"{"key":str,"value":any}"#, set, true),
    ]
}
