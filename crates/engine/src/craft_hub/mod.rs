//! `CraftHub`: Single-Responsibility coordinator between Magic Studio and system-installed
//! ArtCraft applications (`filmcraft`, `effectcraft`, `designcraft`, `photocraft`, `vectorcraft`,
//! `deckcraft`, `anicraft-cli`, `texturecraft`).
//!
//! Safety note: `AniCraft` (<https://github.com/storytold/AniCraft>) is early alpha; `CraftHub`
//! invokes `anicraft-cli` strictly headlessly (rendering PNG frame sequences for `EffectCraft`),
//! while all live Studio UI previews and timelines use `EffectCraft` and `FilmCraft`.

pub mod cli;
pub mod control;
pub mod discovery;
pub mod launch;
pub mod media_ops;
pub mod target;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

pub use discovery::same_canonical_path;
pub use target::CraftTarget;

use crate::{EngineError, Result};
use control::LiveOutcome;

pub struct CraftHub;

impl CraftHub {
    fn prefer_live_bridge() -> bool {
        let stored = crate::project_settings::stored_settings();
        stored.pointer("/craft/prefer_live_bridge").or_else(|| stored.get("craft.prefer_live_bridge")).and_then(Value::as_bool).unwrap_or(true)
    }

    pub fn execute(target: CraftTarget, doc: Option<&Path>, command: &str, params: &Value) -> Result<Value> {
        if Self::prefer_live_bridge() {
            match control::route_single(target, doc, command, params, false) {
                LiveOutcome::Connected(res) => return res,
                LiveOutcome::Unreachable | LiveOutcome::DifferentDocumentOpen => {}
            }
        }
        cli::exec_single(target, doc, command, params, false)
    }

    pub fn execute_and_save(target: CraftTarget, doc: &Path, command: &str, params: &Value) -> Result<Value> {
        if Self::prefer_live_bridge() {
            match control::route_single(target, Some(doc), command, params, true) {
                LiveOutcome::Connected(res) => return res,
                LiveOutcome::Unreachable | LiveOutcome::DifferentDocumentOpen => {}
            }
        }
        cli::exec_single(target, Some(doc), command, params, true)
    }

    pub fn try_live(target: CraftTarget, doc: &Path, command: &str, params: &Value, save: bool) -> Option<Result<Value>> {
        if !Self::prefer_live_bridge() {
            return None;
        }
        match control::route_single(target, Some(doc), command, params, save) {
            LiveOutcome::Connected(res) => Some(res),
            LiveOutcome::Unreachable | LiveOutcome::DifferentDocumentOpen => None,
        }
    }

    pub fn run_batch(target: CraftTarget, input: Option<&Path>, output: Option<&Path>, steps: &[(&str, Value)]) -> Result<Vec<Value>> {
        let in_place = match (input, output) {
            (Some(src), Some(dst)) => same_canonical_path(src, dst),
            (Some(_), None) => true,
            _ => false,
        };
        if Self::prefer_live_bridge()
            && in_place
            && let Some(doc) = input.filter(|p| p.is_file())
            && let Some(live_res) = control::try_live_batch(target, doc, steps, output.is_some())
        {
            return live_res;
        }
        cli::exec_batch(target, input, output, steps)
    }

    pub fn run_effect_steps(project: Option<&Path>, save: bool, steps: &[(String, Value)]) -> Result<Vec<Value>> {
        let borrowed: Vec<(&str, Value)> = steps.iter().map(|(id, params)| (id.as_str(), params.clone())).collect();
        let output = if save { project } else { None };
        Self::run_batch(CraftTarget::Effect, project, output, &borrowed)
    }

    pub fn run_film_steps(project: Option<&Path>, fresh: bool, save: bool, steps: &[Value]) -> Result<Vec<Value>> {
        let mut pairs: Vec<(&str, Value)> = Vec::with_capacity(steps.len());
        for step in steps {
            let id =
                step["id"].as_str().or_else(|| step["command"].as_str()).ok_or_else(|| EngineError::InvalidParams(format!("missing command id in {step}")))?;
            let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
            pairs.push((id, params));
        }
        let input = if fresh { None } else { project.filter(|p| p.is_file()) };
        let output = if save { project } else { None };
        Self::run_batch(CraftTarget::Film, input, output, &pairs)
    }

    pub(crate) fn render_effect_frame(project: &Path, comp: Option<&str>, seconds: f64, max_side: u32, output: &Path) -> Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let scale = (max_side as f64 / 1920.0).clamp(0.1, 1.0);
        let mut live_steps = Vec::new();
        if let Some(c) = comp.filter(|s| !s.is_empty()) {
            live_steps.push(("comp.open", json!({"comp": c})));
        }
        live_steps.push(("comp.saveFrameAs", json!({"path": output, "time": seconds.max(0.0), "scale": scale, "transparent": true})));
        if Self::prefer_live_bridge()
            && let Some(live_res) = control::try_live_batch(CraftTarget::Effect, project, &live_steps, false)
        {
            live_res?;
            return Ok(());
        }
        cli::render_effect_frame(project, comp, seconds, max_side, output)
    }

    pub(crate) fn render_effect_clip(project: &Path, params: &Value) -> Result<Value> {
        let output_str = params["output"].as_str().ok_or_else(|| EngineError::InvalidParams("output path is required".into()))?;
        if let Some(parent) = Path::new(output_str).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let live_steps = [("renderQueue.add", params.clone()), ("renderQueue.render", json!({"wait": true}))];
        if Self::prefer_live_bridge()
            && let Some(live_res) = control::try_live_batch(CraftTarget::Effect, project, &live_steps, false)
        {
            let list = live_res?;
            return Ok(list.into_iter().last().unwrap_or_else(|| json!({"ok": true, "output": output_str})));
        }
        cli::render_effect_clip(project, params)
    }

    pub fn render_effect_template(template: &Path, values: Option<&Value>, output: &Path, frame_time: Option<f64>) -> Result<Value> {
        cli::render_effect_template(template, values, output, frame_time)
    }

    pub(crate) fn render_film_frame(project: &Path, tick: crate::time::Tick, scale: f32, output: &Path) -> Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let params = json!({ "path": output, "format": "png", "target": "program", "time": tick.0, "scale": scale });
        if let Some(res) = Self::try_live(CraftTarget::Film, project, "file.exportFrame", &params, false) {
            res?;
            return Ok(());
        }
        cli::render_film_frame(project, tick, scale, output)
    }

    pub(crate) fn anicraft_render_frame(doc: &Path, frame: u32, output: &Path) -> Result<()> {
        cli::render_anicraft_frame(doc, frame, output)
    }

    pub(crate) fn anicraft_render_seq(doc: &Path, start: u32, end: Option<u32>, out_dir: &Path) -> Result<Vec<PathBuf>> {
        cli::render_anicraft_sequence(doc, start, end, out_dir)
    }

    pub(crate) fn probe_film_media(filename: &Path) -> Result<Value> {
        cli::probe_media(filename)
    }

    pub fn open_live(target: CraftTarget, file: &Path) -> Result<Value> {
        launch::open_or_focus(target, Some(file), None, None)
    }

    pub fn launch_with_control(target: CraftTarget, file: Option<&Path>, port: Option<u16>, workspace_dir: Option<&Path>) -> Result<Value> {
        launch::open_or_focus(target, file, port, workspace_dir)
    }

    pub fn close_opened_apps() -> Result<Value> {
        launch::close_opened_apps()
    }

    pub fn control_addr(target: CraftTarget) -> Result<SocketAddr> {
        discovery::control_addr(target)
    }

    pub fn live_execute(target: CraftTarget, port: Option<u16>, command: &str, params: &Value) -> Result<Value> {
        control::live_execute(target, port, command, params)
    }

    pub fn live_control(target: CraftTarget, port: Option<u16>, method: &str, params: &Value) -> Result<Value> {
        control::live_control(target, port, method, params)
    }
}

#[allow(dead_code)]
pub(crate) fn run_effect_steps(project: Option<&Path>, save: bool, steps: &[(String, Value)]) -> Result<Vec<Value>> {
    CraftHub::run_effect_steps(project, save, steps)
}
