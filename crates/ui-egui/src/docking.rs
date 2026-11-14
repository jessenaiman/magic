//! Workstation `DockArea` host dispatching docked panels into isolated [`crate::StageHost`]
//! views while re-exporting workspace types and commands from [`magic_view_api::docking`].

use craft_ui::docking::{DockArea, DockContent, DockEvent, Permissions, WindowKind};
use egui::{ImageSource, Ui, Vec2};
use magic_view_api::icons::catalog;
use magic_view_api::{ViewCtx, ViewMode};

pub use magic_view_api::docking::{
    DockFrameOutcome, DockWorkspaceState, PRESET_NAMES, PanelId, build_dock_style, dock_id, execute_dock_command, inspect_docking, is_dock_command_id,
    preset_layout,
};

use crate::StageHost;

struct MagicDockContent<'a, 'b> {
    ctx: &'a mut ViewCtx<'b>,
    host: &'a mut StageHost,
    active_view: ViewMode,
}

impl DockContent<PanelId> for MagicDockContent<'_, '_> {
    fn body(&mut self, ui: &mut Ui, panel: &PanelId) {
        match (*panel, self.active_view) {
            (PanelId::Canvas, ViewMode::Home) => self.host.home_mut().show(ui, self.ctx),
            (PanelId::Canvas | PanelId::Navigator | PanelId::Inspector | PanelId::Timeline, mode) => {
                self.host.stage_mut(mode).draw_panel(ui, self.ctx, *panel);
            }
            (PanelId::Scopes, _) => self.host.stage_mut(ViewMode::Studio).draw_panel(ui, self.ctx, PanelId::Scopes),
            (PanelId::Assets, _) => crate::shell::draw_stage_assets_panel(ui, self.ctx),
            (PanelId::History, _) => crate::modals::draw_history_panel_body(ui, self.ctx),
            (PanelId::Fonts, _) => magic_view_api::fonts_ui::draw_font_manager_body(ui, self.ctx),
        }
    }

    fn logical_label(&self, panel: &PanelId) -> String {
        self.host.stage(self.active_view).panel_title(*panel).to_owned()
    }

    fn permissions(&self, panel: &PanelId) -> Permissions {
        panel.permissions()
    }

    fn floating_size(&self, panel: &PanelId) -> Option<Vec2> {
        Some(panel.floating_size())
    }

    fn icon(&self, panel: &PanelId) -> Option<ImageSource<'static>> {
        Some(catalog().source(panel.icon_name()))
    }

    fn window_kind(&self, panel: &PanelId) -> WindowKind {
        panel.window_kind()
    }
}

pub fn show_dock_area(ui: &mut Ui, ctx: &mut ViewCtx<'_>, host: &mut StageHost, modal_open: bool) -> DockFrameOutcome {
    let active_view = ctx.state.effective_view();
    let layout = ctx.state.docking.workspace.layout.clone();
    let locked = ctx.state.docking.workspace.locked;
    let auto_collapse = ctx.state.docking.auto_collapse_iconic;
    let maximized = ctx.state.docking.maximized;
    let raise_panel = ctx.state.docking.raise_next_frame.take();
    let style = build_dock_style(ui);

    let mut content = MagicDockContent { ctx, host, active_view };
    let output = DockArea::new(dock_id())
        .locked(locked)
        .auto_collapse_iconic(auto_collapse)
        .maximized(maximized)
        .raise_panel(raise_panel.unwrap_or(PanelId::Canvas))
        .input_blocked(modal_open)
        .show_content(ui, &layout, &style, &mut content);

    if let Some(focused) = output.focused {
        ctx.state.focused_panel = focused.as_id().to_owned();
    }

    for event in output.events {
        match event {
            DockEvent::ToggleMaximize { panel } => {
                let target = ctx.state.focused_panel.as_str();
                let parsed = PanelId::parse(target).unwrap_or(PanelId::Canvas);
                ctx.state.docking.maximized = if ctx.state.docking.maximized == Some(parsed) { None } else { Some(parsed) };
                let _ = panel;
            }
            DockEvent::ToggleAutoCollapse => {
                ctx.state.docking.auto_collapse_iconic = !ctx.state.docking.auto_collapse_iconic;
            }
            DockEvent::FlyoutChanged { panel } => {
                ctx.state.docking.flyout = panel;
            }
            _ => {}
        }
    }

    if !output.actions.is_empty() && ctx.state.docking.workspace.apply_all(output.actions, |panel| panel.permissions()).is_ok() {
        ctx.state.docking.customized = true;
    }

    let mut dropped_files = Vec::new();
    for wf in output.window_files {
        for df in wf.dropped {
            dropped_files.push(df.path().to_path_buf());
        }
    }

    DockFrameOutcome { forwarded_events: output.forwarded_events, forwarded_modifiers: output.forwarded_modifiers.unwrap_or_default(), dropped_files }
}
