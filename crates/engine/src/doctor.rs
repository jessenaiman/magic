//! `app.doctor` (`magic-cli app doctor`): verify host Craft applications (`DesignCraft`,
//! `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft` via live `--control` API or `<app>-cli`),
//! `storytold/craft-ai-integrations` SharedStore health, `storytold/craft-extensions` store status,
//! and `craft-settings` configuration.

use serde_json::{Value, json};

use crate::craft_api::{HOST_CRAFT_APPS, probe_live_app};
use crate::{Result, Session};

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

fn checks(s: &Session) -> Vec<Check> {
    let mut out = Vec::new();

    for (app, cli, port, _, _, _) in HOST_CRAFT_APPS {
        let probe = probe_live_app(app);
        let live = probe["connected"].as_bool() == Some(true);
        let cli_ok = probe["cli_installed"].as_bool() == Some(true);
        if live || cli_ok {
            out.push(Check::ok(format!("host:{app}"), format!("{app} ready (control_connected={live} on 127.0.0.1:{port}, {cli}_installed={cli_ok})")));
        } else {
            out.push(Check::problem(
                "warn",
                format!("host:{app}"),
                format!("neither `{app} --control {port}` nor `{cli}` is reachable on this host"),
                format!("Install {app} via ArtCraft Launcher (putting `{cli}` on PATH) or start `{app} --control {port}`"),
            ));
        }
    }

    let ai = crate::craft_ai::craft_ai_status();
    let ai_enabled = ai["enabled"].as_bool().unwrap_or(false);
    let conn_count = ai["connections"].as_array().map_or(0, Vec::len);
    out.push(Check::ok(
        "ai:craft-ai-integrations",
        format!("ai.enabled={ai_enabled}, {conn_count} SharedStore connection(s) configured via storytold/craft-ai-integrations"),
    ));

    let ext = crate::extensions::extensions_status();
    out.push(Check::ok(
        "extensions:craft-extensions",
        format!(
            "store_exists={}, installed_packages={} (third-party disabled)",
            ext["exists"].as_bool().unwrap_or(false),
            ext["installed_packages"].as_u64().unwrap_or(0)
        ),
    ));

    let settings_report = s.settings.report_json();
    if settings_report["has_problems"].as_bool() == Some(true) || settings_report["read_only"].as_bool() == Some(true) {
        out.push(Check::problem(
            "warn",
            "settings:config",
            format!("craft-settings LoadReport flagged problems or read-only mode: {settings_report}"),
            "Inspect config.toml (or any config.broken-*.toml backup beside it) and verify folder write permissions",
        ));
    } else {
        out.push(Check::ok(
            "settings:config",
            format!("craft-settings ready (persistent={}, path={})", settings_report["persistent"], settings_report["path"].as_str().unwrap_or("memory")),
        ));
    }

    out
}

pub fn cmd_app_doctor(s: &mut Session, p: &Value) -> Result<Value> {
    let items: Vec<Value> = checks(s).into_iter().map(|c| c.to_json()).collect();
    let ok = items.iter().all(|c| c["status"] != "error");
    let mut res = json!({
        "ok": ok,
        "checks": items,
    });
    if p.get("updates").and_then(Value::as_bool).unwrap_or(false) {
        res["update"] = crate::update::cmd_app_update(s, &json!({}))?;
    }
    Ok(res)
}
