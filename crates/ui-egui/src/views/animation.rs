//! `Animations` view (`#view-animation`): Craft-project-native character rigging & animation
//! matching `04-animations-craft-rig.jpg`, with underlying audio transport and `EffectCraft`
//! (`V2 .ecproj`) ↔ `FilmCraft` (`V1 .fcproj`) track switching.

use std::collections::HashMap;

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align2, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Shape, Stroke, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, ScalarSpec, child_pane, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image, pill_toggle,
    ring_slider_h, xy_path_pad,
};
use crate::state::{UiAction, ViewCtx};
use crate::views::animation_dock::draw_theatre_dock;
use crate::views::animation_inspector::{draw_puppet_and_3d_inspector, draw_time_and_stagger_card};
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_roster_bar(ui, ctx);
    ui.add_space(6.0);
    let cid = active_character(ctx);
    let remaining = ui.available_rect_before_wrap();
    let dock_h = if ctx.state.animation.dock_tab == "curves" { 214.0 } else { 194.0 };
    if ctx.state.show_bottom_dock
        && let Some(vert) = split_rect(remaining, SplitAxis::Vertical, SplitSize::FixedSecond(dock_h), 8.0, 200.0)
    {
        let mut upper = child_pane(ui, vert.first, "anim_upper");
        draw_upper_columns(&mut upper, ctx);
        let mut dock = child_pane(ui, vert.second, "anim_theatre_dock");
        draw_theatre_dock(&mut dock, ctx, &cid);
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

fn humanize_slug(raw: &str) -> String {
    if raw.contains(' ') {
        return raw.to_owned();
    }
    raw.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|part| {
            let mut ch = part.chars();
            match ch.next() {
                Some(f) => format!("{}{}", f.to_uppercase(), ch.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_character_display_name(ch: &Value) -> String {
    let id = ch["id"].as_str().unwrap_or("character");
    let raw = ch["name"].as_str().unwrap_or(id);
    let generic = matches!(raw.to_ascii_lowercase().as_str(), "parts" | "front" | "side" | "back" | "sprites" | "cutout" | "rig" | "layers");
    if !generic {
        return humanize_slug(raw);
    }
    if let Some(grp) = ch["group"].as_str().filter(|g| !g.is_empty() && !g.eq_ignore_ascii_case("animations")) {
        return humanize_slug(grp);
    }
    for ptr in ["/folder", "/views/0/drawing", "/views/0/latest_frame", "/views/0/pcraft_path", "/views/0/ecproj_path"] {
        if let Some(path_str) = ch.pointer(ptr).and_then(Value::as_str) {
            let segs: Vec<&str> = path_str
                .split(['/', '\\'])
                .filter(|s| {
                    !s.is_empty()
                        && !s.contains('.')
                        && !matches!(
                            s.to_ascii_lowercase().as_str(),
                            "assets" | "characters" | "animations" | "parts" | "sprites" | "frames" | "front" | "side" | "back" | "source" | "rig"
                        )
                })
                .collect();
            if let Some(last) = segs.last() {
                return humanize_slug(last);
            }
        }
    }
    if !matches!(id.to_ascii_lowercase().as_str(), "parts" | "front" | "cutout") { humanize_slug(id) } else { "Character Rig".into() }
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
    active_character_obj(ctx).and_then(|c| c["id"].as_str().map(str::to_owned)).or_else(|| ctx.state.selected_character_id.clone()).unwrap_or_default()
}

fn active_view_obj(char_obj: Option<&Value>) -> Option<Value> {
    char_obj?["views"].as_array()?.first().cloned()
}

fn select_rendered_character_frame(ctx: &ViewCtx<'_>, char_obj: Option<&Value>, view_obj: Option<&Value>) -> Option<String> {
    if let Some(last) = &ctx.state.cache.last_character_frame {
        return Some(last.clone());
    }
    if let Some(sprites) = char_obj.and_then(|c| c["sprites"].as_array())
        && let Some(sp) = sprites.iter().find(|s| s["name"].as_str() == Some(ctx.state.animation.selected_action.as_str())).or_else(|| sprites.first())
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

fn draw_upper_columns(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let char_obj = active_character_obj(ctx);
    let view_obj = active_view_obj(char_obj.as_ref());
    let body = ui.available_rect_before_wrap();
    let left_w = if body.width() >= 1180.0 { 286.0 } else { 246.0 };
    let right_w = if body.width() >= 1180.0 { 320.0 } else { 272.0 };

    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(left_w), 8.0, 200.0)
    {
        let mut left = child_pane(ui, left_split.first, "anim_left");
        draw_action_and_path_column(&mut left, ctx);
        left_split.second
    } else {
        body
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(right_w), 8.0, 220.0)
    {
        let mut stage = child_pane(ui, right_split.first, "anim_stage");
        draw_stage(&mut stage, ctx);
        let mut right = child_pane(ui, right_split.second, "anim_puppet");
        draw_puppet_and_3d_inspector(&mut right, ctx, &cid, view_obj.as_ref());
    } else {
        let mut stage = child_pane(ui, after_left, "anim_stage");
        draw_stage(&mut stage, ctx);
    }
    let _ = ui.allocate_rect(body, Sense::hover());
}

fn draw_roster_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let chars = character_entries(ctx);
    let current = active_character(ctx);
    let has_char = !current.is_empty();
    let mut name_counts: HashMap<String, usize> = HashMap::new();

    let bar = ctx.pal.panel_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(RichText::new("CHARACTER ROSTER:").monospace().size(10.0).color(ctx.pal.text_muted));
            if chars.is_empty() {
                ui.label(RichText::new("No .pcraft / .ecproj characters yet — import via + Add Asset or Scribbles").size(11.0).color(ctx.pal.text_muted));
            }
            for (idx, ch) in chars.iter().enumerate() {
                let id = ch["id"].as_str().unwrap_or_default();
                let base_name = resolve_character_display_name(ch);
                let seen = name_counts.entry(base_name.clone()).or_insert(0);
                *seen += 1;
                let unique_name = if *seen > 1 { format!("{base_name} #{}", idx + 1) } else { base_name };

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
                    match ch["animation_style"].as_str().unwrap_or("cutout") {
                        "cutout_2_5d" => "2.5D rig",
                        "sprite_2d" => "2D sprite",
                        "rig_3d" => "3D rig",
                        other => other,
                    }
                    .to_owned()
                } else {
                    craft_tags.join(" · ")
                };
                let pill_lbl = format!("{unique_name} ({tag_str})");
                let btn_id = format!("anim-char-{id}");
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "user", &pill_lbl, &unique_name).selected(current == id)).clicked() {
                    ctx.push(UiAction::SelectCharacter { id: id.to_owned(), jump_animation: false });
                }
            }

            ui.label(RichText::new("·").color(ctx.pal.border));
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("character-mark-run", "pin", "Mark", "Mark puppet joints").enabled(has_char)).clicked() {
                ctx.exec("character.mark", json!({"character": current}), Some("Marked puppet joints"));
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
                BtnSpec::new("character-rig-run", "wand", "Rig .ecproj", "Build EffectCraft .ecproj from .pcraft/.psd").primary().enabled(has_char),
            )
            .clicked()
            {
                ctx.exec("character.rig", json!({"character": current}), Some("Built EffectCraft puppet rig (.ecproj)"));
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
                BtnSpec::new("character-open-effectcraft", "external", "EffectCraft", "Open .ecproj in EffectCraft").enabled(has_char),
            )
            .clicked()
            {
                ctx.exec("character.open", json!({"character": current, "app": "effectcraft"}), Some("Opened .ecproj in EffectCraft"));
            }
            let active_ecproj = active_character_obj(ctx).and_then(|c| {
                c.pointer("/views/0/ecproj_path")
                    .and_then(Value::as_str)
                    .or_else(|| c.pointer("/action_ecprojs/0/path").and_then(Value::as_str))
                    .map(str::to_owned)
            });
            if let Some(ec_path) = active_ecproj
                && action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("character-link-filmcraft", "link", "Link V2", "Dynamic-link character .ecproj onto FilmCraft V2"),
                )
                .clicked()
            {
                ctx.exec("video.link_effectcraft", json!({"project": ec_path, "track": "V2"}), Some("Linked character .ecproj onto FilmCraft V2"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("anim-scribbles-open", "sparkles", "Scribbles", "Open Fidget & Scribbles character/sheet stager").selected(ctx.state.fidget_open),
            )
            .clicked()
            {
                ctx.state.fidget_open = true;
            }
        });
    });
    ctx.reg.record("anim-roster-bar", "Character Roster", &bar.response, None);
}

fn draw_action_and_path_column(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let cid = active_character(ctx);
    let scroll_out = ScrollArea::vertical().id_salt("anim_left_scroll").auto_shrink([false, false]).show(ui, |ui| {
        fill_card(ui, ctx.pal, false, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Animation Style").size(11.5).strong());
                let mode_lbl = match ctx.state.animation.style_tab.as_str() {
                    "sprite-2d" => "2D Sprite",
                    "rig-3d" => "3D Rig",
                    _ => "2.5D Cutout Rig",
                };
                ui.label(RichText::new(mode_lbl).size(10.5).color(ctx.pal.text_muted));
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
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
                ui.label(RichText::new(&ctx.state.animation.selected_action).monospace().size(10.0).color(ctx.pal.text_muted));
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().item_spacing.y = 4.0;
                for (act, lbl) in [("idle", "Idle"), ("fly", "Fly"), ("sing", "Sing"), ("jump", "Jump"), ("turn", "Turn"), ("wave", "Wave"), ("rest", "Rest")] {
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
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-cycle-duration", "Cycle duration", 0.4..=8.0, "s", ctx.pal.accent_sky),
                &mut ctx.state.animation.action_duration,
            );
            pill_toggle(ui, ctx.reg, ctx.pal, "anim-loop-cycle", "Loop action cycle", &mut ctx.state.animation.loop_clip, ctx.pal.accent_sky);
        });
        ui.add_space(6.0);

        let amb = ctx.pal.accent_amber;
        fill_card(ui, ctx.pal, false, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("✎ Path & depth").size(12.0).strong().color(amb));
                ui.label(
                    RichText::new(format!("{:.0}% → {:.0}%", ctx.state.animation.start_size_pct, ctx.state.animation.end_size_pct))
                        .monospace()
                        .size(10.0)
                        .color(ctx.pal.text_muted),
                );
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
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-clear-path", "Clear", "Clear root path in .ecproj")).clicked()
                    && !cid.is_empty()
                {
                    ctx.exec("character.save_path", json!({"character": cid, "points": []}), Some("Cleared walk path in .ecproj"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-undo-point", "Undo point", "Step back path coordinate")).clicked() {
                    ctx.state.animation.path_x = (ctx.state.animation.path_x - 40.0).max(0.0);
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only("anim-apply-path", "Apply path", "Write walk path keyframes into .ecproj").primary().enabled(!cid.is_empty()),
                )
                .clicked()
                {
                    ctx.exec(
                        "character.save_path",
                        json!({
                            "character": cid,
                            "points": [
                                [200.0, ctx.state.animation.path_y],
                                [ctx.state.animation.path_x, ctx.state.animation.path_y]
                            ],
                            "duration": ctx.state.animation.travel_duration,
                            "start_scale": ctx.state.animation.start_size_pct,
                            "end_scale": ctx.state.animation.end_size_pct,
                        }),
                        Some("Applied walk path to .ecproj"),
                    );
                }
            });
        });
        ui.add_space(6.0);
        draw_time_and_stagger_card(ui, ctx, &cid);
    });
    let resp = ui.interact(scroll_out.inner_rect, ui.scope_id().with("motion_layers"), Sense::hover());
    ctx.reg.record("motion-layers", "Action & Path", &resp, None);
}

fn draw_stage_audio_and_track_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, total_marks: usize) {
    let has_audio = ctx.state.song_audio_path().is_some();
    let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
    let play_lbl = if ctx.state.audio_playing { "Pause" } else { "Play Audio" };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-audio-play", play_icon, play_lbl, "Play / pause underlying song or FilmCraft audio track").primary().enabled(has_audio),
        )
        .clicked()
        {
            ctx.push(UiAction::ToggleAudio);
        }
        ui.label(RichText::new(format!("0:{:04.1}", ctx.state.animation.scrub_seconds)).monospace().size(9.8).color(if ctx.state.audio_playing {
            ctx.pal.accent_emerald
        } else {
            ctx.pal.text_muted
        }));
        for (src, lbl, tip) in [
            ("effectcraft", "EC · V2 Rig", "Isolate EffectCraft .ecproj character rig track"),
            ("filmcraft", "FC · Film Track", "Preview underlying FilmCraft V1 / program sequence track"),
            ("overlay", "FC + EC", "Composite EffectCraft V2 rig over FilmCraft V1 sequence"),
        ] {
            let id = format!("anim-track-src-{src}");
            let sel = ctx.state.animation.track_source == src;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.animation.track_source = src.into();
                ctx.exec("video.frame", json!({"seconds": ctx.state.animation.scrub_seconds}), None);
            }
        }
        for (aud, lbl, tip) in
            [("song", "♪ Song", "Monitor underlying song audio stem"), ("filmcraft", "FC Mix", "Monitor rendered FilmCraft sequence audio mix")]
        {
            let id = format!("anim-audio-src-{aud}");
            let sel = ctx.state.animation.audio_source == aud;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.animation.audio_source = aud.into();
                if ctx.state.audio_playing {
                    let t = ctx.state.animation.scrub_seconds;
                    ctx.push(UiAction::SeekAudio(t));
                }
            }
        }
        pill_toggle(ui, ctx.reg, ctx.pal, "motion-grid", "Grid", &mut ctx.state.animation.show_grid, ctx.pal.accent_sky);
        let marks_lbl = format!("Marks ({total_marks})");
        pill_toggle(ui, ctx.reg, ctx.pal, "motion-marks", &marks_lbl, &mut ctx.state.animation.show_marks, ctx.pal.accent_emerald);
        pill_toggle(ui, ctx.reg, ctx.pal, "motion-path-visible", "Path", &mut ctx.state.animation.show_path_hud, ctx.pal.accent_amber);
        if ctx.state.animation.puppet_record_live {
            status_pill(ui, ctx.pal, "REC Pin Drag", ctx.pal.danger);
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-render-frame", "image", "Render", "Render active EffectCraft or FilmCraft frame at playhead")
                .enabled(!cid.is_empty() || ctx.state.animation.track_source == "filmcraft"),
        )
        .clicked()
        {
            if ctx.state.animation.track_source == "filmcraft" {
                ctx.exec("video.frame", json!({"seconds": ctx.state.animation.scrub_seconds}), Some("Rendered FilmCraft frame"));
            } else {
                ctx.exec("character.frame", json!({"character": cid, "time": ctx.state.animation.scrub_seconds}), Some("Rendered EffectCraft frame"));
            }
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("motion-export", "film", "Publish V2", "Render ProRes 4444 V2 pass into FilmCraft").primary().enabled(!cid.is_empty()),
        )
        .clicked()
        {
            ctx.exec(
                "character.shot",
                json!({"character": cid, "sprite": ctx.state.animation.selected_action, "duration": ctx.state.animation.action_duration, "studio": true}),
                Some("Published V2 Character Animation"),
            );
        }
    });
}

fn draw_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let char_obj = active_character_obj(ctx);
    let view_obj = active_view_obj(char_obj.as_ref());
    let cid = active_character(ctx);
    let frame_path = select_rendered_character_frame(ctx, char_obj.as_ref(), view_obj.as_ref());
    let film_bg = ctx.state.film_frame_at_time(ctx.state.animation.scrub_seconds);
    let joints = view_obj.as_ref().and_then(|v| v["joints"].as_array()).cloned().unwrap_or_default();
    let pins = view_obj.as_ref().and_then(|v| v["pins"].as_array()).cloned().unwrap_or_default();
    let total_marks = joints.len() + pins.len();
    let ecproj_file = view_obj.as_ref().and_then(|v| v["ecproj_path"].as_str().or_else(|| v["photo_doc"].as_str())).unwrap_or("assets/characters/rig.ecproj");

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        draw_stage_audio_and_track_bar(ui, ctx, &cid, total_marks);

        let avail = ui.available_size();
        let sense = if ctx.state.animation.puppet_record_live { Sense::click_and_drag() } else { Sense::click() };
        let (outer_rect, resp) = ui.allocate_exact_size(Vec2::new(avail.x.max(180.0), (avail.y - 6.0).max(140.0)), sense);
        let tc_badge = match ctx.state.animation.track_source.as_str() {
            "filmcraft" => "V1 · FilmCraft .fcproj",
            "overlay" => "FC V1 + EC V2 · Composite",
            _ => "V2 · .pcraft / .ecproj",
        };
        let inner = paint_filmbar_frame(
            ui,
            outer_rect,
            ctx.pal,
            FilmbarStyle {
                orientation: FilmbarOrientation::Horizontal,
                selected: ctx.state.animation.puppet_record_live,
                hovered: resp.hovered(),
                timecode: Some(tc_badge),
            },
        );
        ui.painter().rect_filled(inner, egui::CornerRadius::same(4), ctx.pal.bg_stage);
        if matches!(ctx.state.animation.track_source.as_str(), "filmcraft" | "overlay") {
            let painted_fc = paint_local_image(ui, inner, film_bg.as_deref(), ctx.pal.bg_stage, 4);
            if ctx.state.animation.track_source == "filmcraft" && !painted_fc {
                paint_craft_doc_card(
                    ui,
                    inner.shrink(16.0),
                    ctx.pal,
                    "FC",
                    "film/Song.fcproj",
                    "Click Assemble in Studio or Render to preview FilmCraft V1 track",
                );
            }
        }

        if ctx.state.animation.show_grid || ctx.state.animation.view_layout != "active" {
            let mid = inner.center();
            ui.painter().line_segment([Pos2::new(inner.min.x, mid.y), Pos2::new(inner.max.x, mid.y)], Stroke::new(1.0, ctx.pal.border_subtle));
            ui.painter().line_segment([Pos2::new(mid.x, inner.min.y), Pos2::new(mid.x, inner.max.y)], Stroke::new(1.0, ctx.pal.border_subtle));
            if ctx.state.animation.view_layout != "active" {
                ui.painter().text(
                    Pos2::new(inner.min.x + 10.0, inner.min.y + 10.0),
                    Align2::LEFT_TOP,
                    format!("3D VIEW · {}", ctx.state.animation.view_layout.to_ascii_uppercase()),
                    FontId::monospace(9.5),
                    ctx.pal.accent_sky,
                );
            }
        }

        let char_h = (inner.height() * 0.78).clamp(80.0, 320.0);
        let char_w = (inner.width() * 0.72).min(char_h * 1.35).max(80.0);
        let char_rect = Rect::from_center_size(inner.center(), Vec2::new(char_w, char_h));
        if ctx.state.animation.track_source != "filmcraft" {
            if !paint_local_image(ui, char_rect, frame_path.as_deref(), egui::Color32::TRANSPARENT, 4) {
                paint_craft_doc_card(
                    ui,
                    char_rect,
                    ctx.pal,
                    "EC",
                    ecproj_file.rsplit('/').next().unwrap_or(ecproj_file),
                    "Click Render or Rig .ecproj to render character",
                );
            }
            capture_live_puppet_drag(ui, ctx, &cid, char_rect, &resp);
            paint_stage_overlays(ui, ctx, inner, char_rect, view_obj.as_ref(), &joints, &pins);
        }
        ctx.reg.record("motion-canvas", "Animation Canvas", &resp, None);
    });
    ctx.reg.record("motion-stage", "Animation Stage", &panel.response, None);
}

fn capture_live_puppet_drag(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, char_rect: Rect, resp: &egui::Response) {
    if !ctx.state.animation.puppet_record_live {
        return;
    }
    if ctx.state.animation.puppet_show_mesh {
        for gx in 1..5 {
            let fx = (gx as f32) / 5.0;
            let x = char_rect.min.x + fx * char_rect.width();
            ui.painter()
                .line_segment([Pos2::new(x, char_rect.min.y), Pos2::new(x, char_rect.max.y)], Stroke::new(0.8, ctx.pal.accent_emerald.gamma_multiply(0.32)));
        }
    }
    if resp.dragged()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let nx = ((pos.x - char_rect.min.x) / char_rect.width().max(1.0)).clamp(0.0, 1.0) as f64;
        let ny = ((pos.y - char_rect.min.y) / char_rect.height().max(1.0)).clamp(0.0, 1.0) as f64;
        let t = ctx.state.animation.scrub_seconds + (ctx.state.animation.puppet_recorded_samples.len() as f64) / 24.0;
        ctx.state.animation.puppet_recorded_samples.push([t, nx * 1024.0, ny * 1024.0]);
    }
    if ctx.state.animation.puppet_recorded_samples.len() >= 2 {
        let pts: Vec<Pos2> = ctx
            .state
            .animation
            .puppet_recorded_samples
            .iter()
            .map(|s| Pos2::new(char_rect.min.x + (s[1] as f32 / 1024.0) * char_rect.width(), char_rect.min.y + (s[2] as f32 / 1024.0) * char_rect.height()))
            .collect();
        ui.painter().add(Shape::line(pts, Stroke::new(2.0, ctx.pal.danger)));
    }
    if resp.drag_stopped() && !cid.is_empty() && !ctx.state.animation.puppet_recorded_samples.is_empty() {
        let samples: Vec<Value> = ctx.state.animation.puppet_recorded_samples.drain(..).map(|s| json!([s[0], s[1], s[2]])).collect();
        ctx.exec(
            "character.puppet.record",
            json!({
                "character": cid,
                "part": ctx.state.animation.selected_part,
                "smoothing": ctx.state.animation.puppet_smoothing,
                "draft_deform": ctx.state.animation.puppet_draft_deform,
                "show_mesh": ctx.state.animation.puppet_show_mesh,
                "samples": samples,
            }),
            Some("Recorded live Puppet pin take into .ecproj"),
        );
    }
}

fn paint_stage_overlays(ui: &mut Ui, ctx: &ViewCtx<'_>, inner: Rect, char_rect: Rect, view_obj: Option<&Value>, joints: &[Value], pins: &[Value]) {
    if ctx.state.animation.show_path_hud
        && let Some(pts) = ctx.state.cache.character_path["points"].as_array()
        && pts.len() >= 2
    {
        let mapped: Vec<Pos2> = pts
            .iter()
            .filter_map(|pt| {
                let x = pt["x"].as_f64()? as f32;
                let y = pt["y"].as_f64()? as f32;
                Some(Pos2::new(inner.min.x + (x / 1920.0).clamp(0.02, 0.98) * inner.width(), inner.min.y + (y / 1080.0).clamp(0.02, 0.98) * inner.height()))
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

    if ctx.state.animation.show_marks && (!joints.is_empty() || !pins.is_empty()) {
        let (cw, ch) = view_obj
            .and_then(|v| v["canvas_size"].as_array())
            .filter(|a| a.len() == 2)
            .and_then(|a| Some((a[0].as_f64()? as f32, a[1].as_f64()? as f32)))
            .unwrap_or((1024.0, 1024.0));
        for j in joints {
            if let Some(arr) = j["joint"].as_array().filter(|a| a.len() == 2)
                && let (Some(jx), Some(jy)) = (arr[0].as_f64(), arr[1].as_f64())
            {
                let pt = Pos2::new(
                    char_rect.min.x + (jx as f32 / cw.max(1.0)).clamp(0.0, 1.0) * char_rect.width(),
                    char_rect.min.y + (jy as f32 / ch.max(1.0)).clamp(0.0, 1.0) * char_rect.height(),
                );
                ui.painter().circle_filled(pt, 4.0, ctx.pal.bg_input);
                ui.painter().circle_stroke(pt, 4.0, Stroke::new(1.8, ctx.pal.accent_amber));
            }
        }
        for pin in pins {
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
}
