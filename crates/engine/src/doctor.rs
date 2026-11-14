//! `app.doctor` (`magic-cli app doctor`): verify host Craft applications (`DesignCraft`,
//! `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft` via live `--control` API or `<app>-cli`),
//! `craft-ai-core` configuration, and `craft-settings` health.

use serde_json::{Value, json};

use crate::workspace::{CRAFT_APPS, control_probe, resolve_executable_path};
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

    for (app, cli, port) in CRAFT_APPS {
        let live = control_probe(app);
        let cli_ok = resolve_executable_path(cli).is_some();
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

    let anicraft_cli_ok = resolve_executable_path("anicraft-cli").is_some();
    if anicraft_cli_ok {
        out.push(Check::ok("host:anicraft", "anicraft ready (anicraft-cli installed for stdio serve/mcp)"));
    } else {
        out.push(Check::problem(
            "warn",
            "host:anicraft",
            "`anicraft-cli` is not installed on PATH",
            "Install AniCraft (`anicraft-cli` for stdio `serve` / `mcp` / `render`) when using animated vector sprites",
        ));
    }

    let deckcraft_cli_ok = resolve_executable_path("deckcraft-cli").is_some();
    if deckcraft_cli_ok {
        out.push(Check::ok("host:deckcraft", "deckcraft ready (deckcraft-cli installed for Storyboard decks)"));
    } else {
        out.push(Check::problem(
            "warn",
            "host:deckcraft",
            "`deckcraft-cli` is not installed on PATH",
            "Install DeckCraft (`deckcraft-cli` for Storyboard decks and presentation slide templates)",
        ));
    }

    let craft_clis = ["designcraft-cli", "photocraft-cli", "vectorcraft-cli", "effectcraft-cli", "anicraft-cli", "filmcraft-cli", "deckcraft-cli"];
    let installed_clis: Vec<&str> = craft_clis.iter().copied().filter(|cli| resolve_executable_path(cli).is_some()).collect();
    out.push(Check::ok(
        "cli:discovery",
        format!("{}/{} Craft CLIs installed on PATH: [{}]", installed_clis.len(), craft_clis.len(), installed_clis.join(", ")),
    ));

    let gate = crate::workspace::craft_agent_gate();
    out.push(Check::ok(
        "gate:craft-flags-and-agents",
        format!(
            "craft-flags: ai_mcp={}, ai_models={}; craft-ai-agents: switched_on={}, magic_allowed={}",
            gate["ai_mcp"], gate["ai_models"], gate["agents_switched_on"], gate["magic_allowed"]
        ),
    ));

    let font_count = craft_font_catalog::Catalog::global().families().len();
    out.push(Check::ok("fonts:craft-font-catalog", format!("{font_count} font families indexed in craft-font-catalog")));

    let ai = crate::craft_ai::craft_ai_status();
    let ai_enabled = ai["enabled"].as_bool().unwrap_or(false);
    let conn_count = ai["connections"].as_array().map_or(0, Vec::len);
    let store_path = ai["storage"]["path"].as_str().unwrap_or("in-memory");
    out.push(Check::ok(
        "ai:craft-ai-core",
        format!("ai.enabled={ai_enabled}, {conn_count} connection(s) configured via craft-ai-core (storage: {store_path})"),
    ));

    let tools_summary = match craft_ai_client::AppTools::standard("magic") {
        Ok(tools) => {
            let reports = tools.report();
            let ready_count = reports.iter().filter(|r| matches!(r.status, craft_ai_client::RoleStatus::Ready(_))).count();
            format!("{} tool(s) registered, {} ready", reports.len(), ready_count)
        }
        Err(e) => format!("in-memory / fallback: {e}"),
    };
    out.push(Check::ok("ai:craft-ai-client", tools_summary));

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
