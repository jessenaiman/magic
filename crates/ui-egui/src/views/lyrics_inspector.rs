//! `05 Lyrics` (`V3 · lettering/lyrics.ecproj`) supplementary `EffectCraft` × `FilmCraft`
//! controls (`Sheet 08` Zone C — `T06`, `T21`–`T26`):
//! - `T21`: System & Variable Font enumeration (`text.fonts`, `text.fontFeatures`), `wght`/`wdth`/`slnt`
//!   axes (`layer.setText` `variations`), and `EFFECTCRAFT_FONT_DIRS` (`+ Font Dir`).
//! - `T22`: Point vs Paragraph Box (`layer.newText`), Metrics/Optical Kerning, OpenType Ligatures,
//!   Justification (`Left`/`Center`/`Right`/`Justify`), and `LTR`/`RTL` BiDi shaping.
//! - `T23`: Full Text Animator Range Selector (`Characters`/`Words`/`Lines`, `Start`/`End`/`Offset`,
//!   `Ramp Up`/`Ramp Down`/`Triangle`/`Square`, `Ease High`/`Low`, `Per-Char 3D`, `Wiggly Selector`).
//! - `T24`: All 9 Layer Styles + Global Light (`layer.layerStyle`) and 3D Text Extrusion + Bevels.
//! - `T25`: Inline JavaScript Expression editor (`prop.setExpression`, `timeBase: "comp"`) + `@ Pick-Whip`.
//! - `T26` & `T06`: Bidirectional `V3 ↔ C1` Captions (`Subtitle`, `CEA-608`, `CEA-708`, `Teletext`
//!   in `srt`/`vtt`/`scc`/`mcc`/`stl`/`ttml`), `Upgrade to Graphic`, and `.ectemplate` / `.fcgt` workflow.

use egui::{RichText, ScrollArea, Ui};
use magic::craft_effects::LYRICS_EFFECTS;
use serde_json::json;

use crate::controls::{EffectRowSpec, ScalarSpec, badge_accent_color, effect_row_button, fill_card, pill_toggle, ring_slider_h, rotary_dial};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

/// Compact font discovery, `+ Font Dir`, `Point/Box`, `Kerning`, & `LTR/RTL` strip (`T21`–`T22`).
pub fn draw_font_shaping_strip(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Font Family & OpenType Layout (T21–T22)").size(11.5).strong().color(ctx.pal.accent_sky));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (font, short_lbl) in [("Inter", "Inter"), ("Fraunces", "Fraunces"), ("Space Grotesk", "Grotesk"), ("JetBrains Mono", "JB Mono")] {
            let id = format!("words-font-{font}");
            let sel = ctx.state.lyrics.font_preset == font;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, short_lbl, font).selected(sel)).clicked() {
                ctx.state.lyrics.font_preset = font.into();
                ctx.exec(
                    "effect.exec",
                    json!({"domain": "lyrics", "command": "layer.setText", "params": {"font": font}}),
                    Some("Updated EffectCraft lyric font"),
                );
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-scan-fonts", "refresh", "Rescan system & variable fonts (text.fonts)")).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "lyrics", "command": "text.fonts", "params": {"variable": true, "rescan": true}}),
                Some("Rescanned system & variable fonts"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-font-dir", "+ Font Dir", "Add folder to EFFECTCRAFT_FONT_DIRS (T21)")).clicked() {
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
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (bmode, lbl) in [("point", "Point"), ("box", "Box")] {
            let id = format!("words-box-{bmode}");
            let sel = ctx.state.lyrics.text_box_mode == bmode;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Point vs Paragraph Box text (T22)").selected(sel)).clicked() {
                ctx.state.lyrics.text_box_mode = bmode.into();
                let params = if bmode == "box" {
                    json!({"text": "Lyric Line", "box": [1280.0, 260.0]})
                } else {
                    json!({"text": "Lyric Line", "position": [960.0, 540.0]})
                };
                ctx.exec("effect.exec", json!({"domain": "lyrics", "command": "layer.newText", "params": params}), None);
            }
        }
        for (kern, lbl) in [("metrics", "Metrics"), ("optical", "Optical")] {
            let id = format!("words-kern-{kern}");
            let sel = ctx.state.lyrics.kerning_mode == kern;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "OpenType kerning mode").selected(sel)).clicked() {
                ctx.state.lyrics.kerning_mode = kern.into();
                ctx.exec("effect.exec", json!({"domain": "lyrics", "command": "layer.setText", "params": {"kerning": kern}}), None);
            }
        }
        for (jst, lbl) in [("left", "L"), ("center", "C"), ("right", "R"), ("justify", "J")] {
            let id = format!("words-just-{jst}");
            let sel = ctx.state.lyrics.justify == jst;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Paragraph justification").selected(sel)).clicked() {
                ctx.state.lyrics.justify = jst.into();
                ctx.exec("effect.exec", json!({"domain": "lyrics", "command": "layer.setText", "params": {"justify": jst}}), None);
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
            let dir = if ctx.state.lyrics.rtl_bidi { "rtl" } else { "ltr" };
            ctx.exec(
                "effect.exec",
                json!({"domain": "lyrics", "command": "layer.setText", "params": {"direction": dir, "ligatures": ctx.state.lyrics.ligatures}}),
                None,
            );
        }
    });
}

/// Right-hand inspector (`[Animators] [Styles & 3D] [FX & Expr]`) for `T21`–`T25`.
pub fn draw_lyrics_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tid, lbl, tip) in [
                ("animators", "Animators", "Fonts, Transform, Variable Axes & Text Range Selectors (T21-T23)"),
                ("styles", "Styles & 3D", "All 9 Layer Styles + Global Light & 3D Extrude (T24)"),
                ("fx", "FX & Expr", "Craft Typography Effects & JS Expressions + Pick-Whip (T25)"),
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
            "fx" => draw_typography_fx_and_expr_tab(ui, ctx),
            _ => draw_transform_and_range_animators_tab(ui, ctx),
        });
    });
    ctx.reg.record("words-inspector", "Lyrics Inspector", &panel.response, None);
}

fn draw_transform_and_range_animators_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_font_shaping_strip(ui, ctx);
    ui.separator();

    ui.label(RichText::new("Transform & Skew").size(12.0).strong());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        rotary_dial(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("words-rot-dial", "Rotate", -180.0..=180.0, "°", ctx.pal.accent_emerald),
            &mut ctx.state.lyrics.rotate_deg,
        );
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
    });
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-font-size", "Font Size", 18.0..=160.0, "px", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.font_size,
    );
    ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("words-tracking", "Tracking", -80.0..=300.0, "px", ctx.pal.accent_sky), &mut ctx.state.lyrics.tracking);
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-stretch-x", "Stretch X", 40.0..=200.0, "%", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.scale_x,
    );
    ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("words-stretch-y", "Stretch Y", 40.0..=200.0, "%", ctx.pal.accent_emerald),
        &mut ctx.state.lyrics.scale_y,
    );

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
        ctx.exec(
            "effect.exec",
            json!({
                "domain": "lyrics",
                "command": "layer.setText",
                "params": {
                    "font": ctx.state.lyrics.font_preset,
                    "size": ctx.state.lyrics.font_size,
                    "tracking": ctx.state.lyrics.tracking,
                    "ligatures": ctx.state.lyrics.ligatures,
                    "variations": {
                        "wght": ctx.state.lyrics.var_weight,
                        "wdth": ctx.state.lyrics.var_width,
                        "slnt": ctx.state.lyrics.var_slant,
                    }
                }
            }),
            Some("Applied Variable Font axes & OpenType shaping"),
        );
    }

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
        ctx.exec(
            "effect.exec",
            json!({
                "domain": "lyrics",
                "command": "text.addAnimator",
                "params": {
                    "basedOn": ctx.state.lyrics.range_unit,
                    "shape": ctx.state.lyrics.range_shape,
                    "start": ctx.state.lyrics.range_start_pct,
                    "end": ctx.state.lyrics.range_end_pct,
                    "offset": ctx.state.lyrics.range_offset_pct,
                    "easeHigh": ctx.state.lyrics.ease_high_pct,
                    "easeLow": ctx.state.lyrics.ease_low_pct,
                    "perCharacter3d": ctx.state.lyrics.per_char_3d,
                    "wiggly": ctx.state.lyrics.wiggly_selector,
                }
            }),
            Some("Added Text Range Animator to lyrics.ecproj"),
        );
    }
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
                ctx.exec(
                    "effect.exec",
                    json!({
                        "domain": "lyrics",
                        "command": "layer.layerStyle",
                        "params": {"style": skind, "enabled": true, "globalLight": ctx.state.lyrics.global_light_deg},
                    }),
                    Some("Applied EffectCraft Layer Style"),
                );
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
        ui.allocate_ui_with_layout(egui::Vec2::new(rem_w, 68.0), egui::Layout::top_down(egui::Align::Min), |ui| {
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
            ctx.exec(
                "effect.exec",
                json!({
                    "domain": "lyrics",
                    "command": "prop.set",
                    "params": {
                        "property": "geometryOptions/extrusionDepth",
                        "value": ctx.state.lyrics.extrude_depth,
                        "bevelStyle": ctx.state.lyrics.bevel_style,
                    }
                }),
                Some("Applied 3D text extrusion & bevel"),
            );
        }
    });
}

fn draw_typography_fx_and_expr_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("CLI Craft Typography Effects").size(11.5).strong());
    for fx in LYRICS_EFFECTS {
        let btn_id = format!("words-fx-{}", fx.id);
        let sel = ctx.state.lyrics.selected_effect_id == fx.id;
        if effect_row_button(ui, ctx.reg, ctx.pal, EffectRowSpec { id: &btn_id, badge: fx.badge, label: fx.label, effect_id: fx.craft_command, selected: sel })
            .clicked()
        {
            let _ = badge_accent_color(ctx.pal, fx.badge);
            ctx.state.lyrics.selected_effect_id = fx.id.into();
            ctx.exec("words.effect_install", json!({"name": fx.id}), Some("Applied Craft typography effect"));
        }
    }

    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Inline JS Expression (T25)").size(11.5).strong().color(ctx.pal.accent_emerald));
        let ok_expr = !ctx.state.lyrics.expr_code.trim().is_empty();
        status_pill(ui, ctx.pal, if ok_expr { "JS Ready" } else { "Disabled" }, if ok_expr { ctx.pal.accent_emerald } else { ctx.pal.text_muted });
    });
    text_input(
        ui,
        ctx.reg,
        "words-expr-code",
        "JavaScript Expression",
        &mut ctx.state.lyrics.expr_code,
        "wiggle(2.5, 8)",
        (ui.available_width() - 4.0).max(140.0),
    );
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        for (lbl, snippet) in [
            ("wiggle", "wiggle(2.5, 8)"),
            ("loopOut", "loopOut(\"cycle\")"),
            ("beatPulse", "100 + Math.sin(time * 8.0) * 14"),
        ] {
            let id = format!("words-expr-snip-{lbl}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Load expression preset")).clicked() {
                ctx.state.lyrics.expr_code = snippet.into();
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-expr-whip", "@ Pick-Whip", "prop.pickWhip")).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "lyrics", "command": "prop.pickWhip", "params": {"source": "transform.position", "target": "thisComp.layer(\"Audio Amplitude\").effect(\"Both Channels\")(\"Slider\")"}}),
                Some("Bound lyric property via Pick-Whip"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-expr-apply", "Apply JS", "prop.setExpression").primary()).clicked() {
            ctx.exec(
                "effect.exec",
                json!({
                    "domain": "lyrics",
                    "command": "prop.setExpression",
                    "params": {"property": "transform.position", "expression": ctx.state.lyrics.expr_code, "timeBase": "comp"},
                }),
                Some("Applied JS expression to lyric layer"),
            );
        }
    });
}

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
                ctx.exec(
                    "bridge.templates",
                    json!({"action": "export", "domain": "effectcraft", "name": ctx.state.lyrics.template_name}),
                    Some("Exported .ectemplate Essential Graphics package"),
                );
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
