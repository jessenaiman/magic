//! Host ArtCraft application bridge (`crates/engine/src/craft_api.rs`):
//! connects Magic Studio to the five host Craft applications required for video production —
//! **`DesignCraft`**, **`PhotoCraft`**, **`VectorCraft`**, **`EffectCraft`**, and **`FilmCraft`** —
//! through each application's live `--control` JSON-lines TCP API (`127.0.0.1:<port>`) and its
//! installed `<app>-cli` binary.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Value, json};

const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_millis(90);
const PROBE_READ_TIMEOUT: Duration = Duration::from_millis(250);
const MUTATE_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const MUTATE_READ_TIMEOUT: Duration = Duration::from_millis(5000);

/// Canonical host Craft applications coordinated by Magic Studio:
/// `(app_slug, cli_binary, default_control_port, port_env, token_env, badge)`.
pub const HOST_CRAFT_APPS: &[(&str, &str, u16, &str, &str, &str)] = &[
    ("designcraft", "designcraft-cli", 7979, "DESIGNCRAFT_CONTROL_PORT", "DESIGNCRAFT_CONTROL_TOKEN", "DC"),
    ("photocraft", "photocraft-cli", 7878, "PHOTOCRAFT_CONTROL_PORT", "PHOTOCRAFT_CONTROL_TOKEN", "PC"),
    ("vectorcraft", "vectorcraft-cli", 7980, "VECTORCRAFT_CONTROL_PORT", "VECTORCRAFT_CONTROL_TOKEN", "VC"),
    ("effectcraft", "effectcraft-cli", 9877, "EFFECTCRAFT_CONTROL_PORT", "EFFECTCRAFT_CONTROL_TOKEN", "EC"),
    ("filmcraft", "filmcraft-cli", 9876, "FILMCRAFT_CONTROL_PORT", "FILMCRAFT_CONTROL_TOKEN", "FC"),
];

struct CachedProbe {
    effectcraft: Option<Vec<Value>>,
    vectorcraft: Option<Vec<Value>>,
    filmcraft: Option<Vec<Value>>,
    endpoints: Value,
}

static CACHE: OnceLock<Mutex<Option<CachedProbe>>> = OnceLock::new();

/// Documented `--control` port, CLI name, and optional bearer-token env var per host Craft app.
pub fn app_control_spec(app: &str) -> Option<(u16, &'static str, &'static str, &'static str)> {
    match app.trim().to_ascii_lowercase().as_str() {
        "designcraft" | "dc" => Some((7979, "DESIGNCRAFT_CONTROL_PORT", "DESIGNCRAFT_CONTROL_TOKEN", "DC")),
        "photocraft" | "pc" => Some((7878, "PHOTOCRAFT_CONTROL_PORT", "PHOTOCRAFT_CONTROL_TOKEN", "PC")),
        "vectorcraft" | "vc" => Some((7980, "VECTORCRAFT_CONTROL_PORT", "VECTORCRAFT_CONTROL_TOKEN", "VC")),
        "effectcraft" | "ec" => Some((9877, "EFFECTCRAFT_CONTROL_PORT", "EFFECTCRAFT_CONTROL_TOKEN", "EC")),
        "filmcraft" | "fc" => Some((9876, "FILMCRAFT_CONTROL_PORT", "FILMCRAFT_CONTROL_TOKEN", "FC")),
        _ => None,
    }
}

/// Canonical `<app>-cli` executable name for a host Craft application.
pub fn app_cli_name(app: &str) -> Option<&'static str> {
    match app.trim().to_ascii_lowercase().as_str() {
        "designcraft" | "dc" => Some("designcraft-cli"),
        "photocraft" | "pc" => Some("photocraft-cli"),
        "vectorcraft" | "vc" => Some("vectorcraft-cli"),
        "effectcraft" | "ec" => Some("effectcraft-cli"),
        "filmcraft" | "fc" => Some("filmcraft-cli"),
        _ => None,
    }
}

/// Resolve strict `127.0.0.1:<port>` control socket address and optional bearer token for `app`.
pub fn resolve_endpoint(app: &str) -> Option<(SocketAddr, Option<String>, &'static str)> {
    let (default_port, port_env, token_env, badge) = app_control_spec(app)?;
    let port = std::env::var(port_env)
        .ok()
        .and_then(|raw| {
            let trimmed = raw.trim();
            let suffix = trimmed.strip_prefix("127.0.0.1:").or_else(|| trimmed.strip_prefix("localhost:")).unwrap_or(trimmed);
            suffix.parse::<u16>().ok().filter(|p| *p > 0)
        })
        .unwrap_or(default_port);
    let token = std::env::var(token_env).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    Some((addr, token, badge))
}

fn default_endpoints_json() -> Value {
    let mut map = serde_json::Map::new();
    for (app, cli, _, _, _, _) in HOST_CRAFT_APPS {
        let entry = match resolve_endpoint(app) {
            Some((addr, token, badge)) => json!({
                "app": app,
                "cli": cli,
                "cli_installed": crate::workspace::resolve_executable_path(cli).is_some(),
                "badge": badge,
                "endpoint": addr.to_string(),
                "connected": false,
                "token_configured": token.is_some(),
            }),
            None => json!({"app": app, "connected": false}),
        };
        map.insert((*app).to_owned(), entry);
    }
    Value::Object(map)
}

fn send_json_line(
    addr: SocketAddr,
    token: Option<&str>,
    method: &str,
    params: Value,
    connect_timeout: Duration,
    io_timeout: Duration,
) -> std::result::Result<Value, String> {
    if addr.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err("refusing non-loopback control address".into());
    }
    let mut stream = TcpStream::connect_timeout(&addr, connect_timeout).map_err(|e| format!("{addr} unreachable: {e}"))?;
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(io_timeout));
    let _ = stream.set_write_timeout(Some(io_timeout));

    let mut req = json!({
        "id": 1,
        "method": method,
        "params": params,
    });
    if let Some(tok) = token
        && let Some(obj) = req.as_object_mut()
    {
        obj.insert("token".into(), json!(tok));
    }
    writeln!(stream, "{req}").and_then(|_| stream.flush()).map_err(|e| format!("write {addr}: {e}"))?;

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let n = reader.by_ref().take((MAX_REPLY_BYTES + 1) as u64).read_until(b'\n', &mut line).map_err(|e| format!("read {addr}: {e}"))?;
    if n == 0 || line.len() > MAX_REPLY_BYTES {
        return Err(format!("empty or oversized reply from {addr}"));
    }
    let reply: Value = serde_json::from_slice(&line).map_err(|e| format!("invalid JSON from {addr}: {e}"))?;
    if reply.get("ok").and_then(Value::as_bool) == Some(false) {
        let err = reply.get("error").and_then(Value::as_str).unwrap_or("remote Craft control error");
        return Err(err.to_owned());
    }
    Ok(reply.get("result").cloned().unwrap_or(reply))
}

/// Probe whether a host Craft desktop app is listening on its `--control` port and whether its CLI is installed.
pub fn probe_live_app(app: &str) -> Value {
    let Some((addr, token, badge)) = resolve_endpoint(app) else {
        return json!({"app": app, "connected": false, "error": "unknown Craft app"});
    };
    let cli = app_cli_name(app).unwrap_or_default();
    let cli_path = crate::workspace::resolve_executable_path(cli);
    let probe_res = send_json_line(addr, token.as_deref(), "engine.commands", json!({}), PROBE_CONNECT_TIMEOUT, PROBE_READ_TIMEOUT);
    match probe_res {
        Ok(res) => {
            let cmd_count = res.get("commands").and_then(Value::as_array).map_or_else(|| res.as_array().map_or(0, Vec::len), Vec::len);
            json!({
                "app": app,
                "cli": cli,
                "cli_installed": cli_path.is_some(),
                "cli_path": cli_path,
                "badge": badge,
                "endpoint": addr.to_string(),
                "connected": true,
                "token_configured": token.is_some(),
                "commands_count": cmd_count,
            })
        }
        Err(reason) => json!({
            "app": app,
            "cli": cli,
            "cli_installed": cli_path.is_some(),
            "cli_path": cli_path,
            "badge": badge,
            "endpoint": addr.to_string(),
            "connected": false,
            "token_configured": token.is_some(),
            "hint": format!("Run `{app} --control {}` for live API access or ensure `{cli}` is on PATH", addr.port()),
            "detail": reason,
        }),
    }
}

/// Invoke any raw control method (`engine.execute`, `ui.render`, `document.inspect`, etc.) on a running host Craft app.
pub fn call_control_method(app: &str, method: &str, params: Value) -> std::result::Result<Value, String> {
    let (addr, token, _) = resolve_endpoint(app).ok_or_else(|| format!("unsupported Craft app `{app}`"))?;
    send_json_line(addr, token.as_deref(), method, params, MUTATE_CONNECT_TIMEOUT, MUTATE_READ_TIMEOUT)
}

/// Execute an engine command on a running host Craft app via `--control`, falling back to `<app>-cli` on the host machine.
pub fn execute_on_host_app(app: &str, doc_path: Option<&Path>, command: &str, params: Value) -> std::result::Result<Value, String> {
    if let Ok(live) = execute_in_live_app(app, command, params.clone()) {
        return Ok(json!({"transport": "control", "app": app, "result": live}));
    }
    let cli = app_cli_name(app).ok_or_else(|| format!("unknown Craft app `{app}`"))?;
    let exe = crate::workspace::resolve_executable_path(cli).ok_or_else(|| format!("`{app}` is not running on --control and `{cli}` was not found on PATH"))?;
    let mut cmd = Command::new(exe);
    if let Some(doc) = doc_path {
        cmd.arg(doc);
    }
    cmd.arg("run").arg(command).arg(params.to_string());
    let out = cmd.output().map_err(|e| format!("failed to execute `{cli}`: {e}"))?;
    if !out.status.success() {
        return Err(format!("`{cli}` exited with {}: {}", out.status, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = serde_json::from_str::<Value>(stdout.trim()).unwrap_or_else(|_| json!({"stdout": stdout.trim()}));
    Ok(json!({"transport": "cli", "app": app, "cli": cli, "result": parsed}))
}

/// Run `<app>-cli` with explicit arguments on the host machine (for `--export`, `script`, or batch commands).
pub fn run_host_cli(app: &str, args: &[&str]) -> std::result::Result<Value, String> {
    let cli = app_cli_name(app).ok_or_else(|| format!("unknown Craft app `{app}`"))?;
    let exe = crate::workspace::resolve_executable_path(cli).ok_or_else(|| format!("`{cli}` is not installed on PATH (configure `[tools]` in config.toml)"))?;
    let out = Command::new(exe).args(args).output().map_err(|e| format!("failed to spawn `{cli}`: {e}"))?;
    if !out.status.success() {
        return Err(format!("`{cli}` failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = serde_json::from_str::<Value>(stdout.trim()).unwrap_or_else(|_| json!({"stdout": stdout.trim()}));
    Ok(parsed)
}

fn normalize_live_effect_list(app: &str, badge: &str, raw: &Value) -> Vec<Value> {
    let arr_opt = raw
        .as_array()
        .or_else(|| raw.get("effects").and_then(Value::as_array))
        .or_else(|| raw.pointer("/data/effects").and_then(Value::as_array))
        .or_else(|| raw.get("data").and_then(Value::as_array));
    let Some(items) = arr_opt else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let match_name = item.get("id").or_else(|| item.get("match_name")).or_else(|| item.get("matchName")).and_then(Value::as_str)?;
            let label = item.get("name").or_else(|| item.get("label")).and_then(Value::as_str).unwrap_or(match_name);
            let category = item.get("category").or_else(|| item.get("folder")).and_then(Value::as_str).unwrap_or("Live API");
            let craft_cmd = if app == "filmcraft" { "effects.apply" } else { "effect.apply" };
            Some(json!({
                "id": match_name,
                "badge": badge,
                "craft_app": app,
                "craft_command": craft_cmd,
                "match_name": match_name,
                "category": category,
                "label": label,
                "summary": format!("Live {app} `--control` API effect ({category})"),
                "params": {},
                "cli_available": true,
                "live_api": true,
            }))
        })
        .collect()
}

fn query_live_effects_uncached(app: &str) -> Option<Vec<Value>> {
    let (addr, token, badge) = resolve_endpoint(app)?;
    let list_cmd = if app == "filmcraft" { "effects.list" } else { "effect.list" };
    let res = send_json_line(addr, token.as_deref(), "engine.execute", json!({"command": list_cmd, "params": {}}), PROBE_CONNECT_TIMEOUT, PROBE_READ_TIMEOUT)
        .or_else(|_| send_json_line(addr, token.as_deref(), list_cmd, json!({}), PROBE_CONNECT_TIMEOUT, PROBE_READ_TIMEOUT))
        .ok()?;
    let parsed = normalize_live_effect_list(app, badge, &res);
    if parsed.is_empty() { None } else { Some(parsed) }
}

type LiveCraftSnapshot = (Value, Option<Vec<Value>>, Option<Vec<Value>>, Option<Vec<Value>>);

pub fn cached_live_snapshot() -> LiveCraftSnapshot {
    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = lock.lock()
        && let Some(cached) = guard.as_ref()
    {
        return (cached.endpoints.clone(), cached.effectcraft.clone(), cached.vectorcraft.clone(), cached.filmcraft.clone());
    }
    (default_endpoints_json(), None, None, None)
}

pub fn live_craft_snapshot(force_refresh: bool) -> LiveCraftSnapshot {
    if !force_refresh {
        return cached_live_snapshot();
    }
    let dc_probe = probe_live_app("designcraft");
    let pc_probe = probe_live_app("photocraft");
    let vc_probe = probe_live_app("vectorcraft");
    let ec_probe = probe_live_app("effectcraft");
    let fc_probe = probe_live_app("filmcraft");

    let ec_fx = if ec_probe["connected"] == true { query_live_effects_uncached("effectcraft") } else { None };
    let vc_fx = if vc_probe["connected"] == true { query_live_effects_uncached("vectorcraft") } else { None };
    let fc_fx = if fc_probe["connected"] == true { query_live_effects_uncached("filmcraft") } else { None };

    let endpoints = json!({
        "designcraft": dc_probe,
        "photocraft": pc_probe,
        "vectorcraft": vc_probe,
        "effectcraft": ec_probe,
        "filmcraft": fc_probe,
    });

    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(CachedProbe { effectcraft: ec_fx.clone(), vectorcraft: vc_fx.clone(), filmcraft: fc_fx.clone(), endpoints: endpoints.clone() });
    }
    (endpoints, ec_fx, vc_fx, fc_fx)
}

pub fn execute_in_live_app(app: &str, command: &str, params: Value) -> std::result::Result<Value, String> {
    let (addr, token, _) = resolve_endpoint(app).ok_or_else(|| format!("unsupported Craft app `{app}`"))?;
    send_json_line(addr, token.as_deref(), "engine.execute", json!({"command": command, "params": params}), MUTATE_CONNECT_TIMEOUT, MUTATE_READ_TIMEOUT)
        .or_else(|_| send_json_line(addr, token.as_deref(), command, params, MUTATE_CONNECT_TIMEOUT, MUTATE_READ_TIMEOUT))
}

pub fn save_in_live_app(app: &str, path: Option<&Path>) -> std::result::Result<Value, String> {
    let save_params = match path {
        Some(p) => json!({"path": p}),
        None => json!({}),
    };
    execute_in_live_app(app, "file.save", save_params.clone()).or_else(|_| execute_in_live_app(app, "project.save", save_params))
}

pub fn cmd_craft_api_status(_s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let force = p.get("refresh").and_then(Value::as_bool).unwrap_or(true);
    let (endpoints, ec_fx, vc_fx, fc_fx) = live_craft_snapshot(force);
    Ok(json!({
        "ok": true,
        "endpoints": endpoints,
        "live_effect_counts": {
            "effectcraft": ec_fx.as_ref().map_or(0, Vec::len),
            "vectorcraft": vc_fx.as_ref().map_or(0, Vec::len),
            "filmcraft": fc_fx.as_ref().map_or(0, Vec::len),
        }
    }))
}

pub fn cmd_craft_live_save(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let app = p.get("app").and_then(Value::as_str).unwrap_or("effectcraft");
    let resolved = p.get("path").and_then(Value::as_str).filter(|r| !r.trim().is_empty()).map(|raw| crate::workspace::resolve_workspace_path(s, raw));
    match save_in_live_app(app, resolved.as_deref()) {
        Ok(reply) => Ok(json!({"ok": true, "app": app, "saved_via_control": true, "reply": reply})),
        Err(detail) => Ok(json!({
            "ok": false,
            "app": app,
            "saved_via_control": false,
            "detail": detail,
        })),
    }
}
