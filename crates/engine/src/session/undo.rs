use serde_json::{Value, json};

use crate::history::Step;
use crate::{EngineError, Result, Session};

impl Session {
    pub fn undo(&mut self) -> Result<Value> {
        let step = self.history.undo.pop().ok_or_else(|| EngineError::InvalidParams("Nothing to undo".into()))?;
        let current = self.project.take().ok_or_else(|| EngineError::InvalidParams("No open project to undo".into()))?;
        let current_rev = self.project_revision;
        let label = step.label.clone();
        self.history.redo.push(Step { label: step.label, project: current, revision: current_rev });
        self.project = Some(step.project);
        self.project_revision = step.revision;
        crate::recovery::sync_dirty(self);
        Ok(json!({
            "ok": true,
            "undone": label,
            "dirty": self.is_dirty(),
            "revision": self.project_revision,
            "can_undo": !self.history.undo.is_empty(),
            "can_redo": true,
        }))
    }

    pub fn redo(&mut self) -> Result<Value> {
        let step = self.history.redo.pop().ok_or_else(|| EngineError::InvalidParams("Nothing to redo".into()))?;
        let current = self.project.take().ok_or_else(|| EngineError::InvalidParams("No open project to redo".into()))?;
        let current_rev = self.project_revision;
        let label = step.label.clone();
        self.history.undo.push(Step { label: step.label, project: current, revision: current_rev });
        self.project = Some(step.project);
        self.project_revision = step.revision;
        crate::recovery::sync_dirty(self);
        Ok(json!({
            "ok": true,
            "redone": label,
            "dirty": self.is_dirty(),
            "revision": self.project_revision,
            "can_undo": true,
            "can_redo": !self.history.redo.is_empty(),
        }))
    }
}

pub fn cmd_edit_undo(s: &mut Session, _: &Value) -> Result<Value> {
    s.undo()
}

pub fn cmd_edit_redo(s: &mut Session, _: &Value) -> Result<Value> {
    s.redo()
}
