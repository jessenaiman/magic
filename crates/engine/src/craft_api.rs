//! Pure-Rust loopback `--control` JSON-lines client (`127.0.0.1:<port>`) for querying live Craft
//! libraries and triggering each running Craft desktop app's own built-in `file.save`.
//!
//! Architecture alignment (`craft-libs/README.md`, `effectcraft/docs/control-protocol.md`,
//! `filmcraft/docs/control-protocol.md`, `vectorcraft/docs/control-protocol.md`):
//! - Magic Studio holds **zero** `Cargo.toml` dependencies on sibling Craft app crates and never
//!   spawns duplicate background app instances.
//! - External AI agents connect to the user's open Craft apps with each app's own MCP bridge
//!   (`<app>-cli mcp --bridge <port>`), while Magic queries live effect registries, applies live
//!   layer/text edits, and invokes the open app's built-in atomic `file.save` over the same
//!   `127.0.0.1` `--control` socket.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const MAX_REPLY_BYTES: usize = 8 * 1024 * 1024;
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_millis(95);
const PROBE_READ_TIMEOUT: Duration = Duration::from_millis(240);
const MUTATE_CONNECT_TIMEOUT: Duration = Duration::from_millis(180);
const MUTATE_READ_TIMEOUT: Duration = Duration::from_millis(1800);
const CACHE_TTL: Duration = Duration::from_secs(4);

struct CachedProbe {
    captured_at: Instant,
    effectcraft: Option<Vec<Value>>,
    vectorcraft: Option<Vec<Value>>,
    filmcraft: Option<Vec<Value>>,
    endpoints: Value,
}

/// Default documented `--control` port and optional bearer-token env var per Craft app.
pub fn app_control_spec(app: &str) -> Option<(u16, &'static str, &'static str, &'static str)> {
    match app.trim().to_ascii_lowercase().as_str() {
        "effectcraft" | "ec" => Some((9877, "EFFECTCRAFT_CONTROL_PORT", "EFFECTCRAFT_CONTROL_TOKEN", "EC")),
        "vectorcraft" | "vc" => Some((7979, "VECTORCRAFT_CONTROL_PORT", "VECTORCRAFT_CONTROL_TOKEN", "VC")),
        "filmcraft" | "fc" => Some((9876, "FILMCRAFT_CONTROL_PORT", "FILMCRAFT_CONTROL_TOKEN", "FC")),
        "photocraft" | "pc" => Some((9878, "PHOTOCRAFT_CONTROL_PORT", "PHOTOCRAFT_CONTROL_TOKEN", "PC")),
        "designcraft" | "dc" => Some((9879, "DESIGNCRAFT_CONTROL_PORT", "DESIGNCRAFT_CONTROL_TOKEN", "DC")),
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
            let suffix = trimmed
                .strip_prefix("127.0.0.1:")
                .or_else(|| trimmed.strip_prefix("localhost:"))
                .unwrap_or(trimmed);
            suffix.parse::<u16>().ok().filter(|p| *p > 0)
        })
        .unwrap_or(default_port);
    let token = std::env::var(token_env)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    Some((addr, token, badge))
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
    let mut stream = TcpStream::connect_timeout(&addr, connect_timeout)
        .map_err(|e| format!("{addr} unreachable: {e}"))?;
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
    let n = reader
        .by_ref()
        .take((MAX_REPLY_BYTES + 1) as u64)
        .read_until(b'\n', &mut line)
        .map_err(|e| format!("read {addr}: {e}"))?;
    if n == 0 || line.len() > MAX_REPLY_BYTES {
        return Err(format!("empty or oversized reply from {addr}"));
    }
    let reply: Value = serde_json::from_slice(&line).map_err(|e| format!("invalid JSON from {addr}: {e}"))?;
    if reply.get("ok").and_then(Value::as_bool) == Some(false) {
        let err = reply
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("remote Craft control error");
        return Err(err.to_owned());
    }
    Ok(reply.get("result").cloned().unwrap_or(reply))
}

/// Probe whether a local Craft desktop app is currently listening on its `--control` port.
pub fn probe_live_app(app: &str) -> Value {
    let Some((addr, token, badge)) = resolve_endpoint(app) else {
        return json!({"app": app, "connected": false, "error": "unknown Craft app"});
    };
    let probe_res = send_json_line(
        addr,
        token.as_deref(),
        "engine.commands",
        json!({}),
        PROBE_CONNECT_TIMEOUT,
        PROBE_READ_TIMEOUT,
    )
    .or_else(|_| {
        send_json_line(
            addr,
            token.as_deref(),
            "ui.inspect",
            json!({}),
            PROBE_CONNECT_TIMEOUT,
            PROBE_READ_TIMEOUT,
        )
    });
    match probe_res {
        Ok(res) => {
            let cmd_count = res
                .get("commands")
                .and_then(Value::as_array)
                .map_or_else(|| res.as_array().map_or(0, Vec::len), Vec::len);
            json!({
                "app": app,
                "badge": badge,
                "endpoint": addr.to_string(),
                "connected": true,
                "token_configured": token.is_some(),
                "commands_count": cmd_count,
            })
        }
        Err(reason) => json!({
            "app": app,
            "badge": badge,
            "endpoint": addr.to_string(),
            "connected": false,
            "token_configured": token.is_some(),
            "hint": format!("Launch `{app} --control {}` so Magic and `{app}-cli mcp --bridge {}` share your live open session", addr.port(), addr.port()),
            "detail": reason,
        }),
    }
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
            let match_name = item
                .get("id")
                .or_else(|| item.get("match_name"))
                .or_else(|| item.get("matchName"))
                .and_then(Value::as_str)?;
            let label = item
                .get("name")
                .or_else(|| item.get("label"))
                .and_then(Value::as_str)
                .unwrap_or(match_name);
            let category = item
                .get("category")
                .or_else(|| item.get("folder"))
                .and_then(Value::as_str)
                .unwrap_or("Live API");
            let craft_cmd = if app == "filmcraft" { "clip.effect.add" } else { "effect.apply" };
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
    let res = send_json_line(
        addr,
        token.as_deref(),
        "engine.execute",
        json!({"command": list_cmd, "params": {}}),
        PROBE_CONNECT_TIMEOUT,
        PROBE_READ_TIMEOUT,
    )
    .or_else(|_| {
        send_json_line(
            addr,
            token.as_deref(),
            list_cmd,
            json!({}),
            PROBE_CONNECT_TIMEOUT,
            PROBE_READ_TIMEOUT,
        )
    })
    .ok()?;
    let parsed = normalize_live_effect_list(app, badge, &res);
    if parsed.is_empty() { None } else { Some(parsed) }
}

/// Snapshot of live `--control` endpoints + any live-queried effects, cached briefly (`4s`) so UI
/// redraws stay instantaneous unless `force_refresh` is requested.
pub fn live_craft_snapshot(force_refresh: bool) -> (Value, Option<Vec<Value>>, Option<Vec<Value>>, Option<Vec<Value>>) {
    static CACHE: OnceLock<Mutex<Option<CachedProbe>>> = OnceLock::new();
    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if !force_refresh
        && let Ok(guard) = lock.lock()
        && let Some(cached) = guard.as_ref()
        && cached.captured_at.elapsed() < CACHE_TTL
    {
        return (
            cached.endpoints.clone(),
            cached.effectcraft.clone(),
            cached.vectorcraft.clone(),
            cached.filmcraft.clone(),
        );
    }

    let ec_probe = probe_live_app("effectcraft");
    let vc_probe = probe_live_app("vectorcraft");
    let fc_probe = probe_live_app("filmcraft");

    let ec_fx = if ec_probe["connected"] == true { query_live_effects_uncached("effectcraft") } else { None };
    let vc_fx = if vc_probe["connected"] == true { query_live_effects_uncached("vectorcraft") } else { None };
    let fc_fx = if fc_probe["connected"] == true { query_live_effects_uncached("filmcraft") } else { None };

    let endpoints = json!({
        "effectcraft": ec_probe,
        "vectorcraft": vc_probe,
        "filmcraft": fc_probe,
    });

    if let Ok(mut guard) = lock.lock() {
        *guard = Some(CachedProbe {
            captured_at: Instant::now(),
            effectcraft: ec_fx.clone(),
            vectorcraft: vc_fx.clone(),
            filmcraft: fc_fx.clone(),
            endpoints: endpoints.clone(),
        });
    }
    (endpoints, ec_fx, vc_fx, fc_fx)
}

/// Execute an engine command directly on the user's running local Craft desktop app via `--control`.
pub fn execute_in_live_app(app: &str, command: &str, params: Value) -> std::result::Result<Value, String> {
    let (addr, token, _) = resolve_endpoint(app).ok_or_else(|| format!("unsupported Craft app `{app}`"))?;
    send_json_line(
        addr,
        token.as_deref(),
        "engine.execute",
        json!({"command": command, "params": params}),
        MUTATE_CONNECT_TIMEOUT,
        MUTATE_READ_TIMEOUT,
    )
    .or_else(|_| {
        send_json_line(
            addr,
            token.as_deref(),
            command,
            params,
            MUTATE_CONNECT_TIMEOUT,
            MUTATE_READ_TIMEOUT,
        )
    })
}

/// Trigger the running Craft desktop app's own built-in atomic `file.save` over `--control` so
/// Magic Studio and any connected `<app>-cli mcp --bridge` agent share the exact same saved file.
pub fn save_in_live_app(app: &str, path: Option<&Path>) -> std::result::Result<Value, String> {
    let save_params = match path {
        Some(p) => json!({"path": p}),
        None => json!({}),
    };
    execute_in_live_app(app, "file.save", save_params.clone())
        .or_else(|_| execute_in_live_app(app, "project.save", save_params))
}

/// `craft.api_status`: inspect live `--control` API reachability across EffectCraft (`:9877`),
/// VectorCraft (`:7979`), and FilmCraft (`:9876`) without spawning duplicate processes.
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

/// `craft.live_save`: invoke built-in `file.save` inside the user's open local Craft app over `--control`.
pub fn cmd_craft_live_save(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let app = p.get("app").and_then(Value::as_str).unwrap_or("effectcraft");
    let target_path = p
        .get("path")
        .and_then(Value::as_str)
        .filter(|raw| !raw.trim().is_empty())
        .map(|raw| crate::workspace::resolve_workspace_path(s, raw));
    match save_in_live_app(app, target_path.as_deref()) {
        Ok(res) => Ok(json!({
            "ok": true,
            "app": app,
            "saved_via_live_control": true,
            "result": res,
        })),
        Err(err) => {
            let (addr_str, port_num) = resolve_endpoint(app)
                .map_or_else(|| ("127.0.0.1:9877".to_owned(), 9877), |(a, _, _)| (a.to_string(), a.port()));
            Err(crate::EngineError::InvalidParams(format!(
                "{app} is not connected on {addr_str} ({err}). Launch `{app} --control {port_num}` to use its built-in live save."
            )))
        }
    }
}
