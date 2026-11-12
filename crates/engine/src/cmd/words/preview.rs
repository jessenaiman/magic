//! Saved kinetic-lyric preview extraction built on `storytold/craft-libs` (`craft-doc`) without subprocesses.

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub fn frame(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = p["kind"].as_str().unwrap_or("lyrics");
    if kind == "motion" {
        return crate::cmd::motion::cmd_motion_preview_frame(s, p);
    }
    let time = p["time"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| EngineError::InvalidParams("time must be nonnegative seconds".into()))?;
    if !matches!(kind, "lyrics" | "stage") {
        return Err(EngineError::InvalidParams("kind must be lyrics, motion, or stage".into()));
    }
    let dir = super::workspace_dir(s)?;
    let project = dir.join("lettering/lyrics.ecproj");
    if !project.is_file() {
        let _ = super::lettering::cmd_words_letter(s, &json!({"render": false}));
    }
    let folder = dir.join("lettering/.preview");
    std::fs::create_dir_all(&folder)?;
    let out = folder.join(format!("{kind}-{}.png", (time * 1000.0).round() as u64));
    if out.is_file() {
        return Ok(json!({"frame": out, "source": project, "kind": kind, "time": time}));
    }
    let scale = p["scale"].as_f64().unwrap_or(0.67).clamp(0.25, 1.0);
    let max_side = (1920.0 * scale).round() as u32;
    if project.is_file() && crate::native_media::extract_or_copy_preview(&project, &out, max_side)? {
        return Ok(json!({"frame": out, "source": project, "kind": kind, "time": time}));
    }
    Ok(json!({"empty": true, "kind": kind, "time": time, "source": project}))
}
