//! `app.doctor` (`magic-cli app doctor`): verify Magic's own `--control` listener,
//! `storytold/craft-ai-integrations` SharedStore health, and `craft-settings` configuration.

use serde_json::{Value, json};

use crate::{Result, Session};

struct Check {
    name: String,
    status: &'static str,
    message: String,
    remediation: Option<String>,
}

impl Check {
    fn ok(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: "ok",
            message: message.into(),
            remediation: None,
        }
    }

    fn problem(
        status: &'static str,
        name: impl Into<String>,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status,
            message: message.into(),
            remediation: Some(remediation.into()),
        }
    }

    fn to_json(&self) -> Value {
        let mut out = json!({"name": self.name, "status": self.status, "message": self.message});
        if let Some(fix) = &self.remediation {
            out["remediation"] = json!(fix);
        }
        out
    }
}

fn checks(s: &Session) -> Vec<Check> {
    let mut out = Vec::new();

    let ai = crate::craft_ai::craft_ai_status();
    let ai_enabled = ai["enabled"].as_bool().unwrap_or(false);
    let conn_count = ai["connections"].as_array().map_or(0, Vec::len);
    out.push(Check::ok(
        "ai:craft-ai-integrations",
        format!(
            "ai.enabled={ai_enabled}, {conn_count} SharedStore connection(s) configured via storytold/craft-ai-integrations"
        ),
    ));

    let settings_report = s.settings.report_json();
    if settings_report["has_problems"].as_bool() == Some(true)
        || settings_report["read_only"].as_bool() == Some(true)
    {
        out.push(Check::problem(
            "warn",
            "settings:config",
            format!(
                "craft-settings LoadReport flagged problems or read-only mode: {settings_report}"
            ),
            "Inspect config.toml (or any config.broken-*.toml backup beside it) and verify folder write permissions",
        ));
    } else {
        out.push(Check::ok(
            "settings:config",
            format!(
                "craft-settings ready (persistent={}, path={})",
                settings_report["persistent"],
                settings_report["path"].as_str().unwrap_or("memory")
            ),
        ));
    }

    out.push(match s.control_address {
        Some(addr) => Check::ok("control", format!("live control listening on {addr}")),
        None => Check::ok("control", "headless session (launch `magic --control 9875` for live UI control)"),
    });

    if let Some(proj) = &s.project_path {
        if proj.is_file() {
            out.push(Check::ok("project", format!("open project {}", proj.display())));
        } else {
            out.push(Check::problem(
                "warn",
                "project",
                format!("project path missing on disk: {}", proj.display()),
                "Run `file.save` or re-open an existing `.magic` project",
            ));
        }
    }

    out
}

pub fn cmd_app_doctor(s: &mut Session, _p: &Value) -> Result<Value> {
    let found = checks(s);
    let list: Vec<Value> = found.iter().map(Check::to_json).collect();
    let ok = found.iter().all(|c| c.status == "ok");
    Ok(json!({ "ok": ok, "checks": list }))
}
