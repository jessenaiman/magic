//! `07 Studio` (`film/<song>.fcproj`) BS.1770-4 Loudness (`M`/`S`/`I`/`TP` 4-bar meters),
//! 3-Strip Console Mixer (`A1`/`A2`/`Master`), 35+ Audio DSP, Essential Graphics (`.fcgt`),
//! Broadcast Captions (`C1`), & Pure-Rust Export Queue (`T39`–`T42`).

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::json;

use crate::controls::{ScalarSpec, pill_toggle, ring_slider_h};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

/// `[Audio]` tab: ITU-R BS.1770-4 4-Bar Loudness (`M`, `S`, `I`, `TP`), 3-Strip Console Mixer, 35+ DSP & Ducking (`T39`–`T40`).
pub fn draw_audio_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("BS.1770-4 Loudness & 4× True-Peak (T39)").size(11.5).strong().color(ctx.pal.accent_emerald));
        status_pill(ui, ctx.pal, "Audio-Clock Master", ctx.pal.accent_sky);
    });

    draw_four_bar_loudness_meters(ui, ctx);

    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("fc-lufs-target", "Normalize Target", -24.0..=-9.0, "LUFS", ctx.pal.accent_emerald),
        &mut ctx.state.studio.audio_target_lufs,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (lbl, lufs) in [("Streaming -14", -14.0), ("EBU R128 -23", -23.0), ("ATSC A/85 -24", -24.0)] {
            let id = format!("fc-lufs-pre-{lbl}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "clip.audioGain LUFS preset")).clicked() {
                ctx.state.studio.audio_target_lufs = lufs;
                ctx.exec(
                    "video.finish",
                    json!({"op": "audio_gain", "mode": "normalizeAll", "target_lufs": lufs, "true_peak_dbtp": -1.0}),
                    Some("Normalized clip loudness (BS.1770-4 + True-Peak)"),
                );
            }
        }
        for xf in ["Constant Power", "Constant Gain", "Exp Fade"] {
            let id = format!("fc-aud-xf-{xf}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, xf, "Apply audio crossfade")).clicked() {
                ctx.exec("video.finish", json!({"op": "audio_crossfade", "transition": xf, "duration": 0.5}), Some("Applied audio crossfade"));
            }
        }
    });

    ui.separator();
    draw_three_strip_console_mixer(ui, ctx);

    ui.separator();
    ui.label(RichText::new("35+ Audio DSP, Essential Sound & Ducking (T40)").size(11.5).strong().color(ctx.pal.accent_amber));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (dsp, short_lbl) in [
            ("parametric_eq_full", "Parametric EQ"),
            ("multiband_compressor", "Multiband Comp"),
            ("deesser", "DeEsser"),
            ("denoiser_spectral", "Spectral Denoise"),
            ("dehummer", "DeHummer"),
            ("declicker", "DeClicker"),
            ("dereverb", "DeReverb"),
            ("brickwall_limiter", "Brickwall Limiter"),
            ("convolution_reverb", "Conv Reverb"),
            ("vocal_enhancer", "Vocal Enhancer"),
            ("loudness_radar", "Loudness Radar"),
        ] {
            let id = format!("fc-dsp-{dsp}");
            let sel = ctx.state.studio.audio_dsp_fx == dsp;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, short_lbl, dsp).selected(sel)).clicked() {
                ctx.state.studio.audio_dsp_fx = dsp.into();
                ctx.exec(
                    "video.finish",
                    json!({"op": "mixer_insert", "strip": "A1", "slot": ctx.state.studio.mixer_insert_slot, "pre_fader": true, "effect": dsp}),
                    Some("Inserted pure-Rust Audio DSP effect"),
                );
            }
        }
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("fc-duck-db", "Auto-Ducking Reduction", -24.0..=-3.0, "dB", ctx.pal.accent_amber),
        &mut ctx.state.studio.audio_duck_db,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-duck-run", "Generate Ducking", "essentialSound.generateDucking").primary()).clicked() {
            ctx.exec(
                "video.finish",
                json!({"op": "audio_ducking", "against": "dialogue", "reduce_db": ctx.state.studio.audio_duck_db, "fade_s": 0.35}),
                Some("Generated music bed ducking keyframes"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-ess-repair", "Repair Dialogue", "essentialSound.set spectral repair")).clicked() {
            ctx.exec(
                "video.finish",
                json!({"op": "essential_sound", "values": {"repair.noise.on": true, "repair.rumble.on": true, "clarity.dynamics.on": true}}),
                Some("Applied Essential Sound dialogue repair"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-gcc-sync", "GCC-PHAT Sync", "sequence.syncAudio subsample")).clicked() {
            ctx.exec("video.finish", json!({"op": "audio_sync"}), Some("Aligned multi-track audio via GCC-PHAT"));
        }
    });
}

fn draw_four_bar_loudness_meters(ui: &mut Ui, ctx: &ViewCtx<'_>) {
    let tgt = ctx.state.studio.audio_target_lufs;
    let bars = [
        ("M", "Momentary", tgt + 0.8, "LUFS", ctx.pal.accent_emerald),
        ("S", "Short-Term", tgt + 0.3, "LUFS", ctx.pal.accent_sky),
        ("I", "Integrated", tgt, "LUFS", ctx.pal.accent_emerald),
        ("TP", "True-Peak 4×", -1.0, "dBTP", ctx.pal.accent_amber),
    ];
    let avail_w = (ui.available_width() - 4.0).max(180.0);
    let (panel_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, 66.0), Sense::hover());
    ui.painter().rect(panel_rect, CornerRadius::same(6), Color32::from_rgb(10, 14, 21), Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
    let tgt_frac = ((tgt + 36.0) / 36.0).clamp(0.08, 0.96) as f32;
    for (idx, (code, _tip, val, unit, col)) in bars.into_iter().enumerate() {
        let row_y = panel_rect.min.y + 6.0 + (idx as f32) * 14.2;
        ui.painter().text(Pos2::new(panel_rect.min.x + 8.0, row_y + 5.0), Align2::LEFT_CENTER, code, FontId::monospace(9.0), ctx.pal.text_secondary);
        let track_min_x = panel_rect.min.x + 28.0;
        let track_max_x = (panel_rect.max.x - 74.0).max(track_min_x + 40.0);
        let track_r = Rect::from_min_max(Pos2::new(track_min_x, row_y + 1.5), Pos2::new(track_max_x, row_y + 9.5));
        ui.painter().rect_filled(track_r, CornerRadius::same(3), ctx.pal.bg_input);

        let frac = if code == "TP" { ((val + 18.0) / 18.0).clamp(0.12, 0.94) as f32 } else { ((val + 36.0) / 36.0).clamp(0.08, 0.96) as f32 };
        let fill_r = Rect::from_min_max(track_r.min, Pos2::new(track_r.min.x + track_r.width() * frac, track_r.max.y));
        ui.painter().rect_filled(fill_r, CornerRadius::same(3), col);

        if code != "TP" {
            let tx = track_r.min.x + track_r.width() * tgt_frac;
            ui.painter().line_segment([Pos2::new(tx, track_r.min.y - 1.0), Pos2::new(tx, track_r.max.y + 1.0)], Stroke::new(1.2, ctx.pal.text_primary));
        }
        ui.painter().text(
            Pos2::new(panel_rect.max.x - 6.0, row_y + 5.0),
            Align2::RIGHT_CENTER,
            format!("{val:+.1} {unit}"),
            FontId::monospace(8.5),
            ctx.pal.text_primary,
        );
    }
}

fn draw_three_strip_console_mixer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Console Mixer & Automation (T40)").size(11.2).strong().color(ctx.pal.accent_sky));
        ui.label(RichText::new(format!("Insert: {}", ctx.state.studio.audio_dsp_fx)).monospace().size(9.0).color(ctx.pal.text_muted));
    });
    for (idx, (strip_code, strip_lbl, col)) in
        [("A1", "A1 · Song Bed", ctx.pal.accent_emerald), ("A2", "A2 · Dialogue / SFX", ctx.pal.accent_sky), ("Master", "Master Bus", ctx.pal.accent_amber)]
            .into_iter()
            .enumerate()
    {
        ui.horizontal(|ui| {
            let fader_w = (ui.available_width() - 62.0).max(110.0);
            ui.allocate_ui_with_layout(Vec2::new(fader_w, 28.0), Layout::top_down(Align::Min), |ui| {
                if ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new(&format!("fc-strip-gain-{strip_code}"), strip_lbl, -36.0..=12.0, "dB", col),
                    &mut ctx.state.studio.mixer_gains_db[idx],
                )
                .changed()
                {
                    let db = ctx.state.studio.mixer_gains_db[idx];
                    ctx.exec(
                        "video.finish",
                        json!({"op": "mixer_strip", "strip": strip_code, "volume_db": db, "pan": 0.0, "automation": ctx.state.studio.audio_auto_mode}),
                        None,
                    );
                }
            });
            let muted = ctx.state.studio.mixer_mutes[idx];
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::label_only(&format!("fc-strip-mute-{strip_code}"), if muted { "Muted" } else { "Mute" }, "Toggle mixer strip mute").selected(muted),
            )
            .clicked()
            {
                ctx.state.studio.mixer_mutes[idx] = !muted;
                ctx.exec("video.finish", json!({"op": "mixer_strip", "strip": strip_code, "mute": !muted}), None);
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for mode in ["Off", "Read", "Latch", "Touch", "Write"] {
            let id = format!("fc-auto-{mode}");
            let sel = ctx.state.studio.audio_auto_mode == mode;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, mode, "mixer.writeAutomation").selected(sel)).clicked() {
                ctx.state.studio.audio_auto_mode = mode.into();
                ctx.exec("video.finish", json!({"op": "mixer_automation", "strip": "A1", "mode": mode}), None);
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-clip-mixer", "Clip 0 dB", "clipMixer.set")).clicked() {
            ctx.exec("video.finish", json!({"op": "clip_mixer", "gain_db": 0.0, "pan": 0.0}), None);
        }
    });
}

/// `[Titles]` tab: OpenType Horizontal/Vertical Graphics, Responsive Pinning, Roll/Crawl, `.fcgt` & `C1` Captions (`T41`).
pub fn draw_titles_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    if ctx.state.studio.title_text.is_empty() {
        let proj = ctx.state.project_name();
        if proj != "No project" {
            ctx.state.studio.title_text = proj;
        }
    }
    ui.label(RichText::new("Essential Graphics & OpenType Titles (T41)").size(11.5).strong().color(ctx.pal.accent_emerald));
    text_input(ui, ctx.reg, "fc-title-text", "Graphic Title Text", &mut ctx.state.studio.title_text, "Project Title", (ui.available_width() - 4.0).max(140.0));
    pill_toggle(
        ui,
        ctx.reg,
        ctx.pal,
        "fc-title-vert",
        "Vertical Typesetting (graphics.newVerticalText)",
        &mut ctx.state.studio.title_vertical,
        ctx.pal.accent_sky,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-new-title", "+ Title Layer", "graphics.newText / newVerticalText").primary()).clicked() {
            let txt = if ctx.state.studio.title_text.trim().is_empty() { ctx.state.project_name() } else { ctx.state.studio.title_text.clone() };
            ctx.exec(
                "video.finish",
                json!({
                    "op": "graphic_text",
                    "text": txt,
                    "vertical": ctx.state.studio.title_vertical,
                    "size": 72.0,
                    "font": "Inter",
                    "duration": 5.0,
                }),
                Some("Created FilmCraft OpenType graphic title"),
            );
        }
        for shp in ["rectangle", "ellipse"] {
            let id = format!("fc-new-shape-{shp}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, shp, "graphics.newShape")).clicked() {
                ctx.exec("video.finish", json!({"op": "graphic_shape", "shape": shp, "duration": 5.0}), Some("Added graphic shape layer"));
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-style-bg", "Stroke + Pill BG", "graphics.set multi-stroke & rounded background"))
            .clicked()
        {
            ctx.exec(
                "video.finish",
                json!({"op": "graphic_style", "props": {"strokeWidth": 4.0, "backgroundEnabled": true, "backgroundCornerRadius": 14.0, "shadowOpacity": 0.65}}),
                Some("Applied multi-stroke & rounded card style"),
            );
        }
    });

    ui.separator();
    ui.label(RichText::new("Responsive Design — Pinning, Intro/Outro & Roll/Crawl").size(11.0).strong().color(ctx.pal.accent_amber));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (pin_lbl, edges) in [("Bottom-Left", json!(["bottom", "left"])), ("Top-Right", json!(["top", "right"])), ("Center Safe", json!(["center"]))] {
            let id = format!("fc-pin-{pin_lbl}");
            let sel = ctx.state.studio.title_pin_edge.eq_ignore_ascii_case(pin_lbl);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, pin_lbl, "graphics.setPinning").selected(sel)).clicked() {
                ctx.state.studio.title_pin_edge = pin_lbl.to_ascii_lowercase();
                ctx.exec("video.finish", json!({"op": "graphic_pin", "pin_to": "videoFrame", "edges": edges}), Some("Pinned graphic to frame edges"));
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-resp-time", "Lock 18f In/Out", "graphics.setResponsiveTime")).clicked() {
            ctx.exec("video.finish", json!({"op": "responsive_time", "intro_frames": 18, "outro_frames": 18}), Some("Protected 18f intro/outro keyframes"));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for rc in ["static", "roll", "crawlLeft", "crawlRight"] {
            let id = format!("fc-rc-{rc}");
            let sel = ctx.state.studio.title_roll_crawl == rc;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, rc, "graphics.setRoll").selected(sel)).clicked() {
                ctx.state.studio.title_roll_crawl = rc.into();
                ctx.exec("video.finish", json!({"op": "roll_crawl", "mode": rc, "ease_in": 12, "ease_out": 12}), Some("Configured credits roll/crawl"));
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-export-fcgt", "Save .fcgt", "bridge.templates export_fcgt")).clicked() {
            ctx.exec("bridge.templates", json!({"action": "export_fcgt", "domain": "filmcraft", "name": "studio-title"}), Some("Saved .fcgt Graphic Template"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-import-fcgt", "+ .fcgt/.ectpl", "Import .fcgt or .ectemplate")).clicked() {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportFcgt,
                title: "Import .fcgt / .ectemplate".into(),
                extensions: vec!["fcgt".into(), "ectemplate".into()],
                directory: false,
                save: false,
            });
        }
    });

    ui.separator();
    ui.label(RichText::new("Broadcast Captions Track C1 (T26, T41)").size(11.0).strong().color(ctx.pal.accent_sky));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for std_n in ["Subtitle", "CEA-608", "CEA-708", "Teletext"] {
            let id = format!("fc-cap-std-{std_n}");
            let sel = ctx.state.studio.caption_standard == std_n;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, std_n, "Caption broadcast standard").selected(sel)).clicked() {
                ctx.state.studio.caption_standard = std_n.into();
            }
        }
        for ext in ["srt", "vtt", "scc", "mcc", "stl", "ttml"] {
            let id = format!("fc-cap-ext-{ext}");
            let sel = ctx.state.studio.caption_file_ext == ext;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, ext, "Caption sidecar format").selected(sel)).clicked() {
                ctx.state.studio.caption_file_ext = ext.into();
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-cap-sync", "Sync V3→C1", "Push Lyrics to C1")).clicked() {
            ctx.exec(
                "words.captions_sync",
                json!({"direction": "v3_to_c1", "track_format": ctx.state.studio.caption_standard, "format": ctx.state.studio.caption_file_ext}),
                Some("Synced Lyrics onto C1 Captions"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-cap-export", "Export Sidecar", "Export C1 caption file")).clicked() {
            ctx.exec(
                "words.captions_sync",
                json!({"direction": "v3_to_c1", "track_format": ctx.state.studio.caption_standard, "format": ctx.state.studio.caption_file_ext, "export_sidecar": true}),
                Some("Exported broadcast caption sidecar"),
            );
        }
    });
}

/// `[Export]` tab: Pure-Rust Codecs, Containers, HDR PQ/HLG, CRF/VBR/CBR, Offline Guard & Queue Jobs (`T42`).
pub fn draw_export_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Pure-Rust Codec & Container Matrix (T42)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for fmt in ["h264", "hevc", "prores", "dnxhr", "av1", "vp9", "exr", "png", "gif", "wav", "flac", "mp3"] {
            let id = format!("fc-exp-fmt-{fmt}");
            let sel = ctx.state.studio.export_format == fmt;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, fmt, "Select pure-Rust encoder").selected(sel)).clicked() {
                ctx.state.studio.export_format = fmt.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new("Container:").size(10.5).color(ctx.pal.text_muted));
        for cnt in ["mp4", "mov", "mxf", "mkv", "webm"] {
            let id = format!("fc-exp-cnt-{cnt}");
            let sel = ctx.state.studio.export_container == cnt;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, cnt, "Output container").selected(sel)).clicked() {
                ctx.state.studio.export_container = cnt.into();
            }
        }
        ui.label(RichText::new("HDR:").size(10.5).color(ctx.pal.text_muted));
        for hdr in ["sdr", "pq", "hlg"] {
            let id = format!("fc-exp-hdr-{hdr}");
            let sel = ctx.state.studio.export_hdr_transfer == hdr;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, hdr, "HDR 10-bit transfer").selected(sel)).clicked() {
                ctx.state.studio.export_hdr_transfer = hdr.into();
            }
        }
    });

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for bm in ["crf", "vbr", "cbr"] {
            let id = format!("fc-exp-bm-{bm}");
            let sel = ctx.state.studio.bitrate_mode == bm;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, bm, "Rate control mode").selected(sel)).clicked() {
                ctx.state.studio.bitrate_mode = bm.into();
            }
        }
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("fc-exp-crf", "CRF / Quality", 0.0..=51.0, "", ctx.pal.accent_emerald),
        &mut ctx.state.studio.crf_value,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("fc-exp-scale", "Render Scale", 0.25..=2.0, "×", ctx.pal.accent_sky),
        &mut ctx.state.studio.export_scale,
    );

    pill_toggle(ui, ctx.reg, ctx.pal, "fc-exp-gpu", "Linear-Light GPU Compositor", &mut ctx.state.studio.gpu_rendering, ctx.pal.accent_emerald);
    pill_toggle(ui, ctx.reg, ctx.pal, "fc-exp-hw", "Hardware Encoder (VideoToolbox / NVENC)", &mut ctx.state.studio.hw_encoding, ctx.pal.accent_sky);
    pill_toggle(ui, ctx.reg, ctx.pal, "fc-exp-burn", "Burn-in C1 Captions", &mut ctx.state.studio.burn_captions, ctx.pal.accent_amber);
    pill_toggle(ui, ctx.reg, ctx.pal, "fc-exp-offline", "Allow Offline Media Override (T08)", &mut ctx.state.studio.allow_offline, ctx.pal.danger);
    pill_toggle(ui, ctx.reg, ctx.pal, "fc-exp-queue", "Background Export Queue", &mut ctx.state.studio.queue_export, ctx.pal.accent_amber);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("fc-exp-master-run", "upload", "Render Master Film", "Execute video.export with full FilmCraft matrix").primary(),
        )
        .clicked()
        {
            ctx.exec(
                "video.export",
                json!({
                    "format": ctx.state.studio.export_format,
                    "container": ctx.state.studio.export_container,
                    "hdrTransfer": ctx.state.studio.export_hdr_transfer,
                    "bitrateMode": ctx.state.studio.bitrate_mode,
                    "crf": ctx.state.studio.crf_value as u64,
                    "scale": ctx.state.studio.export_scale,
                    "quality": ctx.state.studio.export_quality as u64,
                    "gpuRendering": ctx.state.studio.gpu_rendering,
                    "hwEncoding": ctx.state.studio.hw_encoding,
                    "burnCaptions": ctx.state.studio.burn_captions,
                    "allowOffline": ctx.state.studio.allow_offline,
                    "queue": ctx.state.studio.queue_export,
                }),
                Some("Started FilmCraft master export"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-exp-jobs", "Poll Jobs ETA", "jobs.list background progress")).clicked() {
            ctx.exec("video.finish", json!({"op": "jobs_list"}), Some("Queried FilmCraft background export queue"));
        }
    });
}
