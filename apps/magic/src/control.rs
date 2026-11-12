//! Bounded IPv4 loopback JSON-lines `--control` transport (`127.0.0.1:<port>`), aligned with
//! upstream ArtCraft control channels (`DeckCraft`, `DesignCraft`, `SoundCraft`, `CADCraft`).
//!
//! Security & framing properties:
//! - Binds strictly to `127.0.0.1` (`Ipv4Addr::LOCALHOST`, never `::1` or `0.0.0.0`); port `0` picks a free OS port.
//! - Unauthenticated on loopback by default; enforces constant-time bearer check when `MAGIC_CONTROL_TOKEN` is set.
//! - Bounds each request line to `16 MiB` (`take(MAX_LINE + 1).read_until`), skips blank lines, and immediately
//!   closes the socket on any non-JSON, HTTP-smuggled, oversized, or unauthorized line (`filmcraft#119`).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use subtle::ConstantTimeEq;

const MAX_LINE: usize = 16 * 1024 * 1024;
const MAX_CONNECTIONS: usize = 16;
pub type Dispatch = Arc<dyn Fn(&str, Value) -> Result<Value, String> + Send + Sync>;

pub fn parse_control_addr(spec: &str) -> Result<SocketAddr, String> {
    let trimmed = spec.trim();
    let normalized = if let Some(port) = trimmed.strip_prefix("localhost:") {
        format!("127.0.0.1:{port}")
    } else if trimmed.contains(':') {
        trimmed.to_owned()
    } else {
        format!("127.0.0.1:{trimmed}")
    };
    let addr: SocketAddr = normalized
        .parse()
        .map_err(|_| "--control requires a port (0..=65535) or 127.0.0.1:PORT")?;
    if addr.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err("--control must bind strictly to 127.0.0.1".into());
    }
    Ok(addr)
}

pub fn start(spec: &str, token: Option<&str>, dispatch: Dispatch) -> Result<SocketAddr, String> {
    let expected_token = token.map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned);
    let addr = parse_control_addr(spec)?;
    let listener = TcpListener::bind(addr).map_err(|e| format!("control bind {addr}: {e}"))?;
    let bound = listener.local_addr().map_err(|e| e.to_string())?;
    std::thread::Builder::new()
        .name("magic-control".into())
        .spawn(move || {
            let active = Arc::new(AtomicUsize::new(0));
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let _ = stream.set_nodelay(true);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
                if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                    active.fetch_sub(1, Ordering::SeqCst);
                    let _ = writeln!(
                        stream,
                        "{}",
                        json!({"id":null,"ok":false,"error":"busy: control connection limit reached; closing the connection"})
                    );
                    continue;
                }
                let active_worker = active.clone();
                let expected = expected_token.clone();
                let dispatch = dispatch.clone();
                if std::thread::Builder::new()
                    .name("magic-control-client".into())
                    .spawn(move || {
                        if let Ok(reader) = stream.try_clone() {
                            serve_lines(
                                BufReader::new(reader),
                                &mut stream,
                                expected.as_deref(),
                                |m, p| dispatch(m, p),
                            );
                        }
                        active_worker.fetch_sub(1, Ordering::SeqCst);
                    })
                    .is_err()
                {
                    active.fetch_sub(1, Ordering::SeqCst);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(bound)
}

fn serve_lines<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
    expected_token: Option<&str>,
    dispatch: impl Fn(&str, Value) -> Result<Value, String>,
) {
    loop {
        let mut line = Vec::new();
        let read = reader.by_ref().take((MAX_LINE + 1) as u64).read_until(b'\n', &mut line);
        match read {
            Ok(0) | Err(_) => break,
            _ => {}
        }
        let parsed = if line.len() > MAX_LINE || !line.ends_with(b"\n") {
            Err("oversized or incomplete control request; closing the connection")
        } else if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        } else {
            serde_json::from_slice::<Value>(&line)
                .map_err(|_| "invalid JSON request; closing the connection")
                .and_then(|v| {
                    if !v.is_object() || v["method"].as_str().is_none() {
                        return Err("request requires a string `method`; closing the connection");
                    }
                    if let Some(expected) = expected_token {
                        let given = v["token"].as_str().unwrap_or_default();
                        if !bool::from(given.as_bytes().ct_eq(expected.as_bytes())) {
                            return Err("unauthorized control token; closing the connection");
                        }
                    }
                    if v.get("params").is_some_and(|p| !p.is_object()) {
                        return Err("params must be an object; closing the connection");
                    }
                    Ok(v)
                })
        };
        let request = match parsed {
            Ok(v) => v,
            Err(error) => {
                let _ = writeln!(writer, "{}", json!({"id":null,"ok":false,"error":error}));
                let _ = writer.flush();
                break;
            }
        };
        let method = request["method"].as_str().unwrap_or_default();
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let reply = match dispatch(method, params) {
            Ok(result) => json!({"id":request["id"],"ok":true,"result":result}),
            Err(error) => json!({"id":request["id"],"ok":false,"error":error}),
        };
        if writeln!(writer, "{reply}").and_then(|_| writer.flush()).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_control_addr;

    #[test]
    fn parses_strict_ipv4_loopback_and_ephemeral_zero() {
        assert!(parse_control_addr("9875").is_ok());
        assert!(parse_control_addr("0").is_ok());
        assert!(parse_control_addr("localhost:9875").is_ok());
        assert!(parse_control_addr("127.0.0.1:9875").is_ok());
        assert!(parse_control_addr("[::1]:9875").is_err());
        assert!(parse_control_addr("127.0.0.2:9875").is_err());
        assert!(parse_control_addr("0.0.0.0:9875").is_err());
    }
}
