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

use magic_view_api::controls::{ScalarSpec, fill_card, pill_toggle, ring_slider_h, rotary_dial};
use magic_view_api::fonts_ui::{FontPickerSpec, font_family_combo};
use magic_view_api::widgets::{BtnSpec, action_button, status_pill};
use magic_view_api::{PickerTarget, UiAction, ViewCtx};

use crate::state::AnimationViewState;

/// Right-hand inspector (`[Puppet] [3D & Mesh]`) for `T11`–`T16`.
pub fn draw_puppet_and_3d_inspector(state: &mut AnimationViewState, ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, view_obj: Option<&Value>) {
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tid, ico, lbl, tip) in [
                ("puppet", "puppetPin", "Puppet", "5-Pin Puppet Mesh, Live Record, Null Rig & Follow-Through (T11-T13)"),
                ("3d", "mesh3d", "3D & Mesh", "craft-mesh (.glb/.gltf/.obj/.fbx/.stl/.ply/.3mf), DoF Cam & IBL Lights (T14-T16)"),
            ] {
                let id = format!("anim-insp-tab-{tid}");
                let sel = state.inspector_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, ico, lbl, tip).selected(sel)).clicked() {
                    state.inspector_tab = tid.into();
                }
            }
        });
        let avail_fonts = ctx.state.available_fonts.clone();
        let _ = font_family_combo(
            ui,
            ctx.reg,
            ctx.pal,
            FontPickerSpec { id_prefix: "anim-font", label: "Stage Font" },
            &mut state.font_family,
            &avail_fonts,
            &mut ctx.state.font_picker_filter,
            ctx.actions,
        );
        ui.separator();
        ScrollArea::vertical().id_salt("anim_right_insp_scroll").auto_shrink([false, false]).show(ui, |ui| match state.inspector_tab.as_str() {
            "3d" => draw_3d_camera_and_lights_tab(state, ui, ctx, cid, view_obj),
            _ => draw_puppet_tab(state, ui, ctx, cid, view_obj),
        });
    });
}

fn draw_puppet_tab(state: &mut AnimationViewState, ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, view_obj: Option<&Value>) {
    let has_char = !cid.is_empty();
    let pin_cnt = view_obj.and_then(|v| v["pins"].as_array()).map_or(0, Vec::len);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("360° Joint & Bone Dials").size(11.5).strong());
        status_pill(ui, ctx.pal, &format!("{pin_cnt} pins"), ctx.pal.accent_emerald);
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        rotary_dial(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-dial-rot", "Rotate", -180.0..=180.0, "°", ctx.pal.accent_emerald), &mut state.bone_rotation);
        rotary_dial(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-dial-jaw", "Jaw", 0.0..=45.0, "°", ctx.pal.accent_amber), &mut state.jaw_open_deg);
        rotary_dial(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-dial-wing", "Wing", -60.0..=60.0, "°", ctx.pal.accent_sky), &mut state.wing_bank_deg);
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for part in ["head", "torso", "wing_l", "wing_r", "jaw", "tail"] {
            let id = format!("anim-part-{part}");
            let sel = state.selected_part == part;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, part, "Target rig layer").selected(sel)).clicked() {
                state.selected_part = part.into();
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
                "part": state.selected_part,
                "prop": "rotation",
                "value": state.bone_rotation,
                "time": state.scrub_seconds,
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
            let sel = state.puppet_pin_kind == kind;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select EffectCraft puppet pin type").selected(sel)).clicked() {
                state.puppet_pin_kind = kind.into();
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
                    "layer": state.selected_part,
                    "part": state.selected_part,
                    "kind": state.puppet_pin_kind,
                    "position": [state.pivot_x * 1024.0, state.pivot_y * 1024.0],
                }),
                Some("Added EffectCraft Puppet pin"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-pin-set", "Update", "character.puppet.setPin").enabled(has_char)).clicked() {
            ctx.exec(
                "character.puppet.setPin",
                json!({
                    "character": cid,
                    "layer": state.selected_part,
                    "part": state.selected_part,
                    "pin": "Puppet Pin 1",
                    "rotation": state.bone_rotation,
                    "scale": state.bone_scale / 100.0,
                }),
                Some("Updated Puppet pin handles"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-pin-sel", "Select", "character.puppet.selectPins").enabled(has_char)).clicked() {
            ctx.exec("character.puppet.selectPins", json!({"character": cid, "layer": state.selected_part, "pins": ["Puppet Pin 1"]}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("anim-pin-del", "trash", "character.puppet.removePin").enabled(has_char)).clicked() {
            ctx.exec("character.puppet.removePin", json!({"character": cid, "layer": state.selected_part, "pin": "Puppet Pin 1"}), Some("Removed Puppet pin"));
        }
    });

    ui.separator();
    ui.label(RichText::new("Live Pin Record & Null Rig (T12–T13)").size(11.5).strong().color(ctx.pal.accent_amber));
    ui.horizontal_wrapped(|ui| {
        pill_toggle(ui, ctx.reg, ctx.pal, "anim-puppet-rec-live", "Arm Live Drag", &mut state.puppet_record_live, ctx.pal.danger);
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-puppet-rec-commit", "circle", "Record Gesture", "character.puppet.record: Commit gesture samples to pin").enabled(has_char),
        )
        .clicked()
        {
            let t = state.scrub_seconds;
            let px = state.pivot_x * 1024.0;
            let py = state.pivot_y * 1024.0;
            let samples = json!([[t, px, py], [t + 0.1, px + 5.0, py - 3.0], [t + 0.2, px, py]]);
            ctx.exec(
                "character.puppet.record",
                json!({
                    "character": cid,
                    "layer": state.selected_part,
                    "pin": "Puppet Pin 1",
                    "samples": samples,
                    "smoothing": state.puppet_smoothing,
                    "draft_deform": state.puppet_draft_deform,
                    "show_mesh": state.puppet_show_mesh,
                }),
                Some("Recorded Puppet pin gesture in EffectCraft"),
            );
        }
    });
    ui.horizontal_wrapped(|ui| {
        pill_toggle(ui, ctx.reg, ctx.pal, "anim-puppet-draft", "Draft Deform", &mut state.puppet_draft_deform, ctx.pal.accent_sky);
        pill_toggle(ui, ctx.reg, ctx.pal, "anim-puppet-mesh", "Mesh", &mut state.puppet_show_mesh, ctx.pal.accent_emerald);
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-puppet-smooth", "Record Smoothing", 0.0..=10.0, "px", ctx.pal.accent_amber),
        &mut state.puppet_smoothing,
    );
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-follow-delay", "Follow Delay", 0.01..=0.35, "s", ctx.pal.accent_sky), &mut state.follow_delay);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-nulls-follow", "Pins → Nulls", "paths.pointsFollowNulls (T13)").enabled(has_char))
            .clicked()
        {
            ctx.exec(
                "character.puppet.nullRig",
                json!({
                    "character": cid,
                    "part": state.selected_part,
                    "layer": state.selected_part,
                    "mode": "points_follow_nulls",
                }),
                Some("Bound Puppet pins to controller Nulls"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-pins-carry-nulls", "Nulls → Pins", "paths.nullsFollowPoints (T13)").enabled(has_char))
            .clicked()
        {
            ctx.exec(
                "character.puppet.nullRig",
                json!({
                    "character": cid,
                    "part": state.selected_part,
                    "layer": state.selected_part,
                    "mode": "nulls_follow_points",
                }),
                Some("Attached follower Nulls to Puppet pins"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-follow-apply", "Cloth Follow", "character.puppet.follow (T13)").enabled(has_char))
            .clicked()
        {
            ctx.exec(
                "character.puppet.follow",
                json!({
                    "character": cid,
                    "part": state.selected_part,
                    "layer": state.selected_part,
                    "leader": "Pin 1",
                    "pins": ["Pin 2", "Pin 3"],
                    "delay": state.follow_delay,
                    "amount": 100,
                    "cascade": true,
                }),
                Some("Applied secondary wing/cloth follow-through"),
            );
        }
    });

    ui.separator();
    ui.label(RichText::new("ScriptUI & Craft Host Actions").size(11.5).strong().color(ctx.pal.accent_sky));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-script-run", "play", "Run Script", "character.script.run: Execute ExtendScript / ScriptUI in EffectCraft").enabled(has_char),
        )
        .clicked()
        {
            ctx.exec(
                "character.script.run",
                json!({
                    "character": cid,
                    "script": "app.project.activeItem",
                }),
                Some("Executed script in EffectCraft"),
            );
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-open-effectcraft", "external", "Open in EffectCraft", "Open character comp directly in EffectCraft").enabled(has_char),
        )
        .clicked()
        {
            ctx.exec("workspace.open", json!({"app": "effectcraft", "domain": "animation"}), Some("Opened in EffectCraft"));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-open-anicraft", "external", "Open in AniCraft", "Open character / sprite in AniCraft").enabled(has_char),
        )
        .clicked()
        {
            ctx.exec("workspace.open", json!({"app": "anicraft"}), Some("Opened in AniCraft"));
        }
    });
}

#[allow(clippy::cognitive_complexity)]
fn draw_3d_camera_and_lights_tab(state: &mut AnimationViewState, ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str, view_obj: Option<&Value>) {
    ui.label(RichText::new("3D Models (T14)").size(11.5).strong().color(ctx.pal.accent_sky));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("anim-import-3d", "mesh3d", "+ 3D Mesh", "Import & validate .glb/.gltf/.obj/.fbx/.stl/.ply/.3mf via craft-mesh").primary(),
        )
        .clicked()
        {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportModel3d,
                title: "Import 3D Mesh (.glb/.gltf/.obj/.fbx/.stl/.ply/.3mf)".into(),
                extensions: vec!["glb".into(), "gltf".into(), "obj".into(), "fbx".into(), "stl".into(), "ply".into(), "3mf".into()],
                directory: false,
                save: false,
            });
        }
    });

    if let Some(m3d) = view_obj.and_then(|v| v.get("model3d")).filter(|m| m.is_object()) {
        ctx.pal.inset_frame().show(ui, |ui| {
            let fmt = m3d["format"].as_str().unwrap_or("glb");
            let verts = m3d["vertices"].as_u64().unwrap_or(0);
            let tris = m3d["triangles"].as_u64().unwrap_or(0);
            let joints = m3d["joints"].as_u64().unwrap_or(0);
            let anims = m3d["animations"].as_u64().unwrap_or(0);
            ui.label(
                RichText::new(format!("craft-mesh ({fmt}): {verts} verts · {tris} tris · {joints} joints · {anims} clips"))
                    .monospace()
                    .size(9.5)
                    .color(ctx.pal.accent_emerald),
            );
            if let Some(mpath) = m3d["path"].as_str().filter(|p| !p.is_empty()) {
                let mpath_owned = mpath.to_owned();
                ui.horizontal_wrapped(|ui| {
                    for out_fmt in ["glb", "obj", "stl"] {
                        let btn_id = format!("anim-mesh-conv-{out_fmt}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "download", out_fmt, "Convert 3D mesh via craft-mesh (mesh.convert)"))
                            .clicked()
                        {
                            ctx.exec(
                                "mesh.convert",
                                json!({"path": mpath_owned, "format": out_fmt, "character": cid}),
                                Some("Converted 3D mesh via craft-mesh"),
                            );
                        }
                    }
                });
            }
        });
    }

    ui.separator();
    ui.label(RichText::new("DoF Camera & 4-View (T15)").size(11.5).strong().color(ctx.pal.accent_emerald));
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-cam-2node", "Two-Node POI Camera", &mut state.cam_two_node, ctx.pal.accent_emerald);
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-cam-dof", "Depth of Field", &mut state.cam_dof, ctx.pal.accent_sky);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-cam-focal", "Focal Length", 15.0..=200.0, "mm", ctx.pal.accent_emerald), &mut state.cam_focal_mm);
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("anim-cam-ap", "Aperture", 0.0..=120.0, "px", ctx.pal.accent_amber), &mut state.cam_aperture_px);
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("anim-cam-focus", "Focus Distance", 100.0..=3000.0, "px", ctx.pal.accent_sky),
        &mut state.cam_focus_dist,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for tool in ["orbit", "pan", "dolly"] {
            let id = format!("anim-cam-tool-{tool}");
            let sel = state.cam_tool == tool;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, tool, "Camera navigation tool").selected(sel)).clicked() {
                state.cam_tool = tool.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for vw in ["active", "front", "top", "left"] {
            let id = format!("anim-view-{vw}");
            let sel = state.view_layout == vw;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, vw, "Switch 3D viewport").selected(sel)).clicked() {
                state.view_layout = vw.into();
            }
        }
    });

    ui.separator();
    ui.label(RichText::new("Lights & Environment IBL (T16)").size(11.5).strong().color(ctx.pal.accent_amber));
    pill_toggle(ui, ctx.reg, ctx.pal, "anim-light-shadows", "Cast 3D Shadows", &mut state.light_casts_shadows, ctx.pal.accent_amber);
}
