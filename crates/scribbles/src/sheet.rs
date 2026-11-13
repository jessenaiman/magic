//! Sprite-sheet cell geometry and `plush-9` part-slot schema metadata (`magic-scribbles`, Layer 4).
//!
//! All raster background removal, chroma keying, subject isolation, edge refinement, and vector
//! silhouette tracing are delegated to `PhotoCraft`, `VectorCraft`, and `EffectCraft` via
//! `native_media` in `crates/engine` using `image` and `craft-atomic`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractedPart {
    pub id: String,
    pub parent: Option<String>,
    pub file: PathBuf,
    pub bounds: [u32; 4],
    pub joint: [f64; 2],
    pub pins: Vec<[f64; 2]>,
    pub outline: Vec<[u32; 2]>,
}

/// Rectangular crop cell `[x, y, width, height]` inside a `cols x rows` sprite sheet of `(w, h)`.
pub fn sprite_cell_rect(width: u32, height: u32, grid: (u32, u32), index: u32) -> [u32; 4] {
    let cols = grid.0.max(1);
    let rows = grid.1.max(1);
    let cw = (width / cols).max(1);
    let ch = (height / rows).max(1);
    let col = index % cols;
    let row = index / cols;
    [col * cw, row * ch, cw, ch]
}

type RigSlot<'a> = (&'a str, Option<&'a str>, [f64; 2], [f64; 2]);

/// Canonical `plush-9` hierarchical rig slots (`id`, `parent`, normalized joint `[u, v]`, pivot `[u, v]`).
pub fn plush9_part_slots(width: u32, height: u32) -> Vec<Value> {
    let (wf, hf) = (f64::from(width.max(1)), f64::from(height.max(1)));
    let scale_pt = |u: f64, v: f64| [(wf * u).round(), (hf * v).round()];
    let slots: &[RigSlot<'_>] = &[
        ("torso", None, [0.50, 0.55], [0.5, 0.5]),
        ("head", Some("torso"), [0.50, 0.43], [0.5, 0.85]),
        ("arm_L", Some("torso"), [0.425, 0.51], [0.7, 0.2]),
        ("arm_R", Some("torso"), [0.575, 0.51], [0.3, 0.2]),
        ("leg_L", Some("torso"), [0.445, 0.65], [0.5, 0.2]),
        ("leg_R", Some("torso"), [0.555, 0.65], [0.5, 0.2]),
        ("tail", Some("torso"), [0.37, 0.57], [0.85, 0.35]),
        ("ear_L", Some("head"), [0.45, 0.25], [0.5, 0.85]),
        ("ear_R", Some("head"), [0.55, 0.25], [0.5, 0.85]),
    ];
    slots
        .iter()
        .map(|(id, parent, [ju, jv], [pu, pv])| {
            json!({
                "id": id,
                "parent": parent,
                "joint": scale_pt(*ju, *jv),
                "pivot_uv": [*pu, *pv],
            })
        })
        .collect()
}

/// Z-ordering from farthest back to frontmost for layered character rigs.
pub const PLUSH9_Z_ORDER: &[&str] = &["tail", "leg_L", "arm_L", "torso", "leg_R", "arm_R", "head", "ear_L", "ear_R"];
