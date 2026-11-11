//! `02 Project` supplementary `FilmCraft` × `EffectCraft` bridge controls (`Sheet 08` Zone A):
//! - `T01` / `T02` / `T10`: Per-row `EC` Dynamic-Link composition selector (`video.list_ec_comps`
//!   with `Used in N` count), multi-codec alpha intermediate selector (`prores` 4444 / `webm`
//!   VP9+Alpha+Opus / `exr` 32f), rational `[fps_num, fps_den]` sync (`23.976`, `24`, `29.97 DF`,
//!   `59.94`, `60`), `↻ Sync Comps` (`video.sync_links`), `Bake (.mov)` (`video.bake_link`), and `Unbake`.
//! - `T03`: Bidirectional `OTIO` / `FCPXML` / `Premiere XML` / `CMX 3600 EDL` / `AAF` / `OMF` / `ALE`
//!   interchange drawer (`film_to_effect` & `effect_to_film`) with linked `.ecproj` prerender toggle.
//! - `T04` & `T05`: `+ Lottie (.json/.lottie)` & `+ .AEP` dry-run loss-report importer.
//! - `T08`: Left-rail `Offline Media & Render-Source Guard` (`bridge.preflight`).

use egui::{Align, Layout, RichText, Ui};
use serde_json::{Value, json};

use crate::controls::{fill_card, pill_toggle};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill};

fn fps_pair_for_preset(preset: &str) -> (i64, i64) {
    match preset {
        "23.976" => (24000, 1001),
        "25" => (25, 1),
        "29.97" => (30000, 1001),
        "30" => (30, 1),
        "59.94" => (60000, 1001),
        "60" => (60, 1),
        _ => (24, 1),
    }
}

/// Toolbar action buttons for `T03` Interchange, `T04` Lottie import, and `T05` `.aep` dry-run import.
pub fn draw_trunk_bridge_buttons(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    if action_button(
        ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::new(
            "pf-sync-links",
            "refresh",
            "Sync Comps",
            "Re-render dirty linked .ecproj comps & replaceFootage in place (T01)",
        ),
    )
    .clicked()
    {
        ctx.exec(
            "video.sync_links",
            json!({}),
            Some("Synced dirty EffectCraft linked compositions"),
        );
    }

    if action_button(
        ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::new(
            "pf-interchange-toggle",
            "external",
            "Interchange ▾",
            "OTIO / FCPXML / XML / EDL / AAF / OMF / ALE (T03)",
        )
        .selected(ctx.state.project.interchange_open),
    )
    .clicked()
    {
        ctx.state.project.interchange_open = !ctx.state.project.interchange_open;
    }

    if action_button(
        ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::new("pf-import-lottie", "sparkles", "+ Lottie", "Import .json / .lottie into EffectCraft (T04)"),
    )
    .clicked()
    {
        ctx.push(UiAction::PickFile {
            target: PickerTarget::ImportLottie,
            title: "Import Lottie (.json / .lottie)".into(),
            extensions: vec!["json".into(), "lottie".into()],
            directory: false,
            save: false,
        });
    }

    if action_button(
        ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::new("pf-import-aep", "folder", "+ .AEP", "Dry-run inspect & import .aep into .ecproj (T05)"),
    )
    .clicked()
    {
        ctx.push(UiAction::PickFile {
            target: PickerTarget::ImportAep,
            title: "Inspect & Import .AEP Project".into(),
            extensions: vec!["aep".into()],
            directory: false,
            save: false,
        });
    }
}

/// Collapsible `Interchange` (`T03`), `.AEP` dry-run report (`T05`), and `EC` Dynamic-Link (`T01`/`T02`/`T10`) panels.
pub fn draw_trunk_bridge_drawers(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    if ctx.state.project.interchange_open {
        draw_interchange_drawer(ui, ctx);
    }
    if let Some(aep_src) = ctx
        .state
        .cache
        .aep_report
        .get("source")
        .and_then(Value::as_str)
        .map(str::to_owned)
    {
        draw_aep_dry_run_banner(ui, ctx, &aep_src);
    }
    if let Some(ec_path) = ctx.state.project.link_popup_path.clone() {
        draw_dynamic_link_config_bar(ui, ctx, &ec_path);
    }
}

fn draw_interchange_drawer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.add_space(4.0);
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Interchange (T03):").size(11.0).strong().color(ctx.pal.accent_sky));
            for fmt in ["otio", "fcpxml", "xml", "edl", "aaf", "omf", "ale"] {
                let id = format!("pf-ix-fmt-{fmt}");
                let sel = ctx.state.project.interchange_format == fmt;
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only(&id, fmt, "Select NLE interchange format").selected(sel),
                )
                .clicked()
                {
                    ctx.state.project.interchange_format = fmt.into();
                }
            }
            pill_toggle(
                ui,
                ctx.reg,
                ctx.pal,
                "pf-ix-prerender",
                "Prerender .ecproj",
                &mut ctx.state.project.interchange_prerender,
                ctx.pal.accent_emerald,
            );
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("pf-ix-export", "upload", "Export → EC", "Export FilmCraft timeline interchange to EffectCraft")
                    .primary(),
            )
            .clicked()
            {
                let fmt = ctx.state.project.interchange_format.clone();
                let prerender = ctx.state.project.interchange_prerender;
                ctx.exec(
                    "video.markers",
                    json!({
                        "action": "interchange",
                        "direction": "film_to_effect",
                        "format": fmt,
                        "prerender": prerender,
                    }),
                    Some("Exported FilmCraft timeline interchange to EffectCraft"),
                );
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("pf-ix-import", "download", "Import ← EC", "Import EffectCraft timeline interchange into FilmCraft"),
            )
            .clicked()
            {
                let fmt = ctx.state.project.interchange_format.clone();
                let prerender = ctx.state.project.interchange_prerender;
                ctx.exec(
                    "video.markers",
                    json!({
                        "action": "interchange",
                        "direction": "effect_to_film",
                        "format": fmt,
                        "prerender": prerender,
                    }),
                    Some("Imported EffectCraft interchange into FilmCraft"),
                );
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("pf-ix-markers-csv", "copy", "Markers CSV", "Export sequence review notes CSV (T35)"),
            )
            .clicked()
            {
                ctx.exec(
                    "video.markers",
                    json!({"action": "export_csv"}),
                    Some("Exported FilmCraft sequence markers CSV"),
                );
            }
        });
    });
}

fn draw_aep_dry_run_banner(ui: &mut Ui, ctx: &mut ViewCtx<'_>, aep_src: &str) {
    let loss_cnt = ctx
        .state
        .cache
        .aep_report
        .get("loss_count")
        .or_else(|| ctx.state.cache.aep_report.pointer("/report/loss_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    ui.add_space(4.0);
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            status_pill(ui, ctx.pal, ".AEP Dry-Run", ctx.pal.accent_amber);
            ui.label(
                RichText::new(format!(
                    "{} · {loss_cnt} conversion notice(s)",
                    aep_src.rsplit('/').next().unwrap_or(aep_src)
                ))
                .size(11.0)
                .color(ctx.pal.text_secondary),
            );
            pill_toggle(
                ui,
                ctx.reg,
                ctx.pal,
                "pf-aep-allow-losses",
                "Allow Losses",
                &mut ctx.state.project.aep_allow_losses,
                ctx.pal.accent_amber,
            );
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("pf-aep-commit", "check", "Convert to .ecproj", "Execute file.importAep").primary(),
            )
            .clicked()
            {
                let allow = ctx.state.project.aep_allow_losses;
                ctx.exec(
                    "effect.import_aep",
                    json!({"source": aep_src, "dry_run": false, "allow_losses": allow}),
                    Some("Imported .aep into EffectCraft (.ecproj)"),
                );
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("pf-aep-dismiss", "close", "Dismiss .aep report")).clicked() {
                ctx.state.cache.aep_report = Value::Null;
            }
        });
    });
}

fn draw_dynamic_link_config_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, ec_path: &str) {
    let comps: Vec<String> = ctx
        .state
        .cache
        .ec_comps
        .get("compositions")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().or_else(|| c["name"].as_str()).map(str::to_owned))
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| vec!["Main".into()]);
    let used_in_count = ctx
        .state
        .cache
        .ec_comps
        .get("used_in_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let baked = ctx
        .state
        .cache
        .ec_comps
        .get("baked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ec_live = ctx
        .state
        .cache
        .bridge_status
        .pointer("/effectcraft/tcp_listening")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    ui.add_space(4.0);
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                status_pill(
                    ui,
                    ctx.pal,
                    if baked { "Baked (.mov)" } else { "Dynamic Link" },
                    if baked { ctx.pal.accent_amber } else { ctx.pal.accent_emerald },
                );
                status_pill(
                    ui,
                    ctx.pal,
                    if ec_live { "EC :9877 Live" } else { "EC Disconnected (CLI)" },
                    if ec_live { ctx.pal.accent_emerald } else { ctx.pal.text_muted },
                );
                ui.label(
                    RichText::new(ec_path.rsplit('/').next().unwrap_or(ec_path))
                        .monospace()
                        .size(10.5)
                        .color(ctx.pal.text_primary),
                );
                status_pill(
                    ui,
                    ctx.pal,
                    &format!("Used in {used_in_count}"),
                    if used_in_count > 0 { ctx.pal.accent_sky } else { ctx.pal.text_muted },
                );
                ui.label(RichText::new("Comp:").size(10.5).color(ctx.pal.text_muted));
                for cname in &comps {
                    let id = format!("pf-link-comp-{cname}");
                    let sel = ctx.state.project.link_comp_name == *cname;
                    if action_button(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        BtnSpec::label_only(&id, cname, "Select .ecproj composition (T01)").selected(sel),
                    )
                    .clicked()
                    {
                        ctx.state.project.link_comp_name = cname.clone();
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("pf-link-close", "close", "Close link bar")).clicked() {
                        ctx.state.project.link_popup_path = None;
                    }
                });
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Track:").size(10.5).color(ctx.pal.text_muted));
                for trk in ["V2", "V3", "V4"] {
                    let id = format!("pf-link-track-{trk}");
                    let sel = ctx.state.project.link_track == trk;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, trk, "Target FilmCraft track").selected(sel)).clicked() {
                        ctx.state.project.link_track = trk.into();
                    }
                }
                ui.label(RichText::new("Alpha Cache:").size(10.5).color(ctx.pal.text_muted));
                for (codec, lbl) in [
                    ("prores", "ProRes 4444"),
                    ("webm", "VP9+Alpha"),
                    ("exr", "OpenEXR 32f"),
                ] {
                    let id = format!("pf-link-codec-{codec}");
                    let sel = ctx.state.project.link_alpha_codec == codec;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Alpha intermediate codec (T02)").selected(sel)).clicked() {
                        ctx.state.project.link_alpha_codec = codec.into();
                    }
                }
                ui.label(RichText::new("FPS:").size(10.5).color(ctx.pal.text_muted));
                for fps in ["23.976", "24", "29.97", "59.94", "60"] {
                    let id = format!("pf-link-fps-{fps}");
                    let sel = ctx.state.project.link_fps_preset == fps;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, fps, "Exact rational frame rate (T10)").selected(sel)).clicked() {
                        ctx.state.project.link_fps_preset = fps.into();
                    }
                }
                let (fps_n, fps_d) = fps_pair_for_preset(&ctx.state.project.link_fps_preset);
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("pf-link-apply", "link", "Link & Render", "Link comp with trim-preserving replaceFootage").primary(),
                )
                .clicked()
                {
                    ctx.exec(
                        "video.link_effectcraft",
                        json!({
                            "project": ec_path,
                            "comp": ctx.state.project.link_comp_name,
                            "track": ctx.state.project.link_track,
                            "format": ctx.state.project.link_alpha_codec,
                            "fps_num": fps_n,
                            "fps_den": fps_d,
                            "sync_fps": true,
                        }),
                        Some("Linked EffectCraft composition onto FilmCraft timeline"),
                    );
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only("pf-link-bake", "Bake (.mov)", "Render & Replace linked comp with ProRes 4444 .mov (T01)"),
                )
                .clicked()
                {
                    ctx.exec(
                        "video.bake_link",
                        json!({
                            "project": ec_path,
                            "comp": ctx.state.project.link_comp_name,
                            "track": ctx.state.project.link_track,
                        }),
                        Some("Baked linked .ecproj comp to ProRes 4444 .mov"),
                    );
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only("pf-link-unbake", "Unbake", "Restore live .ecproj link (unbake: true)"),
                )
                .clicked()
                {
                    ctx.exec(
                        "video.link_effectcraft",
                        json!({
                            "project": ec_path,
                            "comp": ctx.state.project.link_comp_name,
                            "track": ctx.state.project.link_track,
                            "format": ctx.state.project.link_alpha_codec,
                            "unbake": true,
                        }),
                        Some("Restored live unbaked .ecproj link"),
                    );
                }
            });
        });
    });
}

/// Left-rail `Offline Media & Render-Source Protection` card (`T08` + live `:9876`/`:9877` bridge state).
pub fn draw_offline_and_source_guard_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let pre = &ctx.state.cache.bridge_preflight;
    let all_online = pre["all_online"].as_bool().unwrap_or(true);
    let protected_ok = pre["source_overwrite_blocked"].as_bool().unwrap_or(true);
    let ec_missing = pre["ecproj_missing_count"].as_u64().unwrap_or(0);
    let fc_live = ctx
        .state
        .cache
        .bridge_status
        .pointer("/filmcraft/tcp_listening")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ec_live = ctx
        .state
        .cache
        .bridge_status
        .pointer("/effectcraft/tcp_listening")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    fill_card(ui, ctx.pal, false, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Media & Bridge Status").size(11.5).strong().color(ctx.pal.accent_sky));
            status_pill(
                ui,
                ctx.pal,
                if all_online && protected_ok { "Media Online" } else { "Offline Media" },
                if all_online && protected_ok { ctx.pal.accent_emerald } else { ctx.pal.danger },
            );
        });
        ui.horizontal_wrapped(|ui| {
            status_pill(
                ui,
                ctx.pal,
                if fc_live { "FC :9876 Live" } else { "FC Disconnected" },
                if fc_live { ctx.pal.accent_emerald } else { ctx.pal.text_muted },
            );
            status_pill(
                ui,
                ctx.pal,
                if ec_live { "EC :9877 Live" } else { "EC Disconnected" },
                if ec_live { ctx.pal.accent_emerald } else { ctx.pal.text_muted },
            );
        });
        ui.label(
            RichText::new(format!(
                "Missing footage: {ec_missing} · Output guard: {}",
                if protected_ok { "armed" } else { "conflict" }
            ))
            .monospace()
            .size(9.5)
            .color(ctx.pal.text_muted),
        );
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("project-preflight-check", "refresh", "Check Offline", "Run media.status + footage.check + file.checkOutput (T08)"),
            )
            .clicked()
            {
                ctx.exec("bridge.preflight", json!({"action": "check"}), Some("Checked FilmCraft & EffectCraft media status"));
                ctx.exec("bridge.status", json!({"ensure_tokens": true}), None);
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("project-preflight-relink", "link", "Relink", "Run media.relink + footage.replace across trunks (T08)"),
            )
            .clicked()
            {
                ctx.exec("bridge.preflight", json!({"action": "relink"}), Some("Relinked offline media across FilmCraft & EffectCraft"));
            }
        });
    });
}
