//! Privacy-scrubbed diagnostics copy (`Copy Diagnostics` & `Report a Problem`), preference +
//! `storytold/craft-ai-integrations`, project
//! cache refresh, and safe HTTPS opener.

use std::path::PathBuf;

use magic_engine::Session;
use magic_ui_egui::{AccentChoice, InterfaceStyle, ThemeMode, UiShellState};
use serde_json::{Value, json};

fn scrub_home(raw: &str) -> String {
    let Ok(home) = std::env::var("HOME") else {
        return raw.to_owned();
    };
    let trimmed = home.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() { raw.to_owned() } else { raw.replace(trimmed, "~") }
}

/// Build a sanitized Markdown + JSON diagnostics payload (scrubs `$HOME` to `~` and never includes secrets/tokens).
pub fn build_sanitized_diagnostics(session: &mut Session, ui_state: &mut UiShellState, last_render_ms: f32) -> String {
    if ui_state.cache.doctor.is_null()
        && let Ok(doc) = session.execute("app.doctor", json!({}))
    {
        ui_state.cache.doctor = doc;
    }
    if ui_state.cache.ai_status.is_null()
        && let Ok(ai) = session.execute("ai.status", json!({}))
    {
        ui_state.cache.ai_status = ai;
    }

    let proj_display = ui_state.project_path.as_ref().map_or_else(|| "none".into(), |p| scrub_home(&p.to_string_lossy()));

    let summary = ui_state.ai.problem_summary.trim();
    let steps = ui_state.ai.problem_steps.trim();
    let mut out = String::new();
    out.push_str("# Magic Studio Sanitized Diagnostics\n\n");
    if !summary.is_empty() {
        out.push_str(&format!("**Summary:** {}\n\n", scrub_home(summary)));
    }
    if !steps.is_empty() {
        out.push_str(&format!("**Steps:**\n{}\n\n", scrub_home(steps)));
    }

    let payload = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "renderer": "eframe-wgpu (storytold/egui)",
        "last_render_ms": last_render_ms,
        "view": ui_state.effective_view().as_id(),
        "interface_style": ui_state.interface_style.as_str(),
        "theme": ui_state.theme_mode.as_str(),
        "accent": ui_state.accent_choice.as_str(),
        "ui_scale_pct": ui_state.ui_scale_pct,
        "project": proj_display,
        "ai_enabled": ui_state.ai.enabled,
        "ai_roles": ui_state.cache.ai_status.get("roles").cloned().unwrap_or(Value::Null),
        "craft_apps": ui_state.cache.doctor.get("apps").cloned().unwrap_or(Value::Null),
    });
    let json_text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into());
    out.push_str("```json\n");
    out.push_str(&scrub_home(&json_text));
    out.push_str("\n```\n");
    out
}

/// Hydrate appearance preferences and `craft-ai-integrations` status.
pub fn hydrate_prefs_and_ai(session: &mut Session, ui_state: &mut UiShellState) {
    if let Ok(prefs_res) = session.execute("prefs.get", json!({})) {
        let p = &prefs_res["prefs"];
        if let Some(st) = p.pointer("/appearance/interface_style").and_then(Value::as_str).and_then(InterfaceStyle::parse) {
            ui_state.interface_style = st;
        }
        if let Some(tm) = p.pointer("/appearance/theme").and_then(Value::as_str).and_then(ThemeMode::parse) {
            ui_state.theme_mode = tm;
        }
        if let Some(ac) = p.pointer("/appearance/accent").and_then(Value::as_str).and_then(AccentChoice::parse) {
            ui_state.accent_choice = ac;
        }
        if let Some(hc) = p.pointer("/appearance/accessible_contrast").and_then(Value::as_bool) {
            ui_state.high_contrast = hc;
        }
        if let Some(sc) = p.pointer("/appearance/ui_scale").and_then(Value::as_f64) {
            let pct = if sc <= 4.0 { (sc * 100.0).round() as u16 } else { sc.round() as u16 };
            ui_state.ui_scale_pct = pct.clamp(80, 200);
        }
        if let Some(as_en) = p.pointer("/autosave/enabled").and_then(Value::as_bool) {
            ui_state.autosave_enabled = as_en;
        }
        if let Some(as_min) = p.pointer("/autosave/interval_minutes").and_then(Value::as_f64) {
            ui_state.autosave_minutes = as_min.clamp(1.0, 60.0);
        }
        if let Some(fdirs) = p.pointer("/lyrics/font_dirs").and_then(Value::as_str) {
            ui_state.lyrics.font_dir_extra = fdirs.to_owned();
            apply_app_font_dirs(fdirs);
        }
        if let Some(fp_url) = p.pointer("/fonts/preview_service_url").and_then(Value::as_str).filter(|u| !u.trim().is_empty()) {
            ui_state.font_preview_url = fp_url.to_owned();
        }
    }
    if let Ok(ai) = session.execute("ai.status", json!({})) {
        sync_ai_ui_from_status(ui_state, &ai);
        ui_state.cache.ai_status = ai;
    }
}

pub fn sync_ai_ui_from_status(ui_state: &mut UiShellState, ai: &Value) {
    if let Some(en) = ai.get("enabled").and_then(Value::as_bool) {
        ui_state.ai.enabled = en;
    }
    if let Some(fe) = ai.pointer("/fidget/enabled").and_then(Value::as_bool) {
        ui_state.ai.fidget_enabled = fe;
    }
    if let Some(fs) = ai.pointer("/fidget/seamless").and_then(Value::as_bool) {
        ui_state.ai.fidget_seamless = fs;
    }
    if let Some(fc) = ai.pointer("/fidget/connection").and_then(Value::as_str) {
        ui_state.ai.fidget_connection = fc.to_owned();
    }
    if let Some(fm) = ai.pointer("/fidget/model").and_then(Value::as_str) {
        ui_state.ai.fidget_model = fm.to_owned();
    }
    if let Some(se) = ai.pointer("/scribbles/enabled").and_then(Value::as_bool) {
        ui_state.ai.scribbles_enabled = se;
    }
    if let Some(sc) = ai.pointer("/scribbles/connection").and_then(Value::as_str) {
        ui_state.ai.scribbles_connection = sc.to_owned();
    }
    if let Some(sm) = ai.pointer("/scribbles/model").and_then(Value::as_str) {
        ui_state.ai.scribbles_model = sm.to_owned();
    }
    if let Some(tpl) = ai.pointer("/scribbles/template").and_then(Value::as_str) {
        ui_state.ai.scribbles_template = tpl.to_owned();
    }
}

pub fn hydrate_selected_character(session: &mut Session, ui_state: &mut UiShellState) {
    let chars = ui_state.cache.characters["characters"].as_array().cloned().unwrap_or_default();
    let active_id = ui_state.selected_character_id.clone().or_else(|| chars.first().and_then(|c| c["id"].as_str().map(str::to_owned)));
    let Some(cid) = active_id else {
        ui_state.cache.character_keys = Value::Null;
        ui_state.cache.character_path = Value::Null;
        return;
    };
    let has_ecproj = chars
        .iter()
        .find(|c| c["id"].as_str() == Some(cid.as_str()))
        .and_then(|c| c["views"].as_array())
        .is_some_and(|vs| vs.iter().any(|v| v["ecproj"].as_bool() == Some(true)));
    if has_ecproj {
        ui_state.cache.character_keys = session.execute("character.keys", json!({"character": cid, "op": "list"})).unwrap_or(Value::Null);
        ui_state.cache.character_path = session.execute("character.path", json!({"character": cid, "op": "inspect"})).unwrap_or(Value::Null);
    } else {
        ui_state.cache.character_keys = Value::Null;
        ui_state.cache.character_path = Value::Null;
    }
}

fn hydrate_open_project_caches(session: &mut Session, ui_state: &mut UiShellState) {
    if let Ok(lyr) = session.execute("song.lyrics", json!({})) {
        ui_state.cache.song_lyrics = lyr;
    }
    if let Ok(wf) = session.execute("song.waveform", json!({"bins": 240})) {
        ui_state.cache.song_waveform = wf;
    }
    if let Ok(pages) = session.execute("storyboard.pages", json!({"full": true})) {
        ui_state.cache.storyboard_pages = pages;
    }
    if let Ok(chk) = session.execute("storyboard.check", json!({})) {
        ui_state.cache.storyboard_check = chk;
    }
    if let Ok(chars) = session.execute("character.list", json!({})) {
        ui_state.cache.characters = chars;
    }
    hydrate_selected_character(session, ui_state);
    if let Ok(words) = session.execute("words.read", json!({})) {
        ui_state.cache.words_doc = words;
    }
    if let Ok(wfx) = session.execute("words.effects", json!({})) {
        ui_state.cache.words_effects = wfx;
    }
    if let Ok(mot) = session.execute("motion.read", json!({})) {
        ui_state.cache.motion_doc = mot;
    }
    if let Ok(mfx) = session.execute("motion.effects", json!({})) {
        ui_state.cache.motion_effects = mfx;
    }
    if let Ok(film) = session.execute("video.film", json!({})) {
        ui_state.cache.film_status = film;
    }
    if let Ok(tl) = session.execute("video.timeline", json!({})) {
        ui_state.cache.timeline = tl;
    }
}

pub fn refresh_cached_state(session: &mut Session, ui_state: &mut UiShellState, initial: bool) {
    let prev_proj = ui_state.project_path.clone();
    if let Ok(snap) = session.execute("project.snapshot", json!({})) {
        ui_state.project_path = snap["project"].as_str().map(PathBuf::from);
        ui_state.cache.snapshot = snap;
    } else {
        ui_state.project_path = session.project_path.clone();
    }
    if ui_state.project_path.is_some() && (prev_proj != ui_state.project_path || ui_state.studio.title_text.is_empty()) {
        ui_state.studio.title_text = ui_state.project_name();
    }
    if initial {
        if let Ok(cat) = session.execute("project.catalog", json!({})) {
            ui_state.cache.catalog = cat;
        }
        if let Ok(ai) = session.execute("ai.status", json!({})) {
            sync_ai_ui_from_status(ui_state, &ai);
            ui_state.cache.ai_status = ai;
        }
    }
    if ui_state.project_path.is_some() {
        hydrate_open_project_caches(session, ui_state);
    }
    if let Some(song) = ui_state.song_audio_path()
        && let Ok(probe) = magic_util::media::probe_audio_in_process(std::path::Path::new(&song))
        && let Some(ticks) = probe["duration"].as_i64()
    {
        ui_state.audio_duration_sec = magic_util::time::Tick(ticks).seconds();
    } else if let Some(sec) = ui_state
        .cache
        .snapshot
        .pointer("/data/status/duration_seconds")
        .or_else(|| ui_state.cache.timeline.get("duration_seconds"))
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v > 0.0)
    {
        ui_state.audio_duration_sec = sec;
    }
}

pub fn open_https_url(ctx: &egui::Context, url: &str) {
    let trimmed = url.trim();
    if trimmed.starts_with("https://") && !trimmed.chars().any(char::is_control) {
        ctx.open_url(egui::OpenUrl::new_tab(trimmed));
    }
}

pub fn apply_app_font_dirs(raw: &str) {
    let dirs: Vec<PathBuf> = std::env::split_paths(raw).filter(|p| !p.as_os_str().is_empty()).collect();
    craft_font_catalog::Catalog::global().set_app_dirs(dirs);
    craft_font_catalog::Catalog::global().rescan_if_changed();
}
