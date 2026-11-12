//! `Project` overview & Craft file trunk (`#view-project`): precision match for
//! `02-project-craft-files.jpg` (`166374e1.jpeg`) plus `Sheet 08` Zone A `FilmCraft` × `EffectCraft`
//! bridge drawers (`project_bridge.rs`: `T01`–`T05`, `T08`, `T10`).

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{child_pane, fill_card, paint_craft_badge, paint_local_image, paint_readiness_light};
use crate::icons::icon_image;
use crate::state::{PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::views::project_bridge::{draw_offline_and_source_guard_card, draw_trunk_bridge_buttons, draw_trunk_bridge_drawers};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let body = ui.available_rect_before_wrap();
    let left_w = if body.width() >= 1120.0 { 296.0 } else { 264.0 };
    if let Some(split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(left_w), 14.0, 280.0) {
        let div_x = split.first.max.x + 7.0;
        ui.painter().line_segment([Pos2::new(div_x, body.min.y), Pos2::new(div_x, body.max.y)], Stroke::new(1.0, ctx.pal.border_subtle));
        let mut left_ui = child_pane(ui, split.first, "project_left");
        draw_stage_stack(&mut left_ui, ctx);
        let mut right_ui = child_pane(ui, split.second, "project_right");
        draw_files_trunk(&mut right_ui, ctx);
        let _ = ui.allocate_rect(body, Sense::hover());
    } else {
        draw_stage_stack(ui, ctx);
    }
}

fn collect_lyric_lines(ctx: &ViewCtx<'_>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(sections) = ctx.state.cache.song_lyrics.get("sections").and_then(Value::as_array) {
        for sec in sections {
            if let Some(lines) = sec.get("lines").and_then(Value::as_array) {
                for line in lines {
                    if let Some(t) = line["text"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        out.push(t.to_owned());
                    }
                }
            }
        }
    }
    if out.is_empty()
        && let Some(anims) = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array)
    {
        for anim in anims.iter().take(8) {
            let line: String = anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).collect::<Vec<_>>().join(" ");
            if !line.trim().is_empty() {
                out.push(line);
            }
        }
    }
    out
}

fn draw_stage_stack(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ScrollArea::vertical().id_salt("project_stage_scroll").show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let (ir, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                icon_image("sidebar", 15.0, ctx.pal.text_secondary).paint_at(ui, ir);
                ui.label(RichText::new("Song & Stages").size(13.0).strong().color(ctx.pal.text_primary));
            });
            ui.add_space(6.0);
            draw_song_and_lyrics_card(ui, ctx);
            ui.add_space(8.0);
            draw_offline_and_source_guard_card(ui, ctx);
            ui.add_space(8.0);
            draw_pipeline_jumps(ui, ctx);
        });
    });
}

fn draw_song_waveform(ui: &mut Ui, ctx: &mut ViewCtx<'_>, song_file: &str, has_song: bool) {
    let w = (ui.available_width() - 2.0).max(140.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::click());
    let base_y = rect.max.y - 3.0;
    ui.painter().line_segment([Pos2::new(rect.min.x, base_y), Pos2::new(rect.max.x, base_y)], Stroke::new(1.0, ctx.pal.border));
    if has_song {
        let mut seed: u32 = 0x5A17_9C21;
        for b in song_file.bytes() {
            seed = seed.wrapping_mul(31).wrapping_add(u32::from(b));
        }
        let bars = ((w / 3.0) as usize).clamp(24, 96);
        let dur = ctx.state.audio_duration_sec.max(1.0);
        let play_frac = (ctx.state.audio_time_sec / dur).clamp(0.0, 1.0) as f32;
        for i in 0..bars {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let frac = (i as f32) / (bars as f32);
            let env = (frac * std::f32::consts::PI).sin().powf(0.55);
            let amp = (0.18 + ((seed >> 16) as f32 / 65535.0) * 0.82) * env;
            let bh = (amp * 13.0).max(2.0);
            let x = rect.min.x + frac * rect.width();
            let col = if frac >= play_frac { ctx.pal.accent_emerald.gamma_multiply(0.82) } else { ctx.pal.accent_emerald.gamma_multiply(0.36) };
            ui.painter().line_segment([Pos2::new(x, base_y - bh), Pos2::new(x, base_y)], Stroke::new(1.8, col));
        }
        let px = rect.min.x + play_frac * rect.width();
        ui.painter().line_segment([Pos2::new(px, rect.min.y + 2.0), Pos2::new(px, base_y + 2.0)], Stroke::new(1.6, Color32::WHITE));
        if resp.clicked()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            let next_f = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0) as f64;
            ctx.state.audio_time_sec = next_f * dur;
        }
    }
}

fn recessed_action_button(ui: &mut Ui, ctx: &mut ViewCtx<'_>, id: &str, label: &str, centered: bool, tooltip: &str) -> bool {
    let w = (ui.available_width() - 2.0).max(88.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    let fill = if resp.hovered() { ctx.pal.bg_hover } else { ctx.pal.bg_input };
    let border = if resp.hovered() { ctx.pal.accent_emerald } else { ctx.pal.border };
    ui.painter().rect(rect, CornerRadius::same(6), fill, Stroke::new(1.0, border), StrokeKind::Inside);
    if centered {
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.5), ctx.pal.text_primary);
    } else {
        ui.painter().text(Pos2::new(rect.min.x + 10.0, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(11.5), ctx.pal.text_muted);
    }
    let resp = resp.on_hover_text(tooltip);
    ctx.reg.record(id, label, &resp, None);
    resp.clicked()
}

fn draw_section_status_header(ui: &mut Ui, light_color: Color32, title: &str, title_color: Color32, right_meta: &str, muted_color: Color32) {
    let w = (ui.available_width() - 2.0).max(140.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
    paint_readiness_light(ui, Pos2::new(rect.min.x + 7.0, rect.center().y), light_color);
    ui.painter().text(Pos2::new(rect.min.x + 20.0, rect.center().y), Align2::LEFT_CENTER, title, FontId::proportional(12.5), title_color);
    let max_meta_chars = (((rect.width() - 120.0).max(48.0) / 6.2) as usize).max(8);
    let shown_meta = if right_meta.chars().count() > max_meta_chars {
        let mut s: String = right_meta.chars().take(max_meta_chars.saturating_sub(1)).collect();
        s.push('…');
        s
    } else {
        right_meta.to_owned()
    };
    ui.painter().text(Pos2::new(rect.max.x - 2.0, rect.center().y), Align2::RIGHT_CENTER, &shown_meta, FontId::proportional(11.0), muted_color);
}

fn draw_song_and_lyrics_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let song_path = ctx.state.song_audio_path().unwrap_or_default();
    let has_song = !song_path.is_empty();
    let song_file = song_path.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("No audio linked");
    let lyric_lines = collect_lyric_lines(ctx);
    let timed_words = ctx.state.cache.snapshot.pointer("/data/status/timed_word_count").and_then(Value::as_u64).unwrap_or(0);

    let card = egui::Frame::new()
        .fill(ctx.pal.bg_panel)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
        .stroke(Stroke::new(1.3, ctx.pal.accent_emerald.gamma_multiply(0.76)))
        .show(ui, |ui| {
            let inner_w = (ui.available_width() - 4.0).max(180.0);
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.vertical(|ui| {
                // 1. Song section
                let scol = if has_song { ctx.pal.accent_emerald } else { ctx.pal.accent_amber };
                draw_section_status_header(ui, scol, "Song", ctx.pal.accent_emerald, song_file, ctx.pal.text_muted);
                ui.add_space(2.0);
                draw_song_waveform(ui, ctx, song_file, has_song);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let pick_w = (ui.available_width() - 82.0).max(96.0);
                    ui.allocate_ui_with_layout(Vec2::new(pick_w, 28.0), Layout::top_down(Align::Min), |ui| {
                        if recessed_action_button(ui, ctx, "song-upload", "Change audio…", false, "Select song audio file") {
                            ctx.push(UiAction::PickFile {
                                target: PickerTarget::SetSongAudio,
                                title: "Select Song Audio".into(),
                                extensions: vec!["wav".into(), "flac".into(), "mp3".into(), "m4a".into(), "ogg".into()],
                                directory: false,
                                save: false,
                            });
                        }
                    });
                    if action_button(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        BtnSpec::new("song-transcribe", "sparkles", "Align", "Run FilmCraft speech alignment & write timing-reference.json").enabled(has_song),
                    )
                    .clicked()
                    {
                        ctx.exec("audio.analyze", json!({}), Some("Transcribed song audio"));
                    }
                });

                // 2. Lyrics & notes section
                ui.add_space(10.0);
                let lcol = if lyric_lines.is_empty() { ctx.pal.accent_amber } else { ctx.pal.accent_emerald };
                draw_section_status_header(ui, lcol, "Lyrics & notes", ctx.pal.accent_amber, &format!("{} lines", lyric_lines.len()), ctx.pal.text_muted);
                ui.add_space(4.0);
                ctx.pal.inset_frame().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ScrollArea::vertical().id_salt("proj_lyrics_box").max_height(98.0).show(ui, |ui| {
                        let preview_text = if lyric_lines.is_empty() {
                            "No lyrics loaded yet — align song audio or import timed lyrics.".to_owned()
                        } else {
                            lyric_lines.iter().take(8).cloned().collect::<Vec<_>>().join("\n")
                        };
                        ui.label(RichText::new(preview_text).monospace().size(10.5).color(ctx.pal.text_secondary));
                    });
                });
                ui.add_space(6.0);
                if recessed_action_button(ui, ctx, "song-open-lyrics", "Open Lyrics & Motion →", true, "Jump to Lyrics & Motion authoring workspace") {
                    ctx.push(UiAction::SwitchView(ViewMode::Design));
                }

                // 3. Transcript section — clearly separated below divider with generous spacing
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);
                let tcol = if timed_words > 0 { ctx.pal.accent_emerald } else { ctx.pal.accent_amber };
                draw_section_status_header(ui, tcol, "Transcript", ctx.pal.accent_sky, &format!("{timed_words} timed words"), ctx.pal.text_muted);
                ui.add_space(6.0);
                if recessed_action_button(ui, ctx, "song-import-transcript", "Import timed lyrics…", false, "Import timed transcript JSON, SRT, or VTT") {
                    ctx.push(UiAction::PickFile {
                        target: PickerTarget::ImportTranscript,
                        title: "Import Timed Transcript".into(),
                        extensions: vec!["json".into(), "srt".into(), "vtt".into()],
                        directory: false,
                        save: false,
                    });
                }
            });
        });
    ctx.reg.record("block-song", "Song Block", &card.response, None);
}

fn stage_jump_tile(ui: &mut Ui, ctx: &mut ViewCtx<'_>, width: f32, id: &str, label: &str, centered: bool, desc: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let fill = if resp.hovered() { ctx.pal.bg_hover } else { ctx.pal.bg_panel };
    let border = if resp.hovered() { ctx.pal.accent_emerald } else { ctx.pal.border };
    ui.painter().rect(rect, CornerRadius::same(7), fill, Stroke::new(1.0, border), StrokeKind::Inside);
    if centered {
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(12.5), ctx.pal.text_primary);
    } else {
        ui.painter().text(Pos2::new(rect.min.x + 12.0, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(12.5), ctx.pal.text_primary);
    }
    let resp = resp.on_hover_text(desc);
    ctx.reg.record(id, label, &resp, None);
    resp.clicked()
}

fn draw_pipeline_jumps(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let grid_resp = ui.vertical(|ui| {
        let full_w = (ui.available_width() - 2.0).max(180.0);
        let half_w = ((full_w - 8.0) * 0.5).max(86.0);

        for pair in [
            [
                ("project-jump-storyboard", "Storyboard →", "Edit DesignCraft spreads & check readiness", ViewMode::Storyboard),
                ("project-jump-animation", "Animation →", "Rig PhotoCraft (.pcraft) & EffectCraft (.ecproj) puppets", ViewMode::Animation),
            ],
            [
                ("project-jump-lyrics", "Lyrics →", "Author EffectCraft (lettering/lyrics.ecproj) kinetic lyrics", ViewMode::Design),
                ("project-jump-motion", "Motion →", "Author VectorCraft (.svg) & EffectCraft (actions.ecproj) cues", ViewMode::Motion),
            ],
        ] {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (btn_id, label, desc, target_view) in pair {
                    if stage_jump_tile(ui, ctx, half_w, btn_id, label, false, desc) {
                        ctx.push(UiAction::SwitchView(target_view));
                    }
                }
            });
            ui.add_space(6.0);
        }

        if stage_jump_tile(ui, ctx, full_w, "project-jump-studio", "Studio →", false, "Assemble and export FilmCraft (.fcproj) sequence") {
            ctx.push(UiAction::SwitchView(ViewMode::Studio));
        }
        ui.add_space(8.0);
        if stage_jump_tile(ui, ctx, full_w, "project-close-opened-apps", "Close apps", true, "Gracefully close sibling ArtCraft windows opened by Magic") {
            ctx.exec("app.doctor", json!({}), Some("Checked Magic Studio workspace health"));
        }
    });
    ctx.reg.record("blocks", "Production Stages", &grid_resp.response, None);
}

fn cycle_type_filter(cur: &str) -> &'static str {
    match cur {
        "all" => "craft",
        "craft" => "image",
        _ => "all",
    }
}

fn draw_group_by_domain_button(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let active = ctx.state.project.group_by_folder;
    let label = "Group by Craft Domain";
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(11.5), ctx.pal.text_primary);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 44.0, 26.0), Sense::click());
    let fill = if resp.hovered() { ctx.pal.bg_hover } else { ctx.pal.bg_input };
    ui.painter().rect(rect, CornerRadius::same(6), fill, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);

    let chk = Rect::from_min_size(Pos2::new(rect.min.x + 8.0, rect.center().y - 6.5), Vec2::splat(13.0));
    if active {
        ui.painter().rect_filled(chk, CornerRadius::same(4), ctx.pal.accent_emerald);
        ui.painter().text(chk.center(), Align2::CENTER_CENTER, "✓", FontId::proportional(10.0), Color32::from_rgb(0x06, 0x16, 0x12));
    } else {
        ui.painter().rect(chk, CornerRadius::same(4), ctx.pal.bg_stage, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
    }
    ui.painter().text(Pos2::new(chk.max.x + 6.0, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(11.5), ctx.pal.text_primary);
    ui.painter().text(Pos2::new(rect.max.x - 9.0, rect.center().y), Align2::CENTER_CENTER, "▾", FontId::proportional(10.5), ctx.pal.text_muted);
    if resp.clicked() {
        ctx.state.project.group_by_folder = !ctx.state.project.group_by_folder;
    }
    ctx.reg.record("pf-group-toggle", label, &resp, Some(active.to_string()));
}

fn mint_outline_button(ui: &mut Ui, ctx: &mut ViewCtx<'_>, id: &str, label: &str, tooltip: &str) -> bool {
    let font = FontId::proportional(11.5);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), ctx.pal.accent_emerald);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 26.0), Sense::click());
    let fill = if resp.hovered() { ctx.pal.accent_emerald.gamma_multiply(0.28) } else { ctx.pal.accent_emerald_bg };
    ui.painter().rect(rect, CornerRadius::same(6), fill, Stroke::new(1.2, ctx.pal.accent_emerald), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, ctx.pal.accent_mint);
    let resp = resp.on_hover_text(tooltip);
    ctx.reg.record(id, label, &resp, None);
    resp.clicked()
}

fn draw_files_trunk(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let trunk = &ctx.state.cache.snapshot["data"]["trunk"];
    let trunk_files = trunk["files"].as_array().cloned().unwrap_or_default();
    let total_count = trunk["count"].as_u64().map_or(trunk_files.len(), |n| n as usize);
    let reg_count = trunk["registered_asset_count"].as_u64().unwrap_or(0);

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Project Craft Files & Rendered Assets").size(15.0).strong().color(ctx.pal.text_primary));
            ui.label(RichText::new(format!("{total_count} Files · {reg_count} registered assets")).size(11.5).color(ctx.pal.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                draw_trunk_bridge_buttons(ui, ctx);
            });
        });
        ui.add_space(4.0);

        ui.horizontal_wrapped(|ui| {
            let type_lbl = match ctx.state.project.type_filter.as_str() {
                "craft" => "Type: Craft Projects  ▾",
                "image" => "Type: Rendered Assets  ▾",
                _ => "Type: All Craft Files  ▾",
            };
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("pf-filter-cycle", type_lbl, "Cycle Craft file type filter")).clicked() {
                ctx.state.project.type_filter = cycle_type_filter(&ctx.state.project.type_filter).into();
            }

            draw_group_by_domain_button(ui, ctx);

            let search_w = (ui.available_width() - 256.0).clamp(110.0, 380.0);
            text_input(ui, ctx.reg, "pf-search", "Filter files", &mut ctx.state.project.search, "Filter by name…", search_w);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if mint_outline_button(
                    ui,
                    ctx,
                    "pf-import",
                    "+ Import Craft File",
                    "Import .fcproj, .designcraft, .ecproj, .pcraft, .vectorcraft, or .svg into project",
                ) {
                    ctx.push(UiAction::PickFile {
                        target: PickerTarget::ImportTrunkFile,
                        title: "Import Craft Project or Media File".into(),
                        extensions: vec![
                            "pcraft".into(),
                            "psd".into(),
                            "vectorcraft".into(),
                            "svg".into(),
                            "ecproj".into(),
                            "designcraft".into(),
                            "fcproj".into(),
                            "anicraft".into(),
                            "png".into(),
                            "mov".into(),
                            "mp4".into(),
                        ],
                        directory: false,
                        save: false,
                    });
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("pf-add-asset", "plus", "Add Asset", "Add Craft or image asset")).clicked() {
                    ctx.push(UiAction::PickFile {
                        target: PickerTarget::AddProjectAsset,
                        title: "Add Craft Document or Artwork Asset".into(),
                        extensions: vec![
                            "pcraft".into(),
                            "psd".into(),
                            "vectorcraft".into(),
                            "svg".into(),
                            "ecproj".into(),
                            "anicraft".into(),
                            "png".into(),
                            "jpg".into(),
                            "jpeg".into(),
                            "webp".into(),
                        ],
                        directory: false,
                        save: false,
                    });
                }
            });
        });

        draw_trunk_bridge_drawers(ui, ctx);
        ui.add_space(3.0);
        ui.separator();
        draw_trunk_file_list(ui, ctx, &trunk_files);
    });
    ctx.reg.record("project-files-panel", "Project Craft Files", &panel.response, None);
}

fn is_craft_doc_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".fcproj", ".designcraft", ".ecproj", ".pcraft", ".psd", ".vectorcraft", ".svg", ".anicraft"].iter().any(|ext| lower.ends_with(ext))
}

fn normalize_domain_heading(raw: &str) -> &str {
    match raw {
        "FilmCraft" => "FilmCraft (.fcproj)",
        "DesignCraft & PDF" => "DesignCraft (.designcraft)",
        "EffectCraft & Linked Comps" => "EffectCraft (.ecproj)",
        "Photo & Vector" => "PhotoCraft & VectorCraft (.pcraft / .svg)",
        other => other,
    }
}

fn draw_trunk_file_list(ui: &mut Ui, ctx: &mut ViewCtx<'_>, trunk_files: &[Value]) {
    let query = ctx.state.project.search.trim().to_ascii_lowercase();
    let tfilter = ctx.state.project.type_filter.clone();
    let grouped = ctx.state.project.group_by_folder;

    let filtered: Vec<&Value> = trunk_files
        .iter()
        .filter(|f| {
            let name = f["name"].as_str().unwrap_or_default().to_ascii_lowercase();
            let rel = f["rel_dir"].as_str().unwrap_or_default().to_ascii_lowercase();
            let path = f["path"].as_str().unwrap_or_default();
            if !query.is_empty() && !name.contains(&query) && !rel.contains(&query) {
                return false;
            }
            match tfilter.as_str() {
                "craft" => f["is_craft_project"].as_bool() == Some(true) || is_craft_doc_path(path),
                "image" => f["is_image"].as_bool() == Some(true),
                _ => true,
            }
        })
        .collect();

    let list_out = ScrollArea::vertical().id_salt("pf_list_scroll").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.5;
        ui.vertical(|ui| {
            if filtered.is_empty() {
                ui.label(
                    RichText::new("No matching Craft or asset files in project trunk — click + Add Asset or + Import Craft File.").color(ctx.pal.text_muted),
                );
                return;
            }
            let mut last_group = String::new();
            for (idx, file) in filtered.iter().enumerate() {
                let raw_grp = file["group_label"].as_str().unwrap_or("Project Files");
                let grp_lbl = normalize_domain_heading(raw_grp);
                if grouped && grp_lbl != last_group {
                    let grp_count =
                        filtered.iter().filter(|item| normalize_domain_heading(item["group_label"].as_str().unwrap_or("Project Files")) == grp_lbl).count();
                    if idx > 0 {
                        ui.add_space(5.0);
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("▾ {grp_lbl}")).size(11.5).strong().color(ctx.pal.text_secondary));
                        ui.label(RichText::new(format!("({grp_count})")).monospace().size(9.5).color(ctx.pal.text_muted));
                        let rem = ui.available_width().max(16.0);
                        let (lrect, _) = ui.allocate_exact_size(Vec2::new(rem, 10.0), Sense::hover());
                        ui.painter().line_segment(
                            [Pos2::new(lrect.min.x + 4.0, lrect.center().y), Pos2::new(lrect.max.x, lrect.center().y)],
                            Stroke::new(1.0, ctx.pal.border_subtle),
                        );
                    });
                    ui.add_space(1.5);
                    last_group = grp_lbl.to_owned();
                }
                draw_single_trunk_row(ui, ctx, idx, file);
            }
        });
    });
    let list_resp = ui.interact(list_out.inner_rect, ui.scope_id().with("pf_list"), Sense::hover());
    ctx.reg.record("pf-list", "Asset List", &list_resp, None);
}

fn draw_single_trunk_row(ui: &mut Ui, ctx: &mut ViewCtx<'_>, idx: usize, file: &Value) {
    let name = file["name"].as_str().unwrap_or("file");
    let path = file["path"].as_str().unwrap_or_default();
    let rel_dir = file["rel_dir"].as_str().unwrap_or("./");
    let raw_badge = file["badge"].as_str().unwrap_or("FILE");
    let badge = match raw_badge {
        "PC" => "PCRAFT",
        "VC" => "VCRAFT",
        other => other,
    };
    let detail = file["detail"].as_str().unwrap_or("");
    let open_lbl = file["open_label"].as_str().unwrap_or("Open");
    let is_image = file["is_image"].as_bool() == Some(true) || path.to_ascii_lowercase().ends_with(".svg");
    let can_animate =
        file["can_animate"].as_bool() == Some(true) || path.to_ascii_lowercase().ends_with(".pcraft") || path.to_ascii_lowercase().ends_with(".vectorcraft");
    let can_link_fc = file["can_link_filmcraft"].as_bool() == Some(true) || path.to_ascii_lowercase().ends_with(".ecproj");
    let has_registered_asset = file["assets"].as_array().is_some_and(|a| !a.is_empty());

    let primary_title = if matches!(badge, "FC" | "DC" | "EC") && rel_dir != "./" { format!("{rel_dir}{name}") } else { name.to_owned() };
    let subtitle = if has_registered_asset {
        let asset_rel = file.pointer("/assets/0/path").and_then(Value::as_str).unwrap_or(name);
        format!("▪ {asset_rel}")
    } else if detail.is_empty() {
        rel_dir.to_owned()
    } else if rel_dir == "./" {
        format!("./ · {detail}")
    } else {
        format!("{rel_dir}{name} · {detail}")
    };

    let row_w = (ui.available_width() - 6.0).max(260.0);
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(row_w, 40.0), Sense::hover());
    ui.painter().rect(row_rect, CornerRadius::same(7), ctx.pal.bg_input, Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);

    let act_reserve = if can_animate || can_link_fc { 242.0 } else { 156.0 };
    let badge_w = if badge == "VCRAFT" {
        108.0
    } else if can_link_fc {
        116.0
    } else {
        68.0
    };
    let act_min_x = (row_rect.max.x - act_reserve).max(row_rect.min.x + 200.0);
    let badge_min_x = (act_min_x - badge_w - 6.0).max(row_rect.min.x + 150.0);

    let thumb_rect = Rect::from_min_size(Pos2::new(row_rect.min.x + 7.0, row_rect.center().y - 14.0), Vec2::splat(28.0));
    let show_art = matches!(badge, "PCRAFT" | "VCRAFT" | "PNG" | "JPG" | "WEBP" | "SVG") || has_registered_asset;
    let preview_src = file["preview_path"].as_str().or_else(|| is_image.then_some(path));
    if !(show_art && paint_local_image(ui, thumb_rect, preview_src, ctx.pal.bg_stage, 5)) {
        let (tile_bg, tile_fg) = if badge == "FC" {
            (ctx.pal.accent_emerald_bg, ctx.pal.accent_emerald)
        } else {
            (Color32::from_rgba_unmultiplied(0x52, 0x3f, 0x1c, 130), ctx.pal.accent_amber)
        };
        ui.painter().rect(thumb_rect, CornerRadius::same(5), tile_bg, Stroke::new(1.0, tile_fg.gamma_multiply(0.45)), StrokeKind::Inside);
        let short_glyph = match badge {
            "PCRAFT" => "PC",
            "VCRAFT" => "VC",
            other => other,
        };
        ui.painter().text(thumb_rect.center(), Align2::CENTER_CENTER, short_glyph, FontId::monospace(8.5), tile_fg);
    }

    let text_x = thumb_rect.max.x + 9.0;
    let text_clip = Rect::from_min_max(Pos2::new(text_x, row_rect.min.y), Pos2::new(badge_min_x - 6.0, row_rect.max.y));
    let text_painter = ui.painter().with_clip_rect(text_clip);
    text_painter.text(Pos2::new(text_x, row_rect.min.y + 13.5), Align2::LEFT_CENTER, &primary_title, FontId::proportional(12.0), ctx.pal.text_primary);
    let sub_col = if has_registered_asset { ctx.pal.accent_sky } else { ctx.pal.text_muted };
    text_painter.text(Pos2::new(text_x, row_rect.min.y + 28.0), Align2::LEFT_CENTER, &subtitle, FontId::proportional(10.0), sub_col);

    let badge_rect = Rect::from_min_max(Pos2::new(badge_min_x, row_rect.min.y), Pos2::new(act_min_x - 2.0, row_rect.max.y));
    let mut badge_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(badge_rect).layout(Layout::left_to_right(Align::Center)).scope_id(ui.scope_id().with(("pf_badge", idx))));
    badge_ui.spacing_mut().item_spacing.x = 4.0;
    paint_craft_badge(&mut badge_ui, ctx.pal, badge, false);
    if badge == "VCRAFT" {
        paint_craft_badge(&mut badge_ui, ctx.pal, "SVG", false);
    } else if can_link_fc {
        status_pill(&mut badge_ui, ctx.pal, "Link", ctx.pal.accent_emerald);
    }

    let act_rect = Rect::from_min_max(Pos2::new(act_min_x, row_rect.min.y), Pos2::new(row_rect.max.x - 6.0, row_rect.max.y));
    let mut act_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(act_rect).layout(Layout::right_to_left(Align::Center)).scope_id(ui.scope_id().with(("pf_act", idx))));
    act_ui.spacing_mut().item_spacing.x = 5.0;

    if can_animate {
        let anim_id = format!("pf-animate-{idx}");
        if action_button(&mut act_ui, ctx.reg, ctx.pal, BtnSpec::label_only(&anim_id, "Animate →", "Rig asset in Animations")).clicked() {
            let stem = name.split('.').next().unwrap_or(name);
            let asset_id = file.pointer("/assets/0/character").or_else(|| file.pointer("/assets/0/id")).and_then(Value::as_str).unwrap_or(stem).to_owned();
            ctx.push(UiAction::SelectCharacter { id: asset_id, jump_animation: true });
        }
    }

    let open_id = format!("pf-open-{idx}");
    let open_txt = if open_lbl.contains("Craft") { format!("{open_lbl} ↗") } else { open_lbl.to_owned() };
    if action_button(&mut act_ui, ctx.reg, ctx.pal, BtnSpec::label_only(&open_id, &open_txt, "Open in sibling Craft app")).clicked() {
        let app = craft_app_for_path(path);
        ctx.exec("workspace.open", json!({"app": app, "path": path}), Some("Opened in Craft editor"));
    }

    if can_link_fc {
        let link_id = format!("pf-link-fc-{idx}");
        let track_guess = if rel_dir.starts_with("lettering/") {
            "V3"
        } else if rel_dir.starts_with("motion/") {
            "V4"
        } else {
            "V2"
        };
        let sel = ctx.state.project.link_popup_path.as_deref() == Some(path);
        if action_button(
            &mut act_ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::icon_only(&link_id, "link", "Configure EC Dynamic-Link Comp, Alpha Codec & Rational FPS (T01/T02/T10)").selected(sel),
        )
        .clicked()
        {
            ctx.state.project.link_popup_path = Some(path.to_owned());
            ctx.state.project.link_track = track_guess.into();
            ctx.exec("video.list_ec_comps", json!({"project": path}), None);
        }
    }
}

fn craft_app_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".vectorcraft") || lower.ends_with(".svg") {
        "vectorcraft"
    } else if lower.ends_with(".designcraft") || lower.ends_with(".pdf") {
        "designcraft"
    } else if lower.ends_with(".ecproj") || lower.ends_with(".anicraft") {
        "effectcraft"
    } else if lower.ends_with(".fcproj") || lower.ends_with(".mp4") || lower.ends_with(".mov") {
        "filmcraft"
    } else {
        "photocraft"
    }
}
