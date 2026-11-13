//! Pure easing, transform property, and nested symbol loop primitives.

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    Classic(f32),
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    pub const PRESETS: &'static [(&'static str, Self)] = &[
        ("Linear", Self::Linear),
        ("Quad In", Self::QuadIn),
        ("Quad Out", Self::QuadOut),
        ("Quad In Out", Self::QuadInOut),
        ("Cubic In", Self::CubicIn),
        ("Cubic Out", Self::CubicOut),
        ("Cubic In Out", Self::CubicInOut),
        ("Smooth", Self::CubicBezier(0.25, 0.1, 0.25, 1.0)),
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::QuadIn => "quad_in",
            Self::QuadOut => "quad_out",
            Self::QuadInOut => "quad_in_out",
            Self::CubicIn => "cubic_in",
            Self::CubicOut => "cubic_out",
            Self::CubicInOut => "cubic_in_out",
            Self::Classic(_) => "classic",
            Self::CubicBezier(_, _, _, _) => "cubic_bezier",
        }
    }

    pub fn as_bezier(&self) -> (f32, f32, f32, f32) {
        match *self {
            Self::Linear => (0.0, 0.0, 1.0, 1.0),
            Self::QuadIn => (0.55, 0.085, 0.68, 0.53),
            Self::QuadOut => (0.25, 0.46, 0.45, 0.94),
            Self::QuadInOut => (0.455, 0.03, 0.515, 0.955),
            Self::CubicIn => (0.55, 0.055, 0.675, 0.19),
            Self::CubicOut => (0.215, 0.61, 0.355, 1.0),
            Self::CubicInOut => (0.645, 0.045, 0.355, 1.0),
            Self::Classic(strength) => {
                let s = (strength / 100.0).clamp(-1.0, 1.0);
                if s >= 0.0 {
                    (0.33 * (1.0 - s), 0.33 + 0.67 * s, 0.67 - 0.32 * s, 1.0)
                } else {
                    let a = -s;
                    (0.33 + 0.32 * a, 0.0, 0.67 + 0.33 * a, 0.67 * (1.0 - a))
                }
            }
            Self::CubicBezier(x1, y1, x2, y2) => (x1.clamp(0.0, 1.0), y1, x2.clamp(0.0, 1.0), y2),
        }
    }

    pub fn apply(&self, t: f32) -> f32 {
        let u = t.clamp(0.0, 1.0);
        match *self {
            Self::Linear => u,
            Self::QuadIn => u * u,
            Self::QuadOut => 1.0 - (1.0 - u) * (1.0 - u),
            Self::QuadInOut => {
                if u < 0.5 {
                    2.0 * u * u
                } else {
                    1.0 - (-2.0 * u + 2.0).powi(2) * 0.5
                }
            }
            Self::CubicIn => u * u * u,
            Self::CubicOut => 1.0 - (1.0 - u).powi(3),
            Self::CubicInOut => {
                if u < 0.5 {
                    4.0 * u * u * u
                } else {
                    1.0 - (-2.0 * u + 2.0).powi(3) * 0.5
                }
            }
            Self::Classic(strength) => {
                let s = (strength / 100.0).clamp(-1.0, 1.0);
                u * (1.0 + s * (1.0 - u))
            }
            Self::CubicBezier(x1, y1, x2, y2) => solve_cubic_bezier(x1.clamp(0.0, 1.0), y1, x2.clamp(0.0, 1.0), y2, u),
        }
    }
}

fn bezier_coord(p1: f32, p2: f32, s: f32) -> f32 {
    let inv = 1.0 - s;
    3.0 * inv * inv * s * p1 + 3.0 * inv * s * s * p2 + s * s * s
}

fn solve_cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let (mut lo, mut hi, mut s) = (0.0_f32, 1.0_f32, x);
    for _ in 0..16 {
        let err = bezier_coord(x1, x2, s) - x;
        if err.abs() < 1e-5 {
            break;
        }
        if err < 0.0 {
            lo = s
        } else {
            hi = s
        };
        s = 0.5 * (lo + hi);
    }
    bezier_coord(y1, y2, s)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    #[default]
    Loop,
    PlayOnce,
    SingleFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Property {
    X,
    Y,
    ScaleX,
    ScaleY,
    Rotation,
    Skew,
    Alpha,
    Tint,
}

impl Property {
    pub const ALL: [Self; 8] = [Self::X, Self::Y, Self::ScaleX, Self::ScaleY, Self::Rotation, Self::Skew, Self::Alpha, Self::Tint];

    pub const fn key(&self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::ScaleX => "scale_x",
            Self::ScaleY => "scale_y",
            Self::Rotation => "rotation",
            Self::Skew => "skew",
            Self::Alpha => "alpha",
            Self::Tint => "tint",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::X => "Position X",
            Self::Y => "Position Y",
            Self::ScaleX => "Scale X",
            Self::ScaleY => "Scale Y",
            Self::Rotation => "Rotation",
            Self::Skew => "Skew",
            Self::Alpha => "Opacity",
            Self::Tint => "Tint",
        }
    }

    pub const fn unit(&self) -> &'static str {
        match self {
            Self::X | Self::Y => "px",
            Self::ScaleX | Self::ScaleY | Self::Alpha | Self::Tint => "%",
            Self::Rotation | Self::Skew => "deg",
        }
    }

    pub const fn group(&self) -> &'static str {
        match self {
            Self::X | Self::Y | Self::ScaleX | Self::ScaleY | Self::Rotation | Self::Skew => "transform",
            Self::Alpha | Self::Tint => "color",
        }
    }

    pub const fn range(&self) -> RangeInclusive<f32> {
        match self {
            Self::X => -1920.0..=1920.0,
            Self::Y => -1080.0..=1080.0,
            Self::ScaleX | Self::ScaleY => 1.0..=400.0,
            Self::Rotation => -14400.0..=14400.0,
            Self::Skew => -85.0..=85.0,
            Self::Alpha | Self::Tint => 0.0..=100.0,
        }
    }
}

pub fn parse_easing(raw: &Value) -> Easing {
    if let Some(arr) = raw.as_array()
        && arr.len() == 4
        && let (Some(x1), Some(y1), Some(x2), Some(y2)) = (arr[0].as_f64(), arr[1].as_f64(), arr[2].as_f64(), arr[3].as_f64())
    {
        return Easing::CubicBezier(x1 as f32, y1 as f32, x2 as f32, y2 as f32);
    }
    if let Some(strength) = raw.get("classic").and_then(Value::as_f64) {
        return Easing::Classic(strength as f32);
    }
    if let Some(bez) = raw.get("bezier").and_then(Value::as_array)
        && bez.len() == 4
        && let (Some(x1), Some(y1), Some(x2), Some(y2)) = (bez[0].as_f64(), bez[1].as_f64(), bez[2].as_f64(), bez[3].as_f64())
    {
        return Easing::CubicBezier(x1 as f32, y1 as f32, x2 as f32, y2 as f32);
    }
    let key = raw.as_str().unwrap_or("cubic_out").trim().to_ascii_lowercase().replace([' ', '-'], "_");
    match key.as_str() {
        "linear" | "step_hold" => Easing::Linear,
        "quad_in" | "ease_in_quad" | "ease_in" => Easing::QuadIn,
        "quad_out" | "ease_out_quad" | "ease_out" => Easing::QuadOut,
        "quad_in_out" | "ease_in_out_quad" | "ease_in_out" => Easing::QuadInOut,
        "cubic_in" | "ease_in_cubic" => Easing::CubicIn,
        "cubic_out" | "ease_out_cubic" | "spring_out" => Easing::CubicOut,
        "cubic_in_out" | "ease_in_out_cubic" | "elastic_pop" => Easing::CubicInOut,
        "smooth" | "smooth_bezier" | "cubic_bezier" => Easing::CubicBezier(0.25, 0.1, 0.25, 1.0),
        _ => Easing::CubicOut,
    }
}

pub fn parse_loop_mode(raw: Option<&str>) -> LoopMode {
    match raw.unwrap_or("loop").trim().to_ascii_lowercase().replace([' ', '-'], "_").as_str() {
        "play_once" | "once" | "playonce" => LoopMode::PlayOnce,
        "single_frame" | "singleframe" | "hold" | "still" => LoopMode::SingleFrame,
        _ => LoopMode::Loop,
    }
}

pub fn evaluate_symbol_frame(loop_mode: LoopMode, first_frame: u32, total_frames: u32, elapsed_frames: u32) -> u32 {
    let len = total_frames.max(1);
    let start = first_frame.min(len - 1);
    match loop_mode {
        LoopMode::SingleFrame => start,
        LoopMode::PlayOnce => (start + elapsed_frames).min(len - 1),
        LoopMode::Loop => (start + elapsed_frames) % len,
    }
}

pub fn easing_catalog() -> Value {
    let presets: Vec<Value> = Easing::PRESETS
        .iter()
        .map(|(label, ease)| {
            let id = match ease {
                Easing::Linear => "linear",
                Easing::QuadIn => "quad_in",
                Easing::QuadOut => "quad_out",
                Easing::QuadInOut => "quad_in_out",
                Easing::CubicIn => "cubic_in",
                Easing::CubicOut => "cubic_out",
                Easing::CubicInOut => "cubic_in_out",
                Easing::Classic(_) => "classic",
                Easing::CubicBezier(_, _, _, _) => "smooth",
            };
            let (x1, y1, x2, y2) = ease.as_bezier();
            let samples = [0.0_f32, 0.25, 0.5, 0.75, 1.0].map(|t| ease.apply(t));
            json!({ "id": id, "name": label, "does": ease.label(), "bezier": [x1, y1, x2, y2], "samples": samples })
        })
        .collect();
    json!(presets)
}

pub fn property_catalog() -> Value {
    let props: Vec<Value> = Property::ALL
        .iter()
        .map(|p| {
            let r = p.range();
            json!({ "key": p.key(), "label": p.label(), "unit": p.unit(), "group": p.group(), "min": *r.start(), "max": *r.end() })
        })
        .collect();
    json!(props)
}

pub fn sample_eased_curve(from: &[f64], to: &[f64], delay: f64, duration: f64, easing: Easing, subdivisions: usize) -> Vec<(f64, Vec<f64>)> {
    let mut out = Vec::new();
    if delay > 0.0 {
        out.push((0.0, from.to_vec()));
    }
    let count = if matches!(easing, Easing::Linear) { 1 } else { subdivisions.clamp(2, 16) };
    for step in 0..=count {
        let t = (step as f32) / (count as f32);
        let k = f64::from(easing.apply(t));
        let time = delay + duration * f64::from(t);
        let vals: Vec<f64> = from.iter().zip(to.iter()).map(|(&a, &b)| a + (b - a) * k).collect();
        out.push((time, vals));
    }
    out
}
