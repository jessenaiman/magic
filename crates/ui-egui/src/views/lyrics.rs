//! `Lyrics` view (`#lyrics-workspace-root`): `EffectCraft` `V3` Kinetic Typography workspace
//! matching `05-lyrics-v3-effectcraft.jpg`, backed by `lettering/lyrics.ecproj`,
//! `lettering/.preview/*.png`, and `Sheet 08` Zone C (`lyrics_inspector.rs`: `T06`, `T21`–`T26`).

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, ScalarSpec, child_pane, fill_card, paint_craft_doc_card,
    paint_filmbar_frame, paint_local_image, ring_slider_h,
};
use crate::state::ViewCtx;
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::views::lyrics_inspector::{
    draw_captions_and_template_strip, draw_font_shaping_strip, draw_lyrics_inspector,
};
use crate::widgets::{BtnSpec, action_button, text_input};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let body = ui.available_rect_before_wrap();
    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(252.0), 8.0, 180.0)
    {
        let mut rail = child_pane(ui, left_split.first, "lyrics_moments");
        draw_moment_rail(&mut rail, ctx);
        left_split.second
    } else {
        body
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(316.0), 8.0, 220.0)
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
    let chosen = ctx
        .state
        .lyrics
        .selected_anim_id
        .as_deref()
        .and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id)))
        .or_else(|| anims.first())?;
    let id = chosen["animation_id"].as_str()?.to_owned();
    let rev = chosen["source_revision"].as_u64().unwrap_or(0);
    Some((id, rev))
}

fn draw_moment_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx
        .state
        .cache
        .words_doc
        .pointer("/document/animations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Lyric Moments").size(12.5).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("lettering/lyrics.ecproj").monospace().size(9.5).color(ctx.pal.text_muted));
            });
        });
        text_input(
            ui,
            ctx.reg,
            "words-lyrics-search",
            "Search lyrics",
            &mut ctx.state.lyrics.search,
            "Filter lyrics…",
            ui.available_width().max(160.0),
        );
        draw_font_shaping_strip(ui, ctx);
        ui.separator();

        let query = ctx.state.lyrics.search.trim().to_ascii_lowercase();
        ScrollArea::vertical()
            .id_salt("words_moments_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if anims.is_empty() {
                    ui.label(
                        RichText::new("No timed lyric sections in lettering/lyrics.ecproj yet — click Align Audio or ↻.")
                            .size(11.0)
                            .color(ctx.pal.text_muted),
                    );
                    return;
                }
                for (idx, anim) in anims.iter().enumerate() {
                    let id = anim["animation_id"].as_str().unwrap_or("sec");
                    let sec_name = anim["scope"]["section"].as_str().unwrap_or(id);
                    let anchor = anim["song_anchor"].as_f64().unwrap_or(0.0);
                    let preview_frame = anim["preview_frame"]
                        .as_str()
                        .or_else(|| anim["lyrics_frame"].as_str())
                        .or_else(|| anim["background_path"].as_str());
                    let preview_line: String = anim["words"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|w| w["text"].as_str())
                        .take(8)
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !query.is_empty()
                        && !preview_line.to_ascii_lowercase().contains(&query)
                        && !sec_name.to_ascii_lowercase().contains(&query)
                    {
                        continue;
                    }
                    let selected = ctx
                        .state
                        .lyrics
                        .selected_anim_id
                        .as_deref()
                        .map_or(idx == 0, |sel| sel == id);

                    let card_w = ui.available_width().max(170.0);
                    let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 76.0), Sense::click());
                    let border = if selected {
                        Stroke::new(1.6, ctx.pal.accent_emerald)
                    } else if resp.hovered() {
                        Stroke::new(1.0, ctx.pal.accent_sky)
                    } else {
                        Stroke::new(1.0, ctx.pal.border_subtle)
                    };
                    ui.painter().rect(
                        crect,
                        CornerRadius::same(8),
                        if selected { ctx.pal.accent_emerald_bg } else { ctx.pal.bg_input },
                        border,
                        StrokeKind::Inside,
                    );
                    let img_r = Rect::from_min_size(Pos2::new(crect.max.x - 68.0, crect.min.y + 8.0), Vec2::new(60.0, 60.0));
                    if !paint_local_image(ui, img_r, preview_frame, ctx.pal.bg_stage, 5) {
                        ui.painter().rect_filled(img_r, CornerRadius::same(5), ctx.pal.bg_stage);
                        ui.painter().text(img_r.center(), Align2::CENTER_CENTER, "EC", FontId::monospace(9.5), ctx.pal.accent_violet);
                    }
                    let dot_col = if selected { ctx.pal.accent_emerald } else { ctx.pal.accent_amber };
                    ui.painter().circle_filled(Pos2::new(crect.min.x + 11.0, crect.min.y + 15.0), 3.0, dot_col);
                    ui.painter().text(
                        Pos2::new(crect.min.x + 19.0, crect.min.y + 15.0),
                        Align2::LEFT_CENTER,
                        format!("0:{anchor:04.1}"),
                        FontId::monospace(9.5),
                        ctx.pal.text_muted,
                    );
                    ui.painter().text(
                        Pos2::new(crect.min.x + 10.0, crect.min.y + 42.0),
                        Align2::LEFT_CENTER,
                        truncate_words(&preview_line, 21),
                        FontId::proportional(11.5),
                        ctx.pal.text_primary,
                    );
                    if resp.clicked() {
                        ctx.state.lyrics.selected_anim_id = Some(id.to_owned());
                        ctx.state.lyrics.scrub_seconds = anchor;
                    }
                    ctx.reg.record(format!("words-moment-{id}-0"), &preview_line, &resp, None);
                    ui.add_space(6.0);
                }
            });
    });
    ctx.reg.record("words-lyrics", "Lyric Moments", &panel.response, None);
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
    let anims = ctx
        .state
        .cache
        .words_doc
        .pointer("/document/animations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let active = ctx
        .state
        .lyrics
        .selected_anim_id
        .as_deref()
        .and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id)))
        .or_else(|| anims.first());
    let ecproj_meta = &ctx.state.cache.words_doc["ecproj"];
    let sec_heading = active
        .and_then(|a| a["scope"]["section"].as_str().or_else(|| a["animation_id"].as_str()))
        .unwrap_or("Lyrics Stage");
    let ecproj_summary = if ecproj_meta.is_object() {
        let schema = ecproj_meta["schema"].as_u64().unwrap_or(0);
        let layers = ecproj_meta["layer_count"].as_u64().unwrap_or(0);
        format!("lettering/lyrics.ecproj · schema {schema} · {layers} layers")
    } else {
        "lettering/lyrics.ecproj (click Publish V3 to build)".to_owned()
    };

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(sec_heading).size(12.5).strong());
            ui.label(RichText::new(ecproj_summary).monospace().size(10.0).color(ctx.pal.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-reload", "refresh", "Reload words.read & words.effects")).clicked() {
                    ctx.exec("words.read", json!({}), None);
                    ctx.exec("words.effects", json!({}), None);
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-align-audio", "music", "Align Audio", "Run speech alignment on song audio")).clicked() {
                    ctx.exec("audio.analyze", json!({}), Some("Aligned song audio"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-preview-frame", "image", "Render Frame", "Render lettering/lyrics.ecproj frame at scrub time")).clicked() {
                    let kind = if ctx.state.lyrics.backdrop_mode == "scene" { "stage" } else { "lyrics" };
                    ctx.exec("words.preview_frame", json!({"kind": kind, "time": ctx.state.lyrics.scrub_seconds}), Some("Rendered EffectCraft lyrics frame"));
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

        ui.horizontal(|ui| {
            let strip_w = (ui.available_width() - 104.0).max(160.0);
            let slots_n = anims.len().clamp(1, 6);
            let cell_w = ((strip_w - ((slots_n.saturating_sub(1)) as f32) * 4.0) / (slots_n as f32)).max(44.0);
            for (i, anim) in anims.iter().take(slots_n).enumerate() {
                let (cell_rect, cresp) = ui.allocate_exact_size(Vec2::new(cell_w, 44.0), Sense::click());
                let is_sel = active.and_then(|a| a["animation_id"].as_str()) == anim["animation_id"].as_str()
                    || (active.is_none() && i == 0);
                let cell_inner = paint_filmbar_frame(
                    ui,
                    cell_rect,
                    ctx.pal,
                    FilmbarStyle {
                        orientation: FilmbarOrientation::Horizontal,
                        selected: is_sel,
                        hovered: cresp.hovered(),
                        timecode: None,
                    },
                );
                let fpath = anim["preview_frame"]
                    .as_str()
                    .or_else(|| anim["lyrics_frame"].as_str())
                    .or_else(|| anim["background_path"].as_str());
                let _ = paint_local_image(ui, cell_inner, fpath, ctx.pal.bg_stage, 2);
                if cresp.clicked()
                    && let Some(aid) = anim["animation_id"].as_str()
                {
                    ctx.state.lyrics.selected_anim_id = Some(aid.to_owned());
                    ctx.state.lyrics.scrub_seconds = anim["song_anchor"].as_f64().unwrap_or(0.0);
                }
            }
            if anims.is_empty() {
                let (empty_r, _) = ui.allocate_exact_size(Vec2::new(strip_w, 44.0), Sense::hover());
                let _ = paint_filmbar_frame(
                    ui,
                    empty_r,
                    ctx.pal,
                    FilmbarStyle {
                        orientation: FilmbarOrientation::Horizontal,
                        selected: false,
                        hovered: false,
                        timecode: None,
                    },
                );
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-open-effectcraft", "external", "EffectCraft", "Open lettering/lyrics.ecproj in EffectCraft")).clicked() {
                ctx.exec("workspace.open", json!({"app": "effectcraft", "domain": "lyrics"}), Some("Opened lettering/lyrics.ecproj"));
            }
        });

        let avail = ui.available_size();
        let dock_reserve = if ctx.state.show_bottom_dock { 164.0 } else { 72.0 };
        let aspect_w = avail.x.max(220.0);
        let aspect_h = (aspect_w * 9.0 / 16.0).min((avail.y - dock_reserve).max(136.0));
        let (outer_stage, stage_resp) = ui.allocate_exact_size(Vec2::new(aspect_w, aspect_h), Sense::click());
        let stage_rect = paint_filmbar_frame(
            ui,
            outer_stage,
            ctx.pal,
            FilmbarStyle {
                orientation: FilmbarOrientation::Horizontal,
                selected: false,
                hovered: stage_resp.hovered(),
                timecode: Some("V3 · lyrics.ecproj"),
            },
        );
        ctx.reg.record("words-stage", "Lyrics Stage", &stage_resp, None);

        let rendered_frame = ctx.state.cache.last_words_frame.as_deref().or_else(|| {
            if ctx.state.lyrics.backdrop_mode == "scene" {
                active.and_then(|a| a["preview_frame"].as_str().or_else(|| a["lyrics_frame"].as_str()))
            } else {
                active.and_then(|a| a["lyrics_frame"].as_str().or_else(|| a["preview_frame"].as_str()))
            }
        });
        let bg = if ctx.state.lyrics.backdrop_mode == "paper" { ctx.pal.stage_paper } else { ctx.pal.bg_stage };
        if !paint_local_image(ui, stage_rect, rendered_frame, bg, 4) {
            let bg_img = active.and_then(|a| a["background_path"].as_str());
            if ctx.state.lyrics.backdrop_mode == "scene" {
                let _ = paint_local_image(ui, stage_rect, bg_img, bg, 4);
            }
            paint_real_stage_words(ui, ctx, stage_rect, active);
        }

        ui.horizontal_wrapped(|ui| {
            let scrub_w = (ui.available_width() - 340.0).max(110.0);
            ui.allocate_ui_with_layout(Vec2::new(scrub_w, 30.0), Layout::top_down(Align::Min), |ui| {
                ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("words-scrub", "Scrub", 0.0..=ctx.state.audio_duration_sec.max(60.0), "s", ctx.pal.accent_emerald), &mut ctx.state.lyrics.scrub_seconds);
            });
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-seed-markers", "pin", "Sync Cues", "Sync section markers into FilmCraft")).clicked() {
                ctx.exec("video.markers", json!({"action": "seed"}), Some("Synced lyric section markers"));
            }
            let is_scene = ctx.state.lyrics.backdrop_mode == "scene";
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-bg-scene", "Scene", "Preview stage render").selected(is_scene)).clicked() {
                ctx.state.lyrics.backdrop_mode = "scene".into();
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-bg-paper", "Lyrics Alpha", "Preview lyrics.ecproj render").selected(!is_scene)).clicked() {
                ctx.state.lyrics.backdrop_mode = "paper".into();
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-render-studio", "film", "Publish V3", "Build lettering/lyrics.ecproj & publish V3 Kinetic Lyrics").primary()).clicked() {
                ctx.exec("words.letter", json!({"render": true, "studio": true}), Some("Published Kinetic Lyrics onto V3"));
            }
        });

        draw_captions_and_template_strip(ui, ctx);
        if ctx.state.show_bottom_dock {
            draw_real_lyrics_timeline(ui, ctx, &anims, active);
        }
    });
    ctx.reg.record("words-stage-panel", "Stage Panel", &panel.response, None);
}

fn draw_real_lyrics_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anims: &[Value], active: Option<&Value>) {
    let total_dur = ctx.state.audio_duration_sec.max(30.0);
    let cues = ctx.state.cache.words_doc["graphics"].as_array().cloned().unwrap_or_default();
    let active_words = active.and_then(|a| a["words"].as_array()).cloned().unwrap_or_default();
    let has_audio = ctx.state.song_audio_path().is_some();

    let cue_spans: Vec<CraftTimelineSpan> = cues
        .iter()
        .map(|c| {
            let st = c["start"].as_f64().unwrap_or(0.0);
            CraftTimelineSpan {
                start_sec: st,
                end_sec: c["end"].as_f64().unwrap_or(st + 1.0),
                label: String::new(),
            }
        })
        .collect();
    let audio_spans = if has_audio {
        vec![CraftTimelineSpan {
            start_sec: 0.0,
            end_sec: total_dur,
            label: "Song Audio".into(),
        }]
    } else {
        Vec::new()
    };
    let lyric_spans: Vec<CraftTimelineSpan> = anims
        .iter()
        .map(|a| {
            let st = a["song_anchor"].as_f64().unwrap_or(0.0);
            let dur = a["authored_duration"].as_f64().unwrap_or(2.5);
            CraftTimelineSpan {
                start_sec: st,
                end_sec: st + dur.max(0.2),
                label: a["section"].as_str().unwrap_or("Lyrics").to_owned(),
            }
        })
        .collect();
    let word_spans: Vec<CraftTimelineSpan> = active_words
        .iter()
        .take(36)
        .map(|w| {
            let st = w["start"].as_f64().unwrap_or(0.0);
            CraftTimelineSpan {
                start_sec: st,
                end_sec: w["end"].as_f64().unwrap_or(st + 0.25),
                label: w["text"].as_str().unwrap_or("").to_owned(),
            }
        })
        .collect();

    let dirty_v3 = ctx.state.dirty_tracks.contains("V3");
    let lanes = [
        CraftTimelineLane { code: "CUES", title: "CUES", color: ctx.pal.accent_violet, filmbar: false, dirty: false, spans: cue_spans, frames: &[], empty_hint: "" },
        CraftTimelineLane { code: "AUDIO", title: "AUDIO", color: ctx.pal.accent_emerald, filmbar: false, dirty: false, spans: audio_spans, frames: &[], empty_hint: "No audio" },
        CraftTimelineLane { code: "LYRICS", title: "LYRICS", color: ctx.pal.accent_amber, filmbar: false, dirty: dirty_v3, spans: lyric_spans, frames: &[], empty_hint: "lettering/lyrics.ecproj" },
        CraftTimelineLane { code: "WORDS", title: "WORDS", color: ctx.pal.accent_sky, filmbar: false, dirty: false, spans: word_spans, frames: &[], empty_hint: "" },
    ];
    let mut scrub = ctx.state.lyrics.scrub_seconds;
    if draw_craft_timeline(ui, ctx, "words", total_dur, &mut scrub, &lanes) {
        ctx.state.lyrics.scrub_seconds = scrub;
    }
}

fn paint_real_stage_words(ui: &mut Ui, ctx: &mut ViewCtx<'_>, stage_rect: Rect, active: Option<&Value>) {
    let words: Vec<(String, String, bool)> = active
        .and_then(|a| a["words"].as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|w| {
                    Some((
                        w["word_id"].as_str().unwrap_or("").to_owned(),
                        w["text"].as_str()?.to_owned(),
                        w["emphasis"].as_bool() == Some(true),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    if words.is_empty() {
        paint_craft_doc_card(ui, stage_rect.shrink(16.0), ctx.pal, "EC", "lettering/lyrics.ecproj", "Click Publish V3 or Render Frame once song lyrics are timed");
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
            ui.painter().text(wrect.center() + egui::vec2(1.5, 1.5), Align2::CENTER_CENTER, text, FontId::proportional(font_px), Color32::from_rgba_unmultiplied(8, 12, 18, 210));
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
