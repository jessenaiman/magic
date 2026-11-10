//! `app.doctor` (`magic-cli app doctor`): install, ArtCraft Launcher, CLI, and `--control`
//! readiness checks in the Craft suite's standard doctor shape (`checks: [{name, status, message, remediation?}]`).
//!
//! System Craft application installations and updates are owned by the ArtCraft Launcher
//! (`storytold/craft-launcher`), so `app.doctor` verifies local desktop/CLI binaries and active
//! `--control` listeners without polling GitHub's release API for sibling Craft apps.

use std::path::Path;

use serde_json::{Value, json};

use crate::craft_hub::CraftTarget;
use crate::craft_hub::control::is_listening;
use crate::craft_hub::discovery::{control_addr, detect_craft_launcher, resolve_executable_path};
use crate::{Result, Session};

/// One doctor finding.
struct Check {
    name: String,
    status: &'static str,
    message: String,
    remediation: Option<String>,
}

impl Check {
    fn ok(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), status: "ok", message: message.into(), remediation: None }
    }

    fn problem(status: &'static str, name: impl Into<String>, message: impl Into<String>, remediation: impl Into<String>) -> Self {
        Self { name: name.into(), status, message: message.into(), remediation: Some(remediation.into()) }
    }

    fn to_json(&self) -> Value {
        let mut out = json!({"name": self.name, "status": self.status, "message": self.message});
        if let Some(fix) = &self.remediation {
            out["remediation"] = json!(fix);
        }
        out
    }
}

fn tool_check(name: &str, missing: &'static str, remediation: &str) -> Check {
    match resolve_executable_path(name) {
        Some(path) => Check::ok(format!("tool:{name}"), format!("{name} is at {}", path.display())),
        None => Check::problem(missing, format!("tool:{name}"), format!("{name} was not found"), remediation),
    }
}

fn craft_target_checks(target: CraftTarget) -> [Check; 2] {
    let missing = if target.optional_in_doctor() { "warn" } else { "fail" };
    let remedy = format!("Install or update {} via the ArtCraft Launcher (https://getartcraft.com)", target.name());
    let cli_check = tool_check(target.cli_name(), missing, &remedy);

    let app_name = target.name();
    let app_check = match resolve_executable_path(app_name) {
        Some(path) => {
            let live_note = control_addr(target)
                .ok()
                .map(|addr| if is_listening(addr) { format!(" (--control listening on {addr})") } else { format!(" (--control idle on {addr})") })
                .unwrap_or_default();
            Check::ok(format!("app:{app_name}"), format!("{app_name} is at {}{live_note}", path.display()))
        }
        None => Check::problem(missing, format!("app:{app_name}"), format!("{app_name} desktop binary was not found"), &remedy),
    };
    [app_check, cli_check]
}

fn checks(s: &Session) -> Vec<Check> {
    let mut out = Vec::new();
    match detect_craft_launcher() {
        Some(path) => out.push(Check::ok("launcher:artcraft", format!("ArtCraft Launcher registered at {}", path.display()))),
        None => out.push(Check::problem(
            "warn",
            "launcher:artcraft",
            "ArtCraft Launcher desktop entry or binary not detected in standard user paths",
            "Keep Craft apps updated with the ArtCraft Launcher (https://github.com/storytold/craft-launcher)",
        )),
    }

    for target in CraftTarget::ALL {
        out.extend(craft_target_checks(target));
    }

    out.push(tool_check("magic-cli", "fail", "Build it with `cargo build -p magic-cli` and put target/debug on PATH"));
    out.push(tool_check("ffmpeg", "warn", "Optional fallback media tool (FilmCraft and craft-audio handle primary media)"));
    out.push(tool_check("codex", "warn", "Optional if using AGY or Flow for Scribbles image generation"));

    out.push(match s.control_address {
        Some(addr) => Check::ok("control", format!("live control listening on {addr}")),
        None => Check::ok("control", "no live control listener in this session (headless)"),
    });

    let project_dir = s.project_path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf).unwrap_or_else(|| s.cwd.clone());
    let server_file = project_dir.join(".video-server.json");
    if server_file.is_file() {
        out.push(Check::problem(
            "warn",
            "video-server",
            format!("{} is present; its server is not verified", server_file.display()),
            "Delete the file if no video preview server is running for this project",
        ));
    }
    out
}

/// `app.doctor`: local ArtCraft Launcher, `<product>` / `<product>-cli`, and `--control` checks.
pub fn cmd_app_doctor(s: &mut Session, _p: &Value) -> Result<Value> {
    let found = checks(s);
    let checks: Vec<Value> = found.iter().map(Check::to_json).collect();
    Ok(json!({"ok": true, "checks": checks}))
}
