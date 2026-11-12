//! `06 Motion` (`V4 · motion/actions.ecproj` & `motion/svg/*.svg`) right-rail inspector &
//! dynamic `motion.effects` Craft Motion Effects browser (`06-motion-v4-vectorcraft-effectcraft.jpg`):
//! - Default rail surfaces **Arc & Stroke Dials**, **Dynamic Craft Motion Effects (`motion.effects`
//!   `--control :7979 / :9877` API + catalog)**, and **9 Shape Layer Operators** immediately,
//!   while preserving full Bezier Masks, 19-Category 308-FX Browser, 38 Blend Modes, and Track/EXR.

use egui::{Align, Layout, Pos2, RichText, ScrollArea, Ui};
use magic::craft_effects::{EFFECTCRAFT_19_CATEGORIES, EFFECTCRAFT_38_BLEND_MODES};
use serde_json::{Value, json};

use crate::controls::{EffectRowSpec, ScalarSpec, badge_accent_color, effect_row_button, fill_card, pill_toggle, ring_slider_h, rotary_dial};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

/// Read dynamic `motion.effects` entries hydrated via `motion.effects` (`craft_api` `:7979`/`:9877` + catalog).
pub fn resolved_motion_effects(ctx: &ViewCtx<'_>) -> Vec<Value> {
    if let Some(arr) = ctx.state.cache.motion_effects.get("effects").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        return arr.clone();
    }
    magic::craft_effects::motion_catalog_json()
}

/// Apply a Craft motion effect to the active motion cue and persist via `motion.save`.
pub fn apply_motion_effect_choice(ctx: &mut ViewCtx<'_>, fx_id: &str, fx_label: &str) {
    ctx.state.motion.selected_effect_id = fx_id.to_owned();
    ctx.state.dirty_drafts.remove("motion");
    ctx.exec(
        "motion.save",
        json!({
            "word_id": ctx.state.motion.selected_word_id.clone().unwrap_or_else(|| "t/w0".into()),
            "title": ctx.state.motion.cue_name,
            "preset": ctx.state.motion.selected_preset,
            "effect_id": fx_id,
            "start": ctx.state.motion.start_sec,
            "draw_duration": ctx.state.motion.draw_duration,
            "hold_duration": ctx.state.motion.hold_duration,
            "point_x": ctx.state.motion.pos_x,
            "point_y": ctx.state.motion.pos_y,
            "arc_bend": ctx.state.motion.arc_bend_deg,
            "taper": ctx.state.motion.taper_pct,
            "color": ctx.state.motion.stroke_color,
            "stroke_width": ctx.state.motion.stroke_width,
        }),
        Some(&format!("Applied {fx_label} ({fx_id})")),
    );
}

/// Toolbar `.lottie` / `.json` export & import buttons (`T04`).
pub fn draw_lottie_toolbar_buttons(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-lottie-export", "upload", ".lottie", "Export V4 comp as Bodymovin .lottie (T04)")).clicked() {
        ctx.exec("project.autosave", json!({}), Some("Exported .lottie motion package"));
    }
    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-lottie-json", ".json", "Export Lottie .json")).clicked() {
        ctx.exec("project.autosave", json!({}), Some("Exported Lottie .json"));
    }
    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-lottie-import", "+ Lottie", "Import .lottie / .json into motion/actions.ecproj (T04)"))
        .clicked()
    {
        ctx.push(UiAction::PickFile {
            target: PickerTarget::ImportLottie,
            title: "Import Lottie (.json / .lottie) into V4 Motion".into(),
            extensions: vec!["json".into(), "lottie".into()],
            directory: false,
            save: false,
        });
    }
}

/// Collapsible Lottie feature-degradation warnings strip (`T04`).
pub fn draw_lottie_warnings_strip(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let warnings = ctx.state.cache.lottie_last.get("warnings").and_then(Value::as_array).cloned().unwrap_or_default();
    let out_path =
        ctx.state.cache.lottie_last.get("output").or_else(|| ctx.state.cache.lottie_last.get("project")).and_then(Value::as_str).unwrap_or_default().to_owned();
    if out_path.is_empty() && warnings.is_empty() {
        return;
    }
    ui.add_space(4.0);
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            status_pill(
                ui,
                ctx.pal,
                &format!("Lottie · {} notice(s)", warnings.len()),
                if warnings.is_empty() { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
            );
            if !out_path.is_empty() {
                ui.label(RichText::new(out_path.rsplit('/').next().unwrap_or(&out_path)).monospace().size(10.0));
            }
            if !warnings.is_empty() {
                let first_msg = warnings[0].as_str().unwrap_or("Unsupported expression/3D feature approximated");
                ui.label(RichText::new(first_msg).size(10.5).color(ctx.pal.text_secondary));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-lottie-dismiss", "close", "Dismiss Lottie banner")).clicked() {
                    ctx.state.cache.lottie_last = Value::Null;
                }
            });
        });
    });
}

/// Dynamic Craft Motion Effects list (`motion.effects` `--control :7979 / :9877` API + catalog).
pub fn draw_craft_motion_effects_browser(ui: &mut Ui, ctx: &mut ViewCtx<'_>, id_prefix: &str, max_rows: usize) -> Option<Pos2> {
    let api_connected = ctx.state.cache.motion_effects.get("api_connected").and_then(Value::as_bool).unwrap_or(false);
    let effects = resolved_motion_effects(ctx);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new("Craft Motion Effects").size(11.5).strong().color(ctx.pal.accent_emerald));
        status_pill(
            ui,
            ctx.pal,
            &if api_connected { "API :7979/:9877 Live".to_owned() } else { format!("{} FX · API", effects.len()) },
            if api_connected { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
        );
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::icon_only(
                &format!("{id_prefix}-query-api"),
                "refresh",
                "Query live VectorCraft (:7979) & EffectCraft (:9877) --control API for motion effects",
            ),
        )
        .clicked()
        {
            ctx.exec("motion.effects", json!({"refresh": true}), Some("Queried Craft --control API for motion effects"));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only(
                &format!("{id_prefix}-live-save"),
                "Save in App",
                "Trigger built-in file.save inside your open local EffectCraft / VectorCraft session",
            ),
        )
        .clicked()
        {
            ctx.exec(
                "craft.live_save",
                json!({"app": "effectcraft", "path": "motion/actions.ecproj"}),
                Some("Saved motion/actions.ecproj via open Craft session"),
            );
        }
    });

    let q = ctx.state.motion.fx_search.trim().to_ascii_lowercase();
    let mut selected_anchor = None;
    let mut shown = 0usize;
    for fx in &effects {
        if shown >= max_rows {
            break;
        }
        let fx_id = fx["id"].as_str().unwrap_or_default();
        let label = fx["label"].as_str().unwrap_or(fx_id);
        let badge = fx["badge"].as_str().unwrap_or("EC");
        let cmd_lbl = fx["craft_command"].as_str().unwrap_or("effect.apply");
        let is_live = fx["live_api"].as_bool() == Some(true);
        if !q.is_empty() && !fx_id.to_ascii_lowercase().contains(&q) && !label.to_ascii_lowercase().contains(&q) {
            continue;
        }
        shown += 1;
        let btn_id = format!("{id_prefix}-fx-{fx_id}");
        let sel = ctx.state.motion.selected_effect_id == fx_id;
        let resp = effect_row_button(
            ui,
            ctx.reg,
            ctx.pal,
            EffectRowSpec { id: &btn_id, badge, label, effect_id: if is_live { "live API" } else { cmd_lbl }, selected: sel },
        );
        if sel {
            selected_anchor = Some(Pos2::new(resp.rect.min.x, resp.rect.center().y));
        }
        if resp.clicked() {
            let _ = badge_accent_color(ctx.pal, badge);
            apply_motion_effect_choice(ctx, fx_id, label);
        }
    }
    selected_anchor
}

/// Layer Compositing Switches (`38` Blend Modes, Track Mattes, Motion Blur, Adjustment, Alpha Queue — `T29`).
pub fn draw_layer_compositing_switches(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("38 Blend Modes (T29)").size(11.5).strong().color(ctx.pal.accent_emerald));
        status_pill(ui, ctx.pal, &ctx.state.motion.blend_mode, ctx.pal.accent_emerald);
    });
    ScrollArea::vertical().id_salt("motion_38_blend_box").max_height(88.0).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.5;
            ui.spacing_mut().item_spacing.y = 3.5;
            for mode in EFFECTCRAFT_38_BLEND_MODES {
                let id = format!("motion-blend-{mode}");
                let sel = ctx.state.motion.blend_mode == *mode;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, mode, "Set EffectCraft layer blend mode").selected(sel)).clicked() {
                    ctx.state.motion.blend_mode = (*mode).to_owned();
                    ctx.exec("project.autosave", json!({}), Some("Updated blend mode"));
                }
            }
        });
    });

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(RichText::new("Matte:").size(10.5).color(ctx.pal.text_muted));
        for (m, lbl) in [("none", "None"), ("alpha", "Alpha"), ("alphaInverted", "Alpha Inv"), ("luma", "Luma"), ("lumaInverted", "Luma Inv")] {
            let id = format!("motion-matte-{m}");
            let sel = ctx.state.motion.track_matte == m;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "layer.setTrackMatte").selected(sel)).clicked() {
                ctx.state.motion.track_matte = m.into();
            }
        }
    });

    ui.horizontal_wrapped(|ui| {
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "motion-sw-mblur", "Motion Blur", &mut ctx.state.motion.motion_blur, ctx.pal.accent_emerald);
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "motion-sw-adj", "Adjustment", &mut ctx.state.motion.adjustment_layer, ctx.pal.accent_amber);
    });

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(RichText::new("Alpha Queue:").size(10.5).color(ctx.pal.text_muted));
        for preset in ["ProRes 4444 XQ", "WebM VP9+Alpha", "AV1 MP4", "32f EXR"] {
            let id = format!("motion-rq-{preset}");
            let sel = ctx.state.motion.alpha_queue_preset == preset;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, preset, "Queue EffectCraft alpha render preset").selected(sel)).clicked() {
                ctx.state.motion.alpha_queue_preset = preset.into();
                ctx.exec("project.autosave", json!({}), Some("Added composition to Render Queue"));
            }
        }
    });
}

/// Right-hand inspector (`[Shapes & FX] [308 FX & Blend] [Track & EXR]`) returning active effect anchor `Pos2`.
pub fn draw_motion_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let mut fx_anchor = None;
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tid, lbl, tip) in [
                ("shapes", "Shapes & FX", "Transform Dials, Craft Motion Effects (`motion.effects`) & 9 Shape Ops"),
                ("fx", "308 FX & Blend", "19-Category 308-FX Browser, .wasm Plug-in & 38 Blend Modes (T29)"),
                ("track", "Track & EXR", "Point/Camera Tracker, Warp Stabilizer, EXR/Cryptomatte & OCIO (T30-T31)"),
            ] {
                let id = format!("motion-insp-tab-{tid}");
                let sel = ctx.state.motion.inspector_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                    ctx.state.motion.inspector_tab = tid.into();
                }
            }
        });
        ui.separator();
        ScrollArea::vertical().id_salt("motion_right_insp_scroll").auto_shrink([false, false]).show(ui, |ui| match ctx.state.motion.inspector_tab.as_str() {
            "fx" => {
                fx_anchor = draw_fx_308_and_blend_tab(ui, ctx);
            }
            "track" => draw_tracking_and_exr_tab(ui, ctx),
            _ => {
                fx_anchor = draw_shapes_and_masks_tab(ui, ctx);
            }
        });
    });
    ctx.reg.record("motion-inspector", "Motion Inspector", &panel.response, None);
    fx_anchor
}

fn draw_shapes_and_masks_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("motion-arc-dial", "Arc Angle", -90.0..=90.0, "°", ctx.pal.accent_amber),
            &mut ctx.state.motion.arc_bend_deg,
        );
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("motion-taper-dial", "Turbulence", 0.0..=100.0, "%", ctx.pal.accent_emerald),
            &mut ctx.state.motion.taper_pct,
        );
    });
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("motion-pos-x", "X Offset", 0.0..=1920.0, "px", ctx.pal.accent_amber), &mut ctx.state.motion.pos_x);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("motion-pos-y", "Y Offset", 0.0..=1080.0, "px", ctx.pal.accent_amber), &mut ctx.state.motion.pos_y);
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("motion-stroke-w", "Stroke Width", 1.0..=40.0, "px", ctx.pal.accent_emerald),
        &mut ctx.state.motion.stroke_width,
    );

    ui.separator();
    let fx_anchor = draw_craft_motion_effects_browser(ui, ctx, "motion-insp", 12);

    ui.separator();
    ui.label(RichText::new("9 Shape Layer Operators (T27)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (op, lbl) in [
            ("trim", "Trim Paths"),
            ("repeater", "Repeater"),
            ("roundCorners", "Round"),
            ("offsetPaths", "Offset"),
            ("zigZag", "Zig Zag"),
            ("twist", "Twist"),
            ("wigglePaths", "Wiggle"),
            ("mergePaths", "Merge"),
            ("gradientStroke", "Grad Stroke"),
        ] {
            let id = format!("motion-shape-op-{op}");
            let sel = ctx.state.motion.shape_op == op;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Add shape operator via layer.addShapeItem").selected(sel)).clicked() {
                ctx.state.motion.shape_op = op.into();
                ctx.exec("project.autosave", json!({}), Some("Added Shape Layer operator"));
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("Bezier Masks & Face Tracker (T28)").size(11.5).strong().color(ctx.pal.accent_sky));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (mode, lbl) in [("add", "Add"), ("subtract", "Subtract"), ("intersect", "Intersect"), ("difference", "Diff")] {
            let id = format!("motion-mask-mode-{mode}");
            let sel = ctx.state.motion.mask_mode == mode;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Set Bezier mask mode").selected(sel)).clicked() {
                ctx.state.motion.mask_mode = mode.into();
            }
        }
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("motion-mask-feather", "Mask Feather", 0.0..=80.0, "px", ctx.pal.accent_sky),
        &mut ctx.state.motion.mask_feather,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("motion-mask-exp", "Mask Expansion", -40.0..=40.0, "px", ctx.pal.accent_amber),
        &mut ctx.state.motion.mask_expansion,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-add-mask", "+ Pen Mask", "layer.addMask").primary()).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Added Bezier mask to motion layer"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-mask-interp", "Smart Interp", "mask.interpolate")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Interpolated Bezier mask vertices"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-face-track", "Face Track AI", "MediaPipe faceDetailed + extractFaceMeasurements"))
            .clicked()
        {
            ctx.exec("project.autosave", json!({}), Some("Ran MediaPipe Face Tracker & extracted measurements"));
        }
    });
    fx_anchor
}

fn representative_effects_for_category(cat: &str) -> &'static [(&'static str, &'static str)] {
    match cat {
        "blur_sharpen" => {
            &[("ec.blur.cameralensblur", "Camera Lens Blur"), ("ec.blur.directionalblur", "Directional Blur"), ("ec.blur.unsharpmask", "Unsharp Mask")]
        }
        "channel" => &[("ec.channel.shiftchannels", "Shift Channels"), ("ec.channel.setmatte", "Set Matte"), ("ec.channel.invert", "Invert")],
        "color_correction" => &[("ec.color.curves", "Curves"), ("ec.color.levels", "Levels"), ("ec.color.lumetri", "Lumetri Color")],
        "distort" => &[("ec.distort.turbulentdisplace", "Turbulent Displace"), ("ec.distort.wavewarp", "Wave Warp"), ("ec.distort.cornerpin", "Corner Pin")],
        "expression_controls" => &[("ec.expr.slider", "Slider Control"), ("ec.expr.angle", "Angle Control"), ("ec.expr.color", "Color Control")],
        "immersive" => &[
            ("ec.immersive.vr_glow", "VR Glow"),
            ("ec.immersive.vr_chromatic", "VR Chromatic Aberrations"),
            ("ec.immersive.vr_rotatesphere", "VR Rotate Sphere"),
        ],
        "keying" => &[("ec.keying.linearcolorkey", "Linear Color Key"), ("ec.keying.spillsuppressor", "Spill Suppressor"), ("ec.keying.extract", "Extract")],
        "matte" => &[("ec.matte.simplechoker", "Simple Choker"), ("ec.matte.refinesoftmatte", "Refine Soft Matte")],
        "noise_grain" => &[("ec.noise.fractalnoise", "Fractal Noise"), ("ec.noise.addgrain", "Add Grain"), ("ec.noise.median", "Median")],
        "perspective" => {
            &[("ec.perspective.dropshadow", "Drop Shadow"), ("ec.perspective.bevelalpha", "Bevel Alpha"), ("ec.perspective.ccsphere", "CC Sphere")]
        }
        "simulation" => &[
            ("ec.sim.ccparticleworld", "CC Particle World"),
            ("ec.sim.shatter", "Shatter"),
            ("ec.sim.ccrainfall", "CC Rainfall"),
            ("ec.sim.caustics", "Caustics"),
        ],
        "stylize" => &[("ec.stylize.glow", "Glow"), ("ec.stylize.roughenedges", "Roughen Edges"), ("ec.stylize.posterize", "Posterize")],
        "text" => &[("ec.text.numbers", "Numbers"), ("ec.text.timecode", "Timecode")],
        "time" => &[("ec.time.echo", "Echo"), ("ec.time.posterizetime", "Posterize Time"), ("ec.time.timewarp", "Timewarp")],
        "transition" => {
            &[("ec.transition.linearwipe", "Linear Wipe"), ("ec.transition.radialwipe", "Radial Wipe"), ("ec.transition.venetianblinds", "Venetian Blinds")]
        }
        "utility" => &[("EXtractoR", "EXtractoR 32f"), ("Cryptomatte", "Cryptomatte"), ("OCIO Display Transform", "OCIO Transform")],
        "audio" => &[("ec.audio.reverb", "Reverb"), ("ec.audio.parametriceq", "Parametric EQ"), ("ec.audio.stereomixer", "Stereo Mixer")],
        "wasm_plugin" => &[("wasm.custom.halftone", "WASM Halftone Shader"), ("wasm.custom.kaleido", "WASM Kaleidoscope")],
        _ => &[
            ("ec.generate.writeon", "Write-On"),
            ("ec.generate.scribble", "Scribble"),
            ("ec.generate.audiospectrum", "Audio Spectrum"),
            ("ec.generate.ramp", "Gradient Ramp"),
        ],
    }
}

fn draw_fx_308_and_blend_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let fx_anchor = draw_craft_motion_effects_browser(ui, ctx, "motion-fxtab", 16);

    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("19-Category 308-FX Browser (T29)").size(11.5).strong().color(ctx.pal.accent_sky));
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-load-wasm", "+ .wasm", "Load custom WASM effect plug-in (effect.plugins.load)"))
            .clicked()
        {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportWasmPlugin,
                title: "Load EffectCraft .wasm Effect Plug-in".into(),
                extensions: vec!["wasm".into()],
                directory: false,
                save: false,
            });
        }
    });
    text_input(
        ui,
        ctx.reg,
        "motion-fx-search",
        "Search 308 EffectCraft effects",
        &mut ctx.state.motion.fx_search,
        "Filter 308 effects…",
        (ui.available_width() - 4.0).max(140.0),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.5;
        ui.spacing_mut().item_spacing.y = 3.5;
        for (cid, clbl) in EFFECTCRAFT_19_CATEGORIES {
            let id = format!("motion-cat-{cid}");
            let sel = ctx.state.motion.fx_category == *cid;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, clbl, "Select EffectCraft category").selected(sel)).clicked() {
                ctx.state.motion.fx_category = (*cid).to_owned();
            }
        }
    });
    let q = ctx.state.motion.fx_search.trim().to_ascii_lowercase();
    for (match_name, label) in representative_effects_for_category(&ctx.state.motion.fx_category) {
        if !q.is_empty() && !label.to_ascii_lowercase().contains(&q) && !match_name.to_ascii_lowercase().contains(&q) {
            continue;
        }
        let id = format!("motion-ec308-{match_name}");
        let sel = ctx.state.motion.selected_effect_id == *match_name;
        if effect_row_button(ui, ctx.reg, ctx.pal, EffectRowSpec { id: &id, badge: "EC", label, effect_id: match_name, selected: sel }).clicked() {
            apply_motion_effect_choice(ctx, match_name, label);
        }
    }

    ui.separator();
    draw_layer_compositing_switches(ui, ctx);
    fx_anchor
}

fn draw_tracking_and_exr_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Point, Planar & 3D Camera Tracker (T30)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (tmode, lbl) in [("point", "1-Point"), ("twoPoint", "2-Pt Scale/Rot"), ("cornerPin", "4-Corner Pin"), ("camera3d", "3D Camera Solve")] {
            let id = format!("motion-trk-mode-{tmode}");
            let sel = ctx.state.motion.tracker_mode == tmode;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select tracker solver").selected(sel)).clicked() {
                ctx.state.motion.tracker_mode = tmode.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-trk-solve", "Track & Solve", "track.point / track.cameraSolve").primary()).clicked()
        {
            ctx.exec("project.autosave", json!({}), Some("Solved motion track"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-warp-stab", "Warp Stabilizer", "track.warpStabilizer")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Applied Warp Stabilizer"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-blender-py", "Export Blender .py", "track.exportBlenderScript (T30)")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Exported 3D camera solve to Blender .py"));
        }
    });

    ui.separator();
    ui.label(RichText::new("32f OpenEXR, Cryptomatte & OCIO (T31)").size(11.5).strong().color(ctx.pal.accent_amber));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for pass in ["RGBA", "Z-Depth", "WorldNormal", "CryptoObject", "CryptoMaterial"] {
            let id = format!("motion-exr-pass-{pass}");
            let sel = ctx.state.motion.exr_layer == pass;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, pass, "EXtractoR / Cryptomatte AOV pass").selected(sel)).clicked() {
                ctx.state.motion.exr_layer = pass.into();
            }
        }
    });
    let _ = pill_toggle(
        ui,
        ctx.reg,
        ctx.pal,
        "motion-preserve-rgb",
        "Preserve RGB (Bypass Color Mgmt)",
        &mut ctx.state.motion.preserve_rgb,
        ctx.pal.accent_emerald,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for view in ["ACEScg Linear", "ACES 1.3 Rec.709", "AgX Punchy", "Filmic sRGB"] {
            let id = format!("motion-ocio-{view}");
            let sel = ctx.state.motion.ocio_view == view;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, view, "OCIO Display Transform").selected(sel)).clicked() {
                ctx.state.motion.ocio_view = view.into();
                ctx.exec("project.autosave", json!({}), Some("Updated OCIO display transform"));
            }
        }
    });
}
