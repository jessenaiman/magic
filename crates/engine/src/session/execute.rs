use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde_json::Value;

use crate::cmd::project::format::Project;
use crate::commands::{CommandSpec, find};
use crate::history::Step;
use crate::{EngineError, Result, Session, check_param_types};

fn is_lifecycle_command(id: &str) -> bool {
    matches!(id, "project.new" | "project.open" | "project.close" | "file.new" | "file.open" | "file.close" | "recovery.restore")
}

fn handle_post_execute(
    s: &mut Session,
    id: &str,
    spec: &CommandSpec,
    project_before: Option<PathBuf>,
    snapshot_before: Option<(Project, u64)>,
    saves_before: u64,
    out: &Value,
) {
    if is_lifecycle_command(id) || s.project_path != project_before {
        s.history.clear();
        if matches!(id, "project.close" | "file.close") {
            s.clear_active_recovery();
        } else if id == "recovery.restore" {
            crate::recovery::sync_dirty(s);
        }
    } else if s.project_saves.load(Ordering::Relaxed) > saves_before {
        s.mark_saved();
        s.clear_active_recovery();
    } else if let Some((prev_project, prev_rev)) = snapshot_before
        && s.project.as_ref().is_some_and(|curr| *curr != prev_project)
    {
        s.mark_dirty();
        s.history.push(Step { label: spec.label.into(), project: prev_project, revision: prev_rev });
        crate::recovery::sync_dirty(s);
    }
    s.record(out.clone());
}

impl Session {
    pub fn execute(&mut self, id: &str, params: Value) -> Result<Value> {
        let Some(spec) = find(id) else {
            return Err(EngineError::UnknownCommand(id.to_owned()));
        };
        if let Err(reason) = (spec.enabled)(self) {
            return Err(EngineError::CommandFailed(id.to_owned(), reason));
        }
        check_param_types(spec.params, &params)?;
        let project_before = self.project_path.clone();
        let saves_before = self.project_saves.load(Ordering::Relaxed);
        let snapshot_before = if spec.journal && !matches!(id, "edit.undo" | "edit.redo" | "prefs.set") && !is_lifecycle_command(id) {
            self.project.clone().map(|p| (p, self.project_revision))
        } else {
            None
        };
        let mut out = (spec.run)(self, &params)?;
        if out.is_object() {
            out["ok"] = Value::Bool(true);
        }
        handle_post_execute(self, id, spec, project_before, snapshot_before, saves_before, &out);
        Ok(out)
    }
}
