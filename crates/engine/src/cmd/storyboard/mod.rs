//! Creative `DesignCraft` Storyboard (`storyboard.designcraft` / `.idml`): optional,
//! aspect-ratio-flexible print/digital storyboard documents coordinated with host `DesignCraft`
//! (`designcraft --control 7979` / `designcraft-cli`) that never block production readiness (`B10-SB`, `U25`).

use std::path::{Path, PathBuf};

use craft_zip::{Archive, Compression, Limits, Writer};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub const DEFAULT_STORYBOARD_FILE: &str = "storyboard.designcraft";
const DESIGN_LIMITS: Limits = Limits { max_entries: 4_096, max_total_bytes: 128 << 20 };
const ENTRY_MAX: u64 = 16 << 20;
const LETTER_W_PT: f64 = 612.0;
const LETTER_H_PT: f64 = 792.0;

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn resolve_design_path(s: &Session, p: &Value) -> Result<PathBuf> {
    if let Some(raw) = p.get("path").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
        let mut path = crate::workspace::resolve_workspace_path(s, raw);
        if path.extension().is_none() {
            path.set_extension("designcraft");
        }
        return Ok(path);
    }
    let dir = crate::workshop::project_dir(s)?;
    let (design, idml) = (dir.join(DEFAULT_STORYBOARD_FILE), dir.join("storyboard.idml"));
    Ok(if !design.is_file() && idml.is_file() { idml } else { design })
}

fn rendered_slides(project_dir: &Path) -> Vec<PathBuf> {
    let mut slides: Vec<PathBuf> = std::fs::read_dir(project_dir.join("storyboard/slides"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && crate::workspace::has_extension(p, &["png"]))
        .collect();
    slides.sort();
    slides
}

fn read_designcraft_document(doc_path: &Path) -> Option<Value> {
    let bytes = std::fs::read(doc_path).ok()?;
    let mut zip = Archive::open(&bytes, DESIGN_LIMITS).ok()?;
    if zip.has_duplicate_names() {
        return None;
    }
    let raw = zip.read("document.json", ENTRY_MAX).ok().flatten()?;
    serde_json::from_slice::<Value>(&raw).ok()
}

fn write_designcraft_zip(doc_path: &Path, doc: &Value) -> Result<()> {
    let meta = json!({"format": "designcraft", "version": 1, "generator": "magic"});
    let meta_bytes = serde_json::to_vec_pretty(&meta).map_err(|e| bad(e.to_string()))?;
    let doc_bytes = serde_json::to_vec_pretty(doc).map_err(|e| bad(e.to_string()))?;
    let mut writer = Writer::new();
    writer.add("meta.json", &meta_bytes, Compression::Deflated).map_err(|e| bad(e.to_string()))?;
    writer.add("document.json", &doc_bytes, Compression::Deflated).map_err(|e| bad(e.to_string()))?;
    crate::doc_state::write_file(doc_path, writer.finish().map_err(|e| bad(e.to_string()))?)
}

fn write_minimal_design_seed(doc_path: &Path, title: &str, width_pt: f64, height_pt: f64) -> Result<()> {
    let doc = json!({
        "format": "designcraft",
        "version": 1,
        "title": title,
        "page_size_pt": [width_pt, height_pt],
        "sections": ["Cover"],
        "pages": [{
            "index": 0,
            "section": "Cover",
            "title": title,
            "bounds": [0.0, 0.0, width_pt, height_pt],
            "items": [{
                "kind": "text",
                "name": "title",
                "rect": [36.0, 36.0, width_pt - 72.0, 120.0],
                "text": title,
            }],
        }],
    });
    write_designcraft_zip(doc_path, &doc)
}

fn page_count_from_doc(doc: &Value) -> Option<usize> {
    if let Some(arr) = doc["pages"].as_array() {
        return Some(arr.len());
    }
    doc["spreads"]
        .as_array()
        .map(|spreads| spreads.iter().map(|sp| sp["pages"].as_array().map_or(1, Vec::len)).sum())
        .filter(|&n| n > 0)
}

pub fn cmd_storyboard_status(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let dir = doc_path.parent().unwrap_or_else(|| Path::new("."));
    let exists = doc_path.is_file();
    let slides = rendered_slides(dir);
    let live_info = if exists && crate::workspace::control_probe("designcraft") {
        crate::workspace::control_call("designcraft", "document.inspect", json!({})).ok()
    } else {
        None
    };
    let zip_doc = exists.then(|| read_designcraft_document(&doc_path)).flatten();
    let page_count = live_info
        .as_ref()
        .and_then(|v| v["page_count"].as_u64().or_else(|| v["pages"].as_array().map(|a| a.len() as u64)))
        .map(|n| n as usize)
        .or_else(|| zip_doc.as_ref().and_then(page_count_from_doc))
        .unwrap_or(0)
        .max(slides.len());

    let doc_pages = zip_doc.as_ref().and_then(|d| d["pages"].as_array());
    let sections: Vec<String> = zip_doc
        .as_ref()
        .and_then(|d| d["sections"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    let pages: Vec<Value> = (0..page_count)
        .map(|i| {
            let dp = doc_pages.and_then(|arr| arr.get(i));
            let title = dp.and_then(|v| v["title"].as_str()).map(str::to_owned).unwrap_or_else(|| format!("Page {}", i + 1));
            let section = dp.and_then(|v| v["section"].as_str()).unwrap_or("Cover");
            let bounds = dp.and_then(|v| v.get("bounds")).cloned().unwrap_or_else(|| json!([0.0, 0.0, LETTER_W_PT, LETTER_H_PT]));
            json!({
                "page": i,
                "index": i,
                "section": section,
                "title": title,
                "bounds": bounds,
                "preview_png": slides.get(i),
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "app": "designcraft",
        "exists": exists,
        "path": doc_path,
        "slides_dir": dir.join("storyboard/slides"),
        "rendered_slides": slides,
        "slide_count": page_count,
        "page_count": page_count,
        "sections": sections,
        "pages": pages,
        "inspect": live_info,
        "production_ready": true,
        "light": if exists { "green" } else { "optional" },
    }))
}

pub fn cmd_storyboard_open(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let launched = (crate::workspace::control_probe("designcraft")
        && crate::workspace::control_call("designcraft", "app.open", json!({"path": doc_path})).is_ok())
        || (doc_path.is_file()
            && crate::workspace::resolve_executable_path("designcraft").is_some()
            && crate::workspace::open_editor("designcraft", &doc_path).is_ok());
    Ok(json!({"ok": true, "app": "designcraft", "path": doc_path, "exists": doc_path.is_file(), "launched": launched}))
}

pub fn cmd_storyboard_new(s: &mut Session, p: &Value) -> Result<Value> {
    let mut doc_path = resolve_design_path(s, p)?;
    let overwrite = p["overwrite"].as_bool().unwrap_or(false);
    let tpl_opt = p["template"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|r| crate::workspace::resolve_workspace_path(s, r))
        .or_else(|| crate::project_settings::storyboard_template_path(s));
    if p.get("path").is_none() && tpl_opt.as_ref().and_then(|t| t.extension()).is_some_and(|e| e.eq_ignore_ascii_case("idml")) {
        doc_path.set_extension("idml");
    }
    if doc_path.exists() && !overwrite {
        return Err(bad(format!("storyboard already exists: {} (pass overwrite=true)", doc_path.display())));
    }
    if let Some(dir) = doc_path.parent() {
        std::fs::create_dir_all(dir.join("storyboard/slides"))?;
    }
    let title = p["title"].as_str().filter(|t| !t.trim().is_empty()).unwrap_or("Storyboard");
    let w_pt = p["width"].as_f64().filter(|v| v.is_finite() && *v >= 72.0).unwrap_or(LETTER_W_PT);
    let h_pt = p["height"].as_f64().filter(|v| v.is_finite() && *v >= 72.0).unwrap_or(LETTER_H_PT);
    if let Some(tpl) = tpl_opt.filter(|t| t.is_file()) {
        crate::doc_state::atomic_copy_file(&tpl, &doc_path, overwrite)?;
    } else if let Some(dp) = doc_path.to_str().filter(|_| crate::workspace::resolve_executable_path("designcraft-cli").is_some()) {
        let new_arg = format!("file.new={}", json!({"width": w_pt, "height": h_pt, "pages": 1, "facingPages": false}));
        let frame_arg = format!("frame.create={}", json!({"rect": [36.0, 36.0, w_pt - 72.0, 120.0], "content": "text", "text": title}));
        let cli_res = if crate::workspace::has_extension(&doc_path, &["idml"]) {
            crate::workspace::run_cli("designcraft-cli", &["run", "--cmd", &new_arg, "--cmd", &frame_arg, "--export", dp], None)
        } else {
            let save_arg = format!("file.saveAs={}", json!({"path": dp}));
            crate::workspace::run_cli("designcraft-cli", &["run", "--cmd", &new_arg, "--cmd", &frame_arg, "--cmd", &save_arg], None)
        };
        if cli_res.is_err() {
            write_minimal_design_seed(&doc_path, title, w_pt, h_pt)?;
        }
    } else {
        write_minimal_design_seed(&doc_path, title, w_pt, h_pt)?;
    }
    cmd_storyboard_status(s, &json!({"path": doc_path}))
}

pub fn cmd_storyboard_add_section(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let name =
        p["section"].as_str().or_else(|| p["name"].as_str()).map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("section or name is required"))?;
    let dp = doc_path.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let cli_out = doc_path
        .is_file()
        .then(|| crate::workspace::resolve_executable_path("designcraft-cli").is_some())
        .filter(|&avail| avail)
        .and_then(|_| {
            let cmd_arg = format!("frame.create={}", json!({"rect": [36.0, 36.0, LETTER_W_PT - 72.0, 84.0], "content": "text", "text": name}));
            let save_arg = format!("file.saveAs={}", json!({"path": dp}));
            crate::workspace::run_cli("designcraft-cli", &["run", "--in", dp, "--cmd", &cmd_arg, "--cmd", &save_arg], None).ok()
        });
    let out = if let Some(res) = cli_out {
        res
    } else {
        let mut doc = read_designcraft_document(&doc_path).unwrap_or_else(|| json!({"format": "designcraft", "version": 1, "sections": [], "pages": []}));
        let idx = doc["pages"].as_array().map_or(0, Vec::len);
        if let Some(secs) = doc["sections"].as_array_mut()
            && !secs.iter().any(|v| v.as_str() == Some(name))
        {
            secs.push(json!(name));
        }
        if let Some(pages) = doc["pages"].as_array_mut() {
            pages.push(json!({
                "index": idx,
                "section": name,
                "title": name,
                "bounds": [0.0, 0.0, LETTER_W_PT, LETTER_H_PT],
                "items": [],
            }));
        }
        write_designcraft_zip(&doc_path, &doc)?;
        "updated local designcraft document".into()
    };
    Ok(json!({"ok": true, "app": "designcraft", "path": doc_path, "section": name, "result": out}))
}

pub fn cmd_storyboard_add_element(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let section = p["section"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("section is required"))?;
    let name = p["name"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("name is required"))?;
    let dp = doc_path.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let cli_out = doc_path
        .is_file()
        .then(|| crate::workspace::resolve_executable_path("designcraft-cli").is_some())
        .filter(|&avail| avail)
        .and_then(|_| {
            let cmd_arg = match p["image"].as_str().filter(|v| !v.trim().is_empty()) {
                Some(img) => format!("file.place={}", json!({"path": crate::workspace::resolve_workspace_path(s, img)})),
                None => format!(
                    "frame.create={}",
                    json!({"rect": [36.0, 140.0, LETTER_W_PT - 72.0, 180.0], "content": "text", "text": p["text"].as_str().unwrap_or(name)})
                ),
            };
            let save_arg = format!("file.saveAs={}", json!({"path": dp}));
            crate::workspace::run_cli("designcraft-cli", &["run", "--in", dp, "--cmd", &cmd_arg, "--cmd", &save_arg], None).ok()
        });
    let out = if let Some(res) = cli_out {
        res
    } else {
        let mut doc = read_designcraft_document(&doc_path).unwrap_or_else(|| json!({"format": "designcraft", "version": 1, "sections": [section], "pages": []}));
        let item = match p["image"].as_str().filter(|v| !v.trim().is_empty()) {
            Some(img) => json!({"kind": "image", "name": name, "section": section, "path": crate::workspace::resolve_workspace_path(s, img)}),
            None => json!({"kind": "text", "name": name, "section": section, "text": p["text"].as_str().unwrap_or(name)}),
        };
        if let Some(pages) = doc["pages"].as_array_mut() {
            if let Some(page) = pages.iter_mut().find(|pg| pg["section"].as_str() == Some(section)).or_else(|| pages.first_mut()) {
                if let Some(items) = page["items"].as_array_mut() {
                    items.push(item);
                } else {
                    page["items"] = json!([item]);
                }
            } else {
                pages.push(json!({"index": 0, "section": section, "title": section, "bounds": [0.0, 0.0, LETTER_W_PT, LETTER_H_PT], "items": [item]}));
            }
        }
        write_designcraft_zip(&doc_path, &doc)?;
        "updated local designcraft document".into()
    };
    Ok(json!({"ok": true, "app": "designcraft", "path": doc_path, "section": section, "name": name, "result": out}))
}

pub fn cmd_storyboard_export(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let dir = doc_path.parent().unwrap_or_else(|| Path::new("."));
    let dp = doc_path.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let format = p["format"].as_str().unwrap_or("png").to_ascii_lowercase();
    if matches!(format.as_str(), "pdf" | "idml" | "epub") {
        let out_file =
            p["output"].as_str().map(|r| crate::workspace::resolve_workspace_path(s, r)).unwrap_or_else(|| dir.join(format!("storyboard.{format}")));
        let op = out_file.to_str().ok_or_else(|| bad("invalid export path"))?;
        let res = if crate::workspace::control_probe("designcraft") {
            crate::workspace::control_call("designcraft", "app.export", json!({"path": op, "format": format}))
                .map(|v| v.to_string())
                .map_err(|e| EngineError::ExternalTool("designcraft".into(), e))?
        } else {
            crate::workspace::run_cli("designcraft-cli", &["run", "--in", dp, "--export", op], None)
                .map_err(|e| EngineError::ExternalTool("designcraft-cli".into(), e))?
        };
        return Ok(json!({"ok": true, "app": "designcraft", "format": format, "output": out_file, "result": res}));
    }
    let slides_dir = p["png_dir"].as_str().map(|r| crate::workspace::resolve_workspace_path(s, r)).unwrap_or_else(|| dir.join("storyboard/slides"));
    std::fs::create_dir_all(&slides_dir)?;
    let sd = slides_dir.to_str().ok_or_else(|| bad("invalid slides dir"))?;
    let res = crate::workspace::run_cli("designcraft-cli", &["run", "--in", dp, "--all-pages", sd], None)
        .map_err(|e| EngineError::ExternalTool("designcraft-cli".into(), e))?;
    Ok(json!({"ok": true, "app": "designcraft", "format": "png", "slides_dir": slides_dir, "slides": rendered_slides(dir), "result": res}))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{cmd_storyboard_new, write_minimal_design_seed};
    use crate::Session;

    #[test]
    fn u25_starter_storyboard_template_is_copied_and_never_mutated() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let sandbox = std::env::temp_dir().join(format!("magic_u25_tpl_{nanos}"));
        std::fs::create_dir_all(sandbox.join("proj")).unwrap();
        let starter_tpl = sandbox.join("custom-starter.indt");
        write_minimal_design_seed(&starter_tpl, "Starter Master", 612.0, 792.0).unwrap();
        let original_bytes = std::fs::read(&starter_tpl).unwrap();

        let mut session = Session { project_path: Some(sandbox.join("proj/song.magic")), ..Session::default() };
        session.settings.set_value("project.storyboard_template", &json!(starter_tpl.to_string_lossy()), false).unwrap();
        let status = cmd_storyboard_new(&mut session, &json!({"title": "New Storyboard"})).unwrap();
        assert_eq!(status["exists"], true);
        assert_eq!(status["production_ready"], true);
        assert_eq!(status["slide_count"], 1);
        assert_eq!(std::fs::read(&starter_tpl).unwrap(), original_bytes, "starter template must never be mutated");
        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
