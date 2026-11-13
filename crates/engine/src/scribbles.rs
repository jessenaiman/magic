//! User-editable `extensions/scribbles/` workflows (`fidget.*`, `scribbles.*`) gated strictly on
//! an explicit saved Storyboard slide (`slide` / `storyboard_slide` / `sheet`).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn load_ext_json(s: Option<&Session>, file: &str) -> Result<Value> {
    let ext = crate::extensions::resolve_scribbles_extension(s)?;
    crate::extensions::read_extension_json(&ext, file)
}

fn slide_refusal(s: &Session) -> String {
    let root = s.project_path.as_deref().and_then(Path::parent).map_or_else(|| "<project>".into(), |d| d.display().to_string());
    format!(
        "Random image generation requests are refused: place your image prompt and reference images together on one self-contained Storyboard slide, \
         save/export it (e.g. `storyboard/slides/slide-01.png`), and pass `\"slide\": \"<saved_slide_path>\"` \
         (check here for the saved storyboard slide: {root}/storyboard/slides/<slide>.png)."
    )
}

pub fn require_saved_storyboard_slide(s: &Session, p: &Value) -> Result<PathBuf> {
    let raw = p.get("slide").or_else(|| p.get("storyboard_slide")).or_else(|| p.get("sheet")).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty());
    let resolved = crate::workspace::resolve_workspace_path(s, raw.ok_or_else(|| bad(slide_refusal(s)))?);
    if !resolved.is_file() {
        return Err(bad(slide_refusal(s)));
    }
    let canon = resolved.canonicalize()?;
    let base = s.project_path.as_deref().and_then(Path::parent).and_then(|d| d.canonicalize().ok()).or_else(|| s.cwd.canonicalize().ok());
    if base.as_ref().is_some_and(|b| !canon.starts_with(b) && !canon.starts_with(crate::workspace::workspace_root(s))) {
        return Err(bad(slide_refusal(s)));
    }
    if let Some(roots) = &s.automation_roots {
        roots.check_read(&canon)?;
    }
    Ok(canon)
}

fn prepare_req(s: &Session, p: &Value, raw_kind: &str) -> Result<(Value, Value)> {
    let slide = require_saved_storyboard_slide(s, p)?;
    let kind_id = if raw_kind == "moppet" { "cutout_rig" } else { raw_kind };
    let all = load_ext_json(Some(s), "templates")?;
    let spec = all["kinds"].get(kind_id).ok_or_else(|| bad(format!("unknown Scribbles template `{kind_id}`")))?;
    let [w, h] = [spec["canvas"][0].as_u64().unwrap_or(1024), spec["canvas"][1].as_u64().unwrap_or(1536)];
    let body = p["prompt"].as_str().map(str::trim).unwrap_or_default();
    let layout = spec["layout"]
        .as_str()
        .unwrap_or_default()
        .replace("{action}", p["action"].as_str().unwrap_or("walk"))
        .replace("{direction}", p["direction"].as_str().unwrap_or("side"))
        .replace("{prompt}", body);
    let shared = all["shared"].as_str().unwrap_or_default();
    let prompt = format!("check here for the saved storyboard slide: {}\nCanvas: {w}x{h} px.\n{layout}\n{body}\n{shared}", slide.display());
    let name = p["name"].as_str().map(str::to_owned).unwrap_or_else(|| format!("scribbles-{kind_id}"));
    let req = json!({
        "kind": kind_id, "slide": slide, "references": [slide], "prompt": prompt.trim(),
        "name": name, "character": p["character"], "action": p["action"], "direction": p["direction"],
    });
    let meta = json!({
        "role": "fidget", "kind": kind_id, "label": spec["label"], "canvas": [w, h],
        "slide": slide, "save_command": spec["save_command"], "prompt": req["prompt"],
    });
    Ok((req, meta))
}

pub fn cmd_fidget_prepare(s: &mut Session, p: &Value) -> Result<Value> {
    let (mut req, meta) = prepare_req(s, p, p["kind"].as_str().or_else(|| p["template"].as_str()).unwrap_or("custom"))?;
    req["ok"] = json!(true);
    req["stage"] = json!("template_preview");
    req["fidget"] = meta;
    Ok(req)
}

pub fn cmd_fidget_stage(s: &mut Session, p: &Value) -> Result<Value> {
    let prefs = s.settings.prefs();
    let kind = p["kind"].as_str().or_else(|| p["template"].as_str()).or(prefs.scribbles.template.as_deref()).unwrap_or("custom");
    let (mut req, mut meta) = prepare_req(s, p, kind)?;
    meta["structured"] = crate::craft_ai::rephrase_prompt_with_fidget(kind, req["prompt"].as_str().unwrap_or_default())?;
    req["fidget"] = meta.clone();
    if p["seamless"].as_bool().unwrap_or(prefs.fidget.seamless) {
        return Ok(json!({"ok": true, "stage": "completed_seamless", "seamless": true, "fidget": meta, "generated": cmd_scribbles_generate(s, &req)?}));
    }
    req["ok"] = json!(true);
    req["stage"] = json!("staged");
    Ok(req)
}

pub fn cmd_scribbles_generate(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = p["kind"].as_str().or_else(|| p["template"].as_str()).or_else(|| p["fidget"]["kind"].as_str()).unwrap_or("custom");
    if kind == "moppet" {
        return cmd_scribbles_moppet(s, p);
    }
    let (req, meta) = prepare_req(s, p, kind)?;
    let mut out = crate::craft_ai::cmd_image_generate(s, &req)?;
    out["scribbles"] = meta;
    Ok(out)
}

fn draw_kind(s: &mut Session, p: &Value, kind: &str) -> Result<Value> {
    let mut m = p.clone();
    m["kind"] = json!(kind);
    cmd_scribbles_generate(s, &m)
}

macro_rules! kind_cmd {
    ($f:ident, $k:literal) => {
        pub fn $f(s: &mut Session, p: &Value) -> Result<Value> {
            draw_kind(s, p, $k)
        }
    };
}
kind_cmd!(cmd_scribbles_new, "new");
kind_cmd!(cmd_scribbles_image, "image");
kind_cmd!(cmd_scribbles_sheet, "sheet");
kind_cmd!(cmd_scribbles_poses, "poses");
kind_cmd!(cmd_scribbles_poses_2x2, "poses_2x2");
kind_cmd!(cmd_scribbles_parts_chroma, "parts_chroma");
kind_cmd!(cmd_scribbles_actions, "actions");
kind_cmd!(cmd_scribbles_layers, "layers");
kind_cmd!(cmd_scribbles_custom, "custom");

pub fn cmd_scribbles_moppet(s: &mut Session, p: &Value) -> Result<Value> {
    let slide = require_saved_storyboard_slide(s, p)?;
    let root = crate::workshop::project_dir(s)?;
    let cid = crate::workspace::sanitize_asset_id(p["character"].as_str().or_else(|| p["name"].as_str()).unwrap_or("character"));
    let steps_doc = load_ext_json(Some(s), "steps")?;
    let wf_id = p["workflow"].as_str().or_else(|| steps_doc["default_workflow"].as_str()).unwrap_or("moppet");
    let wf = steps_doc["workflows"].get(wf_id).ok_or_else(|| bad(format!("unknown workflow `{wf_id}`")))?;
    let staging = root.join("assets/characters").join(&cid).join("scribbles_bundle");
    std::fs::create_dir_all(&staging)?;
    for st in wf["steps"].as_array().into_iter().flatten() {
        let kind = st["kind"].as_str().unwrap_or("new");
        let res = draw_kind(s, &json!({"slide": slide, "prompt": p["prompt"], "character": cid, "name": format!("{cid}-{kind}")}), kind)?;
        if let (Some(src), Some(out_name)) = (res["image"].as_str(), st["output"].as_str()) {
            let _ = std::fs::copy(src, staging.join(out_name));
        }
    }
    let ingested = if wf["ingest"].as_bool().unwrap_or(false) { Some(cmd_scribbles_ingest(s, &json!({"dir": staging, "character": cid}))?) } else { None };
    Ok(json!({"ok": true, "workflow": wf_id, "character": cid, "slide": slide, "ingested": ingested}))
}

pub fn cmd_scribbles_ingest(s: &mut Session, p: &Value) -> Result<Value> {
    let root = crate::workshop::project_dir(s)?;
    let dir_raw = p["dir"].as_str().filter(|d| !d.trim().is_empty()).ok_or_else(|| bad("scribbles.ingest requires `dir`"))?;
    let src_dir = crate::workspace::resolve_workspace_path(s, dir_raw);
    let cid = crate::workspace::sanitize_asset_id(p["character"].as_str().unwrap_or("character"));
    let dest = root.join("assets/characters").join(&cid);
    std::fs::create_dir_all(&dest)?;
    for name in ["character.png", "parts-sheet.png", "pose-sheet.png", "handoff.json"] {
        if src_dir.join(name).is_file() {
            crate::doc_state::atomic_copy_file(&src_dir.join(name), &dest.join(name), true)?;
        }
    }
    Ok(json!({"ok": true, "character_id": cid, "source_dir": src_dir, "dest_dir": dest}))
}

pub fn cmd_scribbles_templates(s: &mut Session, _p: &Value) -> Result<Value> {
    let ext = crate::extensions::resolve_scribbles_extension(Some(s))?;
    let tpl = crate::extensions::read_extension_json(&ext, "templates")?;
    let steps = crate::extensions::read_extension_json(&ext, "steps").unwrap_or_else(|_| json!({}));
    Ok(json!({
        "ok": true,
        "extension_id": ext.id,
        "extension_trust": ext.trust.as_str(),
        "extension_dir": ext.dir,
        "note": tpl["note"],
        "kinds": tpl["kinds"],
        "workflows": steps["workflows"],
        "craft_ai": crate::craft_ai::cmd_ai_status(s, &json!({}))?,
    }))
}

pub fn cmd_scribbles_svg(s: &mut Session, p: &Value) -> Result<Value> {
    let sheet = require_saved_storyboard_slide(s, p)?;
    let cid = crate::workspace::sanitize_asset_id(p["character"].as_str().unwrap_or("character"));
    let action = p["action"].as_str().unwrap_or("wave");
    let svg = crate::workshop::project_dir(s)?.join("assets/characters").join(&cid).join("sprites").join(action).join(format!("{cid}-{action}.svg"));
    crate::native_media::trace_silhouette_svg(&sheet, &svg, 1.5, false)?;
    let attached = s.execute("project.add_asset", json!({"path": svg, "kind": "image", "character": cid, "load": false}))?;
    Ok(json!({"ok": true, "svg": svg, "sheet": sheet, "character": cid, "asset_id": attached["asset_id"]}))
}

pub fn cmd_scribbles_reassemble(s: &mut Session, p: &Value) -> Result<Value> {
    let cid = crate::workspace::sanitize_asset_id(p["character"].as_str().unwrap_or("character"));
    let view = p["view"].as_str().unwrap_or("front");
    let cdir = crate::workshop::project_dir(s)?.join("assets/characters").join(&cid);
    let sheet = p["sheet"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(|r| crate::workspace::resolve_workspace_path(s, r))
        .unwrap_or_else(|| cdir.join("parts-sheet.png"));
    crate::native_media::split_parts_sheet(&sheet, &cdir.join(view), &cid, view, p["chroma"].as_str().or(Some("#00FF00")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{cmd_fidget_prepare, cmd_scribbles_generate};
    use crate::Session;

    #[test]
    fn random_prompt_without_saved_storyboard_slide_is_refused() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let sandbox = std::env::temp_dir().join(format!("scribbles_slide_gate_{nanos}"));
        std::fs::create_dir_all(sandbox.join("storyboard/slides")).unwrap();
        let mut s = Session { cwd: sandbox.clone(), project_path: Some(sandbox.join("demo.magic")), ..Session::default() };
        let err = cmd_scribbles_generate(&mut s, &json!({"prompt": "random prompt"})).unwrap_err().to_string();
        assert!(err.contains("check here for the saved storyboard slide"), "got: {err}");
        crate::doc_state::write_file(sandbox.join("storyboard/slides/slide-01.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        let ok = cmd_fidget_prepare(&mut s, &json!({"kind": "new", "prompt": "fox", "slide": "storyboard/slides/slide-01.png"})).unwrap();
        assert!(ok["prompt"].as_str().unwrap().contains("check here for the saved storyboard slide"));
        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
