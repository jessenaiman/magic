use crate::Session;

impl Session {
    pub fn mark_dirty(&mut self) {
        self.next_revision = self.next_revision.saturating_add(1);
        self.project_revision = self.next_revision;
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = self.project_revision;
    }
}
