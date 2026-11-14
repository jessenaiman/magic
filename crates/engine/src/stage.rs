//! Stage domain contract (`StageDomain`) for isolating studio stages from engine internals.
//!
//! `01 Storyboard` (`StoryboardStage`) is intentionally standalone (`timeline_track() == None`):
//! it manages an independent `.designcraft` document and never feeds `FilmCraft` timeline tracks.

use serde_json::{Value, json};

use crate::commands::CommandSpec;
use crate::{Result, Session};

/// FilmCraft timeline track target owned by a production stage, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackRole {
    V1Backgrounds,
    V2Animation,
    V3Lyrics,
    V4Motion,
    C1Captions,
    Finishing,
}

/// Engine-side contract implemented by a studio stage domain.
pub trait StageDomain: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn timeline_track(&self) -> Option<TrackRole>;
    fn commands(&self) -> Vec<CommandSpec>;
    fn snapshot(&self, session: &mut Session) -> Result<Value>;
}

/// Standalone `01 Storyboard` stage domain (`timeline_track() == None`).
pub struct StoryboardStage;

impl StageDomain for StoryboardStage {
    fn id(&self) -> &'static str {
        "storyboard"
    }

    fn label(&self) -> &'static str {
        "Storyboard"
    }

    fn timeline_track(&self) -> Option<TrackRole> {
        None
    }

    fn commands(&self) -> Vec<CommandSpec> {
        crate::command_sets::project::specs_storyboard()
    }

    fn snapshot(&self, session: &mut Session) -> Result<Value> {
        crate::cmd::storyboard::cmd_storyboard_pages(session, &json!({}))
    }
}
