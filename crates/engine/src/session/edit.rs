//! In-memory project editing helpers for [`crate::Session`].

use crate::Session;

impl Session {
    pub fn mark_dirty(&mut self) {
        self.project_revision = self.project_revision.saturating_add(1);
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = self.project_revision;
    }
}
