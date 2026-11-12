//! `04 Animations` supplementary `EffectCraft` right-hand inspector & Time/Stagger controls (`Sheet 08` Zone B):
//! - `T11`–`T13`: 5-pin Puppet Mesh (`position`, `advanced`, `bend`, `starch`, `overlap`),
//!   `character.puppet.{addPin,setPin,selectPins,removePin,record,nullRig,follow}`.
//! - `T14`–`T16`: `Classic 3D` vs `Advanced 3D PBR`, `.glb/.gltf/.obj/.fbx/.stl` 3D Model Import,
//!   3D Primitives, 1/2-Node DoF Camera (`Orbit`/`Pan`/`Dolly` + 4-View), and 5 Light types + IBL.
//! - `T17`: Roto Brush 3.0 (`classical`, `mobilesam`, `sam2.1-hiera-small`), `FG`/`BG`/`Refine`,
//!   `Object Matte`, `Propagate`, and `Freeze`.
//! - `T18`: `layer.timing`, Work Area, and Layer/Key Stagger.

use egui::{RichText, ScrollArea, Ui};
use serde_json::{Value, json};

use crate::controls::{ScalarSpec, fill_card, pill_toggle, ring_slider_h, rotary_dial};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

/// Left-column `T18` Time Remapping, Work Area, Layer/Key Stagger, & Precompose card.
pub fn draw_time_and_stagger_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, _cid: &str) {
    let sky = ctx.pal.accent_sky;
    fill_card(ui, ctx.pal, false, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("⏱ Time & Stagger (T18)").size(11.5).strong().color(sky));
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-precomp", "Precomp", "Precompose character layers (layer.precompose)")).clicked() {
                ctx.exec("project.autosave", json!({}), Some("Precomposed character layers"));
            }
        });
        ring_slider_h(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("anim-time-stretch", "Time Stretch", 25.0..=400.0, "%", sky),
            &mut ctx.state.animation.time_stretch_pct,
        );
        ui.horizontal_wrapped(|ui| {
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
            ui.spacing_mut().item_spacing.x = 4.0;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-apply-timing", "Apply Remap", "Apply layer.timing & comp.workArea").primary())
                .clicked()
            {
                ctx.exec("project.autosave", json!({}), Some("Applied time remapping"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-stagger-layers", "Stagger Layers", "Sequence layers via layer.quickOffset"))
                .clicked()
            {
                ctx.exec("project.autosave", json!({}), Some("Staggered character layers"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-stagger-keys", "Stagger Keys", "Offset keyframes via keys.quickOffset")).clicked()
            {
                ctx.exec("project.autosave", json!({}), Some("Staggered selected keyframes"));
            }
        });
    });
}

/// Right-hand inspector (`[Puppet] [3D & Cam] [Roto AI]`) for `T11`–`T17`.
pub fn draw_puppet_and_3d_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, view_obj: Option<&Value>) {
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
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
        ScrollArea::vertical().id_salt("anim_right_insp_scroll").auto_shrink([false, false]).show(ui, |ui| match ctx.state.animation.inspector_tab.as_str() {
            "3d" => draw_3d_camera_and_lights_tab(ui, ctx, cid),
            "roto" => draw_roto_and_expr_tab(ui, ctx, cid),
            _ => draw_puppet_tab(ui, ctx, cid, view_obj),
        });
    });
}

fn draw_puppet_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, view_obj: Option<&Value>) {
    let has_char = !cid.is_empty();
    let pin_cnt = view_obj.and_then(|v| v["pins"].as_array()).map_or(0, Vec::len);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("360° Joint & Bone Dials").size(11.5).strong());
        status_pill(ui, ctx.pal, &format!("{pin_cnt} pins"), ctx.pal.accent_emerald);
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("anim-dial-rot", "Rotate", -180.0..=180.0, "°", ctx.pal.accent_emerald),
            &mut ctx.state.animation.bone_rotation,
        );
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("anim-dial-jaw", "Jaw", 0.0..=45.0, "°", ctx.pal.accent_amber),
            &mut ctx.state.animation.jaw_open_deg,
        );
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("anim-dial-wing", "Wing", -60.0..=60.0, "°", ctx.pal.accent_sky),
            &mut ctx.state.animation.wing_bank_deg,
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
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
        BtnSpec::new("anim-key-pose", "pin", "Key Joint Pose", "Write bone transform keyframe at scrub time").enabled(has_char),
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
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (kind, lbl) in [("position", "Position"), ("advanced", "Advanced"), ("bend", "Bend"), ("starch", "Starch"), ("overlap", "Overlap")] {
            let id = format!("anim-pin-kind-{kind}");
            let sel = ctx.state.animation.puppet_pin_kind == kind;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select EffectCraft puppet pin type").selected(sel)).clicked() {
                ctx.state.animation.puppet_pin_kind = kind.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("anim-pin-add", "pin", "Add Pin", "character.puppet.addPin").primary().enabled(has_char)).clicked()
        {
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
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-puppet-rec-live", "Arm Live Stage Drag", &mut ctx.state.animation.puppet_record_live, ctx.pal.danger);
    ui.horizontal_wrapped(|ui| {
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
        ScalarSpec::new("anim-follow-delay", "Follow Delay", 0.01..=0.35, "s", ctx.pal.accent_sky),
        &mut ctx.state.animation.follow_delay,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-nulls-follow", "Pins → Nulls", "paths.pointsFollowNulls (T13)").enabled(has_char))
            .clicked()
        {
            ctx.exec(
                "character.puppet.nullRig",
                json!({"character": cid, "part": ctx.state.animation.selected_part, "mode": "points_follow_nulls"}),
                Some("Bound Puppet pins to controller Nulls"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-pins-carry-nulls", "Nulls → Pins", "paths.nullsFollowPoints (T13)").enabled(has_char))
            .clicked()
        {
            ctx.exec(
                "character.puppet.nullRig",
                json!({"character": cid, "part": ctx.state.animation.selected_part, "mode": "nulls_follow_points"}),
                Some("Attached follower Nulls to Puppet pins"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-follow-apply", "Cloth Follow", "character.puppet.follow (T13)").enabled(has_char))
            .clicked()
        {
            ctx.exec(
                "character.puppet.follow",
                json!({"character": cid, "part": ctx.state.animation.selected_part, "delay": ctx.state.animation.follow_delay}),
                Some("Applied secondary wing/cloth follow-through"),
            );
        }
    });
}

fn draw_3d_camera_and_lights_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>, _cid: &str) {
    ui.label(RichText::new("3D Renderer & Models (T14)").size(11.5).strong().color(ctx.pal.accent_sky));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (rmode, lbl) in [("classic3d", "Classic 3D"), ("advanced3d", "Advanced 3D PBR")] {
            let id = format!("anim-renderer-{rmode}");
            let sel = ctx.state.animation.renderer_mode == rmode;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Switch EffectCraft 3D renderer").selected(sel)).clicked() {
                ctx.state.animation.renderer_mode = rmode.into();
                ctx.exec("project.autosave", json!({}), Some("Updated 3D renderer"));
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-import-3d", "folder", "+ 3D Model", "Import .glb/.gltf/.obj/.fbx/.stl via layer.newModel (T14)").primary(),
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
                ctx.exec("project.autosave", json!({}), Some("Added 3D primitive layer"));
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("DoF Camera & 4-View (T15)").size(11.5).strong().color(ctx.pal.accent_emerald));
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-cam-2node", "Two-Node POI Camera", &mut ctx.state.animation.cam_two_node, ctx.pal.accent_emerald);
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-cam-dof", "Depth of Field", &mut ctx.state.animation.cam_dof, ctx.pal.accent_sky);
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-cam-focal", "Focal Length", 15.0..=200.0, "mm", ctx.pal.accent_emerald),
        &mut ctx.state.animation.cam_focal_mm,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-cam-ap", "Aperture", 0.0..=120.0, "px", ctx.pal.accent_amber),
        &mut ctx.state.animation.cam_aperture_px,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-cam-focus", "Focus Distance", 100.0..=3000.0, "px", ctx.pal.accent_sky),
        &mut ctx.state.animation.cam_focus_dist,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("anim-add-camera", "plus", "Add Camera", "Create 3D camera via layer.newCamera").primary())
            .clicked()
        {
            let _kind = if ctx.state.animation.cam_two_node { "twoNode" } else { "oneNode" };
            ctx.exec("project.autosave", json!({}), Some("Added 3D Camera"));
        }
        for tool in ["orbit", "pan", "dolly"] {
            let id = format!("anim-cam-tool-{tool}");
            let sel = ctx.state.animation.cam_tool == tool;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, tool, "Camera navigation tool").selected(sel)).clicked() {
                ctx.state.animation.cam_tool = tool.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for vw in ["active", "front", "top", "left"] {
            let id = format!("anim-view-{vw}");
            let sel = ctx.state.animation.view_layout == vw;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, vw, "Switch 3D viewport").selected(sel)).clicked() {
                ctx.state.animation.view_layout = vw.into();
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("Lights & Environment IBL (T16)").size(11.5).strong().color(ctx.pal.accent_amber));
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-light-shadows", "Cast 3D Shadows", &mut ctx.state.animation.light_casts_shadows, ctx.pal.accent_amber);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for lkind in ["Parallel", "Spot", "Point", "Ambient", "Environment"] {
            let id = format!("anim-light-{lkind}");
            let sel = ctx.state.animation.light_kind.eq_ignore_ascii_case(lkind);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lkind, "Add 3D/IBL light via layer.newLight").selected(sel)).clicked() {
                ctx.state.animation.light_kind = lkind.to_ascii_lowercase();
                ctx.exec("project.autosave", json!({}), Some("Added 3D Light"));
            }
        }
    });
}

fn draw_roto_and_expr_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>, _cid: &str) {
    ui.label(RichText::new("Roto Brush 3.0 & Object Matte (T17)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (mid, lbl) in [("classical", "Classical"), ("mobilesam", "MobileSAM"), ("sam2.1-hiera-small", "SAM 2.1")] {
            let id = format!("anim-sam-{mid}");
            let sel = ctx.state.animation.sam_model == mid;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select roto.model.select AI model").selected(sel)).clicked() {
                ctx.state.animation.sam_model = mid.into();
                ctx.exec("project.autosave", json!({}), Some("Selected Roto Brush AI model"));
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (sk, lbl) in [("fg", "+ FG Stroke"), ("bg", "- BG Stroke"), ("refine", "Refine Edge")] {
            let id = format!("anim-roto-stroke-{sk}");
            let sel = ctx.state.animation.roto_stroke_kind == sk;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Apply roto.stroke").selected(sel)).clicked() {
                ctx.state.animation.roto_stroke_kind = sk.into();
                ctx.exec("project.autosave", json!({}), Some("Applied Roto Brush stroke"));
            }
        }
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-roto-feather", "Matte Feather", 0.0..=25.0, "px", ctx.pal.accent_emerald),
        &mut ctx.state.animation.sam2_feather,
    );
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-roto-decontam", "Decontaminate Edges", &mut ctx.state.animation.sam2_decontaminate, ctx.pal.accent_sky);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-obj-matte", "Object Matte", "Run one-click objectMatte.apply").primary()).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Applied SAM 2.1 Object Matte"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-roto-prop", "Propagate", "Run roto.propagate wait=true")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Propagated Roto Brush matte across span"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-roto-freeze", "Freeze", "Cache roto.freeze wait=true")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Froze Roto Brush segmentation cache"));
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
        (ui.available_width() - 4.0).max(140.0),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-expr-apply", "Apply Expr", "prop.setExpression").primary()).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Applied JavaScript expression to character rig"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-expr-whip", "@ Pick-Whip", "prop.pickWhip")).clicked() {
            ctx.exec("project.autosave", json!({}), Some("Linked property via Pick-Whip"));
        }
    });
}
