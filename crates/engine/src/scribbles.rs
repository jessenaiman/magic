//! Scribbles engine commands — thin dispatch to [`craft_scribbles`] (L4), `PhotoCraft`, `VectorCraft`, and `EffectCraft`.

use std::path::PathBuf;

use craft_scribbles::{DEFAULT_FLOW_PROJECT_URL, DEFAULT_MOPPET_MAKER_URL, VideoClipRequest};
use serde_json::{Value, json};

use crate::{CraftHub, EngineError, Result, Session};

const TEMPLATES: &str = include_str!("scribbles/templates.json");

pub const FIDGET_NOTE: &str = "NOTE: This template is a working structure, not a final specification — follow it to the best of your ability and report any areas or commands that need improvement after use.";

fn bad(message: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(message.into())
}

pub(crate) fn templates() -> Result<Value> {
    serde_json::from_str(TEMPLATES).map_err(|e| bad(format!("scribbles templates: {e}")))
}

pub fn format_fidget_markdown(template: &str, save_command: &str, tags: &[String], canvas: Option<[u64; 2]>, body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with("---\n") && trimmed.contains(FIDGET_NOTE) {
        return trimmed.to_owned();
    }
    let tags_str = if tags.is_empty() { format!("[magic, fidget, {template}]") } else { format!("[{}]", tags.join(", ")) };
    let canvas_line = canvas.map(|[w, h]| format!("canvas: [{w}, {h}]\n")).unwrap_or_default();
    format!(
        "---\n\
         role: fidget\n\
         template: {template}\n\
         version: 1\n\
         status: working-draft\n\
         tags: {tags_str}\n\
         save_command: {save_command}\n\
         {canvas_line}---\n\
         > {FIDGET_NOTE}\n\n\
         {trimmed}\n\n\
         ## Save Command\n\
         Save and register the output using `magic-cli exec {save_command}`.\n"
    )
}

pub fn parse_frontmatter_template(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let rest = trimmed.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    for line in rest[..end].lines() {
        if let Some((k, v)) = line.split_once(':')
            && k.trim() == "template"
        {
            let value = v.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn layout(kind: &Value, p: &Value) -> Result<String> {
    let mut text = kind["layout"].as_str().unwrap_or_default().to_string();
    if let Some(actions) = kind["actions"].as_object() {
        let name = p["action"].as_str().unwrap_or("walk");
        let action = actions.get(name).ok_or_else(|| bad(format!("action must be one of {}", actions.keys().cloned().collect::<Vec<_>>().join(", "))))?;
        text = text
            .replace("{frames}", "8")
            .replace("{action}", &format!("a {name} cycle"))
            .replace("{view}", action["view"].as_str().unwrap_or_default())
            .replace("{keys}", action["keys"].as_str().unwrap_or_default());
    }
    if text.contains("{prompt}") {
        let prompt = p["prompt"].as_str().filter(|t| !t.trim().is_empty());
        text = text.replace("{prompt}", prompt.ok_or_else(|| bad("prompt is required"))?);
    }
    Ok(text)
}

fn compose(all: &Value, kind: &Value, p: &Value) -> Result<String> {
    let [w, h] = [kind["canvas"][0].as_u64().unwrap_or(1024), kind["canvas"][1].as_u64().unwrap_or(1536)];
    let mut parts = vec![format!("Draw ONE image on a {w} x {h} px canvas."), layout(kind, p)?];
    for rule in kind["rules"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        parts.push(all[rule].as_str().unwrap_or_default().to_string());
    }
    if kind["layout"].as_str() != Some("{prompt}")
        && !kind["layout"].as_str().unwrap_or_default().contains("{prompt}")
        && let Some(prompt) = p["prompt"].as_str().filter(|t| !t.trim().is_empty())
    {
        parts.push(format!("Character: {prompt}"));
    }
    if let Some(notes) = p["notes"].as_str().filter(|t| !t.trim().is_empty()) {
        parts.push(format!("Notes: {notes}"));
    }
    let shared = kind["shared_override"].as_str().or_else(|| all["shared"].as_str()).unwrap_or_default();
    if !shared.is_empty() {
        parts.push(shared.to_string());
    }
    Ok(parts.join("\n"))
}

#[derive(serde::Serialize)]
pub(crate) struct PreparedDrawing {
    pub request: Value,
    pub fidget: Value,
}

impl PreparedDrawing {
    pub fn prepare(p: &Value, name: &str) -> Result<Self> {
        let all = templates()?;
        let kind = all["kinds"].get(name).ok_or_else(|| bad(format!("unknown drawing kind {name}")))?;
        if p.get("reference").is_some() && p.get("references").is_some() {
            return Err(bad("use reference or references, not both"));
        }
        let references = match p.get("references").or_else(|| p.get("reference")) {
            None => Vec::new(),
            Some(Value::String(one)) => vec![json!(one)],
            Some(Value::Array(many)) => many.clone(),
            _ => return Err(bad("references must be an image path or array of image paths")),
        };
        if references.len() > 5 || references.iter().any(|r| r.as_str().is_none_or(|s| s.trim().is_empty())) {
            return Err(bad("references must contain at most five nonempty image paths"));
        }
        if kind["needs_reference"] == true && references.is_empty() {
            return Err(bad(format!("{name} needs reference=<image of the character>")));
        }
        if name == "new" && p["prompt"].as_str().is_none_or(|t| t.trim().is_empty()) {
            return Err(bad("new needs prompt=<the character to draw>"));
        }
        let prompt = compose(&all, kind, p)?;
        let save_command = kind["save_command"].as_str().unwrap_or("project.add_asset").to_owned();
        let save_kind = kind["save_kind"].as_str().unwrap_or("image").to_owned();
        let tags: Vec<String> = kind["tags"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect();
        let canvas = [kind["canvas"][0].as_u64().unwrap_or(1024), kind["canvas"][1].as_u64().unwrap_or(1536)];
        let markdown = format_fidget_markdown(name, &save_command, &tags, Some(canvas), &prompt);
        let mut request = json!({
            "kind": name,
            "prompt": prompt,
            "markdown": markdown,
            "transparent_background": true,
            "timeout_s": p["timeout_s"].as_u64().unwrap_or(900),
            "name": p["name"].as_str().map(str::to_owned).unwrap_or_else(|| format!("scribbles-{name}")),
        });
        if !references.is_empty() {
            request["references"] = json!(references);
        }
        for key in ["provider", "model", "character", "project", "action", "count", "remove_background", "vectorize", "chroma"] {
            if let Some(value) = p.get(key) {
                request[key] = value.clone();
            }
        }
        Ok(Self {
            request,
            fidget: json!({
                "role": "fidget",
                "kind": name,
                "label": kind["label"],
                "reference": kind["reference"],
                "canvas": kind["canvas"],
                "tags": tags,
                "save_command": save_command,
                "save_kind": save_kind,
                "note": FIDGET_NOTE,
                "prompt": prompt,
                "markdown": markdown,
            }),
        })
    }
}

pub fn cmd_fidget_prepare(_: &mut Session, p: &Value) -> Result<Value> {
    let name = p["kind"].as_str().or_else(|| p["template"].as_str()).unwrap_or("custom");
    let prepared = PreparedDrawing::prepare(p, name)?;
    let mut request = prepared.request;
    request["ok"] = json!(true);
    request["fidget"] = prepared.fidget;
    Ok(request)
}

pub fn cmd_scribbles_open(s: &mut Session, p: &Value) -> Result<Value> {
    let mut out = craft_scribbles::open_moppet_maker_window(p["url"].as_str()).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    if p["ingest"].as_bool().unwrap_or(true)
        && s.project_path.is_some()
        && let Ok(ingested) = cmd_scribbles_ingest(s, p)
    {
        out["ingested"] = ingested;
    }
    Ok(out)
}

pub fn cmd_scribbles_ingest(s: &mut Session, p: &Value) -> Result<Value> {
    let project = s.project_path.clone().ok_or_else(|| EngineError::InvalidParams("Open a project before ingesting Moppet Maker assets".into()))?;
    let root = project.parent().ok_or_else(|| EngineError::InvalidParams("Project has no parent directory".into()))?;
    let source_dir = p["dir"].as_str().filter(|d| !d.trim().is_empty()).map(PathBuf::from);
    let requested_char = p["character"].as_str().filter(|c| !c.trim().is_empty());

    let bundle =
        craft_scribbles::ingest_moppet_downloads(root, source_dir.as_deref(), requested_char).map_err(|e| EngineError::InvalidParams(e.to_string()))?;

    let mut registered = Vec::new();
    let char_id = &bundle.character_id;
    for (opt_path, suffix) in [(&bundle.character_png, "character"), (&bundle.parts_sheet_png, "parts-sheet"), (&bundle.pose_sheet_png, "pose-sheet")] {
        if let Some(path) = opt_path
            && let Ok(res) = s.execute(
                "project.add_asset",
                json!({"path": path, "kind": "image", "character": char_id, "name": format!("{char_id}-{suffix}"), "load": false}),
            )
        {
            registered.push(res);
        }
    }
    if let Some(ref pose_png) = bundle.pose_sheet_png {
        let _ = s.execute(
            "character.sprite",
            json!({"character": char_id, "action": &bundle.action, "name": &bundle.action, "sheet": pose_png, "columns": bundle.grid.0, "rows": bundle.grid.1, "fps": 6}),
        );
    }
    if bundle.parts_json.is_some() && bundle.character_png.is_some() {
        let _ = s.execute("character.cut", json!({"character": char_id, "view": &bundle.view, "rig": true}));
    }
    if let Some(ref svg_path) = bundle.animated_svg
        && let Ok(res) = s.execute(
            "project.add_asset",
            json!({"path": svg_path, "kind": "image", "character": char_id, "name": format!("{char_id}-{}-svg", bundle.action), "load": false}),
        )
    {
        registered.push(res);
    }

    Ok(json!({
        "ok": true,
        "character_id": bundle.character_id,
        "character_name": bundle.character_name,
        "view": bundle.view,
        "action": bundle.action,
        "grid": [bundle.grid.0, bundle.grid.1],
        "source_dir": bundle.source_dir,
        "dest_dir": bundle.dest_dir,
        "handoff_json": bundle.handoff_json,
        "parts_json": bundle.parts_json,
        "character_png": bundle.character_png,
        "parts_sheet_png": bundle.parts_sheet_png,
        "pose_sheet_png": bundle.pose_sheet_png,
        "animated_svg": bundle.animated_svg,
        "registered_count": registered.len(),
        "registered": registered,
    }))
}

fn resolve_template_request(p: &Value) -> Result<(Value, Value)> {
    let all = templates()?;
    let kinds = &all["kinds"];
    if let Some(fidget) = p.get("fidget").filter(|v| v.is_object()) {
        let kind_name = fidget["kind"]
            .as_str()
            .filter(|k| kinds.get(*k).is_some())
            .ok_or_else(|| bad("Scribbles requires a valid template in `fidget.kind`; inspect `scribbles.templates`"))?;
        let spec = &kinds[kind_name];
        let mut meta = fidget.clone();
        if meta.get("save_command").is_none() {
            meta["save_command"] = spec["save_command"].clone();
        }
        if meta.get("save_kind").is_none() {
            meta["save_kind"] = spec["save_kind"].clone();
        }
        if meta.get("note").is_none() {
            meta["note"] = json!(FIDGET_NOTE);
        }
        return Ok((p.clone(), meta));
    }
    if let Some(kind_name) = p["kind"].as_str().or_else(|| p["template"].as_str()) {
        let prepared = PreparedDrawing::prepare(p, kind_name)?;
        return Ok((prepared.request, prepared.fidget));
    }
    let frontmatter_kind = p["markdown"].as_str().and_then(parse_frontmatter_template).or_else(|| p["prompt"].as_str().and_then(parse_frontmatter_template));
    if let Some(kind_name) = frontmatter_kind {
        let spec = kinds.get(&kind_name).ok_or_else(|| bad(format!("Unknown Scribbles template `{kind_name}`")))?;
        return Ok((
            p.clone(),
            json!({
                "role": "fidget",
                "kind": kind_name,
                "label": spec["label"],
                "reference": spec["reference"],
                "canvas": spec["canvas"],
                "tags": spec["tags"],
                "save_command": spec["save_command"],
                "save_kind": spec["save_kind"],
                "note": FIDGET_NOTE,
            }),
        ));
    }
    if p["prompt"].as_str().is_some_and(|v| !v.trim().is_empty()) {
        let prepared = PreparedDrawing::prepare(p, "custom")?;
        return Ok((prepared.request, prepared.fidget));
    }
    Err(bad("Scribbles requires a template (`kind`, `template`, or `fidget` from `fidget.prepare`); inspect `scribbles.templates`"))
}

fn resolve_scribbles_provider(request: &Value) -> Result<String> {
    if let Some(p) = request["provider"].as_str().map(str::trim).filter(|v| !v.is_empty()) {
        return Ok(p.to_owned());
    }
    let settings = crate::project_settings::stored_settings();
    if let Some(p) = settings["scribbles"]["provider"].as_str().map(str::trim).filter(|v| !v.is_empty()) {
        return Ok(p.to_owned());
    }
    if let Some(p) = settings["image"]["provider"].as_str().map(str::trim).filter(|v| !v.is_empty()) {
        return Ok(p.to_owned());
    }
    Err(bad("no Scribbles provider is chosen: pass `provider` (codex, agy, or flow), or set it with `magic-cli exec scribbles.connect provider=codex`"))
}

pub fn cmd_scribbles_generate(s: &mut Session, p: &Value) -> Result<Value> {
    if !p.is_object() {
        return Err(bad("Scribbles needs a JSON object with a template and prompt"));
    }
    if p["open"].as_bool() == Some(true)
        || (p["prompt"].as_str().is_none_or(|v| v.trim().is_empty())
            && p.get("fidget").is_none()
            && p.get("kind").is_none()
            && p.get("template").is_none()
            && p["markdown"].as_str().is_none_or(|v| v.trim().is_empty()))
    {
        return cmd_scribbles_open(s, p);
    }
    if p["kind"].as_str().or_else(|| p["template"].as_str()).or_else(|| p["fidget"]["kind"].as_str()) == Some("moppet") {
        return cmd_scribbles_moppet(s, p);
    }
    let (mut request, mut metadata) = resolve_template_request(p)?;
    if request["prompt"].as_str().is_none_or(|v| v.trim().is_empty()) {
        return Err(bad("Scribbles needs a nonempty final prompt"));
    }
    request["provider"] = json!(resolve_scribbles_provider(&request)?);
    let count = request.get("count").and_then(Value::as_u64).unwrap_or(1);
    if !(1..=4).contains(&count) {
        return Err(bad("scribbles count must be between 1 and 4"));
    }
    let mut runs = Vec::new();
    let mut last_result = Value::Null;
    for index in 0..count {
        let mut iter_req = request.clone();
        if count > 1 {
            let base_name = request["name"].as_str().unwrap_or("scribbles-item");
            iter_req["name"] = json!(format!("{base_name}-{}", index + 1));
        }
        let generated = crate::image_generation::generate(s, &iter_req)?;
        if metadata["save_command"] == "character.sprite"
            && let (Some(img), Some(char_id)) = (generated["image"].as_str(), iter_req["character"].as_str().filter(|v| !v.is_empty()))
        {
            let action_name = iter_req["action"].as_str().unwrap_or("walk");
            let _ =
                s.execute("character.sprite", json!({"character": char_id, "name": action_name, "action": action_name, "sheet": img, "columns": 4, "rows": 2}));
        }
        runs.push(generated.clone());
        last_result = generated;
    }
    let canvas = &metadata["canvas"];
    metadata["canvas_matches"] = json!(last_result["width"] == canvas[0] && last_result["height"] == canvas[1]);
    metadata["loop_count"] = json!(count);
    if count > 1 {
        metadata["items"] = json!(runs);
    }
    last_result["scribbles"] = metadata;
    Ok(last_result)
}

fn draw(s: &mut Session, p: &Value, name: &str) -> Result<Value> {
    let prepared = PreparedDrawing::prepare(p, name)?;
    let mut request = prepared.request;
    request["fidget"] = prepared.fidget;
    cmd_scribbles_generate(s, &request)
}

macro_rules! kind_command {
    ($fn:ident, $kind:literal) => {
        pub fn $fn(s: &mut Session, p: &Value) -> Result<Value> {
            draw(s, p, $kind)
        }
    };
}

kind_command!(cmd_scribbles_new, "new");
kind_command!(cmd_scribbles_image, "image");
kind_command!(cmd_scribbles_sheet, "sheet");
kind_command!(cmd_scribbles_poses, "poses");
kind_command!(cmd_scribbles_poses_2x2, "poses_2x2");
kind_command!(cmd_scribbles_parts_chroma, "parts_chroma");
kind_command!(cmd_scribbles_actions, "actions");
kind_command!(cmd_scribbles_layers, "layers");
kind_command!(cmd_scribbles_custom, "custom");

pub fn cmd_scribbles_moppet(s: &mut Session, p: &Value) -> Result<Value> {
    if p["open"].as_bool() == Some(true) {
        return cmd_scribbles_open(s, p);
    }
    let project = s.project_path.clone().ok_or_else(|| bad("Open a project before running scribbles.moppet"))?;
    let root = project.parent().ok_or_else(|| bad("Project has no parent directory"))?;
    let char_raw = p["character"].as_str().or_else(|| p["name"].as_str()).filter(|v| !v.trim().is_empty()).unwrap_or("moppet");
    let char_id = crate::workspace::sanitize_asset_id(char_raw);
    let prompt = p["prompt"].as_str().map(str::trim).filter(|v| !v.is_empty()).unwrap_or(char_raw);
    let provider = resolve_scribbles_provider(p)?;
    let model = p["model"].as_str().filter(|v| !v.trim().is_empty());

    let staging_dir = root.join("assets/characters").join(&char_id).join("scribbles_moppet_bundle");
    std::fs::create_dir_all(&staging_dir)?;

    let char_png = staging_dir.join("character.png");
    let existing_ref =
        p["reference"].as_str().filter(|v| !v.trim().is_empty()).map(|raw| crate::workspace::resolve_workspace_path(s, raw)).filter(|pb| pb.is_file());
    let char_gen = if let Some(ref ref_path) = existing_ref
        && p["reuse_reference"].as_bool().unwrap_or(false)
    {
        std::fs::copy(ref_path, &char_png)?;
        json!({ "image": char_png, "reused_reference": true })
    } else {
        let mut req1 = json!({"prompt": prompt, "character": char_id, "provider": provider, "remove_background": true, "name": format!("{char_id}-character")});
        if let Some(m) = model {
            req1["model"] = json!(m);
        }
        if let Some(notes) = p["notes"].as_str() {
            req1["notes"] = json!(notes);
        }
        let step1 = if let Some(ref ref_path) = existing_ref {
            req1["reference"] = json!(ref_path);
            draw(s, &req1, "image")?
        } else {
            draw(s, &req1, "new")?
        };
        let drawn_path = step1["image"].as_str().ok_or_else(|| bad("character draw produced no image path"))?;
        std::fs::copy(drawn_path, &char_png)?;
        step1
    };

    let parts_png = staging_dir.join("parts-sheet.png");
    let mut req2 = json!({"prompt": prompt, "reference": char_png, "character": char_id, "provider": provider, "name": format!("{char_id}-parts-sheet")});
    if let Some(m) = model {
        req2["model"] = json!(m);
    }
    let step2 = draw(s, &req2, "parts_chroma")?;
    std::fs::copy(step2["image"].as_str().ok_or_else(|| bad("parts_chroma draw produced no image"))?, &parts_png)?;
    let _ = std::fs::copy(&parts_png, root.join("assets/characters").join(&char_id).join("parts-sheet.png"));

    let poses_png = staging_dir.join("pose-sheet.png");
    let mut req3 = json!({"prompt": prompt, "reference": char_png, "character": char_id, "provider": provider, "name": format!("{char_id}-pose-sheet")});
    if let Some(m) = model {
        req3["model"] = json!(m);
    }
    let step3 = draw(s, &req3, "poses_2x2")?;
    std::fs::copy(step3["image"].as_str().ok_or_else(|| bad("poses_2x2 draw produced no image"))?, &poses_png)?;

    let handoff_doc = json!({"name": char_raw, "id": char_id, "provider": provider, "prompt": prompt, "character_image": "character.png", "parts_sheet": "parts-sheet.png", "pose_sheet": "pose-sheet.png"});
    craft_atomic::atomic_write(&staging_dir.join("handoff.json"), &serde_json::to_vec_pretty(&handoff_doc).unwrap_or_default())?;
    let ingested = cmd_scribbles_ingest(s, &json!({ "dir": staging_dir, "character": char_id }))?;
    let reassembled = if p["reassemble"].as_bool().unwrap_or(true) {
        cmd_scribbles_reassemble(s, &json!({"character": char_id, "view": "front", "sheet": parts_png, "rig": p["rig"].as_bool().unwrap_or(true)})).ok()
    } else {
        None
    };

    Ok(json!({
        "ok": true,
        "format": "magic.scribbles-moppet-pack",
        "character": char_id,
        "provider": provider,
        "image": char_png,
        "character_image": char_png,
        "parts_sheet": parts_png,
        "pose_sheet": poses_png,
        "steps": {"character": char_gen, "parts_sheet": step2, "pose_sheet": step3},
        "ingested": ingested,
        "reassembled": reassembled,
    }))
}

pub fn cmd_scribbles_templates(_: &mut Session, _: &Value) -> Result<Value> {
    let all = templates()?;
    Ok(json!({
        "ok": true,
        "model": all["model"],
        "note": all["note"],
        "kinds": all["kinds"],
        "providers": [
            {"id": crate::craft_ai_codex::CODEX_PROVIDER, "label": "Codex", "mode": "local"},
            {"id": crate::craft_ai_codex::AGY_PROVIDER, "label": "AGY (Gemini Flash)", "mode": "local"},
            {"id": crate::craft_ai_codex::FLOW_PROVIDER, "label": "Google Flow (Browser-Agent)", "mode": "browser-agent"}
        ],
        "flow": {
            "provider": "flow",
            "tool": "Moppet Maker",
            "url": DEFAULT_MOPPET_MAKER_URL,
            "project_url": DEFAULT_FLOW_PROJECT_URL,
            "outputs": ["character.png", "parts-sheet.png", "pose-sheet.png", "handoff.json"],
        },
        "craft_ai": crate::craft_ai_codex::craft_ai_status(),
    }))
}

pub fn cmd_scribbles_video(s: &mut Session, p: &Value) -> Result<Value> {
    let prompt = p["prompt"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("scribbles.video needs prompt=<motion description>"))?;
    let project = s.project_path.clone().ok_or_else(|| bad("Open a project before generating video clips"))?;
    let root = project.parent().ok_or_else(|| bad("Project has no parent directory"))?;
    let name =
        p["name"].as_str().map(crate::workspace::sanitize_asset_id).filter(|v| !v.is_empty()).unwrap_or_else(|| format!("flow-clip-{}", std::process::id()));
    let out_mp4 = root.join("assets/clips").join(format!("{name}.mp4"));
    let req = VideoClipRequest {
        prompt,
        out_mp4: &out_mp4,
        model: p["model"].as_str(),
        aspect: p["aspect"].as_str(),
        duration: p["duration"].as_str(),
        resolution: p["resolution"].as_str(),
        ingredient: p["ingredient"].as_str().or_else(|| p["reference"].as_str()),
    };
    craft_scribbles::generate_video_clip(&req).map_err(|e| bad(e.to_string()))?;
    let attached =
        s.execute("project.add_asset", json!({"path": out_mp4, "kind": "video", "name": p["name"].as_str().unwrap_or("Flow video clip"), "load": false}))?;
    Ok(
        json!({"ok": true, "format": "magic.flow-video", "provider": "flow", "prompt": prompt, "video": out_mp4, "asset_id": attached["asset_id"], "status": "ready"}),
    )
}

pub fn cmd_scribbles_svg(s: &mut Session, p: &Value) -> Result<Value> {
    let sheet_raw = p["sheet"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("scribbles.svg needs sheet=<2x2 pose sheet PNG>"))?;
    let sheet = crate::workspace::resolve_workspace_path(s, sheet_raw);
    let project = s.project_path.clone().ok_or_else(|| bad("Open a project before building animated SVGs"))?;
    let root = project.parent().ok_or_else(|| bad("Project has no parent directory"))?;
    let char_id = p["character"].as_str().map(crate::workspace::sanitize_asset_id).unwrap_or_else(|| "moppet".into());
    let action = p["action"].as_str().unwrap_or("wave");
    let out_svg = root.join("assets/characters").join(&char_id).join("sprites").join(action).join(format!("{char_id}-{action}.svg"));
    let fps = p["fps"].as_u64().unwrap_or(6);
    if craft_scribbles::build_animated_svg(&sheet, &out_svg, fps).is_err() {
        CraftHub::trace_subject_vector(&sheet, &out_svg, None, true, false)?;
    }
    let sprite_res = s
        .execute("character.sprite", json!({"character": char_id, "action": action, "name": action, "sheet": sheet, "columns": 2, "rows": 2, "fps": fps}))
        .ok();
    let attached = s.execute(
        "project.add_asset",
        json!({"path": out_svg, "kind": "image", "character": char_id, "name": format!("{char_id}-{action}-svg"), "load": false}),
    )?;
    Ok(
        json!({"ok": true, "format": "magic.animated-svg", "svg": out_svg, "sheet": sheet, "character": char_id, "action": action, "fps": fps, "asset_id": attached["asset_id"], "sprite_project": sprite_res.and_then(|v| v.get("effect_project").cloned())}),
    )
}

/// Separate a `parts-sheet.png` using native `PhotoCraft` (`layer.removeBackground` / `select.colorRange` +
/// `layer.maskAllObjects` + `file.export.layersToFiles`) and optionally rig in `EffectCraft`.
pub fn cmd_scribbles_reassemble(s: &mut Session, p: &Value) -> Result<Value> {
    let project = s.project_path.clone().ok_or_else(|| bad("Open a project before reassembling a parts sheet"))?;
    let root = project.parent().ok_or_else(|| bad("Project has no parent directory"))?;
    let char_id = p["character"].as_str().map(crate::workspace::sanitize_asset_id).unwrap_or_else(|| "moppet".into());
    let view = p["view"].as_str().unwrap_or("front");
    let char_dir = root.join("assets/characters").join(&char_id);
    let sheet = p["sheet"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
        .unwrap_or_else(|| char_dir.join("parts-sheet.png"));
    if !sheet.is_file() {
        return Err(bad(format!("parts sheet {} does not exist", sheet.display())));
    }
    let chroma = p["chroma"].as_str().or(Some("#00FF00"));
    let mut out = CraftHub::split_parts_sheet_photo(&sheet, &char_dir.join(view), &char_id, view, chroma)?;
    if p["rig"].as_bool().unwrap_or(true) {
        if let Ok(cut_res) = s.execute("character.cut", json!({"character": char_id, "view": view, "rig": true})) {
            out["cut_and_rig"] = cut_res;
        }
        let action = p["action"].as_str().unwrap_or("idle");
        if let Ok(act_res) = s.execute("character.action", json!({"character": char_id, "view": view, "action": action})) {
            out["animated_action"] = act_res;
        }
    }
    Ok(out)
}
