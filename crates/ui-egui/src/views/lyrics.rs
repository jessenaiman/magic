//! `Lyrics` view (`#lyrics-workspace-root`): `EffectCraft` `V3` Kinetic Typography workspace
//! backed by `lettering/lyrics.ecproj` and `lettering/.preview/*.png` rendered frames.
//! Eliminates dummy lyric strings and synthetic splines: stage and moment cards display real
//! rendered frames from `lettering/lyrics.ecproj` and real timed artist words from `words.read`.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    EffectRowSpec, FilmbarOrientation, FilmbarStyle, ScalarSpec, child_pane, effect_row_button,
    fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image, pill_toggle,
    ring_slider_h, rotary_dial,
};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, text_input};

const FALLBACK_LYRICS_FX: &[(&str, &str, &str)] = &[
    ("ec.text.typewriter_range", "EC", "Typewriter Range"),
    ("ec.text.per_word_pop", "EC", "Per-Word Scale Pop"),
    ("ec.text.per_char_3d_flip", "EC", "3D Per-Char Flip"),
    ("ec.stylize.glow", "EC", "Soft Chorus Glow"),
    ("ec.perspective.dropshadow", "EC", "Drop Shadow"),
    ("vc.envelope.arc", "VC", "Envelope Arc Warp"),
    ("vc.path.offset_halo", "VC", "Offset Sticker Halo"),
    ("pc.layerStyle.stroke_halo", "PC", "Outer Stroke Halo"),
    ("pc.filter.paper_grain", "PC", "Paper Grain Press"),
];

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let body = ui.available_rect_before_wrap();
    if let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(248.0), 8.0, 180.0) {
        let mut rail = child_pane(ui, left_split.first, "lyrics_moments");
        draw_moment_rail(&mut rail, ctx);
        if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(312.0), 8.0, 220.0) {
            let mut center = child_pane(ui, right_split.first, "lyrics_stage");
            draw_stage_and_timeline(&mut center, ctx);
            let mut insp = child_pane(ui, right_split.second, "lyrics_insp");
            draw_inspector(&mut insp, ctx);
        } else {
            let mut center = child_pane(ui, left_split.second, "lyrics_stage");
            draw_stage_and_timeline(&mut center, ctx);
        }
        let _ = ui.allocate_rect(body, Sense::hover());
    } else {
        draw_stage_and_timeline(ui, ctx);
    }
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
            ui.label(RichText::new("Lyric moments").size(12.5).strong());
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
        ui.separator();

        let query = ctx.state.lyrics.search.trim().to_ascii_lowercase();
        ScrollArea::vertical()
            .id_salt("words_moments_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if anims.is_empty() {
                    ui.label(
                        RichText::new("No timed lyric sections in lettering/lyrics.ecproj yet — transcribe song audio or click ↻.")
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
                        .take(7)
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

                    ui.label(
                        RichText::new(sec_name.to_ascii_uppercase())
                            .monospace()
                            .size(9.5)
                            .color(ctx.pal.text_muted),
                    );
                    let card_w = ui.available_width().max(170.0);
                    let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 72.0), Sense::click());
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
                    let img_r = Rect::from_min_size(Pos2::new(crect.min.x + 6.0, crect.min.y + 6.0), Vec2::new(48.0, 32.0));
                    if !paint_local_image(ui, img_r, preview_frame, ctx.pal.bg_stage, 4) {
                        ui.painter().rect_filled(img_r, CornerRadius::same(4), ctx.pal.bg_stage);
                        ui.painter().text(img_r.center(), Align2::CENTER_CENTER, "EC", FontId::monospace(9.0), ctx.pal.accent_violet);
                    }
                    ui.painter().text(
                        Pos2::new(img_r.max.x + 6.0, crect.min.y + 15.0),
                        Align2::LEFT_CENTER,
                        format!("[0:{anchor:04.1}] {}", truncate_words(&preview_line, 18)),
                        FontId::proportional(10.5),
                        ctx.pal.text_primary,
                    );
                    let mode_tag = anim["teaching_mode"].as_str().unwrap_or("sing_along");
                    ui.painter().text(
                        Pos2::new(img_r.max.x + 6.0, crect.min.y + 30.0),
                        Align2::LEFT_CENTER,
                        mode_tag,
                        FontId::monospace(9.0),
                        ctx.pal.accent_amber,
                    );
                    ui.painter().text(
                        Pos2::new(crect.min.x + 6.0, crect.max.y - 12.0),
                        Align2::LEFT_CENTER,
                        format!("0:{anchor:04.1}  {}", truncate_words(&preview_line, 24)),
                        FontId::proportional(9.5),
                        ctx.pal.text_secondary,
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
        "lettering/lyrics.ecproj (not built yet — click Letter / Publish V3)".to_owned()
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
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("words-preview-frame", "image", "Render Frame", "Render lettering/lyrics.ecproj frame at scrub time"),
                )
                .clicked()
                {
                    let kind = if ctx.state.lyrics.backdrop_mode == "scene" { "stage" } else { "lyrics" };
                    ctx.exec(
                        "words.preview_frame",
                        json!({"kind": kind, "time": ctx.state.lyrics.scrub_seconds}),
                        Some("Rendered EffectCraft lyrics frame"),
                    );
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
                pill_toggle(ui, ctx.reg, ctx.pal, "words-draw-toggle", "Draw", &mut ctx.state.lyrics.draw_enabled, ctx.pal.accent_emerald);
                let is_design = ctx.state.lyrics.sub_mode == "design";
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-mode-motion", "sparkles", "Motion", "Word entrance & range mode").selected(!is_design)).clicked() {
                    ctx.state.lyrics.sub_mode = "motion".into();
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-mode-design", "type", "Lyrics", "Typography & layout mode").selected(is_design)).clicked() {
                    ctx.state.lyrics.sub_mode = "design".into();
                }
            });
        });

        // Horizontal 35mm filmbar scene strip showing real section frames + EffectCraft launcher
        ui.horizontal(|ui| {
            ui.label(RichText::new("Sections").size(10.5).color(ctx.pal.text_muted));
            let strip_w = (ui.available_width() - 108.0).max(160.0);
            let (strip_rect, sresp) = ui.allocate_exact_size(Vec2::new(strip_w, 38.0), Sense::click());
            let strip_inner = paint_filmbar_frame(
                ui,
                strip_rect,
                ctx.pal,
                FilmbarStyle {
                    orientation: FilmbarOrientation::Horizontal,
                    selected: false,
                    hovered: sresp.hovered(),
                    timecode: None,
                },
            );
            let slots_n = anims.len().clamp(1, 8);
            let cell_w = (strip_inner.width() / (slots_n as f32)).max(32.0);
            for (i, anim) in anims.iter().take(slots_n).enumerate() {
                let cell = Rect::from_min_size(
                    Pos2::new(strip_inner.min.x + (i as f32) * cell_w + 2.0, strip_inner.min.y + 1.0),
                    Vec2::new(cell_w - 4.0, strip_inner.height() - 2.0),
                );
                let fpath = anim["preview_frame"]
                    .as_str()
                    .or_else(|| anim["lyrics_frame"].as_str())
                    .or_else(|| anim["background_path"].as_str());
                let is_sel = active.and_then(|a| a["animation_id"].as_str()) == anim["animation_id"].as_str();
                let _ = paint_local_image(ui, cell, fpath, ctx.pal.bg_stage, 2);
                ui.painter().rect_stroke(
                    cell,
                    CornerRadius::same(2),
                    Stroke::new(1.0, if is_sel { ctx.pal.accent_emerald } else { ctx.pal.border_subtle }),
                    StrokeKind::Inside,
                );
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("words-open-effectcraft", "external", "EffectCraft", "Open lettering/lyrics.ecproj in EffectCraft"),
            )
            .clicked()
            {
                ctx.exec("workspace.open", json!({"app": "effectcraft", "domain": "lyrics"}), Some("Opened lettering/lyrics.ecproj"));
            }
        });

        // 16:9 Kinetic Lyrics Stage framed with horizontal 35mm filmbars
        let avail = ui.available_size();
        let aspect_w = avail.x.max(220.0);
        let aspect_h = (aspect_w * 9.0 / 16.0).min((avail.y - 118.0).max(148.0));
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

        let rendered_frame = ctx
            .state
            .cache
            .last_words_frame
            .as_deref()
            .or_else(|| {
                if ctx.state.lyrics.backdrop_mode == "scene" {
                    active.and_then(|a| a["preview_frame"].as_str().or_else(|| a["lyrics_frame"].as_str()))
                } else {
                    active.and_then(|a| a["lyrics_frame"].as_str().or_else(|| a["preview_frame"].as_str()))
                }
            });
        let bg = if ctx.state.lyrics.backdrop_mode == "paper" {
            ctx.pal.stage_paper
        } else {
            ctx.pal.bg_stage
        };
        if !paint_local_image(ui, stage_rect, rendered_frame, bg, 4) {
            let bg_img = active.and_then(|a| a["background_path"].as_str());
            if ctx.state.lyrics.backdrop_mode == "scene" {
                let _ = paint_local_image(ui, stage_rect, bg_img, bg, 4);
            }
            paint_real_stage_words(ui, ctx, stage_rect, active);
        }

        // Transport & Stage bar
        ui.horizontal_wrapped(|ui| {
            let scrub_w = (ui.available_width() - 340.0).max(110.0);
            ui.allocate_ui(Vec2::new(scrub_w, 28.0), |ui| {
                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-scrub", "Scrub", 0.0..=ctx.state.audio_duration_sec.max(60.0), "s", ctx.pal.accent_emerald),
                    &mut ctx.state.lyrics.scrub_seconds,
                );
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

        draw_real_lyrics_timeline(ui, ctx, &anims, active);
    });
    ctx.reg.record("words-stage-panel", "Stage Panel", &panel.response, None);
}

fn draw_real_lyrics_timeline(ui: &mut Ui, ctx: &ViewCtx<'_>, anims: &[Value], active: Option<&Value>) {
    let total_dur = ctx.state.audio_duration_sec.max(30.0);
    let cues = ctx.state.cache.words_doc["graphics"].as_array().cloned().unwrap_or_default();
    let active_words = active.and_then(|a| a["words"].as_array()).cloned().unwrap_or_default();

    for (lane, accent) in [
        ("CUES", ctx.pal.accent_sky),
        ("AUDIO", ctx.pal.text_muted),
        ("LYRICS", ctx.pal.accent_emerald),
        ("WORDS", ctx.pal.accent_amber),
    ] {
        ui.horizontal(|ui| {
            ui.allocate_ui(Vec2::new(48.0, 14.0), |ui| {
                ui.label(RichText::new(lane).monospace().size(9.0).color(ctx.pal.text_muted));
            });
            let lw = ui.available_width().max(80.0);
            let (lrect, _) = ui.allocate_exact_size(Vec2::new(lw, 13.0), Sense::hover());
            ui.painter().rect_filled(lrect, CornerRadius::same(3), ctx.pal.bg_input);
            match lane {
                "CUES" => {
                    for c in &cues {
                        let st = c["start"].as_f64().unwrap_or(0.0);
                        let en = c["end"].as_f64().unwrap_or(st + 1.0);
                        let x0 = lrect.min.x + ((st / total_dur).clamp(0.0, 1.0) as f32) * lw;
                        let w = (((en - st).max(0.2) / total_dur) as f32 * lw).max(6.0);
                        ui.painter().rect_filled(
                            Rect::from_min_size(Pos2::new(x0, lrect.min.y + 2.0), Vec2::new(w.min(lrect.max.x - x0), 9.0)),
                            CornerRadius::same(2),
                            accent.gamma_multiply(0.6),
                        );
                    }
                }
                "AUDIO" => {
                    if ctx.state.song_audio_path().is_some() {
                        ui.painter().rect_filled(lrect.shrink2(Vec2::new(2.0, 3.0)), CornerRadius::same(2), accent.gamma_multiply(0.35));
                    }
                }
                "LYRICS" => {
                    for a in anims {
                        let st = a["song_anchor"].as_f64().unwrap_or(0.0);
                        let dur = a["authored_duration"].as_f64().unwrap_or(2.5);
                        let x0 = lrect.min.x + ((st / total_dur).clamp(0.0, 1.0) as f32) * lw;
                        let w = ((dur.max(0.2) / total_dur) as f32 * lw).max(8.0);
                        ui.painter().rect_filled(
                            Rect::from_min_size(Pos2::new(x0, lrect.min.y + 2.0), Vec2::new(w.min(lrect.max.x - x0), 9.0)),
                            CornerRadius::same(2),
                            accent.gamma_multiply(0.55),
                        );
                    }
                }
                _ => {
                    for w in active_words.iter().take(32) {
                        let st = w["start"].as_f64().unwrap_or(0.0);
                        let en = w["end"].as_f64().unwrap_or(st + 0.25);
                        let x0 = lrect.min.x + ((st / total_dur).clamp(0.0, 1.0) as f32) * lw;
                        let pw = (((en - st).max(0.12) / total_dur) as f32 * lw).max(4.0);
                        ui.painter().rect_filled(
                            Rect::from_min_size(Pos2::new(x0, lrect.min.y + 2.0), Vec2::new(pw.min(lrect.max.x - x0), 9.0)),
                            CornerRadius::same(2),
                            accent.gamma_multiply(0.65),
                        );
                    }
                }
            }
        });
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
        paint_craft_doc_card(
            ui,
            stage_rect.shrink(16.0),
            ctx.pal,
            "EC",
            "lettering/lyrics.ecproj",
            "Click Publish V3 or Render Frame once song lyrics are timed",
        );
        return;
    }

    let mut cursor = Pos2::new(stage_rect.min.x + 28.0, stage_rect.min.y + 34.0);
    for (idx, (wid, text, emphasis)) in words.iter().enumerate() {
        let font_px = if *emphasis { 28.0 } else { 20.0 };
        let fg = if ctx.state.lyrics.backdrop_mode == "paper" {
            if *emphasis {
                Color32::from_rgb(0xa3, 0x3b, 0x20)
            } else {
                Color32::from_rgb(0x17, 0x3f, 0x49)
            }
        } else if *emphasis {
            ctx.pal.accent_amber
        } else {
            ctx.pal.text_primary
        };
        let galley = ui
            .painter()
            .layout_no_wrap(text.clone(), FontId::proportional(font_px), fg);
        if cursor.x + galley.size().x > stage_rect.max.x - 24.0 {
            cursor.x = stage_rect.min.x + 28.0;
            cursor.y += 32.0;
        }
        if cursor.y + 28.0 > stage_rect.max.y - 8.0 {
            break;
        }
        let wrect = Rect::from_min_size(cursor, galley.size() + egui::vec2(10.0, 6.0));
        let selected = ctx
            .state
            .lyrics
            .selected_word_id
            .as_deref()
            .map_or(idx == 0, |sel| sel == wid.as_str());
        if selected {
            ui.painter().rect_stroke(
                wrect,
                CornerRadius::same(5),
                Stroke::new(1.5, ctx.pal.accent_emerald),
                StrokeKind::Inside,
            );
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
        ui.painter()
            .text(wrect.center(), Align2::CENTER_CENTER, text, FontId::proportional(font_px), fg);
        let hit = ui.interact(wrect, ui.scope_id().with(("word_box", idx)), Sense::click());
        if hit.clicked() {
            ctx.state.lyrics.selected_word_id = Some(wid.clone());
        }
        ctx.reg.record(format!("word-box-{idx}"), text, &hit, None);
        cursor.x = wrect.max.x + 6.0;
    }
}

fn draw_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ScrollArea::vertical()
            .id_salt("lyrics_inspector_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(RichText::new("Transform & Skew").size(12.5).strong());
                ui.horizontal(|ui| {
                    if rotary_dial(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        ScalarSpec::new("words-item-rotate", "Rotate", -180.0..=180.0, "°", ctx.pal.accent_amber),
                        &mut ctx.state.lyrics.rotate_deg,
                    )
                    .changed()
                    {
                        ctx.state.dirty_drafts.insert("lyrics".into());
                    }
                    rotary_dial(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        ScalarSpec::new("words-skew-slant", "Skew Slant", -70.0..=70.0, "°", ctx.pal.accent_sky),
                        &mut ctx.state.lyrics.skew_slant_deg,
                    );
                    rotary_dial(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        ScalarSpec::new("words-skew-axis", "Skew Axis", -180.0..=180.0, "°", ctx.pal.accent_emerald),
                        &mut ctx.state.lyrics.skew_axis_deg,
                    );
                });

                ui.horizontal_wrapped(|ui| {
                    for deg in [-90.0, -15.0, 0.0, 15.0, 90.0] {
                        let lbl = if deg > 0.0 { format!("+{deg:.0}°") } else { format!("{deg:.0}°") };
                        let id = format!("words-rot-preset-{deg}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, &lbl, "Set word rotation angle")).clicked() {
                            ctx.state.lyrics.rotate_deg = deg;
                        }
                    }
                });

                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-scale-x", "Stretch X", 25.0..=250.0, "%", ctx.pal.accent_amber),
                    &mut ctx.state.lyrics.scale_x,
                );
                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-scale-y", "Stretch Y", 25.0..=250.0, "%", ctx.pal.accent_amber),
                    &mut ctx.state.lyrics.scale_y,
                );
                pill_toggle(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    "words-3d-per-char",
                    "EffectCraft 3D Per-Character Layer",
                    &mut ctx.state.lyrics.per_char_3d,
                    ctx.pal.accent_emerald,
                );

                ui.separator();
                ui.label(RichText::new("T Lettering & Craft Effects (words.effects)").size(12.0).strong());
                ui.horizontal_wrapped(|ui| {
                    for preset in ["Song font", "Song bold", "Serif italic", "Sans quiet"] {
                        let id = format!("words-font-{preset}");
                        let sel = ctx.state.lyrics.font_preset == preset;
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, preset, "Select font style").selected(sel)).clicked() {
                            ctx.state.lyrics.font_preset = preset.into();
                        }
                    }
                });
                ui.add_space(3.0);
                draw_mapped_lyrics_effects(ui, ctx);
                ui.add_space(4.0);

                if ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-item-size", "Font Size", 40.0..=240.0, "px", ctx.pal.accent_emerald),
                    &mut ctx.state.lyrics.font_size,
                )
                .changed()
                {
                    ctx.state.dirty_drafts.insert("lyrics".into());
                }
                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-item-tracking", "Tracking", -80.0..=300.0, "", ctx.pal.accent_sky),
                    &mut ctx.state.lyrics.tracking,
                );
                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-glow-radius", "Glow Radius", 0.0..=64.0, "px", ctx.pal.accent_amber),
                    &mut ctx.state.lyrics.glow_radius,
                );
                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("words-item-stroke-width", "Outline Halo", 0.0..=24.0, "px", ctx.pal.accent_amber),
                    &mut ctx.state.lyrics.stroke_width,
                );
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Fill").size(10.5).color(ctx.pal.text_muted));
                    text_input(ui, ctx.reg, "words-item-color", "Fill hex", &mut ctx.state.lyrics.color_hex, "#f0f4fc", 84.0);
                    ui.label(RichText::new("Outline").size(10.5).color(ctx.pal.text_muted));
                    text_input(ui, ctx.reg, "words-item-outline", "Outline hex", &mut ctx.state.lyrics.outline_hex, "#0e141e", 84.0);
                });
            });
    });
    ctx.reg.record("words-effects-rail", "Lyrics Inspector", &panel.response, None);
}

fn draw_mapped_lyrics_effects(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let dynamic_fx = ctx
        .state
        .cache
        .words_doc
        .pointer("/tools/craft_effects")
        .or_else(|| ctx.state.cache.words_effects.get("effects"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let items: Vec<(String, String, String)> = if dynamic_fx.is_empty() {
        FALLBACK_LYRICS_FX
            .iter()
            .map(|(id, badge, label)| ((*id).to_owned(), (*badge).to_owned(), (*label).to_owned()))
            .collect()
    } else {
        dynamic_fx
            .iter()
            .filter_map(|fx| {
                Some((
                    fx["id"].as_str()?.to_owned(),
                    fx["badge"].as_str().unwrap_or("EC").to_owned(),
                    fx["label"].as_str()?.to_owned(),
                ))
            })
            .collect()
    };

    for (fx_id, badge, label) in items {
        let btn_id = format!("words-fx-{fx_id}");
        let sel = ctx.state.lyrics.selected_effect_id == fx_id;
        if effect_row_button(
            ui,
            ctx.reg,
            ctx.pal,
            EffectRowSpec {
                id: &btn_id,
                badge: &badge,
                label: &label,
                effect_id: &fx_id,
                selected: sel,
            },
        )
        .clicked()
        {
            ctx.state.lyrics.selected_effect_id = fx_id.clone();
            ctx.exec("words.effect_install", json!({"name": fx_id}), Some("Selected Craft typography effect"));
        }
        ui.add_space(2.0);
    }
}
