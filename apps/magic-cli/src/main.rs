//! `magic-cli`: headless CLI and MCP server over `magic::Session` and live `--control`
//! (`magic --control 9875`), plus sibling bridges to `FilmCraft` (`:9876` +
//! `FILMCRAFT_CONTROL_TOKEN`) and `EffectCraft` (`:9877` + `mcp --autosave`).

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod subcommands;

use std::io::{BufRead, Write};

use clap::{Args, CommandFactory, Parser, Subcommand};
use serde_json::{Value, json};
use subcommands::{
    BridgeCmd, CaptionsArgs, ControlsArgs, ExportArgs, FinishArgs, ImportAepArgs, InterchangeArgs,
    LottieArgs, MarkersArgs,
};

const AFTER_HELP: &str = "\
CREATE:   magic-cli new --audio song.wav --name \"My Song\" [--wizard]
CONTROLS: magic-cli controls [filter] [--stage studio] [--live]
BRIDGE:   magic-cli bridge status | preflight | sync | link <p.ecproj> | fc <cmd> | ec <cmd>
FINISH:   magic-cli finish <op> [key=val ...] | magic-cli export [out.mp4] --codec prores
INTEROP:  magic-cli interchange export --format otio | magic-cli lottie export | magic-cli import-aep in.aep
MCP:      magic-cli mcp [--connect 127.0.0.1:9875 | --headless]
LIVE:     magic --control 9875, then magic-cli --connect 127.0.0.1:9875 ui inspect";

#[derive(Parser)]
#[command(name = "magic-cli", about = "Drive Magic Studio, FilmCraft (:9876), and EffectCraft (:9877) headlessly or over live `--control`", after_help = AFTER_HELP)]
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
    /// Override FilmCraft `--control` port (`FILMCRAFT_CONTROL_PORT`, default `9876`)
    #[arg(long, global = true)]
    pub filmcraft_port: Option<u16>,
    /// Override EffectCraft `--control` port (`EFFECTCRAFT_CONTROL_PORT`, default `9877`)
    #[arg(long, global = true)]
    pub effectcraft_port: Option<u16>,
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
    wizard: bool,
    #[arg(long)]
    style: Option<String>,
    #[arg(long)]
    world: Option<String>,
    #[arg(long)]
    character: Option<String>,
    #[arg(long)]
    motion: Option<String>,
    #[arg(long)]
    mode: Option<String>,
    #[arg(long)]
    overwrite: bool,
    arguments: Vec<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new song project (`--audio` transcribes timestamps, syncs FilmCraft lyrics, and initializes printable `storyboard.pdf`)
    New(NewArgs),
    /// List engine commands (`id` and `label`; `describe` shows one in full)
    Commands { filter: Option<String> },
    /// List all `--control` RPC methods, ports (`:9875`, `:9876`, `:9877`), and stage commands
    Controls(ControlsArgs),
    /// Describe one command as JSON
    Describe { id: String },
    /// Inspect project, storyboard, timeline, bridge (`:9876`/`:9877`), preflight, history, or doctor state as JSON
    Inspect { target: Option<String> },
    /// Cross-app `FilmCraft` (`:9876`) × `EffectCraft` (`:9877`) Dynamic Link, preflight, history, templates, and `--control` bridge
    #[command(subcommand)]
    Bridge(BridgeCmd),
    /// Bidirectional `OTIO` / `FCPXML` / `XML` / `EDL` / `AAF` / `OMF` / `ALE` sequence interchange (`T03`)
    Interchange(InterchangeArgs),
    /// Two-way Lottie (`.json`) and dotLottie (`.lottie`) export & import (`T04`)
    Lottie(LottieArgs),
    /// Clean-room After Effects `.aep` / `.aepx` import with dry-run loss preview (`T05`)
    ImportAep(ImportAepArgs),
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
    let mut remote = magic_mcp::Remote::new(magic_mcp::normalize_control_addr(spec), magic::workspace::control_token());
    magic_mcp::Backend::execute(&mut remote, method, params)
}

pub(crate) struct Target {
    pub(crate) session: Option<magic::Session>,
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

fn prompt_optional(label: &str) -> Option<String> {
    let _ = write!(std::io::stderr(), "{label}: ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_ok() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    None
}

fn put_opt_str(params: &mut Value, key: &str, val: Option<String>) {
    if let Some(v) = val {
        params[key] = json!(v);
    }
}

fn build_wizard_params(base: Option<Value>, args: &NewArgs) -> Value {
    let mut wiz = base.filter(Value::is_object).unwrap_or_else(|| json!({}));
    let wizard = args.wizard;
    if let Some(v) = args.style.clone().or_else(|| wizard.then(|| prompt_optional("Wizard [1/4] Overall Visual Style (D01, optional)")).flatten()) {
        wiz["style"] = json!(v);
    }
    if let Some(v) = args.world.clone().or_else(|| wizard.then(|| prompt_optional("Wizard [2/4] Master World Location (L01, optional)")).flatten()) {
        wiz["world"] = json!(v.clone());
        wiz["world_prompt"] = json!(v);
    }
    if let Some(v) = args.character.clone().or_else(|| wizard.then(|| prompt_optional("Wizard [3/4] Lead Character Prompt (C01, optional)")).flatten()) {
        wiz["character"] = json!(v);
    }
    if let Some(v) = args.motion.clone().or_else(|| wizard.then(|| prompt_optional("Wizard [4/4] Song-Action Motion Spec (M01, optional)")).flatten()) {
        wiz["motion"] = json!(v);
    }
    wiz
}

fn run_new(target: &mut Target, args: NewArgs) -> Result<(), String> {
    let mut params = parse_params(&args.arguments, None)?;
    put_opt_str(&mut params, "path", args.path.clone());
    put_opt_str(&mut params, "root", args.root.clone());
    put_opt_str(&mut params, "name", args.name.clone());
    put_opt_str(&mut params, "audio", args.audio.clone());
    put_opt_str(&mut params, "transcript", args.transcript.clone());
    put_opt_str(&mut params, "mode", args.mode.clone());
    if args.overwrite {
        params["overwrite"] = json!(true);
    }
    let is_anim = params.get("mode").and_then(Value::as_str) == Some("animation");
    if !is_anim && params.get("audio").and_then(Value::as_str).is_none_or(|s| s.trim().is_empty()) {
        return Err(
            "magic-cli new requires --audio <SONG_FILE> (or audio=<SONG_FILE>) so Magic can transcribe timestamps, sync the FilmCraft lyric track, and start the storyboard.".into(),
        );
    }
    let wiz = build_wizard_params(params.get("wizard").cloned(), &args);
    if wiz.as_object().is_some_and(|m| !m.is_empty()) {
        params["wizard"] = wiz;
    }
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
        "project" => target.call("workshop.status", json!({}))?,
        "storyboard" => target.call("storyboard.check", json!({}))?,
        "timeline" | "sequence" => target.call("video.timeline", json!({}))?,
        "bridge" => target.call("bridge.status", json!({}))?,
        "preflight" => target.call("bridge.preflight", json!({"action": "check"}))?,
        "history" => target.call("bridge.history", json!({"op": "list"}))?,
        "doctor" => target.call("app.doctor", json!({}))?,
        "effects" => json!({
            "words": target.call("words.effects", json!({})).unwrap_or(Value::Null),
            "motion": target.call("motion.effects", json!({})).unwrap_or(Value::Null),
        }),
        "all" => json!({
            "project": target.call("workshop.status", json!({}))?,
            "storyboard": target.call("storyboard.check", json!({})).unwrap_or(Value::Null),
            "timeline": target.call("video.timeline", json!({})).unwrap_or(Value::Null),
            "bridge": target.call("bridge.status", json!({})).unwrap_or(Value::Null),
        }),
        other => return Err(format!("inspect: unknown `{other}` (all | project | storyboard | timeline | bridge | preflight | history | effects | doctor)")),
    };
    emit(&value);
    Ok(())
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
        "filmcraft" | "fc" => "ui.filmcraft".to_owned(),
        "effectcraft" | "ec" => "ui.effectcraft".to_owned(),
        "craft" => "ui.craft".to_owned(),
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
            ("filmcraft" | "fc" | "effectcraft" | "ec", _) => Some("method"),
            ("craft", _) => Some("app"),
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

fn resolve_shorthand(tokens: &[String]) -> Result<(String, Vec<String>), String> {
    let known: Vec<&str> = magic::command_specs().iter().map(|c| c.id).collect();
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
        && let Some(command) = magic::find_command(&id)
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
    let command = magic::find_command(id).ok_or_else(|| format!("Unknown command '{id}'"))?;
    emit(&command.describe());
    Ok(())
}

fn apply_port_overrides(cli: &Cli, session: &mut Option<magic::Session>) {
    if let Some(s) = session.as_mut() {
        if let Some(fp) = cli.filmcraft_port {
            let _ = s.execute("prefs.set", json!({"key": "craft.filmcraft_port", "value": fp}));
        }
        if let Some(ep) = cli.effectcraft_port {
            let _ = s.execute("prefs.set", json!({"key": "craft.effectcraft_port", "value": ep}));
        }
        if let Some(ref dirs) = cli.font_dirs {
            let _ = s.execute("prefs.set", json!({"key": "lyrics.font_dirs", "value": dirs}));
        }
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

    let roots = magic::AutomationRoots::from_args(&cli.automation_read_roots, &cli.automation_write_roots)?;
    let live = cli.connect.is_some();
    let mut session = (!live).then(|| magic::Session::user_session().with_automation_roots(roots));
    apply_port_overrides(&cli, &mut session);
    if let (Some(path), Some(session)) = (cli.project.as_deref(), session.as_mut()) {
        session.execute("file.open", json!({ "path": path })).map_err(|e| e.to_string())?;
    }
    let mut target = Target {
        session,
        connect: cli.connect,
        project: cli.project,
        save: cli.save,
        save_as: cli.save_as,
        dry: cli.dry_run,
    };
    match (cli.command_id, cli.command) {
        (Some(_), Some(_)) => Err("Use --command or a subcommand, not both.".into()),
        (Some(id), None) => {
            let params = parse_params(&[], cli.params.as_deref())?;
            target.run(&id, params)
        }
        (None, Some(command)) => match command {
            Cmd::New(args) => run_new(&mut target, args),
            Cmd::Commands { filter } => list_commands(&mut target, filter),
            Cmd::Controls(args) => subcommands::run_controls(&mut target, args),
            Cmd::Describe { id } => describe(&id),
            Cmd::Inspect { target: what } => inspect(&mut target, what),
            Cmd::Bridge(b) => subcommands::run_bridge(&mut target, b),
            Cmd::Interchange(args) => subcommands::run_interchange(&mut target, args),
            Cmd::Lottie(args) => subcommands::run_lottie(&mut target, args),
            Cmd::ImportAep(args) => subcommands::run_import_aep(&mut target, args),
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
            "Use --connect or --project, not both: --connect uses the project open in the live window."
        );
        std::process::exit(2);
    }
    if let Err(err) = run(cli) {
        let _ = writeln!(std::io::stderr(), "{err}");
        std::process::exit(1);
    }
}
