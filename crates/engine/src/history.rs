//! Bounded in-memory undo and redo history for the open project.

/// Maximum undo steps kept in memory; oldest is evicted when exceeded.
pub const MAX_STEPS: usize = 200;

#[derive(Clone, Debug)]
pub struct Step {
    /// Command that made the change (shown as "Undo <label>").
    pub label: String,
    /// Project state captured before the edit.
    pub snapshot: serde_json::Value,
}

#[derive(Clone, Debug, Default)]
pub struct History {
    pub undo: Vec<Step>,
    pub redo: Vec<Step>,
}

impl History {
    pub fn push(&mut self, step: Step) {
        self.undo.push(step);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
