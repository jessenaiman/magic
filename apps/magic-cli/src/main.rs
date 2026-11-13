//! `magic-cli`: headless CLI, live `app` control client, and stdio MCP server over `magic_engine::Session`
//! (`magic --control 9875`), aligned with `designcraft/docs/mcp.md` and `designcraft/docs/agents.md`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod subcommands;

use std::io::{BufRead, Write};

use clap::{Args, CommandFactory, Parser, Subcommand};
use serde_json::{Value, json};
use subcommands::{
    AnimationArgs, CaptionsArgs, ControlsArgs, EffectArgs, ExportArgs, FilmArgs, FinishArgs, InterchangeArgs, MarkersArgs, MeshCliArgs, StoryboardArgs,
    StudioArgs, TimelineCliArgs,
};

const AFTER_HELP: &str = "\
CREATE:     magic-cli new [existing_project] --audio song.wav --name \"My Song\"
STORYBOARD: magic-cli storyboard status | open | new | add-section <name> | add-element <section> <name> | export [out]
STUDIO:     magic-cli studio map-tracks --map lyrics:V3,V5:chorus --map motion:V4:beats
CONTROLS:   magic-cli controls [filter] [--stage studio] [--live]
FINISH:     magic-cli finish <op> [key=val ...] | magic-cli export [out.mp4] --codec prores
INTEROP:    magic-cli interchange export --format otio | magic-cli lottie export | magic-cli import-aep in.aep
MCP:        magic-cli mcp [--connect 127.0.0.1:9875 | --headless]
LIVE:       magic --control 9875, then magic-cli app --port 9875 ui.inspect";

#[derive(Parser)]
#[command(
    name = "magic-cli",
    about = "Drive Magic Studio headlessly or over live `--control 9875`",
    after_help = AFTER_HELP
)]
pub struct Cli {
    /// Open this `.magic` project first (headless)
    #[arg(long, global = true)]
    pub project: Option<String>,
    /// Save the project back to --project when done
    #[arg(long, global = true)]
    pub save: bool,
    /// Save the project to this path when done
    #[arg(long, global = true)]
    pub save_as: Option<String>,
    /// Connect to a running `magic --control <PORT>` window (`127.0.0.1:9875` or `9875`)
    #[arg(long, visible_aliases = ["bridge", "control"], global = true)]
    pub connect: Option<String>,
    /// Extra font directories forwarded as `EFFECTCRAFT_FONT_DIRS`
    #[arg(long, global = true)]
    pub font_dirs: Option<String>,
    /// Preview mutations without applying them
    #[arg(long, global = true)]
    pub dry_run: bool,
    /// JSON output (default)
    #[arg(long, global = true)]
    pub json: bool,
    /// Confine headless reads to this folder (repeatable)
    #[arg(long = "automation-read-root", global = true)]
    pub automation_read_roots: Vec<String>,
    /// Confine headless writes to this folder (repeatable)
    #[arg(long = "automation-write-root", global = true)]
    pub automation_write_roots: Vec<String>,
    /// Run an engine or UI command by ID, like the exec subcommand
    #[arg(long = "command")]
    command_id: Option<String>,
    /// JSON parameters for --command
    #[arg(long, requires = "command_id")]
    params: Option<String>,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Args)]
struct NewArgs {
    /// Optional existing project directory or archive to import into the new project
    existing_project: Option<String>,
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    root: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    audio: Option<String>,
    #[arg(long)]
    transcript: Option<String>,
    #[arg(long)]
    mode: Option<String>,
    #[arg(long)]
    overwrite: bool,
    arguments: Vec<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new song project (`--audio` transcribes timestamps, creates the FilmCraft project, and starts `storyboard.deckcraft`)
    New(NewArgs),
    /// Direct pass-through to live `filmcraft --control 9876` or `filmcraft-cli --project <song.fcproj> [--save] exec`
    Film(FilmArgs),
    /// Direct pass-through to live `effectcraft --control 9877` or `effectcraft-cli exec <id> --params '<json>' ... --json`
    Effect(EffectArgs),
    /// One-step Animation promotion (`promote`), host registry (`hosts`), and reverse Studio lookup (`locate`)
    Animation(AnimationArgs),
    /// Ergonomic Studio commands (`map-tracks` batch track-to-group mapping into Lyrics & Motion)
    Studio(StudioArgs),
    /// Thin Storyboard commands (`status`, `open`, `new`, `add-section`, `add-element`, `export`)
    Storyboard(StoryboardArgs),
    /// List engine commands (`id` and `label`; `describe` shows one in full)
    Commands { filter: Option<String> },
    /// List all `--control` RPC methods (`:9875`) and stage commands
    Controls(ControlsArgs),
    /// Describe one command as JSON
    Describe { id: String },
    /// Inspect project, storyboard, timeline, or doctor state as JSON
    Inspect { target: Option<String> },
    /// Show live Magic Studio window or headless workspace status
    Status,
    /// Bidirectional `OTIO` / `FCPXML` / `XML` / `EDL` / `AAF` / `OMF` / `ALE` sequence interchange (`T03`)
    Interchange(InterchangeArgs),
    /// Full `craft-timeline` operations (`inspect`, `detect`, `export`, `import`, `timecode`, `ale`, `edl`)
    Timeline(TimelineCliArgs),
    /// Pure-Rust `craft-mesh` 3D scene inspection & conversion (`inspect`, `convert` across `.glb`, `.gltf`, `.obj`, `.fbx`, `.stl`, `.ply`, `.3mf`)
    Mesh(MeshCliArgs),
    /// Execute a FilmCraft Studio finishing operation (`T32`–`T42`: edit, trim, color, FX, audio, mixer, titles)
    Finish(FinishArgs),
    /// Export master film via FilmCraft's pure-Rust codecs (`H.264`, `HEVC`, `ProRes`, `DNxHR`, `APV`, `AV1`, `MXF`, `EXR`, `Opus`) (`T42`)
    Export(ExportArgs),
    /// Bidirectional `V3` Lyrics ↔ `C1` FilmCraft broadcast captions (`Subtitle`, `CEA-608`, `CEA-708`, `Teletext`) (`T26`)
    Captions(CaptionsArgs),
    /// Manage timeline markers, 64-swatch custom colors, and 15-column review CSV export (`T35`)
    Markers(MarkersArgs),
    /// Run one command (`exec <id> [key=value ...]` or `--params '<json>'`)
    Exec {
        command: String,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Run commands from a JSONL file or stdin (`-`), one `{"id","params"}` per line
    Run {
        #[arg(default_value = "-")]
        file: String,
        /// Continue executing subsequent lines when a command fails
        #[arg(long)]
        keep_going: bool,
    },
    /// Run the Model Context Protocol server on stdio (`--connect 127.0.0.1:9875` or `--headless`)
    Mcp {
        #[arg(long)]
        headless: bool,
    },
    /// Call a running `magic --control <PORT>` window directly (RFC-0012 `app` subcommand)
    App {
        /// Control port or `127.0.0.1:PORT` (defaults to `--connect` or `127.0.0.1:9875`)
        #[arg(long)]
        port: Option<String>,
        /// Explicit control-protocol method name (e.g. `ui.inspect`, `ui.screenshot`)
        #[arg(long)]
        method: Option<String>,
        /// Command ID or control method when `--method` is omitted
        target: Option<String>,
        /// Optional JSON parameters or `key=value` pairs
        arguments: Vec<String>,
    },
    /// Control a live desktop window through `--connect 127.0.0.1:9875`
    Ui {
        action: String,
        target: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    #[command(external_subcommand)]
    Registry(Vec<String>),
}

pub(crate) fn emit(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn dry_result(id: &str, params: &Value) -> Value {
    json!({ "dry_run": true, "id": id, "params": params })
}

pub(crate) fn parse_params(tokens: &[String], raw: Option<&str>) -> Result<Value, String> {
    let mut params = match raw {
        Some(t) => serde_json::from_str::<Value>(t).map_err(|e| format!("--params: {e}"))?,
        None => json!({}),
    };
    if !params.is_object() {
        return Err("--params must be a JSON object".into());
    }
    for token in tokens {
        if let Some(obj) = token.strip_prefix('{')
            && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&format!("{{{obj}"))
        {
            for (k, v) in map {
                params[k] = v;
            }
            continue;
        }
        let (k, v) = token.split_once('=').ok_or_else(|| format!("Expected key=value, got {token:?}"))?;
        params[k] = serde_json::from_str(v).unwrap_or_else(|_| json!(v));
    }
    Ok(params)
}

fn call_live_control(connect: Option<&str>, method: &str, params: Value) -> Result<Value, String> {
    let spec = connect.ok_or("Live UI commands require --connect 127.0.0.1:9875 (or --control <PORT>)")?;
    let mut remote = magic_mcp::Remote::new(magic_mcp::normalize_control_addr(spec), None);
    magic_mcp::Backend::execute(&mut remote, method, params)
}

pub(crate) struct Target {
    pub(crate) session: Option<magic_engine::Session>,
    pub(crate) connect: Option<String>,
    pub(crate) project: Option<String>,
    pub(crate) save: bool,
    pub(crate) save_as: Option<String>,
    pub(crate) dry: bool,
}

impl Target {
    pub(crate) fn call(&mut self, id: &str, params: Value) -> Result<Value, String> {
        if self.connect.is_some() || magic_mcp::is_live_shell_method(id) {
            return call_live_control(self.connect.as_deref(), id, params);
        }
        let session = self.session.as_mut().ok_or("no headless session: --connect runs commands in the live app")?;
        session.execute(id, params).map_err(|e| e.to_string())
    }

    pub(crate) fn call_unless_dry(&mut self, id: &str, params: Value) -> Result<Value, String> {
        if self.dry { Ok(dry_result(id, &params)) } else { self.call(id, params) }
    }

    pub(crate) fn save_project(&mut self) -> Result<(), String> {
        if self.dry {
            return Ok(());
        }
        let path = if self.save_as.is_some() { self.save_as.clone() } else { self.project.clone() };
        if self.save_as.is_some() || self.save {
            let params = path.map_or_else(|| json!({}), |path| json!({ "path": path }));
            self.call("file.save", params)?;
        }
        Ok(())
    }

    pub(crate) fn run(&mut self, id: &str, params: Value) -> Result<(), String> {
        emit(&self.call_unless_dry(id, params)?);
        self.save_project()
    }
}

fn put_opt_str(params: &mut Value, key: &str, val: Option<String>) {
    if let Some(v) = val {
        params[key] = json!(v);
    }
}

fn build_new_params(args: NewArgs) -> Result<Value, String> {
    let mut extra_tokens = args.arguments;
    let existing_project = match args.existing_project {
        Some(tok) if tok.contains('=') || tok.trim_start().starts_with('{') => {
            extra_tokens.insert(0, tok);
            None
        }
        other => other,
    };
    let mut params = parse_params(&extra_tokens, None)?;
    put_opt_str(&mut params, "existing_project", existing_project);
    put_opt_str(&mut params, "path", args.path);
    put_opt_str(&mut params, "root", args.root);
    put_opt_str(&mut params, "name", args.name);
    put_opt_str(&mut params, "audio", args.audio);
    put_opt_str(&mut params, "transcript", args.transcript);
    put_opt_str(&mut params, "mode", args.mode);
    if args.overwrite {
        params["overwrite"] = json!(true);
    }
    let is_anim = params.get("mode").and_then(Value::as_str) == Some("animation");
    let has_existing = params.get("existing_project").and_then(Value::as_str).is_some_and(|s| !s.trim().is_empty());
    if !is_anim && !has_existing && params.get("audio").and_then(Value::as_str).is_none_or(|s| s.trim().is_empty()) {
        return Err(concat!(
            "magic-cli new requires --audio <SONG_FILE> (or audio=<SONG_FILE>) so Magic can transcribe ",
            "timestamps, sync the FilmCraft lyric track, and start the storyboard."
        )
        .into());
    }
    Ok(params)
}

fn run_new(target: &mut Target, args: NewArgs) -> Result<(), String> {
    let params = build_new_params(args)?;
    target.run("project.new", params)
}

fn list_commands(target: &mut Target, filter: Option<String>) -> Result<(), String> {
    let list = target.call("command.list", json!({ "filter": filter.unwrap_or_default() }))?;
    let rows = list["commands"].as_array().map_or(&[][..], Vec::as_slice);
    for row in rows {
        println!("{}  {}", row["id"].as_str().unwrap_or_default(), row["label"].as_str().unwrap_or_default());
    }
    Ok(())
}

fn inspect(target: &mut Target, what: Option<String>) -> Result<(), String> {
    let value = match what.as_deref().unwrap_or("all") {
        "project" => target.call("workshop.status", json!({})).or_else(|_| target.call("project.catalog", json!({})))?,
        "storyboard" => target.call("storyboard.status", json!({}))?,
        "timeline" | "sequence" => target.call("video.timeline", json!({}))?,
        "doctor" => target.call("app.doctor", json!({}))?,
        "ui" => target.call("ui.inspect", json!({}))?,
        "effects" => json!({
            "words": target.call("words.effects", json!({})).unwrap_or(Value::Null),
            "motion": target.call("motion.effects", json!({})).unwrap_or(Value::Null),
        }),
        "all" => {
            let proj = target.call("workshop.status", json!({})).ok();
            let ui_live = if target.connect.is_some() { target.call("ui.inspect", json!({})).ok() } else { None };
            json!({
                "ok": true,
                "mode": if target.connect.is_some() { "live" } else { "headless" },
                "ui": ui_live,
                "project": proj,
                "storyboard": target.call("storyboard.status", json!({})).ok(),
                "timeline": target.call("video.timeline", json!({})).ok(),
                "doctor": target.call("app.doctor", json!({})).ok(),
            })
        }
        other => return Err(format!("inspect: unknown `{other}` (all | ui | project | storyboard | timeline | effects | doctor)")),
    };
    emit(&value);
    Ok(())
}

fn auto_detect_live_magic() -> Option<String> {
    let sock: std::net::SocketAddr = magic_mcp::DEFAULT_CONTROL_ADDR.parse().ok()?;
    std::net::TcpStream::connect_timeout(&sock, std::time::Duration::from_millis(45)).ok()?;
    Some(magic_mcp::DEFAULT_CONTROL_ADDR.to_owned())
}

fn run_file(target: &mut Target, file: &str, keep_going: bool) -> Result<(), String> {
    let reader: Box<dyn BufRead> = if file == "-" {
        Box::new(std::io::stdin().lock())
    } else {
        Box::new(std::io::BufReader::new(std::fs::File::open(file).map_err(|e| format!("{file}: {e}"))?))
    };
    let mut prior_results = Vec::new();
    let mut first_err: Option<String> = None;
    for (n, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let text = line.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let item: Value = serde_json::from_str(text).map_err(|e| format!("line {}: {e}", n + 1))?;
        let id = item["id"].as_str().or_else(|| item["command"].as_str()).unwrap_or_default();
        let raw_params = item.get("params").cloned().unwrap_or_else(|| json!({}));
        let params = magic_mcp::interpolate_batch_params(&raw_params, &prior_results);
        match target.call_unless_dry(id, params) {
            Ok(result) => {
                prior_results.push(result.clone());
                println!("{}", json!({ "line": n + 1, "id": id, "ok": true, "result": result }));
            }
            Err(err) => {
                prior_results.push(Value::Null);
                println!("{}", json!({ "line": n + 1, "id": id, "ok": false, "error": err }));
                if !keep_going {
                    return Err(err);
                }
                if first_err.is_none() {
                    first_err = Some(err);
                }
            }
        }
    }
    target.save_project()?;
    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

fn ui_action(action: &str, target: Option<String>, mut args: Vec<String>, raw: Option<&str>) -> Result<(String, Value), String> {
    let method = match action {
        "inspect" => "ui.inspect".to_owned(),
        "state" => "ui.state".to_owned(),
        "drafts" => "ui.drafts".to_owned(),
        "elements" => "ui.elements".to_owned(),
        "view" | "workspace" => "ui.set".to_owned(),
        "locate" => "ui.viewer.locate".to_owned(),
        "hit" => "ui.viewer.hit".to_owned(),
        "timeline-locate" => "ui.timeline.locate".to_owned(),
        "timeline-hit" => "ui.timeline.hit".to_owned(),
        "render-frame" => "render.frame".to_owned(),
        "menu" => "ui.menu.list".to_owned(),
        "keymap" => "ui.keymap.press".to_owned(),
        "playback" => "ui.playback".to_owned(),
        "perf" => "perf.stats".to_owned(),
        "quit" => "app.quit".to_owned(),
        id if id.contains('.') => id.to_owned(),
        id => format!("ui.{id}"),
    };
    if let Some(target) = target {
        let key = match (action, method.as_str()) {
            ("view", _) | (_, "ui.set_view") => Some("view"),
            ("workspace", _) => Some("studioWorkspace"),
            ("keymap", _) => Some("chord"),
            ("elements", _) => Some("prefix"),
            (_, "ui.focus" | "ui.click") => Some("id"),
            (_, "ui.playback") => Some("action"),
            (_, "ui.key") => Some("key"),
            (_, "ui.type" | "ui.text") => Some("text"),
            (_, "ui.screenshot" | "render.frame") => Some("path"),
            _ => None,
        };
        let bare = !target.contains('=') && !target.starts_with('{');
        args.insert(0, key.filter(|_| bare).map_or_else(|| target.clone(), |key| format!("{key}={target}")));
    }
    Ok((method, parse_params(&args, raw)?))
}

fn run_ui(target: &mut Target, action: &str, value: Option<String>, params: Option<&str>, arguments: Vec<String>) -> Result<(), String> {
    if target.connect.is_none() {
        return Err("Live UI commands require --connect 127.0.0.1:9875 (or --control <PORT>)".into());
    }
    let (method, params) = ui_action(action, value, arguments, params)?;
    let result = if target.dry { dry_result(&method, &params) } else { call_live_control(target.connect.as_deref(), &method, params)? };
    emit(&result);
    Ok(())
}

fn run_app_call(target: &mut Target, port: Option<String>, method: Option<String>, positional: Option<String>, arguments: Vec<String>) -> Result<(), String> {
    let endpoint = port.or_else(|| target.connect.clone()).unwrap_or_else(|| magic_mcp::DEFAULT_CONTROL_ADDR.to_owned());
    let chosen = method.clone().or(positional.clone()).ok_or("magic-cli app requires `--method <METHOD>` or `<COMMAND_OR_METHOD>`")?;
    let mut tokens = arguments;
    if method.is_some()
        && let Some(pos_json) = positional
    {
        tokens.insert(0, pos_json);
    }
    let raw_json = if tokens.len() == 1 && tokens[0].trim_start().starts_with('{') { Some(tokens.remove(0)) } else { None };
    let params = parse_params(&tokens, raw_json.as_deref())?;
    let result = if target.dry { dry_result(&chosen, &params) } else { call_live_control(Some(&endpoint), &chosen, params)? };
    emit(&result);
    Ok(())
}

fn resolve_shorthand(tokens: &[String]) -> Result<(String, Vec<String>), String> {
    let known: Vec<&str> = magic_engine::command_specs().iter().map(|c| c.id).collect();
    let words: Vec<&String> = tokens.iter().take_while(|t| !t.contains('=') && !t.starts_with('-') && !t.starts_with('{')).collect();
    for n in (1..=words.len()).rev() {
        let raw_candidate = words[..n].iter().map(|w| w.as_str()).collect::<Vec<_>>().join(".");
        if known.contains(&raw_candidate.as_str()) {
            return Ok((raw_candidate, tokens[n..].to_vec()));
        }
        let snake_candidate = words[..n].iter().map(|w| w.replace('-', "_")).collect::<Vec<_>>().join(".");
        if known.contains(&snake_candidate.as_str()) {
            return Ok((snake_candidate, tokens[n..].to_vec()));
        }
    }
    Err(format!("Unknown command '{}'. Run `magic-cli commands` or `magic-cli controls`.", tokens.join(" ")))
}

fn run_shorthand(target: &mut Target, tokens: &[String]) -> Result<(), String> {
    let (id, rest) = resolve_shorthand(tokens)?;
    if rest.iter().any(|t| t == "--help" || t == "-h")
        && let Some(command) = magic_engine::find_command(&id)
    {
        emit(&command.describe());
        return Ok(());
    }
    let mut pairs = Vec::new();
    let mut raw = None;
    let mut iter = rest.into_iter();
    while let Some(token) = iter.next() {
        if token == "--params" {
            raw = iter.next();
        } else if let Some(value) = token.strip_prefix("--params=") {
            raw = Some(value.to_owned());
        } else {
            pairs.push(token);
        }
    }
    let params = parse_params(&pairs, raw.as_deref())?;
    target.run(&id, params)
}

fn describe(id: &str) -> Result<(), String> {
    let command = magic_engine::find_command(id).ok_or_else(|| format!("Unknown command '{id}'"))?;
    emit(&command.describe());
    Ok(())
}

fn apply_font_overrides(cli: &Cli, session: &mut Option<magic_engine::Session>) {
    if let (Some(s), Some(dirs)) = (session.as_mut(), cli.font_dirs.as_ref()) {
        let _ = s.execute("prefs.set", json!({"key": "lyrics.font_dirs", "value": dirs}));
    }
}

fn run(cli: Cli) -> Result<(), String> {
    if let (None, Some(Cmd::Mcp { headless })) = (&cli.command_id, &cli.command) {
        return magic_mcp::run_mcp_server(
            "magic-cli",
            cli.connect.as_deref(),
            *headless,
            &cli.automation_read_roots,
            &cli.automation_write_roots,
            cli.project.as_deref(),
        );
    }

    if cli.connect.is_some() && (!cli.automation_read_roots.is_empty() || !cli.automation_write_roots.is_empty()) {
        return Err("Pass --automation-read-root / --automation-write-root to `magic --control`, not to `--connect`.".into());
    }

    let roots = magic_engine::AutomationRoots::from_args(&cli.automation_read_roots, &cli.automation_write_roots)?;
    let effective_connect = if cli.project.is_some() {
        None
    } else {
        cli.connect
            .clone()
            .or_else(|| if cli.automation_read_roots.is_empty() && cli.automation_write_roots.is_empty() { auto_detect_live_magic() } else { None })
    };
    let live = effective_connect.is_some();
    let mut session = (!live).then(|| magic_engine::Session::user_session().with_automation_roots(roots));
    apply_font_overrides(&cli, &mut session);
    if let (Some(path), Some(session)) = (cli.project.as_deref(), session.as_mut()) {
        session.execute("file.open", json!({ "path": path })).map_err(|e| e.to_string())?;
    }
    let mut target = Target { session, connect: effective_connect, project: cli.project, save: cli.save, save_as: cli.save_as, dry: cli.dry_run };
    match (cli.command_id, cli.command) {
        (Some(_), Some(_)) => Err("Use --command or a subcommand, not both.".into()),
        (Some(id), None) => {
            let params = parse_params(&[], cli.params.as_deref())?;
            target.run(&id, params)
        }
        (None, Some(command)) => match command {
            Cmd::New(args) => run_new(&mut target, args),
            Cmd::Film(args) => subcommands::run_film(&mut target, args),
            Cmd::Effect(args) => subcommands::run_effect(&mut target, args),
            Cmd::Animation(args) => subcommands::run_animation(&mut target, args),
            Cmd::Studio(args) => subcommands::run_studio(&mut target, args),
            Cmd::Storyboard(args) => subcommands::run_storyboard(&mut target, args),
            Cmd::Commands { filter } => list_commands(&mut target, filter),
            Cmd::Controls(args) => subcommands::run_controls(&mut target, args),
            Cmd::Describe { id } => describe(&id),
            Cmd::Inspect { target: what } => inspect(&mut target, what),
            Cmd::Status => inspect(&mut target, None),
            Cmd::Interchange(args) => subcommands::run_interchange(&mut target, args),
            Cmd::Timeline(args) => subcommands::run_timeline_cli(&mut target, args),
            Cmd::Mesh(args) => subcommands::run_mesh_cli(&mut target, args),
            Cmd::Finish(args) => subcommands::run_finish(&mut target, args),
            Cmd::Export(args) => subcommands::run_export(&mut target, args),
            Cmd::Captions(args) => subcommands::run_captions(&mut target, args),
            Cmd::Markers(args) => subcommands::run_markers(&mut target, args),
            Cmd::Exec { command, params, arguments } => {
                let params = parse_params(&arguments, params.as_deref())?;
                target.run(&command, params)
            }
            Cmd::Run { file, keep_going } => run_file(&mut target, &file, keep_going),
            Cmd::Mcp { .. } => Ok(()),
            Cmd::App { port, method, target: pos, arguments } => run_app_call(&mut target, port, method, pos, arguments),
            Cmd::Ui { action, target: value, params, arguments } => run_ui(&mut target, &action, value, params.as_deref(), arguments),
            Cmd::Registry(tokens) => run_shorthand(&mut target, &tokens),
        },
        (None, None) => {
            Cli::command().print_help().map_err(|e| e.to_string())?;
            println!();
            Ok(())
        }
    }
}

fn main() {
    let cli = Cli::parse();
    if cli.connect.is_some() && cli.project.is_some() {
        let _ = writeln!(
            std::io::stderr(),
            "note: --project supplied; running headlessly against {} instead of --connect",
            cli.project.as_deref().unwrap_or_default()
        );
    }
    if let Err(err) = run(cli) {
        let _ = writeln!(std::io::stderr(), "{err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use serde_json::json;

    use super::{AFTER_HELP, Cli, Cmd, build_new_params};
    use crate::subcommands::{FilmCmd, StudioCmd, build_film_call, build_storyboard_call, parse_studio_mappings};

    #[test]
    fn cli_new_existing_project_and_removed_wizard_archive_flags() {
        assert!(!AFTER_HELP.contains("ARCHIVE:"));
        assert!(!AFTER_HELP.contains("--wizard"));

        let parsed = Cli::try_parse_from(["magic-cli", "new", "./legacy-song", "--audio", "song.wav", "--name", "My Song"]).unwrap();
        let Some(Cmd::New(args)) = parsed.command else {
            panic!("expected Cmd::New");
        };
        let params = build_new_params(args).unwrap();
        assert_eq!(params["existing_project"], json!("./legacy-song"));
        assert_eq!(params["audio"], json!("song.wav"));
        assert_eq!(params["name"], json!("My Song"));

        assert!(Cli::try_parse_from(["magic-cli", "new", "--wizard", "--audio", "song.wav"]).is_err());
    }

    #[test]
    fn cli_studio_map_tracks_and_storyboard_subcommands_forward_cleanly() {
        let studio = Cli::try_parse_from(["magic-cli", "studio", "map-tracks", "--map", "lyrics:V3,V5:chorus", "--map", "motion:V4:beats"]).unwrap();
        let Some(Cmd::Studio(s_args)) = studio.command else {
            panic!("expected Cmd::Studio");
        };
        let StudioCmd::MapTracks(m_args) = s_args.command;
        let payload = parse_studio_mappings(&m_args).unwrap();
        assert_eq!(
            payload,
            json!({
                "mappings": [
                    {"target": "lyrics", "tracks": ["V3", "V5"], "group": "chorus"},
                    {"target": "motion", "tracks": ["V4"], "group": "beats"}
                ]
            })
        );

        for (argv, expected_id) in [
            (vec!["magic-cli", "storyboard", "status"], "storyboard.status"),
            (vec!["magic-cli", "storyboard", "open"], "storyboard.open"),
            (vec!["magic-cli", "storyboard", "new"], "storyboard.new"),
            (vec!["magic-cli", "storyboard", "add-section", "Chorus"], "storyboard.add_section"),
            (vec!["magic-cli", "storyboard", "add-element", "Chorus", "LeadHero"], "storyboard.add_element"),
            (vec!["magic-cli", "storyboard", "export", "out/spreads"], "storyboard.export"),
        ] {
            let cli = Cli::try_parse_from(argv).unwrap();
            let Some(Cmd::Storyboard(sb)) = cli.command else {
                panic!("expected Cmd::Storyboard");
            };
            let (cmd_id, _) = build_storyboard_call(sb.command).unwrap();
            assert_eq!(cmd_id, expected_id);
        }

        let film_cli = Cli::try_parse_from(["magic-cli", "film", "exec", "timeline.razor", "seconds=3.5"]).unwrap();
        let Some(Cmd::Film(f_args)) = film_cli.command else {
            panic!("expected Cmd::Film");
        };
        let (f_id, f_payload) = build_film_call(f_args.command).unwrap();
        assert_eq!(f_id, "video.exec");
        assert_eq!(f_payload["command"], json!("timeline.razor"));
        assert_eq!(f_payload["params"], json!({"seconds": 3.5}));

        let film_ro = Cli::try_parse_from(["magic-cli", "film", "exec", "sequence.inspect", "--no-save"]).unwrap();
        let Some(Cmd::Film(ro_args)) = film_ro.command else {
            panic!("expected Cmd::Film");
        };
        let FilmCmd::Exec { .. } = &ro_args.command;
        let (_, ro_payload) = build_film_call(ro_args.command).unwrap();
        assert_eq!(ro_payload["save"], json!(false));
    }
}
