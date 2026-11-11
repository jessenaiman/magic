//! Loopback `--control` JSON-lines client and `<product>-cli --bridge` live session router.
//!
//! Speaks the documented `127.0.0.1:<port>` newline-delimited JSON control protocol across every
//! ArtCraft desktop application (`MethodParams` and AniCraft's `CmdParams`), enforcing:
//! - Fast non-blocking port reachability checks.
//! - Target-specific authentication (`FILMCRAFT_CONTROL_TOKEN` inline or first-frame `auth`).
//! - PhotoCraft ambient `file.*` path refusal awareness (routing `file.*` to `photocraft-cli run`).
//! - Canonical open-document gating so commands targeting a file on disk never mutate a different
//!   document open in the designer's window.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value, json};

use super::cli;
use super::discovery::{control_addr, read_control_token, same_canonical_path};
use super::target::{ControlAuth, ControlWire, CraftTarget};
use crate::{EngineError, Result};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);

pub enum LiveOutcome<T> {
    Unreachable,
    DifferentDocumentOpen,
    Connected(Result<T>),
}

pub fn is_listening(addr: SocketAddr) -> bool {
    TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok()
}

pub fn extract_doc_path(target: CraftTarget, info: &Value) -> Option<PathBuf> {
    if target == CraftTarget::Photo {
        let active = info.pointer("/session/active").and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok())?;
        let doc = info.pointer("/session/documents")?.as_array()?.get(active)?;
        return doc["path"].as_str().filter(|s| !s.is_empty()).map(PathBuf::from);
    }
    info.get("path").or_else(|| info.get("projectPath")).or_else(|| info.get("file")).and_then(Value::as_str).filter(|s| !s.is_empty()).map(PathBuf::from)
}

pub fn is_doc_dirty(target: CraftTarget, info: &Value) -> bool {
    if target == CraftTarget::Photo {
        let active = info.pointer("/session/active").and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok());
        let doc = active.and_then(|i| info.pointer("/session/documents")?.as_array()?.get(i));
        return doc.is_some_and(|d| d["dirty"] == json!(true));
    }
    info["dirty"] == json!(true)
}

struct ControlConn {
    target: CraftTarget,
    inline_token: Option<String>,
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    next_id: u64,
}

impl ControlConn {
    fn connect(target: CraftTarget, addr: SocketAddr, read_timeout: Duration) -> std::result::Result<Self, Option<EngineError>> {
        let Ok(stream) = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) else {
            return Err(None);
        };
        let setup = (|| -> Result<Self> {
            stream.set_read_timeout(Some(read_timeout)).map_err(|e| EngineError::ExternalTool(target.name().into(), format!("set read timeout: {e}")))?;
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| EngineError::ExternalTool(target.name().into(), format!("set write timeout: {e}")))?;
            let writer = stream.try_clone().map_err(|e| EngineError::ExternalTool(target.name().into(), format!("clone stream: {e}")))?;
            let token = read_control_token(target, addr.port());
            let inline_token = if target.auth() == ControlAuth::InlineToken { token.clone() } else { None };
            let mut conn = Self { target, inline_token, writer, reader: BufReader::new(stream), next_id: 1 };
            if target.auth() == ControlAuth::FirstFrameAuth
                && let Some(tok) = token
            {
                let _ = conn.send_envelope("auth", &json!({"token": tok}))?;
            }
            Ok(conn)
        })();
        setup.map_err(Some)
    }

    fn send_envelope(&mut self, method: &str, params: &Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let mut req = Map::new();
        if self.target.wire() == ControlWire::CmdParams {
            req.insert("id".into(), json!(id));
            req.insert("cmd".into(), json!(method));
            req.insert("params".into(), params.clone());
        } else {
            req.insert("id".into(), json!(id.to_string()));
            if let Some(ref tok) = self.inline_token {
                req.insert("token".into(), json!(tok));
            }
            req.insert("method".into(), json!(method));
            req.insert("params".into(), params.clone());
        }

        let mut line = serde_json::to_vec(&Value::Object(req)).map_err(|e| EngineError::InvalidParams(format!("serialize error: {e}")))?;
        line.push(b'\n');
        self.writer.write_all(&line).map_err(|e| EngineError::ExternalTool(self.target.name().into(), format!("control write error: {e}")))?;
        self.writer.flush().map_err(|e| EngineError::ExternalTool(self.target.name().into(), format!("control flush error: {e}")))?;

        let mut reply_line = String::new();
        self.reader.read_line(&mut reply_line).map_err(|e| EngineError::ExternalTool(self.target.name().into(), format!("control read error: {e}")))?;
        if reply_line.trim().is_empty() {
            return Err(EngineError::ExternalTool(self.target.name().into(), "empty response from control server".into()));
        }
        let parsed: Value =
            serde_json::from_str(&reply_line).map_err(|e| EngineError::ExternalTool(self.target.name().into(), format!("invalid JSON reply: {e}")))?;
        if parsed["ok"] == true {
            Ok(parsed)
        } else {
            let err = parsed["error"].as_str().unwrap_or("control request failed");
            Err(EngineError::CommandFailed(method.to_string(), err.to_string()))
        }
    }

    fn send_result(&mut self, method: &str, params: &Value) -> Result<Value> {
        let parsed = self.send_envelope(method, params)?;
        Ok(parsed.get("result").cloned().unwrap_or(Value::Null))
    }

    fn execute_command(&mut self, command: &str, params: &Value) -> Result<Value> {
        let (method, body) = self.target.control_envelope(command, params);
        self.send_result(&method, &body).map_err(|e| match e {
            EngineError::CommandFailed(_, msg) => EngineError::CommandFailed(command.to_string(), msg),
            other => other,
        })
    }

    fn matches_open_document(&mut self, expected: &Path) -> Result<bool> {
        let (method, params) = self.target.live_inspect_envelope();
        let info = self.send_result(&method, &params)?;
        Ok(extract_doc_path(self.target, &info).is_some_and(|open| same_canonical_path(&open, expected)))
    }

    fn save_document(&mut self, doc: &Path) -> Result<()> {
        let (method, params) = self.target.live_save_envelope(doc);
        let _ = self.send_result(&method, &params)?;
        Ok(())
    }
}

fn timeout_for_command(command: &str) -> Duration {
    if command.contains("export") || command.contains("render") || command == "app.save" { Duration::from_secs(120) } else { Duration::from_secs(30) }
}

pub fn live_control(target: CraftTarget, port: Option<u16>, method: &str, params: &Value) -> Result<Value> {
    let addr = match port {
        Some(p) => SocketAddr::from(([127, 0, 0, 1], p)),
        None => control_addr(target)?,
    };
    let mut conn = match ControlConn::connect(target, addr, timeout_for_command(method)) {
        Ok(c) => c,
        Err(Some(e)) => return Err(e),
        Err(None) => return Err(EngineError::ExternalTool(target.name().into(), format!("no live --control endpoint on {addr}"))),
    };
    conn.send_envelope(method, params)
}

pub fn live_execute(target: CraftTarget, port: Option<u16>, command: &str, params: &Value) -> Result<Value> {
    let addr = match port {
        Some(p) => SocketAddr::from(([127, 0, 0, 1], p)),
        None => control_addr(target)?,
    };
    if target == CraftTarget::Film && command == "file.exportMedia" && is_listening(addr) {
        return cli::exec_film_bridge(addr, command, params);
    }
    let mut conn = match ControlConn::connect(target, addr, timeout_for_command(command)) {
        Ok(c) => c,
        Err(Some(e)) => return Err(e),
        Err(None) => return Err(EngineError::ExternalTool(target.name().into(), format!("no live --control endpoint on {addr}"))),
    };
    conn.execute_command(command, params)
}

pub fn route_single(target: CraftTarget, doc: Option<&Path>, command: &str, params: &Value, save: bool) -> LiveOutcome<Value> {
    if target.forbids_live_command(command) {
        return LiveOutcome::Unreachable;
    }
    let addr = match control_addr(target) {
        Ok(a) => a,
        Err(e) => return LiveOutcome::Connected(Err(e)),
    };
    let mut conn = match ControlConn::connect(target, addr, timeout_for_command(command)) {
        Ok(c) => c,
        Err(Some(e)) => return LiveOutcome::Connected(Err(e)),
        Err(None) => return LiveOutcome::Unreachable,
    };
    if let Some(expected) = doc {
        match conn.matches_open_document(expected) {
            Ok(true) => {}
            Ok(false) => return LiveOutcome::DifferentDocumentOpen,
            Err(e) => return LiveOutcome::Connected(Err(e)),
        }
    }
    if target == CraftTarget::Film && command == "file.exportMedia" {
        drop(conn);
        return LiveOutcome::Connected(cli::exec_film_bridge(addr, command, params));
    }
    let res = conn.execute_command(command, params).and_then(|val| {
        if save && let Some(path) = doc {
            conn.save_document(path)?;
        }
        Ok(val)
    });
    LiveOutcome::Connected(res)
}

pub fn try_live_batch(target: CraftTarget, doc: &Path, steps: &[(&str, Value)], save: bool) -> Option<Result<Vec<Value>>> {
    if steps.iter().any(|(id, _)| *id != "file.save" && target.forbids_live_command(id)) {
        return None;
    }
    let addr = control_addr(target).ok()?;
    let timeout = if steps.iter().any(|(id, _)| id.contains("export") || id.contains("render")) { Duration::from_secs(120) } else { Duration::from_secs(30) };
    let mut conn = match ControlConn::connect(target, addr, timeout) {
        Ok(c) => c,
        Err(Some(e)) => return Some(Err(e)),
        Err(None) => return None,
    };
    match conn.matches_open_document(doc) {
        Ok(true) => {}
        Ok(false) => return None,
        Err(e) => return Some(Err(e)),
    }
    let result = (|| -> Result<Vec<Value>> {
        let mut out = Vec::with_capacity(steps.len());
        for (cmd_id, params) in steps {
            if *cmd_id == "file.save" {
                conn.save_document(doc)?;
                out.push(json!({"saved": doc}));
            } else {
                out.push(conn.execute_command(cmd_id, params)?);
            }
        }
        if save {
            conn.save_document(doc)?;
        }
        Ok(out)
    })();
    Some(result)
}
