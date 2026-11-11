//! Lightweight ArtCraft MCP server (`magic-cli mcp` / `magic-mcp`) for Magic Studio.
//!
//! Magic is a lightweight coordinator shell — NOT a frame-rendering or timeline timing server.
//! Timeline-to-storyboard and audio-to-frame precision live in `FilmCraft` (`filmcraft-cli`),
//! `EffectCraft` (`effectcraft-cli`), and `DesignCraft` (`designcraft-cli`), which `magic::Session`
//! invokes via `CraftHub` (`<craft>-cli`) or live `--control` bridges.
//!
//! Reference:
//! - <https://github.com/storytold/vectorcraft/blob/main/README.md>
//! - <https://github.com/storytold/designcraft/blob/main/docs/mcp.md>

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use magic::{AutomationRoots, Session};
use serde_json::{Value, json};

/// Default loopback control port used when `MAGIC_CONTROL_PORT` is unset.
pub const DEFAULT_CONTROL_ADDR: &str = "127.0.0.1:9876";

/// Normalize `"9876"` -> `"127.0.0.1:9876"` or keep an explicit `"127.0.0.1:9876"`.
pub fn normalize_control_addr(spec: &str) -> String {
    let trimmed = spec.trim();
    if trimmed.contains(':') {
        trimmed.to_owned()
    } else {
        format!("127.0.0.1:{trimmed}")
    }
}

/// Default control address for auto-discovery (`MAGIC_CONTROL_PORT` if set, else `127.0.0.1:9876`).
pub fn default_control_addr() -> String {
    std::env::var("MAGIC_CONTROL_PORT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| normalize_control_addr(&s))
        .unwrap_or_else(|| DEFAULT_CONTROL_ADDR.to_owned())
}

/// True when `id` addresses the live `eframe`/`egui` desktop shell rather than `Session::execute`.
pub fn is_live_shell_method(id: &str) -> bool {
    id.starts_with("ui.")
        || id.starts_with("window.")
        || matches!(id, "app.quit" | "app.restart" | "app.hide" | "perf.stats")
}

fn is_direct_control_method(id: &str) -> bool {
    is_live_shell_method(id) || id.starts_with("engine.")
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
        Self {
            session: Session::user_session().with_automation_roots(roots),
        }
    }
}

impl Backend for Headless {
    fn execute(&mut self, id: &str, params: Value) -> Result<Value, String> {
        self.session.execute(id, params).map_err(|e| e.to_string())
    }
}

/// Loopback TCP backend talking to a running `magic --control <port>` window.
pub struct Remote {
    pub addr: String,
    pub token: Option<String>,
    next_id: u64,
}

impl Remote {
    pub fn new(addr: impl Into<String>, token: Option<String>) -> Self {
        Self {
            addr: normalize_control_addr(&addr.into()),
            token,
            next_id: 1,
        }
    }

    /// Connect to a live `magic --control` socket and verify reachability at server startup.
    pub fn connect_verified(addr: impl Into<String>, token: Option<String>) -> Result<Self, String> {
        let normalized = normalize_control_addr(&addr.into());
        let sock: SocketAddr = normalized
            .parse()
            .map_err(|_| format!("invalid loopback control address `{normalized}`"))?;
        if !sock.ip().is_loopback() || sock.port() == 0 {
            return Err("control address must be loopback (127.0.0.1 or ::1) with port 1..=65535".into());
        }
        let stream = TcpStream::connect_timeout(&sock, Duration::from_millis(600)).map_err(|e| e.to_string())?;
        drop(stream);
        let tok = token.ok_or_else(|| "control token missing (~/.config/magic/control-token or MAGIC_CONTROL_TOKEN)".to_owned())?;
        if tok.len() != 64 || !tok.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("MAGIC_CONTROL_TOKEN must contain 64 hexadecimal characters".into());
        }
        Ok(Self {
            addr: normalized,
            token: Some(tok),
            next_id: 1,
        })
    }
}

impl Backend for Remote {
    fn is_live(&self) -> bool {
        true
    }

    fn execute(&mut self, id: &str, params: Value) -> Result<Value, String> {
        let addr: SocketAddr = self.addr.parse().map_err(|_| "--connect requires a loopback IP:port")?;
        if !addr.ip().is_loopback() || addr.port() == 0 {
            return Err("control address must be loopback with port 1..=65535".into());
        }
        let token = self.token.as_deref().ok_or("MAGIC_CONTROL_TOKEN (or ~/.config/magic/control-token) is required for live --connect")?;
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("MAGIC_CONTROL_TOKEN must contain 64 hexadecimal characters".into());
        }
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|e| format!("connect {}: {e}", self.addr))?;
        stream.set_read_timeout(Some(Duration::from_secs(300))).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(120))).map_err(|e| e.to_string())?;
        let req_id = self.next_id;
        self.next_id += 1;
        let (method, params) = if is_direct_control_method(id) {
            (id, params)
        } else {
            ("engine.execute", json!({"command": id, "params": params}))
        };
        let mut msg = json!({
            "id": req_id,
            "method": method,
            "params": params,
        });
        if let Some(tok) = &self.token {
            msg["token"] = json!(tok);
        }
        let line = format!("{msg}\n");
        stream.write_all(line.as_bytes()).map_err(|e| format!("write {}: {e}", self.addr))?;
        stream.flush().map_err(|e| format!("flush {}: {e}", self.addr))?;
        let mut reader = BufReader::new(stream);
        let mut reply = String::new();
        reader.read_line(&mut reply).map_err(|e| format!("read {}: {e}", self.addr))?;
        let parsed: Value = serde_json::from_str(reply.trim()).map_err(|e| format!("invalid reply: {e}"))?;
        if parsed["ok"] == true {
            Ok(parsed.get("result").cloned().unwrap_or(parsed))
        } else {
            Err(parsed["error"].as_str().unwrap_or("remote command failed").to_owned())
        }
    }
}

/// Start `magic-cli mcp` / `magic-mcp` using the lightweight `VectorCraft` / `DesignCraft` contract:
/// - `connect`: attach to live `magic --control` or exit with `cannot connect to <addr>: <err>`.
/// - `headless` or `read_roots`/`write_roots`: run an in-process [`Headless`] coordinator session.
/// - neither flag: probe `127.0.0.1:9876` (`MAGIC_CONTROL_PORT`) once and fall back to [`Headless`].
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
            "{bin_name}: --automation-read-root and --automation-write-root imply --headless; pass them to `magic --control` to confine a live window"
        ));
    }
    let roots = AutomationRoots::from_args(read_roots, write_roots)?;

    if let Some(spec) = connect {
        let addr = normalize_control_addr(spec);
        let remote = Remote::connect_verified(&addr, magic::workspace::control_token())
            .map_err(|e| format!("{bin_name}: cannot connect to {addr}: {e}"))?;
        let _ = writeln!(std::io::stderr(), "{bin_name}: connected to live Magic window at {addr}");
        return Server::new(remote).serve();
    }

    if !headless && roots.is_none() && initial_project.is_none() {
        let addr = default_control_addr();
        if let Ok(remote) = Remote::connect_verified(&addr, magic::workspace::control_token()) {
            let _ = writeln!(std::io::stderr(), "{bin_name}: attached to live Magic window at {addr}");
            return Server::new(remote).serve();
        }
    }

    let mut session = Session::user_session().with_automation_roots(roots);
    if let Some(path) = initial_project {
        session.execute("file.open", json!({ "path": path })).map_err(|e| e.to_string())?;
    }
    Server::new(Headless::new(session)).serve()
}

mod server;
pub use server::Server;
