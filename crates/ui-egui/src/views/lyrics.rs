//! `Lyrics` view (`#lyrics-workspace-root`): `EffectCraft` `V3` Kinetic Typography workspace
//! matching `05-lyrics-v3-effectcraft.jpg`, featuring a dedicated **Left 35mm Film Reel**,
//! underlying audio playback, and `EffectCraft` (`V3`) ↔ `FilmCraft` (`V1`) track switching.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, ScalarSpec, child_pane, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image, ring_slider_h,
};
use crate::state::{UiAction, ViewCtx};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::views::lyrics_inspector::{draw_captions_and_template_strip, draw_lyrics_inspector};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

struct LyricReelEntry {
    id: String,
    anchor_sec: f64,
    line_text: String,
    frame_path: Option<String>,
}

// TODO(MAGIC-REVIEW): CHANGE lyric preview provenance to the real renderer; coordinate correction alone does not validate frames.
pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let body = ui.available_rect_before_wrap();
    let left_w = if body.width() >= 1180.0 { 246.0 } else { 218.0 };
    let right_w = if body.width() >= 1180.0 { 324.0 } else { 280.0 };

    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(left_w), 8.0, 180.0)
    {
        let mut rail = child_pane(ui, left_split.first, "lyrics_moments");
        draw_moment_film_reel(&mut rail, ctx);
        left_split.second
    } else {
        body
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(right_w), 8.0, 220.0)
    {
        let mut center = child_pane(ui, right_split.first, "lyrics_stage");
        draw_stage_and_timeline(&mut center, ctx);
        let mut insp = child_pane(ui, right_split.second, "lyrics_insp");
        draw_lyrics_inspector(&mut insp, ctx);
    } else {
        let mut center = child_pane(ui, after_left, "lyrics_stage");
        draw_stage_and_timeline(&mut center, ctx);
    }
    let _ = ui.allocate_rect(body, Sense::hover());
}

fn active_section_revision(ctx: &ViewCtx<'_>) -> Option<(String, u64)> {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array)?;
    let chosen =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first())?;
    let id = chosen["animation_id"].as_str()?.to_owned();
    let rev = chosen["source_revision"].as_u64().unwrap_or(0);
    Some((id, rev))
}

fn collect_lyric_reel_entries(ctx: &ViewCtx<'_>, anims: &[Value]) -> Vec<LyricReelEntry> {
    let film_frames = ctx.state.active_film_frames();
    if !anims.is_empty() {
        return anims
            .iter()
            .enumerate()
            .map(|(idx, anim)| {
                let id = anim["animation_id"].as_str().unwrap_or("sec").to_owned();
                let sec_name = anim["scope"]["section"].as_str().unwrap_or(&id);
                let anchor_sec = anim["song_anchor"].as_f64().unwrap_or((idx as f64) * 4.0);
                let words_str: String = anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).take(8).collect::<Vec<_>>().join(" ");
                let line_text = if words_str.is_empty() { sec_name.to_owned() } else { words_str };
                let frame_path = anim["preview_frame"]
                    .as_str()
                    .or_else(|| anim["lyrics_frame"].as_str())
                    .or_else(|| anim["background_path"].as_str())
                    .filter(|p| !p.is_empty())
                    .map(str::to_owned)
                    .or_else(|| if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).cloned() });
                LyricReelEntry { id, anchor_sec, line_text, frame_path }
            })
            .collect();
    }

    let raw_lines: Vec<String> = ctx
        .state
        .cache
        .song_lyrics
        .get("lyrics")
        .and_then(Value::as_str)
        .map(|txt| txt.lines().map(str::trim).filter(|l| !l.is_empty()).take(8).map(str::to_owned).collect())
        .unwrap_or_default();

    let count = raw_lines.len().max(film_frames.len()).clamp(4, 8);
    let total = ctx.state.audio_duration_sec.max(24.0);
    (0..count)
        .map(|idx| {
            let anchor_sec = (idx as f64) * (total / (count as f64));
            let line_text = raw_lines.get(idx).cloned().unwrap_or_else(|| format!("Reel Beat {} · Align Audio", idx + 1));
            let frame_path = if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).cloned() };
            LyricReelEntry { id: format!("reel-{}", idx + 1), anchor_sec, line_text, frame_path }
        })
        .collect()
}

/// Dedicated Left 35mm Film Reel (`FilmbarOrientation::Vertical`) for `Lyrics` moments & FilmCraft scenes.
fn draw_moment_film_reel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let entries = collect_lyric_reel_entries(ctx, &anims);

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Film Reel · Lyrics").size(12.5).strong());
            status_pill(
                ui,
                ctx.pal,
                if anims.is_empty() { "Scene Reel" } else { "Timed V3" },
                if anims.is_empty() { ctx.pal.accent_amber } else { ctx.pal.accent_emerald },
            );
        });
        text_input(
            ui,
            ctx.reg,
            "words-lyrics-search",
            "Search lyrics",
            &mut ctx.state.lyrics.search,
            "Filter lyric moments…",
            (ui.available_width() - 4.0).max(120.0),
        );
        ui.add_space(4.0);

        let query = ctx.state.lyrics.search.trim().to_ascii_lowercase();
        ScrollArea::vertical().id_salt("words_moments_scroll").auto_shrink([false, false]).show(ui, |ui| {
            for (idx, item) in entries.iter().enumerate() {
                if !query.is_empty() && !item.line_text.to_ascii_lowercase().contains(&query) && !item.id.to_ascii_lowercase().contains(&query) {
                    continue;
                }
                let selected = ctx.state.lyrics.selected_anim_id.as_deref().map_or(idx == 0, |sel| sel == item.id)
                    || (anims.is_empty() && (ctx.state.lyrics.scrub_seconds - item.anchor_sec).abs() < 2.2);

                let reel_w = (ui.available_width() - 4.0).max(140.0);
                let (crect, resp) = ui.allocate_exact_size(Vec2::new(reel_w, 104.0), Sense::click());
                let inner = paint_filmbar_frame(
                    ui,
                    crect,
                    ctx.pal,
                    FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None },
                );
                if !paint_local_image(ui, inner, item.frame_path.as_deref(), ctx.pal.bg_stage, 3) {
                    paint_craft_doc_card(ui, inner, ctx.pal, "EC", "lyrics.ecproj", "");
                }
                // Cinema bottom caption band inside the 35mm reel frame
                let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 32.0).max(inner.min.y)), inner.max);
                ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 222));
                ui.painter().text(
                    Pos2::new(band.min.x + 6.0, band.min.y + 8.5),
                    Align2::LEFT_CENTER,
                    format!("0:{:04.1}", item.anchor_sec),
                    FontId::monospace(8.8),
                    if selected { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
                );
                ui.painter().text(
                    Pos2::new(band.min.x + 6.0, band.max.y - 8.5),
                    Align2::LEFT_CENTER,
                    truncate_words(&item.line_text, 22),
                    FontId::proportional(10.5),
                    ctx.pal.text_primary,
                );
                if resp.clicked() {
                    if !anims.is_empty() {
                        ctx.state.lyrics.selected_anim_id = Some(item.id.clone());
                    }
                    ctx.state.lyrics.scrub_seconds = item.anchor_sec;
                    ctx.push(UiAction::SeekAudio(item.anchor_sec));
                }
                ctx.reg.record(format!("words-moment-{}-0", item.id), &item.line_text, &resp, None);
                ui.add_space(6.0);
            }
        });
    });
    ctx.reg.record("words-lyrics", "Lyric Film Reel", &panel.response, None);
}

fn truncate_words(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn draw_stage_and_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let active =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first());

    let center_rect = ui.available_rect_before_wrap();
    // Generous 178px dock height guarantees Time Ruler + all 4 lanes (FILM/CUES, AUDIO, LYRICS, WORDS) stay 100% visible!
    let timeline_h = 178.0;
    if ctx.state.show_bottom_dock
        && let Some(vert) = split_rect(center_rect, SplitAxis::Vertical, SplitSize::FixedSecond(timeline_h), 8.0, 180.0)
    {
        let mut stage_ui = child_pane(ui, vert.first, "lyrics_upper_stage");
        draw_lyrics_stage_card(&mut stage_ui, ctx, &anims, active);
        let mut tl_ui = child_pane(ui, vert.second, "lyrics_lower_timeline");
        fill_card(&mut tl_ui, ctx.pal, true, |ui| {
            draw_real_lyrics_timeline(ui, ctx, &anims, active);
        });
        let _ = ui.allocate_rect(center_rect, Sense::hover());
    } else {
        draw_lyrics_stage_card(ui, ctx, &anims, active);
    }
}

fn draw_lyrics_audio_and_track_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_audio = ctx.state.song_audio_path().is_some();
    let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
    let play_lbl = if ctx.state.audio_playing { "Pause" } else { "Play Audio" };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("words-audio-play", play_icon, play_lbl, "Play / pause underlying song or FilmCraft audio track").primary().enabled(has_audio),
        )
        .clicked()
        {
            ctx.push(UiAction::ToggleAudio);
        }
        ui.label(RichText::new(format!("0:{:04.1}", ctx.state.lyrics.scrub_seconds)).monospace().size(9.8).color(if ctx.state.audio_playing {
            ctx.pal.accent_emerald
        } else {
            ctx.pal.text_muted
        }));
        for (src, lbl, tip) in [
            ("effectcraft", "EC · V3 Lyrics", "Isolate EffectCraft lettering/lyrics.ecproj track"),
            ("filmcraft", "FC · Film Track", "Preview underlying FilmCraft V1 / program sequence track"),
            ("overlay", "FC + V3", "Composite EffectCraft V3 lyrics over FilmCraft V1 sequence"),
        ] {
            let id = format!("words-track-src-{src}");
            let sel = ctx.state.lyrics.track_source == src;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.lyrics.track_source = src.into();
                ctx.exec("video.frame", json!({"seconds": ctx.state.lyrics.scrub_seconds}), None);
            }
        }
        for (aud, lbl, tip) in
            [("song", "♪ Song", "Monitor underlying song audio stem"), ("filmcraft", "FC Mix", "Monitor rendered FilmCraft sequence audio mix")]
        {
            let id = format!("words-audio-src-{aud}");
            let sel = ctx.state.lyrics.audio_source == aud;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.lyrics.audio_source = aud.into();
                if ctx.state.audio_playing {
                    let t = ctx.state.lyrics.scrub_seconds;
                    ctx.push(UiAction::SeekAudio(t));
                }
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-open-effectcraft", "external", "EffectCraft", "Open lettering/lyrics.ecproj in EffectCraft"))
            .clicked()
        {
            ctx.exec("workspace.open", json!({"app": "effectcraft", "domain": "lyrics"}), Some("Opened lettering/lyrics.ecproj"));
        }
    });
}

fn draw_lyrics_stage_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, _anims: &[Value], active: Option<&Value>) {
    let ecproj_meta = &ctx.state.cache.words_doc["ecproj"];
    let sec_heading = active.and_then(|a| a["scope"]["section"].as_str().or_else(|| a["animation_id"].as_str())).unwrap_or("Lyrics Stage");
    let ecproj_summary = if ecproj_meta.is_object() {
        let schema = ecproj_meta["schema"].as_u64().unwrap_or(0);
        let layers = ecproj_meta["layer_count"].as_u64().unwrap_or(0);
        format!("lettering/lyrics.ecproj · v{schema} · {layers} layers")
    } else {
        "lettering/lyrics.ecproj".to_owned()
    };

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label(RichText::new(sec_heading).size(12.5).strong());
            ui.label(RichText::new(ecproj_summary).monospace().size(9.5).color(ctx.pal.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-reload", "refresh", "Reload words.read & words.effects")).clicked() {
                    ctx.exec("words.read", json!({}), None);
                    ctx.exec("words.effects", json!({}), None);
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-align-audio", "music", "Align Audio", "Run speech alignment on song audio"))
                    .clicked()
                {
                    ctx.exec("audio.analyze", json!({}), Some("Aligned song audio"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("words-preview-frame", "image", "Render Frame", "Render active EffectCraft or FilmCraft frame at scrub time"),
                )
                .clicked()
                {
                    if ctx.state.lyrics.track_source == "filmcraft" {
                        ctx.exec("video.frame", json!({"seconds": ctx.state.lyrics.scrub_seconds}), Some("Rendered FilmCraft frame"));
                    } else {
                        let kind = if ctx.state.lyrics.backdrop_mode == "scene" { "stage" } else { "lyrics" };
                        ctx.exec(
                            "words.preview_frame",
                            json!({"kind": kind, "time": ctx.state.lyrics.scrub_seconds}),
                            Some("Rendered EffectCraft lyrics frame"),
                        );
                    }
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-save", "save", "Save", "Save current lyric section edits")).clicked()
                    && let Some((anim_id, rev)) = active_section_revision(ctx)
                {
                    let payload = json!({
                        "animation_id": anim_id,
                        "expected_source_revision": rev,
                        "teaching_mode": ctx.state.lyrics.teaching_mode,
                        "style": {
                            "size": ctx.state.lyrics.font_size,
                            "color": ctx.state.lyrics.color_hex,
                            "outline_color": ctx.state.lyrics.outline_hex,
                        }
                    });
                    ctx.state.dirty_drafts.remove("lyrics");
                    ctx.exec("words.save", payload, Some("Saved lyric section"));
                }
            });
        });

        draw_lyrics_audio_and_track_bar(ui, ctx);

        let captions_open = ctx.state.lyrics.draw_enabled;
        let footer_reserve = if captions_open { 102.0 } else { 40.0 };
        let avail = ui.available_size();
        let max_h = (avail.y - footer_reserve).max(114.0);
        let aspect_w = (avail.x - 2.0).max(180.0);
        let aspect_h = (aspect_w * 9.0 / 16.0).min(max_h);
        let (outer_stage, stage_resp) = ui.allocate_exact_size(Vec2::new(aspect_w, aspect_h), Sense::click());
        let tc_badge = match ctx.state.lyrics.track_source.as_str() {
            "filmcraft" => "V1 · FilmCraft .fcproj",
            "effectcraft" => "V3 · lyrics.ecproj",
            _ => "FC V1 + EC V3 · Composite",
        };
        let stage_rect = paint_filmbar_frame(
            ui,
            outer_stage,
            ctx.pal,
            FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: stage_resp.hovered(), timecode: Some(tc_badge) },
        );
        ctx.reg.record("words-stage", "Lyrics Stage", &stage_resp, None);

        let film_bg = ctx.state.film_frame_at_time(ctx.state.lyrics.scrub_seconds);
        let rendered_frame = ctx.state.cache.last_words_frame.as_deref().or_else(|| {
            if ctx.state.lyrics.backdrop_mode == "scene" {
                active.and_then(|a| a["preview_frame"].as_str().or_else(|| a["lyrics_frame"].as_str()))
            } else {
                active.and_then(|a| a["lyrics_frame"].as_str().or_else(|| a["preview_frame"].as_str()))
            }
        });
        let bg = if ctx.state.lyrics.backdrop_mode == "paper" { ctx.pal.stage_paper } else { ctx.pal.bg_stage };

        if ctx.state.lyrics.track_source == "filmcraft" {
            if !paint_local_image(ui, stage_rect, film_bg.as_deref(), ctx.pal.bg_stage, 4) {
                paint_craft_doc_card(
                    ui,
                    stage_rect.shrink(14.0),
                    ctx.pal,
                    "FC",
                    "film/Song.fcproj",
                    "Click Assemble in Studio or Render Frame to preview FilmCraft V1 track",
                );
            }
        } else if !paint_local_image(ui, stage_rect, rendered_frame, bg, 4) {
            if ctx.state.lyrics.track_source == "overlay" && ctx.state.lyrics.backdrop_mode == "scene" {
                let bg_img = active.and_then(|a| a["background_path"].as_str()).or(film_bg.as_deref());
                let _ = paint_local_image(ui, stage_rect, bg_img, bg, 4);
            } else {
                ui.painter().rect_filled(stage_rect, CornerRadius::same(4), bg);
            }
            paint_real_stage_words(ui, ctx, stage_rect, active);
        }

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let scrub_w = (ui.available_width() - 396.0).clamp(96.0, 260.0);
            ui.allocate_ui_with_layout(Vec2::new(scrub_w, 28.0), Layout::top_down(Align::Min), |ui| {
                if ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-scrub", "Scrub", 0.0..=ctx.state.audio_duration_sec.max(60.0), "s", ctx.pal.accent_emerald),
                    &mut ctx.state.lyrics.scrub_seconds,
                )
                .changed()
                {
                    let s = ctx.state.lyrics.scrub_seconds;
                    ctx.push(UiAction::SeekAudio(s));
                }
            });
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-seed-markers", "pin", "Sync Cues", "Sync section markers into FilmCraft")).clicked() {
                ctx.exec("video.markers", json!({"action": "seed"}), Some("Synced lyric section markers"));
            }
            let is_scene = ctx.state.lyrics.backdrop_mode == "scene";
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-bg-scene", "Scene", "Preview stage render").selected(is_scene)).clicked() {
                ctx.state.lyrics.backdrop_mode = "scene".into();
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-bg-paper", "Alpha", "Preview lyrics.ecproj render").selected(!is_scene)).clicked()
            {
                ctx.state.lyrics.backdrop_mode = "paper".into();
            }
            let cap_lbl = if ctx.state.lyrics.draw_enabled { "C1 & Templates ▴" } else { "C1 & Templates ▾" };
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::label_only("words-captions-drawer-toggle", cap_lbl, "Toggle C1 Broadcast Captions & .ectemplate / .fcgt controls (T06, T26)")
                    .selected(ctx.state.lyrics.draw_enabled),
            )
            .clicked()
            {
                ctx.state.lyrics.draw_enabled = !ctx.state.lyrics.draw_enabled;
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("words-render-studio", "film", "Publish V3", "Build lettering/lyrics.ecproj & publish V3 Kinetic Lyrics").primary(),
            )
            .clicked()
            {
                ctx.exec("words.letter", json!({"render": true, "studio": true}), Some("Published Kinetic Lyrics onto V3"));
            }
        });

        if ctx.state.lyrics.draw_enabled {
            draw_captions_and_template_strip(ui, ctx);
        }
    });
    ctx.reg.record("words-stage-panel", "Stage Panel", &panel.response, None);
}

fn draw_real_lyrics_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anims: &[Value], active: Option<&Value>) {
    let total_dur = ctx.state.audio_duration_sec.max(30.0);
    let cues = ctx.state.cache.words_doc["graphics"].as_array().cloned().unwrap_or_default();
    let active_words = active.and_then(|a| a["words"].as_array()).cloned().unwrap_or_default();
    let has_audio = ctx.state.song_audio_path().is_some();
    let film_frames = ctx.state.active_film_frames();
    let show_fc = ctx.state.lyrics.track_source != "effectcraft";

    let cue_spans: Vec<CraftTimelineSpan> = cues
        .iter()
        .map(|c| {
            let st = c["start"].as_f64().unwrap_or(0.0);
            CraftTimelineSpan { start_sec: st, end_sec: c["end"].as_f64().unwrap_or(st + 1.0), label: String::new() }
        })
        .collect();
    let cue_keys: Vec<f64> = anims.iter().filter_map(|a| a["song_anchor"].as_f64()).collect();
    let audio_spans = if has_audio {
        vec![CraftTimelineSpan {
            start_sec: 0.0,
            end_sec: total_dur,
            label: if ctx.state.lyrics.audio_source == "filmcraft" { "FilmCraft Audio Mix".into() } else { "Song Audio".into() },
        }]
    } else {
        Vec::new()
    };
    let lyric_spans: Vec<CraftTimelineSpan> = anims
        .iter()
        .map(|a| {
            let st = a["song_anchor"].as_f64().unwrap_or(0.0);
            let dur = a["authored_duration"].as_f64().unwrap_or(2.5);
            CraftTimelineSpan { start_sec: st, end_sec: st + dur.max(0.2), label: a["section"].as_str().unwrap_or("Lyrics").to_owned() }
        })
        .collect();
    let word_spans: Vec<CraftTimelineSpan> = active_words
        .iter()
        .take(36)
        .map(|w| {
            let st = w["start"].as_f64().unwrap_or(0.0);
            CraftTimelineSpan { start_sec: st, end_sec: w["end"].as_f64().unwrap_or(st + 0.25), label: w["text"].as_str().unwrap_or("").to_owned() }
        })
        .collect();

    let dirty_v3 = ctx.state.dirty_tracks.contains("V3");
    let top_lane = if show_fc {
        CraftTimelineLane {
            code: "V1",
            title: "FC V1 FILM",
            color: ctx.pal.accent_violet,
            filmbar: true,
            waveform: false,
            dirty: false,
            spans: cue_spans,
            keyframes: &cue_keys,
            frames: &film_frames,
            empty_hint: "FilmCraft V1 sequence",
        }
    } else {
        CraftTimelineLane {
            code: "CUES",
            title: "CUES",
            color: ctx.pal.accent_violet,
            filmbar: false,
            waveform: false,
            dirty: false,
            spans: cue_spans,
            keyframes: &cue_keys,
            frames: &[],
            empty_hint: "Section cue pins",
        }
    };
    let lanes = [
        top_lane,
        CraftTimelineLane {
            code: "AUDIO",
            title: "AUDIO",
            color: ctx.pal.accent_emerald,
            filmbar: false,
            waveform: has_audio,
            dirty: false,
            spans: audio_spans,
            keyframes: &[],
            frames: &[],
            empty_hint: "No audio linked",
        },
        CraftTimelineLane {
            code: "LYRICS",
            title: "EC V3 LYRICS",
            color: ctx.pal.accent_amber,
            filmbar: false,
            waveform: false,
            dirty: dirty_v3,
            spans: lyric_spans,
            keyframes: &[],
            frames: &[],
            empty_hint: "lettering/lyrics.ecproj",
        },
        CraftTimelineLane {
            code: "WORDS",
            title: "WORDS",
            color: ctx.pal.accent_sky,
            filmbar: false,
            waveform: false,
            dirty: false,
            spans: word_spans,
            keyframes: &[],
            frames: &[],
            empty_hint: "Timed words (click Align Audio)",
        },
    ];
    let mut scrub = ctx.state.lyrics.scrub_seconds;
    if draw_craft_timeline(ui, ctx, "words", total_dur, &mut scrub, &lanes) {
        ctx.state.lyrics.scrub_seconds = scrub;
        ctx.push(UiAction::SeekAudio(scrub));
    }
}

fn paint_real_stage_words(ui: &mut Ui, ctx: &mut ViewCtx<'_>, stage_rect: Rect, active: Option<&Value>) {
    let words: Vec<(String, String, bool)> = active
        .and_then(|a| a["words"].as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|w| Some((w["word_id"].as_str().unwrap_or("").to_owned(), w["text"].as_str()?.to_owned(), w["emphasis"].as_bool() == Some(true))))
                .collect()
        })
        .unwrap_or_default();

    if words.is_empty() {
        if let Some(lyrics_str) = ctx.state.cache.song_lyrics["lyrics"].as_str() {
            let lines: Vec<&str> = lyrics_str.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
            if !lines.is_empty() {
                let total = ctx.state.audio_duration_sec.max(24.0);
                let l_idx = (((ctx.state.lyrics.scrub_seconds / total) * (lines.len() as f64)).floor() as usize).min(lines.len().saturating_sub(1));
                let banner = Rect::from_center_size(
                    Pos2::new(stage_rect.center().x, stage_rect.max.y - (stage_rect.height() * 0.22).clamp(28.0, 56.0)),
                    Vec2::new((stage_rect.width() - 36.0).max(120.0), 42.0),
                );
                ui.painter().rect_filled(banner, CornerRadius::same(8), Color32::from_rgba_unmultiplied(8, 12, 18, 212));
                ui.painter().text(banner.center(), Align2::CENTER_CENTER, lines[l_idx], FontId::proportional(17.5), ctx.pal.text_primary);
                return;
            }
        }
        paint_craft_doc_card(
            ui,
            stage_rect.shrink(14.0),
            ctx.pal,
            "EC",
            "lettering/lyrics.ecproj",
            "Click Align Audio then Publish V3 to render kinetic lyrics",
        );
        return;
    }

    let mut cursor = Pos2::new(stage_rect.min.x + 28.0, stage_rect.min.y + 34.0);
    for (idx, (wid, text, emphasis)) in words.iter().enumerate() {
        let font_px = if *emphasis { 28.0 } else { 20.0 };
        let fg = if ctx.state.lyrics.backdrop_mode == "paper" {
            if *emphasis { Color32::from_rgb(0xa3, 0x3b, 0x20) } else { Color32::from_rgb(0x17, 0x3f, 0x49) }
        } else if *emphasis {
            ctx.pal.accent_amber
        } else {
            ctx.pal.text_primary
        };
        let galley = ui.painter().layout_no_wrap(text.clone(), FontId::proportional(font_px), fg);
        if cursor.x + galley.size().x > stage_rect.max.x - 24.0 {
            cursor.x = stage_rect.min.x + 28.0;
            cursor.y += 32.0;
        }
        if cursor.y + 28.0 > stage_rect.max.y - 8.0 {
            break;
        }
        let wrect = Rect::from_min_size(cursor, galley.size() + egui::vec2(10.0, 6.0));
        let selected = ctx.state.lyrics.selected_word_id.as_deref().map_or(idx == 0, |sel| sel == wid.as_str());
        if selected {
            ui.painter().rect_stroke(wrect, CornerRadius::same(5), Stroke::new(1.5, ctx.pal.accent_emerald), StrokeKind::Inside);
        }
        if ctx.state.lyrics.backdrop_mode == "scene" {
            ui.painter().text(
                wrect.center() + egui::vec2(1.5, 1.5),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(font_px),
                Color32::from_rgba_unmultiplied(8, 12, 18, 210),
            );
        }
        ui.painter().text(wrect.center(), Align2::CENTER_CENTER, text, FontId::proportional(font_px), fg);
        let hit = ui.interact(wrect, ui.scope_id().with(("word_box", idx)), Sense::click());
        if hit.clicked() {
            ctx.state.lyrics.selected_word_id = Some(wid.clone());
        }
        ctx.reg.record(format!("word-box-{idx}"), text, &hit, None);
        cursor.x = wrect.max.x + 6.0;
    }
}
