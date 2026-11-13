//! Ergonomic `FilmCraft` × `EffectCraft`, `Studio`, and `Storyboard` CLI subcommands for `magic-cli`.
//!
//! Covers `--control` discovery (`controls`), Studio batch track mapping (`studio map-tracks`),
//! thin Storyboard operations (`storyboard status | open | new | add-section | add-element | export`),
//! bidirectional timeline interchange (`interchange`), FilmCraft Studio finishing (`finish`),
//! pure-Rust codec export (`export`), V3 ↔ C1 broadcast captions (`captions`), and review markers (`markers`).

use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::{Target, emit, parse_params};

#[derive(Args)]
pub struct ControlsArgs {
    /// Filter commands or `--control` methods by substring
    pub filter: Option<String>,
    /// Filter by workstation stage (`project`, `storyboard`, `animation`, `lyrics`, `motion`, `studio`, `ui`)
    #[arg(long)]
    pub stage: Option<String>,
    /// Query live visible `ui.elements` when `--connect` is active
    #[arg(long)]
    pub live: bool,
}

#[derive(Args)]
pub struct EffectArgs {
    #[command(subcommand)]
    pub command: EffectCmd,
}

#[derive(Subcommand)]
pub enum EffectCmd {
    /// Pass any EffectCraft engine command straight to live `:9877` `--control` or `effectcraft-cli exec <id> --params '<json>' ... --json`
    Exec {
        /// EffectCraft command ID (e.g. `comp.info`, `layer.newText`, `prop.addKey`, `file.exportTimeline`)
        id: String,
        /// Force `--save` even on read-only commands
        #[arg(long, conflicts_with = "no_save")]
        save: bool,
        /// Omit `--save` on `effectcraft-cli exec`
        #[arg(long)]
        no_save: bool,
        /// Run against a fresh `--empty` EffectCraft project
        #[arg(long)]
        empty: bool,
        /// Optional explicit `.ecproj` path override (defaults to `effects/lyrics.ecproj` or `--domain motion`)
        #[arg(long)]
        ecproj: Option<String>,
        /// Default domain (`lyrics` or `motion`) when `--ecproj` is omitted
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
}

#[derive(Args)]
pub struct FilmArgs {
    #[command(subcommand)]
    pub command: FilmCmd,
}

#[derive(Subcommand)]
pub enum FilmCmd {
    /// Pass any FilmCraft engine command straight to live `:9876` `--control` or `filmcraft-cli --project <song.fcproj> [--save] exec`
    Exec {
        /// FilmCraft command ID (e.g. `sequence.inspect`, `timeline.razor`, `effects.setParam`)
        id: String,
        /// Force `--save` even on read-only commands
        #[arg(long, conflicts_with = "no_save")]
        save: bool,
        /// Omit `--save` on `filmcraft-cli exec`
        #[arg(long)]
        no_save: bool,
        /// Optional explicit `.fcproj` path override
        #[arg(long)]
        fcproj: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
}

#[derive(Args)]
pub struct AnimationArgs {
    #[command(subcommand)]
    pub command: AnimationCmd,
}

#[derive(Subcommand)]
pub enum AnimationCmd {
    /// One-step render/reuse ProRes 4444 RGBA character clip and hot-swap onto FilmCraft V2 (`animation.promote`)
    Promote {
        /// Character ID (e.g. `fox`)
        character: String,
        /// Action / sprite / GLB clip name
        #[arg(long, default_value = "idle")]
        action: String,
        /// Graphic Symbol loop mode (`loop`, `play_once`, or `single_frame`)
        #[arg(long, default_value = "loop")]
        loop_mode: String,
        /// Starting frame index inside symbol clip
        #[arg(long, default_value_t = 0)]
        first_frame: u32,
        /// Studio placement start timestamp in seconds
        #[arg(long, default_value_t = 0.0)]
        start_sec: f64,
        /// Duration in seconds
        #[arg(long, default_value_t = 3.0)]
        duration: f64,
        /// Optional explicit `.ecproj` path override
        #[arg(long)]
        ecproj: Option<String>,
        /// Force fresh render even if cached `.mov` is newer than `.ecproj`
        #[arg(long)]
        force: bool,
    },
    /// List registered AssetHost & AnimationHost backends and validated extension plugins (`animation.hosts`)
    Hosts,
    /// Resolve a Studio playhead timestamp back to character clip, `.ecproj`, and local frame (`animation.locate_from_studio`)
    Locate {
        /// Studio playhead timestamp in seconds
        seconds: f64,
    },
}

pub fn build_animation_call(cmd: AnimationCmd) -> (&'static str, Value) {
    match cmd {
        AnimationCmd::Promote { character, action, loop_mode, first_frame, start_sec, duration, ecproj, force } => (
            "animation.promote",
            json!({
                "character": character,
                "action": action,
                "loop_mode": loop_mode,
                "first_frame": first_frame,
                "start_sec": start_sec,
                "duration": duration,
                "ecproj": ecproj,
                "force": force,
            }),
        ),
        AnimationCmd::Hosts => ("animation.hosts", json!({})),
        AnimationCmd::Locate { seconds } => ("animation.locate_from_studio", json!({"seconds": seconds})),
    }
}

pub fn run_animation(target: &mut Target, args: AnimationArgs) -> Result<(), String> {
    let (id, payload) = build_animation_call(args.command);
    target.run(id, payload)
}

#[derive(Args)]
pub struct StudioArgs {
    #[command(subcommand)]
    pub command: StudioCmd,
}

#[derive(Subcommand)]
pub enum StudioCmd {
    /// Batch-map FilmCraft tracks into reusable Lyrics (`effects/lyrics.ecproj`) and Motion (`effects/motion.ecproj`) groups
    MapTracks(StudioMapTracksArgs),
}

#[derive(Args)]
pub struct StudioMapTracksArgs {
    /// JSON array (or `{"mappings":[...]}` object) passed to `video.map_tracks`
    #[arg(long)]
    pub mappings: Option<String>,
    /// Repeatable `<lyrics|motion>:<V3,V5>[:<group>]` shorthand (e.g. `--map lyrics:V3,V5 --map motion:V4`)
    #[arg(long = "map", value_name = "TARGET:TRACKS[:GROUP]")]
    pub map_specs: Vec<String>,
    /// Default group name when `--map` omits `:<group>`
    #[arg(long)]
    pub group: Option<String>,
}

#[derive(Args)]
pub struct StoryboardArgs {
    #[command(subcommand)]
    pub command: StoryboardCmd,
}

#[derive(Subcommand)]
pub enum StoryboardCmd {
    /// Inspect Storyboard DeckCraft status (`storyboard.status`)
    Status {
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Open `storyboard.deckcraft` in host DeckCraft (`storyboard.open`)
    Open {
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Initialize `storyboard.deckcraft` (`storyboard.new`)
    New {
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Add a section to `storyboard.deckcraft` (`storyboard.add_section`)
    AddSection {
        name: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Add an element to a section in `storyboard.deckcraft` (`storyboard.add_element`)
    AddElement {
        section: String,
        name: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Export `storyboard.deckcraft` (`storyboard.export`)
    Export {
        out: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
}

#[derive(Args)]
pub struct InterchangeArgs {
    /// `export` or `import`
    pub direction: String,
    /// `otio`, `fcpxml`, `xml`, `edl`, `aaf`, `omf`, or `ale`
    #[arg(long, default_value = "otio")]
    pub format: String,
    /// `film_to_effect` or `effect_to_film`
    #[arg(long, default_value = "film_to_effect")]
    pub target: String,
    /// File path to write or read
    #[arg(long)]
    pub path: Option<String>,
    /// Pre-render incompatible graphic clips during export
    #[arg(long, default_value_t = true)]
    pub prerender: bool,
}

#[derive(Args)]
pub struct FinishArgs {
    /// `video.finish` op (`workspace`, `mark_in`, `mark_out`, `insert`, `overwrite`, `lift`, `extract`, `trim_mode`, `lumetri_*`, `audio_*`, `jobs_list`)
    pub op: String,
    #[arg(long)]
    pub params: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Optional destination path (`exports/<song>.mp4` by default)
    pub output: Option<String>,
    /// `h264`, `hevc`, `prores`, `dnxhr`, `apv`, `av1`, `vp9`, `mxf`, `exr`, `dpx`, `png`, `tiff`, `tga`, `gif`, `wav`, `aiff`, `mp3`, `aac`, `flac`, or `opus`
    #[arg(long, default_value = "h264")]
    pub codec: String,
    #[arg(long)]
    pub container: Option<String>,
    #[arg(long)]
    pub preset: Option<String>,
    /// `crf`, `vbr`, or `cbr`
    #[arg(long, default_value = "crf")]
    pub bitrate_mode: String,
    #[arg(long, default_value_t = 18)]
    pub crf: u32,
    /// `sdr`, `pq`, or `hlg`
    #[arg(long, default_value = "sdr")]
    pub hdr: String,
    #[arg(long)]
    pub burn_captions: bool,
    #[arg(long)]
    pub allow_offline: bool,
    #[arg(long)]
    pub queue: bool,
    #[arg(long, default_value_t = true)]
    pub gpu: bool,
    #[arg(long)]
    pub hw_encode: bool,
}

#[derive(Args)]
pub struct CaptionsArgs {
    /// `push` (`V3 -> C1`), `pull` (`C1 -> V3`), `upgrade` (`graphics.upgradeCaption`), or `export`
    pub action: String,
    /// `Subtitle`, `CEA-608`, `CEA-708`, or `Teletext`
    #[arg(long, default_value = "Subtitle")]
    pub standard: String,
    /// `srt`, `vtt`, `scc`, `mcc`, `stl`, or `ttml`
    #[arg(long, default_value = "srt")]
    pub format: String,
    #[arg(long, default_value = "en")]
    pub language: String,
}

#[derive(Args)]
pub struct MarkersArgs {
    /// `list`, `add`, `edit`, `export-csv`, `palette`, `seed`, `delete`, or `clear`
    #[arg(default_value = "list")]
    pub action: String,
    #[arg(long)]
    pub params: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Args)]
pub struct TimelineCliArgs {
    /// `inspect`, `detect`, `export`, `import`, `timecode`, `ale`, or `edl`
    #[arg(default_value = "inspect")]
    pub action: String,
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub params: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Args)]
pub struct MeshCliArgs {
    /// `inspect` or `convert`
    #[arg(default_value = "inspect")]
    pub action: String,
    /// Source `.glb`, `.gltf`, `.obj`, `.fbx`, `.stl`, `.ply`, or `.3mf` file
    pub path: String,
    /// Output `.glb`, `.obj`, or `.stl` path (for `convert`)
    #[arg(long)]
    pub output: Option<String>,
}

pub const CONTROL_METHODS: &[(&str, &str)] = &[
    ("engine.execute", "Run any registered coordinator command on the live session"),
    ("engine.commands", "List coordinator commands with live enabled status"),
    ("document.inspect", "Inspect open .magic project snapshot"),
    ("ui.inspect", "Inspect active view, panels, focus, and playback state"),
    ("ui.docking", "Inspect craft-ui panel docking Layout, focused panel, hidden panels, and hitRects"),
    ("ui.dock", "Execute a craft-ui DockCommand operation (open, close, activate, float, move, resizeSplit)"),
    ("ui.elements", "List visible egui widget automation IDs, labels, and logical rects"),
    ("ui.set", "Switch view, studioWorkspace, animTab, theatreTab, lyricsTab, motionTab, theme, style, or widget value"),
    ("ui.viewer.locate", "Map stage coordinates (x, y) to logical window coordinates"),
    ("ui.viewer.hit", "Hit-test logical window coordinates onto active stage coordinates"),
    ("ui.timeline.locate", "Map song seconds/ticks to logical window coordinates on the active timeline"),
    ("ui.timeline.hit", "Hit-test logical window coordinates onto timeline seconds and ticks"),
    ("render.frame", "Render PNG frame for studio, lyrics, motion, animation, or storyboard"),
    ("ui.click", "Inject synthetic pointer click by widget automation ID or {x, y}"),
    ("ui.move", "Inject synthetic pointer move"),
    ("ui.drag", "Interpolate synthetic pointer drag between two endpoints"),
    ("ui.scroll", "Inject synthetic wheel/trackpad scroll"),
    ("ui.key", "Inject key or shortcut chord through craft-keymap"),
    ("ui.keymap.press", "Trigger a craft-keymap binding directly"),
    ("ui.type", "Type text into focused or named egui input"),
    ("ui.focus", "Focus a widget ID or bring window forward"),
    ("ui.screenshot", "Capture full window, named panel, or widget automation ID to PNG"),
    ("ui.resize", "Resize logical viewport bounds (320..=7680 x 240..=4320)"),
    ("ui.playback", "Play, stop, toggle, or inspect song audio preview"),
    ("ui.dialog.state", "Inspect active craft-dialogs file picker state"),
    ("ui.dialog.override", "Pre-seed unattended file picker response"),
    ("ui.menu.list", "Serialize native/in-window menu bar items"),
    ("ui.menu.invoke", "Invoke a menu command directly"),
    ("perf.stats", "Inspect eframe-wgpu FPS, frame render ms, and integer tick rate"),
    ("window.snap", "Toggle left/right desktop window snap"),
    ("app.hide", "Hide window without ending session"),
    ("app.restart", "Guarded session restart"),
    ("app.quit", "Guarded window close (or force=true)"),
];

#[allow(clippy::cognitive_complexity)]
pub fn parse_studio_mappings(args: &StudioMapTracksArgs) -> Result<Value, String> {
    let mut list: Vec<Value> = Vec::new();
    if let Some(raw) = args.mappings.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let parsed: Value = serde_json::from_str(raw).map_err(|e| format!("--mappings: {e}"))?;
        if let Some(arr) = parsed.as_array() {
            list.extend(arr.iter().cloned());
        } else if let Some(arr) = parsed.get("mappings").and_then(Value::as_array) {
            list.extend(arr.iter().cloned());
        } else if parsed.is_object() {
            list.push(parsed);
        } else {
            return Err("--mappings must be a JSON array or `{\"mappings\":[...]}` object".into());
        }
    }
    for spec in &args.map_specs {
        let parts: Vec<&str> = spec.splitn(3, ':').collect();
        if parts.len() < 2 {
            return Err(format!("invalid --map `{spec}` (expected `<lyrics|motion>:<V3,V5>[:<group>]`)"));
        }
        let target = parts[0].trim();
        let tracks: Vec<String> = parts[1].split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
        if target.is_empty() || tracks.is_empty() {
            return Err(format!("invalid --map `{spec}` (provide non-empty target and comma-separated tracks)"));
        }
        let mut entry = json!({
            "target": target,
            "tracks": tracks,
        });
        let grp = parts.get(2).map(|g| g.trim()).filter(|g| !g.is_empty()).map(str::to_owned).or_else(|| args.group.clone());
        if let Some(g) = grp {
            entry["group"] = json!(g);
        }
        list.push(entry);
    }
    if list.is_empty() {
        return Err("provide `--mappings '<json>'` or repeatable `--map <lyrics|motion>:<V3,V5>[:<group>]`".into());
    }
    Ok(json!({ "mappings": list }))
}

pub fn build_effect_call(cmd: EffectCmd) -> Result<(&'static str, Value), String> {
    match cmd {
        EffectCmd::Exec { id, save, no_save, empty, ecproj, domain, params, arguments } => {
            let cmd_params = parse_params(&arguments, params.as_deref())?;
            let mut payload = json!({
                "command": id,
                "params": cmd_params,
            });
            if save {
                payload["save"] = json!(true);
            } else if no_save {
                payload["save"] = json!(false);
            }
            if empty {
                payload["empty"] = json!(true);
            }
            if let Some(proj) = ecproj {
                payload["project"] = json!(proj);
            }
            if let Some(dom) = domain {
                payload["domain"] = json!(dom);
            }
            Ok(("effect.exec", payload))
        }
    }
}

pub fn run_effect(target: &mut Target, args: EffectArgs) -> Result<(), String> {
    let (id, payload) = build_effect_call(args.command)?;
    target.run(id, payload)
}

pub fn build_film_call(cmd: FilmCmd) -> Result<(&'static str, Value), String> {
    match cmd {
        FilmCmd::Exec { id, save, no_save, fcproj, params, arguments } => {
            let cmd_params = parse_params(&arguments, params.as_deref())?;
            let mut payload = json!({
                "command": id,
                "params": cmd_params,
            });
            if save {
                payload["save"] = json!(true);
            } else if no_save {
                payload["save"] = json!(false);
            }
            if let Some(proj) = fcproj {
                payload["project"] = json!(proj);
            }
            Ok(("video.exec", payload))
        }
    }
}

pub fn run_film(target: &mut Target, args: FilmArgs) -> Result<(), String> {
    let (id, payload) = build_film_call(args.command)?;
    target.run(id, payload)
}

pub fn run_studio(target: &mut Target, args: StudioArgs) -> Result<(), String> {
    match args.command {
        StudioCmd::MapTracks(map_args) => {
            let payload = parse_studio_mappings(&map_args)?;
            target.run("video.map_tracks", payload)
        }
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn build_storyboard_call(cmd: StoryboardCmd) -> Result<(&'static str, Value), String> {
    match cmd {
        StoryboardCmd::Status { path, params, arguments } => {
            let mut p = parse_params(&arguments, params.as_deref())?;
            if let Some(v) = path {
                p["path"] = json!(v);
            }
            Ok(("storyboard.status", p))
        }
        StoryboardCmd::Open { path, page, params, arguments } => {
            let mut p = parse_params(&arguments, params.as_deref())?;
            if let Some(v) = path {
                p["path"] = json!(v);
            }
            if let Some(pg) = page {
                p["page"] = json!(pg);
            }
            Ok(("storyboard.open", p))
        }
        StoryboardCmd::New { path, overwrite, params, arguments } => {
            let mut p = parse_params(&arguments, params.as_deref())?;
            if let Some(v) = path {
                p["path"] = json!(v);
            }
            if overwrite {
                p["overwrite"] = json!(true);
            }
            Ok(("storyboard.new", p))
        }
        StoryboardCmd::AddSection { name, path, params, arguments } => {
            let mut p = parse_params(&arguments, params.as_deref())?;
            p["name"] = json!(name.clone());
            if p.get("section").is_none() {
                p["section"] = json!(name);
            }
            if let Some(v) = path {
                p["path"] = json!(v);
            }
            Ok(("storyboard.add_section", p))
        }
        StoryboardCmd::AddElement { section, name, path, params, arguments } => {
            let mut p = parse_params(&arguments, params.as_deref())?;
            p["section"] = json!(section);
            p["name"] = json!(name.clone());
            if p.get("element").is_none() {
                p["element"] = json!(name);
            }
            if let Some(v) = path {
                p["path"] = json!(v);
            }
            Ok(("storyboard.add_element", p))
        }
        StoryboardCmd::Export { out, format, params, arguments } => {
            let mut p = parse_params(&arguments, params.as_deref())?;
            if let Some(dest) = out {
                if p.get("output").is_none() {
                    p["output"] = json!(dest.clone());
                }
                if p.get("path").is_none() {
                    p["path"] = json!(dest);
                }
            }
            if let Some(fmt) = format {
                p["format"] = json!(fmt);
            }
            Ok(("storyboard.export", p))
        }
    }
}

pub fn run_storyboard(target: &mut Target, args: StoryboardArgs) -> Result<(), String> {
    let (id, params) = build_storyboard_call(args.command)?;
    target.run(id, params)
}

pub fn run_controls(target: &mut Target, args: ControlsArgs) -> Result<(), String> {
    let q = args.filter.unwrap_or_default().to_ascii_lowercase();
    let stage_q = args.stage.unwrap_or_default().to_ascii_lowercase();
    let cmds: Vec<Value> = magic_engine::command_specs()
        .iter()
        .filter(|c| {
            (q.is_empty() || c.id.to_ascii_lowercase().contains(&q) || c.label.to_ascii_lowercase().contains(&q))
                && (stage_q.is_empty() || c.id.to_ascii_lowercase().starts_with(&stage_q) || c.menu.iter().any(|m| m.to_ascii_lowercase().contains(&stage_q)))
        })
        .map(magic_engine::commands::CommandSpec::describe)
        .collect();
    let methods: Vec<Value> = CONTROL_METHODS
        .iter()
        .filter(|(m, d)| q.is_empty() || m.to_ascii_lowercase().contains(&q) || d.to_ascii_lowercase().contains(&q))
        .map(|(m, d)| json!({"method": m, "description": d}))
        .collect();
    let live_elements = if args.live && target.connect.is_some() { target.call("ui.elements", json!({})).ok() } else { None };
    emit(&json!({
        "ok": true,
        "default_ports": { "magic": 9875 },
        "control_methods": methods,
        "engine_commands_count": cmds.len(),
        "engine_commands": cmds,
        "live_elements": live_elements,
    }));
    Ok(())
}

pub fn run_interchange(target: &mut Target, args: InterchangeArgs) -> Result<(), String> {
    let direction = if args.direction == "import" || args.target == "effect_to_film" { "effect_to_film" } else { "film_to_effect" };
    target.run(
        "video.markers",
        json!({
            "action": "interchange",
            "direction": direction,
            "format": args.format,
            "path": args.path,
            "pre_render_incompatible": args.prerender,
        }),
    )
}

pub fn run_finish(target: &mut Target, args: FinishArgs) -> Result<(), String> {
    let mut p = parse_params(&args.arguments, args.params.as_deref())?;
    p["op"] = json!(args.op);
    target.run("video.finish", p)
}

pub fn run_export(target: &mut Target, args: ExportArgs) -> Result<(), String> {
    target.run(
        "video.export",
        json!({
            "path": args.output,
            "format": args.codec,
            "container": args.container,
            "preset": args.preset,
            "bitrateMode": args.bitrate_mode,
            "crf": args.crf,
            "hdrTransfer": args.hdr,
            "burnCaptions": args.burn_captions,
            "allowOffline": args.allow_offline,
            "queue": args.queue,
            "gpuRendering": args.gpu,
            "hardwareEncoding": args.hw_encode,
            "fastStartMoov": true,
        }),
    )
}

pub fn run_captions(target: &mut Target, args: CaptionsArgs) -> Result<(), String> {
    let (dir, upgrade, sidecar) = match args.action.as_str() {
        "pull" | "c1_to_v3" => ("c1_to_v3", false, false),
        "upgrade" => ("v3_to_c1", true, false),
        "export" => ("v3_to_c1", false, true),
        _ => ("v3_to_c1", false, false),
    };
    target.run(
        "words.captions_sync",
        json!({
            "direction": dir,
            "track_format": args.standard,
            "format": args.format,
            "language": args.language,
            "upgrade_to_graphic": upgrade,
            "export_sidecar": sidecar,
        }),
    )
}

pub fn run_markers(target: &mut Target, args: MarkersArgs) -> Result<(), String> {
    let mut p = parse_params(&args.arguments, args.params.as_deref())?;
    p["action"] = json!(args.action.replace('-', "_"));
    target.run("video.markers", p)
}

pub fn run_timeline_cli(target: &mut Target, args: TimelineCliArgs) -> Result<(), String> {
    let mut p = parse_params(&args.arguments, args.params.as_deref())?;
    if let Some(fmt) = args.format {
        p["format"] = json!(fmt);
    }
    if let Some(path) = args.path {
        p["path"] = json!(path);
    }
    let cmd = match args.action.as_str() {
        "detect" => "timeline.detect",
        "export" | "write" => "timeline.export",
        "import" | "read" | "parse" => "timeline.import",
        "timecode" | "tc" => "timeline.timecode",
        "ale" => "timeline.ale",
        "edl" => "timeline.edl",
        _ => "timeline.inspect",
    };
    target.run(cmd, p)
}

pub fn run_mesh_cli(target: &mut Target, args: MeshCliArgs) -> Result<(), String> {
    match args.action.as_str() {
        "convert" => target.run("mesh.convert", json!({"path": args.path, "output": args.output})),
        _ => target.run("mesh.inspect", json!({"path": args.path})),
    }
}
