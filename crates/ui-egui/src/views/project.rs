//! `Project` overview & file trunk (`#view-project`): left `Song & Stages` readiness column
//! (`Song`, `Lyrics & notes`, `Transcript`, stage jumps) and right `Project Files & Assets` grouped
//! by `FilmCraft`, `DesignCraft & PDF`, and `Photo & Vector`.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{RichText, ScrollArea, Ui};
use serde_json::{Value, json};

use crate::controls::pill_toggle;
use crate::state::{PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outer = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(outer, egui::Sense::hover());
    ctx.reg.record("view-project", "Project", &resp, None);

    if let Some(split) = split_rect(outer, SplitAxis::Horizontal, SplitSize::FixedFirst(300.0), 10.0, 220.0) {
        let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(split.first).scope_id(ui.scope_id().with("project_left")));
        draw_stage_stack(&mut left_ui, ctx);
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(split.second).scope_id(ui.scope_id().with("project_right")));
        draw_files_trunk(&mut right_ui, ctx);
    } else {
        draw_stage_stack(ui, ctx);
    }
}

fn draw_stage_stack(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ScrollArea::vertical().id_salt("project_stage_scroll").show(ui, |ui| {
        ui.label(RichText::new("Song & Stages").size(12.0).color(ctx.pal.text_muted));
        draw_song_block(ui, ctx);
        ui.add_space(8.0);
        draw_pipeline_jumps(ui, ctx);
    });
}

fn draw_song_block(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let song_path = ctx.state.song_audio_path().unwrap_or_default();
    let card = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            let col = if song_path.is_empty() { ctx.pal.accent_amber } else { ctx.pal.accent_emerald };
            status_pill(ui, ctx.pal, if song_path.is_empty() { "Song · Missing" } else { "Song · Linked" }, col);
        });
        ui.label(
            RichText::new(if song_path.is_empty() { "Link a song recording (.wav/.mp3/.flac) to sync stages." } else { &song_path })
                .size(11.0)
                .color(ctx.pal.text_secondary),
        );
        ui.horizontal_wrapped(|ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("song-upload", "music", "Audio", "Change song audio")).clicked() {
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
                BtnSpec::new("song-transcribe", "sparkles", "Transcribe", "Run FilmCraft alignment & write timing-reference.json")
                    .primary()
                    .enabled(!song_path.is_empty()),
            )
            .clicked()
            {
                ctx.exec("audio.analyze", json!({}), Some("Transcribed song audio"));
            }
        });

        ui.separator();
        ui.label(RichText::new("Lyrics & Transcript").size(12.0).strong().color(ctx.pal.accent_amber));
        ui.horizontal_wrapped(|ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("song-open-lyrics", "type", "Lyrics", "Open Kinetic Lyrics & Motion")).clicked() {
                ctx.push(UiAction::SwitchView(ViewMode::Design));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("song-import-transcript", "upload", "Import", "Import timed transcript JSON or SRT")).clicked()
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
    });
    ctx.reg.record("block-song", "Song Block", &card.response, None);
}

fn draw_pipeline_jumps(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let card = ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Workstation Stages").size(12.5).strong());
        ui.horizontal_wrapped(|ui| {
            for (btn_id, icon, word, desc, target_view) in [
                ("project-jump-storyboard", "book", "Storyboard", "Edit DesignCraft spreads & check readiness", ViewMode::Storyboard),
                ("project-jump-animation", "user", "Animations", "Rig PhotoCraft parts & EffectCraft V2 puppets", ViewMode::Animation),
                ("project-jump-lyrics", "type", "Lyrics", "Author EffectCraft V3 kinetic lyrics", ViewMode::Design),
                ("project-jump-motion", "pen", "Motion", "Author VectorCraft & EffectCraft V4 action cues", ViewMode::Motion),
                ("project-jump-studio", "film", "Studio", "Assemble and export FilmCraft sequence", ViewMode::Studio),
            ] {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(btn_id, icon, word, desc)).clicked() {
                    ctx.push(UiAction::SwitchView(target_view));
                }
            }
        });
    });
    ctx.reg.record("blocks", "Production Stages", &card.response, None);
}

fn draw_files_trunk(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Project Files & Assets").size(14.0).strong().color(ctx.pal.text_primary));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("pf-import", "upload", "Import", "Import file into project trunk")).clicked() {
                    ctx.push(UiAction::PickFile {
                        target: PickerTarget::ImportTrunkFile,
                        title: "Import Project File".into(),
                        extensions: Vec::new(),
                        directory: false,
                        save: false,
                    });
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("pf-add-asset", "plus", "Asset", "Add artwork asset").primary()).clicked() {
                    ctx.push(UiAction::PickFile {
                        target: PickerTarget::AddProjectAsset,
                        title: "Add Artwork Asset".into(),
                        extensions: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into(), "pcraft".into(), "vectorcraft".into()],
                        directory: false,
                        save: false,
                    });
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("pf-paste", "paste", "Paste", "Import image from clipboard")).clicked()
                    && let Some(proj) = ctx.state.project_path.as_ref().and_then(|p| p.to_str()).map(str::to_owned)
                {
                    ctx.exec("clipboard.import_image", json!({"expected_project": proj}), Some("Imported clipboard artwork"));
                }
            });
        });
        ui.horizontal(|ui| {
            text_input(ui, ctx.reg, "pf-search", "Filter files", &mut ctx.state.project.search, "Filter by name…", 240.0);
            pill_toggle(ui, ctx.reg, ctx.pal, "pf-group-toggle", "Group by Craft app", &mut ctx.state.project.group_by_folder, ctx.pal.accent_emerald);
        });
        ui.separator();
        draw_asset_rows(ui, ctx);
    });
    ctx.reg.record("project-files-panel", "Project Files", &panel.response, None);
}

fn draw_asset_rows(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let assets = ctx.state.cache.snapshot.pointer("/data/assets/assets").and_then(Value::as_array).cloned().unwrap_or_default();
    let query = ctx.state.project.search.trim().to_ascii_lowercase();
    let list_out = ScrollArea::vertical().id_salt("pf_list_scroll").show(ui, |ui| {
        // Canonical linked Craft documents header rows (`FilmCraft` + `DesignCraft`)
        if ctx.state.project.group_by_folder && query.is_empty() {
            ui.label(RichText::new("▾ FilmCraft & DesignCraft").size(11.0).strong().color(ctx.pal.text_muted));
            for (row_id, badge, color, file_lbl, app) in [
                ("pf-doc-fcproj", "FC", ctx.pal.accent_emerald, "film/<song>.fcproj", "filmcraft"),
                ("pf-doc-storyboard", "DC", ctx.pal.accent_amber, "storyboard.designcraft", "designcraft"),
            ] {
                ui.horizontal(|ui| {
                    status_pill(ui, ctx.pal, badge, color);
                    ui.label(RichText::new(file_lbl).strong().color(ctx.pal.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(row_id, "external", "Open", "Open in native Craft app")).clicked() {
                            if app == "designcraft" {
                                ctx.exec("storyboard.open", json!({}), Some("Opened DesignCraft"));
                            } else {
                                ctx.exec("video.open", json!({}), Some("Opened FilmCraft"));
                            }
                        }
                    });
                });
            }
            ui.separator();
            ui.label(RichText::new(format!("▾ Photo, Vector & Effect Assets ({})", assets.len())).size(11.0).strong().color(ctx.pal.text_muted));
        }

        if assets.is_empty() {
            ui.label(RichText::new("No registered assets yet — click + Asset or drop PNG / .pcraft / .vectorcraft files.").color(ctx.pal.text_muted));
            return;
        }
        for (idx, asset) in assets.iter().enumerate() {
            let id = asset["id"].as_str().unwrap_or("asset").to_owned();
            let name = asset["name"].as_str().unwrap_or(&id).to_owned();
            let path = asset["path"].as_str().unwrap_or_default().to_owned();
            let kind = asset["kind"].as_str().unwrap_or("PNG");
            if !query.is_empty() && !name.to_ascii_lowercase().contains(&query) && !path.to_ascii_lowercase().contains(&query) {
                continue;
            }
            ui.horizontal(|ui| {
                status_pill(ui, ctx.pal, kind, ctx.pal.accent_sky);
                ui.label(RichText::new(&name).strong().color(ctx.pal.text_primary));
                ui.label(RichText::new(&path).size(10.5).color(ctx.pal.text_muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let open_id = format!("pf-open-{idx}");
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&open_id, "external", "Open", "Open in sibling Craft app")).clicked() {
                        let app = craft_app_for_path(&path);
                        ctx.exec("workspace.open", json!({"app": app, "path": path}), Some("Opened in Craft editor"));
                    }
                    let anim_id = format!("pf-animate-{idx}-{id}");
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&anim_id, "wand", "Animate", "Rig asset in Animations")).clicked() {
                        ctx.push(UiAction::SelectCharacter { id: id.clone(), jump_animation: true });
                    }
                });
            });
            ui.separator();
        }
    });
    let list_resp = ui.interact(list_out.inner_rect, ui.scope_id().with("pf_list"), egui::Sense::hover());
    ctx.reg.record("pf-list", "Asset List", &list_resp, None);
}

fn craft_app_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".vectorcraft") || lower.ends_with(".svg") {
        "vectorcraft"
    } else if lower.ends_with(".designcraft") {
        "designcraft"
    } else if lower.ends_with(".ecproj") {
        "effectcraft"
    } else if lower.ends_with(".fcproj") || lower.ends_with(".mp4") || lower.ends_with(".mov") {
        "filmcraft"
    } else {
        "photocraft"
    }
}
