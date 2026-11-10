//! The command registry: every user-visible action is a command with a stable id
//! (`domain.verb`) and JSON params, dispatched through [`crate::Session::execute`].

use std::sync::OnceLock;

use serde_json::Value;

use crate::{Result, Session};

pub use crate::project::write_project_file;
pub use crate::workspace::{asset_id_for_import, resolve_tool, resolve_workspace_path, sanitize_asset_id, workspace_root};

pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub menu: &'static [&'static str],
    pub shortcut: Option<&'static str>,
    pub params: &'static str,
    pub help: &'static str,
    pub example: &'static str,
    pub enabled: fn(&Session) -> std::result::Result<(), String>,
    pub run: fn(&mut Session, &Value) -> Result<Value>,
    pub journal: bool,
    pub long_running: bool,
}

impl CommandSpec {
    pub fn describe(&self) -> Value {
        serde_json::json!({
            "id": self.id,
            "label": self.label,
            "menu": self.menu,
            "shortcut": self.shortcut,
            "params": self.params,
            "journal": self.journal,
            "long_running": self.long_running,
            "help": self.help,
            "example": self.example,
        })
    }
}

pub fn is_read_only_command(id: &str, params: &Value) -> bool {
    match id {
        "project.catalog" | "file.recent" => params.get("forget").is_none_or(Value::is_null) && params["clear"] != true,
        "storyboard.locations" => {
            params.get("fields").is_none()
                && params.get("images").is_none()
                && params.get("text").is_none()
                && params.get("image").is_none()
                && params.get("refs").is_none()
        }
        "project.attach"
        | "file.list"
        | "backstage.changes"
        | "backstage.events"
        | "app.errors"
        | "project.identity"
        | "project.list_assets"
        | "project.snapshot"
        | "project.search"
        | "workshop.status"
        | "workshop.files"
        | "project.trunk"
        | "storyboard.pages"
        | "storyboard.check"
        | "animation.motion_options"
        | "animation.review"
        | "character.list"
        | "character.actions"
        | "song.lyrics"
        | "words.align"
        | "words.read"
        | "words.studio_track"
        | "words.effects"
        | "words.track"
        | "motion.read"
        | "agent.session"
        | "agent.config"
        | "agent.models"
        | "command.list"
        | "ui.inspect"
        | "ui.state"
        | "ui.elements"
        | "app.doctor"
        | "app.update" => true,
        "project.root" => params.get("root").is_none_or(Value::is_null),
        "words.workspace" => params.get("path").is_none_or(Value::is_null),
        "video.markers" => {
            let has_time_or_name =
                params.get("time").is_some() || params.get("seconds").is_some() || params.get("tick").is_some() || params.get("name").is_some();
            let action = params["action"].as_str().unwrap_or(if has_time_or_name { "add" } else { "list" });
            action == "list"
        }
        "agent.log" => params.get("refresh").and_then(Value::as_bool) != Some(true),
        _ => false,
    }
}

pub fn command_specs() -> &'static [CommandSpec] {
    static SPECS: OnceLock<Vec<CommandSpec>> = OnceLock::new();
    SPECS.get_or_init(crate::command_sets::build)
}

pub fn find(id: &str) -> Option<&'static CommandSpec> {
    command_specs().iter().find(|c| c.id == id)
}

pub(crate) fn always(_: &Session) -> std::result::Result<(), String> {
    Ok(())
}

pub fn check_param_types(description: &str, params: &Value) -> Result<()> {
    crate::check_param_types(description, params)
}
