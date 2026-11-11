//! Native `PhotoCraft`, `VectorCraft`, and `EffectCraft` media pipelines routed through [`CraftHub`]:
//! - Background & subject isolation (`PhotoCraft` `layer.removeBackground`, `select.subject`,
//!   `select.colorRange`, `select.refineEdge`, `layer.maskAllObjects`, plus `EffectCraft` `Keylight (1.2)`).
//! - Non-destructive `PhotoCraft` Storyboard panel grading & styles (`layer.newAdjustmentLayer.curves`,
//!   `layer.newAdjustmentLayer.vibrance`, `layer.newAdjustmentLayer.colorLookup` `.cube`/`.3dl`/`.look`,
//!   `filter.sharpen.smartSharpen`, `layer.layerStyle.*`, and `photocraft-cli batch --actions`).
//! - Vector subject & stroke tracing (`VectorCraft` `imageTrace.make` with `ignoreWhite: true`,
//!   `method: "overlapping"`, `createStrokes`, `imageTrace.expand`).
//! - EffectCraft timeline interchange (`file.exportTimeline`) & ProRes 4444 RGBA renders (`renderQueue.*`).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{CraftHub, CraftTarget, ScopedTempDir, cli};
use crate::{EngineError, Result};

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

impl CraftHub {
    /// True when the target ArtCraft binary or CLI is installed on the host (`PATH` / ArtCraft Launcher).
    pub fn is_installed(target: CraftTarget) -> bool {
        crate::workspace::find_executable(target.cli_name()).is_ok() || crate::workspace::find_executable(target.app_bin()).is_ok()
    }

    /// Export a flat PNG composite preview of any PhotoCraft-supported layered or HDR file
    /// (`.pcraft`, `.psd`, `.psb`, `.ora`, `.xcf`, `.kra`, `.pdn`, `.afphoto`, `.exr`, `.hdr`, `.fits`, `.heic`).
    pub fn export_photo_preview(doc: &Path, out_png: &Path) -> Result<()> {
        cli::export_photo_flat(doc, out_png)
    }

    /// Export a flat PNG or PDF rendition of any VectorCraft-supported vector/Affinity file
    /// (`.vectorcraft`, `.ai`, `.eps`, `.svg`, `.svgz`, `.dxf`, `.emf`, `.wmf`, `.af`, `.afdesign`, `.afpub`).
    pub fn export_vector_preview(doc: &Path, out_file: &Path) -> Result<()> {
        cli::export_vector_flat(doc, out_file)
    }

    /// Run `photocraft-cli batch --actions <actions.json> --in <in_dir> --out <out_dir>` across a folder.
    pub fn batch_grade_photo_folder(actions: &Value, in_dir: &Path, out_dir: &Path) -> Result<Value> {
        if !in_dir.is_dir() {
            return Err(bad(format!("input folder does not exist: {}", in_dir.display())));
        }
        let tmp = ScopedTempDir::new("magic-photocraft-batch")?;
        let actions_path = tmp.path().join("grade.json");
        let raw = serde_json::to_vec_pretty(actions).map_err(|e| bad(e.to_string()))?;
        craft_atomic::atomic_write(&actions_path, &raw)?;
        cli::run_photo_batch_actions(&actions_path, in_dir, out_dir)
    }

    /// Alias for [`Self::remove_background_photo`] used by `craft_ai` and `photo.*` callers.
    pub fn isolate_subject_photo(input: &Path, output: &Path, chroma_hex: Option<&str>, feather_px: f64) -> Result<Value> {
        Self::remove_background_photo(input, output, chroma_hex, feather_px)
    }

    /// Isolate the foreground subject and remove the background of `input` into `output` (`.png` / `.psd` / `.pcraft`)
    /// using native `PhotoCraft` commands — never hand-rolled pixel loops or overwriting `input` headless.
    pub fn remove_background_photo(input: &Path, output: &Path, chroma_hex: Option<&str>, feather_px: f64) -> Result<Value> {
        if !craft_util::media::is_craft_doc_path(input) {
            return Err(bad(format!("input image does not exist: {}", input.display())));
        }
        if super::same_canonical_path(input, output) {
            return Err(bad("PhotoCraft background removal writes to a distinct output path to preserve source pixels"));
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let feather = feather_px.clamp(0.0, 12.0);
        let steps: Vec<(&str, Value)> = if let Some(hex) = chroma_hex.map(str::trim).filter(|s| !s.is_empty()) {
            vec![
                ("select.colorRange", json!({"color": hex, "fuzziness": 42, "invert": true})),
                ("select.refineEdge", json!({"smooth": 2, "feather": feather, "contrast": 20, "shiftEdge": -10})),
                ("layer.layerMask.revealSelection", json!({})),
                ("layer.rasterize.allLayers", json!({})),
                ("select.deselect", json!({})),
            ]
        } else {
            vec![("layer.removeBackground", json!({})), ("select.refineEdge", json!({"smooth": 2, "feather": feather, "contrast": 16, "shiftEdge": -6}))]
        };
        let results = Self::run_batch(CraftTarget::Photo, Some(input), Some(output), &steps)?;
        Ok(json!({
            "ok": true,
            "engine": "PhotoCraft",
            "input": input,
            "output": output,
            "chroma": chroma_hex,
            "steps": results.len(),
        }))
    }

    /// Apply non-destructive PhotoCraft adjustment layers, smart sharpen, background isolation, or
    /// layer styles for a Storyboard image slot, writing both an editable `.pcraft` project and a
    /// flat composite `.png` preview.
    pub fn process_storyboard_photo(input: &Path, out_pcraft: &Path, out_png: &Path, opts: &Value) -> Result<Value> {
        if !craft_util::media::is_craft_doc_path(input) {
            return Err(bad(format!("input image does not exist: {}", input.display())));
        }
        if let Some(parent) = out_pcraft.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = out_png.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut steps: Vec<(&'static str, Value)> = Vec::new();
        if opts["remove_background"].as_bool() == Some(true) {
            if let Some(hex) = opts["chroma"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
                steps.push(("select.colorRange", json!({"color": hex, "fuzziness": 42, "invert": true})));
                steps.push(("select.refineEdge", json!({"smooth": 2, "feather": 0.8, "contrast": 20, "shiftEdge": -10})));
                steps.push(("layer.layerMask.revealSelection", json!({})));
                steps.push(("select.deselect", json!({})));
            } else {
                steps.push(("layer.removeBackground", json!({})));
                steps.push(("select.refineEdge", json!({"smooth": 2, "feather": 0.8, "contrast": 16, "shiftEdge": -6})));
            }
        }
        if opts["auto_tone"].as_bool() == Some(true) {
            steps.push(("image.autoTone", json!({})));
        }
        if opts["auto_contrast"].as_bool() == Some(true) {
            steps.push(("image.autoContrast", json!({})));
        }
        if opts["auto_color"].as_bool() == Some(true) {
            steps.push(("image.autoColor", json!({})));
        }
        if let Some(vib) = opts["vibrance"].as_f64() {
            steps.push(("layer.newAdjustmentLayer.vibrance", json!({"vibrance": vib, "saturation": opts["saturation"].as_f64().unwrap_or(0.0)})));
        }
        if let Some(pts) = opts["curves"].as_array().filter(|a| !a.is_empty()) {
            steps.push(("layer.newAdjustmentLayer.curves", json!({"points": pts})));
        }
        if let Some(lut) = opts["lut"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
            steps.push(("layer.newAdjustmentLayer.colorLookup", json!({"path": lut})));
        }
        if let Some(amt) = opts["sharpen"].as_f64().filter(|&v| v > 0.0) {
            steps.push(("filter.sharpen.smartSharpen", json!({"amount": amt})));
        }
        if let Some(stroke_obj) = opts.get("stroke").filter(|v| v.is_object()) {
            steps.push(("layer.layerStyle.stroke", stroke_obj.clone()));
        }
        if let Some(shadow_obj) = opts.get("drop_shadow").filter(|v| v.is_object()) {
            steps.push(("layer.layerStyle.dropShadow", shadow_obj.clone()));
        }
        steps.push(("file.saveACopy", json!({"path": out_png, "overwrite": true})));
        let res = Self::run_batch(CraftTarget::Photo, Some(input), Some(out_pcraft), &steps)?;
        Ok(json!({
            "ok": true,
            "engine": "PhotoCraft",
            "pcraft": out_pcraft,
            "png": out_png,
            "steps": res.len(),
        }))
    }

    /// Split a character `parts-sheet.png` into clean transparent layers, `.psd`, `.pcraft`, and `parts.json`
    /// using `PhotoCraft`'s native `layer.removeBackground` / `select.colorRange` + `layer.maskAllObjects` +
    /// `file.export.layersToFiles` (`trim: true`) + `info` layer bounds.
    pub fn split_parts_sheet_photo(parts_sheet_png: &Path, view_dir: &Path, char_id: &str, view: &str, chroma_hex: Option<&str>) -> Result<Value> {
        if !parts_sheet_png.is_file() {
            return Err(bad(format!("parts sheet does not exist: {}", parts_sheet_png.display())));
        }
        let layers_dir = view_dir.join("layers");
        let source_dir = view_dir.join("source");
        if layers_dir.is_dir() {
            let _ = fs::remove_dir_all(&layers_dir);
        }
        fs::create_dir_all(&layers_dir)?;
        fs::create_dir_all(&source_dir)?;

        let psd = view_dir.join(format!("{char_id}-{view}.psd"));
        let pcraft = view_dir.join(format!("{char_id}-{view}.pcraft"));
        let drawing_name = format!("{char_id}-{view}-drawing.png");
        let drawing_png = view_dir.join(&drawing_name);

        let mut steps: Vec<(&str, Value)> = Vec::new();
        if let Some(hex) = chroma_hex.filter(|s| !s.trim().is_empty()) {
            steps.push(("select.colorRange", json!({"color": hex, "fuzziness": 44, "invert": true})));
            steps.push(("select.refineEdge", json!({"smooth": 2, "feather": 0.8, "contrast": 22, "shiftEdge": -12})));
            steps.push(("layer.layerMask.revealSelection", json!({})));
            steps.push(("layer.rasterize.allLayers", json!({})));
            steps.push(("select.deselect", json!({})));
        } else {
            steps.push(("layer.removeBackground", json!({})));
        }
        steps.push(("layer.maskAllObjects", json!({})));
        steps.push(("file.saveACopy", json!({"path": &psd, "overwrite": true})));
        steps.push(("file.saveACopy", json!({"path": &drawing_png, "overwrite": true})));
        steps.push(("file.export.layersToFiles", json!({"dir": &layers_dir, "format": "png", "prefix": "", "trim": true})));

        Self::run_batch(CraftTarget::Photo, Some(parts_sheet_png), Some(&pcraft), &steps)?;

        let doc = Self::execute(CraftTarget::Photo, Some(&psd), "info", &json!({}))?;
        let w = doc["width"].as_u64().unwrap_or(1024) as u32;
        let h = doc["height"].as_u64().unwrap_or(1024) as u32;
        let slots = craft_scribbles::plush9_part_slots(w, h);
        let doc_layers = doc["layers"].as_array().cloned().unwrap_or_default();

        let mut exported_pngs: Vec<PathBuf> =
            fs::read_dir(&layers_dir)?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"))).collect();
        exported_pngs.sort();

        let mut parts_val = Vec::new();
        for (idx, slot) in slots.iter().enumerate() {
            let Some(file) = exported_pngs.get(idx) else { break };
            let id = slot["id"].as_str().unwrap_or("part");
            let dest_src = source_dir.join(format!("{id}.png"));
            let _ = fs::copy(file, &dest_src);
            let bounds = doc_layers.get(idx).and_then(|l| l.get("bounds")).cloned().unwrap_or_else(|| json!([0, 0, w, h]));
            let [bx, by, bw, bh] = [
                bounds[0].as_u64().unwrap_or(0) as u32,
                bounds[1].as_u64().unwrap_or(0) as u32,
                bounds[2].as_u64().unwrap_or(u64::from(w)) as u32,
                bounds[3].as_u64().unwrap_or(u64::from(h)) as u32,
            ];
            parts_val.push(json!({
                "id": id,
                "parent": slot["parent"],
                "source": dest_src,
                "file": file,
                "bounds": [bx, by, bw, bh],
                "joint": slot["joint"],
                "outline": [[bx, by], [bx + bw, by], [bx + bw, by + bh], [bx, by + bh]],
            }));
        }
        parts_val.sort_by_key(|p| {
            let id = p["id"].as_str().unwrap_or_default();
            craft_scribbles::PLUSH9_Z_ORDER.iter().position(|&z| z == id).unwrap_or(99)
        });

        let parts_doc = json!({
            "image": drawing_name,
            "view": view,
            "size": [w, h],
            "skeleton": "plush-9",
            "animation_style": "cutout_2_5d",
            "status": "isolated by PhotoCraft layer.removeBackground + layer.maskAllObjects",
            "parts": parts_val,
        });
        let parts_json = view_dir.join("parts.json");
        craft_atomic::atomic_write(&parts_json, &serde_json::to_vec_pretty(&parts_doc).unwrap_or_default())?;
        Ok(json!({
            "ok": true,
            "engine": "PhotoCraft",
            "character": char_id,
            "view": view,
            "psd": psd,
            "pcraft": pcraft,
            "parts_json": parts_json,
            "layers_dir": layers_dir,
            "parts_count": parts_val.len(),
            "parts": parts_val,
        }))
    }

    /// Trace a raster image into clean, background-free vector paths or strokes using native `VectorCraft`
    /// (`imageTrace.make` with `ignoreWhite: true`, `method: "overlapping"`, `createStrokes`, + `imageTrace.expand`).
    pub fn trace_subject_vector(input: &Path, out_svg: &Path, preset: Option<&str>, ignore_white: bool, create_strokes: bool) -> Result<Value> {
        if !input.is_file() {
            return Err(bad(format!("input image does not exist: {}", input.display())));
        }
        if let Some(parent) = out_svg.parent() {
            fs::create_dir_all(parent)?;
        }
        let (w, h) = craft_util::media::fast_dimensions(input).map(|(w, h, _)| (w, h)).or_else(|| image::image_dimensions(input).ok()).unwrap_or((1024, 1024));
        let trace_params = json!({
            "preset": preset.unwrap_or(if create_strokes { "lineArt" } else { "default" }),
            "ignoreWhite": ignore_white,
            "method": "overlapping",
            "createStrokes": create_strokes,
            "createFills": !create_strokes,
            "palette": if create_strokes { "limited" } else { "automatic" },
            "paths": 82,
            "corners": 75,
            "noise": 10,
        });
        let steps = [
            ("file.new", json!({"width": w, "height": h})),
            ("file.place", json!({"path": input, "x": 0, "y": 0})),
            ("object.setBounds", json!({"x": 0, "y": 0, "width": w, "height": h})),
            ("imageTrace.make", trace_params),
            ("imageTrace.expand", json!({})),
            ("object.ungroup", json!({})),
        ];
        let results = Self::run_batch(CraftTarget::Vector, None, Some(out_svg), &steps)?;
        Ok(json!({
            "ok": true,
            "engine": "VectorCraft",
            "input": input,
            "svg": out_svg,
            "width": w,
            "height": h,
            "ignore_white": ignore_white,
            "create_strokes": create_strokes,
            "steps": results.len(),
        }))
    }

    /// Export an `EffectCraft` composition timeline (`.fcpxml`, `.xml`, or `.otio`, pre-rendering
    /// comps as ProRes 4444 RGBA automatically) for direct import into `FilmCraft`.
    pub fn export_effect_timeline(project: &Path, comp: &str, out_interchange: &Path) -> Result<Value> {
        if let Some(parent) = out_interchange.parent() {
            fs::create_dir_all(parent)?;
        }
        let steps = [("comp.open", json!({"comp": comp})), ("file.exportTimeline", json!({"path": out_interchange, "comp": comp}))];
        let res = Self::run_batch(CraftTarget::Effect, Some(project), None, &steps)?;
        Ok(json!({
            "ok": true,
            "engine": "EffectCraft",
            "project": project,
            "comp": comp,
            "interchange": out_interchange,
            "result": res.last(),
        }))
    }
}
