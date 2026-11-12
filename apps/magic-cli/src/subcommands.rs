//! Ergonomic `FilmCraft` × `EffectCraft` CLI subcommands (`T01`–`T42`) for `magic-cli`.
//!
//! Covers `--control` discovery (`controls`), cross-app Dynamic Link & bridge management (`bridge`),
//! bidirectional timeline interchange (`interchange`), two-way Lottie/dotLottie (`lottie`),
//! clean-room After Effects `.aep` dry-run/commit (`import-aep`), FilmCraft Studio finishing
//! (`finish`), pure-Rust codec export (`export`), V3 ↔ C1 broadcast captions (`captions`),
//! and 15-column review markers (`markers`).

use clap::Args;
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
pub struct ConvertArgs {
    /// Converter mode: `hyperframes`, `remotion`, `custom`, or `auto`
    #[arg(default_value = "auto")]
    pub kind: String,
    /// Path to legacy project folder to archive & convert
    pub source: String,
    /// Target `magic/` workspace root (`magic/projects`, `magic/assets`, `magic/animations`)
    #[arg(long)]
    pub magic_root: Option<String>,
    /// Explicit `.magic` project title
    #[arg(long)]
    pub name: Option<String>,
    /// Overwrite existing converted project if present
    #[arg(long)]
    pub overwrite: bool,
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
    ("document.inspect", "Inspect open .magic project snapshot"),
    ("ui.inspect", "Inspect active view, panels, focus, and playback state"),
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

pub fn run_controls(target: &mut Target, args: ControlsArgs) -> Result<(), String> {
    let q = args.filter.unwrap_or_default().to_ascii_lowercase();
    let stage_q = args.stage.unwrap_or_default().to_ascii_lowercase();
    let cmds: Vec<Value> = magic::command_specs()
        .iter()
        .filter(|c| {
            (q.is_empty() || c.id.to_ascii_lowercase().contains(&q) || c.label.to_ascii_lowercase().contains(&q))
                && (stage_q.is_empty() || c.id.to_ascii_lowercase().starts_with(&stage_q) || c.menu.iter().any(|m| m.to_ascii_lowercase().contains(&stage_q)))
        })
        .map(magic::commands::CommandSpec::describe)
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

pub fn run_convert(target: &mut Target, args: ConvertArgs) -> Result<(), String> {
    let res = target.call_unless_dry(
        "project.convert",
        json!({
            "kind": args.kind,
            "source": args.source,
            "magic_root": args.magic_root,
            "name": args.name,
            "overwrite": args.overwrite,
            "emit_terminal": true,
        }),
    )?;
    emit(&res);
    target.save_project()
}
