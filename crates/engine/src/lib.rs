//! Shared project state and command dispatch for Magic Studio.
//!
//! The pure-Rust `eframe` + `egui` desktop shell (`magic`), `magic-cli`, and `magic-cli mcp` share
//! the same command registry and [`Session::execute`] entry point. Magic reads and writes project
//! assets on disk via [`storytold/craft-libs`](https://github.com/storytold/craft-libs), queries
//! running local Craft desktop apps over their `--control` JSON-lines sockets (`craft_api`), and
//! delegates AI/LLM capabilities to [`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations).

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod activity;
pub mod animation_model;
pub mod change_marker;
pub mod cmd;
pub mod command_sets;
pub mod commands;
pub mod confinement;
pub mod craft_ai;
pub mod craft_api;
pub mod craft_effects;
pub mod doc_state;
pub mod doctor;
pub mod error;
pub mod extensions;
pub mod guard;
pub mod history;
pub mod link_watch;
pub mod native_media;
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
pub use doc_state::write_file;
pub use error::{EngineError, Result};
pub use session::{Asset, ProjectAsset, Session};

use serde_json::Value;

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
