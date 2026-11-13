//! Dynamic ArtCraft effect & preset discovery (`crates/engine/src/craft_effects.rs`):
//! queries live `--control` APIs (`PhotoCraft :7878`, `VectorCraft :7980`, `EffectCraft :9877`,
//! `FilmCraft :9876`) and user `.ectemplate` preset folders (`EC_PRESETS_DIR`), paired with a
//! compact offline baseline of verified real Craft commands.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::{Value, json};

pub struct MappedEffect {
    pub id: &'static str,
    pub badge: &'static str,
    pub craft_app: &'static str,
    pub craft_command: &'static str,
    pub match_name: &'static str,
    pub category: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    pub default_params: fn() -> Value,
}

/// All 19 EffectCraft effect categories (`effectcraft/README.md`).
pub const EFFECTCRAFT_19_CATEGORIES: &[(&str, &str)] = &[
    ("blur_sharpen", "Blur & Sharpen"),
    ("channel", "Channel"),
    ("color_correction", "Color Correction"),
    ("distort", "Distort"),
    ("expression_controls", "Expression Controls"),
    ("generate", "Generate"),
    ("immersive", "Immersive Video"),
    ("keying", "Keying"),
    ("matte", "Matte"),
    ("noise_grain", "Noise & Grain"),
    ("perspective", "Perspective"),
    ("simulation", "Simulation"),
    ("stylize", "Stylize"),
    ("text", "Text"),
    ("time", "Time"),
    ("transition", "Transition"),
    ("utility", "Utility & OCIO"),
    ("audio", "Audio DSP"),
    ("wasm_plugin", "WASM Plug-ins"),
];

/// All 38 EffectCraft layer blend modes (`effectcraft/README.md`).
pub const EFFECTCRAFT_38_BLEND_MODES: &[&str] = &[
    "normal",
    "dissolve",
    "dancingDissolve",
    "darken",
    "multiply",
    "colorBurn",
    "classicColorBurn",
    "linearBurn",
    "darkerColor",
    "add",
    "lighten",
    "screen",
    "colorDodge",
    "classicColorDodge",
    "linearDodge",
    "lighterColor",
    "overlay",
    "softLight",
    "hardLight",
    "linearLight",
    "vividLight",
    "pinLight",
    "hardMix",
    "difference",
    "classicDifference",
    "exclusion",
    "subtract",
    "divide",
    "hue",
    "saturation",
    "color",
    "luminosity",
    "stencilAlpha",
    "stencilLuma",
    "silhouetteAlpha",
    "silhouetteLuma",
    "alphaAdd",
    "luminescentPremul",
];

/// All 27 FilmCraft GPU linear-light blend modes (`filmcraft/README.md`).
pub const FILMCRAFT_27_BLEND_MODES: &[&str] = &[
    "Normal",
    "Dissolve",
    "Darken",
    "Multiply",
    "Color Burn",
    "Linear Burn",
    "Darker Color",
    "Lighten",
    "Screen",
    "Color Dodge",
    "Linear Dodge (Add)",
    "Lighter Color",
    "Overlay",
    "Soft Light",
    "Hard Light",
    "Vivid Light",
    "Linear Light",
    "Pin Light",
    "Hard Mix",
    "Difference",
    "Exclusion",
    "Subtract",
    "Divide",
    "Hue",
    "Saturation",
    "Color",
    "Luminosity",
];

/// Verified real-command baseline for Kinetic Lyrics (`V3`).
pub const LYRICS_EFFECTS: &[MappedEffect] = &[
    MappedEffect {
        id: "layer.addTextAnimator:opacity",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "layer.addTextAnimator",
        match_name: "opacity",
        category: "Text Animator",
        label: "Range Reveal (Opacity)",
        summary: "EffectCraft range selector revealing characters or words (`layer.addTextAnimator`)",
        default_params: || json!({"properties": ["opacity"]}),
    },
    MappedEffect {
        id: "layer.addTextAnimator:scale",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "layer.addTextAnimator",
        match_name: "scale",
        category: "Text Animator",
        label: "Word Scale Pop",
        summary: "EffectCraft text animator scaling active sung words (`layer.addTextAnimator`)",
        default_params: || json!({"properties": ["scale"]}),
    },
    MappedEffect {
        id: "layer.addTextAnimator:position",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "layer.addTextAnimator",
        match_name: "position",
        category: "Text Animator",
        label: "Word Rise / Drop",
        summary: "EffectCraft text animator offsetting word position (`layer.addTextAnimator`)",
        default_params: || json!({"properties": ["position"]}),
    },
    MappedEffect {
        id: "layer.enablePerChar3D",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "layer.enablePerChar3D",
        match_name: "perChar3D",
        category: "Text Animator",
        label: "Per-Character 3D",
        summary: "Enable EffectCraft 3D per-character glyph transforms (`layer.enablePerChar3D`)",
        default_params: || json!({"enabled": true}),
    },
    MappedEffect {
        id: "pc.layer.style.stroke",
        badge: "PC",
        craft_app: "photocraft",
        craft_command: "layer.style.set",
        match_name: "stroke",
        category: "Layer Style",
        label: "Outer Stroke",
        summary: "PhotoCraft layer-style outline stroke (`:7878` `layer.style.set`)",
        default_params: || json!({"style": "stroke", "size": 6.0, "position": "outside"}),
    },
    MappedEffect {
        id: "pc.layer.style.dropShadow",
        badge: "PC",
        craft_app: "photocraft",
        craft_command: "layer.style.set",
        match_name: "dropShadow",
        category: "Layer Style",
        label: "Drop Shadow",
        summary: "PhotoCraft layer-style separation shadow (`:7878` `layer.style.set`)",
        default_params: || json!({"style": "dropShadow", "distance": 8.0, "size": 12.0}),
    },
    MappedEffect {
        id: "vc.envelope.warp",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "object.envelope.makeWithWarp",
        match_name: "arc",
        category: "Vector Type",
        label: "Envelope Arc Warp",
        summary: "VectorCraft live vector envelope warp (`:7980` `object.envelope.makeWithWarp`)",
        default_params: || json!({"style": "arc", "bend": 22.0}),
    },
    MappedEffect {
        id: "vc.path.offsetPath",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "object.path.offsetPath",
        match_name: "offsetPath",
        category: "Vector Type",
        label: "Vector Offset Contour",
        summary: "VectorCraft round-join offset path behind display type (`:7980` `object.path.offsetPath`)",
        default_params: || json!({"offset": 6.0, "joins": "round"}),
    },
];

/// Verified real-command baseline for Song-Action Motion (`V4`).
pub const MOTION_EFFECTS: &[MappedEffect] = &[
    MappedEffect {
        id: "layer.addShapeItem:trim",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "layer.addShapeItem",
        match_name: "trim",
        category: "Shape & Stroke Reveal",
        label: "Trim Paths",
        summary: "EffectCraft shape-layer `contents/trim/end` 0% -> 100% draw-on (`layer.addShapeItem`)",
        default_params: || json!({"kind": "trim"}),
    },
    MappedEffect {
        id: "vc.trace.line_art",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "imageTrace.make",
        match_name: "lineArt",
        category: "Vector Trace & Brush",
        label: "Ink Line Trace",
        summary: "VectorCraft stroke extraction (`:7980` `imageTrace.make`)",
        default_params: || json!({"preset": "lineArt", "ignoreWhite": true, "strokes": true}),
    },
    MappedEffect {
        id: "vc.trace.silhouette",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "imageTrace.make",
        match_name: "silhouettes",
        category: "Vector Trace & Brush",
        label: "Cutout Silhouette",
        summary: "VectorCraft filled silhouette extraction (`:7980` `imageTrace.make`)",
        default_params: || json!({"preset": "silhouettes", "ignoreWhite": true, "strokes": false}),
    },
    MappedEffect {
        id: "vc.effect.roughen",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "effect.apply",
        match_name: "roughen",
        category: "Vector Distort",
        label: "Roughen Sketch",
        summary: "VectorCraft hand-inked edge jitter on SVG paths (`:7980` `effect.apply`)",
        default_params: || json!({"size": 2.5, "detail": 14.0, "points": "smooth"}),
    },
    MappedEffect {
        id: "pc.action.play",
        badge: "PC",
        craft_app: "photocraft",
        craft_command: "action.play",
        match_name: "default",
        category: "Photo Actions",
        label: "PhotoCraft Action",
        summary: "Run a saved PhotoCraft `.atn` / macro action (`:7878` `action.play`)",
        default_params: || json!({}),
    },
];

fn installed_apps_status() -> &'static Value {
    static STATUS: OnceLock<Value> = OnceLock::new();
    STATUS.get_or_init(|| {
        json!({
            "effectcraft": crate::workspace::resolve_executable_path("effectcraft-cli").is_some()
                || crate::workspace::resolve_executable_path("effectcraft").is_some(),
            "vectorcraft": crate::workspace::resolve_executable_path("vectorcraft-cli").is_some()
                || crate::workspace::resolve_executable_path("vectorcraft").is_some(),
            "photocraft": crate::workspace::resolve_executable_path("photocraft-cli").is_some()
                || crate::workspace::resolve_executable_path("photocraft").is_some(),
            "deckcraft": crate::workspace::resolve_executable_path("deckcraft-cli").is_some()
                || crate::workspace::resolve_executable_path("deckcraft").is_some(),
            "filmcraft": crate::workspace::resolve_executable_path("filmcraft-cli").is_some()
                || crate::workspace::resolve_executable_path("filmcraft").is_some(),
        })
    })
}

fn ec_preset_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(raw) = std::env::var_os("EC_PRESETS_DIR").filter(|v| !v.is_empty()) {
        dirs.extend(std::env::split_paths(&raw));
    }
    if let Some(data) = crate::workspace::magic_data_dir().and_then(|p| p.parent().map(Path::to_path_buf)) {
        dirs.push(data.join("effectcraft/presets"));
    }
    dirs
}

fn discover_ectemplate_presets() -> Vec<Value> {
    let mut out = Vec::new();
    for dir in ec_preset_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !crate::workspace::has_extension(&path, &["ectemplate", "json"]) {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            out.push(json!({
                "id": format!("preset:{stem}"),
                "badge": "EC",
                "craft_app": "effectcraft",
                "craft_command": "layer.applyTextPreset",
                "match_name": stem,
                "category": "EffectCraft Preset",
                "label": stem.replace(['-', '_'], " "),
                "summary": format!("User EffectCraft preset ({})", path.display()),
                "params": {"preset": path},
                "cli_available": true,
                "live_api": false,
            }));
        }
    }
    out
}

fn filter_effects_value_list(items: Vec<Value>, app_filter: Option<&str>, cat_filter: Option<&str>, text_filter: Option<&str>) -> Vec<Value> {
    let q = text_filter.unwrap_or_default().trim().to_ascii_lowercase();
    items
        .into_iter()
        .filter(|e| {
            app_filter.is_none_or(|a| {
                e["craft_app"].as_str().is_some_and(|ca| ca.eq_ignore_ascii_case(a)) || e["badge"].as_str().is_some_and(|b| b.eq_ignore_ascii_case(a))
            })
        })
        .filter(|e| cat_filter.is_none_or(|c| c.eq_ignore_ascii_case("all") || e["category"].as_str().is_some_and(|cat| cat.eq_ignore_ascii_case(c))))
        .filter(|e| {
            q.is_empty()
                || e["id"].as_str().is_some_and(|s| s.to_ascii_lowercase().contains(&q))
                || e["label"].as_str().is_some_and(|s| s.to_ascii_lowercase().contains(&q))
                || e["summary"].as_str().is_some_and(|s| s.to_ascii_lowercase().contains(&q))
                || e["category"].as_str().is_some_and(|s| s.to_ascii_lowercase().contains(&q))
        })
        .collect()
}

fn serialize_table(table: &[MappedEffect], live_extras: &[&Option<Vec<Value>>]) -> Vec<Value> {
    let status = installed_apps_status();
    let mut seen = HashSet::new();
    let mut out: Vec<Value> = Vec::new();

    for live in live_extras.iter().copied().flatten() {
        for item in live {
            if let Some(id) = item["id"].as_str()
                && seen.insert(id.to_owned())
            {
                out.push(item.clone());
            }
        }
    }
    for preset in discover_ectemplate_presets() {
        if let Some(id) = preset["id"].as_str()
            && seen.insert(id.to_owned())
        {
            out.push(preset);
        }
    }
    for e in table {
        if !seen.insert(e.id.to_owned()) {
            continue;
        }
        out.push(json!({
            "id": e.id,
            "badge": e.badge,
            "craft_app": e.craft_app,
            "craft_command": e.craft_command,
            "match_name": e.match_name,
            "category": e.category,
            "label": e.label,
            "summary": e.summary,
            "params": (e.default_params)(),
            "cli_available": status.get(e.craft_app).and_then(Value::as_bool).unwrap_or(false),
            "live_api": false,
        }));
    }
    out
}

pub fn lyrics_catalog_json() -> Vec<Value> {
    let (_, ec_live, vc_pc_live, _) = crate::craft_api::live_craft_snapshot(false);
    serialize_table(LYRICS_EFFECTS, &[&ec_live, &vc_pc_live])
}

pub fn motion_catalog_json() -> Vec<Value> {
    let (_, ec_live, vc_pc_live, _) = crate::craft_api::live_craft_snapshot(false);
    serialize_table(MOTION_EFFECTS, &[&vc_pc_live, &ec_live])
}

pub fn find_lyrics_effect(id: &str) -> Option<&'static MappedEffect> {
    LYRICS_EFFECTS.iter().find(|e| e.id == id || e.match_name == id)
}

pub fn find_motion_effect(id: &str) -> Option<&'static MappedEffect> {
    MOTION_EFFECTS.iter().find(|e| e.id == id || e.match_name == id)
}

pub fn query_lyrics_effects(params: &Value) -> Value {
    let force_refresh = params.get("refresh").and_then(Value::as_bool).unwrap_or(false);
    let (endpoints, ec_live, vc_pc_live, _) = crate::craft_api::live_craft_snapshot(force_refresh);
    let effects = filter_effects_value_list(
        serialize_table(LYRICS_EFFECTS, &[&ec_live, &vc_pc_live]),
        params.get("app").and_then(Value::as_str),
        params.get("category").and_then(Value::as_str),
        params.get("filter").and_then(Value::as_str),
    );
    let live_connected =
        endpoints["effectcraft"]["connected"] == true || endpoints["vectorcraft"]["connected"] == true || endpoints["photocraft"]["connected"] == true;
    json!({
        "ok": true,
        "domain": "lyrics",
        "track": "V3 (Kinetic Lyrics)",
        "sources": ["effectcraft", "photocraft", "vectorcraft"],
        "api_connected": live_connected,
        "api_endpoints": endpoints,
        "fonts": crate::craft_api::discover_suite_fonts(),
        "installed": installed_apps_status().clone(),
        "count": effects.len(),
        "effects": effects,
    })
}

pub fn query_motion_effects(params: &Value) -> Value {
    let force_refresh = params.get("refresh").and_then(Value::as_bool).unwrap_or(false);
    let (endpoints, ec_live, vc_pc_live, _) = crate::craft_api::live_craft_snapshot(force_refresh);
    let effects = filter_effects_value_list(
        serialize_table(MOTION_EFFECTS, &[&vc_pc_live, &ec_live]),
        params.get("app").and_then(Value::as_str),
        params.get("category").and_then(Value::as_str),
        params.get("filter").and_then(Value::as_str),
    );
    let categories_19: Vec<Value> = EFFECTCRAFT_19_CATEGORIES.iter().map(|(id, lbl)| json!({"id": id, "label": lbl})).collect();
    let live_connected =
        endpoints["vectorcraft"]["connected"] == true || endpoints["effectcraft"]["connected"] == true || endpoints["photocraft"]["connected"] == true;
    json!({
        "ok": true,
        "domain": "motion",
        "track": "V4 (Action Motion)",
        "sources": ["vectorcraft", "effectcraft", "photocraft"],
        "api_connected": live_connected,
        "api_endpoints": endpoints,
        "effectcraft_categories": categories_19,
        "blend_modes": EFFECTCRAFT_38_BLEND_MODES,
        "filmcraft_blend_modes": FILMCRAFT_27_BLEND_MODES,
        "installed": installed_apps_status().clone(),
        "count": effects.len(),
        "effects": effects,
    })
}
