//! Undo/redo [`History`] storing typed `.magic` v1 [`Project`] snapshots and revision numbers.

use crate::cmd::project::format::Project;

pub const MAX_STEPS: usize = 64;

#[derive(Debug, Clone)]
pub struct Step {
    pub label: String,
    pub project: Project,
    pub revision: u64,
}

#[derive(Debug, Default, Clone)]
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
