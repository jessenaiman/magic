//! Stage-to-`FilmCraft` (`film/<song>.fcproj`) sequence hydration (`A1`, `V1`–`V4`, `C1`) and
//! composite Program Monitor frame rasterization (`render_composite_frame`), powered by
//! [`craft_captions`] (`SRT`, `WebVTT`, `TTML`, `SCC` CEA-608, `MCC` CEA-708) and [`craft_timeline::time`].

use std::path::{Path, PathBuf};

use craft_captions::Format as CaptionFormat;
use image::{Rgba, RgbaImage};
use magic_util::time::{TICKS_PER_SECOND, Tick};
use serde_json::{Value, json};

use super::fcproj_doc::{load_fcproj, recompute_sequence_duration, sec_to_ticks};
use crate::{Result, Session};

/// Parse project captions (`film/captions.{srt,vtt,ttml,scc,mcc}` or `transcript.srt`) via [`craft_captions`].
pub(crate) fn parse_srt_captions(project_dir: &Path) -> Vec<Value> {
    let candidates = [
        project_dir.join("film/captions.srt"),
        project_dir.join("film/captions.vtt"),
        project_dir.join("film/captions.ttml"),
        project_dir.join("film/captions.scc"),
        project_dir.join("film/captions.mcc"),
        project_dir.join("transcript.srt"),
    ];
    let Some(cap_path) = candidates.into_iter().find(|p| p.is_file()) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&cap_path) else {
        return Vec::new();
    };
    let ext = cap_path.extension().and_then(|e| e.to_str());
    let fmt = craft_captions::detect(&bytes, ext).unwrap_or(CaptionFormat::Srt);
    let Ok(doc) = craft_captions::parse(&bytes, fmt) else {
        return Vec::new();
    };
    doc.cues
        .into_iter()
        .enumerate()
        .filter_map(|(idx, cue)| {
            let txt = cue.text.trim().to_owned();
            if txt.is_empty() {
                return None;
            }
            let cid = 5000 + idx as u64;
            let start = cue.start.0.max(0);
            let end = cue.end.0.max(start + TICKS_PER_SECOND / 4);
            Some(json!({
                "clip": cid,
                "id": cid,
                "name": txt,
                "text": cue.text,
                "start": start,
                "end": end,
                "enabled": true,
            }))
        })
        .collect()
}

#[allow(clippy::cognitive_complexity)]
/// Hydrate empty `.fcproj` tracks strictly from canonical Magic Studio stage documents:
/// `s.song_audio` (`A1`), `storyboard.designcraft` (`V1`), `assets/characters` (`V2`),
/// `design/words.json` (`V3`), `motion/actions.json` (`V4`), and `craft-captions` (`C1`).
pub(crate) fn hydrate_live_stage_tracks(s: &Session, project_dir: &Path, doc: &mut Value) {
    let song_ticks = crate::workshop::song_ticks(project_dir)
        .or_else(|| s.song_audio.as_ref().and_then(|a| super::media_duration_ticks(&project_dir.join(a), false).ok()))
        .unwrap_or_else(|| Tick::from_units(30, 1))
        .0
        .max(sec_to_ticks(12.0));
    let song_sec = (song_ticks as f64) / (TICKS_PER_SECOND as f64);

    if doc["sequence"]["audio"].get(0).and_then(|t| t["items"].as_array()).is_some_and(Vec::is_empty)
        && let Some(audio) = s.song_audio.as_ref().map(|a| project_dir.join(a)).filter(|p| p.is_file())
    {
        let fname = audio.file_name().and_then(|n| n.to_str()).unwrap_or("Song Master");
        if let Some(a1) = doc["sequence"]["audio"].get_mut(0).and_then(|t| t["items"].as_array_mut()) {
            a1.push(json!({
                "clip": 90_u64,
                "id": 90_u64,
                "name": fname,
                "start": 0_i64,
                "end": song_ticks,
                "source_path": audio,
                "enabled": true,
            }));
        }
    }

    if doc["sequence"]["video"].get(0).and_then(|t| t["items"].as_array()).is_some_and(Vec::is_empty) {
        let mut v1_clips = Vec::new();
        let sb_base = project_dir.join("storyboard.designcraft");
        let pages = crate::cmd::storyboard::parse_storyboard_pages(Some(s), &sb_base);
        let scene_pages: Vec<_> = pages.into_iter().filter(|p| p.section.eq_ignore_ascii_case("scenes") && p.page_role != "section_title").collect();
        let count = scene_pages.len().max(1);
        for (idx, pg) in scene_pages.into_iter().enumerate() {
            let st = (idx as f64 / count as f64) * song_sec;
            let en = ((idx + 1) as f64 / count as f64) * song_sec;
            let img = crate::cmd::storyboard::crud::find_page_preview_png(&sb_base, pg.index)
                .map(|p| p.to_string_lossy().into_owned())
                .or_else(|| pg.slots.iter().find_map(|sl| sl.image.clone()));
            let cid = 1000 + idx as u64;
            v1_clips.push(json!({
                "clip": cid,
                "id": cid,
                "name": pg.title(),
                "start": sec_to_ticks(st),
                "end": sec_to_ticks(en.max(st + 1.0)),
                "source_path": img,
                "enabled": true,
            }));
        }
        if let Some(v1) = doc["sequence"]["video"].get_mut(0).and_then(|t| t["items"].as_array_mut()) {
            *v1 = v1_clips;
        }
    }

    if doc["sequence"]["video"].get(1).and_then(|t| t["items"].as_array()).is_some_and(Vec::is_empty) {
        let mut v2_clips = Vec::new();
        let chars_root = project_dir.join("assets/characters");
        if let Ok(rd) = std::fs::read_dir(&chars_root) {
            let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            dirs.sort();
            let n = dirs.len().max(1);
            for (idx, cdir) in dirs.into_iter().enumerate() {
                let cname = cdir.file_name().and_then(|s| s.to_str()).unwrap_or("Character").to_owned();
                let preview = super::collect_pngs_sorted(&cdir, 2).into_iter().next();
                let st = (idx as f64 / n as f64) * (song_sec * 0.5);
                let en = (st + (song_sec / n as f64).min(song_sec - st)).max(st + 2.0);
                let cid = 2000 + idx as u64;
                v2_clips.push(json!({
                    "clip": cid,
                    "id": cid,
                    "name": format!("{cname} · Performance"),
                    "start": sec_to_ticks(st),
                    "end": sec_to_ticks(en),
                    "source_path": preview,
                    "enabled": true,
                }));
            }
        }
        if let Some(v2) = doc["sequence"]["video"].get_mut(1).and_then(|t| t["items"].as_array_mut()) {
            *v2 = v2_clips;
        }
    }

    if doc["sequence"]["video"].get(2).and_then(|t| t["items"].as_array()).is_some_and(Vec::is_empty) {
        let mut v3_clips = Vec::new();
        if let Ok(raw) = std::fs::read_to_string(project_dir.join("design/words.json"))
            && let Ok(wdoc) = serde_json::from_str::<Value>(&raw)
            && let Some(anims) = wdoc["animations"].as_array()
        {
            for (idx, anim) in anims.iter().enumerate() {
                let st = anim["song_anchor"].as_f64().unwrap_or((idx as f64) * 3.5);
                let dur = anim["authored_duration"].as_f64().unwrap_or(3.0).max(0.4);
                let words_txt: String = anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).collect::<Vec<_>>().join(" ");
                let label = if words_txt.is_empty() { format!("Lyric {}", idx + 1) } else { words_txt };
                let cid = 3000 + idx as u64;
                v3_clips.push(json!({
                    "clip": cid,
                    "id": cid,
                    "name": label,
                    "start": sec_to_ticks(st),
                    "end": sec_to_ticks(st + dur),
                    "enabled": true,
                }));
            }
        }
        if let Some(v3) = doc["sequence"]["video"].get_mut(2).and_then(|t| t["items"].as_array_mut()) {
            *v3 = v3_clips;
        }
    }

    if doc["sequence"]["video"].get(3).and_then(|t| t["items"].as_array()).is_some_and(Vec::is_empty) {
        let mut v4_clips = Vec::new();
        if let Ok(raw) = std::fs::read_to_string(project_dir.join("motion/actions.json"))
            && let Ok(mdoc) = serde_json::from_str::<Value>(&raw)
            && let Some(cues) = mdoc["cues"].as_array()
        {
            for (idx, cue) in cues.iter().enumerate() {
                let st = cue["start"].as_f64().or_else(|| cue["start_sec"].as_f64()).unwrap_or((idx as f64) * 4.0);
                let en = cue["end"].as_f64().or_else(|| cue["end_sec"].as_f64()).unwrap_or(st + 2.2).max(st + 0.35);
                let title = cue["title"].as_str().or_else(|| cue["name"].as_str()).unwrap_or("Motion Cue");
                let cid = 4000 + idx as u64;
                v4_clips.push(json!({
                    "clip": cid,
                    "id": cid,
                    "name": title,
                    "start": sec_to_ticks(st),
                    "end": sec_to_ticks(en),
                    "enabled": true,
                }));
            }
        }
        if let Some(v4) = doc["sequence"]["video"].get_mut(3).and_then(|t| t["items"].as_array_mut()) {
            *v4 = v4_clips;
        }
    }

    if doc["sequence"]["captions"].get(0).and_then(|t| t["captions"].as_array()).is_some_and(Vec::is_empty) {
        let srt_cues = parse_srt_captions(project_dir);
        if !srt_cues.is_empty()
            && let Some(c1) = doc["sequence"]["captions"].get_mut(0)
        {
            c1["captions"] = json!(srt_cues);
        }
    }

    if doc["sequence"]["duration"].as_i64().unwrap_or(0) < song_ticks {
        doc["sequence"]["duration"] = json!(song_ticks);
    }
    recompute_sequence_duration(doc);
}

pub(crate) fn render_composite_frame(project_dir: &Path, fcproj_path: &Path, tick: Tick, out_png: &Path) -> Result<()> {
    let doc = load_fcproj(fcproj_path);
    let mut canvas = RgbaImage::from_pixel(1280, 720, Rgba([12, 16, 24, 255]));
    let t = tick.0;

    for (idx, trk) in doc["sequence"]["video"].as_array().into_iter().flatten().enumerate() {
        if trk["muted"].as_bool() == Some(true) || trk["enabled"].as_bool() == Some(false) {
            continue;
        }
        let active_path = trk["items"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["start"].as_i64().unwrap_or(0) <= t && c["end"].as_i64().unwrap_or(0) >= t)
            .and_then(|c| c["source_path"].as_str())
            .map(PathBuf::from)
            .or_else(|| if idx == 0 { super::collect_pngs_sorted(&project_dir.join("assets/backgrounds"), 1).into_iter().next() } else { None });
        if let Some(p) = active_path
            && let Ok(img) = crate::native_media::load_rgba(&p)
        {
            if idx == 0 {
                let resized = image::imageops::resize(&img, 1280, 720, image::imageops::FilterType::Triangle);
                image::imageops::overlay(&mut canvas, &resized, 0, 0);
            } else {
                let resized = image::imageops::resize(&img, 480, 480, image::imageops::FilterType::Triangle);
                image::imageops::overlay(&mut canvas, &resized, 400, 140);
            }
        }
    }
    crate::native_media::encode_png_atomic(&canvas, out_png)
}
