//! Undo and redo of in-memory project edits (`edit.undo`, `edit.redo`).

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

impl Session {
    pub fn undo(&mut self) -> Result<Value> {
        let step = self.history.undo.pop().ok_or_else(|| EngineError::InvalidParams("Nothing to undo".into()))?;
        let label = step.label.clone();
        self.history.redo.push(step);
        self.mark_dirty();
        Ok(json!({"ok": true, "undone": label, "can_undo": !self.history.undo.is_empty(), "can_redo": true}))
    }

    pub fn redo(&mut self) -> Result<Value> {
        let step = self.history.redo.pop().ok_or_else(|| EngineError::InvalidParams("Nothing to redo".into()))?;
        let label = step.label.clone();
        self.history.undo.push(step);
        self.mark_dirty();
        Ok(json!({"ok": true, "redone": label, "can_undo": true, "can_redo": !self.history.redo.is_empty()}))
    }
}

pub fn cmd_edit_undo(s: &mut Session, _: &Value) -> Result<Value> {
    s.undo()
}

pub fn cmd_edit_redo(s: &mut Session, _: &Value) -> Result<Value> {
    s.redo()
}
