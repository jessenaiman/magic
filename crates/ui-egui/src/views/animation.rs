//! `Animations` view (`#view-animation`): Craft-project-native character rigging & animation
//! backed directly by `PhotoCraft` (`.pcraft` / `.psd`), `VectorCraft` (`.vectorcraft` / `.svg`),
//! headless `anicraft-cli` (`.anicraft`), and `EffectCraft` (`.ecproj`) documents.
//! Stage renders actual frames on disk (`frames/*.png`, `sprites/*/frames/*.png`, `parts.json`
//! drawings) and overlays ONLY real `parts.json` joints/pins, real `character.path` walk curves,
//! and real `character.keys` keyframes.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{
    Align, Align2, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Shape,
    Stroke, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, ScalarSpec, badge_accent_color, child_pane, fill_card,
    paint_craft_doc_card, paint_filmbar_frame, paint_local_image, pill_toggle, ring_slider_h,
    rotary_dial, xy_path_pad,
};
use crate::state::{UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_roster_bar(ui, ctx);
    ui.add_space(6.0);
    let remaining = ui.available_rect_before_wrap();
    if let Some(vert) = split_rect(remaining, SplitAxis::Vertical, SplitSize::FixedSecond(132.0), 8.0, 220.0) {
        let mut upper = child_pane(ui, vert.first, "anim_upper");
        draw_upper_columns(&mut upper, ctx);
        let mut dock = child_pane(ui, vert.second, "anim_theatre_dock");
        draw_theatre_dock(&mut dock, ctx);
        let _ = ui.allocate_rect(remaining, Sense::hover());
    } else {
        draw_upper_columns(ui, ctx);
    }
}

fn character_entries(ctx: &ViewCtx<'_>) -> Vec<Value> {
    ctx.state
        .cache
        .characters
        .get("characters")
        .or_else(|| ctx.state.cache.snapshot.pointer("/data/characters"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn active_character_obj(ctx: &ViewCtx<'_>) -> Option<Value> {
    let chars = character_entries(ctx);
    if let Some(id) = &ctx.state.selected_character_id
        && let Some(found) = chars.iter().find(|c| c["id"].as_str() == Some(id.as_str()))
    {
        return Some(found.clone());
    }
    chars.into_iter().next()
}

fn active_character(ctx: &ViewCtx<'_>) -> String {
    active_character_obj(ctx)
        .and_then(|c| c["id"].as_str().map(str::to_owned))
        .or_else(|| ctx.state.selected_character_id.clone())
        .unwrap_or_default()
}

fn active_view_obj(char_obj: Option<&Value>) -> Option<Value> {
    char_obj?["views"].as_array()?.first().cloned()
}

fn select_rendered_character_frame(ctx: &ViewCtx<'_>, char_obj: Option<&Value>, view_obj: Option<&Value>) -> Option<String> {
    if let Some(last) = &ctx.state.cache.last_character_frame {
        return Some(last.clone());
    }
    if let Some(sprites) = char_obj.and_then(|c| c["sprites"].as_array())
        && let Some(sp) = sprites
            .iter()
            .find(|s| s["name"].as_str() == Some(ctx.state.animation.selected_action.as_str()))
            .or_else(|| sprites.first())
        && let Some(frames) = sp["frames"].as_array()
        && !frames.is_empty()
    {
        let fps = sp["fps"].as_u64().unwrap_or(24).max(1) as f64;
        let idx = ((ctx.state.animation.scrub_seconds.max(0.0) * fps).floor() as usize) % frames.len();
        if let Some(f) = frames.get(idx).and_then(Value::as_str) {
            return Some(f.to_owned());
        }
    }
    if let Some(v) = view_obj {
        if let Some(frames) = v["rendered_frames"].as_array()
            && !frames.is_empty()
        {
            let idx = ((ctx.state.animation.scrub_seconds.max(0.0) * 24.0).floor() as usize) % frames.len();
            if let Some(f) = frames.get(idx).and_then(Value::as_str) {
                return Some(f.to_owned());
            }
        }
        if let Some(latest) = v["latest_frame"].as_str().or_else(|| v["drawing"].as_str()) {
            return Some(latest.to_owned());
        }
    }
    let cid = active_character(ctx);
    ctx.state
        .cache
        .snapshot
        .pointer("/data/drawer/characters")
        .and_then(Value::as_array)?
        .iter()
        .find(|c| c["char_id"].as_str() == Some(cid.as_str()) || c["id"].as_str() == Some(cid.as_str()))?
        .get("thumb")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn active_background_thumb(ctx: &ViewCtx<'_>) -> Option<String> {
    ctx.state
        .cache
        .snapshot
        .pointer("/data/drawer/backgrounds/0/thumb")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn draw_upper_columns(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let body = ui.available_rect_before_wrap();
    if let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(264.0), 8.0, 180.0) {
        let mut left = child_pane(ui, left_split.first, "anim_left");
        draw_action_and_path_column(&mut left, ctx);
        if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(298.0), 8.0, 220.0) {
            let mut stage = child_pane(ui, right_split.first, "anim_stage");
            draw_stage(&mut stage, ctx);
            let mut right = child_pane(ui, right_split.second, "anim_puppet");
            draw_puppet_inspector(&mut right, ctx);
        } else {
            let mut stage = child_pane(ui, left_split.second, "anim_stage");
            draw_stage(&mut stage, ctx);
        }
        let _ = ui.allocate_rect(body, Sense::hover());
    } else {
        draw_stage(ui, ctx);
    }
}

fn draw_roster_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let chars = character_entries(ctx);
    let current = active_character(ctx);
    let bar = ctx.pal.panel_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("CHARACTER ROSTER:").monospace().size(10.5).color(ctx.pal.text_muted));
            if chars.is_empty() {
                ui.label(RichText::new("No .pcraft / .ecproj characters yet — import via + Asset").size(11.0).color(ctx.pal.text_muted));
            }
            for ch in &chars {
                let id = ch["id"].as_str().unwrap_or_default();
                let name = ch["name"].as_str().unwrap_or(id);
                let v0 = ch["views"].as_array().and_then(|vs| vs.first());
                let mut craft_tags = Vec::new();
                if v0.and_then(|v| v["pcraft_path"].as_str()).is_some() {
                    craft_tags.push(".pcraft");
                } else if v0.and_then(|v| v["psd_path"].as_str()).is_some() {
                    craft_tags.push(".psd");
                }
                if v0.and_then(|v| v["vector_doc"].as_str()).is_some() {
                    craft_tags.push(".svg");
                }
                if v0.and_then(|v| v["ecproj_path"].as_str()).is_some() || v0.and_then(|v| v["ecproj"].as_bool()) == Some(true) {
                    craft_tags.push(".ecproj");
                }
                let tag_str = if craft_tags.is_empty() {
                    ch["animation_style"].as_str().unwrap_or("cutout").to_owned()
                } else {
                    craft_tags.join(" · ")
                };
                let pill_lbl = format!("{name} ({tag_str})");
                let btn_id = format!("anim-char-{id}");
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "user", &pill_lbl, name).selected(current == id)).clicked() {
                    ctx.push(UiAction::SelectCharacter {
                        id: id.to_owned(),
                        jump_animation: false,
                    });
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has_char = !current.is_empty();
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("character-open-effectcraft", "external", "EffectCraft", "Open .ecproj in EffectCraft").enabled(has_char),
                )
                .clicked()
                {
                    ctx.exec("character.open", json!({"character": current, "app": "effectcraft"}), Some("Opened .ecproj in EffectCraft"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("character-open-vectorcraft", "pen", "VectorCraft", "Open .vectorcraft / .svg in VectorCraft").enabled(has_char),
                )
                .clicked()
                {
                    ctx.exec("character.open", json!({"character": current, "app": "vectorcraft"}), Some("Opened VectorCraft"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("character-open-photocraft", "image", "PhotoCraft", "Open .pcraft / .psd in PhotoCraft").enabled(has_char),
                )
                .clicked()
                {
                    ctx.exec("character.open", json!({"character": current, "app": "photocraft"}), Some("Opened .pcraft in PhotoCraft"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("character-rig-run", "wand", "Rig .ecproj", "Build EffectCraft .ecproj from .pcraft/.psd")
                        .primary()
                        .enabled(has_char),
                )
                .clicked()
                {
                    ctx.exec("character.rig", json!({"character": current}), Some("Built EffectCraft puppet rig (.ecproj)"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("character-cut-run", "scissors", "Cut .pcraft", "Cut layered parts into .pcraft via PhotoCraft").enabled(has_char),
                )
                .clicked()
                {
                    ctx.exec("character.cut", json!({"character": current, "remove_background": true}), Some("Cut parts into .pcraft"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("character-mark-run", "pin", "Mark", "Mark puppet joints").enabled(has_char),
                )
                .clicked()
                {
                    ctx.exec("character.mark", json!({"character": current}), Some("Marked puppet joints"));
                }
            });
        });
    });
    ctx.reg.record("anim-roster-bar", "Character Roster", &bar.response, None);
}

fn draw_action_and_path_column(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let scroll_out = ScrollArea::vertical().id_salt("anim_left_scroll").auto_shrink([false, false]).show(ui, |ui| {
        fill_card(ui, ctx.pal, false, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Animation Style").size(11.5).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mode_lbl = match ctx.state.animation.style_tab.as_str() {
                        "sprite-2d" => "2D Sprite (.anicraft)",
                        "rig-3d" => "3D Anim (.ecproj)",
                        _ => "2.5D Rig (.pcraft + .ecproj)",
                    };
                    ui.label(RichText::new(mode_lbl).size(10.0).color(ctx.pal.text_secondary));
                });
            });
            ui.horizontal(|ui| {
                for (tid, lbl, tip) in [
                    ("sprite-2d", "2D Sprite", "Headless anicraft-cli sprite cycle -> .ecproj"),
                    ("cutout-2-5d", "2.5D Rig", "2.5D PhotoCraft (.pcraft) + EffectCraft (.ecproj)"),
                    ("rig-3d", "3D Anim", "EffectCraft 3D model rig (.ecproj)"),
                ] {
                    let btn_id = format!("style-tab-{tid}");
                    let sel = ctx.state.animation.style_tab == tid;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, lbl, tip).selected(sel)).clicked() {
                        ctx.state.animation.style_tab = tid.into();
                    }
                }
            });
        });
        ui.add_space(6.0);

        fill_card(ui, ctx.pal, false, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("▷ Action").size(12.0).strong().color(ctx.pal.accent_sky));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(&ctx.state.animation.selected_action).monospace().size(10.0).color(ctx.pal.text_muted));
                });
            });
            ui.horizontal_wrapped(|ui| {
                for (act, lbl) in [
                    ("idle", "Idle"),
                    ("jump", "Jump"),
                    ("swim", "Swim"),
                    ("turn", "Turn"),
                    ("down", "Down"),
                    ("left", "Left"),
                    ("right", "Right"),
                    ("up", "Up"),
                    ("wave", "Wave"),
                    ("rest", "Rest"),
                ] {
                    let btn_id = format!("anim-act-{act}");
                    let sel = ctx.state.animation.selected_action == act;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, lbl, "Key action cycle into .ecproj").selected(sel)).clicked() {
                        ctx.state.animation.selected_action = act.into();
                        if !cid.is_empty() {
                            ctx.exec("character.action", json!({"character": cid, "action": act}), Some("Keyed character action in .ecproj"));
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Action name").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "anim-action-name", "Action name", &mut ctx.state.animation.action_name_input, "sing", 136.0);
            });
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-cycle-duration", "Cycle duration", 0.4..=8.0, "s", ctx.pal.accent_sky),
                &mut ctx.state.animation.action_duration,
            );
            pill_toggle(
                ui,
                ctx.reg,
                ctx.pal,
                "anim-loop-cycle",
                "Loop action cycle",
                &mut ctx.state.animation.loop_clip,
                ctx.pal.accent_sky,
            );
        });
        ui.add_space(6.0);

        let amb = ctx.pal.accent_amber;
        fill_card(ui, ctx.pal, false, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("✎ Path & depth").size(12.0).strong().color(amb));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.0}% → {:.0}%", ctx.state.animation.start_size_pct, ctx.state.animation.end_size_pct))
                            .monospace()
                            .size(10.0)
                            .color(ctx.pal.text_muted),
                    );
                });
            });
            ui.label(
                RichText::new("Author EffectCraft root travel keyframes and start/end perspective scale.")
                    .size(10.5)
                    .color(ctx.pal.text_muted),
            );
            xy_path_pad(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-path-x", "Point X", 0.0..=1920.0, "px", amb),
                &mut ctx.state.animation.path_x,
                ScalarSpec::new("anim-path-y", "Point Y", 0.0..=1080.0, "px", amb),
                &mut ctx.state.animation.path_y,
            );
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-travel-dur", "Travel duration", 0.2..=12.0, "s", amb),
                &mut ctx.state.animation.travel_duration,
            );
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-start-size", "Start size", 20.0..=250.0, "%", amb),
                &mut ctx.state.animation.start_size_pct,
            );
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-end-size", "End size", 20.0..=250.0, "%", amb),
                &mut ctx.state.animation.end_size_pct,
            );
            ui.horizontal(|ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-clear-path", "Clear", "Clear root path in .ecproj")).clicked()
                    && !cid.is_empty()
                {
                    ctx.exec("character.path", json!({"character": cid, "op": "clear"}), Some("Cleared walk path in .ecproj"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-undo-point", "Undo point", "Step back path coordinate")).clicked() {
                    ctx.state.animation.path_x = (ctx.state.animation.path_x - 40.0).max(0.0);
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only("anim-apply-path", "Apply path", "Write walk path keyframes into .ecproj")
                        .primary()
                        .enabled(!cid.is_empty()),
                )
                .clicked()
                {
                    ctx.exec(
                        "character.path",
                        json!({
                            "character": cid,
                            "op": "apply",
                            "points": [
                                {"x": 200.0, "y": ctx.state.animation.path_y, "time": 0.0, "scale": ctx.state.animation.start_size_pct},
                                {"x": ctx.state.animation.path_x, "y": ctx.state.animation.path_y, "time": ctx.state.animation.travel_duration, "scale": ctx.state.animation.end_size_pct}
                            ],
                        }),
                        Some("Applied walk path to .ecproj"),
                    );
                }
            });
        });
    });
    let resp = ui.interact(scroll_out.inner_rect, ui.scope_id().with("motion_layers"), Sense::hover());
    ctx.reg.record("motion-layers", "Action & Path", &resp, None);
}

fn draw_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let char_obj = active_character_obj(ctx);
    let view_obj = active_view_obj(char_obj.as_ref());
    let cid = active_character(ctx);
    let frame_path = select_rendered_character_frame(ctx, char_obj.as_ref(), view_obj.as_ref());
    let bg_thumb = active_background_thumb(ctx);
    let joints = view_obj.as_ref().and_then(|v| v["joints"].as_array()).cloned().unwrap_or_default();
    let pins = view_obj.as_ref().and_then(|v| v["pins"].as_array()).cloned().unwrap_or_default();
    let total_marks = joints.len() + pins.len();
    let ecproj_file = view_obj
        .as_ref()
        .and_then(|v| v["ecproj_path"].as_str().or_else(|| v["photo_doc"].as_str()))
        .unwrap_or("assets/characters/<id>/<id>-front.ecproj");

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            pill_toggle(ui, ctx.reg, ctx.pal, "motion-grid", "Grid", &mut ctx.state.animation.show_grid, ctx.pal.accent_sky);
            let marks_lbl = format!("Rig Marks ({total_marks})");
            pill_toggle(ui, ctx.reg, ctx.pal, "motion-marks", &marks_lbl, &mut ctx.state.animation.show_marks, ctx.pal.accent_emerald);
            pill_toggle(ui, ctx.reg, ctx.pal, "motion-path-visible", "Walk Path", &mut ctx.state.animation.show_path_hud, ctx.pal.accent_amber);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("motion-export", "film", "Publish V2", "Render ProRes 4444 V2 pass into FilmCraft")
                        .primary()
                        .enabled(!cid.is_empty()),
                )
                .clicked()
                {
                    ctx.exec(
                        "character.shot",
                        json!({"character": cid, "sprite": ctx.state.animation.selected_action, "duration": ctx.state.animation.action_duration, "studio": true}),
                        Some("Published V2 Character Animation"),
                    );
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("anim-render-frame", "image", "Render Frame", "Render EffectCraft .ecproj frame at scrub time").enabled(!cid.is_empty()),
                )
                .clicked()
                {
                    ctx.exec(
                        "character.frame",
                        json!({"character": cid, "time": ctx.state.animation.scrub_seconds}),
                        Some("Rendered EffectCraft frame"),
                    );
                }
            });
        });

        let avail = ui.available_size();
        let (outer_rect, resp) = ui.allocate_exact_size(Vec2::new(avail.x.max(200.0), (avail.y - 6.0).max(160.0)), Sense::click());
        let inner = paint_filmbar_frame(
            ui,
            outer_rect,
            ctx.pal,
            FilmbarStyle {
                orientation: FilmbarOrientation::Vertical,
                selected: false,
                hovered: resp.hovered(),
                timecode: Some("V2 · .pcraft / .ecproj"),
            },
        );
        let _ = paint_local_image(ui, inner, bg_thumb.as_deref(), ctx.pal.bg_stage, 4);

        if ctx.state.animation.show_grid {
            let mid = inner.center();
            ui.painter().line_segment(
                [Pos2::new(inner.min.x, mid.y), Pos2::new(inner.max.x, mid.y)],
                Stroke::new(1.0, ctx.pal.border_subtle),
            );
            ui.painter().line_segment(
                [Pos2::new(mid.x, inner.min.y), Pos2::new(mid.x, inner.max.y)],
                Stroke::new(1.0, ctx.pal.border_subtle),
            );
        }

        let char_h = (inner.height() * 0.72).clamp(90.0, 280.0);
        let char_rect = Rect::from_center_size(inner.center(), Vec2::new(char_h * 1.25, char_h));
        if !paint_local_image(ui, char_rect, frame_path.as_deref(), egui::Color32::TRANSPARENT, 4) {
            paint_craft_doc_card(
                ui,
                char_rect,
                ctx.pal,
                "EC",
                ecproj_file.rsplit('/').next().unwrap_or(ecproj_file),
                "Click Render Frame or Rig .ecproj to render character",
            );
        }

        // Overlay ONLY real walk path keyframes returned from EffectCraft (`character.path op=inspect`)
        if ctx.state.animation.show_path_hud
            && let Some(pts) = ctx.state.cache.character_path["points"].as_array()
            && pts.len() >= 2
        {
            let mapped: Vec<Pos2> = pts
                .iter()
                .filter_map(|pt| {
                    let x = pt["x"].as_f64()? as f32;
                    let y = pt["y"].as_f64()? as f32;
                    Some(Pos2::new(
                        inner.min.x + (x / 1920.0).clamp(0.02, 0.98) * inner.width(),
                        inner.min.y + (y / 1080.0).clamp(0.02, 0.98) * inner.height(),
                    ))
                })
                .collect();
            if mapped.len() >= 2 {
                ui.painter().add(Shape::line(mapped.clone(), Stroke::new(2.2, ctx.pal.accent_amber)));
                for p in mapped {
                    ui.painter().circle_filled(p, 4.2, ctx.pal.bg_input);
                    ui.painter().circle_stroke(p, 4.2, Stroke::new(1.8, ctx.pal.accent_amber));
                }
            }
        }

        // Overlay ONLY real joints and puppet pins read from `parts.json`
        if ctx.state.animation.show_marks && total_marks > 0 {
            let (cw, ch) = view_obj
                .as_ref()
                .and_then(|v| v["canvas_size"].as_array())
                .filter(|a| a.len() == 2)
                .and_then(|a| Some((a[0].as_f64()? as f32, a[1].as_f64()? as f32)))
                .unwrap_or((1024.0, 1024.0));
            for j in &joints {
                if let Some(arr) = j["joint"].as_array().filter(|a| a.len() == 2)
                    && let (Some(jx), Some(jy)) = (arr[0].as_f64(), arr[1].as_f64())
                {
                    let pt = Pos2::new(
                        char_rect.min.x + (jx as f32 / cw.max(1.0)).clamp(0.0, 1.0) * char_rect.width(),
                        char_rect.min.y + (jy as f32 / ch.max(1.0)).clamp(0.0, 1.0) * char_rect.height(),
                    );
                    ui.painter().circle_filled(pt, 4.0, ctx.pal.bg_input);
                    ui.painter().circle_stroke(pt, 4.0, Stroke::new(1.8, ctx.pal.accent_sky));
                }
            }
            for pin in &pins {
                if let Some(arr) = pin["pin"].as_array().filter(|a| a.len() == 2)
                    && let (Some(px), Some(py)) = (arr[0].as_f64(), arr[1].as_f64())
                {
                    let pt = Pos2::new(
                        char_rect.min.x + (px as f32 / cw.max(1.0)).clamp(0.0, 1.0) * char_rect.width(),
                        char_rect.min.y + (py as f32 / ch.max(1.0)).clamp(0.0, 1.0) * char_rect.height(),
                    );
                    ui.painter().circle_filled(pt, 3.6, ctx.pal.bg_input);
                    ui.painter().circle_stroke(pt, 3.6, Stroke::new(1.7, ctx.pal.accent_emerald));
                }
            }
        }

        let header_lbl = if cid.is_empty() {
            "No character selected".to_owned()
        } else {
            let fname = frame_path
                .as_deref()
                .and_then(|p| p.rsplit('/').next())
                .unwrap_or_else(|| ecproj_file.rsplit('/').next().unwrap_or(&cid));
            format!("{cid} · {fname} · {:.2}s", ctx.state.animation.scrub_seconds)
        };
        ui.painter().text(
            Pos2::new(inner.center().x, inner.min.y + 14.0),
            Align2::CENTER_CENTER,
            header_lbl,
            FontId::monospace(10.5),
            ctx.pal.text_primary,
        );
        ctx.reg.record("motion-canvas", "Animation Canvas", &resp, None);
    });
    ctx.reg.record("motion-stage", "Animation Stage", &panel.response, None);
}

fn draw_puppet_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let char_obj = active_character_obj(ctx);
    let view_obj = active_view_obj(char_obj.as_ref());
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ScrollArea::vertical().id_salt("anim_puppet_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.label(RichText::new("Craft Documents & Puppet Rig").size(12.0).strong().color(ctx.pal.accent_amber));
            if let Some(v) = &view_obj {
                ui.horizontal_wrapped(|ui| {
                    for (key, lbl) in [
                        ("pcraft_path", ".pcraft"),
                        ("psd_path", ".psd"),
                        ("vector_doc", ".svg/.vectorcraft"),
                        ("ecproj_path", ".ecproj"),
                        ("anicraft_path", ".anicraft"),
                    ] {
                        if let Some(p) = v[key].as_str() {
                            let fname = p.rsplit('/').next().unwrap_or(lbl);
                            status_pill(ui, ctx.pal, fname, badge_accent_color(ctx.pal, lbl));
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                rotary_dial(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("bone-rotation", "Head Tilt", -90.0..=90.0, "°", ctx.pal.accent_sky),
                    &mut ctx.state.animation.bone_rotation,
                );
                rotary_dial(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("puppet-jaw-open", "Jaw Open", 0.0..=45.0, "°", ctx.pal.accent_amber),
                    &mut ctx.state.animation.jaw_open_deg,
                );
                rotary_dial(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("puppet-wing-bank", "Wing Bank", -60.0..=60.0, "°", ctx.pal.accent_emerald),
                    &mut ctx.state.animation.wing_bank_deg,
                );
            });

            ui.horizontal_wrapped(|ui| {
                let en = !cid.is_empty();
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("authoring-puppet-rig-face", "wand", "Rig Face", "Rig facial mesh in .ecproj").enabled(en)).clicked() {
                    ctx.exec("character.puppet.rig_face", json!({"character": cid}), Some("Rigged facial mesh in .ecproj"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("authoring-puppet-lipsync", "music", "Lipsync", "Sync mouth visemes to song").primary().enabled(en),
                )
                .clicked()
                {
                    ctx.exec(
                        "character.puppet.lipsync",
                        json!({"character": cid, "intensity": ctx.state.animation.lip_intensity / 100.0}),
                        Some("Synced mouth visemes in .ecproj"),
                    );
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("authoring-puppet-follow", "sparkles", "Follow", "Cascade secondary follow pins").enabled(en)).clicked() {
                    ctx.exec(
                        "character.puppet.follow",
                        json!({"character": cid, "delay": ctx.state.animation.follow_delay}),
                        Some("Applied puppet.follow in .ecproj"),
                    );
                }
            });

            ui.label(RichText::new("Mouth Visemes").size(10.5).color(ctx.pal.text_muted));
            ui.horizontal_wrapped(|ui| {
                for v in ["AA", "EE", "OH", "OO", "MBP", "Rest"] {
                    let id = format!("puppet-viseme-{v}");
                    let sel = ctx.state.animation.viseme == v;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, v, "Preview mouth viseme").selected(sel)).clicked() {
                        ctx.state.animation.viseme = v.into();
                    }
                }
            });

            ui.label(RichText::new("Expressions").size(10.5).color(ctx.pal.text_muted));
            ui.horizontal_wrapped(|ui| {
                for expr in ["happy", "surprised", "focused", "blink", "neutral"] {
                    let id = format!("puppet-expr-{expr}");
                    let sel = ctx.state.animation.expression == expr;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, expr, "Keyframe expression in .ecproj").selected(sel)).clicked() {
                        ctx.state.animation.expression = expr.into();
                        if !cid.is_empty() {
                            ctx.exec("character.puppet.expression", json!({"character": cid, "expression": expr}), Some("Keyframed expression"));
                        }
                    }
                }
            });

            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("puppet-lip-intensity", "Lip Intensity", 0.0..=150.0, "%", ctx.pal.accent_amber),
                &mut ctx.state.animation.lip_intensity,
            );
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("puppet-follow-delay", "Follow Delay", 0.01..=0.35, "s", ctx.pal.accent_sky),
                &mut ctx.state.animation.follow_delay,
            );
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("authoring-part-apply", "check", "Apply Transform", "Write part transform keyframe to .ecproj").enabled(!cid.is_empty()),
            )
            .clicked()
            {
                ctx.exec(
                    "character.set",
                    json!({
                        "character": cid,
                        "part": ctx.state.animation.selected_part,
                        "prop": "rotation",
                        "value": ctx.state.animation.bone_rotation,
                        "time": ctx.state.animation.scrub_seconds,
                    }),
                    Some("Updated part transform in .ecproj"),
                );
            }
            status_pill(ui, ctx.pal, "V2 · Character Animation (.ecproj)", ctx.pal.accent_emerald);
        });
    });
    ctx.reg.record("motion-inspector", "Puppet Inspector", &panel.response, None);
}

fn draw_theatre_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let key_layers = ctx.state.cache.character_keys["layers"].as_array().cloned().unwrap_or_default();
    let dur = ctx.state.animation.action_duration.max(4.0);

    let dock = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⊞ Theatre (.ecproj Keys)").size(12.0).strong().color(ctx.pal.accent_emerald));
            for (tid, word) in [("theatre", "Theatre"), ("keys", "Keys"), ("curves", "Curves")] {
                let id = format!("anim-dock-{tid}");
                let sel = ctx.state.animation.dock_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, word, "Switch EffectCraft keyframe view").selected(sel)).clicked() {
                    ctx.state.animation.dock_tab = tid.into();
                    if !cid.is_empty() {
                        ctx.exec("character.keys", json!({"character": cid, "op": "list"}), None);
                    }
                }
            }
            ui.label(
                RichText::new(format!("RANGE 0.0s–{dur:.1}s"))
                    .monospace()
                    .size(10.0)
                    .color(ctx.pal.text_muted),
            );
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("motion-scrub", "Scrub", 0.0..=dur, "s", ctx.pal.accent_emerald),
                &mut ctx.state.animation.scrub_seconds,
            );
        });

        let w = ui.available_width().max(160.0);
        let h = ui.available_height().max(42.0);
        let (track_rect, tresp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        ui.painter().rect_filled(track_rect, CornerRadius::same(6), ctx.pal.bg_input);
        if tresp.clicked()
            && let Some(pos) = tresp.interact_pointer_pos()
        {
            let frac = ((pos.x - track_rect.min.x) / track_rect.width()).clamp(0.0, 1.0) as f64;
            ctx.state.animation.scrub_seconds = frac * dur;
        }

        let mut key_count = 0usize;
        for (l_idx, layer) in key_layers.iter().take(4).enumerate() {
            let y = track_rect.min.y + 11.0 + (l_idx as f32) * 10.5;
            if y > track_rect.max.y - 4.0 {
                break;
            }
            for prop in layer["properties"].as_array().into_iter().flatten() {
                for k in prop["keys"].as_array().into_iter().flatten() {
                    if let Some(t) = k["time"].as_f64() {
                        let x = track_rect.min.x + 8.0 + ((t / dur).clamp(0.0, 1.0) as f32) * (w - 16.0);
                        ui.painter().circle_filled(Pos2::new(x, y), 3.4, ctx.pal.accent_amber);
                        key_count += 1;
                    }
                }
            }
        }
        if key_count == 0 {
            ui.painter().text(
                track_rect.center(),
                Align2::CENTER_CENTER,
                "No EffectCraft keyframes sampled yet — rig .ecproj and trigger an Action or Apply Path",
                FontId::monospace(10.0),
                ctx.pal.text_muted,
            );
        }
        let play_x = track_rect.min.x + 8.0 + ((ctx.state.animation.scrub_seconds / dur).clamp(0.0, 1.0) as f32) * (w - 16.0);
        ui.painter().line_segment(
            [Pos2::new(play_x, track_rect.min.y + 2.0), Pos2::new(play_x, track_rect.max.y - 2.0)],
            Stroke::new(1.5, ctx.pal.accent_emerald),
        );
    });
    ctx.reg.record("anim-theatre-dock", "Theatre Dock", &dock.response, None);
}
