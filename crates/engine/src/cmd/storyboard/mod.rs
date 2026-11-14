//! Creative `DeckCraft` Storyboard (`storyboard.deckcraft` / `storyboard.pptx`): optional,
//! aspect-ratio-flexible presentation decks that never block production readiness (`B10-SB`, `U25`).

use std::path::{Path, PathBuf};

use craft_zip::{Archive, Compression, Limits, Writer};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub const DEFAULT_STORYBOARD_FILE: &str = "storyboard.deckcraft";
const DECK_LIMITS: Limits = Limits { max_entries: 4_096, max_total_bytes: 128 << 20 };
const ENTRY_MAX: u64 = 16 << 20;

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn resolve_deck_path(s: &Session, p: &Value) -> Result<PathBuf> {
    if let Some(raw) = p.get("path").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
        let mut path = crate::workspace::resolve_workspace_path(s, raw);
        if path.extension().is_none() {
            path.set_extension("deckcraft");
        }
        return Ok(path);
    }
    let dir = crate::workshop::project_dir(s)?;
    let (deck, pptx) = (dir.join(DEFAULT_STORYBOARD_FILE), dir.join("storyboard.pptx"));
    Ok(if !deck.is_file() && pptx.is_file() { pptx } else { deck })
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

fn inspect_deck_zip_slides(deck_path: &Path) -> usize {
    let Ok(bytes) = std::fs::read(deck_path) else { return 0 };
    let Ok(mut zip) = Archive::open(&bytes, DECK_LIMITS) else { return 0 };
    if zip.has_duplicate_names() {
        return 0;
    }
    if let Ok(Some(raw)) = zip.read("document.json", ENTRY_MAX)
        && let Ok(doc) = serde_json::from_slice::<Value>(&raw)
        && let Some(arr) = doc["slides"].as_array()
    {
        return arr.len();
    }
    (1..=512).take_while(|i| zip.read(&format!("ppt/slides/slide{i}.xml"), ENTRY_MAX).ok().flatten().is_some()).count()
}

fn write_minimal_deck_seed(deck_path: &Path, title: &str, width: u32, height: u32) -> Result<()> {
    let doc = json!({"format": "deckcraft", "version": 1, "title": title, "slide_size": [width, height], "slides": [{"index": 0, "title": title}]});
    let doc_bytes = serde_json::to_vec_pretty(&doc).map_err(|e| bad(e.to_string()))?;
    let slide_xml = format!("<p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><!-- {title} {width}x{height} --></p:sld>");
    let mut writer = Writer::new();
    writer.add("document.json", &doc_bytes, Compression::Deflated).map_err(|e| bad(e.to_string()))?;
    writer.add("ppt/slides/slide1.xml", slide_xml.as_bytes(), Compression::Deflated).map_err(|e| bad(e.to_string()))?;
    crate::doc_state::write_file(deck_path, writer.finish().map_err(|e| bad(e.to_string()))?)
}

pub fn cmd_storyboard_status(s: &mut Session, p: &Value) -> Result<Value> {
    let deck = resolve_deck_path(s, p)?;
    let dir = deck.parent().unwrap_or_else(|| Path::new("."));
    let exists = deck.is_file();
    let slides = rendered_slides(dir);
    let cli_info = exists.then(|| deck.to_str()).flatten().and_then(|dp| crate::craft_api::run_host_cli("deckcraft", &["info", dp]).ok());
    let slide_count = cli_info
        .as_ref()
        .and_then(|v| v["slides"].as_u64().or_else(|| v["slide_count"].as_u64()))
        .map_or_else(|| if exists { inspect_deck_zip_slides(&deck).max(slides.len()) } else { slides.len() }, |n| n as usize);
    let pages: Vec<Value> = (0..slide_count).map(|i| json!({"index": i, "title": format!("Slide {}", i + 1), "preview_png": slides.get(i)})).collect();
    Ok(json!({
        "ok": true, "exists": exists, "path": deck, "slides_dir": dir.join("storyboard/slides"),
        "rendered_slides": slides, "slide_count": slide_count, "pages": pages, "cli_info": cli_info,
        "production_ready": true, "light": if exists { "green" } else { "optional" },
    }))
}

pub fn cmd_storyboard_open(s: &mut Session, p: &Value) -> Result<Value> {
    let deck = resolve_deck_path(s, p)?;
    let launched = crate::craft_api::execute_in_live_app("deckcraft", "file.open", json!({"path": deck})).is_ok()
        || (deck.is_file() && crate::workspace::resolve_executable_path("deckcraft").is_some() && crate::workspace::open_editor("deckcraft", &deck).is_ok());
    Ok(json!({"ok": true, "app": "deckcraft", "path": deck, "exists": deck.is_file(), "launched": launched}))
}

pub fn cmd_storyboard_new(s: &mut Session, p: &Value) -> Result<Value> {
    let mut deck = resolve_deck_path(s, p)?;
    let overwrite = p["overwrite"].as_bool().unwrap_or(false);
    let tpl_opt = p["template"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|r| crate::workspace::resolve_workspace_path(s, r))
        .or_else(|| crate::project_settings::storyboard_template_path(s));
    if p.get("path").is_none() && tpl_opt.as_ref().and_then(|t| t.extension()).is_some_and(|e| e.eq_ignore_ascii_case("pptx")) {
        deck.set_extension("pptx");
    }
    if deck.exists() && !overwrite {
        return Err(bad(format!("storyboard already exists: {} (pass overwrite=true)", deck.display())));
    }
    if let Some(dir) = deck.parent() {
        std::fs::create_dir_all(dir.join("storyboard/slides"))?;
    }
    let title = p["title"].as_str().filter(|t| !t.trim().is_empty()).unwrap_or("Storyboard");
    let (w, h) = (p["width"].as_u64().unwrap_or(1920).clamp(320, 7680) as u32, p["height"].as_u64().unwrap_or(1080).clamp(240, 4320) as u32);
    if let Some(tpl) = tpl_opt.filter(|t| t.is_file()) {
        crate::doc_state::atomic_copy_file(&tpl, &deck, overwrite)?;
    } else if let Some(dp) = deck.to_str().filter(|_| crate::workspace::resolve_executable_path("deckcraft-cli").is_some()) {
        let new_arg = format!("file.new={}", json!({"blank": true}));
        let slide_arg = format!("slide.new={}", json!({"layout": "titleOnly", "title": title}));
        crate::craft_api::run_host_cli("deckcraft", &["run", "--cmd", &new_arg, "--cmd", &slide_arg, "--save", dp]).map_err(bad)?;
    } else {
        write_minimal_deck_seed(&deck, title, w, h)?;
    }
    cmd_storyboard_status(s, &json!({"path": deck}))
}

pub fn cmd_storyboard_add_section(s: &mut Session, p: &Value) -> Result<Value> {
    let deck = resolve_deck_path(s, p)?;
    let name =
        p["section"].as_str().or_else(|| p["name"].as_str()).map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("section or name is required"))?;
    let dp = deck.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let out =
        crate::craft_api::run_host_cli("deckcraft", &["run", dp, "--cmd", &format!("section.add={}", json!({"name": name})), "--save", dp]).map_err(bad)?;
    Ok(json!({"ok": true, "path": deck, "section": name, "result": out}))
}

pub fn cmd_storyboard_add_element(s: &mut Session, p: &Value) -> Result<Value> {
    let deck = resolve_deck_path(s, p)?;
    let section = p["section"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("section is required"))?;
    let name = p["name"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("name is required"))?;
    let dp = deck.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let cmd_arg = match p["image"].as_str().filter(|v| !v.trim().is_empty()) {
        Some(img) => format!("insert.picture={}", json!({"section": section, "name": name, "path": crate::workspace::resolve_workspace_path(s, img)})),
        None => format!("shape.insert={}", json!({"section": section, "name": name, "text": p["text"].as_str().unwrap_or(name)})),
    };
    let out = crate::craft_api::run_host_cli("deckcraft", &["run", dp, "--cmd", &cmd_arg, "--save", dp]).map_err(bad)?;
    Ok(json!({"ok": true, "path": deck, "section": section, "name": name, "result": out}))
}

pub fn cmd_storyboard_export(s: &mut Session, p: &Value) -> Result<Value> {
    let deck = resolve_deck_path(s, p)?;
    let dir = deck.parent().unwrap_or_else(|| Path::new("."));
    let dp = deck.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let format = p["format"].as_str().unwrap_or("png").to_ascii_lowercase();
    if matches!(format.as_str(), "pdf" | "pptx") {
        let out_file = p["output"].as_str().map(|r| crate::workspace::resolve_workspace_path(s, r)).unwrap_or_else(|| dir.join(format!("storyboard.{format}")));
        let op = out_file.to_str().ok_or_else(|| bad("invalid export path"))?;
        let res = crate::craft_api::run_host_cli("deckcraft", &["convert", dp, op]).map_err(bad)?;
        return Ok(json!({"ok": true, "format": format, "output": out_file, "result": res}));
    }
    let slides_dir = p["png_dir"].as_str().map(|r| crate::workspace::resolve_workspace_path(s, r)).unwrap_or_else(|| dir.join("storyboard/slides"));
    std::fs::create_dir_all(&slides_dir)?;
    let sd = slides_dir.to_str().ok_or_else(|| bad("invalid slides dir"))?;
    let res = crate::craft_api::run_host_cli("deckcraft", &["render", dp, "--all", sd]).map_err(bad)?;
    Ok(json!({"ok": true, "format": "png", "slides_dir": slides_dir, "slides": rendered_slides(dir), "result": res}))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{cmd_storyboard_new, write_minimal_deck_seed};
    use crate::Session;

    #[test]
    fn u25_starter_storyboard_template_is_copied_and_never_mutated() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let sandbox = std::env::temp_dir().join(format!("magic_u25_tpl_{nanos}"));
        std::fs::create_dir_all(sandbox.join("proj")).unwrap();
        let starter_tpl = sandbox.join("custom-starter.potx");
        write_minimal_deck_seed(&starter_tpl, "Starter Master", 1080, 1920).unwrap();
        let original_bytes = std::fs::read(&starter_tpl).unwrap();

        let mut session = Session { project_path: Some(sandbox.join("proj/song.magic")), ..Session::default() };
        session.settings.set_value("project.storyboard_template", &json!(starter_tpl.to_string_lossy()), false).unwrap();
        let status = cmd_storyboard_new(&mut session, &json!({"title": "New Deck"})).unwrap();
        assert_eq!(status["exists"], true);
        assert_eq!(status["production_ready"], true);
        assert_eq!(status["slide_count"], 1);
        assert_eq!(std::fs::read(&starter_tpl).unwrap(), original_bytes, "starter template must never be mutated");
        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
