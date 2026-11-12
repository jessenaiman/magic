//! Standard ArtCraft MCP server (`magic-cli mcp` / `magic-mcp`) for Magic Studio.
//!
//! By default `magic-cli mcp` runs an isolated headless [`Session`] over stdio with zero network
//! sockets (`deckcraft/docs/mcp.md`, `cadcraft/docs/cli.md`). When `--connect <PORT|127.0.0.1:PORT>`
//! is explicitly supplied, it drives a running `magic --control <PORT>` window over IPv4 loopback.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod server;
pub use server::Server;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use magic::{AutomationRoots, Session};
use serde_json::{Value, json};

/// Default IPv4 loopback control address (`127.0.0.1:9875`).
pub const DEFAULT_CONTROL_ADDR: &str = "127.0.0.1:9875";

/// Maximum reply frame length on `--connect` (`16 MiB` including terminating `\n`, matching DeckCraft).
pub const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;

/// Normalize `"9875"` or `"localhost:9875"` to `"127.0.0.1:9875"` with zero DNS lookup.
pub fn normalize_control_addr(spec: &str) -> String {
    let trimmed = spec.trim();
    if let Some(port) = trimmed.strip_prefix("localhost:") {
        format!("127.0.0.1:{port}")
    } else if trimmed.contains(':') {
        trimmed.to_owned()
    } else {
        format!("127.0.0.1:{trimmed}")
    }
}

/// Parse and enforce strict `127.0.0.1:<1..=65535>` so tokens are never sent to `::1` or `127.0.0.x` (`wordcraft#3699`).
pub fn parse_ipv4_loopback(spec: &str) -> Result<SocketAddr, String> {
    let normalized = normalize_control_addr(spec);
    let addr: SocketAddr = normalized.parse().map_err(|_| format!("--connect requires `PORT` or `127.0.0.1:PORT`, got `{spec}`"))?;
    if addr.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) || addr.port() == 0 {
        return Err("--connect only connects to 127.0.0.1 on port 1..=65535".into());
    }
    Ok(addr)
}

/// True when `id` addresses the live `eframe`/`egui` desktop shell rather than `Session::execute`.
pub fn is_live_shell_method(id: &str) -> bool {
    id.starts_with("ui.")
        || id.starts_with("window.")
        || matches!(id, "document.inspect" | "app.open" | "app.save" | "app.quit" | "app.restart" | "app.hide" | "perf.stats" | "render.frame")
}

fn is_direct_control_method(id: &str) -> bool {
    is_live_shell_method(id) || id.starts_with("engine.") || id == "command"
}

fn is_read_only_call(id: &str, params: &Value) -> bool {
    matches!(
        id,
        "engine.commands"
            | "document.inspect"
            | "ui.inspect"
            | "ui.state"
            | "ui.drafts"
            | "ui.elements"
            | "ui.menu.list"
            | "ui.viewer.locate"
            | "ui.viewer.hit"
            | "ui.timeline.locate"
            | "ui.timeline.hit"
            | "perf.stats"
    ) || (!is_direct_control_method(id) && magic::commands::is_read_only_command(id, params))
}

/// Interpolate `"$N"`, `"$N.field"`, or `"$last.field"` step references in batch parameters.
pub fn interpolate_batch_params(params: &Value, prior_results: &[Value]) -> Value {
    match params {
        Value::String(s) => resolve_step_ref(s, prior_results).unwrap_or_else(|| Value::String(s.clone())),
        Value::Array(items) => Value::Array(items.iter().map(|v| interpolate_batch_params(v, prior_results)).collect()),
        Value::Object(map) => Value::Object(map.iter().map(|(k, v)| (k.clone(), interpolate_batch_params(v, prior_results))).collect()),
        other => other.clone(),
    }
}

fn resolve_step_ref(expr: &str, prior_results: &[Value]) -> Option<Value> {
    let rest = expr.strip_prefix('$')?;
    let (head, tail) = match rest.split_once('.') {
        Some((h, t)) => (h, Some(t)),
        None => (rest, None),
    };
    let root = if head == "last" {
        prior_results.last()?
    } else {
        let idx_1 = head.parse::<usize>().ok()?.checked_sub(1)?;
        prior_results.get(idx_1)?
    };
    let mut cur = root;
    if let Some(path) = tail {
        for seg in path.split('.') {
            cur = if let Ok(i) = seg.parse::<usize>() { cur.get(i)? } else { cur.get(seg)? };
        }
    }
    Some(cur.clone())
}

/// Execution backend for MCP tool calls.
pub trait Backend {
    fn execute(&mut self, id: &str, params: Value) -> Result<Value, String>;
    fn is_live(&self) -> bool {
        false
    }
}

/// In-process headless [`Session`] backend.
#[derive(Default)]
pub struct Headless {
    pub session: Session,
}

impl Headless {
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    pub fn with_roots(roots: Option<AutomationRoots>) -> Self {
        Self { session: Session::user_session().with_automation_roots(roots) }
    }
}

impl Backend for Headless {
    fn execute(&mut self, id: &str, params: Value) -> Result<Value, String> {
        self.session.execute(id, params).map_err(|e| e.to_string())
    }
}

/// IPv4 loopback TCP backend (`127.0.0.1:<port>`) talking to a running `magic --control <port>` window.
pub struct Remote {
    pub addr: String,
    pub token: Option<String>,
    next_id: u64,
}

impl Remote {
    pub fn new(addr: impl Into<String>, token: Option<String>) -> Self {
        Self { addr: normalize_control_addr(&addr.into()), token: token.filter(|t| !t.trim().is_empty()), next_id: 1 }
    }

    /// Connect to a live `magic --control` socket on `127.0.0.1` and verify reachability at startup.
    pub fn connect_verified(addr: impl Into<String>, token: Option<String>) -> Result<Self, String> {
        let raw = addr.into();
        let sock = parse_ipv4_loopback(&raw)?;
        let stream = TcpStream::connect_timeout(&sock, Duration::from_millis(600)).map_err(|e| e.to_string())?;
        drop(stream);
        Ok(Self::new(sock.to_string(), token))
    }
}

impl Backend for Remote {
    fn is_live(&self) -> bool {
        true
    }

    fn execute(&mut self, id: &str, params: Value) -> Result<Value, String> {
        let addr = parse_ipv4_loopback(&self.addr)?;
        let read_only = is_read_only_call(id, &params);
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|e| format!("connect {addr}: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(90))).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(60))).map_err(|e| e.to_string())?;

        let req_id = self.next_id;
        self.next_id += 1;
        let (method, wire_params) = if is_direct_control_method(id) { (id, params) } else { ("engine.execute", json!({"command": id, "params": params})) };
        let mut msg = json!({
            "id": req_id,
            "method": method,
            "params": wire_params,
        });
        if let Some(tok) = &self.token {
            msg["token"] = json!(tok);
        }
        let line = format!("{msg}\n");

        let outcome_unknown = |detail: String| -> String {
            if read_only {
                format!("{id} ({addr}): {detail}")
            } else {
                format!("outcome unknown for `{id}` ({addr}): {detail}; the command may still have run — inspect the project before retrying")
            }
        };

        stream.write_all(line.as_bytes()).map_err(|e| outcome_unknown(format!("write failed after sending started: {e}")))?;
        stream.flush().map_err(|e| outcome_unknown(format!("flush failed: {e}")))?;

        let mut reader = BufReader::new(stream);
        let mut raw_reply = Vec::new();
        let n =
            reader.by_ref().take((MAX_REPLY_BYTES + 1) as u64).read_until(b'\n', &mut raw_reply).map_err(|e| outcome_unknown(format!("read failed: {e}")))?;
        if n == 0 {
            return Err(outcome_unknown("connection closed before reply".into()));
        }
        if raw_reply.len() > MAX_REPLY_BYTES || !raw_reply.ends_with(b"\n") {
            return Err(outcome_unknown(format!("reply exceeded {MAX_REPLY_BYTES} bytes or lacked newline")));
        }
        let reply_str = std::str::from_utf8(&raw_reply).map_err(|_| outcome_unknown("reply was not valid UTF-8".into()))?;
        let parsed: Value = serde_json::from_str(reply_str.trim()).map_err(|e| outcome_unknown(format!("invalid JSON reply: {e}")))?;
        if parsed.get("id").and_then(Value::as_u64) != Some(req_id) {
            return Err(outcome_unknown(format!("mismatched reply id (expected {req_id}, got {})", parsed.get("id").unwrap_or(&Value::Null))));
        }
        match parsed.get("ok").and_then(Value::as_bool) {
            Some(true) => Ok(parsed.get("result").cloned().unwrap_or(Value::Null)),
            Some(false) => Err(parsed["error"].as_str().unwrap_or("remote command failed").to_owned()),
            None => Err(outcome_unknown("reply missing boolean `ok` field".into())),
        }
    }
}

/// Start `magic-cli mcp` / `magic-mcp` following upstream ArtCraft conventions (`deckcraft/docs/mcp.md`):
/// - `--connect <PORT|127.0.0.1:PORT>`: drive a running `magic --control` window on `127.0.0.1`.
/// - default (or `--headless`): run an isolated in-process [`Headless`] session over stdio (never silently auto-probing loopback).
pub fn run_mcp_server(
    bin_name: &str,
    connect: Option<&str>,
    headless: bool,
    read_roots: &[String],
    write_roots: &[String],
    initial_project: Option<&str>,
) -> Result<(), String> {
    if connect.is_some() && headless {
        return Err(format!("{bin_name}: pass --connect or --headless, not both"));
    }
    if connect.is_some() && (!read_roots.is_empty() || !write_roots.is_empty()) {
        return Err(format!(
            "{bin_name}: --automation-read-root and --automation-write-root confine headless mode only; pass them to `magic --control` instead"
        ));
    }
    let roots = AutomationRoots::from_args(read_roots, write_roots)?;

    if let Some(spec) = connect {
        let addr = normalize_control_addr(spec);
        let remote = Remote::connect_verified(&addr, magic::workspace::control_token()).map_err(|e| format!("{bin_name}: cannot connect to {addr}: {e}"))?;
        let _ = writeln!(std::io::stderr(), "{bin_name}: connected to live Magic window at {addr}");
        return Server::new(remote).serve();
    }

    let mut session = Session::user_session().with_automation_roots(roots);
    if let Some(path) = initial_project {
        session.execute("file.open", json!({ "path": path })).map_err(|e| e.to_string())?;
    }
    Server::new(Headless::new(session)).serve()
}

#[cfg(test)]
mod tests {
    use super::{Backend, Remote, interpolate_batch_params, normalize_control_addr, parse_ipv4_loopback};
    use serde_json::json;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    #[test]
    fn normalizes_strict_ipv4_loopback_and_interpolates_batch_refs() {
        assert_eq!(normalize_control_addr("9875"), "127.0.0.1:9875");
        assert_eq!(normalize_control_addr("localhost:9875"), "127.0.0.1:9875");
        assert!(parse_ipv4_loopback("127.0.0.1:9875").is_ok());
        assert!(parse_ipv4_loopback("[::1]:9875").is_err());
        assert!(parse_ipv4_loopback("127.0.0.2:9875").is_err());
        let prior = vec![json!({"project": "/tmp/demo.magic", "items": [{"id": "hero"}]})];
        let resolved = interpolate_batch_params(&json!({"p": "$1.project", "who": "$last.items.0.id"}), &prior);
        assert_eq!(resolved["p"], "/tmp/demo.magic");
        assert_eq!(resolved["who"], "hero");
    }

    #[test]
    fn lost_mutation_reply_reports_outcome_unknown_without_replay() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let mut count = 0usize;
            if let Ok((stream, _)) = listener.accept() {
                let mut r = BufReader::new(stream);
                let mut line = String::new();
                let _ = r.read_line(&mut line);
                count += 1;
            }
            count
        });
        let mut remote = Remote::new(addr.to_string(), None);
        let err = remote.execute("project.autosave", json!({})).unwrap_err();
        assert!(err.contains("outcome unknown"));
        assert_eq!(handle.join().unwrap(), 1);
    }
}
