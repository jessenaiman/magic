//! `character.mark`: character joint/part mark staging into the in-memory `Project`.

use serde_json::Value;

use super::bad;
use crate::{Result, Session};

pub fn cmd_character_mark(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(bad("pending Step 2 Project-backed character mark staging"))
}
