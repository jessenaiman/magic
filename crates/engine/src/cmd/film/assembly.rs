<<<<<<< HEAD
//! `video.assemble`: the film put together by FilmCraft using exact [`Tick`]s and rational
//! [`FrameRate`]s. Song audio on A1, scene plates on V1, character animation on V2, kinetic
//! teaching lyrics on V3 (`lettering/lyrics.mov`), and song-action drawings on V4 (`motion/actions.mov`).

use crate::time::{FrameRate, Tick, frame_rate_json, parse_frame_rate, parse_tick};
use crate::{EngineError, Result, Session};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
=======
//! `video.assemble`: builds and updates the unified `film/<song>.fcproj` sequence from exact
//! [`Tick`]s and rational [`FrameRate`]s across `A1` Song Master, `V1` Storyboard Scenes,
//! `V2` Character Animation, `V3` Kinetic Lyrics, `V4` Action Motion, and `C1` Captions.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::time::{FrameRate, Tick, frame_rate_json, parse_frame_rate, parse_tick};
use crate::{EngineError, Result, Session};
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

pub(crate) const BACKGROUND_TRACK: &str = "V1";
pub(crate) const CHARACTER_TRACK: &str = "V2";
pub(crate) const LYRICS_TRACK: &str = "V3";
pub(crate) const MOTION_TRACK: &str = "V4";
pub(crate) const SONG_TRACK: &str = "A1";

pub(crate) fn track_role(label: &str) -> Option<&'static str> {
    match label {
        BACKGROUND_TRACK => Some("background"),
        CHARACTER_TRACK => Some("character"),
        LYRICS_TRACK => Some("lyrics"),
        MOTION_TRACK => Some("motion"),
        SONG_TRACK => Some("song"),
        _ => None,
    }
}

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(message.into())
}

fn arg(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub(crate) fn run(_dir: &Path, project: &Path, fresh: bool, steps: &[Value]) -> Result<Vec<Value>> {
    crate::cmd::film::run(project, fresh, steps)
}

pub(crate) fn ids(result: &Value, field: &str) -> Vec<u64> {
    result[field].as_array().into_iter().flatten().filter_map(Value::as_u64).collect()
}

fn background(s: &Session, dir: &Path, entry: &Value) -> Result<PathBuf> {
    if let Some(path) = entry["path"].as_str() {
        let path = dir.join(path);
        return path.is_file().then_some(path).ok_or_else(|| invalid("background image path missing"));
    }
    let id = entry["asset"].as_str().ok_or_else(|| invalid("each background needs asset or path"))?;
    s.project_assets
        .iter()
        .find(|a| a.id == id)
        .and_then(|a| a.path.clone())
        .map(|p| dir.join(p))
        .filter(|p| p.is_file())
        .ok_or_else(|| invalid(format!("No image for asset {id}")))
}

fn spans(times: &[Tick], duration: Tick) -> Vec<(Tick, Tick)> {
    times.iter().enumerate().map(|(i, from)| (*from, (times.get(i + 1).copied().unwrap_or(duration) - *from).max(Tick::ZERO))).collect()
}

fn resolve_scene_image(scene: &Value, dir: &Path, assets: &[crate::ProjectAsset]) -> Option<PathBuf> {
    let asset_id = scene["image_asset_id"].as_str().or_else(|| scene["background_asset"].as_str()).or_else(|| scene["asset"].as_str());
    if let Some(id) = asset_id
        && let Some(path) = assets.iter().find(|a| a.id == id).and_then(|a| a.path.as_ref()).map(|p| dir.join(p)).filter(|p| p.is_file())
    {
        return Some(path);
    }
    scene["image_path"].as_str().or_else(|| scene["path"].as_str()).map(|p| dir.join(p)).filter(|p| p.is_file())
}

fn backgrounds_from_words(words_doc: &Value, dir: &Path, assets: &[crate::ProjectAsset]) -> Vec<(Tick, PathBuf)> {
    let mut out: Vec<(Tick, PathBuf)> = words_doc["animations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|sc| {
            let start = sc.get("song_anchor_tick").or_else(|| sc.get("song_anchor")).and_then(parse_tick).unwrap_or(Tick::ZERO);
            Some((start, resolve_scene_image(sc, dir, assets)?))
        })
        .collect();
    out.sort_by_key(|a| a.0);
    let min_gap = Tick::from_units(1, 1000);
    out.dedup_by(|a, b| a.1 == b.1 || (a.0 - b.0).abs() < min_gap);
    out
}

fn auto_backgrounds(s: &Session, dir: &Path, duration: Tick) -> Option<Vec<(Tick, PathBuf)>> {
    let imgs: Vec<PathBuf> =
        s.project_assets.iter().filter(|a| a.kind == "image").filter_map(|a| a.path.as_ref().map(|p| dir.join(p))).filter(|p| p.is_file()).collect();
    if let Ok(bytes) = std::fs::read(dir.join("design/words.json"))
        && let Ok(doc) = serde_json::from_slice::<Value>(&bytes)
    {
        let bgs = backgrounds_from_words(&doc, dir, &s.project_assets);
        let custom = doc["animations"].as_array().is_some_and(|a| a.iter().any(|x| x["background_custom"] == true));
        if bgs.len() > 1 || (!bgs.is_empty() && (custom || imgs.len() <= 1)) {
            return Some(bgs);
        }
    }
    if imgs.is_empty() {
        return None;
    }
    let count = imgs.len() as i64;
    let step = duration.mul_ratio(1, count).max(Tick::from_units(1, 1));
    Some(imgs.into_iter().enumerate().map(|(i, p)| (step.mul_ratio(i as i64, 1), p)).collect())
}

struct ClipSpec {
    from: Tick,
    to: Option<Tick>,
    source_in: Option<Tick>,
    path: PathBuf,
    name: Option<String>,
}

struct Inputs {
    name: String,
<<<<<<< HEAD
    audio: PathBuf,
=======
    audio: Option<PathBuf>,
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    lettering: Option<PathBuf>,
    motion_overlay: Option<PathBuf>,
    backgrounds: Vec<ClipSpec>,
    characters: Vec<ClipSpec>,
    lyrics_clips: Vec<ClipSpec>,
    duration: Tick,
}

fn parse_clip_list(dir: &Path, entries: Option<&Vec<Value>>, path_key: &str) -> Result<Vec<ClipSpec>> {
    let mut out = Vec::new();
    for entry in entries.into_iter().flatten() {
        let raw_path = entry[path_key].as_str().or_else(|| entry["path"].as_str());
        let clip = raw_path
            .map(|c| {
                let p = PathBuf::from(c);
                if p.is_absolute() { p } else { dir.join(p) }
            })
            .filter(|c| c.is_file())
            .ok_or_else(|| invalid(format!("{path_key} clip missing: {}", entry[path_key])))?;
        let from = entry.get("from_tick").or_else(|| entry.get("from")).or_else(|| entry.get("start")).and_then(parse_tick).unwrap_or(Tick::ZERO);
        let to = entry.get("to_tick").or_else(|| entry.get("to")).or_else(|| entry.get("end")).and_then(parse_tick);
        let source_in = entry.get("source_in_tick").or_else(|| entry.get("source_in")).and_then(parse_tick);
        let name = entry["name"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
        out.push(ClipSpec { from, to, source_in, path: clip, name });
    }
    out.sort_by_key(|a| a.from);
    Ok(out)
}

fn inputs(s: &Session, dir: &Path, p: &Value) -> Result<Inputs> {
    let name = s.project_path.as_ref().and_then(|p| p.file_stem()).map_or("Film".into(), |n| n.to_string_lossy().into_owned());
<<<<<<< HEAD
    let audio = s.song_audio.clone().map(|a| dir.join(a)).filter(|a| a.is_file()).ok_or_else(|| invalid("No song audio in this project"))?;
    let lettering_candidate = dir.join(p["lettering"].as_str().unwrap_or("lettering/lyrics.mov"));
    let motion_candidate = dir.join(p["motion"].as_str().unwrap_or("motion/actions.mov"));
    let explicit_skip = p.get("lettering").is_some_and(Value::is_null);
    let duration = crate::workshop::song_ticks(dir)
        .or_else(|| crate::cmd::film::media_duration_ticks(&audio, false).ok())
        .ok_or_else(|| invalid("Song length unknown; analyze the song first"))?;
=======
    let audio = s.song_audio.clone().map(|a| dir.join(a)).filter(|a| a.is_file());
    let lettering_candidate = dir.join(p["lettering"].as_str().unwrap_or("lettering/lyrics.mov"));
    let motion_candidate = dir.join(p["motion"].as_str().unwrap_or("motion/actions.mov"));
    let duration = crate::workshop::song_ticks(dir)
        .or_else(|| audio.as_deref().and_then(|a| crate::cmd::film::media_duration_ticks(a, false).ok()))
        .unwrap_or_else(|| Tick::from_units(30, 1));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let backgrounds = if let Some(entries) = p["backgrounds"].as_array().filter(|b| !b.is_empty()) {
        let mut bgs = entries
            .iter()
            .map(|e| {
                let from = e.get("from_tick").or_else(|| e.get("from")).or_else(|| e.get("start")).and_then(parse_tick).unwrap_or(Tick::ZERO);
                let to = e.get("to_tick").or_else(|| e.get("to")).or_else(|| e.get("end")).and_then(parse_tick);
                let name = e["name"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
                Ok(ClipSpec { from, to, source_in: None, path: background(s, dir, e)?, name })
            })
            .collect::<Result<Vec<_>>>()?;
        bgs.sort_by_key(|a| a.from);
        bgs
    } else {
        auto_backgrounds(s, dir, duration)
<<<<<<< HEAD
            .ok_or_else(|| {
                let known: Vec<&str> = s.project_assets.iter().map(|a| a.id.as_str()).collect();
                invalid(format!("backgrounds: [{{\"asset\":id,\"from_tick\":ticks}}, ...] is required; assets: {}", known.join(", ")))
            })?
=======
            .unwrap_or_default()
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            .into_iter()
            .map(|(from, path)| ClipSpec { from, to: None, source_in: None, path, name: None })
            .collect()
    };
    let characters = parse_clip_list(dir, p["characters"].as_array(), "clip")?;
    let lyrics_clips = parse_clip_list(dir, p["lyrics"].as_array(), "clip")?;
<<<<<<< HEAD
    let lettering = if lettering_candidate.is_file() {
        Some(lettering_candidate)
    } else if explicit_skip || !lyrics_clips.is_empty() {
        None
    } else {
        return Err(invalid(format!("No lettering render at {}; run words.letter first", lettering_candidate.display())));
    };
=======
    let lettering = lettering_candidate.is_file().then_some(lettering_candidate);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let motion_overlay = motion_candidate.is_file().then_some(motion_candidate);
    Ok(Inputs { name, audio, lettering, motion_overlay, backgrounds, characters, lyrics_clips, duration })
}

<<<<<<< HEAD
fn build_film(dir: &Path, project: &Path, film: &Inputs, fps: FrameRate) -> Result<()> {
=======
fn build_film(s: &Session, dir: &Path, project: &Path, film: &Inputs, fps: FrameRate) -> Result<()> {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let fps = fps.sane();
    let mut unique_paths: Vec<PathBuf> = Vec::new();
    let mut push_unique = |p: &Path| -> usize {
        if let Some(idx) = unique_paths.iter().position(|existing| existing == p) {
            idx
        } else {
            let idx = unique_paths.len();
            unique_paths.push(p.to_path_buf());
            idx
        }
    };
<<<<<<< HEAD
    let audio_idx = push_unique(&film.audio);
=======
    let audio_idx: Option<usize> = film.audio.as_deref().map(&mut push_unique);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let bg_indices: Vec<usize> = film.backgrounds.iter().map(|b| push_unique(&b.path)).collect();
    let lettering_idx: Option<usize> = film.lettering.as_deref().map(&mut push_unique);
    let motion_idx: Option<usize> = film.motion_overlay.as_deref().map(&mut push_unique);
    let char_indices: Vec<usize> = film.characters.iter().map(|c| push_unique(&c.path)).collect();
    let lyr_indices: Vec<usize> = film.lyrics_clips.iter().map(|l| push_unique(&l.path)).collect();

    let media: Vec<String> = unique_paths.iter().map(|p| arg(p)).collect();
    let made = run(
        dir,
        project,
        true,
        &[
            json!({"id": "file.newProject", "params": {"name": film.name}}),
            json!({"id": "file.newSequence", "params": {"name": film.name, "width": 1920, "height": 1080, "fps": fps.as_f64()}}),
            json!({"id": "file.import", "params": {"paths": media}}),
        ],
    )?;
    let items = made.get(2).map(|r| ids(r, "items")).unwrap_or_default();
    if items.len() != media.len() {
        return Err(invalid(format!("FilmCraft imported {} of {} files", items.len(), media.len())));
    }

    let times: Vec<Tick> = film.backgrounds.iter().map(|b| b.from).collect();
    let default_spans = spans(&times, film.duration);
<<<<<<< HEAD
    let mut place =
        vec![json!({"id": "timeline.place", "params": {"item": items[audio_idx], "track": SONG_TRACK, "frame": 0, "insert": false, "video": false}})];
    let mut names_for_placements: Vec<Option<String>> = vec![None];
=======
    let mut place = Vec::new();
    let mut names_for_placements: Vec<Option<String>> = Vec::new();

    if let Some(a_idx) = audio_idx {
        place.push(json!({"id": "timeline.place", "params": {"item": items[a_idx], "track": SONG_TRACK, "frame": 0, "insert": false, "video": false}}));
        names_for_placements.push(None);
    }
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

    for ((bg, (from, default_len)), &item_idx) in film.backgrounds.iter().zip(default_spans).zip(&bg_indices) {
        let length = bg.to.map(|to| (to - from).max(Tick::ZERO)).unwrap_or(default_len);
        let start_frame = fps.frame_at(from).max(0);
        let end_frame = fps.frame_at(from + length).max(start_frame + 1);
        place.push(json!({
            "id": "timeline.place",
            "params": {
                "item": items[item_idx],
                "track": BACKGROUND_TRACK,
                "frame": start_frame,
                "insert": false,
                "duration": {"frame": end_frame - start_frame}
            }
        }));
        names_for_placements.push(bg.name.clone());
    }

    for (ch, &item_idx) in film.characters.iter().zip(&char_indices) {
        let start_frame = fps.frame_at(ch.from).max(0);
        let mut params = json!({"item": items[item_idx], "track": CHARACTER_TRACK, "frame": start_frame, "insert": false, "audio": false});
        if let Some(to) = ch.to {
            let end_frame = fps.frame_at(to).max(start_frame + 1);
            params["duration"] = json!({"frame": end_frame - start_frame});
            params["sourceIn"] = json!(ch.source_in.unwrap_or(ch.from).0);
        }
        place.push(json!({"id": "timeline.place", "params": params}));
        names_for_placements.push(ch.name.clone());
    }

    if !film.lyrics_clips.is_empty() {
        for (lyr, &item_idx) in film.lyrics_clips.iter().zip(&lyr_indices) {
            let start_frame = fps.frame_at(lyr.from).max(0);
            let mut params = json!({"item": items[item_idx], "track": LYRICS_TRACK, "frame": start_frame, "insert": false, "audio": false});
            if let Some(to) = lyr.to {
                let end_frame = fps.frame_at(to).max(start_frame + 1);
                params["duration"] = json!({"frame": end_frame - start_frame});
                params["sourceIn"] = json!(lyr.source_in.unwrap_or(lyr.from).0);
            }
            place.push(json!({"id": "timeline.place", "params": params}));
            names_for_placements.push(lyr.name.clone());
        }
    } else if let Some(l_idx) = lettering_idx {
        place.push(json!({"id": "timeline.place", "params": {"item": items[l_idx], "track": LYRICS_TRACK, "frame": 0, "insert": false, "audio": false}}));
        names_for_placements.push(Some("Kinetic Lyrics".into()));
    }

    if let Some(m_idx) = motion_idx {
        place.push(json!({"id": "timeline.place", "params": {"item": items[m_idx], "track": MOTION_TRACK, "frame": 0, "insert": false, "audio": false}}));
        names_for_placements.push(Some("Action Motion".into()));
    }

<<<<<<< HEAD
    let placed = run(dir, project, false, &place)?;
    let nb = film.backgrounds.len();
    let nc = film.characters.len();
    let bg_clips: Vec<u64> = placed[1..=nb].iter().flat_map(|r| ids(r, "clips")).collect();
    let mut post_steps = vec![json!({"id": "clip.fillFrame", "params": {"clips": bg_clips}})];

=======
    let placed = if place.is_empty() { Vec::new() } else { run(dir, project, false, &place)? };
    let mut post_steps = Vec::new();
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    for (idx, (bg, (from, default_len))) in film.backgrounds.iter().zip(spans(&times, film.duration)).enumerate() {
        let length = bg.to.map(|to| (to - from).max(Tick::ZERO)).unwrap_or(default_len);
        let label = bg.name.clone().unwrap_or_else(|| format!("Scene {}", idx + 1));
        post_steps.push(json!({
            "id": "markers.addRange",
            "params": {
                "time": from.0,
                "duration": length.0,
                "name": format!("scene_{}", idx + 1),
                "comment": label,
                "color": "Lavender",
            }
        }));
    }
<<<<<<< HEAD

=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    for (res, custom_name) in placed.iter().zip(names_for_placements.iter()) {
        if let Some(name) = custom_name
            && let Some(&clip_id) = ids(res, "clips").first()
        {
            post_steps.push(json!({"id": "clip.rename", "params": {"clip": clip_id, "name": name}}));
        }
    }
<<<<<<< HEAD
    if !film.characters.is_empty() || nc > 0 {
        post_steps.push(json!({"id": "timeline.setTrack", "params": {"track": CHARACTER_TRACK, "name": "Character Animation", "enabled": true}}));
    }
    if !film.lyrics_clips.is_empty() || lettering_idx.is_some() {
        post_steps.push(json!({"id": "timeline.setTrack", "params": {"track": LYRICS_TRACK, "name": "Kinetic Lyrics", "enabled": true}}));
    }
    if motion_idx.is_some() {
        post_steps.push(json!({"id": "timeline.setTrack", "params": {"track": MOTION_TRACK, "name": "Action Motion", "enabled": true}}));
    }
    run(dir, project, false, &post_steps)?;
=======
    if !post_steps.is_empty() {
        run(dir, project, false, &post_steps)?;
    }

    // Hydrate any remaining authored stage tracks (`storyboard.designcraft`, `words.json`, `actions.json`, `captions.srt`)
    let mut doc = super::fcproj_doc::load_fcproj(project);
    super::fcproj_doc::hydrate_live_stage_tracks(s, dir, &mut doc);
    super::fcproj_doc::save_fcproj(project, &mut doc)?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    Ok(())
}

fn export(dir: &Path, project: &Path, name: &str) -> Result<PathBuf> {
    let exports = dir.join("exports");
    std::fs::create_dir_all(&exports)?;
    let out = exports.join(format!("{name}.mp4"));
    crate::cmd::film::export(project, &out, false)?;
    Ok(out)
}

pub(crate) fn ensure_song_film(s: &mut Session, audio: &Path) -> Result<PathBuf> {
    let dir = crate::workshop::project_dir(s)?;
    let wanted = crate::cmd::film::film_path(s)?;
    if wanted.is_file() {
        crate::cmd::film::link(s, &wanted)?;
        return Ok(wanted);
    }
    if let Some(folder) = wanted.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let name = s.project_path.as_ref().and_then(|p| p.file_stem()).map_or("Film".into(), |n| n.to_string_lossy().into_owned());
    let song = dir.join(audio);
    let made = run(
        &dir,
        &wanted,
        true,
        &[
            json!({"id": "file.newProject", "params": {"name": name}}),
<<<<<<< HEAD
            json!({"id": "file.newSequence", "params": {"name": name, "width": 1920, "height": 1080, "fps": FrameRate::FPS_30.as_f64()}}),
            json!({"id": "file.import", "params": {"paths": [arg(&song)]}}),
        ],
    )?;
    let item = made.get(2).map(|r| ids(r, "items")).unwrap_or_default().first().copied().ok_or_else(|| invalid("FilmCraft did not import the song"))?;
    run(&dir, &wanted, false, &[json!({"id": "timeline.place", "params": {"item": item, "track": SONG_TRACK, "frame": 0, "insert": false, "video": false}})])?;
=======
            json!({"id": "file.newSequence", "params": {"name": name, "width": 1920, "height": 1080, "fps": FrameRate::FPS_24.as_f64()}}),
            json!({"id": "file.import", "params": {"paths": [arg(&song)]}}),
        ],
    )?;
    if let Some(item) = made.get(2).and_then(|r| ids(r, "items").first().copied()) {
        run(
            &dir,
            &wanted,
            false,
            &[json!({"id": "timeline.place", "params": {"item": item, "track": SONG_TRACK, "frame": 0, "insert": false, "video": false}})],
        )?;
    }
    let mut doc = super::fcproj_doc::load_fcproj(&wanted);
    super::fcproj_doc::hydrate_live_stage_tracks(s, &dir, &mut doc);
    super::fcproj_doc::save_fcproj(&wanted, &mut doc)?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    crate::cmd::film::link(s, &wanted)?;
    Ok(wanted)
}

pub fn cmd_video_assemble(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let film = inputs(s, &dir, p)?;
    let project = crate::cmd::film::film_path(s)?;
    if let Some(folder) = project.parent() {
        std::fs::create_dir_all(folder)?;
    }
<<<<<<< HEAD
    let fps = p.get("fps").and_then(parse_frame_rate).unwrap_or(FrameRate::FPS_30);
    build_film(&dir, &project, &film, fps)?;
    crate::cmd::film::link(s, &project)?;
    let exported = if p["export"] == false { None } else { Some(export(&dir, &project, &film.name)?) };
=======
    let fps = p.get("fps").and_then(parse_frame_rate).unwrap_or(FrameRate::FPS_24);
    build_film(s, &dir, &project, &film, fps)?;
    crate::cmd::film::link(s, &project)?;
    let exported = if p["export"] == true { Some(export(&dir, &project, &film.name)?) } else { None };
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let backgrounds: Vec<Value> = film.backgrounds.iter().map(|b| json!({"from_tick": b.from.0, "image": b.path, "name": b.name})).collect();
    Ok(json!({
        "ok": true,
        "project": project,
        "export": exported,
        "duration_tick": film.duration.0,
        "frame_rate": frame_rate_json(fps),
        "backgrounds": backgrounds,
        "characters": film.characters.len(),
        "lyrics": film.lyrics_clips.len(),
        "lettering": film.lettering,
        "motion": film.motion_overlay,
    }))
}
