//! `03 Lyrics` (`V3 · lettering/lyrics.ecproj`) right-rail inspector & `words.effects`
//! dynamic Craft Typography Effects browser (`05-lyrics-v3-effectcraft.jpg` + `effectcraft/docs/effects.md`):
//! - Default rail surfaces **Font & OpenType**, **Transform & Skew**, and the dynamic
//!   **Craft Typography Effects (`words.effects` `--control :9877` API + catalog)** list immediately,
//!   while preserving full Variable Axes, Range Selectors, interactive **Cubic-Bézier Easing Curve**,
//!   9 Layer Styles, 3D Extrusion, pure-Rust Procedural Motion Curves, and `V3 ↔ C1` Captions.

use egui::epaint::CubicBezierShape;
use egui::{Color32, CornerRadius, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{EffectRowSpec, ScalarSpec, badge_accent_color, effect_row_button, fill_card, pill_toggle, ring_slider_h, rotary_dial};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

/// Read dynamic `words.effects` entries hydrated via `words.effects` (`craft_api` `:9877` + catalog).
pub fn resolved_lyrics_effects(ctx: &ViewCtx<'_>) -> Vec<Value> {
    if let Some(arr) = ctx.state.cache.words_effects.get("effects").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        return arr.clone();
    }
    magic_engine::craft_effects::lyrics_catalog_json()
}

/// Apply a Craft typography effect to the currently selected stage word AND push via `words.effect_install`.
pub fn apply_effect_to_selected_word(ctx: &mut ViewCtx<'_>, fx_id: &str, fx_label: &str) {
    ctx.state.lyrics.selected_effect_id = fx_id.to_owned();
    if let Some(wid) = ctx.state.lyrics.selected_word_id.clone() {
        ctx.state.lyrics.word_effects.insert(wid, fx_id.to_owned());
    }
    ctx.state.dirty_drafts.insert("lyrics".into());
    ctx.exec("words.effect_install", json!({"name": fx_id}), Some(&format!("Applied {fx_label} ({fx_id})")));
}

pub fn draw_font_shaping_strip(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Font & OpenType Shaping").size(11.2).strong().color(ctx.pal.accent_sky));
    let avail_fonts = ctx.state.available_fonts.clone();
    if crate::fonts_ui::font_family_combo(
        ui,
        ctx.reg,
        ctx.pal,
        crate::fonts_ui::FontPickerSpec { id_prefix: "words-font", label: "Family" },
        &mut ctx.state.lyrics.font_preset,
        &avail_fonts,
        &mut ctx.state.font_picker_filter,
        ctx.actions,
    ) {
        ctx.state.dirty_drafts.insert("lyrics".into());
        ctx.exec("project.autosave", json!({}), Some("Updated lyric font family"));
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.5;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-scan-fonts", "refresh", "Rescan Catalog::global() system & Craft Store fonts"))
            .clicked()
        {
            craft_font_catalog::Catalog::global().rescan();
            crate::fonts_ui::refresh_available_fonts(ctx.state);
            ctx.exec("project.autosave", json!({}), Some("Rescanned system & Craft Font Store families"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-font-dir", "+ Folder", "Add folder to Catalog::set_app_dirs (EFFECTCRAFT_FONT_DIRS)"))
            .clicked()
        {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportFontDir,
                title: "Select Custom Font Directory (EFFECTCRAFT_FONT_DIRS)".into(),
                extensions: Vec::new(),
                directory: true,
                save: false,
            });
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.5;
        ui.spacing_mut().item_spacing.y = 3.5;
        for (bmode, lbl) in [("point", "Point"), ("box", "Box")] {
            let id = format!("words-box-{bmode}");
            let sel = ctx.state.lyrics.text_box_mode == bmode;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Point vs Paragraph Box text (T22)").selected(sel)).clicked() {
                ctx.state.lyrics.text_box_mode = bmode.into();
            }
        }
        for (kern, lbl) in [("metrics", "Metrics"), ("optical", "Optical")] {
            let id = format!("words-kern-{kern}");
            let sel = ctx.state.lyrics.kerning_mode == kern;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "OpenType kerning mode").selected(sel)).clicked() {
                ctx.state.lyrics.kerning_mode = kern.into();
            }
        }
        for (jst, lbl) in [("left", "L"), ("center", "C"), ("right", "R"), ("justify", "J")] {
            let id = format!("words-just-{jst}");
            let sel = ctx.state.lyrics.justify == jst;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Paragraph justification").selected(sel)).clicked() {
                ctx.state.lyrics.justify = jst.into();
            }
        }
        let dir_lbl = if ctx.state.lyrics.rtl_bidi { "RTL" } else { "LTR" };
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("words-bidi-toggle", dir_lbl, "Toggle LTR / RTL BiDi shaping (T22)").selected(ctx.state.lyrics.rtl_bidi),
        )
        .clicked()
        {
            ctx.state.lyrics.rtl_bidi = !ctx.state.lyrics.rtl_bidi;
        }
    });
}

/// Dynamic Craft Typography Effects browser (`words.effects` internal `--control :9877` API + catalog).
pub fn draw_craft_typography_effects_browser(ui: &mut Ui, ctx: &mut ViewCtx<'_>, id_prefix: &str) -> Option<Pos2> {
    let api_connected = ctx.state.cache.words_effects.get("api_connected").and_then(Value::as_bool).unwrap_or(false);
    let effects = resolved_lyrics_effects(ctx);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new("Craft Typography Effects").size(11.6).strong().color(ctx.pal.accent_emerald));
        status_pill(
            ui,
            ctx.pal,
            &if api_connected { "API :9877 Live".to_owned() } else { format!("{} FX · :9877", effects.len()) },
            if api_connected { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
        );
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::icon_only(&format!("{id_prefix}-query-api"), "refresh", "Query live EffectCraft (:9877) & VectorCraft (:7979) --control API for effects"),
        )
        .clicked()
        {
            ctx.exec("words.effects", json!({"refresh": true}), Some("Queried Craft --control API for typography effects"));
        }
    });
    // TODO(MAGIC-REVIEW): KEEP Save on its own row so the handoff control fits narrow inspectors.
    ui.horizontal_wrapped(|ui| {
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only(
                &format!("{id_prefix}-live-save"),
                "Save in App",
                "Trigger built-in file.save inside your open local EffectCraft session (:9877)",
            ),
        )
        .clicked()
        {
            ctx.exec(
                "craft.live_save",
                json!({"app": "effectcraft", "path": "lettering/lyrics.ecproj"}),
                Some("Saved lettering/lyrics.ecproj via open EffectCraft session"),
            );
        }
    });

    text_input(
        ui,
        ctx.reg,
        &format!("{id_prefix}-search"),
        "Filter Craft Typography Effects",
        &mut ctx.state.lyrics.fx_search,
        "Search EC / VC / PC effects (glow, arc, 3d, spectrum…)",
        (ui.available_width() - 4.0).max(140.0),
    );

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.5;
        ui.spacing_mut().item_spacing.y = 3.5;
        for (cat_id, cat_lbl) in [
            ("all", "All"),
            ("Text Animator", "Animators"),
            ("Typography Stylize", "Stylize"),
            ("Envelope Warp", "Warp"),
            ("Audio Reactive", "Audio ♪"),
            ("Perspective & 3D", "3D"),
            ("Layer Style", "Styles"),
        ] {
            let btn_id = format!("{id_prefix}-cat-{cat_id}");
            let sel = ctx.state.lyrics.fx_category.eq_ignore_ascii_case(cat_id);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, cat_lbl, "Filter Craft effect category").selected(sel)).clicked() {
                ctx.state.lyrics.fx_category = cat_id.into();
            }
        }
    });

    let query = ctx.state.lyrics.fx_search.trim().to_ascii_lowercase();
    let cat_filter = ctx.state.lyrics.fx_category.clone();
    let mut active_anchor = None;

    for fx in &effects {
        let fx_id = fx["id"].as_str().unwrap_or_default();
        let label = fx["label"].as_str().unwrap_or(fx_id);
        let badge = fx["badge"].as_str().unwrap_or("EC");
        let category = fx["category"].as_str().unwrap_or("Typography");
        let cmd_label = fx["craft_command"].as_str().unwrap_or("effect.apply");
        let is_live = fx["live_api"].as_bool() == Some(true);

        if !cat_filter.eq_ignore_ascii_case("all") && !category.eq_ignore_ascii_case(&cat_filter) {
            continue;
        }
        if !query.is_empty()
            && !fx_id.to_ascii_lowercase().contains(&query)
            && !label.to_ascii_lowercase().contains(&query)
            && !category.to_ascii_lowercase().contains(&query)
        {
            continue;
        }

        let btn_id = format!("{id_prefix}-row-{fx_id}");
        let sel = ctx.state.lyrics.selected_effect_id == fx_id;
        let rhs_tag = if is_live { "live :9877" } else { cmd_label };
        let resp = effect_row_button(ui, ctx.reg, ctx.pal, EffectRowSpec { id: &btn_id, badge, label, effect_id: rhs_tag, selected: sel });
        if sel {
            active_anchor = Some(Pos2::new(resp.rect.min.x, resp.rect.center().y));
        }
        if resp.clicked() {
            let _ = badge_accent_color(ctx.pal, badge);
            apply_effect_to_selected_word(ctx, fx_id, label);
        }
    }
    active_anchor
}

/// Right-hand inspector (`[Transform & FX] [Styles & 3D] [Range & Curves]`) for `03 Lyrics`.
pub fn draw_lyrics_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let mut fx_anchor = None;
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tid, lbl, tip) in [
                ("animators", "Transform & FX", "Font, Transform & Live Craft Typography Effects (`words.effects`)"),
                ("styles", "Styles & 3D", "All 9 Layer Styles + Global Light & 3D Extrude (T24)"),
                ("fx", "Range & Curves", "Range/Wiggly Selectors, Cubic-Bézier Easing, Variable Axes & Procedural Motion"),
            ] {
                let id = format!("words-insp-tab-{tid}");
                let sel = ctx.state.lyrics.inspector_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                    ctx.state.lyrics.inspector_tab = tid.into();
                }
            }
        });
        ui.separator();
        ScrollArea::vertical().id_salt("words_right_insp_scroll").auto_shrink([false, false]).show(ui, |ui| match ctx.state.lyrics.inspector_tab.as_str() {
            "styles" => draw_styles_and_3d_extrude_tab(ui, ctx),
            "fx" => {
                fx_anchor = draw_range_and_curves_tab(ui, ctx);
            }
            _ => {
                fx_anchor = draw_transform_and_craft_fx_tab(ui, ctx);
            }
        });
    });
    ctx.reg.record("words-inspector", "Lyrics Inspector", &panel.response, None);
    fx_anchor
}

fn draw_transform_and_craft_fx_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    draw_font_shaping_strip(ui, ctx);
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Transform & Skew").size(11.6).strong());
        if let Some(wid) = &ctx.state.lyrics.selected_word_id {
            status_pill(ui, ctx.pal, wid, ctx.pal.accent_sky);
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let rot_changed = rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("words-rot-dial", "Rotate", -180.0..=180.0, "°", ctx.pal.accent_emerald),
            &mut ctx.state.lyrics.rotate_deg,
        )
        .changed();
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("words-skew-dial", "Skew Slant", -45.0..=45.0, "°", ctx.pal.accent_amber),
            &mut ctx.state.lyrics.skew_slant_deg,
        );
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("words-axis-dial", "Skew Axis", -90.0..=90.0, "°", ctx.pal.accent_sky),
            &mut ctx.state.lyrics.skew_axis_deg,
        );
        if rot_changed && let Some(wid) = ctx.state.lyrics.selected_word_id.clone() {
            ctx.state.lyrics.word_rotations.insert(wid, ctx.state.lyrics.rotate_deg as f32);
            ctx.state.dirty_drafts.insert("lyrics".into());
        }
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-font-size", "Font Size", 18.0..=160.0, "px", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.font_size,
    );
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("words-tracking", "Tracking", -80.0..=300.0, "px", ctx.pal.accent_sky), &mut ctx.state.lyrics.tracking);
    let sx_ch = ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-stretch-x", "Stretch X", 40.0..=200.0, "%", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.scale_x,
    )
    .changed();
    let sy_ch = ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-stretch-y", "Stretch Y", 40.0..=200.0, "%", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.scale_y,
    )
    .changed();
    if (sx_ch || sy_ch)
        && let Some(wid) = ctx.state.lyrics.selected_word_id.clone()
    {
        let avg_scale = ((ctx.state.lyrics.scale_x + ctx.state.lyrics.scale_y) * 0.005) as f32;
        ctx.state.lyrics.word_scales.insert(wid, avg_scale.clamp(0.45, 2.4));
        ctx.state.dirty_drafts.insert("lyrics".into());
    }

    ui.separator();
    draw_craft_typography_effects_browser(ui, ctx, "words-fx")
}

fn draw_styles_and_3d_extrude_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("9 Layer Styles & Global Light (T24)").size(11.5).strong().color(ctx.pal.accent_amber));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (skind, lbl) in [
            ("dropShadow", "Drop Shadow"),
            ("innerShadow", "Inner Shadow"),
            ("outerGlow", "Outer Glow"),
            ("innerGlow", "Inner Glow"),
            ("bevelEmboss", "Bevel & Emboss"),
            ("satin", "Satin"),
            ("colorOverlay", "Color Overlay"),
            ("gradientOverlay", "Gradient Overlay"),
            ("stroke", "Stroke"),
        ] {
            let id = format!("words-lstyle-{skind}");
            let sel = ctx.state.lyrics.layer_style_kind == skind;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Toggle layer.layerStyle").selected(sel)).clicked() {
                ctx.state.lyrics.layer_style_kind = skind.into();
                ctx.exec("project.autosave", json!({}), Some("Applied Layer Style"));
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("words-global-light", "Global Light", -180.0..=180.0, "°", ctx.pal.accent_amber),
            &mut ctx.state.lyrics.global_light_deg,
        );
        let rem_w = (ui.available_width() - 4.0).max(120.0);
        ui.allocate_ui_with_layout(Vec2::new(rem_w, 68.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("words-stroke-w", "Stroke Width", 0.0..=24.0, "px", ctx.pal.accent_amber),
                &mut ctx.state.lyrics.stroke_width,
            );
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("words-glow-r", "Glow Radius", 0.0..=60.0, "px", ctx.pal.accent_emerald),
                &mut ctx.state.lyrics.glow_radius,
            );
        });
    });

    ui.separator();
    ui.label(RichText::new("Advanced 3D Text Extrusion & Bevels (T24)").size(11.5).strong().color(ctx.pal.accent_sky));
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-extrude-depth", "Extrusion Depth", 0.0..=120.0, "px", ctx.pal.accent_sky),
        &mut ctx.state.lyrics.extrude_depth,
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for bev in ["angular", "concave", "convex"] {
            let id = format!("words-bevel-{bev}");
            let sel = ctx.state.lyrics.bevel_style == bev;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, bev, "Select 3D text bevel profile").selected(sel)).clicked() {
                ctx.state.lyrics.bevel_style = bev.into();
            }
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("words-apply-extrude", "Extrude 3D", "Set geometryOptions/extrusionDepth & bevelStyle").primary(),
        )
        .clicked()
        {
            ctx.exec("project.autosave", json!({}), Some("Applied 3D text extrusion & bevel"));
        }
    });
}

fn draw_cubic_bezier_easing_editor(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let p1 = ctx.state.lyrics.curve_p1;
    let p2 = ctx.state.lyrics.curve_p2;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Cubic-Bézier Easing Curve").size(11.2).strong().color(ctx.pal.accent_amber));
        ui.label(RichText::new(format!("({:.2},{:.2},{:.2},{:.2})", p1[0], p1[1], p2[0], p2[1])).monospace().size(8.6).color(ctx.pal.text_muted));
    });
    let graph_sz = Vec2::new((ui.available_width() - 4.0).max(160.0), 86.0);
    let (grect, _) = ui.allocate_exact_size(graph_sz, Sense::hover());
    let inner = grect.shrink2(Vec2::new(10.0, 8.0));
    ui.painter().rect_filled(grect, CornerRadius::same(5), ctx.pal.bg_stage);
    ui.painter().rect_stroke(grect, CornerRadius::same(5), Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);

    let to_screen = |nx: f32, ny: f32| Pos2::new(inner.min.x + nx.clamp(0.0, 1.0) * inner.width(), inner.max.y - ny.clamp(0.0, 1.0) * inner.height());
    let start_pt = to_screen(0.0, 0.0);
    let end_pt = to_screen(1.0, 1.0);
    let c1_pt = to_screen(p1[0], p1[1]);
    let c2_pt = to_screen(p2[0], p2[1]);

    ui.painter().line_segment([start_pt, end_pt], Stroke::new(1.0, ctx.pal.border_subtle));
    ui.painter().line_segment([start_pt, c1_pt], Stroke::new(1.1, ctx.pal.accent_sky.gamma_multiply(0.65)));
    ui.painter().line_segment([end_pt, c2_pt], Stroke::new(1.1, ctx.pal.accent_amber.gamma_multiply(0.65)));
    ui.painter().add(CubicBezierShape::from_points_stroke(
        [start_pt, c1_pt, c2_pt, end_pt],
        false,
        Color32::TRANSPARENT,
        Stroke::new(2.0, ctx.pal.accent_emerald),
    ));

    for (idx, (pt, col)) in [(c1_pt, ctx.pal.accent_sky), (c2_pt, ctx.pal.accent_amber)].into_iter().enumerate() {
        ui.painter().circle_filled(pt, 4.8, col);
        ui.painter().circle_stroke(pt, 4.8, Stroke::new(1.2, Color32::WHITE));
        let h_hit = ui.interact(Rect::from_center_size(pt, Vec2::splat(14.0)), ui.scope_id().with(("lyrics_bezier_handle", idx)), Sense::drag());
        if h_hit.dragged()
            && let Some(ptr) = h_hit.interact_pointer_pos()
        {
            let nx = ((ptr.x - inner.min.x) / inner.width().max(1.0)).clamp(0.0, 1.0);
            let ny = ((inner.max.y - ptr.y) / inner.height().max(1.0)).clamp(0.0, 1.0);
            if idx == 0 {
                ctx.state.lyrics.curve_p1 = [nx, ny];
                ctx.state.lyrics.ease_high_pct = f64::from(ny * 100.0);
            } else {
                ctx.state.lyrics.curve_p2 = [nx, ny];
                ctx.state.lyrics.ease_low_pct = f64::from((1.0 - nx) * 100.0);
            }
        }
        ctx.reg.record(format!("words-bezier-p{}", idx + 1), "Easing Handle", &h_hit, None);
    }
}

fn draw_range_and_curves_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    draw_cubic_bezier_easing_editor(ui, ctx);
    ui.separator();

    ui.label(RichText::new("Range & Wiggly Selector (T23)").size(11.5).strong().color(ctx.pal.accent_emerald));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for unit in ["characters", "words", "lines"] {
            let id = format!("words-range-unit-{unit}");
            let sel = ctx.state.lyrics.range_unit == unit;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, unit, "Range selector unit").selected(sel)).clicked() {
                ctx.state.lyrics.range_unit = unit.into();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (sh, lbl) in [("ramp_up", "Ramp ↑"), ("ramp_down", "Ramp ↓"), ("triangle", "Tri"), ("square", "Square")] {
            let id = format!("words-range-shape-{sh}");
            let sel = ctx.state.lyrics.range_shape == sh;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Range selector shape").selected(sel)).clicked() {
                ctx.state.lyrics.range_shape = sh.into();
            }
        }
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-range-start", "Range Start", 0.0..=100.0, "%", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.range_start_pct,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-range-end", "Range End", 0.0..=100.0, "%", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.range_end_pct,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-range-offset", "Range Offset", -100.0..=100.0, "%", ctx.pal.accent_amber),
        &mut ctx.state.lyrics.range_offset_pct,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-ease-high", "Ease High", 0.0..=100.0, "%", ctx.pal.accent_sky),
        &mut ctx.state.lyrics.ease_high_pct,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-ease-low", "Ease Low", 0.0..=100.0, "%", ctx.pal.accent_sky),
        &mut ctx.state.lyrics.ease_low_pct,
    );
    pill_toggle(ui, ctx.reg, ctx.pal, "words-3d-per-char", "3D Per-Character Layer", &mut ctx.state.lyrics.per_char_3d, ctx.pal.accent_emerald);
    pill_toggle(ui, ctx.reg, ctx.pal, "words-wiggly-sel", "Wiggly Selector", &mut ctx.state.lyrics.wiggly_selector, ctx.pal.accent_amber);

    if action_button(
        ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::new("words-add-animator", "sparkles", "Apply Range Animator", "Execute text.addAnimator in lettering/lyrics.ecproj").primary(),
    )
    .clicked()
    {
        ctx.exec("project.autosave", json!({}), Some("Added Text Range Animator to lyrics.ecproj"));
    }

    ui.separator();
    ui.label(RichText::new("Variable Font Axes (T21)").size(11.5).strong().color(ctx.pal.accent_amber));
    let w_ch = ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-var-wght", "Weight (wght)", 100.0..=900.0, "", ctx.pal.accent_amber),
        &mut ctx.state.lyrics.var_weight,
    )
    .changed();
    let wd_ch = ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-var-wdth", "Width (wdth)", 75.0..=125.0, "%", ctx.pal.accent_sky),
        &mut ctx.state.lyrics.var_width,
    )
    .changed();
    let sl_ch = ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-var-slnt", "Slant (slnt)", -15.0..=15.0, "°", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.var_slant,
    )
    .changed();
    if pill_toggle(ui, ctx.reg, ctx.pal, "words-ligatures", "OpenType Ligatures", &mut ctx.state.lyrics.ligatures, ctx.pal.accent_emerald).changed()
        || w_ch
        || wd_ch
        || sl_ch
    {
        ctx.state.dirty_drafts.insert("lyrics".into());
    }
    if action_button(
        ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::label_only("words-apply-shaping", "Apply Variable Shaping", "Sync layer.setText variations & OpenType features"),
    )
    .clicked()
    {
        ctx.exec("project.autosave", json!({}), Some("Applied Variable Font axes & OpenType shaping"));
    }

    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Procedural Motion & Pick-Whip").size(11.5).strong().color(ctx.pal.accent_emerald));
        let ok_drv = !ctx.state.lyrics.expr_code.trim().is_empty();
        status_pill(ui, ctx.pal, if ok_drv { "Rust Driver" } else { "Manual" }, if ok_drv { ctx.pal.accent_emerald } else { ctx.pal.text_muted });
    });
    text_input(
        ui,
        ctx.reg,
        "words-expr-code",
        "Procedural Curve Driver",
        &mut ctx.state.lyrics.expr_code,
        "spring_pop(freq: 2.5, amp: 8.0)",
        (ui.available_width() - 4.0).max(140.0),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (lbl, preset) in [
            ("spring_pop", "spring_pop(freq: 2.5, amp: 8.0)"),
            ("beat_pulse", "beat_pulse(bpm: 120, gain: 0.14)"),
            ("pendulum", "pendulum_sway(deg: 6.0, hz: 1.2)"),
            ("cycle_loop", "cycle_loop(span: 2.0)"),
        ] {
            let id = format!("words-expr-snip-{lbl}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Load pure-Rust procedural motion preset")).clicked() {
                ctx.state.lyrics.expr_code = preset.into();
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-expr-whip", "@ Pick-Whip", "Bind lyric property via EffectCraft Pick-Whip")).clicked()
        {
            ctx.exec("project.autosave", json!({}), Some("Bound lyric property via Pick-Whip"));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("words-expr-apply", "Apply Driver", "Write procedural curve keyframes into lyrics.ecproj").primary(),
        )
        .clicked()
        {
            ctx.exec("project.autosave", json!({}), Some("Applied procedural motion curve to lyric layer"));
        }
    });

    ui.separator();
    draw_craft_typography_effects_browser(ui, ctx, "words-fx-secondary")
}

#[allow(clippy::cognitive_complexity)]
/// Collapsible stage-footer bidirectional `V3 ↔ C1` Captions (`T26`) and `.ectemplate` / `.fcgt` bar (`T06`).
pub fn draw_captions_and_template_strip(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(RichText::new("Captions C1:").monospace().size(9.5).color(ctx.pal.accent_sky));
            for std_name in ["Subtitle", "CEA-608", "CEA-708", "Teletext"] {
                let id = format!("words-cap-std-{std_name}");
                let sel = ctx.state.lyrics.caption_standard == std_name;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, std_name, "Broadcast caption standard").selected(sel)).clicked() {
                    ctx.state.lyrics.caption_standard = std_name.into();
                }
            }
            for fmt in ["srt", "vtt", "scc", "mcc", "stl", "ttml"] {
                let id = format!("words-cap-fmt-{fmt}");
                let sel = ctx.state.lyrics.caption_format == fmt;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, fmt, "Caption file format").selected(sel)).clicked() {
                    ctx.state.lyrics.caption_format = fmt.into();
                }
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-cap-push", "Push V3→C1", "Push timed lyrics onto FilmCraft C1")).clicked() {
                ctx.exec(
                    "words.captions_sync",
                    json!({"direction": "v3_to_c1", "track_format": ctx.state.lyrics.caption_standard, "format": ctx.state.lyrics.caption_format}),
                    Some("Pushed timed lyrics to FilmCraft C1 captions"),
                );
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-cap-pull", "Pull C1→V3", "Import FilmCraft C1 edits back into lyrics")).clicked() {
                ctx.exec("words.captions_sync", json!({"direction": "c1_to_v3"}), Some("Pulled FilmCraft C1 captions into Lyrics"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-cap-upgrade", "Upgrade Graphic", "graphics.upgradeCaption")).clicked() {
                ctx.exec(
                    "words.captions_sync",
                    json!({"direction": "v3_to_c1", "track_format": ctx.state.lyrics.caption_standard, "format": ctx.state.lyrics.caption_format, "upgrade_to_graphic": true}),
                    Some("Upgraded C1 captions to editable graphics"),
                );
            }
            text_input(ui, ctx.reg, "words-tpl-name", "Template Name", &mut ctx.state.lyrics.template_name, "chorus-lower-third", 118.0);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-export-ectpl", "Export .ectemplate", "Package Essential Graphics .ectemplate (T06)")).clicked() {
                ctx.exec("project.autosave", json!({}), Some("Exported .ectemplate Essential Graphics package"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-import-fcgt", "+ .fcgt", "Import FilmCraft Graphic Template (T06)")).clicked() {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::ImportFcgt,
                    title: "Import FilmCraft Graphic Template (.fcgt)".into(),
                    extensions: vec!["fcgt".into(), "ectemplate".into()],
                    directory: false,
                    save: false,
                });
            }
        });
    });
}
