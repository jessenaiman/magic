//! `character.action`: chain native EffectCraft / AniCraft CLI actions (no hardcoded pose tables).

use serde_json::{Value, json};

use super::bad;
use crate::{Result, Session};

pub fn cmd_character_action(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(bad("pending Step 2 EffectCraft/AniCraft CLI chain"))
}

pub fn cmd_character_actions(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"ok": true, "actions": []}))
}
