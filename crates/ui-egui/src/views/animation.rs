//! `Animations` view (`#view-animation`): Character Roster, `2D Sprite | 2.5D Rig | 3D` style
//! switch, `Action` cycle controls (`Cycle duration` hollow-ring slider + `Loop action cycle` pill
//! toggle), `Path & depth` (`Point X` horizontal + `Point Y` vertical sliders), 35mm **filmbar**
//! puppet stage, `EffectCraft` 11-pin Face & Viseme rotary dials, and `Theatre | Keys | Curves` dock.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align2, CornerRadius, FontId, Pos2, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, ScalarSpec, paint_filmbar_frame, pill_toggle, ring_slider_h, rotary_dial, xy_path_pad};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outer = ui.available_rect_before_wrap();
    let root_resp = ui.allocate_rect(outer, Sense::hover());
    ctx.reg.record("view-animation", "Animation", &root_resp, None);

    ui.vertical(|ui| {
        draw_roster_bar(ui, ctx);
        ui.add_space(6.0);
        let remaining = ui.available_rect_before_wrap();
        if let Some(vert) = split_rect(remaining, SplitAxis::Vertical, SplitSize::FixedSecond(120.0), 8.0, 220.0) {
            let mut upper = ui.new_child(egui::UiBuilder::new().max_rect(vert.first).scope_id(ui.scope_id().with("anim_upper")));
            draw_upper_columns(&mut upper, ctx);
            let mut dock = ui.new_child(egui::UiBuilder::new().max_rect(vert.second).scope_id(ui.scope_id().with("anim_theatre_dock")));
            draw_theatre_dock(&mut dock, ctx);
        } else {
            draw_upper_columns(ui, ctx);
        }
    });
}

fn active_character(ctx: &ViewCtx<'_>) -> String {
    if let Some(id) = &ctx.state.selected_character_id {
        return id.clone();
    }
    ctx.state.cache.snapshot.pointer("/data/characters/0/id").and_then(Value::as_str).unwrap_or("lead").to_owned()
}

fn draw_upper_columns(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let body = ui.available_rect_before_wrap();
    if let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(255.0), 8.0, 180.0) {
        let mut left = ui.new_child(egui::UiBuilder::new().max_rect(left_split.first).scope_id(ui.scope_id().with("anim_left")));
        draw_action_and_path_column(&mut left, ctx);
        if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(295.0), 8.0, 220.0) {
            let mut stage = ui.new_child(egui::UiBuilder::new().max_rect(right_split.first).scope_id(ui.scope_id().with("anim_stage")));
            draw_stage(&mut stage, ctx);
            let mut right = ui.new_child(egui::UiBuilder::new().max_rect(right_split.second).scope_id(ui.scope_id().with("anim_puppet")));
            draw_puppet_inspector(&mut right, ctx);
        } else {
            let mut stage = ui.new_child(egui::UiBuilder::new().max_rect(left_split.second).scope_id(ui.scope_id().with("anim_stage")));
            draw_stage(&mut stage, ctx);
        }
    } else {
        draw_stage(ui, ctx);
    }
}

fn draw_roster_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let chars = ctx.state.cache.snapshot.pointer("/data/characters").and_then(Value::as_array).cloned().unwrap_or_default();
    let current = active_character(ctx);
    let bar = ctx.pal.panel_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("CHARACTER ROSTER:").monospace().size(10.5).color(ctx.pal.text_muted));
            for ch in &chars {
                let id = ch["id"].as_str().unwrap_or("lead");
                let name = ch["name"].as_str().unwrap_or(id);
                let word = name.split_whitespace().next().unwrap_or("Cast");
                let btn_id = format!("anim-char-{id}");
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "user", word, name).selected(current == id)).clicked() {
                    ctx.state.selected_character_id = Some(id.to_owned());
                }
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("character-mark-run", "pin", "Mark", "Mark puppet joints")).clicked() {
                ctx.exec("character.mark", json!({"character": current}), Some("Marked puppet joints"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("character-cut-run", "scissors", "Cut", "Cut layered parts via PhotoCraft")).clicked() {
                ctx.exec("character.cut", json!({"character": current, "remove_background": true}), Some("Cut parts in PhotoCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("character-rig-run", "wand", "Rig", "Build EffectCraft puppet rig").primary()).clicked() {
                ctx.exec("character.rig", json!({"character": current}), Some("Built EffectCraft puppet rig"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("character-open-photocraft", "image", "Photo", "Open in PhotoCraft")).clicked() {
                ctx.exec("character.open", json!({"character": current, "app": "photocraft"}), Some("Opened PhotoCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("character-open-effectcraft", "sparkles", "Effect", "Open in EffectCraft")).clicked() {
                ctx.exec("character.open", json!({"character": current, "app": "effectcraft"}), Some("Opened EffectCraft"));
            }
        });
    });
    ctx.reg.record("anim-roster-bar", "Character Roster", &bar.response, None);
}

fn draw_action_and_path_column(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ScrollArea::vertical().id_salt("anim_left_scroll").show(ui, |ui| {
            ui.label(RichText::new("Animation Style").size(12.0).strong());
            ui.horizontal_wrapped(|ui| {
                for (tid, word, tip) in [
                    ("sprite-2d", "Sprite", "Headless anicraft-cli sprite cycle"),
                    ("cutout-2-5d", "Cutout", "2.5D PhotoCraft + EffectCraft rig"),
                    ("rig-3d", "Model", "EffectCraft 3D model rig"),
                ] {
                    let btn_id = format!("style-tab-{tid}");
                    let sel = ctx.state.animation.style_tab == tid;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "cube", word, tip).selected(sel)).clicked() {
                        ctx.state.animation.style_tab = tid.into();
                    }
                }
            });

            ui.separator();
            ui.label(RichText::new("Action").size(12.0).strong().color(ctx.pal.accent_sky));
            ui.horizontal_wrapped(|ui| {
                for act in ["idle", "jump", "swim", "turn", "wave", "rest"] {
                    let btn_id = format!("anim-act-{act}");
                    let sel = ctx.state.animation.selected_action == act;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "play", act, "Select action cycle").selected(sel)).clicked() {
                        ctx.state.animation.selected_action = act.into();
                        ctx.exec("character.action", json!({"character": cid, "action": act}), Some("Keyed character action"));
                    }
                }
            });
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-cycle-duration", "Cycle duration", 0.4..=8.0, "s", ctx.pal.accent_sky),
                &mut ctx.state.animation.action_duration,
            );
            pill_toggle(ui, ctx.reg, ctx.pal, "anim-loop-cycle", "Loop action cycle", &mut ctx.state.animation.loop_clip, ctx.pal.accent_sky);

            ui.separator();
            let amb = ctx.pal.accent_amber;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Path & depth").size(12.0).strong().color(amb));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.0}% → {:.0}%", ctx.state.animation.start_size_pct, ctx.state.animation.end_size_pct))
                            .monospace()
                            .size(10.0)
                            .color(ctx.pal.text_muted),
                    );
                });
            });
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
            ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-end-size", "End size", 20.0..=250.0, "%", amb), &mut ctx.state.animation.end_size_pct);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("anim-apply-path", "check", "Apply", "Save character walk path").primary()).clicked() {
                ctx.exec(
                    "character.save_path",
                    json!({
                        "character": cid,
                        "points": [[ctx.state.animation.path_x, ctx.state.animation.path_y]],
                    }),
                    Some("Saved character motion path"),
                );
            }
        });
    });
    ctx.reg.record("motion-layers", "Action & Path", &panel.response, None);
}

fn draw_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            pill_toggle(ui, ctx.reg, ctx.pal, "motion-grid", "Grid", &mut ctx.state.animation.show_grid, ctx.pal.accent_sky);
            pill_toggle(ui, ctx.reg, ctx.pal, "motion-marks", "Pins", &mut ctx.state.animation.show_marks, ctx.pal.accent_emerald);
            pill_toggle(ui, ctx.reg, ctx.pal, "motion-path-visible", "Path", &mut ctx.state.animation.show_path_hud, ctx.pal.accent_amber);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-export", "film", "Publish", "Render ProRes 4444 V2 pass").primary()).clicked() {
                ctx.exec(
                    "character.shot",
                    json!({"character": cid, "sprite": ctx.state.animation.selected_action, "duration": ctx.state.animation.action_duration, "studio": true}),
                    Some("Published V2 Character Animation"),
                );
            }
        });

        let avail = ui.available_size();
        let (outer_rect, resp) = ui.allocate_exact_size(Vec2::new(avail.x.max(200.0), (avail.y - 8.0).max(160.0)), Sense::click());
        let inner = paint_filmbar_frame(
            ui,
            outer_rect,
            ctx.pal,
            FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected: false, hovered: resp.hovered(), timecode: Some("V2 · ProRes 4444") },
        );
        if ctx.state.animation.show_grid {
            let mid = inner.center();
            ui.painter().line_segment([Pos2::new(inner.min.x, mid.y), Pos2::new(inner.max.x, mid.y)], Stroke::new(1.0, ctx.pal.border_subtle));
            ui.painter().line_segment([Pos2::new(mid.x, inner.min.y), Pos2::new(mid.x, inner.max.y)], Stroke::new(1.0, ctx.pal.border_subtle));
        }
        if ctx.state.animation.show_path_hud {
            let p_start = Pos2::new(inner.min.x + inner.width() * 0.25, inner.max.y - 34.0);
            let p_mid = Pos2::new(inner.center().x, inner.center().y + 18.0);
            let p_end = Pos2::new(inner.min.x + inner.width() * 0.76, inner.center().y - 8.0);
            ui.painter().line_segment([p_start, p_mid], Stroke::new(2.0, ctx.pal.accent_amber));
            ui.painter().line_segment([p_mid, p_end], Stroke::new(2.0, ctx.pal.accent_amber));
            for p in [p_start, p_mid, p_end] {
                ui.painter().circle_filled(p, 4.0, ctx.pal.accent_amber);
            }
        }
        if ctx.state.animation.show_marks {
            let head = Pos2::new(inner.center().x, inner.center().y - 22.0);
            for (dx, dy) in [(-18.0, -6.0), (18.0, -6.0), (0.0, -16.0), (-10.0, 8.0), (10.0, 8.0), (0.0, 16.0)] {
                let pin = Pos2::new(head.x + dx, head.y + dy);
                ui.painter().circle_filled(pin, 3.5, ctx.pal.bg_input);
                ui.painter().circle_stroke(pin, 3.5, Stroke::new(1.6, ctx.pal.accent_emerald));
            }
        }
        ui.painter().text(
            Pos2::new(inner.center().x, inner.min.y + 18.0),
            Align2::CENTER_CENTER,
            format!("{cid} · {} · viseme {}", ctx.state.animation.selected_action, ctx.state.animation.viseme),
            FontId::proportional(12.5),
            ctx.pal.text_primary,
        );
        ctx.reg.record("motion-canvas", "Animation Canvas", &resp, None);
    });
    ctx.reg.record("motion-stage", "Animation Stage", &panel.response, None);
}

fn draw_puppet_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ScrollArea::vertical().id_salt("anim_puppet_scroll").show(ui, |ui| {
            ui.label(RichText::new("Puppet & Face Visemes (EffectCraft)").size(12.0).strong().color(ctx.pal.accent_amber));
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
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("authoring-puppet-rig-face", "wand", "Face", "Rig 11-pin facial mesh")).clicked() {
                    ctx.exec("character.puppet.rig_face", json!({"character": cid}), Some("Rigged 11-pin facial mesh"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("authoring-puppet-lipsync", "music", "Lipsync", "Sync mouth visemes to song").primary())
                    .clicked()
                {
                    ctx.exec(
                        "character.puppet.lipsync",
                        json!({"character": cid, "intensity": ctx.state.animation.lip_intensity / 100.0}),
                        Some("Synced mouth visemes"),
                    );
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("authoring-puppet-follow", "sparkles", "Follow", "Cascade secondary follow pins")).clicked()
                {
                    ctx.exec("character.puppet.follow", json!({"character": cid, "delay": ctx.state.animation.follow_delay}), Some("Applied puppet.follow"));
                }
            });

            ui.label(RichText::new("Viseme").size(10.5).color(ctx.pal.text_muted));
            ui.horizontal_wrapped(|ui| {
                for v in ["AA", "EE", "OH", "OO", "MBP", "Rest"] {
                    let id = format!("puppet-viseme-{v}");
                    let sel = ctx.state.animation.viseme == v;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, "music", v, "Preview mouth viseme").selected(sel)).clicked() {
                        ctx.state.animation.viseme = v.into();
                    }
                }
            });

            ui.label(RichText::new("Expressions").size(10.5).color(ctx.pal.text_muted));
            ui.horizontal_wrapped(|ui| {
                for expr in ["happy", "surprised", "focused", "blink", "neutral"] {
                    let id = format!("puppet-expr-{expr}");
                    let sel = ctx.state.animation.expression == expr;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, "sparkles", expr, "Keyframe expression").selected(sel)).clicked() {
                        ctx.state.animation.expression = expr.into();
                        ctx.exec("character.puppet.expression", json!({"character": cid, "expression": expr}), Some("Keyframed expression"));
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("authoring-part-apply", "check", "Apply", "Write part transform to rig")).clicked() {
                ctx.exec(
                    "character.set",
                    json!({
                        "character": cid,
                        "part": ctx.state.animation.selected_part,
                        "prop": "rotation",
                        "value": ctx.state.animation.bone_rotation,
                        "time": ctx.state.animation.scrub_seconds,
                    }),
                    Some("Updated part transform"),
                );
            }
            status_pill(ui, ctx.pal, "V2 · Character Animation", ctx.pal.accent_emerald);
        });
    });
    ctx.reg.record("motion-inspector", "Puppet Inspector", &panel.response, None);
}

fn draw_theatre_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let dock = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            for (tid, word) in [("theatre", "Theatre"), ("keys", "Keys"), ("curves", "Curves")] {
                let id = format!("anim-dock-{tid}");
                let sel = ctx.state.animation.dock_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, "sliders", word, "Switch animation timeline mode").selected(sel)).clicked() {
                    ctx.state.animation.dock_tab = tid.into();
                }
            }
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("motion-scrub", "Scrub", 0.0..=ctx.state.animation.action_duration.max(4.0), "s", ctx.pal.accent_emerald),
                &mut ctx.state.animation.scrub_seconds,
            );
        });

        let w = ui.available_width().max(160.0);
        let (track_rect, _) = ui.allocate_exact_size(Vec2::new(w, 42.0), Sense::hover());
        ui.painter().rect_filled(track_rect, CornerRadius::same(6), ctx.pal.bg_input);
        for i in 1..9 {
            let x = track_rect.min.x + (i as f32) * (w / 9.5);
            let y = track_rect.min.y + 13.0;
            ui.painter().circle_filled(Pos2::new(x, y), 3.2, ctx.pal.accent_amber);
            let y2 = track_rect.max.y - 11.0 - ((i % 3) as f32) * 4.0;
            ui.painter().circle_filled(Pos2::new(x, y2), 2.5, ctx.pal.accent_sky);
        }
    });
    ctx.reg.record("anim-theatre-dock", "Theatre Dock", &dock.response, None);
}
