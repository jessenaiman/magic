//! Craft design effect catalogs & live `--control` API discovery for Kinetic Lyrics
//! (`words.effects`, V3), Song-Action Motion (`motion.effects`, V4), EffectCraft's 19-category
//! 308-effect catalog (`effectcraft/docs/effects.md`, `T29`), and FilmCraft's 27 GPU linear-light
//! blend modes (`filmcraft/README.md`, `T38`).
//!
//! When a user's local Craft desktop app is running with `--control` (`EffectCraft :9877`,
//! `VectorCraft :7979`, `FilmCraft :9876`), `query_lyrics_effects` and `query_motion_effects`
//! query the live API over `127.0.0.1` (`crate::craft_api`) and merge live effects with the
//! documented Craft effect catalog — requiring zero `Cargo.toml` sibling app dependencies.

use std::collections::HashSet;
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

/// All 19 EffectCraft effect categories (`effectcraft/docs/effects.md`, `T29`).
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

/// All 38 EffectCraft layer blend modes (`T29`).
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

/// All 27 FilmCraft GPU linear-light blend modes (`T38`).
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

pub const LYRICS_EFFECTS: &[MappedEffect] = &[
    MappedEffect {
        id: "ec.text.typewriter_range",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "text.addAnimator",
        match_name: "ADBE Text Animator",
        category: "Text Animator",
        label: "Typewriter Range",
        summary: "Range selector reveals characters/words synced to vocal syllables",
        default_params: || json!({"selector": "range", "start": 0.0, "end": 100.0, "opacity": 0.0}),
    },
    MappedEffect {
        id: "ec.text.per_word_pop",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "text.addAnimator",
        match_name: "ADBE Text Scale 3D",
        category: "Text Animator",
        label: "Per-Word Scale Pop",
        summary: "Overshoots active sung word to 132% with eased range falloff",
        default_params: || json!({"basedOn": "words", "scale": [132.0, 132.0], "easeHigh": 65.0}),
    },
    MappedEffect {
        id: "ec.text.per_char_3d_flip",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "text.addAnimator",
        match_name: "ADBE Text Rotation X",
        category: "Text Animator",
        label: "3D Character Flip",
        summary: "Enables per-character 3D layer axis and swings glyphs forward",
        default_params: || json!({"perCharacter3d": true, "rotationX": -80.0, "tracking": 6.0}),
    },
    MappedEffect {
        id: "ec.stylize.glow",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.stylize.glow",
        category: "Typography Stylize",
        label: "Glow",
        summary: "32-bpc GPU halo glow matched to EffectCraft Stylize > Glow",
        default_params: || json!({"threshold": 60.0, "radius": 14.0, "intensity": 1.1}),
    },
    MappedEffect {
        id: "ec.generate.cclightsweep",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.generate.cclightsweep",
        category: "Typography Stylize",
        label: "CC Light Sweep",
        summary: "Animated specular shimmer sweeping across chorus hero words",
        default_params: || json!({"direction": 28.0, "width": 42.0, "sweepIntensity": 38.0}),
    },
    MappedEffect {
        id: "ec.stylize.roughenedges",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.stylize.roughenedges",
        category: "Typography Stylize",
        label: "Roughen Edges",
        summary: "Hand-cut storybook ink edge texture on bold display type",
        default_params: || json!({"border": 4.5, "edgeSharpness": 1.8, "scale": 65.0}),
    },
    MappedEffect {
        id: "ec.perspective.dropshadow",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.perspective.dropshadow",
        category: "Perspective & 3D",
        label: "Drop Shadow",
        summary: "Soft stage separation shadow behind bright storybook lettering",
        default_params: || json!({"opacity": 75.0, "direction": 135.0, "distance": 8.0, "softness": 12.0}),
    },
    MappedEffect {
        id: "ec.perspective.bevelalpha",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.perspective.bevelalpha",
        category: "Perspective & 3D",
        label: "Bevel Alpha",
        summary: "Chiseled rim highlight along glyph alpha edges",
        default_params: || json!({"edgeThickness": 2.4, "lightAngle": -45.0, "lightIntensity": 0.45}),
    },
    MappedEffect {
        id: "ec.generate.audiospectrum",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.generate.audiospectrum",
        category: "Audio Reactive",
        label: "Audio Spectrum",
        summary: "EffectCraft FFT frequency bars driven directly by the song layer",
        default_params: || json!({"bands": 48, "maxHeight": 120.0, "softness": 20.0}),
    },
    MappedEffect {
        id: "ec.generate.audiowaveform",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.generate.audiowaveform",
        category: "Audio Reactive",
        label: "Audio Waveform Ribbon",
        summary: "EffectCraft live vocal waveform ribbon tethered beneath active lyrics",
        default_params: || json!({"samples": 256, "amplitude": 64.0, "thickness": 4.0}),
    },
    MappedEffect {
        id: "ec.distort.bezierwarp",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.distort.bezierwarp",
        category: "Envelope Warp",
        label: "Bezier Ribbon Warp",
        summary: "12-tangent cubic mesh bending lyric phrases along stage arcs",
        default_params: || json!({"quality": 6, "bend": 18.0}),
    },
    MappedEffect {
        id: "ec.distort.wavewarp",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.distort.wavewarp",
        category: "Envelope Warp",
        label: "Wave Warp",
        summary: "Sinusoidal lyric wave synced to tempo for melodic choruses",
        default_params: || json!({"waveHeight": 10.0, "waveWidth": 72.0, "waveSpeed": 1.2}),
    },
    MappedEffect {
        id: "ec.distort.turbulentdisplace",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.distort.turbulentdisplace",
        category: "Envelope Warp",
        label: "Turbulent Boil",
        summary: "Animated hand-lettered cel boil across expressive words",
        default_params: || json!({"amount": 11.0, "size": 26.0, "complexity": 2.0}),
    },
    MappedEffect {
        id: "ec.sim.ccstarburst",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.sim.ccstarburst",
        category: "Simulation & Time",
        label: "CC Star Burst",
        summary: "Particle star burst accent on downbeat hero words",
        default_params: || json!({"scatter": 32.0, "speed": 0.65, "gridSpacing": 6.0}),
    },
    MappedEffect {
        id: "ec.time.echo",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.time.echo",
        category: "Simulation & Time",
        label: "Motion Echo Trail",
        summary: "Temporal ghost trail behind fast kinetic word leaps",
        default_params: || json!({"echoTime": -0.033, "numberOfEchoes": 4, "decay": 0.68}),
    },
    MappedEffect {
        id: "vc.envelope.arc",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "object.envelope.makeWithWarp",
        match_name: "warp.arc",
        category: "Envelope Warp",
        label: "Envelope Arc",
        summary: "VectorCraft live envelope arc warp preserving crisp cubic outlines",
        default_params: || json!({"style": "arc", "bend": 22.0, "horizontalDistortion": 0.0, "verticalDistortion": 0.0}),
    },
    MappedEffect {
        id: "vc.path.offset_halo",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "object.path.offsetPath",
        match_name: "offsetPath",
        category: "Layer Style",
        label: "Vector Offset Sticker",
        summary: "VectorCraft round-join offset contour behind display type",
        default_params: || json!({"offset": 6.0, "joins": "round", "miterLimit": 4.0}),
    },
    MappedEffect {
        id: "pc.layerStyle.stroke_halo",
        badge: "PC",
        craft_app: "photocraft",
        craft_command: "layerStyle",
        match_name: "stroke",
        category: "Layer Style",
        label: "Outer Stroke Halo",
        summary: "PhotoCraft outside layer-style stroke for high-contrast read clarity",
        default_params: || json!({"size": 6.0, "position": "outside", "opacity": 100.0}),
    },
    MappedEffect {
        id: "pc.filter.paper_grain",
        badge: "PC",
        craft_app: "photocraft",
        craft_command: "filter.noise.addNoise",
        match_name: "addNoise",
        category: "Layer Style",
        label: "Paper Grain Press",
        summary: "PhotoCraft warm monochromatic letterpress grain on title cards",
        default_params: || json!({"amount": 4.5, "distribution": "gaussian", "monochromatic": true}),
    },
];

pub const MOTION_EFFECTS: &[MappedEffect] = &[
    MappedEffect {
        id: "vc.trace.line_art",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "imageTrace.make",
        match_name: "lineArt",
        category: "Vector Trace & Brush",
        label: "Ink Line Trace",
        summary: "VectorCraft background-free stroke extraction (`ignoreWhite: true`)",
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
        summary: "VectorCraft clean filled action silhouette without paper backdrop",
        default_params: || json!({"preset": "silhouettes", "ignoreWhite": true, "strokes": false}),
    },
    MappedEffect {
        id: "vc.brush.chalk_charcoal",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "brush.apply",
        match_name: "chalkCharcoal",
        category: "Vector Trace & Brush",
        label: "Chalk Art Brush",
        summary: "VectorCraft dry storybook chalk stroke along action splines",
        default_params: || json!({"library": "artistic_chalk", "width": 8.0}),
    },
    MappedEffect {
        id: "vc.stroke.tapered_quill",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "stroke.widthProfile.set",
        match_name: "widthProfile1",
        category: "Vector Trace & Brush",
        label: "Tapered Quill Profile",
        summary: "VectorCraft variable-width stroke tapering from tail to tip",
        default_params: || json!({"profile": "widthProfile1", "taper": 65.0}),
    },
    MappedEffect {
        id: "vc.effect.roughen",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "effect.apply",
        match_name: "roughen",
        category: "Vector Distort",
        label: "Roughen Sketch",
        summary: "VectorCraft organic hand-inked edge jitter on clean SVG paths",
        default_params: || json!({"size": 2.5, "detail": 14.0, "points": "smooth"}),
    },
    MappedEffect {
        id: "vc.effect.zig_zag",
        badge: "VC",
        craft_app: "vectorcraft",
        craft_command: "effect.apply",
        match_name: "zigZag",
        category: "Vector Distort",
        label: "Zig-Zag Wave",
        summary: "VectorCraft smooth sinusoidal ripples on water and wind cues",
        default_params: || json!({"size": 6.0, "ridgesPerSegment": 5, "points": "smooth"}),
    },
    MappedEffect {
        id: "ec.shape.trim_paths",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "layer.addShapeItem",
        match_name: "trim",
        category: "Shape & Stroke Reveal",
        label: "Trim Paths",
        summary: "EffectCraft shape-layer `contents/trim/end` 0% -> 100% draw-on",
        default_params: || json!({"kind": "trim", "start": 0.0, "end": 100.0}),
    },
    MappedEffect {
        id: "ec.generate.writeon",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.generate.writeon",
        category: "Shape & Stroke Reveal",
        label: "Write-On Reveal",
        summary: "EffectCraft animated brush nib revealing custom motion trails",
        default_params: || json!({"brushSize": 9.0, "hardness": 85.0, "strokeLength": 1.2}),
    },
    MappedEffect {
        id: "ec.generate.scribble",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.generate.scribble",
        category: "Shape & Stroke Reveal",
        label: "Hand Scribble Fill",
        summary: "EffectCraft animated cross-hatch scribble inside closed vector cues",
        default_params: || json!({"angle": 35.0, "strokeWidth": 3.5, "wigglesPerSecond": 6.0}),
    },
    MappedEffect {
        id: "ec.distort.turbulentdisplace",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.distort.turbulentdisplace",
        category: "Motion Boil & Trail",
        label: "Turbulent Line Boil",
        summary: "EffectCraft animated cel-animation line boil on drawn props",
        default_params: || json!({"amount": 12.0, "size": 28.0, "complexity": 2.0}),
    },
    MappedEffect {
        id: "ec.distort.wavewarp",
        badge: "EC",
        craft_app: "effectcraft",
        craft_command: "effect.apply",
        match_name: "ec.distort.wavewarp",
        category: "Motion Boil & Trail",
        label: "Swimming Wave Warp",
        summary: "EffectCraft continuous phase-traveling wave across action drawings",
        default_params: || json!({"waveHeight": 8.0, "waveWidth": 64.0, "waveSpeed": 1.4}),
    },
    MappedEffect {
        id: "pc.filter.motion_trail",
        badge: "PC",
        craft_app: "photocraft",
        craft_command: "filter.blur.motionBlur",
        match_name: "motionBlur",
        category: "Motion Boil & Trail",
        label: "Motion Trail Stamp",
        summary: "PhotoCraft directional velocity smear behind fast-moving stamps",
        default_params: || json!({"angle": 18.0, "distance": 16.0}),
    },
];

fn installed_apps_status() -> &'static Value {
    static STATUS: OnceLock<Value> = OnceLock::new();
    STATUS.get_or_init(|| {
        json!({
            "effectcraft": crate::workspace::resolve_executable_path("effectcraft").is_some(),
            "vectorcraft": crate::workspace::resolve_executable_path("vectorcraft").is_some(),
            "photocraft": crate::workspace::resolve_executable_path("photocraft").is_some(),
            "designcraft": crate::workspace::resolve_executable_path("designcraft").is_some(),
            "filmcraft": crate::workspace::resolve_executable_path("filmcraft").is_some(),
        })
    })
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

    // Merge live effects queried directly from open `--control` Craft apps first, then fill catalog items.
    for live in live_extras.iter().copied().flatten() {
        for item in live {
            if let Some(id) = item["id"].as_str()
                && seen.insert(id.to_owned())
            {
                out.push(item.clone());
            }
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
    let (_, ec_live, vc_live, _) = crate::craft_api::live_craft_snapshot(false);
    serialize_table(LYRICS_EFFECTS, &[&ec_live, &vc_live])
}

pub fn motion_catalog_json() -> Vec<Value> {
    let (_, ec_live, vc_live, _) = crate::craft_api::live_craft_snapshot(false);
    serialize_table(MOTION_EFFECTS, &[&vc_live, &ec_live])
}

pub fn find_lyrics_effect(id: &str) -> Option<&'static MappedEffect> {
    LYRICS_EFFECTS.iter().find(|e| e.id == id || e.match_name == id)
}

pub fn find_motion_effect(id: &str) -> Option<&'static MappedEffect> {
    MOTION_EFFECTS.iter().find(|e| e.id == id || e.match_name == id)
}

pub fn query_lyrics_effects(params: &Value) -> Value {
    let force_refresh = params.get("refresh").and_then(Value::as_bool).unwrap_or(false);
    let (endpoints, ec_live, vc_live, _) = crate::craft_api::live_craft_snapshot(force_refresh);
    let app = params.get("app").and_then(Value::as_str);
    let category = params.get("category").and_then(Value::as_str);
    let filter = params.get("filter").and_then(Value::as_str);
    let merged = serialize_table(LYRICS_EFFECTS, &[&ec_live, &vc_live]);
    let effects = filter_effects_value_list(merged, app, category, filter);
    let live_connected = endpoints["effectcraft"]["connected"] == true || endpoints["vectorcraft"]["connected"] == true;
    json!({
        "ok": true,
        "domain": "lyrics",
        "track": "V3 (Kinetic Lyrics)",
        "sources": ["effectcraft", "vectorcraft", "photocraft"],
        "api_connected": live_connected,
        "api_endpoints": endpoints,
        "installed": installed_apps_status().clone(),
        "count": effects.len(),
        "effects": effects,
    })
}

pub fn query_motion_effects(params: &Value) -> Value {
    let force_refresh = params.get("refresh").and_then(Value::as_bool).unwrap_or(false);
    let (endpoints, ec_live, vc_live, _) = crate::craft_api::live_craft_snapshot(force_refresh);
    let app = params.get("app").and_then(Value::as_str);
    let category = params.get("category").and_then(Value::as_str);
    let filter = params.get("filter").and_then(Value::as_str);
    let merged = serialize_table(MOTION_EFFECTS, &[&vc_live, &ec_live]);
    let effects = filter_effects_value_list(merged, app, category, filter);
    let categories_19: Vec<Value> = EFFECTCRAFT_19_CATEGORIES.iter().map(|(id, lbl)| json!({"id": id, "label": lbl})).collect();
    let live_connected = endpoints["vectorcraft"]["connected"] == true || endpoints["effectcraft"]["connected"] == true;
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
