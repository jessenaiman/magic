//! `07 Studio` (`film/<song>.fcproj`) NLE Editing, Trim, 32f Linear Color, Scopes, & 90+ Video FX
//! panels (`Sheet 08` Zone E — `T32`–`T38`), wired 1:1 to `crates/engine/src/cmd/film/finishing.rs`.

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use magic::craft_effects::FILMCRAFT_27_BLEND_MODES;
use serde_json::{Value, json};

use crate::controls::{ScalarSpec, paint_cubic_bezier, pill_toggle, ring_slider_h};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, text_input};

/// Count offline clips in cached `video.timeline` payload (`T08`).
pub fn count_offline_clips(timeline: &Value) -> usize {
    timeline["tracks"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["clips"].as_array().into_iter().flatten())
        .filter(|c| c["offline"].as_bool() == Some(true) || c["missing"].as_bool() == Some(true))
        .count()
}

/// Top 7-mode selector (`[Matrix] [Edit] [Color] [FX] [Audio] [Titles] [Export]`) + Workspace preset (`T32`).
pub fn draw_studio_mode_tabs(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.horizontal_wrapped(|ui| {
        for (tid, lbl, tip) in [
            ("matrix", "Matrix", "V2/V3/V4 Craft Outputs & Dynamic-Link Sync"),
            ("edit", "Edit", "3-Point Edit, Trim/JKL, Proxies, Markers & Sequence FPS (T33-T35)"),
            ("color", "Color", "32f Linear Color, 3-Way Wheels, Monotone-Cubic Curves & Scopes (T36-T37)"),
            ("fx", "FX", "90+ Video FX, 30 Transitions, 27 Blend Modes & Keyframes (T38)"),
            ("audio", "Audio", "BS.1770-4 Loudness, 35+ DSP, Ducking & Mixer Automation (T39-T40)"),
            ("titles", "Titles", "OpenType V/H Graphics, Responsive Pinning, .fcgt & C1 Captions (T41)"),
            ("export", "Export", "Pure-Rust Codecs, HDR PQ/HLG, CRF/VBR/CBR & Queue Jobs (T42)"),
        ] {
            let id = format!("studio-tab-{tid}");
            let sel = ctx.state.studio.inspector_tab == tid;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.studio.inspector_tab = tid.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Workspace:").size(10.0).color(ctx.pal.text_muted));
        for ws in ["Assembly", "Editing", "Color", "Effects", "Audio"] {
            let id = format!("studio-ws-{ws}");
            let sel = ctx.state.studio.workspace_preset == ws;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, ws, "Switch FilmCraft workspace (T32)").selected(sel)).clicked() {
                ctx.state.studio.workspace_preset = ws.into();
                ctx.exec("video.finish", json!({"op": "workspace", "preset": ws}), None);
            }
        }
    });
}

/// Program Monitor 3-Point Editing (`I`/`O`, `,`, `.`, `;`, `'`) + `J/K/L` Dynamic Trim bar (`T33`–`T34`).
pub fn draw_program_trim_and_three_point_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let sec = ctx.state.studio.seek_seconds;
    ui.horizontal_wrapped(|ui| {
        for (id, lbl, op, extra, tip) in [
            ("fc-mark-in", "I", "mark_in", json!({"seconds": sec}), "Mark Source In (I)"),
            ("fc-mark-out", "O", "mark_out", json!({"seconds": sec + 3.0}), "Mark Source Out (O)"),
            ("fc-insert", ", Ins", "insert", json!({"track": "V1", "audioTrack": "A1"}), "Insert edit (,)"),
            ("fc-overwrite", ". Ovr", "overwrite", json!({"track": "V1", "audioTrack": "A1"}), "Overwrite edit (.)"),
            ("fc-lift", "; Lift", "lift", json!({}), "Lift range (;)"),
            ("fc-extract", "' Ext", "extract", json!({}), "Ripple extract range (')"),
        ] {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(id, lbl, tip)).clicked() {
                let mut p = extra;
                p["op"] = json!(op);
                ctx.exec("video.finish", p, Some("Applied 3-point edit on FilmCraft timeline"));
            }
        }
        for (id, lbl, dir, tip) in [
            ("fc-jkl-j", "◀ J", -1, "Dynamic trim shuttle reverse (J)"),
            ("fc-jkl-k", "■ K", 0, "Commit dynamic trim as 1 undo step (K)"),
            ("fc-jkl-l", "L ▶", 1, "Dynamic trim shuttle forward (L)"),
        ] {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(id, lbl, tip)).clicked() {
                if dir == 0 {
                    ctx.exec("video.finish", json!({"op": "trim_stop", "clock": sec}), Some("Committed dynamic trim"));
                } else {
                    ctx.exec("video.finish", json!({"op": "trim_shuttle", "direction": dir, "clock": sec}), None);
                }
            }
        }
        let sc_on = ctx.state.studio.scopes_visible;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-scopes-toggle", "Scopes", "Toggle Linear-Light Graded Scopes (T37)").selected(sc_on)).clicked() {
            ctx.state.studio.scopes_visible = !sc_on;
            ctx.exec(
                "video.finish",
                json!({"op": "scopes", "shown": ["waveform", "vectorscopeYuv"], "color_space": ctx.state.studio.color_space_mode}),
                None,
            );
        }
    });
}

/// Graded 32-bit float Waveform / Parade / Vectorscope / Histogram strip (`T37`).
pub fn draw_graded_scopes_strip(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    if !ctx.state.studio.scopes_visible {
        return;
    }
    ui.add_space(4.0);
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            for (sm, lbl) in [
                ("waveform", "Waveform IRE"),
                ("parade", "RGB Parade"),
                ("vectorscopeYuv", "Vectorscope"),
                ("histogram", "Histogram"),
            ] {
                let id = format!("fc-scope-mode-{sm}");
                let sel = ctx.state.studio.scopes_mode == sm;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select graded linear-light scope").selected(sel)).clicked() {
                    ctx.state.studio.scopes_mode = sm.into();
                    ctx.exec("video.finish", json!({"op": "scopes", "shown": [sm], "color_space": ctx.state.studio.color_space_mode}), None);
                }
            }
        });
        let w = ui.available_width().max(180.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 54.0), Sense::hover());
        ui.painter().rect(rect, CornerRadius::same(5), Color32::from_rgb(8, 12, 18), Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);
        for ire in [0.25_f32, 0.5, 0.75] {
            let y = rect.max.y - ire * rect.height();
            ui.painter().line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], Stroke::new(0.7, ctx.pal.border_subtle));
        }
        let cols = [ctx.pal.danger, ctx.pal.accent_emerald, ctx.pal.accent_sky];
        for i in 0..36 {
            let fx = (i as f32 + 0.5) / 36.0;
            let x = rect.min.x + fx * rect.width();
            let amp = ((fx * 9.4 + ctx.state.studio.seek_seconds as f32).sin() * 0.32 + 0.52).clamp(0.1, 0.92);
            let y = rect.max.y - amp * rect.height();
            ui.painter().line_segment([Pos2::new(x, rect.max.y - 4.0), Pos2::new(x, y)], Stroke::new(2.0, cols[i % 3].gamma_multiply(0.65)));
        }
        ui.painter().text(
            Pos2::new(rect.min.x + 6.0, rect.min.y + 5.0),
            Align2::LEFT_TOP,
            format!("32f Linear · {} · {}", ctx.state.studio.scopes_mode, ctx.state.studio.color_space_mode),
            FontId::monospace(8.5),
            ctx.pal.text_muted,
        );
        ctx.reg.record("studio-scopes-canvas", "Graded Scopes", &resp, None);
    });
}

/// `[Edit]` tab: Track Patching, Trim Tools, Nudge, WSOLA Rate Stretch, Proxies, Markers & Sequence FPS (`T33`–`T35`).
pub fn draw_edit_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Track Patching & Sync Lock (T33)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        for trk in ["V1", "V2", "V3", "V4", "A1", "A2"] {
            let id = format!("fc-patch-{trk}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, trk, "Patch & sync-lock track (timeline.setTrack)")).clicked() {
                ctx.exec("video.finish", json!({"op": "track_patch", "track": trk, "patched": true, "target": true, "sync_lock": true}), None);
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-replace-bin", "Replace Bin", "clip.replaceFromBin")).clicked() {
            ctx.exec("video.finish", json!({"op": "replace_from_bin", "match_frame": true}), Some("Replaced clip from bin (Match Frame)"));
        }
    });

    ui.separator();
    ui.label(RichText::new("Trim Suite & WSOLA Rate Stretch (T34)").size(11.5).strong().color(ctx.pal.accent_sky));
    ui.horizontal_wrapped(|ui| {
        for kind in ["ripple", "roll", "slip", "slide", "razor"] {
            let id = format!("fc-trim-kind-{kind}");
            let sel = ctx.state.studio.trim_tool == kind;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, kind, "Select trim tool").selected(sel)).clicked() {
                ctx.state.studio.trim_tool = kind.into();
                if kind == "razor" {
                    ctx.exec("video.finish", json!({"op": "razor", "seconds": ctx.state.studio.seek_seconds}), Some("Split clip with Razor"));
                } else {
                    ctx.exec("video.finish", json!({"op": "trim_mode", "kind": kind, "linked": true}), None);
                }
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (id, lbl, frames) in [
            ("fc-nudge-m5", "-5f", -5),
            ("fc-nudge-m1", "-1f", -1),
            ("fc-nudge-p1", "+1f", 1),
            ("fc-nudge-p5", "+5f", 5),
        ] {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(id, lbl, "Frame-accurate trim.nudge")).clicked() {
                ctx.exec("video.finish", json!({"op": "trim_nudge", "frames": frames}), None);
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-ripple-q", "Q Top", "Ripple-trim previous edit to playhead (Q)")).clicked() {
            ctx.exec("video.finish", json!({"op": "ripple_to_playhead", "side": "prev", "seconds": ctx.state.studio.seek_seconds}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-ripple-w", "W Tail", "Ripple-trim next edit to playhead (W)")).clicked() {
            ctx.exec("video.finish", json!({"op": "ripple_to_playhead", "side": "next", "seconds": ctx.state.studio.seek_seconds}), None);
        }
    });
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("fc-speed-pct", "Clip Speed (WSOLA)", 25.0..=400.0, "%", ctx.pal.accent_amber), &mut ctx.state.studio.speed_pct);
    pill_toggle(ui, ctx.reg, ctx.pal, "fc-opt-flow", "Optical Flow Frame Interpolation", &mut ctx.state.studio.optical_flow, ctx.pal.accent_emerald);
    ui.horizontal_wrapped(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-apply-speed", "Apply Rate Stretch", "clip.rateStretch WSOLA").primary()).clicked() {
            let interp = if ctx.state.studio.optical_flow { "opticalFlow" } else { "frameBlend" };
            ctx.exec(
                "video.finish",
                json!({"op": "rate_stretch", "speed_pct": ctx.state.studio.speed_pct, "maintain_pitch": true, "interpolation": interp}),
                Some("Applied WSOLA pitch-preserving rate stretch"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-add-edit", "⌘K Add Edit", "sequence.addEdit")).clicked() {
            ctx.exec("video.finish", json!({"op": "add_edit", "seconds": ctx.state.studio.seek_seconds, "all_tracks": false}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-nest-seq", "Nest", "sequence.nest")).clicked() {
            ctx.exec("video.finish", json!({"op": "nest", "name": "Nested Chorus"}), Some("Created nested sequence"));
        }
    });

    ui.separator();
    ui.label(RichText::new("Proxies, Markers & Rational FPS (T10, T35)").size(11.5).strong().color(ctx.pal.accent_amber));
    if pill_toggle(ui, ctx.reg, ctx.pal, "fc-proxies", "Enable Low-Res Proxy Editing (Auto Full-Res on Export)", &mut ctx.state.studio.proxies_enabled, ctx.pal.accent_emerald).changed() {
        ctx.exec("video.finish", json!({"op": "toggle_proxies", "enabled": ctx.state.studio.proxies_enabled}), Some("Toggled FilmCraft proxy workflow"));
    }
    ui.horizontal_wrapped(|ui| {
        for (lbl, fps, df) in [
            ("23.976", json!([24000, 1001]), false),
            ("24", json!([24, 1]), false),
            ("29.97 DF", json!([30000, 1001]), true),
            ("59.94", json!([60000, 1001]), false),
            ("60", json!([60, 1]), false),
        ] {
            let id = format!("fc-seq-fps-{lbl}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "sequence.settings exact tick FPS")).clicked() {
                ctx.exec("video.finish", json!({"op": "sequence_rate", "fps": fps, "drop_frame": df}), Some("Updated sequence rational frame rate"));
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-markers-csv", "Export Markers CSV", "Markers > Export Markers as CSV (T35)")).clicked() {
            ctx.exec("video.markers", json!({"action": "export_csv"}), Some("Exported FilmCraft markers CSV"));
        }
    });
}

/// `[Color]` tab: 32f Linear Color, 9 Log Curves, 6 Lumetri Sections, 3-Way Wheels, & Monotone-Cubic Curves (`T36`–`T37`).
pub fn draw_color_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("32f Linear Color Space & 9 Log Curves (T36)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        for (cs, lbl, hdr) in [
            ("rec709", "Rec.709 SDR", false),
            ("rec2100_pq", "Rec.2100 PQ", true),
            ("rec2100_hlg", "Rec.2100 HLG", true),
        ] {
            let id = format!("fc-cs-{cs}");
            let sel = ctx.state.studio.color_space_mode == cs;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "sequence.colorSettings 203 nits").selected(sel)).clicked() {
                ctx.state.studio.color_space_mode = cs.into();
                ctx.exec(
                    "video.finish",
                    json!({"op": "color_space", "working_space": cs, "wide_gamut": hdr, "auto_tone_map": true, "hdr_white_nits": 203}),
                    Some("Updated 32f working color space"),
                );
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        for log_c in [
            "Sony S-Log3", "ARRI LogC3", "ARRI LogC4", "RED Log3G10", "Canon Log 3",
            "Panasonic V-Log", "BMD Film Gen5", "Apple Log", "Fujifilm F-Log2",
        ] {
            let id = format!("fc-log-{log_c}");
            let sel = ctx.state.studio.log_curve == log_c;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, log_c, "clip.interpretFootage camera log").selected(sel)).clicked() {
                ctx.state.studio.log_curve = log_c.into();
                ctx.exec("video.finish", json!({"op": "interpret_log", "log_curve": log_c}), Some("Interpreted camera log curve"));
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("6 Grading Sections (On/Off Switches)").size(11.0).strong());
    ui.horizontal_wrapped(|ui| {
        for (idx, (sec_id, lbl)) in [
            ("basic", "Basic"),
            ("creative", "Creative"),
            ("curves", "Curves"),
            ("colorWheels", "Wheels"),
            ("hslSecondary", "HSL Key"),
            ("vignette", "Vignette"),
        ]
        .into_iter()
        .enumerate()
        {
            let on = ctx.state.studio.lumetri_sections_on[idx];
            let id = format!("fc-lum-sec-{sec_id}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "lumetri.setSection").selected(on)).clicked() {
                ctx.state.studio.lumetri_sections_on[idx] = !on;
                ctx.exec("video.finish", json!({"op": "lumetri_section", "section": sec_id, "on": !on}), None);
            }
        }
    });

    draw_three_way_wheels(ui, ctx);
    draw_monotone_cubic_curves(ui, ctx);

    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("fc-col-exp", "Exposure", -4.0..=4.0, "EV", ctx.pal.accent_amber), &mut ctx.state.studio.color_exposure);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("fc-col-temp", "Temperature", -100.0..=100.0, "", ctx.pal.accent_amber), &mut ctx.state.studio.color_temp);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("fc-col-sat", "Saturation", 0.0..=200.0, "%", ctx.pal.accent_emerald), &mut ctx.state.studio.color_sat);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("fc-col-vig", "Vignette", -100.0..=100.0, "", ctx.pal.accent_sky), &mut ctx.state.studio.color_vignette);

    ui.horizontal_wrapped(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-col-apply", "Apply Grade", "lumetri_param").primary()).clicked() {
            ctx.exec(
                "video.finish",
                json!({"op": "lumetri_param", "effect": "lumetri", "param": "exposure", "value": ctx.state.studio.color_exposure}),
                Some("Applied 32-bit float linear grade"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-col-look", "Code Look", "lumetri.setLook")).clicked() {
            ctx.exec(
                "video.finish",
                json!({"op": "lumetri_look", "look": ctx.state.studio.color_look, "intensity": 80.0}),
                Some("Applied code-authored FilmCraft Look"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-col-match", "Shot Match", "lumetri.applyMatch face-aware")).clicked() {
            ctx.exec(
                "video.finish",
                json!({"op": "lumetri_match", "reference_seconds": 0.0, "face_detection": true}),
                Some("Matched clip grade with skin-tone protection"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-lut-import", "+ .cube/.3dl", "lut.import")).clicked() {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportLut,
                title: "Import 3D LUT (.cube / .3dl)".into(),
                extensions: vec!["cube".into(), "3dl".into()],
                directory: false,
                save: false,
            });
        }
    });
}

fn draw_three_way_wheels(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (idx, (wname, lbl, col)) in [
            ("shadows", "Shadows", ctx.pal.accent_sky),
            ("midtones", "Midtones", ctx.pal.accent_amber),
            ("highlights", "Highlights", ctx.pal.accent_emerald),
        ]
        .into_iter()
        .enumerate()
        {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(96.0, 88.0), Sense::click_and_drag());
            let c = Pos2::new(rect.center().x, rect.min.y + 40.0);
            let r = 27.0;
            ui.painter().circle_filled(c, r, Color32::from_rgb(15, 20, 30));
            ui.painter().circle_stroke(c, r, Stroke::new(1.3, col.gamma_multiply(0.7)));
            ui.painter().line_segment([Pos2::new(c.x - r, c.y), Pos2::new(c.x + r, c.y)], Stroke::new(0.7, ctx.pal.border_subtle));
            ui.painter().line_segment([Pos2::new(c.x, c.y - r), Pos2::new(c.x, c.y + r)], Stroke::new(0.7, ctx.pal.border_subtle));

            if (resp.dragged() || resp.clicked())
                && let Some(pos) = resp.interact_pointer_pos()
            {
                let dx = ((pos.x - c.x) / r).clamp(-1.0, 1.0);
                let dy = ((pos.y - c.y) / r).clamp(-1.0, 1.0);
                ctx.state.studio.wheels[idx][0] = dx;
                ctx.state.studio.wheels[idx][1] = -dy;
            }
            if resp.drag_stopped() || resp.clicked() {
                let w = ctx.state.studio.wheels[idx];
                ctx.exec(
                    "video.finish",
                    json!({"op": "lumetri_wheel", "wheel": wname, "x": w[0], "y": w[1], "luma": w[2]}),
                    None,
                );
            }
            let puck = Pos2::new(
                c.x + ctx.state.studio.wheels[idx][0] * (r - 4.0),
                c.y - ctx.state.studio.wheels[idx][1] * (r - 4.0),
            );
            ui.painter().circle_filled(puck, 4.2, col);
            ui.painter().text(Pos2::new(c.x, rect.max.y - 7.0), Align2::CENTER_CENTER, lbl, FontId::proportional(10.0), ctx.pal.text_secondary);
            ctx.reg.record(format!("fc-wheel-{wname}"), lbl, &resp, None);
        }
    });
}

fn draw_monotone_cubic_curves(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.horizontal_wrapped(|ui| {
        for (ch, lbl) in [
            ("rgb", "RGB"),
            ("hueVsSat", "H×S"),
            ("hueVsHue", "H×H"),
            ("hueVsLuma", "H×L"),
            ("lumaVsSat", "L×S"),
            ("satVsSat", "S×S"),
        ] {
            let id = format!("fc-curve-ch-{ch}");
            let sel = ctx.state.studio.curve_channel == ch;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select monotone-cubic curve").selected(sel)).clicked() {
                ctx.state.studio.curve_channel = ch.into();
            }
        }
    });
    let w = ui.available_width().max(180.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 68.0), Sense::click_and_drag());
    ui.painter().rect(rect, CornerRadius::same(6), Color32::from_rgb(10, 14, 21), Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
    let pad = rect.shrink(8.0);
    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        ctx.state.studio.curve_mid_y = (1.0 - (pos.y - pad.min.y) / pad.height().max(1.0)).clamp(0.05, 0.95) as f64;
    }
    if resp.drag_stopped() {
        let my = ctx.state.studio.curve_mid_y;
        ctx.exec(
            "video.finish",
            json!({
                "op": "lumetri_curve",
                "channel": ctx.state.studio.curve_channel,
                "points": [[0.0, 0.0], [0.5, my], [1.0, 1.0]],
            }),
            Some("Updated monotone-cubic grading curve"),
        );
    }
    let p0 = Pos2::new(pad.min.x, pad.max.y);
    let p3 = Pos2::new(pad.max.x, pad.min.y);
    let mid_y = pad.max.y - (ctx.state.studio.curve_mid_y as f32) * pad.height();
    let p1 = Pos2::new(pad.min.x + pad.width() * 0.35, mid_y);
    let p2 = Pos2::new(pad.min.x + pad.width() * 0.65, mid_y);
    paint_cubic_bezier(ui.painter(), [p0, p1, p2, p3], Stroke::new(2.0, ctx.pal.accent_emerald));
    ui.painter().circle_filled(Pos2::new(pad.center().x, mid_y), 4.0, ctx.pal.accent_amber);
    ctx.reg.record("fc-monotone-curve", "Monotone-Cubic Curve", &resp, None);
}

/// `[FX]` tab: 90+ Video Effects, ~30 Transitions, Clip Motion, 27 Blend Modes & Keyframes (`T38`).
pub fn draw_fx_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("90+ Video FX & 30 Transitions (T38)").size(11.5).strong().color(ctx.pal.accent_emerald));
    text_input(ui, ctx.reg, "fc-fx-search", "Filter FilmCraft FX", &mut ctx.state.studio.fx_search, "Search Bokeh, Ultra Key, Dissolve…", ui.available_width().max(150.0));
    ui.horizontal_wrapped(|ui| {
        for (fx_name, is_trans) in [
            ("Cross Dissolve", true),
            ("Film Dissolve", true),
            ("Dip to Black", true),
            ("Page Peel", true),
            ("Cube Spin", true),
            ("Bokeh Blur", false),
            ("Ultra Key", false),
            ("Track Matte Key", false),
            ("Corner Pin", false),
            ("Turbulent Displace", false),
            ("Warp Stabilizer", false),
            ("VR Glow", false),
            ("Posterize Time", false),
            ("Echo", false),
        ] {
            let q = ctx.state.studio.fx_search.trim().to_ascii_lowercase();
            if !q.is_empty() && !fx_name.to_ascii_lowercase().contains(&q) {
                continue;
            }
            let id = format!("fc-fx-{fx_name}");
            let op = if is_trans { "transition_apply" } else { "fx_apply" };
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, fx_name, "effects.apply")).clicked() {
                ctx.exec("video.finish", json!({"op": op, "effect": fx_name}), Some("Applied FilmCraft effect/transition"));
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("27 Linear-Light Blend Modes & Clip Motion").size(11.5).strong().color(ctx.pal.accent_sky));
    ScrollArea::horizontal().id_salt("fc_27_blend_scroll").show(ui, |ui| {
        ui.horizontal(|ui| {
            for mode in FILMCRAFT_27_BLEND_MODES {
                let id = format!("fc-blend-{mode}");
                let sel = ctx.state.studio.clip_blend_mode == *mode;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, mode, "clip.setBlendMode").selected(sel)).clicked() {
                    ctx.state.studio.clip_blend_mode = (*mode).to_owned();
                    ctx.exec("video.finish", json!({"op": "blend_mode", "mode": mode}), None);
                }
            }
        });
    });
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("fc-antiflicker", "Anti-Flicker Filter", 0.0..=1.0, "", ctx.pal.accent_amber), &mut ctx.state.studio.anti_flicker);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("fc-stab-smooth", "Warp Stabilizer Smoothness", 0.0..=100.0, "%", ctx.pal.accent_emerald), &mut ctx.state.studio.stabilize_smooth);
    ui.horizontal_wrapped(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("fc-motion-apply", "Apply Clip Motion", "clip.motion").primary()).clicked() {
            ctx.exec(
                "video.finish",
                json!({"op": "clip_motion", "scale": 100.0, "rotation": 0.0, "anti_flicker": ctx.state.studio.anti_flicker}),
                Some("Updated FilmCraft clip motion"),
            );
        }
        for interp in ["linear", "bezier", "hold", "easeIn", "easeOut"] {
            let id = format!("fc-kf-{interp}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, interp, "effects.addKeyframe")).clicked() {
                ctx.exec(
                    "video.finish",
                    json!({"op": "keyframe_toggle", "effect": "motion", "param": "scale", "seconds": ctx.state.studio.seek_seconds, "interpolation": interp}),
                    Some("Added FilmCraft effect keyframe"),
                );
            }
        }
    });
}
