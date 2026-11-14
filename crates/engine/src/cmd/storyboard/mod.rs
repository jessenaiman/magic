//! Read-only `DesignCraft` Storyboard (`storyboard.designcraft`):
//! initializes `storyboard.designcraft` from the starter template, inspects page metadata and
//! renders PNGs through `designcraft-cli`, and opens the file in `DesignCraft`.
//! Never parses internal `.designcraft` archives with `craft_zip` or mutates document objects.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub const DEFAULT_STORYBOARD_FILE: &str = "storyboard.designcraft";
const MASTER_DESIGNCRAFT: &[u8] = include_bytes!("../../../templates/storyboard.designcraft");
const LETTER_W_PT: f64 = 612.0;
const LETTER_H_PT: f64 = 792.0;
const RENDER_SCALE: &str = "1.50";

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn tool_err(e: String) -> EngineError {
    EngineError::ExternalTool("designcraft-cli".into(), e)
}

fn resolve_design_path(s: &Session, p: &Value) -> Result<PathBuf> {
    if let Some(raw) = p.get("path").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
        let mut path = crate::workspace::resolve_workspace_path(s, raw);
        if path.extension().is_none() {
            path.set_extension("designcraft");
        }
        return Ok(path);
    }
    Ok(crate::workshop::project_dir(s)?.join(DEFAULT_STORYBOARD_FILE))
}

fn renders_dir_for(doc_path: &Path) -> PathBuf {
    doc_path.parent().unwrap_or_else(|| Path::new(".")).join("storyboard/renders")
}

fn render_png_path(doc_path: &Path, page: usize) -> PathBuf {
    renders_dir_for(doc_path).join(format!("page-{:03}.png", page + 1))
}

fn count_rendered_pngs(renders_dir: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(renders_dir) else {
        return 0;
    };
    rd.flatten().filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("page-") && n.ends_with(".png"))).count()
}

fn inspect_via_cli(doc_path: &Path) -> Option<(usize, f64, f64)> {
    let dp = doc_path.to_str()?;
    let val = crate::workspace::run_cli_json("designcraft-cli", &["run", "--in", dp, "--cmd", "document.inspect"], None).ok()?;
    let count = val["pageCount"].as_u64()? as usize;
    let w = val.pointer("/settings/pageWidth").and_then(Value::as_f64).filter(|&v| v > 0.0).unwrap_or(LETTER_W_PT);
    let h = val.pointer("/settings/pageHeight").and_then(Value::as_f64).filter(|&v| v > 0.0).unwrap_or(LETTER_H_PT);
    Some((count, w, h))
}

pub fn cmd_storyboard_pages(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let renders_dir = renders_dir_for(&doc_path);
    if !doc_path.is_file() {
        return Ok(json!({
            "ok": true,
            "app": "designcraft",
            "read_only": true,
            "exists": false,
            "path": doc_path,
            "renders_dir": renders_dir,
            "page_count": 0,
            "pages": [],
            "light": "optional",
        }));
    }

    let (cli_count, page_w, page_h) = inspect_via_cli(&doc_path).unwrap_or((0, LETTER_W_PT, LETTER_H_PT));
    let page_count = cli_count.max(count_rendered_pngs(&renders_dir));
    let page_filter = p.get("page").and_then(Value::as_u64).map(|n| n as usize);
    if let Some(req) = page_filter
        && req >= page_count
    {
        return Err(bad(format!("storyboard page {req} not found (page_count={page_count})")));
    }

    let pages_json: Vec<Value> = (0..page_count)
        .filter(|idx| page_filter.is_none_or(|req| req == *idx))
        .map(|idx| {
            let png = render_png_path(&doc_path, idx);
            json!({
                "page": idx,
                "index": idx,
                "page_number": idx + 1,
                "title": format!("Page {:02}", idx + 1),
                "width": page_w,
                "height": page_h,
                "render_png": png.is_file().then_some(png),
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "app": "designcraft",
        "read_only": true,
        "exists": true,
        "path": doc_path,
        "renders_dir": renders_dir,
        "page_count": page_count,
        "pages": pages_json,
        "light": "green",
    }))
}

pub fn cmd_storyboard_status(s: &mut Session, p: &Value) -> Result<Value> {
    cmd_storyboard_pages(s, p)
}

pub fn cmd_storyboard_open(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let launched = doc_path.is_file() && crate::workspace::open_editor("designcraft", &doc_path).is_ok();
    Ok(json!({"ok": true, "app": "designcraft", "path": doc_path, "exists": doc_path.is_file(), "launched": launched}))
}

pub fn cmd_storyboard_new(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    let overwrite = p["overwrite"].as_bool().unwrap_or(false);
    if doc_path.exists() && !overwrite {
        return Err(bad(format!("storyboard already exists: {} (pass overwrite=true)", doc_path.display())));
    }

    let renders_dir = renders_dir_for(&doc_path);
    std::fs::create_dir_all(&renders_dir)?;

    let tpl_opt = p["template"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|r| crate::workspace::resolve_workspace_path(s, r))
        .or_else(|| crate::project_settings::storyboard_template_path(s));
    if let Some(tpl) = tpl_opt.filter(|t| t.is_file()) {
        crate::doc_state::atomic_copy_file(&tpl, &doc_path, overwrite)?;
    } else {
        craft_atomic::atomic_write(&doc_path, MASTER_DESIGNCRAFT)?;
    }

    let dp = doc_path.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let renders_s = renders_dir.to_str().ok_or_else(|| bad("invalid renders directory"))?;
    crate::workspace::run_cli("designcraft-cli", &["run", "--in", dp, "--scale", RENDER_SCALE, "--all-pages", renders_s], None).map_err(tool_err)?;
    cmd_storyboard_pages(s, &json!({"path": doc_path}))
}

pub fn cmd_storyboard_render(s: &mut Session, p: &Value) -> Result<Value> {
    let doc_path = resolve_design_path(s, p)?;
    if !doc_path.is_file() {
        return Err(bad(format!("storyboard does not exist: {}", doc_path.display())));
    }
    let dp = doc_path.to_str().ok_or_else(|| bad("invalid storyboard path"))?;
    let scale = p.get("scale").and_then(Value::as_f64).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(1.5);
    let scale_s = format!("{scale:.2}");

    if let Some(page_u64) = p.get("page").and_then(Value::as_u64) {
        let page = page_u64 as usize;
        let out_png = p
            .get("output")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
            .unwrap_or_else(|| render_png_path(&doc_path, page));
        if let Some(parent) = out_png.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let op = out_png.to_str().ok_or_else(|| bad("invalid render output path"))?;
        let page_s = page.to_string();
        crate::workspace::run_cli("designcraft-cli", &["run", "--in", dp, "--page", &page_s, "--scale", &scale_s, "--export", op], None).map_err(tool_err)?;
        Ok(json!({"ok": true, "app": "designcraft", "page": page, "png": out_png, "scale": scale}))
    } else {
        let renders_dir = renders_dir_for(&doc_path);
        std::fs::create_dir_all(&renders_dir)?;
        let renders_s = renders_dir.to_str().ok_or_else(|| bad("invalid renders directory"))?;
        crate::workspace::run_cli("designcraft-cli", &["run", "--in", dp, "--scale", &scale_s, "--all-pages", renders_s], None).map_err(tool_err)?;
        Ok(json!({"ok": true, "app": "designcraft", "renders_dir": renders_dir, "scale": scale}))
    }
}
