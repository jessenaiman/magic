//! Host ArtCraft application bridge (`crates/engine/src/craft_api.rs`):
//! connects Magic Studio to host Craft applications (`DeckCraft`, `DesignCraft`, `PhotoCraft`,
//! `VectorCraft`, `EffectCraft`, and `FilmCraft`) through each application's live `--control`
//! JSON-lines TCP API (`127.0.0.1:<port>`) and its installed `<app>-cli` binary.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Value, json};

const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_millis(90);
const PROBE_READ_TIMEOUT: Duration = Duration::from_millis(250);
const MUTATE_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const MUTATE_READ_TIMEOUT: Duration = Duration::from_millis(8000);

/// Canonical host Craft applications coordinated by Magic Studio:
/// `(app_slug, cli_binary, default_control_port, port_env, token_env, badge)`.
pub const HOST_CRAFT_APPS: &[(&str, &str, u16, &str, &str, &str)] = &[
    ("deckcraft", "deckcraft-cli", 7981, "DECKCRAFT_CONTROL_PORT", "DECKCRAFT_CONTROL_TOKEN", "DK"),
    ("designcraft", "designcraft-cli", 7979, "DESIGNCRAFT_CONTROL_PORT", "DESIGNCRAFT_CONTROL_TOKEN", "DC"),
    ("photocraft", "photocraft-cli", 7878, "PHOTOCRAFT_CONTROL_PORT", "PHOTOCRAFT_CONTROL_TOKEN", "PC"),
    ("vectorcraft", "vectorcraft-cli", 7980, "VECTORCRAFT_CONTROL_PORT", "VECTORCRAFT_CONTROL_TOKEN", "VC"),
    ("effectcraft", "effectcraft-cli", 9877, "EFFECTCRAFT_CONTROL_PORT", "EFFECTCRAFT_CONTROL_TOKEN", "EC"),
    ("filmcraft", "filmcraft-cli", 9876, "FILMCRAFT_CONTROL_PORT", "FILMCRAFT_CONTROL_TOKEN", "FC"),
];

struct CachedProbe {
    effectcraft: Option<Vec<Value>>,
    vectorcraft: Option<Vec<Value>>,
    photocraft: Option<Vec<Value>>,
    filmcraft: Option<Vec<Value>>,
    endpoints: Value,
}

static CACHE: OnceLock<Mutex<Option<CachedProbe>>> = OnceLock::new();

pub fn invalidate_cached_probe() {
    if let Some(lock) = CACHE.get()
        && let Ok(mut guard) = lock.lock()
    {
        *guard = None;
    }
}

/// Documented `--control` port, CLI name, and optional bearer-token env var for any ArtCraft app.
pub fn app_control_spec(app: &str) -> Option<(u16, String, String, String)> {
    let norm = app.trim().to_ascii_lowercase();
    let slug = norm.strip_suffix("-cli").unwrap_or(&norm);
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    let (canonical, default_port, badge) = match slug {
        "designcraft" | "dc" => ("designcraft", 7979, "DC"),
        "photocraft" | "pc" => ("photocraft", 7878, "PC"),
        "vectorcraft" | "vc" => ("vectorcraft", 7980, "VC"),
        "effectcraft" | "ec" => ("effectcraft", 9877, "EC"),
        "filmcraft" | "fc" => ("filmcraft", 9876, "FC"),
        "soundcraft" | "sc" => ("soundcraft", 9878, "SC"),
        "deckcraft" | "dk" => ("deckcraft", 7981, "DK"),
        "wordcraft" => ("wordcraft", 7982, "WC"),
        "pdfcraft" => ("pdfcraft", 7983, "PF"),
        "cadcraft" => ("cadcraft", 7984, "CD"),
        "gridcraft" => ("gridcraft", 7985, "GC"),
        "texturecraft" => ("texturecraft", 7986, "TC"),
        "materialcraft" => ("materialcraft", 7987, "MC"),
        "lightcraft" => ("lightcraft", 7988, "LC"),
        "figcraft" => ("figcraft", 7989, "FG"),
        "anicraft" => ("anicraft", 7991, "AC"),
        other => (other, 0, "CRAFT"),
    };
    let env_prefix = canonical.to_ascii_uppercase().replace('-', "_");
    Some((default_port, format!("{env_prefix}_CONTROL_PORT"), format!("{env_prefix}_CONTROL_TOKEN"), badge.to_owned()))
}

/// Canonical `<app>-cli` executable name for any ArtCraft application slug.
pub fn app_cli_name(app: &str) -> Option<String> {
    let norm = app.trim().to_ascii_lowercase();
    let slug = norm.strip_suffix("-cli").unwrap_or(&norm);
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    let canonical = match slug {
        "dc" => "designcraft",
        "dk" => "deckcraft",
        "pc" => "photocraft",
        "vc" => "vectorcraft",
        "ec" => "effectcraft",
        "fc" => "filmcraft",
        "sc" => "soundcraft",
        other => other,
    };
    Some(format!("{canonical}-cli"))
}

/// Resolve strict `127.0.0.1:<port>` control socket address and optional bearer token for `app`.
pub fn resolve_endpoint(app: &str) -> Option<(SocketAddr, Option<String>, String)> {
    let (default_port, port_env, token_env, badge) = app_control_spec(app)?;
    let port = std::env::var(&port_env)
        .ok()
        .and_then(|raw| {
            let trimmed = raw.trim();
            let suffix = trimmed.strip_prefix("127.0.0.1:").or_else(|| trimmed.strip_prefix("localhost:")).unwrap_or(trimmed);
            suffix.parse::<u16>().ok().filter(|p| *p > 0)
        })
        .unwrap_or(default_port);
    if port == 0 {
        return None;
    }
    let token = std::env::var(&token_env).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
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
    let cli_path = crate::workspace::resolve_executable_path(&cli);
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

/// Spawn any resolved `<app>-cli` executable with optional stdin payload and return trimmed stdout, failing noisily on non-zero exit.
pub fn run_cli_with_exe(cli_label: &str, exe: &Path, args: &[String], stdin_payload: Option<&str>) -> std::result::Result<String, String> {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if stdin_payload.is_some() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn().map_err(|e| format!("failed to spawn `{cli_label}` (`{}`): {e}", exe.display()))?;
    if let Some(body) = stdin_payload
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(body.as_bytes()).map_err(|e| format!("failed writing stdin to `{cli_label}`: {e}"))?;
    }
    let out = child.wait_with_output().map_err(|e| format!("failed waiting on `{cli_label}`: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
    if !out.status.success() {
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("`{cli_label}` exited with {}: {detail}", out.status));
    }
    Ok(stdout)
}

pub fn run_filmcraft_cli_with_exe(exe: &Path, args: &[String], stdin_payload: Option<&str>) -> std::result::Result<String, String> {
    run_cli_with_exe("filmcraft-cli", exe, args, stdin_payload)
}

pub fn run_effectcraft_cli_with_exe(exe: &Path, args: &[String], stdin_payload: Option<&str>) -> std::result::Result<String, String> {
    run_cli_with_exe("effectcraft-cli", exe, args, stdin_payload)
}

/// True when a `FilmCraft` engine command only inspects state or writes an external export artifact without mutating `.fcproj`.
pub fn is_readonly_filmcraft_command(command: &str) -> bool {
    matches!(
        command,
        "sequence.inspect"
            | "project.inspect"
            | "doc.inspect"
            | "media.status"
            | "media.findMissing"
            | "jobs.list"
            | "export.queue.list"
            | "effects.list"
            | "markers.exportCsv"
            | "file.exportFrame"
            | "file.exportMedia"
    )
}

/// True when an `EffectCraft` engine command only inspects state or exports an external artifact without mutating `.ecproj`.
pub fn is_readonly_effectcraft_command(command: &str) -> bool {
    matches!(
        command,
        "comp.info"
            | "comp.list"
            | "layer.tree"
            | "effects.list"
            | "effect.list"
            | "text.fonts"
            | "text.presets"
            | "text.animatorProperties"
            | "file.exportTimeline"
            | "render.frame"
    )
}

pub fn filmcraft_cli_available() -> bool {
    crate::workspace::resolve_executable_path("filmcraft-cli").is_some()
}

pub fn effectcraft_cli_available() -> bool {
    crate::workspace::resolve_executable_path("effectcraft-cli").is_some()
}

/// Build canonical `filmcraft-cli [--project <p.fcproj> [--save]] exec <id> '<json>'` argv.
pub fn build_filmcraft_exec_args(doc_path: Option<&Path>, save: bool, command: &str, params: &Value) -> Vec<String> {
    let mut args = Vec::with_capacity(6);
    if let Some(doc) = doc_path {
        args.push("--project".into());
        args.push(doc.to_string_lossy().into_owned());
        if save {
            args.push("--save".into());
        }
    }
    args.push("exec".into());
    args.push(command.to_owned());
    args.push(params.to_string());
    args
}

/// Build canonical `filmcraft-cli [--project <p.fcproj> --save | --save-as <p.fcproj>] run -` argv and JSONL stdin.
pub fn build_filmcraft_run_invocation(project: Option<&Path>, fresh: bool, save: bool, steps: &[Value]) -> (Vec<String>, String) {
    let mut args = Vec::with_capacity(5);
    if let Some(proj) = project {
        let proj_str = proj.to_string_lossy().into_owned();
        if fresh || !proj.is_file() {
            args.push("--save-as".into());
            args.push(proj_str);
        } else {
            args.push("--project".into());
            args.push(proj_str);
            if save {
                args.push("--save".into());
            }
        }
    }
    args.push("run".into());
    args.push("-".into());

    let mut payload = String::new();
    for step in steps {
        let id = step["id"].as_str().or_else(|| step["command"].as_str()).unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
        payload.push_str(&json!({"id": id, "params": params}).to_string());
        payload.push('\n');
    }
    (args, payload)
}

/// Parse JSONL output emitted by `filmcraft-cli ... run -` (`{"line","id","ok","result"|"error"}`).
pub fn parse_filmcraft_run_output(stdout: &str) -> std::result::Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: Value = serde_json::from_str(line).map_err(|e| format!("invalid `filmcraft-cli run` JSON line `{line}`: {e}"))?;
        if parsed.get("ok").and_then(Value::as_bool) == Some(false) {
            let id = parsed.get("id").and_then(Value::as_str).unwrap_or("step");
            let err = parsed.get("error").and_then(Value::as_str).unwrap_or("command failed");
            return Err(format!("`filmcraft-cli run` failed at `{id}`: {err}"));
        }
        out.push(parsed.get("result").cloned().unwrap_or(parsed));
    }
    Ok(out)
}

/// Build canonical `filmcraft-cli --project <p.fcproj> export <out> ...` argv (or `exec file.exportMedia` for draft/queue overrides).
pub fn build_filmcraft_export_args(project: &Path, output: &Path, draft: bool, params: &Value, options: &Value) -> Vec<String> {
    let proj_str = project.to_string_lossy().into_owned();
    if options.get("queue").and_then(Value::as_bool) == Some(true) {
        return vec!["--project".into(), proj_str, "exec".into(), "export.queue.add".into(), params.to_string()];
    }
    let needs_engine_json = draft
        || ["burnCaptions", "captionSidecar", "loudnessLufs", "proresProfile", "dnxProfile", "hdrTransfer", "scale", "quality", "alpha"]
            .iter()
            .any(|k| options.get(*k).is_some_and(|v| !v.is_null()));
    if needs_engine_json {
        return vec!["--project".into(), proj_str, "exec".into(), "file.exportMedia".into(), params.to_string()];
    }
    let mut args = vec!["--project".into(), proj_str, "export".into(), output.to_string_lossy().into_owned()];
    if let Some(preset) = options.get("preset").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        args.push("--preset".into());
        args.push(preset.to_owned());
    }
    let start_opt = options.get("startSeconds").and_then(Value::as_f64);
    let end_opt = options.get("endSeconds").and_then(Value::as_f64);
    if start_opt.is_some() || end_opt.is_some() {
        if let Some(st) = start_opt {
            args.push("--start".into());
            args.push(st.to_string());
        }
        if let Some(en) = end_opt {
            args.push("--end".into());
            args.push(en.to_string());
        }
    } else {
        let range = options.get("range").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("entire");
        args.push("--range".into());
        args.push(range.to_owned());
    }
    if options.get("allowOffline").and_then(Value::as_bool) == Some(true) {
        args.push("--allow-offline".into());
    }
    args
}

/// Build canonical `effectcraft-cli exec <id> --params '<json>' [main.ecproj [--save] | --empty [--save-as main.ecproj]] --json` argv.
pub fn build_effectcraft_exec_args(doc_path: Option<&Path>, empty: bool, save: bool, command: &str, params: &Value) -> Vec<String> {
    let mut args = vec!["exec".into(), command.to_owned(), "--params".into(), params.to_string()];
    match doc_path {
        Some(doc) if !empty && doc.is_file() => {
            args.push(doc.to_string_lossy().into_owned());
            if save {
                args.push("--save".into());
            }
        }
        Some(doc) => {
            args.push("--empty".into());
            if save {
                args.push("--save-as".into());
                args.push(doc.to_string_lossy().into_owned());
            }
        }
        None => {
            args.push("--empty".into());
        }
    }
    args.push("--json".into());
    args
}

/// Build canonical `effectcraft-cli run [main.ecproj --commands - [--save] | --commands - --empty --save-as main.ecproj] --json` argv + JSONL.
pub fn build_effectcraft_run_invocation(project: Option<&Path>, save: bool, steps: &[Value]) -> (Vec<String>, String) {
    let mut inferred_save_as: Option<PathBuf> = None;
    let mut payload = String::new();
    for step in steps {
        let cmd = step["command"].as_str().or_else(|| step["id"].as_str()).unwrap_or_default();
        if cmd.is_empty() {
            continue;
        }
        if cmd == "file.saveAs"
            && let Some(raw_p) = step.pointer("/params/path").and_then(Value::as_str).filter(|s| !s.is_empty())
        {
            inferred_save_as = Some(PathBuf::from(raw_p));
            continue;
        }
        if cmd == "file.save" && save && project.is_some() {
            continue;
        }
        let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
        payload.push_str(&json!({"command": cmd, "params": params}).to_string());
        payload.push('\n');
    }

    let target = project.map(Path::to_path_buf).or(inferred_save_as);
    let mut args = vec!["run".into()];
    match target {
        Some(ref doc) if doc.is_file() && project.is_some() => {
            args.push(doc.to_string_lossy().into_owned());
            args.push("--commands".into());
            args.push("-".into());
            if save {
                args.push("--save".into());
            }
        }
        Some(ref doc) => {
            args.push("--commands".into());
            args.push("-".into());
            args.push("--empty".into());
            args.push("--save-as".into());
            args.push(doc.to_string_lossy().into_owned());
        }
        None => {
            args.push("--commands".into());
            args.push("-".into());
            args.push("--empty".into());
        }
    }
    args.push("--json".into());
    (args, payload)
}

/// Parse JSON or JSONL replies from `effectcraft-cli run ... --json`, failing noisily on `"ok": false`.
pub fn parse_effectcraft_run_output(stdout: &str) -> std::result::Result<Vec<Value>, String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if let Ok(whole) = serde_json::from_str::<Value>(trimmed) {
        if whole.get("ok").and_then(Value::as_bool) == Some(false) {
            let err = whole.get("error").and_then(Value::as_str).unwrap_or("effectcraft-cli run failed");
            return Err(format!("`effectcraft-cli run` failed: {err}"));
        }
        if let Some(arr) = whole.get("results").and_then(Value::as_array).or_else(|| whole.as_array()) {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                if item.get("ok").and_then(Value::as_bool) == Some(false) {
                    let id = item.get("command").or_else(|| item.get("id")).and_then(Value::as_str).unwrap_or("step");
                    let err = item.get("error").and_then(Value::as_str).unwrap_or("step failed");
                    return Err(format!("`effectcraft-cli run` failed at `{id}`: {err}"));
                }
                out.push(item.get("result").cloned().unwrap_or_else(|| item.clone()));
            }
            return Ok(out);
        }
        return Ok(vec![whole.get("result").cloned().unwrap_or(whole)]);
    }

    let mut out = Vec::new();
    for raw_line in trimmed.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: Value = serde_json::from_str(line).map_err(|e| format!("invalid `effectcraft-cli run` JSON line `{line}`: {e}"))?;
        if parsed.get("ok").and_then(Value::as_bool) == Some(false) {
            let id = parsed.get("command").or_else(|| parsed.get("id")).and_then(Value::as_str).unwrap_or("step");
            let err = parsed.get("error").and_then(Value::as_str).unwrap_or("step failed");
            return Err(format!("`effectcraft-cli run` failed at `{id}`: {err}"));
        }
        out.push(parsed.get("result").cloned().unwrap_or(parsed));
    }
    Ok(out)
}

/// Build canonical `effectcraft-cli render <main.ecproj> --comp <Comp> --out <out> --format <prores|webm|h264|png|exr>` argv.
pub fn build_effectcraft_render_args(project: &Path, params: &Value) -> std::result::Result<Vec<String>, String> {
    let output = params
        .get("output")
        .or_else(|| params.get("out"))
        .or_else(|| params.get("path"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "`effectcraft` render requires `output` path".to_owned())?;
    let format = params.get("format").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("prores");
    let mut args = vec!["render".into(), project.to_string_lossy().into_owned()];
    if let Some(comp) = params.get("comp").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        args.push("--comp".into());
        args.push(comp.to_owned());
    }
    args.push("--out".into());
    args.push(output.to_owned());
    args.push("--format".into());
    args.push(format.to_owned());
    if format.eq_ignore_ascii_case("exr") {
        args.push("--exr-compression".into());
        args.push(params.get("exrCompression").and_then(Value::as_str).unwrap_or("piz").to_owned());
        args.push("--exr-depth".into());
        args.push(params.get("depth").and_then(Value::as_str).unwrap_or("half").to_owned());
    }
    Ok(args)
}

/// Build canonical `effectcraft-cli render-frame <main.ecproj> [--comp <Comp>] --time <sec> --max-side <px> [--transparent] --out <f.png> --json` argv.
pub fn build_effectcraft_render_frame_args(project: &Path, comp: Option<&str>, seconds: f64, max_side: u32, transparent: bool, output: &Path) -> Vec<String> {
    let mut args = vec!["render-frame".into(), project.to_string_lossy().into_owned()];
    if let Some(c) = comp.map(str::trim).filter(|s| !s.is_empty()) {
        args.push("--comp".into());
        args.push(c.to_owned());
    }
    args.push("--time".into());
    args.push(seconds.max(0.0).to_string());
    args.push("--max-side".into());
    args.push(max_side.clamp(64, 4096).to_string());
    if transparent {
        args.push("--transparent".into());
    }
    args.push("--out".into());
    args.push(output.to_string_lossy().into_owned());
    args.push("--json".into());
    args
}

/// Execute one `FilmCraft` command via live `:9876` `--control` when connected, else `filmcraft-cli --project <doc> [--save] exec`.
pub fn execute_filmcraft_command(doc_path: Option<&Path>, save: Option<bool>, command: &str, params: Value) -> std::result::Result<Value, String> {
    let want_save = save.unwrap_or_else(|| !is_readonly_filmcraft_command(command));
    if probe_live_app("filmcraft")["connected"] == true {
        let live = execute_in_live_app("filmcraft", command, params)?;
        if want_save && command != "file.save" {
            save_in_live_app("filmcraft", doc_path)?;
        }
        return Ok(json!({
            "ok": true,
            "transport": "control",
            "app": "filmcraft",
            "command": command,
            "saved": want_save,
            "result": live,
        }));
    }
    let exe = crate::workspace::resolve_executable_path("filmcraft-cli")
        .ok_or_else(|| "`filmcraft` is not running on `--control 9876` and `filmcraft-cli` was not found on PATH".to_owned())?;
    let args = build_filmcraft_exec_args(doc_path, want_save, command, &params);
    let stdout = run_filmcraft_cli_with_exe(&exe, &args, None)?;
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"stdout": stdout}));
    if parsed.get("ok").and_then(Value::as_bool) == Some(false) {
        let err = parsed.get("error").and_then(Value::as_str).unwrap_or("filmcraft-cli exec failed");
        return Err(format!("`filmcraft-cli exec {command}` failed: {err}"));
    }
    Ok(json!({
        "ok": true,
        "transport": "cli",
        "app": "filmcraft",
        "cli": "filmcraft-cli",
        "command": command,
        "saved": want_save,
        "result": parsed,
    }))
}

/// Run a batch of `FilmCraft` steps via `filmcraft-cli [--project <p.fcproj> --save | --save-as <p.fcproj>] run -`.
pub fn run_filmcraft_steps(project: Option<&Path>, fresh: bool, save: bool, steps: &[Value]) -> std::result::Result<Vec<Value>, String> {
    if steps.is_empty() {
        return Ok(Vec::new());
    }
    let exe = crate::workspace::resolve_executable_path("filmcraft-cli").ok_or_else(|| "`filmcraft-cli` was not found on PATH".to_owned())?;
    let (args, stdin_jsonl) = build_filmcraft_run_invocation(project, fresh, save, steps);
    let stdout = run_filmcraft_cli_with_exe(&exe, &args, Some(&stdin_jsonl))?;
    parse_filmcraft_run_output(&stdout)
}

/// Export media via `filmcraft-cli --project <p.fcproj> export <out> ...` (or `exec file.exportMedia` for draft/queue).
pub fn export_filmcraft_media(project: &Path, output: &Path, draft: bool, params: &Value, options: &Value) -> std::result::Result<Value, String> {
    let exe = crate::workspace::resolve_executable_path("filmcraft-cli").ok_or_else(|| "`filmcraft-cli` was not found on PATH".to_owned())?;
    let args = build_filmcraft_export_args(project, output, draft, params, options);
    let stdout = run_filmcraft_cli_with_exe(&exe, &args, None)?;
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"ok": true, "path": output, "stdout": stdout}));
    if parsed.get("ok").and_then(Value::as_bool) == Some(false) {
        let err = parsed.get("error").and_then(Value::as_str).unwrap_or("filmcraft-cli export failed");
        return Err(format!("`filmcraft-cli export` failed: {err}"));
    }
    Ok(parsed)
}

/// Execute one `EffectCraft` command via live `:9877` `--control` when connected, else `effectcraft-cli exec <id> --params '<json>' ... --json`.
pub fn execute_effectcraft_command(
    doc_path: Option<&Path>,
    empty: bool,
    save: Option<bool>,
    command: &str,
    params: Value,
) -> std::result::Result<Value, String> {
    let want_save = save.unwrap_or_else(|| !is_readonly_effectcraft_command(command));
    if probe_live_app("effectcraft")["connected"] == true {
        let live = execute_in_live_app("effectcraft", command, params)?;
        if want_save && !matches!(command, "file.save" | "file.saveAs") {
            save_in_live_app("effectcraft", doc_path)?;
        }
        return Ok(json!({
            "ok": true,
            "transport": "control",
            "app": "effectcraft",
            "command": command,
            "saved": want_save,
            "result": live,
        }));
    }
    let exe = crate::workspace::resolve_executable_path("effectcraft-cli")
        .ok_or_else(|| "`effectcraft` is not running on `--control 9877` and `effectcraft-cli` was not found on PATH".to_owned())?;
    let args = build_effectcraft_exec_args(doc_path, empty, want_save, command, &params);
    let stdout = run_effectcraft_cli_with_exe(&exe, &args, None)?;
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"stdout": stdout}));
    if parsed.get("ok").and_then(Value::as_bool) == Some(false) {
        let err = parsed.get("error").and_then(Value::as_str).unwrap_or("effectcraft-cli exec failed");
        return Err(format!("`effectcraft-cli exec {command}` failed: {err}"));
    }
    Ok(json!({
        "ok": true,
        "transport": "cli",
        "app": "effectcraft",
        "cli": "effectcraft-cli",
        "command": command,
        "saved": want_save,
        "result": parsed,
    }))
}

/// Run a batch of `EffectCraft` steps via live `:9877` `--control` first, falling back to `effectcraft-cli run ... --json`.
pub fn run_effectcraft_steps(project: Option<&Path>, save: bool, steps: &[Value]) -> std::result::Result<Vec<Value>, String> {
    if steps.is_empty() {
        return Ok(Vec::new());
    }
    if probe_live_app("effectcraft")["connected"] == true {
        if let Some(proj) = project.filter(|p| p.is_file()) {
            let _ = execute_in_live_app("effectcraft", "file.open", json!({"path": proj}));
        }
        let mut results = Vec::with_capacity(steps.len());
        let mut saved_via_step = false;
        for step in steps {
            let cmd = step["command"].as_str().or_else(|| step["id"].as_str()).unwrap_or_default();
            if cmd.is_empty() {
                continue;
            }
            if matches!(cmd, "file.save" | "file.saveAs") {
                saved_via_step = true;
            }
            let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
            let res = execute_in_live_app("effectcraft", cmd, params).map_err(|e| format!("live `effectcraft --control 9877` failed at `{cmd}`: {e}"))?;
            results.push(res);
        }
        if save && !saved_via_step {
            save_in_live_app("effectcraft", project).map_err(|e| format!("live `effectcraft --control 9877` save failed: {e}"))?;
        }
        return Ok(results);
    }
    let exe = crate::workspace::resolve_executable_path("effectcraft-cli")
        .ok_or_else(|| "`effectcraft` is not running on `--control 9877` and `effectcraft-cli` was not found on PATH".to_owned())?;
    let (args, stdin_jsonl) = build_effectcraft_run_invocation(project, save, steps);
    let stdout = run_effectcraft_cli_with_exe(&exe, &args, Some(&stdin_jsonl))?;
    parse_effectcraft_run_output(&stdout)
}

/// Render an `EffectCraft` composition to movie/sequence via `:9877` (`renderQueue.add` + `renderQueue.render {"wait":true}`)
/// or `effectcraft-cli render <main.ecproj> --comp <Comp> --out <out> --format <fmt>`, failing noisily on error.
pub fn render_effectcraft_comp(project: &Path, params: &Value) -> std::result::Result<Value, String> {
    if probe_live_app("effectcraft")["connected"] == true {
        if project.is_file() {
            let _ = execute_in_live_app("effectcraft", "file.open", json!({"path": project}));
        }
        let mut rq_params = params.clone();
        if rq_params.get("channels").is_none() {
            rq_params["channels"] = json!("rgba");
        }
        if rq_params.get("format").and_then(Value::as_str).unwrap_or("prores") == "prores" && rq_params.get("proresProfile").is_none() {
            rq_params["proresProfile"] = json!("4444");
        }
        let added =
            execute_in_live_app("effectcraft", "renderQueue.add", rq_params).map_err(|e| format!("live `effectcraft` `renderQueue.add` failed: {e}"))?;
        let rendered = execute_in_live_app("effectcraft", "renderQueue.render", json!({"wait": true}))
            .map_err(|e| format!("live `effectcraft` `renderQueue.render` failed: {e}"))?;
        return Ok(json!({
            "ok": true,
            "transport": "control",
            "project": project,
            "queued": added,
            "result": rendered,
        }));
    }
    let exe = crate::workspace::resolve_executable_path("effectcraft-cli")
        .ok_or_else(|| "`effectcraft` is not running on `--control 9877` and `effectcraft-cli` was not found on PATH".to_owned())?;
    let args = build_effectcraft_render_args(project, params)?;
    let stdout = run_effectcraft_cli_with_exe(&exe, &args, None)?;
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"ok": true, "stdout": stdout}));
    if parsed.get("ok").and_then(Value::as_bool) == Some(false) {
        let err = parsed.get("error").and_then(Value::as_str).unwrap_or("effectcraft-cli render failed");
        return Err(format!("`effectcraft-cli render` failed: {err}"));
    }
    Ok(json!({
        "ok": true,
        "transport": "cli",
        "project": project,
        "result": parsed,
    }))
}

/// Render a single `EffectCraft` PNG frame via `:9877` (`render.frame`) or `effectcraft-cli render-frame --transparent`, failing noisily on error.
pub fn render_effectcraft_frame(
    project: &Path,
    comp: Option<&str>,
    seconds: f64,
    max_side: u32,
    transparent: bool,
    output: &Path,
) -> std::result::Result<Value, String> {
    if probe_live_app("effectcraft")["connected"] == true {
        if project.is_file() {
            let _ = execute_in_live_app("effectcraft", "file.open", json!({"path": project}));
        }
        let mut req = json!({
            "time": seconds.max(0.0),
            "maxSide": max_side.clamp(64, 4096),
            "transparent": transparent,
            "output": output,
        });
        if let Some(c) = comp.map(str::trim).filter(|s| !s.is_empty()) {
            req["comp"] = json!(c);
        }
        let live = execute_in_live_app("effectcraft", "render.frame", req).map_err(|e| format!("live `effectcraft` `render.frame` failed: {e}"))?;
        return Ok(json!({"ok": true, "transport": "control", "frame": output, "result": live}));
    }
    let exe = crate::workspace::resolve_executable_path("effectcraft-cli")
        .ok_or_else(|| "`effectcraft` is not running on `--control 9877` and `effectcraft-cli` was not found on PATH".to_owned())?;
    let args = build_effectcraft_render_frame_args(project, comp, seconds, max_side, transparent, output);
    let stdout = run_effectcraft_cli_with_exe(&exe, &args, None)?;
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"ok": true, "frame": output, "stdout": stdout}));
    if parsed.get("ok").and_then(Value::as_bool) == Some(false) {
        let err = parsed.get("error").and_then(Value::as_str).unwrap_or("effectcraft-cli render-frame failed");
        return Err(format!("`effectcraft-cli render-frame` failed: {err}"));
    }
    Ok(parsed)
}

/// Execute an engine command on a running host Craft app via `--control`, falling back to `<app>-cli` on the host machine.
pub fn execute_on_host_app(app: &str, doc_path: Option<&Path>, command: &str, params: Value) -> std::result::Result<Value, String> {
    if app.eq_ignore_ascii_case("filmcraft") || app.eq_ignore_ascii_case("fc") {
        return execute_filmcraft_command(doc_path, None, command, params);
    }
    if app.eq_ignore_ascii_case("effectcraft") || app.eq_ignore_ascii_case("ec") {
        return execute_effectcraft_command(doc_path, false, None, command, params);
    }
    if probe_live_app(app)["connected"] == true {
        let live = execute_in_live_app(app, command, params)?;
        return Ok(json!({"transport": "control", "app": app, "result": live}));
    }
    let cli = app_cli_name(app).ok_or_else(|| format!("unknown Craft app `{app}`"))?;
    let exe =
        crate::workspace::resolve_executable_path(&cli).ok_or_else(|| format!("`{app}` is not running on --control and `{cli}` was not found on PATH"))?;
    let mut args = Vec::new();
    if let Some(doc) = doc_path {
        args.push(doc.to_string_lossy().into_owned());
    }
    args.extend(["run".to_owned(), command.to_owned(), params.to_string()]);
    let stdout = run_cli_with_exe(&cli, &exe, &args, None)?;
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"stdout": stdout}));
    Ok(json!({"transport": "cli", "app": app, "cli": cli, "result": parsed}))
}

/// Run `<app>-cli` with explicit arguments on the host machine (for `--export`, `script`, or batch commands).
pub fn run_host_cli(app: &str, args: &[&str]) -> std::result::Result<Value, String> {
    let cli = app_cli_name(app).ok_or_else(|| format!("unknown Craft app `{app}`"))?;
    let exe =
        crate::workspace::resolve_executable_path(&cli).ok_or_else(|| format!("`{cli}` is not installed on PATH (configure `[tools]` in config.toml)"))?;
    let owned_args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    let stdout = run_cli_with_exe(&cli, &exe, &owned_args, None)?;
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"stdout": stdout}));
    Ok(parsed)
}

fn normalize_live_effect_list(app: &str, badge: &str, raw: &Value) -> Vec<Value> {
    let arr_opt = raw
        .as_array()
        .or_else(|| raw.get("effects").and_then(Value::as_array))
        .or_else(|| raw.get("presets").and_then(Value::as_array))
        .or_else(|| raw.get("commands").and_then(Value::as_array))
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
            let craft_cmd = item.get("command").and_then(Value::as_str).unwrap_or(match app {
                "filmcraft" => "effects.apply",
                "photocraft" => "layer.style.set",
                _ => "effect.apply",
            });
            Some(json!({
                "id": match_name,
                "badge": badge,
                "craft_app": app,
                "craft_command": craft_cmd,
                "match_name": match_name,
                "category": category,
                "label": label,
                "summary": format!("Live {app} `--control` API ({category})"),
                "params": {},
                "cli_available": true,
                "live_api": true,
            }))
        })
        .collect()
}

fn query_live_effects_uncached(app: &str) -> Option<Vec<Value>> {
    let (addr, token, badge) = resolve_endpoint(app)?;
    let candidates: &[&str] = match app {
        "filmcraft" => &["effects.list"],
        "effectcraft" => &["effects.list", "effect.list", "text.presets"],
        "photocraft" => &["layer.styles", "actions.list", "filter.list"],
        "vectorcraft" => &["effects.list", "effect.list"],
        _ => &["effects.list"],
    };
    let mut out = Vec::new();
    for list_cmd in candidates {
        let res_opt =
            send_json_line(addr, token.as_deref(), "engine.execute", json!({"command": list_cmd, "params": {}}), PROBE_CONNECT_TIMEOUT, PROBE_READ_TIMEOUT)
                .or_else(|_| send_json_line(addr, token.as_deref(), list_cmd, json!({}), PROBE_CONNECT_TIMEOUT, PROBE_READ_TIMEOUT))
                .ok();
        if let Some(res) = res_opt {
            out.extend(normalize_live_effect_list(app, &badge, &res));
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Query dynamic fonts from open `EffectCraft :9877` / `PhotoCraft :7878` / `VectorCraft :7980`, merged with `craft_font_catalog::Catalog::global()`.
pub fn discover_suite_fonts() -> Vec<String> {
    let mut families = magic_util::media::discover_installed_font_families();
    for (app, cmd) in [("effectcraft", "text.fonts"), ("photocraft", "type.fonts"), ("vectorcraft", "type.fonts")] {
        if let Some((addr, token, _)) = resolve_endpoint(app)
            && let Ok(res) = send_json_line(addr, token.as_deref(), cmd, json!({}), PROBE_CONNECT_TIMEOUT, PROBE_READ_TIMEOUT)
        {
            let arr_opt = res.as_array().or_else(|| res.get("fonts").and_then(Value::as_array));
            for item in arr_opt.into_iter().flatten() {
                let name_opt = item.as_str().or_else(|| item.get("family").or_else(|| item.get("name")).and_then(Value::as_str));
                if let Some(fam) = name_opt.map(str::trim).filter(|s| !s.is_empty())
                    && !families.iter().any(|existing| existing.eq_ignore_ascii_case(fam))
                {
                    families.push(fam.to_owned());
                }
            }
        }
    }
    families.sort();
    families
}

type LiveCraftSnapshot = (Value, Option<Vec<Value>>, Option<Vec<Value>>, Option<Vec<Value>>);

pub fn cached_live_snapshot() -> LiveCraftSnapshot {
    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = lock.lock()
        && let Some(cached) = guard.as_ref()
    {
        let mut vc_or_pc = cached.vectorcraft.clone().unwrap_or_default();
        if let Some(pc) = &cached.photocraft {
            vc_or_pc.extend(pc.iter().cloned());
        }
        let merged_vc_pc = if vc_or_pc.is_empty() { None } else { Some(vc_or_pc) };
        return (cached.endpoints.clone(), cached.effectcraft.clone(), merged_vc_pc, cached.filmcraft.clone());
    }
    (default_endpoints_json(), None, None, None)
}

pub fn live_craft_snapshot(force_refresh: bool) -> LiveCraftSnapshot {
    if !force_refresh {
        return cached_live_snapshot();
    }
    let dk_probe = probe_live_app("deckcraft");
    let dc_probe = probe_live_app("designcraft");
    let pc_probe = probe_live_app("photocraft");
    let vc_probe = probe_live_app("vectorcraft");
    let ec_probe = probe_live_app("effectcraft");
    let fc_probe = probe_live_app("filmcraft");

    let ec_fx = if ec_probe["connected"] == true { query_live_effects_uncached("effectcraft") } else { None };
    let vc_fx = if vc_probe["connected"] == true { query_live_effects_uncached("vectorcraft") } else { None };
    let pc_fx = if pc_probe["connected"] == true { query_live_effects_uncached("photocraft") } else { None };
    let fc_fx = if fc_probe["connected"] == true { query_live_effects_uncached("filmcraft") } else { None };

    let endpoints = json!({
        "deckcraft": dk_probe,
        "designcraft": dc_probe,
        "photocraft": pc_probe,
        "vectorcraft": vc_probe,
        "effectcraft": ec_probe,
        "filmcraft": fc_probe,
    });

    let lock = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(CachedProbe {
            effectcraft: ec_fx.clone(),
            vectorcraft: vc_fx.clone(),
            photocraft: pc_fx.clone(),
            filmcraft: fc_fx.clone(),
            endpoints: endpoints.clone(),
        });
    }
    let mut vc_or_pc = vc_fx.unwrap_or_default();
    if let Some(pc) = pc_fx {
        vc_or_pc.extend(pc);
    }
    (endpoints, ec_fx, (!vc_or_pc.is_empty()).then_some(vc_or_pc), fc_fx)
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
            "vector_and_photo": vc_fx.as_ref().map_or(0, Vec::len),
            "filmcraft": fc_fx.as_ref().map_or(0, Vec::len),
        }
    }))
}

pub fn cmd_craft_live_save(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let app = p.get("app").and_then(Value::as_str).unwrap_or("effectcraft");
    let resolved = p.get("path").and_then(Value::as_str).filter(|r| !r.trim().is_empty()).map(|raw| crate::workspace::resolve_workspace_path(s, raw));
    let reply = save_in_live_app(app, resolved.as_deref()).map_err(|detail| crate::EngineError::ExternalTool(app.to_owned(), detail))?;
    Ok(json!({"ok": true, "app": app, "saved_via_control": true, "reply": reply}))
}

/// Pass any `EffectCraft` engine command (`comp.info`, `layer.newText`, `prop.addKey`, `file.exportTimeline`, etc.)
/// straight to a live `effectcraft --control 9877` session or `effectcraft-cli exec <id> --params '<json>' <project.ecproj> [--save] --json`.
pub fn cmd_effect_exec(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let command = p
        .get("command")
        .or_else(|| p.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| crate::EngineError::InvalidParams("effect.exec requires non-empty `command` (e.g. `comp.info` or `layer.newText`)".into()))?;
    let params = match p.get("params") {
        Some(v) if v.is_object() => v.clone(),
        Some(v) if v.is_null() => json!({}),
        Some(_) => return Err(crate::EngineError::InvalidParams("`params` must be a JSON object".into())),
        None => {
            let mut extra = p.clone();
            if let Some(obj) = extra.as_object_mut() {
                for key in ["command", "id", "save", "project", "empty", "domain"] {
                    obj.remove(key);
                }
            }
            extra
        }
    };
    let empty = p.get("empty").and_then(Value::as_bool).unwrap_or(false);
    let save_override = p.get("save").and_then(Value::as_bool);
    let doc_path = if let Some(raw_proj) = p.get("project").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
        Some(crate::workspace::resolve_workspace_path(s, raw_proj))
    } else if empty {
        None
    } else if let Ok(dir) = crate::workshop::project_dir(s) {
        let domain = p.get("domain").and_then(Value::as_str).unwrap_or("lyrics");
        Some(if domain == "motion" { crate::cmd::motion::motion_ecproj_path(&dir) } else { crate::cmd::words::lyrics_ecproj_path(&dir) })
    } else {
        None
    };
    execute_effectcraft_command(doc_path.as_deref(), empty, save_override, command, params)
        .map_err(|e| crate::EngineError::ExternalTool("effectcraft-cli".into(), e))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::{
        build_effectcraft_exec_args, build_effectcraft_render_args, build_effectcraft_render_frame_args, build_effectcraft_run_invocation,
        build_filmcraft_exec_args, build_filmcraft_export_args, build_filmcraft_run_invocation, is_readonly_effectcraft_command, is_readonly_filmcraft_command,
        parse_effectcraft_run_output, parse_filmcraft_run_output, run_effectcraft_cli_with_exe, run_filmcraft_cli_with_exe,
    };

    #[test]
    fn filmcraft_cli_exec_and_export_args_match_upstream_contract() {
        let proj = Path::new("/tmp/song/film/Song.fcproj");
        let out = Path::new("/tmp/song/exports/Song.mp4");

        assert!(is_readonly_filmcraft_command("sequence.inspect"));
        assert!(!is_readonly_filmcraft_command("timeline.razor"));

        let ro_args = build_filmcraft_exec_args(Some(proj), false, "sequence.inspect", &json!({}));
        assert_eq!(ro_args, vec!["--project", "/tmp/song/film/Song.fcproj", "exec", "sequence.inspect", "{}"]);

        let mut_args = build_filmcraft_exec_args(Some(proj), true, "timeline.razor", &json!({"seconds": 3.5}));
        assert_eq!(mut_args, vec!["--project", "/tmp/song/film/Song.fcproj", "--save", "exec", "timeline.razor", "{\"seconds\":3.5}"]);

        let exp_args = build_filmcraft_export_args(
            proj,
            out,
            false,
            &json!({"path": out, "format": "h264", "wait": true}),
            &json!({"preset": "YouTube 1080p Full HD", "allowOffline": true}),
        );
        assert_eq!(
            exp_args,
            vec![
                "--project",
                "/tmp/song/film/Song.fcproj",
                "export",
                "/tmp/song/exports/Song.mp4",
                "--preset",
                "YouTube 1080p Full HD",
                "--range",
                "entire",
                "--allow-offline",
            ]
        );

        let draft_args = build_filmcraft_export_args(proj, out, true, &json!({"path": out, "scale": 0.5, "wait": true}), &json!({}));
        assert_eq!(draft_args[0..4], ["--project", "/tmp/song/film/Song.fcproj", "exec", "file.exportMedia"]);
    }

    #[test]
    fn effectcraft_cli_exec_run_and_render_args_match_upstream_contract() {
        assert!(is_readonly_effectcraft_command("comp.info"));
        assert!(!is_readonly_effectcraft_command("layer.newText"));

        let missing = Path::new("/tmp/song/effects/lyrics.ecproj");
        let exec_new = build_effectcraft_exec_args(Some(missing), false, true, "comp.new", &json!({"name": "Lyrics"}));
        assert_eq!(
            exec_new,
            vec!["exec", "comp.new", "--params", "{\"name\":\"Lyrics\"}", "--empty", "--save-as", "/tmp/song/effects/lyrics.ecproj", "--json"]
        );

        let steps = vec![
            json!({"command": "comp.new", "params": {"name": "Lyrics"}}),
            json!({"command": "file.saveAs", "params": {"path": "/tmp/song/effects/lyrics.ecproj"}}),
        ];
        let (run_args, stdin_jsonl) = build_effectcraft_run_invocation(None, true, &steps);
        assert_eq!(run_args, vec!["run", "--commands", "-", "--empty", "--save-as", "/tmp/song/effects/lyrics.ecproj", "--json"]);
        assert_eq!(stdin_jsonl.lines().count(), 1);

        let render_args =
            build_effectcraft_render_args(missing, &json!({"comp": "Lyrics", "output": "/tmp/song/effects/lyrics.mov", "format": "prores"})).unwrap();
        assert_eq!(
            render_args,
            vec!["render", "/tmp/song/effects/lyrics.ecproj", "--comp", "Lyrics", "--out", "/tmp/song/effects/lyrics.mov", "--format", "prores"]
        );

        let frame_args = build_effectcraft_render_frame_args(missing, Some("Lyrics"), 1.5, 960, true, Path::new("/tmp/f.png"));
        assert_eq!(
            frame_args,
            vec![
                "render-frame",
                "/tmp/song/effects/lyrics.ecproj",
                "--comp",
                "Lyrics",
                "--time",
                "1.5",
                "--max-side",
                "960",
                "--transparent",
                "--out",
                "/tmp/f.png",
                "--json"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn filmcraft_and_effectcraft_cli_runners_parse_and_fail_noisily() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = std::env::temp_dir().join(format!("magic-craft-cli-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let proj = tmp.join("Song.fcproj");
        std::fs::write(&proj, b"{}").unwrap();
        let mock_fc = tmp.join("filmcraft-cli");
        let fc_script = concat!("#!/bin/sh\ncat >/dev/null\n", "printf '%s\\n' '{\"line\":1,\"id\":\"file.import\",\"ok\":true,\"result\":{\"items\":[7]}}'\n");
        std::fs::write(&mock_fc, fc_script).unwrap();
        std::fs::set_permissions(&mock_fc, std::fs::Permissions::from_mode(0o755)).unwrap();

        let (fc_args, fc_in) = build_filmcraft_run_invocation(Some(&proj), false, true, &[json!({"id": "file.import", "params": {"paths": ["a.mov"]}})]);
        let fc_out = run_filmcraft_cli_with_exe(&mock_fc, &fc_args, Some(&fc_in)).unwrap();
        assert_eq!(parse_filmcraft_run_output(&fc_out).unwrap()[0]["items"], json!([7]));

        let mock_ec_fail = tmp.join("effectcraft-cli");
        std::fs::write(&mock_ec_fail, "#!/bin/sh\nprintf '%s\\n' '{\"ok\":false,\"command\":\"layer.newText\",\"error\":\"missing font\"}' >&1\nexit 0\n")
            .unwrap();
        std::fs::set_permissions(&mock_ec_fail, std::fs::Permissions::from_mode(0o755)).unwrap();
        let ec_stdout = run_effectcraft_cli_with_exe(&mock_ec_fail, &["run".into()], Some("")).unwrap();
        let err = parse_effectcraft_run_output(&ec_stdout).unwrap_err();
        assert!(err.contains("missing font"), "expected noisy error, got: {err}");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
