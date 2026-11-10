//! Shared project state and command dispatch for Magic Studio's lightweight coordinator workspace.
//!
//! The desktop Tauri shell, `magic-cli`, and `magic-cli mcp` share the same command registry
//! and [`Session::execute`] entry point. Magic coordinates installed ArtCraft applications
//! (`DesignCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft`, and headless
//! `anicraft-cli`) via [`CraftHub`] rather than reimplementing media/compositor servers in-process.
//!
//! # Start here
//!
//! - [`Session`] holds the selected `.magic` project, connected assets, and `AutomationRoots`.
//! - [`command_specs`] and [`CommandSpec`] expose stable command IDs, parameter
//!   descriptions, enablement, journaling, and long-running metadata.
//! - [`find_command`] locates a command; [`Session::execute`] checks parameters
//!   and enablement before dispatching its handler and recording applicable edits.
//! - [`EngineError`] describes dispatch, validation, I/O, and external-tool failures.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod activity;
pub mod agent;
mod agent_run;
pub mod animation_model;
pub mod change_marker;
pub mod cmd;
pub mod command_sets;
pub mod commands;
pub mod confinement;
pub mod craft_ai_codex;
pub mod craft_hub;
pub mod doc_state;
pub mod doctor;
pub mod error;
pub mod guard;
pub mod history;
pub mod image_generation;
pub mod link_watch;
pub mod project_lock;
pub(crate) mod read_only_file;
pub mod scribbles;
pub mod session;
pub mod settings;
pub mod time;
pub mod update;
pub mod workspace;

pub use cmd::project;
pub use cmd::project::catalog as project_catalog;
pub use cmd::project::intake as workshop_intake;
pub use cmd::project::settings as project_settings;
pub use cmd::project::transcript as workshop_transcript;
pub use cmd::project::workshop;
pub use commands::{CommandSpec, command_specs, find as find_command};
pub use confinement::AutomationRoots;
pub use craft_hub::{CraftHub, CraftTarget};
pub use doc_state::write_file;
pub use error::{EngineError, Result};
pub use session::{Asset, ProjectAsset, Session};

use serde_json::Value;

/// Rejects a parameter whose JSON type differs from the example value in the command's `params`
/// description (e.g. `"auto_cut": "true"` where the description says `false`). Every caller (CLI,
/// MCP, desktop shell) goes through `Session::execute`, so a mistyped value fails consistently.
pub fn check_param_types(description: &str, params: &Value) -> Result<()> {
    let (Ok(Value::Object(expected)), Some(given)) = (serde_json::from_str::<Value>(description), params.as_object()) else {
        return Ok(());
    };
    for (key, value) in given {
        let wanted = match expected.get(key) {
            Some(Value::Bool(_)) => "boolean",
            Some(Value::Number(_)) => "number",
            _ => continue,
        };
        let ok = if wanted == "boolean" { value.is_boolean() } else { value.is_number() };
        if !ok {
            return Err(EngineError::InvalidParams(format!("`{key}` must be a {wanted}, got {value}")));
        }
    }
    Ok(())
}
