//! `Lyrics` view (`#lyrics-workspace-root`): `EffectCraft` `V3` Kinetic Typography workspace
//! featuring a 35mm horizontal **filmbar** scene strip, spline word stage, multi-lane timeline,
//! 360° rotary `Rotate / Skew Slant / Skew Axis` dials, hollow-ring inline-value sliders, and
//! CLI-mapped `EffectCraft` / `VectorCraft` / `PhotoCraft` typography effects (`words.effects`).

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, ScalarSpec, paint_filmbar_frame, pill_toggle, ring_slider_h, rotary_dial};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, text_input};

const FALLBACK_LYRICS_FX: &[(&str, &str, &str)] = &[
    ("ec.text.typewriter_range", "EC", "Typewriter Range"),
    ("ec.text.per_word_pop", "EC", "Per-Word Scale Pop"),
    ("ec.stylize.glow", "EC", "Glow (ec.stylize.glow)"),
    ("ec.perspective.dropshadow", "EC", "Drop Shadow"),
    ("vc.envelope.arc", "VC", "Envelope Arc"),
    ("pc.layerStyle.stroke_halo", "PC", "Outer Stroke Halo"),
    ("pc.filter.paper_grain", "PC", "Paper Grain Press"),
];

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outer = ui.available_rect_before_wrap();
    let root_resp = ui.allocate_rect(outer, Sense::hover());
    ctx.reg.record("lyrics-workspace-root", "Lyrics Workspace", &root_resp, None);

    ui.vertical(|ui| {
        draw_toolbar(ui, ctx);
        ui.add_space(6.0);
        let body = ui.available_rect_before_wrap();
        if let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(245.0), 8.0, 180.0) {
            let mut rail = ui.new_child(egui::UiBuilder::new().max_rect(left_split.first).scope_id(ui.scope_id().with("lyrics_moments")));
            draw_moment_rail(&mut rail, ctx);
            if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(305.0), 8.0, 220.0) {
                let mut center = ui.new_child(egui::UiBuilder::new().max_rect(right_split.first).scope_id(ui.scope_id().with("lyrics_stage")));
                draw_stage_and_timeline(&mut center, ctx);
                let mut insp = ui.new_child(egui::UiBuilder::new().max_rect(right_split.second).scope_id(ui.scope_id().with("lyrics_insp")));
                draw_inspector(&mut insp, ctx);
            } else {
                let mut center = ui.new_child(egui::UiBuilder::new().max_rect(left_split.second).scope_id(ui.scope_id().with("lyrics_stage")));
                draw_stage_and_timeline(&mut center, ctx);
            }
        } else {
            draw_stage_and_timeline(ui, ctx);
        }
    });
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let is_design = ctx.state.lyrics.sub_mode == "design";
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-mode-design", "type", "Lyrics", "Typography & layout mode").selected(is_design))
                .clicked()
            {
                ctx.state.lyrics.sub_mode = "design".into();
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-mode-motion", "sparkles", "Motion", "Word entrance & range mode").selected(!is_design))
                .clicked()
            {
                ctx.state.lyrics.sub_mode = "motion".into();
            }
            let is_scene = ctx.state.lyrics.backdrop_mode == "scene";
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-bg-scene", "image", "Scene", "Preview over storyboard backdrop").selected(is_scene))
                .clicked()
            {
                ctx.state.lyrics.backdrop_mode = "scene".into();
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-bg-paper", "book", "Paper", "Preview over warm paper stage").selected(!is_scene))
                .clicked()
            {
                ctx.state.lyrics.backdrop_mode = "paper".into();
            }
            pill_toggle(ui, ctx.reg, ctx.pal, "words-draw-toggle", "Draw Path", &mut ctx.state.lyrics.draw_enabled, ctx.pal.accent_emerald);

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-seed-markers", "pin", "Markers", "Sync section markers into FilmCraft")).clicked() {
                ctx.exec("video.markers", json!({"action": "seed"}), Some("Synced lyric section markers"));
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
                        "outline": ctx.state.lyrics.outline_hex,
                    }
                });
                ctx.state.dirty_drafts.remove("lyrics");
                ctx.exec("words.save", payload, Some("Saved lyric section"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("words-render-studio", "film", "Publish", "Render lyrics.ecproj & publish V3 Kinetic Lyrics").primary(),
            )
            .clicked()
            {
                ctx.exec("words.letter", json!({"render": true, "studio": true}), Some("Published Kinetic Lyrics onto V3"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-open-effectcraft", "external", "Effect", "Open lettering/lyrics.ecproj in EffectCraft"))
                .clicked()
            {
                ctx.exec("workspace.open", json!({"app": "effectcraft", "domain": "lyrics"}), Some("Opened EffectCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-reload", "refresh", "Reload words.read & words.effects")).clicked() {
                ctx.exec("words.read", json!({}), None);
                ctx.exec("words.effects", json!({}), None);
            }
        });
    });
}

fn active_section_revision(ctx: &ViewCtx<'_>) -> Option<(String, u64)> {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array)?;
    let chosen =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first())?;
    let id = chosen["animation_id"].as_str()?.to_owned();
    let rev = chosen["source_revision"].as_u64().unwrap_or(0);
    Some((id, rev))
}

fn draw_moment_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Lyric moments").size(12.5).strong());
        text_input(ui, ctx.reg, "words-lyrics-search", "Search lyrics", &mut ctx.state.lyrics.search, "Filter lyrics…", 205.0);
        ui.separator();
        let query = ctx.state.lyrics.search.trim().to_ascii_lowercase();
        ScrollArea::vertical().id_salt("words_moments_scroll").show(ui, |ui| {
            if anims.is_empty() {
                ui.label(RichText::new("Click Refresh or transcribe song audio to load timed lyric moments.").size(11.0).color(ctx.pal.text_muted));
                return;
            }
            for anim in &anims {
                let id = anim["animation_id"].as_str().unwrap_or("sec");
                let sec_name = anim["scope"]["section"].as_str().unwrap_or(id);
                let preview_line: String =
                    anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).take(7).collect::<Vec<_>>().join(" ");
                if !query.is_empty() && !preview_line.to_ascii_lowercase().contains(&query) && !sec_name.to_ascii_lowercase().contains(&query) {
                    continue;
                }
                let selected = ctx.state.lyrics.selected_anim_id.as_deref() == Some(id);
                let btn_id = format!("words-moment-{id}-0");
                let word = sec_name.split_whitespace().next().unwrap_or("Verse");
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "music", word, &preview_line).selected(selected)).clicked() {
                    ctx.state.lyrics.selected_anim_id = Some(id.to_owned());
                    if let Some(anchor) = anim["song_anchor"].as_f64() {
                        ctx.state.lyrics.scrub_seconds = anchor;
                    }
                }
                ui.label(RichText::new(preview_line).size(11.0).color(ctx.pal.text_secondary));
                ui.separator();
            }
        });
    });
    ctx.reg.record("words-lyrics", "Lyric Moments", &panel.response, None);
}

fn draw_stage_and_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let active =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first());

    let panel = ctx.pal.card_frame().show(ui, |ui| {
        // Horizontal 35mm filmbar scene strip above stage
        let strip_w = ui.available_width().max(200.0);
        let (strip_rect, sresp) = ui.allocate_exact_size(Vec2::new(strip_w, 42.0), Sense::click());
        let strip_inner = paint_filmbar_frame(
            ui,
            strip_rect,
            ctx.pal,
            FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: sresp.hovered(), timecode: None },
        );
        let cell_w = (strip_inner.width() / 6.0).max(36.0);
        for i in 0..6 {
            let cell = Rect::from_min_size(
                Pos2::new(strip_inner.min.x + (i as f32) * cell_w + 2.0, strip_inner.min.y + 1.0),
                Vec2::new(cell_w - 4.0, strip_inner.height() - 2.0),
            );
            ui.painter().rect_stroke(
                cell,
                CornerRadius::same(3),
                Stroke::new(1.0, if i == 0 { ctx.pal.accent_emerald } else { ctx.pal.border_subtle }),
                StrokeKind::Inside,
            );
            ui.painter().text(cell.center(), Align2::CENTER_CENTER, format!("S{}", i + 1), FontId::monospace(9.5), ctx.pal.text_muted);
        }

        ui.add_space(4.0);
        let avail = ui.available_size();
        let aspect_w = avail.x.max(220.0);
        let aspect_h = (aspect_w * 9.0 / 16.0).min((avail.y - 120.0).max(150.0));
        let (stage_rect, stage_resp) = ui.allocate_exact_size(Vec2::new(aspect_w, aspect_h), Sense::click());
        let bg = if ctx.state.lyrics.backdrop_mode == "paper" { ctx.pal.stage_paper } else { ctx.pal.bg_stage };
        ui.painter().rect(stage_rect, CornerRadius::same(8), bg, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
        ctx.reg.record("words-stage", "Lyrics Stage", &stage_resp, None);

        if let Some(anim) = active {
            let words = anim["words"].as_array().cloned().unwrap_or_default();
            let mut cursor = Pos2::new(stage_rect.min.x + 28.0, stage_rect.center().y - 18.0);
            for (idx, w) in words.iter().enumerate() {
                let text = w["text"].as_str().unwrap_or("");
                let wid = w["word_id"].as_str().unwrap_or("").to_owned();
                let fg = if ctx.state.lyrics.backdrop_mode == "paper" {
                    Color32::from_rgb(0x17, 0x3f, 0x49)
                } else if idx == 3 {
                    ctx.pal.accent_amber
                } else {
                    ctx.pal.text_primary
                };
                let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(22.0), fg);
                if cursor.x + galley.size().x > stage_rect.max.x - 24.0 {
                    cursor.x = stage_rect.min.x + 28.0;
                    cursor.y += 32.0;
                }
                let wrect = Rect::from_min_size(cursor, galley.size() + egui::vec2(10.0, 6.0));
                let selected = ctx.state.lyrics.selected_word_id.as_deref() == Some(wid.as_str());
                if selected {
                    ui.painter().rect_stroke(wrect, CornerRadius::same(4), Stroke::new(1.5, ctx.pal.accent_emerald), StrokeKind::Inside);
                }
                ui.painter().text(wrect.center(), Align2::CENTER_CENTER, text, FontId::proportional(22.0), fg);
                let hit = ui.interact(wrect, ui.scope_id().with(("word_box", idx)), Sense::click());
                if hit.clicked() {
                    ctx.state.lyrics.selected_word_id = Some(wid);
                }
                ctx.reg.record(format!("word-box-{idx}"), text, &hit, None);
                cursor.x = wrect.max.x + 8.0;
            }
        } else {
            ui.painter().text(
                stage_rect.center(),
                Align2::CENTER_CENTER,
                "Kinetic Lyrics 16:9 Stage (V3 · EffectCraft)",
                FontId::proportional(14.0),
                ctx.pal.text_muted,
            );
        }

        ring_slider_h(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("words-scrub", "Scrub Time", 0.0..=ctx.state.audio_duration_sec.max(60.0), "s", ctx.pal.accent_emerald),
            &mut ctx.state.lyrics.scrub_seconds,
        );

        // Multi-lane timeline (`CUES`, `AUDIO`, `LYRICS`, `WORDS`)
        for (lane, accent) in [("CUES", ctx.pal.accent_sky), ("AUDIO", ctx.pal.text_muted), ("LYRICS", ctx.pal.accent_emerald), ("WORDS", ctx.pal.accent_amber)]
        {
            ui.horizontal(|ui| {
                ui.allocate_ui(Vec2::new(50.0, 15.0), |ui| {
                    ui.label(RichText::new(lane).monospace().size(9.5).color(ctx.pal.text_muted));
                });
                let lw = ui.available_width().max(80.0);
                let (lrect, _) = ui.allocate_exact_size(Vec2::new(lw, 14.0), Sense::hover());
                ui.painter().rect_filled(lrect, CornerRadius::same(3), ctx.pal.bg_input);
                let pill = Rect::from_min_size(Pos2::new(lrect.min.x + 12.0, lrect.min.y + 2.0), Vec2::new((lw * 0.42).max(28.0), 10.0));
                ui.painter().rect_filled(pill, CornerRadius::same(2), accent.gamma_multiply(0.42));
            });
        }
    });
    ctx.reg.record("words-stage-panel", "Stage Panel", &panel.response, None);
}

fn draw_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ScrollArea::vertical().id_salt("lyrics_inspector_scroll").show(ui, |ui| {
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
            if ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("words-item-size", "Font Size", 24.0..=240.0, "px", ctx.pal.accent_emerald),
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
            pill_toggle(ui, ctx.reg, ctx.pal, "words-3d-per-char", "EffectCraft 3D Per-Character", &mut ctx.state.lyrics.per_char_3d, ctx.pal.accent_emerald);

            ui.separator();
            ui.label(RichText::new("Craft Typography Effects (CLI Mapped)").size(12.0).strong());
            draw_mapped_lyrics_effects(ui, ctx);

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
                ScalarSpec::new("words-item-stroke-width", "Stroke Halo", 0.0..=24.0, "px", ctx.pal.accent_amber),
                &mut ctx.state.lyrics.stroke_width,
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fill").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "words-item-color", "Fill hex", &mut ctx.state.lyrics.color_hex, "#f0f4fc", 84.0);
                ui.label(RichText::new("Halo").size(10.5).color(ctx.pal.text_muted));
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
        FALLBACK_LYRICS_FX.iter().map(|(id, badge, label)| ((*id).to_owned(), (*badge).to_owned(), (*label).to_owned())).collect()
    } else {
        dynamic_fx
            .iter()
            .filter_map(|fx| Some((fx["id"].as_str()?.to_owned(), fx["badge"].as_str().unwrap_or("EC").to_owned(), fx["label"].as_str()?.to_owned())))
            .collect()
    };

    ui.horizontal_wrapped(|ui| {
        for (fx_id, badge, label) in items {
            let btn_id = format!("words-fx-{fx_id}");
            let sel = ctx.state.lyrics.selected_effect_id == fx_id;
            let short = format!("{badge}·{}", label.split_whitespace().next().unwrap_or("Effect"));
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "sparkles", &short, &format!("{label} ({fx_id})")).selected(sel)).clicked() {
                ctx.state.lyrics.selected_effect_id = fx_id.clone();
                ctx.exec("words.effect_install", json!({"name": fx_id}), Some("Selected Craft typography effect"));
            }
        }
    });
}
