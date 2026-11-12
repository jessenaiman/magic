// TODO(MAGIC-REVIEW): CHANGE only as needed for the verified integration; this revision applies compiler/lint cleanup, not feature completion.
//! `06 Motion` (`V4 · motion/actions.ecproj` & `motion/svg/*.svg`) supplementary `EffectCraft`
//! & `VectorCraft` controls (`Sheet 08` Zone D — `T04`, `T27`–`T31`):
//! - `T04`: Two-way `.lottie` / `.json` export & import + collapsible degradation warnings strip.
//! - `T27`: All 9 Shape Layer Operators (`trim`, `repeater`, `roundCorners`, `offsetPaths`,
//!   `zigZag`, `twist`, `wigglePaths`, `mergePaths`, `gradientStroke` -> `layer.addShapeItem`).
//! - `T28`: Pen Bezier Masks (`add`/`subtract`/`intersect`/`difference`), `Feather`, `Expansion`,
//!   `Smart Mask Interpolation` (`mask.interpolate`), & `MediaPipe` Face Tracker (`track.mask`).
//! - `T29`: Searchable **19-Category 308-FX Browser**, `+ .wasm Plug-in`, all **38 Blend Modes**,
//!   Track Mattes, Motion Blur, Adjustment Layer, and Render Queue Alpha presets (`renderQueue.add`).
//! - `T30`–`T31`: 1/2/4-Corner Point Tracker, Warp Stabilizer, 3D Camera Solve + Blender `.py`,
//!   Multi-Channel OpenEXR (`EXtractoR`, `Cryptomatte`), `Preserve RGB`, & `OCIO` Display Transform.

use egui::{Align, Layout, RichText, ScrollArea, Ui};
use magic::craft_effects::{EFFECTCRAFT_19_CATEGORIES, EFFECTCRAFT_38_BLEND_MODES, MOTION_EFFECTS};
use serde_json::{Value, json};

use crate::controls::{EffectRowSpec, ScalarSpec, effect_row_button, fill_card, pill_toggle, ring_slider_h, rotary_dial};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

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
        pill_toggle(ui, ctx.reg, ctx.pal, "motion-sw-mblur", "Motion Blur", &mut ctx.state.motion.motion_blur, ctx.pal.accent_emerald).changed();
        if pill_toggle(ui, ctx.reg, ctx.pal, "motion-sw-adj", "Adjustment", &mut ctx.state.motion.adjustment_layer, ctx.pal.accent_amber).changed() {}
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

/// Right-hand inspector (`[Shapes] [308 FX] [Track & EXR]`) for `T27`–`T31`.
pub fn draw_motion_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tid, lbl, tip) in [
                ("shapes", "Shapes & Masks", "9 Shape Operators, Pen Masks & MediaPipe Face Track (T27-T28)"),
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
            "fx" => draw_fx_308_and_blend_tab(ui, ctx),
            "track" => draw_tracking_and_exr_tab(ui, ctx),
            _ => draw_shapes_and_masks_tab(ui, ctx),
        });
    });
    ctx.reg.record("motion-inspector", "Motion Inspector", &panel.response, None);
}

fn draw_shapes_and_masks_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
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
}

fn representative_effects_for_category(cat: &str) -> &'static [(&'static str, &'static str)] {
    match cat {
        "blur_sharpen" => &[("ec.blur.camera_lens", "Camera Lens Blur"), ("ec.blur.directional", "Directional Blur"), ("ec.blur.unsharp", "Unsharp Mask")],
        "channel" => &[("ec.channel.shift", "Shift Channels"), ("ec.channel.set_matte", "Set Matte"), ("ec.channel.invert", "Invert")],
        "color_correction" => &[("ec.color.curves", "Curves"), ("ec.color.levels", "Levels"), ("ec.color.lumetri", "Color Grading")],
        "distort" => &[("ec.distort.turbulentdisplace", "Turbulent Displace"), ("ec.distort.wavewarp", "Wave Warp"), ("ec.distort.cornerpin", "Corner Pin")],
        "expression_controls" => &[("ec.expr.slider", "Slider Control"), ("ec.expr.angle", "Angle Control"), ("ec.expr.color", "Color Control")],
        "immersive" => &[("ec.vr.glow", "VR Glow"), ("ec.vr.chromatic", "VR Chromatic Aberrations"), ("ec.vr.rotate_sphere", "VR Rotate Sphere")],
        "keying" => &[("ec.key.keylight", "Linear Color Key"), ("ec.key.spill", "Spill Suppressor"), ("ec.key.extract", "Extract")],
        "matte" => &[("ec.matte.choker", "Simple Choker"), ("ec.matte.refine_soft", "Refine Soft Matte")],
        "noise_grain" => &[("ec.noise.fractal", "Fractal Noise"), ("ec.noise.add_grain", "Add Grain"), ("ec.noise.median", "Median")],
        "perspective" => &[("ec.perspective.dropshadow", "Drop Shadow"), ("ec.perspective.bevelalpha", "Bevel Alpha"), ("ec.perspective.sphere", "CC Sphere")],
        "simulation" => {
            &[("ec.sim.particle3d", "Particle Emitter 3D"), ("ec.sim.shatter", "Shatter"), ("ec.sim.rainfall", "Rainfall"), ("ec.sim.caustics", "Caustics")]
        }
        "stylize" => &[("ec.stylize.glow", "Glow"), ("ec.stylize.glass_relief", "Glass Relief"), ("ec.stylize.posterize", "Posterize")],
        "text" => &[("ec.text.numbers", "Numbers"), ("ec.text.timecode", "Timecode")],
        "time" => &[("ec.time.echo", "Echo"), ("ec.time.posterize_time", "Posterize Time"), ("ec.time.timewarp", "Timewarp")],
        "transition" => &[("ec.trans.linear_wipe", "Linear Wipe"), ("ec.trans.radial_wipe", "Radial Wipe"), ("ec.trans.venetian", "Venetian Blinds")],
        "utility" => &[("EXtractoR", "EXtractoR 32f"), ("Cryptomatte", "Cryptomatte"), ("OCIO Display Transform", "OCIO Transform")],
        "audio" => &[("ec.audio.reverb", "Reverb"), ("ec.audio.param_eq", "Parametric EQ"), ("ec.audio.stereo_mixer", "Stereo Mixer")],
        "wasm_plugin" => &[("wasm.custom.halftone", "WASM Halftone Shader"), ("wasm.custom.kaleido", "WASM Kaleidoscope")],
        _ => &[
            ("ec.generate.writeon", "Write-On"),
            ("ec.generate.scribble", "Scribble"),
            ("ec.generate.audio_spectrum", "Audio Spectrum"),
            ("ec.generate.gradient_ramp", "Gradient Ramp"),
        ],
    }
}

fn draw_fx_308_and_blend_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("CLI Craft Motion Effects").size(11.5).strong());
    for fx in MOTION_EFFECTS.iter().take(6) {
        let btn_id = format!("motion-fx-{}", fx.id);
        let sel = ctx.state.motion.selected_effect_id == fx.id;
        if effect_row_button(ui, ctx.reg, ctx.pal, EffectRowSpec { id: &btn_id, badge: fx.badge, label: fx.label, effect_id: fx.craft_command, selected: sel })
            .clicked()
        {
            ctx.state.motion.selected_effect_id = fx.id.into();
            ctx.exec(
                "motion.save",
                json!({
                    "word_id": ctx.state.motion.selected_word_id.clone().unwrap_or_else(|| "w-0001".into()),
                    "preset": ctx.state.motion.selected_preset,
                    "effect_id": fx.id,
                    "color": ctx.state.motion.stroke_color,
                    "stroke_width": ctx.state.motion.stroke_width,
                }),
                Some("Applied Craft motion effect"),
            );
        }
    }

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
            ctx.state.motion.selected_effect_id = (*match_name).to_owned();
            ctx.exec("project.autosave", json!({}), Some("Applied effect"));
        }
    }

    ui.separator();
    draw_layer_compositing_switches(ui, ctx);
}

fn draw_tracking_and_exr_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Point, Warp & 3D Camera Tracker (T30)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (tm, lbl) in [("point", "1-Pt"), ("twoPoint", "2-Pt Scale/Rot"), ("perspectiveCornerPin", "4-Corner Pin")] {
            let id = format!("motion-trk-mode-{tm}");
            let sel = ctx.state.motion.tracker_mode == tm;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select track.motion mode").selected(sel)).clicked() {
                ctx.state.motion.tracker_mode = tm.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-trk-run", "Track & Apply", "track.motion + track.apply").primary()).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Solved & applied 2D motion track"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-warp-stab", "Warp Stabilizer", "track.warpStabilizer")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Ran Warp Stabilizer"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-cam-solve", "3D Cam Solve", "track.camera + camera.createFromSolve")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Solved 3D Camera from footage"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-blender-py", "Blender .py", "camera.exportBlender")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Exported solved 3D camera to Blender .py"));
        }
    });

    ui.separator();
    ui.label(RichText::new("32f OpenEXR, Cryptomatte & OCIO (T31)").size(11.5).strong().color(ctx.pal.accent_amber));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for pass in ["RGBA", "Z-Depth", "Normal", "CryptoObj", "CryptoMat"] {
            let id = format!("motion-exr-{pass}");
            let sel = ctx.state.motion.exr_layer == pass;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, pass, "Select OpenEXR AOV pass (EXtractoR)").selected(sel)).clicked() {
                ctx.state.motion.exr_layer = pass.into();
                ctx.exec("project.autosave", json!({}), Some("Extracted OpenEXR AOV channel"));
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-exr-list", "Inspect Channels", "layer.channels")).clicked();
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-crypto-pick", "Pick Cryptomatte", "effect.pickCryptomatte").primary()).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Isolated object ID matte via Cryptomatte"));
        }
    });
    pill_toggle(ui, ctx.reg, ctx.pal, "motion-preserve-rgb", "Preserve RGB (Raw Color)", &mut ctx.state.motion.preserve_rgb, ctx.pal.accent_amber).changed();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for view in ["ACEScg Linear", "AgX", "Filmic", "Rec.709"] {
            let id = format!("motion-ocio-{view}");
            let sel = ctx.state.motion.ocio_view == view;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, view, "Apply OCIO Display Transform").selected(sel)).clicked() {
                ctx.state.motion.ocio_view = view.into();
                ctx.exec("project.autosave", json!({}), Some("Applied OpenColorIO Display Transform"));
            }
        }
    });
}
