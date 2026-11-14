//! The one execution path: every command from a view, the CLI, MCP, or `--control` runs here.

use serde_json::Value;

use crate::{EngineError, Result, Session, check_param_types, commands, guard};

fn execute_command(s: &mut Session, id: &str, params: Value) -> Result<Value> {
    let spec = commands::find(id).ok_or_else(|| EngineError::UnknownCommand(id.to_string()))?;
    check_param_types(spec.params, &params)?;
    if let Some(roots) = &s.automation_roots {
        roots.check_command(s, id, &params)?;
    }
    (spec.enabled)(s).map_err(|e| EngineError::CommandFailed(id.to_string(), e))?;
    let project_before = s.project_path.clone();
    let saves_before = s.project_saves.load(std::sync::atomic::Ordering::Relaxed);
    let out = guard::catch_panic(|| (spec.run)(s, &params))
        .map_err(|panic| EngineError::CommandFailed(id.to_string(), format!("panicked: {panic}")))
        .and_then(|result| result)?;
    let is_lifecycle = matches!(id, "project.open" | "project.new" | "project.close" | "file.open" | "file.new" | "file.close");
    if is_lifecycle || s.project_path != project_before {
        s.history.clear();
    } else if spec.journal
        && !matches!(id, "edit.undo" | "edit.redo")
        && out["ok"] != false
        && s.project_saves.load(std::sync::atomic::Ordering::Relaxed) == saves_before
    {
        s.mark_dirty();
    }
    if spec.journal && !matches!(id, "project.close" | "file.close") {
        s.record(out.clone());
    }
    Ok(out)
}

impl Session {
    /// Execute one command by id with JSON parameters, returning JSON.
    pub fn execute(&mut self, id: &str, params: Value) -> Result<Value> {
        execute_command(self, id, params)
    }
}
