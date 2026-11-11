//! `04 Animations` supplementary `EffectCraft` inspector & Theatre/Graph dock (`Sheet 08` Zone B):
//! - `T11`–`T13`: 5-pin Puppet Mesh (`position`, `advanced`, `bend`, `starch`, `overlap`),
//!   `character.puppet.{addPin,setPin,selectPins,removePin,record,nullRig,follow}`.
//! - `T14`–`T16`: `Classic 3D` vs `Advanced 3D PBR`, `.glb/.gltf/.obj/.fbx/.stl` 3D Model Import,
//!   3D Primitives, 1/2-Node DoF Camera (`Orbit`/`Pan`/`Dolly` + 4-View), and 5 Light types + IBL.
//! - `T17`: Roto Brush 3.0 (`classical`, `mobilesam`, `sam2.1-hiera-small`), `FG`/`BG`/`Refine`,
//!   `Object Matte`, `Propagate`, and `Freeze`.
//! - `T18`–`T20`: `layer.timing`, Work Area, Layer/Key Stagger, `A/P/S/R/T/M/E/U/EE/UU` property
//!   filter + `Separate XY`, and interactive draggable Cubic Bezier Value/Speed Graph Editor.

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    ScalarSpec, fill_card, paint_cubic_bezier, pill_toggle, ring_slider_h, rotary_dial,
};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

/// Left-column `T18` Time Remapping, Work Area, Layer/Key Stagger, & Precompose card.
pub fn draw_time_and_stagger_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    let sky = ctx.pal.accent_sky;
    fill_card(ui, ctx.pal, false, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⏱ Time & Stagger (T18)").size(11.5).strong().color(sky));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only("anim-precomp", "Precomp", "Precompose character layers (layer.precompose)"),
                )
                .clicked()
                {
                    ctx.exec(
                        "effect.exec",
                        json!({"domain": "character", "character": cid, "command": "layer.precompose", "params": {"name": "Character Precomp"}}),
                        Some("Precomposed character layers in .ecproj"),
                    );
                }
            });
        });
        ring_slider_h(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("anim-time-stretch", "Time Stretch", 25.0..=400.0, "%", sky),
            &mut ctx.state.animation.time_stretch_pct,
        );
        ui.horizontal(|ui| {
            pill_toggle(ui, ctx.reg, ctx.pal, "anim-time-rev", "Reverse", &mut ctx.state.animation.time_reverse, sky);
            pill_toggle(ui, ctx.reg, ctx.pal, "anim-freeze", "Freeze", &mut ctx.state.animation.freeze_frame, ctx.pal.accent_amber);
        });
        ring_slider_h(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("anim-work-out", "Work Area Out", 0.5..=20.0, "s", ctx.pal.accent_emerald),
            &mut ctx.state.animation.work_out_sec,
        );
        ring_slider_h(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("anim-stagger-frames", "Stagger Offset", 1.0..=24.0, "f", ctx.pal.accent_amber),
            &mut ctx.state.animation.stagger_frames,
        );
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::label_only("anim-apply-timing", "Apply Remap", "Apply layer.timing & comp.workArea").primary(),
            )
            .clicked()
            {
                ctx.exec(
                    "effect.exec",
                    json!({
                        "domain": "character",
                        "character": cid,
                        "command": "layer.timing",
                        "params": {
                            "enableTimeRemap": true,
                            "stretch": ctx.state.animation.time_stretch_pct,
                            "reverse": ctx.state.animation.time_reverse,
                            "freeze": ctx.state.animation.freeze_frame,
                        }
                    }),
                    Some("Applied EffectCraft time remapping"),
                );
                ctx.exec(
                    "effect.exec",
                    json!({
                        "domain": "character",
                        "character": cid,
                        "command": "comp.workArea",
                        "params": {"start": ctx.state.animation.work_in_sec, "end": ctx.state.animation.work_out_sec},
                    }),
                    None,
                );
            }
            let delta_sec = ctx.state.animation.stagger_frames / 24.0;
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::label_only("anim-stagger-layers", "Stagger Layers", "Sequence layers via layer.quickOffset"),
            )
            .clicked()
            {
                ctx.exec(
                    "effect.exec",
                    json!({"domain": "character", "character": cid, "command": "layer.quickOffset", "params": {"delta": delta_sec}}),
                    Some("Staggered character layers"),
                );
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::label_only("anim-stagger-keys", "Stagger Keys", "Offset keyframes via keys.quickOffset"),
            )
            .clicked()
            {
                ctx.exec(
                    "effect.exec",
                    json!({"domain": "character", "character": cid, "command": "keys.quickOffset", "params": {"delta": delta_sec}}),
                    Some("Staggered selected keyframes"),
                );
            }
        });
    });
}

/// Right-hand inspector (`[Puppet] [3D & Cam] [Roto AI]`) for `T11`–`T17`.
pub fn draw_puppet_and_3d_inspector(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    cid: &str,
    view_obj: Option<&Value>,
) {
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            for (tid, lbl, tip) in [
                ("puppet", "Puppet", "5-Pin Puppet Mesh, Live Record, Null Rig & Follow-Through (T11-T13)"),
                ("3d", "3D & Cam", "Classic/PBR 3D, .GLB Import, 1/2-Node DoF Cam & IBL Lights (T14-T16)"),
                ("roto", "Roto AI", "Roto Brush 3.0 SAM 2.1, Object Matte & JS Expressions (T17)"),
            ] {
                let id = format!("anim-insp-tab-{tid}");
                let sel = ctx.state.animation.inspector_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                    ctx.state.animation.inspector_tab = tid.into();
                }
            }
        });
        ui.separator();
        ScrollArea::vertical()
            .id_salt("anim_right_insp_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| match ctx.state.animation.inspector_tab.as_str() {
                "3d" => draw_3d_camera_and_lights_tab(ui, ctx, cid),
                "roto" => draw_roto_and_expr_tab(ui, ctx, cid),
                _ => draw_puppet_tab(ui, ctx, cid, view_obj),
            });
    });
}

fn draw_puppet_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, view_obj: Option<&Value>) {
    let has_char = !cid.is_empty();
    let pin_cnt = view_obj.and_then(|v| v["pins"].as_array()).map_or(0, Vec::len);
    ui.horizontal(|ui| {
        ui.label(RichText::new("360° Joint & Bone Dials").size(11.5).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            status_pill(ui, ctx.pal, &format!("{pin_cnt} pins"), ctx.pal.accent_emerald);
        });
    });
    ui.horizontal(|ui| {
        rotary_dial(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-dial-rot", "Rotate", -180.0..=180.0, "°", ctx.pal.accent_emerald), &mut ctx.state.animation.bone_rotation);
        rotary_dial(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-dial-jaw", "Jaw", 0.0..=45.0, "°", ctx.pal.accent_amber), &mut ctx.state.animation.jaw_open_deg);
        rotary_dial(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-dial-wing", "Wing", -60.0..=60.0, "°", ctx.pal.accent_sky), &mut ctx.state.animation.wing_bank_deg);
    });
    ui.horizontal_wrapped(|ui| {
        for part in ["head", "torso", "wing_l", "wing_r", "jaw", "tail"] {
            let id = format!("anim-part-{part}");
            let sel = ctx.state.animation.selected_part == part;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, part, "Target rig layer").selected(sel)).clicked() {
                ctx.state.animation.selected_part = part.into();
            }
        }
    });
    if action_button(
        ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::new("anim-key-pose", "key", "Key Joint Pose", "Write bone transform keyframe at scrub time").enabled(has_char),
    )
    .clicked()
    {
        ctx.exec(
            "character.set",
            json!({
                "character": cid,
                "part": ctx.state.animation.selected_part,
                "prop": "rotation",
                "value": ctx.state.animation.bone_rotation,
                "time": ctx.state.animation.scrub_seconds,
            }),
            Some("Keyed bone pose in .ecproj"),
        );
    }

    ui.separator();
    ui.label(RichText::new("5-Pin Puppet Mesh (T11)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        for (kind, lbl) in [
            ("position", "Position"),
            ("advanced", "Advanced"),
            ("bend", "Bend"),
            ("starch", "Starch"),
            ("overlap", "Overlap"),
        ] {
            let id = format!("anim-pin-kind-{kind}");
            let sel = ctx.state.animation.puppet_pin_kind == kind;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select EffectCraft puppet pin type").selected(sel)).clicked() {
                ctx.state.animation.puppet_pin_kind = kind.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("anim-pin-add", "pin", "Add Pin", "character.puppet.addPin").primary().enabled(has_char)).clicked() {
            ctx.exec(
                "character.puppet.addPin",
                json!({
                    "character": cid,
                    "part": ctx.state.animation.selected_part,
                    "kind": ctx.state.animation.puppet_pin_kind,
                    "position": [ctx.state.animation.pivot_x * 1024.0, ctx.state.animation.pivot_y * 1024.0],
                }),
                Some("Added EffectCraft Puppet pin"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-pin-set", "Update", "character.puppet.setPin").enabled(has_char)).clicked() {
            ctx.exec(
                "character.puppet.setPin",
                json!({
                    "character": cid,
                    "part": ctx.state.animation.selected_part,
                    "index": 0,
                    "rotation": ctx.state.animation.bone_rotation,
                    "scale": ctx.state.animation.bone_scale / 100.0,
                }),
                Some("Updated Puppet pin handles"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-pin-sel", "Select", "character.puppet.selectPins").enabled(has_char)).clicked() {
            ctx.exec("character.puppet.selectPins", json!({"character": cid, "indices": [0]}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("anim-pin-del", "trash", "character.puppet.removePin").enabled(has_char)).clicked() {
            ctx.exec("character.puppet.removePin", json!({"character": cid, "index": 0}), Some("Removed Puppet pin"));
        }
    });

    ui.separator();
    ui.label(RichText::new("Live Pin Record & Null Rig (T12–T13)").size(11.5).strong().color(ctx.pal.accent_amber));
    pill_toggle(
        ui,
        ctx.reg,
        ctx.pal,
        "anim-puppet-rec-live",
        "Arm Live Stage Drag Recording",
        &mut ctx.state.animation.puppet_record_live,
        ctx.pal.danger,
    );
    ui.horizontal(|ui| {
        pill_toggle(ui, ctx.reg, ctx.pal, "anim-puppet-draft", "Draft Deform", &mut ctx.state.animation.puppet_draft_deform, ctx.pal.accent_sky);
        pill_toggle(ui, ctx.reg, ctx.pal, "anim-puppet-mesh", "Mesh", &mut ctx.state.animation.puppet_show_mesh, ctx.pal.accent_emerald);
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-puppet-smooth", "Record Smoothing", 0.0..=10.0, "px", ctx.pal.accent_amber),
        &mut ctx.state.animation.puppet_smoothing,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-follow-delay", "Follow-Through Delay", 0.01..=0.35, "s", ctx.pal.accent_sky),
        &mut ctx.state.animation.follow_delay,
    );
    ui.horizontal_wrapped(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-nulls-follow", "Pins → Nulls", "paths.pointsFollowNulls (T13)").enabled(has_char)).clicked() {
            ctx.exec(
                "character.puppet.nullRig",
                json!({"character": cid, "part": ctx.state.animation.selected_part, "mode": "points_follow_nulls"}),
                Some("Bound Puppet pins to controller Nulls"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-pins-carry-nulls", "Nulls → Pins", "paths.nullsFollowPoints (T13)").enabled(has_char)).clicked() {
            ctx.exec(
                "character.puppet.nullRig",
                json!({"character": cid, "part": ctx.state.animation.selected_part, "mode": "nulls_follow_points"}),
                Some("Attached follower Nulls to Puppet pins"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-follow-apply", "Secondary Follow", "character.puppet.follow (T13)").enabled(has_char)).clicked() {
            ctx.exec(
                "character.puppet.follow",
                json!({"character": cid, "part": ctx.state.animation.selected_part, "delay": ctx.state.animation.follow_delay}),
                Some("Applied secondary wing/cloth follow-through"),
            );
        }
    });
}

fn draw_3d_camera_and_lights_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.label(RichText::new("3D Renderer & Models (T14)").size(11.5).strong().color(ctx.pal.accent_sky));
    ui.horizontal(|ui| {
        for (rmode, lbl) in [("classic3d", "Classic 3D"), ("advanced3d", "Advanced 3D PBR")] {
            let id = format!("anim-renderer-{rmode}");
            let sel = ctx.state.animation.renderer_mode == rmode;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Switch EffectCraft 3D renderer").selected(sel)).clicked() {
                ctx.state.animation.renderer_mode = rmode.into();
                ctx.exec(
                    "effect.exec",
                    json!({"domain": "character", "character": cid, "command": "comp.settings", "params": {"renderer": rmode}}),
                    Some("Updated EffectCraft 3D renderer"),
                );
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-import-3d", "folder", "+ .GLB/.OBJ/.FBX/.STL", "Import 3D model via layer.newModel (T14)").primary(),
        )
        .clicked()
        {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportModel3d,
                title: "Import 3D Model (.glb/.gltf/.obj/.fbx/.stl)".into(),
                extensions: vec!["glb".into(), "gltf".into(), "obj".into(), "fbx".into(), "stl".into()],
                directory: false,
                save: false,
            });
        }
        for prim in ["cube", "sphere", "plane", "cylinder", "torus"] {
            let id = format!("anim-prim-{prim}");
            let sel = ctx.state.animation.primitive_kind == prim;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, prim, "Insert 3D primitive (layer.new3dPrimitive)").selected(sel)).clicked() {
                ctx.state.animation.primitive_kind = prim.into();
                ctx.exec(
                    "effect.exec",
                    json!({"domain": "character", "character": cid, "command": "layer.new3dPrimitive", "params": {"kind": prim}}),
                    Some("Added 3D primitive layer"),
                );
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("DoF Camera & 4-View (T15)").size(11.5).strong().color(ctx.pal.accent_emerald));
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-cam-2node", "Two-Node Point-of-Interest Camera", &mut ctx.state.animation.cam_two_node, ctx.pal.accent_emerald);
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-cam-dof", "Enable Depth of Field", &mut ctx.state.animation.cam_dof, ctx.pal.accent_sky);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-cam-focal", "Focal Length", 15.0..=200.0, "mm", ctx.pal.accent_emerald), &mut ctx.state.animation.cam_focal_mm);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-cam-ap", "Aperture", 0.0..=120.0, "px", ctx.pal.accent_amber), &mut ctx.state.animation.cam_aperture_px);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-cam-focus", "Focus Distance", 100.0..=3000.0, "px", ctx.pal.accent_sky), &mut ctx.state.animation.cam_focus_dist);
    ui.horizontal_wrapped(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("anim-add-camera", "plus", "Add Camera", "Create 3D camera via layer.newCamera").primary()).clicked() {
            let kind = if ctx.state.animation.cam_two_node { "twoNode" } else { "oneNode" };
            ctx.exec(
                "effect.exec",
                json!({
                    "domain": "character",
                    "character": cid,
                    "command": "layer.newCamera",
                    "params": {
                        "kind": kind,
                        "focalLength": ctx.state.animation.cam_focal_mm,
                        "aperture": ctx.state.animation.cam_aperture_px,
                        "focusDistance": ctx.state.animation.cam_focus_dist,
                        "enableDepthOfField": ctx.state.animation.cam_dof,
                    }
                }),
                Some("Added EffectCraft 3D Camera"),
            );
        }
        for tool in ["orbit", "pan", "dolly"] {
            let id = format!("anim-cam-tool-{tool}");
            let sel = ctx.state.animation.cam_tool == tool;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, tool, "Camera navigation tool").selected(sel)).clicked() {
                ctx.state.animation.cam_tool = tool.into();
                ctx.exec("bridge.control", json!({"app": "effectcraft", "action": "control", "method": "ui.set", "params": {"tool": tool}}), None);
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        for vw in ["active", "front", "top", "left"] {
            let id = format!("anim-view-{vw}");
            let sel = ctx.state.animation.view_layout == vw;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, vw, "Switch 3D viewport").selected(sel)).clicked() {
                ctx.state.animation.view_layout = vw.into();
                ctx.exec("bridge.control", json!({"app": "effectcraft", "action": "control", "method": "ui.set", "params": {"viewer": {"view": vw}}}), None);
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("Lights & Environment IBL (T16)").size(11.5).strong().color(ctx.pal.accent_amber));
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-light-shadows", "Cast 3D Shadows", &mut ctx.state.animation.light_casts_shadows, ctx.pal.accent_amber);
    ui.horizontal_wrapped(|ui| {
        for lkind in ["Parallel", "Spot", "Point", "Ambient", "Environment"] {
            let id = format!("anim-light-{lkind}");
            let sel = ctx.state.animation.light_kind.eq_ignore_ascii_case(lkind);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lkind, "Add 3D/IBL light via layer.newLight").selected(sel)).clicked() {
                ctx.state.animation.light_kind = lkind.to_ascii_lowercase();
                ctx.exec(
                    "effect.exec",
                    json!({
                        "domain": "character",
                        "character": cid,
                        "command": "layer.newLight",
                        "params": {"kind": lkind, "castsShadows": ctx.state.animation.light_casts_shadows},
                    }),
                    Some("Added EffectCraft 3D Light"),
                );
            }
        }
    });
}

fn draw_roto_and_expr_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.label(RichText::new("Roto Brush 3.0 & Object Matte (T17)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        for (mid, lbl) in [
            ("classical", "Classical"),
            ("mobilesam", "MobileSAM"),
            ("sam2.1-hiera-small", "SAM 2.1"),
        ] {
            let id = format!("anim-sam-{mid}");
            let sel = ctx.state.animation.sam_model == mid;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select roto.model.select AI model").selected(sel)).clicked() {
                ctx.state.animation.sam_model = mid.into();
                ctx.exec(
                    "effect.exec",
                    json!({"domain": "character", "character": cid, "command": "roto.model.select", "params": {"id": mid}}),
                    Some("Selected Roto Brush AI model"),
                );
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (sk, lbl) in [("fg", "+ FG Stroke"), ("bg", "- BG Stroke"), ("refine", "Refine Edge")] {
            let id = format!("anim-roto-stroke-{sk}");
            let sel = ctx.state.animation.roto_stroke_kind == sk;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Apply roto.stroke").selected(sel)).clicked() {
                ctx.state.animation.roto_stroke_kind = sk.into();
                ctx.exec(
                    "effect.exec",
                    json!({
                        "domain": "character",
                        "character": cid,
                        "command": "roto.stroke",
                        "params": {"kind": sk, "points": [[512.0, 420.0], [540.0, 560.0]]},
                    }),
                    Some("Applied Roto Brush stroke"),
                );
            }
        }
    });
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-roto-feather", "Matte Feather", 0.0..=25.0, "px", ctx.pal.accent_emerald), &mut ctx.state.animation.sam2_feather);
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-roto-decontam", "Decontaminate Edge Colors", &mut ctx.state.animation.sam2_decontaminate, ctx.pal.accent_sky);
    ui.horizontal_wrapped(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-obj-matte", "Object Matte", "Run one-click objectMatte.apply").primary()).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "character", "character": cid, "command": "objectMatte.apply", "params": {"point": [512.0, 512.0]}}),
                Some("Applied SAM 2.1 Object Matte"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-roto-prop", "Propagate", "Run roto.propagate wait=true")).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "character", "character": cid, "command": "roto.propagate", "params": {"wait": true}}),
                Some("Propagated Roto Brush matte across span"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-roto-freeze", "Freeze", "Cache roto.freeze wait=true")).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "character", "character": cid, "command": "roto.freeze", "params": {"wait": true}}),
                Some("Froze Roto Brush segmentation cache"),
            );
        }
    });

    ui.separator();
    ui.label(RichText::new("Rig JS Expression (T25)").size(11.5).strong().color(ctx.pal.accent_amber));
    text_input(
        ui,
        ctx.reg,
        "anim-expr-input",
        "Rig JS Expression",
        &mut ctx.state.animation.expr_code,
        "loopOut(\"cycle\")",
        ui.available_width().max(150.0),
    );
    ui.horizontal(|ui| {
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-expr-apply", "Apply Expr", "prop.setExpression").primary()).clicked() {
            ctx.exec(
                "effect.exec",
                json!({
                    "domain": "character",
                    "character": cid,
                    "command": "prop.setExpression",
                    "params": {"property": "transform.rotation", "expression": ctx.state.animation.expr_code, "timeBase": "comp"},
                }),
                Some("Applied JavaScript expression to character rig"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-expr-whip", "@ Pick-Whip", "prop.pickWhip")).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "character", "character": cid, "command": "prop.pickWhip", "params": {"source": "transform.rotation", "target": "parent.transform.rotation"}}),
                Some("Linked property via Pick-Whip"),
            );
        }
    });
}

/// Bottom dock (`[Theatre] [Keys / Props] [Graph Editor]`) covering `T19` & `T20`.
pub fn draw_theatre_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    let dock = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            for (tid, lbl, tip) in [
                ("theatre", "Theatre", "Action clips & V2 timeline placement"),
                ("keys", "Keys / Props", "Property reveal filter A/P/S/R/T/M/E/U/EE/UU & Separate XY (T20)"),
                ("curves", "Graph Editor", "Draggable Cubic Bezier Value & Speed Curve Editor (T19)"),
            ] {
                let id = format!("anim-dock-{tid}");
                let sel = ctx.state.animation.dock_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                    ctx.state.animation.dock_tab = tid.into();
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("anim-place-v2", "film", "Place on V2", "Place animated clip onto FilmCraft V2").primary().enabled(!cid.is_empty()),
                )
                .clicked()
                {
                    ctx.exec(
                        "character.shot",
                        json!({
                            "character": cid,
                            "sprite": ctx.state.animation.selected_action,
                            "from": ctx.state.animation.place_start_sec,
                            "duration": ctx.state.animation.place_duration_sec,
                            "studio": true,
                        }),
                        Some("Placed character clip onto FilmCraft V2"),
                    );
                }
            });
        });

        match ctx.state.animation.dock_tab.as_str() {
            "keys" => draw_property_reveal_dock(ui, ctx, cid),
            "curves" => draw_bezier_graph_editor_dock(ui, ctx, cid),
            _ => draw_theatre_clips_row(ui, ctx, cid),
        }
    });
    ctx.reg.record("anim-theatre-dock", "Animation Theatre Dock", &dock.response, None);
}

fn draw_theatre_clips_row(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("CLIPS:").monospace().size(10.0).color(ctx.pal.text_muted));
        for clip in ["idle", "fly", "sing", "jump", "wave", "rest"] {
            let id = format!("theatre-clip-{clip}");
            let sel = ctx.state.animation.selected_action == clip;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, clip, "Trigger action clip").selected(sel)).clicked() {
                ctx.state.animation.selected_action = clip.into();
            }
        }
        let scrub_w = (ui.available_width() - 220.0).max(120.0);
        ui.allocate_ui_with_layout(Vec2::new(scrub_w, 30.0), Layout::top_down(Align::Min), |ui| {
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-scrub", "Scrub", 0.0..=ctx.state.animation.action_duration.max(4.0), "s", ctx.pal.accent_emerald),
                &mut ctx.state.animation.scrub_seconds,
            );
        });
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("theatre-save-action", "Save Action", "Persist custom action clip").enabled(!cid.is_empty())).clicked() {
            ctx.exec(
                "character.action",
                json!({"character": cid, "action": ctx.state.animation.selected_action, "duration": ctx.state.animation.action_duration}),
                Some("Saved character action cycle"),
            );
        }
    });
}

fn draw_property_reveal_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Reveal (T20):").monospace().size(10.0).color(ctx.pal.text_muted));
        for prop in ["A", "P", "S", "R", "T", "M", "E", "U", "EE", "UU"] {
            let id = format!("anim-prop-{prop}");
            let sel = ctx.state.animation.prop_filter == prop;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, prop, "Reveal timeline property").selected(sel)).clicked() {
                ctx.state.animation.prop_filter = prop.into();
                let cmd = format!("timeline.reveal.{}", prop.to_ascii_lowercase());
                ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": cmd, "params": {}}), None);
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-sep-xyz", "Separate XY", "prop.separateDimensions")).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "character", "character": cid, "command": "prop.separateDimensions", "params": {"property": "transform.position", "separate": true}}),
                Some("Separated X / Y / Z position dimensions"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-prev-key", "◀ Prev", "time.previousKey")).clicked() {
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "time.previousKey", "params": {}}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-add-key", "◆ Key", "prop.addKey").primary()).clicked() {
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "prop.addKey", "params": {"time": ctx.state.animation.scrub_seconds}}), Some("Inserted keyframe"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-next-key", "Next ▶", "time.nextKey")).clicked() {
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "time.nextKey", "params": {}}), None);
        }
    });
}

fn draw_bezier_graph_editor_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.horizontal(|ui| {
        let graph_w = (ui.available_width() - 360.0).clamp(150.0, 340.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(graph_w, 82.0), Sense::click_and_drag());
        ui.painter().rect(
            rect,
            CornerRadius::same(6),
            Color32::from_rgb(0x0b, 0x0f, 0x16),
            Stroke::new(1.0, ctx.pal.border),
            StrokeKind::Inside,
        );
        let pad = rect.shrink2(Vec2::new(10.0, 8.0));
        let p0 = Pos2::new(pad.min.x, pad.max.y);
        let p3 = Pos2::new(pad.max.x, pad.min.y);
        let mut h_out = ctx.state.animation.curve_handle_out;
        let mut h_in = ctx.state.animation.curve_handle_in;
        let p1 = Pos2::new(pad.min.x + h_out[0] * pad.width(), pad.max.y - h_out[1] * pad.height());
        let p2 = Pos2::new(pad.min.x + h_in[0] * pad.width(), pad.max.y - h_in[1] * pad.height());

        if resp.dragged()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            let nx = ((pos.x - pad.min.x) / pad.width().max(1.0)).clamp(0.05, 0.95);
            let ny = (1.0 - (pos.y - pad.min.y) / pad.height().max(1.0)).clamp(0.0, 1.0);
            if pos.distance(p1) <= pos.distance(p2) {
                h_out = [nx, ny];
                ctx.state.animation.curve_handle_out = h_out;
            } else {
                h_in = [nx, ny];
                ctx.state.animation.curve_handle_in = h_in;
            }
        }
        if resp.drag_stopped() {
            ctx.exec(
                "effect.exec",
                json!({
                    "domain": "character",
                    "character": cid,
                    "command": "keys.velocity",
                    "params": {
                        "outInfluence": (h_out[0] * 100.0).clamp(1.0, 100.0),
                        "inInfluence": ((1.0 - h_in[0]) * 100.0).clamp(1.0, 100.0),
                        "timeBase": "comp",
                    }
                }),
                Some("Updated Cubic Bezier keyframe velocity handles"),
            );
        }

        ui.painter().line_segment([p0, p1], Stroke::new(1.0, ctx.pal.accent_amber));
        ui.painter().line_segment([p3, p2], Stroke::new(1.0, ctx.pal.accent_sky));
        paint_cubic_bezier(ui, [p0, p1, p2, p3], Stroke::new(2.2, ctx.pal.accent_emerald));
        ui.painter().circle_filled(p1, 4.4, ctx.pal.accent_amber);
        ui.painter().circle_filled(p2, 4.4, ctx.pal.accent_sky);
        ui.painter().text(Pos2::new(pad.min.x + 2.0, pad.min.y + 2.0), Align2::LEFT_TOP, "Value / Speed Bezier", FontId::monospace(8.5), ctx.pal.text_muted);
        ctx.reg.record("anim-bezier-graph", "Graph Editor Canvas", &resp, None);

        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                for (interp, lbl) in [
                    ("linear", "Linear"),
                    ("bezier", "Bezier"),
                    ("continuousBezier", "Cont."),
                    ("hold", "Hold"),
                    ("roving", "Roving"),
                ] {
                    let id = format!("anim-interp-{interp}");
                    let sel = ctx.state.animation.key_interp == interp;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "keys.setInterpolation").selected(sel)).clicked() {
                        ctx.state.animation.key_interp = interp.into();
                        ctx.exec(
                            "effect.exec",
                            json!({"domain": "character", "character": cid, "command": "keys.setInterpolation", "params": {"inType": interp, "outType": interp}}),
                            Some("Set keyframe interpolation"),
                        );
                    }
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-easy-ease-f9", "Easy Ease F9", "keys.easyEase").primary()).clicked() {
                    ctx.state.animation.curve_handle_out = [0.33, 0.0];
                    ctx.state.animation.curve_handle_in = [0.67, 1.0];
                    ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "keys.easyEase", "params": {"mode": "both"}}), Some("Applied Easy Ease (F9)"));
                }
            });
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-key-vel", "Influence", 1.0..=100.0, "%", ctx.pal.accent_amber),
                &mut ctx.state.animation.key_velocity_pct,
            );
        });
    });
}
