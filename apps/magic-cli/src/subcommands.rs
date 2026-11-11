//! Ergonomic `FilmCraft` × `EffectCraft` CLI subcommands (`T01`–`T42`) for `magic-cli`.
//!
//! Covers `--control` discovery (`controls`), cross-app Dynamic Link & bridge management (`bridge`),
//! bidirectional timeline interchange (`interchange`), two-way Lottie/dotLottie (`lottie`),
//! clean-room After Effects `.aep` dry-run/commit (`import-aep`), FilmCraft Studio finishing
//! (`finish`), pure-Rust codec export (`export`), V3 ↔ C1 broadcast captions (`captions`),
//! and 15-column review markers (`markers`).

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

#[derive(Subcommand)]
pub enum BridgeCmd {
    /// Probe live `FilmCraft` (`:9876` + `FILMCRAFT_CONTROL_TOKEN`) & `EffectCraft` (`:9877` + `mcp --autosave`)
    Status {
        #[arg(long)]
        ensure_tokens: bool,
    },
    /// Run offline-media (`media.status`, `footage.check`) & render-source overwrite (`file.checkOutput`) preflight (`T08`)
    Preflight {
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        relink: bool,
    },
    /// Dynamic-Link an `.ecproj` composition onto FilmCraft (`V2`/`V3`/`V4`, `prores`/`webm`/`exr`, exact rational FPS) (`T01`, `T10`)
    Link {
        project: String,
        #[arg(long)]
        comp: Option<String>,
        #[arg(long, default_value = "V2")]
        track: String,
        #[arg(long, default_value = "prores")]
        format: String,
        #[arg(long, default_value = "24")]
        fps: String,
        #[arg(long, default_value_t = 0.0)]
        from: f64,
    },
    /// List compositions and `used_in` references inside an `.ecproj` (`T01`)
    Comps { project: String },
    /// Check or re-render dirty linked `.ecproj` compositions via trim-preserving `media.replaceFootage` (`T02`)
    Sync {
        #[arg(long)]
        check_only: bool,
    },
    /// Bake a linked `.ecproj` composition to standalone `.mov` media (or `--unbake` back to live `.ecproj`) (`T02`)
    Bake {
        link_id: String,
        #[arg(long)]
        unbake: bool,
    },
    /// Inspect or jump non-linear branching undo states (`edit.history.list` / `edit.history.goto`) (`T09`)
    History {
        #[arg(long, default_value = "effectcraft")]
        app: String,
        #[arg(long)]
        goto: Option<u64>,
    },
    /// Replay 1s crash-recovery journal (`file.recover`) on `filmcraft` or `effectcraft` (`T09`)
    Recover {
        #[arg(long, default_value = "filmcraft")]
        app: String,
    },
    /// Clean-room `.ectemplate` & `.fcgt` template operations (`list|apply|export|install|set|render`) (`T06`)
    Templates {
        action: String,
        #[arg(long, default_value = "filmcraft")]
        domain: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Launch or focus a sibling ArtCraft window with `--control` enabled
    Open {
        app: String,
        path: Option<String>,
        #[arg(long)]
        port: Option<u16>,
    },
    /// Close non-dirty sibling ArtCraft windows opened by Magic
    Close,
    /// Send a `--control` method (`ui.*`) or engine command directly to `FilmCraft` (`:9876`)
    Fc {
        command_or_method: String,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        params: Option<String>,
        arguments: Vec<String>,
    },
    /// Send a `--control` method (`ui.*`) or engine command directly to `EffectCraft` (`:9877`)
    Ec {
        command_or_method: String,
        #[arg(long)]
        port: Option<u16>,
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
pub struct LottieArgs {
    /// `export` or `import`
    pub mode: String,
    /// Input or output `.lottie` / `.json` path
    pub path: Option<String>,
    #[arg(long)]
    pub comp: Option<String>,
    #[arg(long)]
    pub ecproj: Option<String>,
    /// Export plain `.json` instead of `.lottie`
    #[arg(long)]
    pub plain_json: bool,
    /// Omit JS expressions when exporting Lottie
    #[arg(long)]
    pub no_expressions: bool,
}

#[derive(Args)]
pub struct ImportAepArgs {
    /// Source `.aep` or `.aepx` path
    pub path: String,
    /// Commit conversion with `allow_losses=true` (defaults to safe `dry_run=true` loss preview)
    #[arg(long)]
    pub commit: bool,
    #[arg(long)]
    pub ecproj: Option<String>,
}

#[derive(Args)]
pub struct FinishArgs {
    /// `video.finish` operation (`workspace`, `mark_in`, `mark_out`, `insert`, `overwrite`, `lift`, `extract`, `trim_mode`, `rate_stretch`, `trim_shuttle`, `sequence_rate`, `color_space`, `interpret_log`, `lumetri_look`, `lumetri_wheel`, `lumetri_curve`, `scopes`, `fx_apply`, `blend_mode`, `audio_gain`, `audio_ducking`, `audio_sync`, `mixer_strip`, `graphic_text`, `jobs_list`, ...)
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

pub const CONTROL_METHODS: &[(&str, &str)] = &[
    ("engine.execute", "Run any registered coordinator command on the live session"),
    ("engine.commands", "List coordinator commands with live enabled status"),
    ("ui.inspect", "Inspect active view, Sheet 08 tabs, focus, playback, dirtyTracks, and :9876/:9877 bridges"),
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
    ("ui.filmcraft", "Launch, inspect, or forward commands/methods to FilmCraft (:9876)"),
    ("ui.effectcraft", "Launch, inspect, or forward commands/methods to EffectCraft (:9877)"),
    ("ui.craft", "Forward commands/methods to any live ArtCraft app"),
    ("perf.stats", "Inspect eframe-wgpu FPS, frame render ms, and integer tick rate"),
    ("window.snap", "Toggle left/right desktop window snap"),
    ("app.hide", "Hide window without ending session"),
    ("app.restart", "Guarded session restart"),
    ("app.quit", "Guarded window close (or force=true)"),
];

pub fn run_controls(target: &mut Target, args: ControlsArgs) -> Result<(), String> {
    let q = args.filter.unwrap_or_default().to_ascii_lowercase();
    let stage_q = args.stage.unwrap_or_default().to_ascii_lowercase();
    let cmds: Vec<Value> = magic::command_specs()
        .iter()
        .filter(|c| {
            (q.is_empty() || c.id.to_ascii_lowercase().contains(&q) || c.label.to_ascii_lowercase().contains(&q))
                && (stage_q.is_empty()
                    || c.id.to_ascii_lowercase().starts_with(&stage_q)
                    || c.menu.iter().any(|m| m.to_ascii_lowercase().contains(&stage_q)))
        })
        .map(magic::commands::CommandSpec::describe)
        .collect();
    let methods: Vec<Value> = CONTROL_METHODS
        .iter()
        .filter(|(m, d)| q.is_empty() || m.to_ascii_lowercase().contains(&q) || d.to_ascii_lowercase().contains(&q))
        .map(|(m, d)| json!({"method": m, "description": d}))
        .collect();
    let live_elements = if args.live && target.connect.is_some() {
        target.call("ui.elements", json!({})).ok()
    } else {
        None
    };
    emit(&json!({
        "ok": true,
        "default_ports": {
            "magic": 9875,
            "filmcraft": 9876,
            "effectcraft": 9877,
            "designcraft": 7979,
            "photocraft": 9879,
            "vectorcraft": 9880,
            "deckcraft": 7990,
        },
        "control_methods": methods,
        "engine_commands_count": cmds.len(),
        "engine_commands": cmds,
        "live_elements": live_elements,
    }));
    Ok(())
}

pub fn run_bridge(target: &mut Target, cmd: BridgeCmd) -> Result<(), String> {
    match cmd {
        BridgeCmd::Status { ensure_tokens } => target.run("bridge.status", json!({"ensure_tokens": ensure_tokens})),
        BridgeCmd::Preflight { output, relink } => target.run(
            "bridge.preflight",
            json!({
                "action": if relink { "relink" } else { "check" },
                "output": output,
            }),
        ),
        BridgeCmd::Link { project, comp, track, format, fps, from } => target.run(
            "video.link_effectcraft",
            json!({
                "project": project,
                "comp": comp,
                "track": track,
                "format": format,
                "fps_preset": fps,
                "from": from,
            }),
        ),
        BridgeCmd::Comps { project } => target.run("video.list_ec_comps", json!({"project": project})),
        BridgeCmd::Sync { check_only } => target.run("video.sync_links", json!({"render": !check_only})),
        BridgeCmd::Bake { link_id, unbake } => target.run("video.bake_link", json!({"link_id": link_id, "unbake": unbake})),
        BridgeCmd::History { app, goto } => match goto {
            Some(index) => target.run("bridge.history", json!({"op": "goto", "app": app, "index": index})),
            None => target.run("bridge.history", json!({"op": "list", "app": app})),
        },
        BridgeCmd::Recover { app } => target.run("bridge.history", json!({"op": "recover", "app": app})),
        BridgeCmd::Templates { action, domain, path, output, params, arguments } => {
            let mut p = parse_params(&arguments, params.as_deref())?;
            p["action"] = json!(action);
            p["domain"] = json!(domain);
            if let Some(v) = path {
                p["path"] = json!(v);
            }
            if let Some(v) = output {
                p["output"] = json!(v);
            }
            target.run("bridge.templates", p)
        }
        BridgeCmd::Open { app, path, port } => target.run(
            "bridge.control",
            json!({"app": app, "action": "launch", "path": path, "port": port}),
        ),
        BridgeCmd::Close => target.run("workspace.close_opened", json!({})),
        BridgeCmd::Fc { command_or_method, port, params, arguments } => {
            forward_sibling(target, "filmcraft", &command_or_method, port, params.as_deref(), &arguments)
        }
        BridgeCmd::Ec { command_or_method, port, params, arguments } => {
            forward_sibling(target, "effectcraft", &command_or_method, port, params.as_deref(), &arguments)
        }
    }
}

fn forward_sibling(
    target: &mut Target,
    app: &str,
    cmd_or_method: &str,
    port: Option<u16>,
    raw_params: Option<&str>,
    arguments: &[String],
) -> Result<(), String> {
    let inner = parse_params(arguments, raw_params)?;
    let is_ui = cmd_or_method.starts_with("ui.") || cmd_or_method.starts_with("app.") || matches!(cmd_or_method, "render.frame" | "perf.stats");
    let payload = if is_ui {
        json!({"app": app, "action": "control", "method": cmd_or_method, "params": inner, "port": port})
    } else {
        json!({"app": app, "action": "execute", "command": cmd_or_method, "params": inner, "port": port})
    };
    target.run("bridge.control", payload)
}

pub fn run_interchange(target: &mut Target, args: InterchangeArgs) -> Result<(), String> {
    let direction = if args.direction == "import" || args.target == "effect_to_film" {
        "effect_to_film"
    } else {
        "film_to_effect"
    };
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

pub fn run_lottie(target: &mut Target, args: LottieArgs) -> Result<(), String> {
    target.run(
        "effect.lottie",
        json!({
            "mode": args.mode,
            "path": args.path,
            "comp": args.comp,
            "ecproj": args.ecproj,
            "dotlottie": !args.plain_json,
            "include_expressions": !args.no_expressions,
        }),
    )
}

pub fn run_import_aep(target: &mut Target, args: ImportAepArgs) -> Result<(), String> {
    target.run(
        "effect.import_aep",
        json!({
            "path": args.path,
            "dry_run": !args.commit,
            "allow_losses": args.commit,
            "ecproj": args.ecproj,
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
