//! `character.rig`: EffectCraft builds the puppet from the cut PhotoCraft document (`.pcraft` or
//! `.psd` via `Import as Composition` with layer sizes).

use serde_json::Value;

use super::bad;
use crate::{Result, Session};

pub fn cmd_character_rig(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(bad("pending Step 2 EffectCraft CLI chain"))
}
