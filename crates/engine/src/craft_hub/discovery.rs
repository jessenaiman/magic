//! Engine-level binary & control endpoint lookup wired onto [`craft_util::discovery`] and
//! `~/.config/magic/config.toml`.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

pub use craft_util::discovery::{detect_craft_launcher, read_control_token, same_canonical_path, scrubbed_command, token_file_path};

use super::target::CraftTarget;
use crate::{EngineError, Result};

pub fn resolve_executable_path(name: &str) -> Option<PathBuf> {
    let stored = crate::project_settings::stored_settings();
    let cfg = stored.pointer(&format!("/tools/{name}")).and_then(Value::as_str);
    craft_util::discovery::resolve_executable(name, cfg)
}

pub fn require_executable(name: &str) -> Result<PathBuf> {
    resolve_executable_path(name).ok_or_else(|| EngineError::ExternalTool(name.to_owned(), "executable not found via ArtCraft Launcher or PATH".into()))
}

pub fn control_addr(target: CraftTarget) -> Result<SocketAddr> {
    let stored = crate::project_settings::stored_settings();
    let cfg_port = stored.pointer(&format!("/craft/{}_port", target.name())).and_then(Value::as_u64).and_then(|n| u16::try_from(n).ok());
    Ok(craft_util::discovery::control_addr(target, cfg_port))
}

pub fn ensure_control_token(target: CraftTarget, port: u16) -> Result<(Option<String>, Option<PathBuf>)> {
    craft_util::discovery::ensure_control_token(target, port).map_err(|e| EngineError::InvalidParams(e.to_string()))
}

pub fn tool_command(name: &str) -> Result<Command> {
    let bin = require_executable(name)?;
    let mut cmd = Command::new(bin);
    craft_util::discovery::scrub_credentials(&mut cmd);
    Ok(cmd)
}
