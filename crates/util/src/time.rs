//! Exact rational time base (`254_016_000_000` ticks per second), broadcast frame-rate snapping,
//! and SMPTE / feet+frames timecode helpers re-exported directly from [`craft_timeline::time`]
//! and [`craft_timeline::rate`].

pub use craft_timeline::rate;
pub use craft_timeline::time::{FrameRate, TICKS_PER_SECOND, Tick, TimeDisplay, TimeRange, format_time};
pub use craft_timeline::{
    Format as InterchangeFormat, ale, build, decode_text, detect_text, edl, fcp7, fcpxml, fx, model, otio, path as timeline_path, report, time as timeline_time,
};

use serde_json::{Value, json};

pub fn parse_tick(v: &Value) -> Option<Tick> {
    if let Some(t) = v.get("ticks").and_then(Value::as_i64) {
        return Some(Tick(t));
    }
    if let Some(n) = v.as_i64() {
        if n == 0 || n.unsigned_abs() >= (TICKS_PER_SECOND as u64 / 10_000) {
            return Some(Tick(n));
        }
        return Some(Tick::from_units(n, 1));
    }
    v.as_f64().filter(|f| f.is_finite()).map(Tick::from_seconds_f64)
}

pub fn parse_raw_tick(v: &Value) -> Option<Tick> {
    v.get("ticks").and_then(Value::as_i64).or_else(|| v.as_i64()).map(Tick)
}

pub fn parse_frame_rate(v: &Value) -> Option<FrameRate> {
    if let (Some(num), Some(den)) = (v.get("num").and_then(Value::as_i64), v.get("den").and_then(Value::as_i64)) {
        return (num > 0 && den > 0).then(|| FrameRate::new(num, den));
    }
    if let Some(s) = v.as_str() {
        let trimmed = s.trim().trim_end_matches("fps").trim();
        if let Some((num_s, den_s)) = trimmed.split_once('/')
            && let (Ok(num), Ok(den)) = (num_s.trim().parse::<i64>(), den_s.trim().parse::<i64>())
        {
            return (num > 0 && den > 0).then(|| FrameRate::new(num, den));
        }
        return match trimmed {
            "23.976" | "23.98" => Some(FrameRate::FPS_23_976),
            "24" => Some(FrameRate::FPS_24),
            "25" => Some(FrameRate::FPS_25),
            "29.97" => Some(FrameRate::FPS_29_97),
            "30" => Some(FrameRate::FPS_30),
            "48" => Some(FrameRate::FPS_48),
            "50" => Some(FrameRate::FPS_50),
            "59.94" => Some(FrameRate::FPS_59_94),
            "60" => Some(FrameRate::FPS_60),
            "119.88" => Some(FrameRate::FPS_119_88),
            "120" => Some(FrameRate::FPS_120),
            _ => trimmed.parse::<f64>().ok().filter(|&f| f.is_finite() && f > 0.0).map(rate::rate_from_f64),
        };
    }
    if let Some(n) = v.as_i64() {
        return (n > 0).then(|| FrameRate::new(n, 1));
    }
    v.as_f64().filter(|&f| f.is_finite() && f > 0.0).map(rate::rate_from_f64)
}

/// Snap a positive finite `fps` to an exact broadcast [`FrameRate`], falling back to default.
pub fn rate_from_seconds_fps(fps: f64) -> FrameRate {
    if fps.is_finite() && fps > 0.0 { rate::rate_from_f64(fps) } else { FrameRate::default() }
}

/// Convert `seconds` into an exact [`Tick`], clamping non-finite or negative values to zero.
pub fn tick_from_nonneg_seconds(seconds: f64) -> Tick {
    let clamped = if seconds.is_finite() && seconds > 0.0 { seconds } else { 0.0 };
    Tick::from_seconds_f64(clamped)
}

/// Format `seconds` at `fps` as SMPTE timecode via `craft_timeline::time`.
pub fn format_smpte(seconds: f64, fps: f64) -> String {
    let rate = rate_from_seconds_fps(fps);
    let tick = tick_from_nonneg_seconds(seconds);
    format_time(tick, rate, rate.is_ntsc(), TimeDisplay::Timecode, 48_000)
}

/// Return the zero-based frame index containing `seconds` at `fps`.
pub fn frame_at_seconds(seconds: f64, fps: f64) -> i64 {
    let rate = rate_from_seconds_fps(fps);
    let tick = tick_from_nonneg_seconds(seconds);
    rate.frame_at(tick)
}

/// Step `seconds` by `delta_frames` frames at `fps`, clamping at frame `0`.
pub fn step_frame_seconds(seconds: f64, fps: f64, delta_frames: i64) -> f64 {
    let rate = rate_from_seconds_fps(fps);
    let cur_frame = frame_at_seconds(seconds, fps);
    let next_frame = cur_frame.saturating_add(delta_frames).max(0);
    rate.tick_of(next_frame).seconds()
}

pub fn frame_rate_json(rate: FrameRate) -> Value {
    let r = rate.sane();
    json!({ "num": r.num, "den": r.den })
}
