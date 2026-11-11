//! `Project` overview & Craft file trunk (`#view-project`): left `Song & Stages` readiness column
//! (`● Song`, `● Lyrics & notes`, `● Transcript`, stage jump buttons) and right `Project Craft Files
//! & Rendered Assets` grouped by `FilmCraft (.fcproj)`, `DesignCraft (.designcraft)`,
//! `EffectCraft (.ecproj)`, and `PhotoCraft & VectorCraft (.pcraft / .svg)` with real `file://` thumbnails.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{
    Align, Align2, CornerRadius, FontId, Layout, Pos2, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{badge_accent_color, child_pane, fill_card, paint_local_image, pill_toggle};
use crate::state::{PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, text_input};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let body = ui.available_rect_before_wrap();
    if let Some(split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(304.0), 10.0, 240.0) {
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
            let line: String = anim["words"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|w| w["text"].as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if !line.trim().is_empty() {
                out.push(line);
            }
        }
    }
    out
}

fn draw_stage_stack(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ScrollArea::vertical().id_salt("project_stage_scroll").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("◫").size(12.5).color(ctx.pal.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("Song & Stages").size(11.5).color(ctx.pal.text_muted));
            });
        });
        ui.add_space(4.0);
        draw_song_and_lyrics_cards(ui, ctx);
        ui.add_space(8.0);
        draw_pipeline_jumps(ui, ctx);
    });
}

fn draw_song_and_lyrics_cards(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let song_path = ctx.state.song_audio_path().unwrap_or_default();
    let song_file = song_path.rsplit('/').next().unwrap_or("No audio linked");
    let lyric_lines = collect_lyric_lines(ctx);
    let timed_words = ctx
        .state
        .cache
        .snapshot
        .pointer("/data/status/timed_word_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let card = fill_card(ui, ctx.pal, false, |ui| {
        ui.horizontal(|ui| {
            let col = if song_path.is_empty() { ctx.pal.accent_amber } else { ctx.pal.accent_emerald };
            let (drect, _) = ui.allocate_exact_size(Vec2::new(10.0, 16.0), Sense::hover());
            ui.painter().circle_filled(drect.center(), 3.6, col);
            ui.label(RichText::new("Song").size(12.0).strong().color(col));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(song_file).monospace().size(10.5).color(ctx.pal.text_muted));
            });
        });
        ui.label(RichText::new(ctx.state.project_name()).size(12.5).strong().color(ctx.pal.text_primary));
        ui.horizontal_wrapped(|ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("song-upload", "Change audio…", "Select song audio file")).clicked() {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::SetSongAudio,
                    title: "Select Song Audio".into(),
                    extensions: vec!["wav".into(), "flac".into(), "mp3".into(), "m4a".into(), "ogg".into()],
                    directory: false,
                    save: false,
                });
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("song-transcribe", "sparkles", "Transcribe", "Run FilmCraft speech alignment & write timing-reference.json")
                    .primary()
                    .enabled(!song_path.is_empty()),
            )
            .clicked()
            {
                ctx.exec("audio.analyze", json!({}), Some("Transcribed song audio"));
            }
        });

        ui.add_space(6.0);
        ui.separator();

        ui.horizontal(|ui| {
            let lcol = if lyric_lines.is_empty() { ctx.pal.accent_amber } else { ctx.pal.accent_emerald };
            let (drect, _) = ui.allocate_exact_size(Vec2::new(10.0, 16.0), Sense::hover());
            ui.painter().circle_filled(drect.center(), 3.6, lcol);
            ui.label(RichText::new("Lyrics & notes").size(12.0).strong().color(ctx.pal.accent_amber));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{} lines", lyric_lines.len())).size(10.5).color(ctx.pal.text_muted));
            });
        });
        ctx.pal.inset_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let preview_text = if lyric_lines.is_empty() {
                "No lyrics loaded yet — transcribe song audio or import timed lyrics.".to_owned()
            } else {
                lyric_lines.iter().take(7).cloned().collect::<Vec<_>>().join("\n")
            };
            ui.label(RichText::new(preview_text).monospace().size(10.5).color(ctx.pal.text_secondary));
        });
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("song-open-lyrics", "Open Lyrics & Motion →", "Open Kinetic Lyrics workspace"),
        )
        .clicked()
        {
            ctx.push(UiAction::SwitchView(ViewMode::Design));
        }

        ui.add_space(6.0);
        ui.separator();

        ui.horizontal(|ui| {
            let tcol = if timed_words > 0 { ctx.pal.accent_emerald } else { ctx.pal.accent_amber };
            let (drect, _) = ui.allocate_exact_size(Vec2::new(10.0, 16.0), Sense::hover());
            ui.painter().circle_filled(drect.center(), 3.6, tcol);
            ui.label(RichText::new("Transcript").size(12.0).strong().color(tcol));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{timed_words} timed words")).size(10.5).color(ctx.pal.text_muted));
            });
        });
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("song-import-transcript", "Import timed lyrics…", "Import timed transcript JSON or SRT"),
        )
        .clicked()
        {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportTranscript,
                title: "Import Timed Transcript".into(),
                extensions: vec!["json".into(), "srt".into(), "vtt".into()],
                directory: false,
                save: false,
            });
        }
    });
    ctx.reg.record("block-song", "Song Block", &card.response, None);
}

fn draw_pipeline_jumps(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let card = fill_card(ui, ctx.pal, false, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (btn_id, label, desc, target_view) in [
                ("project-jump-storyboard", "Storyboard →", "Edit DesignCraft spreads & check readiness", ViewMode::Storyboard),
                ("project-jump-animation", "Animation →", "Rig PhotoCraft (.pcraft) & EffectCraft (.ecproj) puppets", ViewMode::Animation),
                ("project-jump-lyrics", "Lyrics →", "Author EffectCraft (lettering/lyrics.ecproj) kinetic lyrics", ViewMode::Design),
                ("project-jump-motion", "Motion →", "Author VectorCraft (.svg) & EffectCraft (actions.ecproj) cues", ViewMode::Motion),
                ("project-jump-studio", "Studio →", "Assemble and export FilmCraft (.fcproj) sequence", ViewMode::Studio),
            ] {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(btn_id, label, desc)).clicked() {
                    ctx.push(UiAction::SwitchView(target_view));
                }
            }
        });
    });
    ctx.reg.record("blocks", "Production Stages", &card.response, None);
}

fn draw_files_trunk(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let trunk = &ctx.state.cache.snapshot["data"]["trunk"];
    let trunk_files = trunk["files"].as_array().cloned().unwrap_or_default();
    let total_count = trunk["count"].as_u64().map_or(trunk_files.len(), |n| n as usize);
    let reg_count = trunk["registered_asset_count"].as_u64().unwrap_or(0);

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Project Craft Files & Rendered Assets").size(14.5).strong().color(ctx.pal.text_primary));
            ui.label(
                RichText::new(format!("{total_count} files · {reg_count} registered assets"))
                    .size(11.5)
                    .color(ctx.pal.text_muted),
            );
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (fid, lbl) in [
                ("all", format!("Type: All Files ({total_count})")),
                ("craft", "Craft Projects (.fcproj/.designcraft/.ecproj/.pcraft/.svg)".into()),
                ("image", "Rendered Frames & PNGs".into()),
            ] {
                let active = ctx.state.project.type_filter == fid;
                let btn_id = format!("pf-filter-{fid}");
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, &lbl, "Filter project files").selected(active)).clicked() {
                    ctx.state.project.type_filter = fid.into();
                }
            }
            pill_toggle(
                ui,
                ctx.reg,
                ctx.pal,
                "pf-group-toggle",
                "Group by Craft Domain",
                &mut ctx.state.project.group_by_folder,
                ctx.pal.accent_emerald,
            );
            let search_w = (ui.available_width() - 280.0).clamp(140.0, 320.0);
            text_input(ui, ctx.reg, "pf-search", "Filter files", &mut ctx.state.project.search, "Filter by name…", search_w);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("pf-import", "plus", "Import Craft File", "Import Craft document into project")).clicked() {
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
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("pf-add-asset", "plus", "Add Asset", "Add Craft or image asset").primary()).clicked() {
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
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("pf-paste", "Paste artwork", "Import image from clipboard")).clicked()
                    && let Some(proj) = ctx.state.project_path.as_ref().and_then(|p| p.to_str()).map(str::to_owned)
                {
                    ctx.exec("clipboard.import_image", json!({"expected_project": proj}), Some("Imported clipboard artwork"));
                }
            });
        });
        ui.separator();
        draw_trunk_file_list(ui, ctx, &trunk_files);
    });
    ctx.reg.record("project-files-panel", "Project Files", &panel.response, None);
}

fn is_craft_doc_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".fcproj", ".designcraft", ".ecproj", ".pcraft", ".psd", ".vectorcraft", ".svg", ".anicraft"]
        .iter()
        .any(|ext| lower.ends_with(ext))
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
        if filtered.is_empty() {
            ui.label(
                RichText::new("No matching Craft or asset files in project trunk — click + Add Asset or + Import Craft File.")
                    .color(ctx.pal.text_muted),
            );
            return;
        }
        let mut last_group = String::new();
        for (idx, file) in filtered.iter().enumerate() {
            let grp_lbl = file["group_label"].as_str().unwrap_or("Project Files");
            if grouped && grp_lbl != last_group {
                let grp_cnt = filtered
                    .iter()
                    .filter(|x| x["group_label"].as_str().unwrap_or("Project Files") == grp_lbl)
                    .count();
                if idx > 0 {
                    ui.add_space(6.0);
                }
                ui.label(
                    RichText::new(format!("▾ {grp_lbl} ({grp_cnt})"))
                        .size(11.0)
                        .strong()
                        .color(ctx.pal.text_muted),
                );
                ui.add_space(2.0);
                last_group = grp_lbl.to_owned();
            }
            draw_single_trunk_row(ui, ctx, idx, file);
        }
    });
    let list_resp = ui.interact(list_out.inner_rect, ui.scope_id().with("pf_list"), Sense::hover());
    ctx.reg.record("pf-list", "Asset List", &list_resp, None);
}

fn draw_single_trunk_row(ui: &mut Ui, ctx: &mut ViewCtx<'_>, idx: usize, file: &Value) {
    let name = file["name"].as_str().unwrap_or("file");
    let path = file["path"].as_str().unwrap_or_default();
    let rel_dir = file["rel_dir"].as_str().unwrap_or("./");
    let badge = file["badge"].as_str().unwrap_or("FILE");
    let detail = file["detail"].as_str().unwrap_or("");
    let open_lbl = file["open_label"].as_str().unwrap_or("Open");
    let is_image = file["is_image"].as_bool() == Some(true) || path.to_ascii_lowercase().ends_with(".svg");
    let can_animate = file["can_animate"].as_bool() == Some(true)
        || path.to_ascii_lowercase().ends_with(".pcraft")
        || path.to_ascii_lowercase().ends_with(".ecproj");
    let bcol = badge_accent_color(ctx.pal, badge);

    egui::Frame::new()
        .fill(ctx.pal.bg_input)
        .corner_radius(CornerRadius::same(7))
        .inner_margin(egui::Margin::symmetric(8, 5))
        .stroke(Stroke::new(1.0, ctx.pal.border_subtle))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let preview_src = file["preview_path"].as_str().or_else(|| is_image.then_some(path));
                let (thumb_rect, _) = ui.allocate_exact_size(Vec2::new(36.0, 28.0), Sense::hover());
                if !paint_local_image(ui, thumb_rect, preview_src, ctx.pal.bg_stage, 5) {
                    ui.painter().rect(
                        thumb_rect,
                        CornerRadius::same(5),
                        bcol.gamma_multiply(0.16),
                        Stroke::new(1.0, bcol.gamma_multiply(0.55)),
                        StrokeKind::Inside,
                    );
                    ui.painter()
                        .text(thumb_rect.center(), Align2::CENTER_CENTER, badge, FontId::monospace(9.5), bcol);
                }

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(RichText::new(name).size(12.0).strong().color(ctx.pal.text_primary));
                    ui.label(RichText::new(format!("{rel_dir}{name}")).size(10.0).color(ctx.pal.text_muted));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let open_id = format!("pf-open-{idx}");
                    let open_txt = if open_lbl.contains("Craft") {
                        format!("{open_lbl} ↗")
                    } else {
                        open_lbl.to_owned()
                    };
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&open_id, &open_txt, "Open in sibling Craft app")).clicked() {
                        let app = craft_app_for_path(path);
                        ctx.exec("workspace.open", json!({"app": app, "path": path}), Some("Opened in Craft editor"));
                    }
                    if can_animate {
                        let anim_id = format!("pf-animate-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&anim_id, "Animate →", "Rig asset in Animations")).clicked() {
                            let stem = name.split('.').next().unwrap_or(name);
                            let asset_id = file
                                .pointer("/assets/0/id")
                                .and_then(Value::as_str)
                                .unwrap_or(stem)
                                .to_owned();
                            ctx.push(UiAction::SelectCharacter {
                                id: asset_id,
                                jump_animation: true,
                            });
                        }
                    }
                    if !detail.is_empty() {
                        ui.label(RichText::new(detail).monospace().size(10.0).color(ctx.pal.text_muted));
                    }
                    let chip_w = (badge.len() as f32 * 6.8 + 12.0).max(34.0);
                    let (bchip, _) = ui.allocate_exact_size(Vec2::new(chip_w, 18.0), Sense::hover());
                    ui.painter()
                        .rect_filled(bchip, CornerRadius::same(4), bcol.gamma_multiply(0.18));
                    ui.painter().text(
                        Pos2::new(bchip.center().x, bchip.center().y),
                        Align2::CENTER_CENTER,
                        badge,
                        FontId::monospace(9.5),
                        bcol,
                    );
                });
            });
        });
    ui.add_space(3.0);
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
