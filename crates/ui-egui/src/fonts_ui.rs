//! App-wide typography picker (`egui::ComboBox`) & embedded [`craft_font_manager::FontsPanel`]
//! powered by [`craft_font_catalog::Catalog`], [`craft_font_store::FontStore`] (`SuiteStore`), and
//! [`storytold/font-preview-service`](https://github.com/storytold/font-preview-service) on-demand WOFF2 subsets.

use std::sync::{Arc, Mutex, OnceLock};

use craft_font_catalog::Catalog;
use craft_font_manager::store::{Installer, NoStore, SuiteStore};
use craft_font_manager::{FontsPanel, Options, Runner, Settings};
use egui::{Align2, ComboBox, RichText, ScrollArea, Ui, Vec2};

use crate::state::{ActiveModal, PickerTarget, UiAction, UiShellState, ViewCtx};
use crate::theme::MagicPalette;
use crate::widgets::{AutomationRegistry, BtnSpec, action_button, status_pill, text_input};

pub struct FontPickerSpec<'a> {
    pub id_prefix: &'a str,
    pub label: &'a str,
}

fn shared_fonts_panel() -> &'static Mutex<FontsPanel> {
    static PANEL: OnceLock<Mutex<FontsPanel>> = OnceLock::new();
    PANEL.get_or_init(|| {
        let catalog = Arc::clone(Catalog::global());
        catalog.scan_in_background();
        let installer: Arc<dyn Installer> = match SuiteStore::for_user(Arc::clone(&catalog)) {
            Some(store) => Arc::new(store),
            None => Arc::new(NoStore),
        };
        let index = installer.indexes().map_err(|e| e.to_string());
        let options = Options { catalog, installer, index, runner: Runner::Thread };
        Mutex::new(FontsPanel::new(options, Settings::default()))
    })
}

/// Refresh `state.available_fonts` directly from `craft_font_catalog::Catalog::global()` (direct font catalog discovery).
pub fn refresh_available_fonts(state: &mut UiShellState) {
    state.available_fonts = Catalog::global().families().into_iter().map(|f| f.to_string()).collect();
}

/// Reusable searchable `egui::ComboBox` font family selector wired to `Catalog::global()`,
/// showing style count, missing-font status, and a `Fonts…` button opening `FontsPanel`.
#[allow(clippy::too_many_arguments)]
pub fn font_family_combo(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    spec: FontPickerSpec<'_>,
    current_family: &mut String,
    available_fonts: &[String],
    search_query: &mut String,
    actions: &mut Vec<UiAction>,
) -> bool {
    let mut changed = false;
    let is_installed = Catalog::global().family(current_family).is_some();
    let style_count = Catalog::global().styles(current_family).len();

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new(spec.label).size(10.8).strong().color(pal.accent_sky));

        let combo_id = format!("{}-combo", spec.id_prefix);
        let selected_label = if current_family.trim().is_empty() { "Select Font…" } else { current_family.as_str() };
        let combo_resp = ComboBox::from_id_salt(&combo_id).selected_text(RichText::new(selected_label).size(11.0)).width(156.0).show_ui(ui, |ui| {
            ui.set_min_width(210.0);
            let filter_id = format!("{}-filter", spec.id_prefix);
            text_input(ui, reg, &filter_id, "Filter installed fonts", search_query, "Filter font families…", 198.0);
            ui.separator();
            let q = search_query.trim().to_ascii_lowercase();
            ScrollArea::vertical().id_salt((&combo_id, "scroll")).max_height(220.0).show(ui, |ui| {
                if available_fonts.is_empty() {
                    ui.label(RichText::new("Scanning Catalog::global()… Click Fonts… to browse Google Fonts & Noto.").size(10.0).color(pal.text_muted));
                }
                for fam in available_fonts {
                    if !q.is_empty() && !fam.to_ascii_lowercase().contains(&q) {
                        continue;
                    }
                    let is_sel = current_family.eq_ignore_ascii_case(fam);
                    if ui.selectable_label(is_sel, fam).clicked() && current_family != fam {
                        *current_family = fam.clone();
                        changed = true;
                    }
                }
            });
        });
        reg.record(&combo_id, spec.label, &combo_resp.response, Some(current_family.clone()));

        if is_installed {
            status_pill(ui, pal, &format!("{style_count} st"), pal.accent_emerald);
        } else {
            status_pill(ui, pal, "Uninstalled", pal.accent_amber);
        }

        let mgr_id = format!("{}-open-mgr", spec.id_prefix);
        if action_button(ui, reg, pal, BtnSpec::icon_only(&mgr_id, "type", "Open Craft Font Manager (FontsPanel: Installed + Google Fonts + Noto)")).clicked() {
            actions.push(UiAction::OpenFontManager);
        }
    });

    changed
}

/// Embedded Craft Font Manager body (`craft_font_manager::FontsPanel` + `font-preview-service` subset endpoint).
pub fn draw_font_manager_body(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    if ctx.state.available_fonts.is_empty() {
        refresh_available_fonts(ctx.state);
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        status_pill(ui, ctx.pal, &format!("{} Installed Families", ctx.state.available_fonts.len()), ctx.pal.accent_emerald);
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("font-mgr-rescan", "refresh", "Rescan Fonts", "Rescan system & Craft Font Store")).clicked() {
            Catalog::global().rescan();
            refresh_available_fonts(ctx.state);
            let now = ui.input(|i| i.time);
            ctx.state.set_toast(now, format!("Rescanned {} font families", ctx.state.available_fonts.len()));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("font-mgr-add-dir", "folder", "+ Font Folder", "Register custom font directory in Catalog::set_app_dirs"),
        )
        .clicked()
        {
            ctx.push(UiAction::PickFile {
                target: PickerTarget::ImportFontDir,
                title: "Select Custom Font Folder".into(),
                extensions: Vec::new(),
                directory: true,
                save: false,
            });
        }
    });

    ui.add_space(4.0);
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Font Preview Service (`ghcr.io/storytold/font-preview-service`)").size(11.2).strong().color(ctx.pal.accent_sky));
            ui.horizontal_wrapped(|ui| {
                text_input(
                    ui,
                    ctx.reg,
                    "font-preview-svc-url",
                    "font-preview-service base URL",
                    &mut ctx.state.font_preview_url,
                    "http://127.0.0.1:8000",
                    210.0,
                );
                text_input(
                    ui,
                    ctx.reg,
                    "font-preview-sample-txt",
                    "Sample lyric text (<= 256 codepoints)",
                    &mut ctx.state.font_preview_sample,
                    "Sample lyric text…",
                    230.0,
                );
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("font-preview-save-url", "save", "Save Endpoint", "Persist fonts.preview_service_url"))
                    .clicked()
                {
                    let url = ctx.state.font_preview_url.trim().to_owned();
                    ctx.exec("prefs.set", serde_json::json!({"key": "fonts.preview_service_url", "value": url}), Some("Saved font-preview-service endpoint"));
                }
            });
            if let Some(cp_spec) = magic_util::text::canonical_codepoint_spec(&ctx.state.font_preview_sample) {
                let base = ctx.state.font_preview_url.trim().trim_end_matches('/');
                ui.label(
                    RichText::new(format!("On-demand WOFF2 subset: {base}/v1/faces/{{sha256}}/list/u/{cp_spec}.woff2  (ready: {base}/readyz)"))
                        .monospace()
                        .size(9.2)
                        .color(ctx.pal.text_muted),
                );
            }
        });
    });

    ui.add_space(6.0);
    let mut panel_guard = shared_fonts_panel().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let avail_h = ui.available_height().max(260.0);
    ui.allocate_ui(Vec2::new(ui.available_width(), avail_h), |ui| {
        panel_guard.ui(ui);
    });
    drop(panel_guard);
    refresh_available_fonts(ctx.state);
}

/// Font Store index sources editor (`FontsPanel::sources_ui`) for `Options ▸ Fonts`.
pub fn draw_font_sources_settings(ui: &mut Ui) {
    let mut panel_guard = shared_fonts_panel().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    panel_guard.sources_ui(ui);
}

/// Modal window for `ActiveModal::FontManager` when invoked from the Home catalog.
pub fn draw_font_manager_window(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) -> bool {
    let mut keep_open = true;
    egui::Window::new("Craft Font Manager")
        .id(egui::Id::unique(("magic_modal", "FontManager")))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(true)
        .default_width(720.0)
        .default_height(560.0)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(egui_ctx, |ui| {
            draw_font_manager_body(ui, ctx);
            if ctx.state.active_modal == Some(ActiveModal::FontManager) {
                ui.add_space(4.0);
            }
        });
    keep_open
}
