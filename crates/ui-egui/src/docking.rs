//! Shared workstation panel docking built on [`craft_ui::docking`] (`docs/craft-reference/craft-libs/docs/ui-docking.md`).
//!
//! Owns all dockable panels, tab groups, columns, splits, and floating windows across
//! Magic Studio with transactional `Workspace<PanelId>`, `Layout<PanelId>`, `DockArea`,
//! `DockContent<PanelId>`, `DockStyle::from_ui`, `DockCommand<PanelId>`, native desktop floating
//! windows (`native-windows` feature), and `DockArea::hit_rects` inspection (`ui.docking`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use craft_ui::docking::{DockArea, DockCommand, DockContent, DockEvent, DockStyle, FloatSpace, Layout, Node, Permissions, WindowKind, Workspace, float_space};
use egui::{Id, ImageSource, Modifiers, Ui, Vec2};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::{Value, json};

use crate::icons::catalog;
use crate::state::{ViewCtx, ViewMode};
use crate::views;

pub const PRESET_NAMES: &[&str] = &["Editing", "Storyboard", "Animation", "Color", "Typography"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PanelId {
    Canvas,
    Navigator,
    Inspector,
    Timeline,
    Assets,
    Scopes,
    Fidget,
    History,
    Fonts,
}

impl PanelId {
    pub const ALL: [Self; 9] =
        [Self::Canvas, Self::Navigator, Self::Inspector, Self::Timeline, Self::Assets, Self::Scopes, Self::Fidget, Self::History, Self::Fonts];

    pub const TOGGLEABLE: [Self; 8] = [Self::Navigator, Self::Inspector, Self::Timeline, Self::Assets, Self::Scopes, Self::Fidget, Self::History, Self::Fonts];

    pub const fn as_id(self) -> &'static str {
        match self {
            Self::Canvas => "canvas",
            Self::Navigator => "navigator",
            Self::Inspector => "inspector",
            Self::Timeline => "timeline",
            Self::Assets => "assets",
            Self::Scopes => "scopes",
            Self::Fidget => "fidget",
            Self::History => "history",
            Self::Fonts => "fonts",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let norm = raw.trim().to_ascii_lowercase();
        match norm.as_str() {
            "canvas" | "stage" | "program" | "program monitor" | "files & trunk" => Some(Self::Canvas),
            "navigator" | "scenes" | "rail" | "sidebar" | "song & stages" | "book hierarchy" | "actions & path" | "lyric moments" | "motion cues"
            | "storyboard scenes" => Some(Self::Navigator),
            "inspector" | "properties" | "finishing" | "studio finishing" | "field checklist" | "typography & fx" | "spline & lottie" => Some(Self::Inspector),
            "timeline" | "sequence" | "filmcraft sequence" | "theatre" | "theatre & curves" | "lyrics timeline" | "motion timeline" => Some(Self::Timeline),
            "assets" | "stage assets" | "shelf" => Some(Self::Assets),
            "scopes" | "scopes & meters" | "lumetri scopes" => Some(Self::Scopes),
            "fidget" | "scribbles" | "fidget & scribbles" => Some(Self::Fidget),
            "history" | "undo" | "undo & autosave" => Some(Self::History),
            "fonts" | "font manager" | "font store" => Some(Self::Fonts),
            _ => None,
        }
    }

    pub const fn menu_label(self) -> &'static str {
        match self {
            Self::Canvas => "Stage / Program Monitor",
            Self::Navigator => "Navigator & Scene Reel",
            Self::Inspector => "Inspector & Finishing",
            Self::Timeline => "Sequence & Stage Timeline",
            Self::Assets => "Stage Assets",
            Self::Scopes => "Scopes & Meters",
            Self::Fidget => "Fidget & Scribbles",
            Self::History => "Undo & AutoSave Journal",
            Self::Fonts => "Font Manager",
        }
    }

    pub const fn title(self, view: ViewMode) -> &'static str {
        match self {
            Self::Canvas => match view {
                ViewMode::Project => "Craft Files & Trunk",
                ViewMode::Storyboard => "Storyboard Pages",
                ViewMode::Animation => "Character Artboard",
                ViewMode::Design => "Kinetic Lyrics Stage",
                ViewMode::Motion => "Vector Motion Stage",
                ViewMode::Studio => "Program Monitor",
                ViewMode::Home => "Stage",
            },
            Self::Navigator => match view {
                ViewMode::Project => "Song & Stages",
                ViewMode::Storyboard => "Book Hierarchy",
                ViewMode::Animation => "Actions & Path",
                ViewMode::Design => "Lyric Moments",
                ViewMode::Motion => "Motion Cues",
                ViewMode::Studio => "Storyboard Scenes",
                ViewMode::Home => "Navigator",
            },
            Self::Inspector => match view {
                ViewMode::Project => "Interchange & Bridges",
                ViewMode::Storyboard => "Field Checklist",
                ViewMode::Animation => "Puppet · 3D · Roto",
                ViewMode::Design => "Typography & FX",
                ViewMode::Motion => "Spline & Lottie",
                ViewMode::Studio => "Studio Finishing",
                ViewMode::Home => "Inspector",
            },
            Self::Timeline => match view {
                ViewMode::Animation => "Theatre & Curves",
                ViewMode::Design => "Lyrics Timeline",
                ViewMode::Motion => "Motion Timeline",
                ViewMode::Studio => "FilmCraft Sequence",
                ViewMode::Storyboard => "Storyboard Spreads",
                ViewMode::Project | ViewMode::Home => "Song Timeline",
            },
            Self::Assets => "Stage Assets",
            Self::Scopes => "Scopes & Meters",
            Self::Fidget => "Fidget & Scribbles",
            Self::History => "Undo & AutoSave",
            Self::Fonts => "Font Manager",
        }
    }

    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Canvas => "film",
            Self::Navigator => "sidebar",
            Self::Inspector => "inspector",
            Self::Timeline => "dockBottom",
            Self::Assets => "folder",
            Self::Scopes => "scopesWave",
            Self::Fidget => "sparkles",
            Self::History => "refresh",
            Self::Fonts => "type",
        }
    }

    pub const fn permissions(self) -> Permissions {
        match self {
            Self::Canvas => Permissions { accepts_splits: true, ..Permissions::PROTECTED },
            _ => Permissions { movable: true, floatable: true, closable: true, accepts_tabs: true, accepts_splits: true },
        }
    }

    pub const fn window_kind(self) -> WindowKind {
        match self {
            Self::Canvas | Self::Scopes | Self::Timeline => WindowKind::Normal,
            _ => WindowKind::Palette,
        }
    }

    pub const fn floating_size(self) -> Vec2 {
        match self {
            Self::Canvas => Vec2::new(860.0, 520.0),
            Self::Navigator => Vec2::new(280.0, 460.0),
            Self::Inspector => Vec2::new(356.0, 520.0),
            Self::Timeline => Vec2::new(840.0, 250.0),
            Self::Assets => Vec2::new(540.0, 230.0),
            Self::Scopes => Vec2::new(430.0, 240.0),
            Self::Fidget => Vec2::new(460.0, 390.0),
            Self::History => Vec2::new(400.0, 220.0),
            Self::Fonts => Vec2::new(690.0, 480.0),
        }
    }
}

impl Serialize for PanelId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_id())
    }
}

impl<'de> Deserialize<'de> for PanelId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| de::Error::custom(format!("unknown panel `{raw}`")))
    }
}

pub fn dock_id() -> Id {
    Id::unique("magic_dock_area")
}

fn minimal_canvas_layout() -> Layout<PanelId> {
    Layout { root: Some(Node::Tabs { panels: vec![PanelId::Canvas], active: 0 }), floating: Vec::new() }
}

pub fn preset_layout(name: &str) -> Layout<PanelId> {
    let spec = match name.trim().to_ascii_lowercase().as_str() {
        "storyboard" => json!({
            "root": {
                "Split": {
                    "axis": "Horizontal",
                    "size": {"FixedFirst": 272.0},
                    "first": {"Dock": {"side": "Start", "columns": [{"groups": [{"panels": ["navigator"], "active": 0}], "width": 272.0, "iconic": false}]}},
                    "second": {
                        "Split": {
                            "axis": "Horizontal",
                            "size": {"FixedSecond": 292.0},
                            "first": {"Tabs": {"panels": ["canvas"], "active": 0}},
                            "second": {"Dock": {"side": "End", "columns": [{"groups": [{"panels": ["inspector", "assets"], "active": 0}], "width": 292.0, "iconic": false}]}}
                        }
                    }
                }
            },
            "floating": []
        }),
        "animation" => json!({
            "root": {
                "Split": {
                    "axis": "Horizontal",
                    "size": {"FixedFirst": 268.0},
                    "first": {"Dock": {"side": "Start", "columns": [{"groups": [{"panels": ["navigator"], "active": 0}], "width": 268.0, "iconic": false}]}},
                    "second": {
                        "Split": {
                            "axis": "Horizontal",
                            "size": {"FixedSecond": 320.0},
                            "first": {
                                "Split": {
                                    "axis": "Vertical",
                                    "size": {"FixedSecond": 204.0},
                                    "first": {"Tabs": {"panels": ["canvas"], "active": 0}},
                                    "second": {"Tabs": {"panels": ["timeline", "assets"], "active": 0}}
                                }
                            },
                            "second": {"Dock": {"side": "End", "columns": [{"groups": [{"panels": ["inspector", "fidget"], "active": 0}], "width": 320.0, "iconic": false}]}}
                        }
                    }
                }
            },
            "floating": []
        }),
        "color" => json!({
            "root": {
                "Split": {
                    "axis": "Horizontal",
                    "size": {"FixedFirst": 232.0},
                    "first": {"Dock": {"side": "Start", "columns": [{"groups": [{"panels": ["navigator"], "active": 0}], "width": 232.0, "iconic": false}]}},
                    "second": {
                        "Split": {
                            "axis": "Horizontal",
                            "size": {"FixedSecond": 338.0},
                            "first": {
                                "Split": {
                                    "axis": "Vertical",
                                    "size": {"FixedSecond": 208.0},
                                    "first": {
                                        "Split": {
                                            "axis": "Horizontal",
                                            "size": {"Ratio": 0.68},
                                            "first": {"Tabs": {"panels": ["canvas"], "active": 0}},
                                            "second": {"Tabs": {"panels": ["scopes"], "active": 0}}
                                        }
                                    },
                                    "second": {"Tabs": {"panels": ["timeline"], "active": 0}}
                                }
                            },
                            "second": {"Dock": {"side": "End", "columns": [{"groups": [{"panels": ["inspector"], "active": 0}], "width": 338.0, "iconic": false}]}}
                        }
                    }
                }
            },
            "floating": []
        }),
        "typography" => json!({
            "root": {
                "Split": {
                    "axis": "Horizontal",
                    "size": {"FixedFirst": 232.0},
                    "first": {"Dock": {"side": "Start", "columns": [{"groups": [{"panels": ["navigator"], "active": 0}], "width": 232.0, "iconic": false}]}},
                    "second": {
                        "Split": {
                            "axis": "Horizontal",
                            "size": {"FixedSecond": 344.0},
                            "first": {
                                "Split": {
                                    "axis": "Vertical",
                                    "size": {"FixedSecond": 196.0},
                                    "first": {"Tabs": {"panels": ["canvas"], "active": 0}},
                                    "second": {"Tabs": {"panels": ["timeline"], "active": 0}}
                                }
                            },
                            "second": {"Dock": {"side": "End", "columns": [{"groups": [{"panels": ["inspector", "fonts"], "active": 0}], "width": 344.0, "iconic": false}]}}
                        }
                    }
                }
            },
            "floating": []
        }),
        _ => json!({
            "root": {
                "Split": {
                    "axis": "Horizontal",
                    "size": {"FixedFirst": 252.0},
                    "first": {"Dock": {"side": "Start", "columns": [{"groups": [{"panels": ["navigator"], "active": 0}], "width": 252.0, "iconic": false}]}},
                    "second": {
                        "Split": {
                            "axis": "Horizontal",
                            "size": {"FixedSecond": 336.0},
                            "first": {
                                "Split": {
                                    "axis": "Vertical",
                                    "size": {"FixedSecond": 212.0},
                                    "first": {"Tabs": {"panels": ["canvas"], "active": 0}},
                                    "second": {"Tabs": {"panels": ["timeline"], "active": 0}}
                                }
                            },
                            "second": {"Dock": {"side": "End", "columns": [{"groups": [{"panels": ["inspector", "assets"], "active": 0}], "width": 336.0, "iconic": false}]}}
                        }
                    }
                }
            },
            "floating": []
        }),
    };
    serde_json::from_value(spec).unwrap_or_else(|_| minimal_canvas_layout())
}

fn workspace_from_preset(name: &str) -> Workspace<PanelId> {
    let layout = preset_layout(name);
    serde_json::from_value(json!({"layout": &layout}))
        .unwrap_or_else(|_| serde_json::from_value(json!({"layout": minimal_canvas_layout()})).unwrap_or_default())
}

fn json_panels_contain(node: &Value, target: &str) -> bool {
    match node {
        Value::Object(map) => {
            if let Some(Value::Array(panels)) = map.get("panels")
                && panels.iter().any(|p| p.as_str() == Some(target))
            {
                return true;
            }
            map.values().any(|v| json_panels_contain(v, target))
        }
        Value::Array(items) => items.iter().any(|v| json_panels_contain(v, target)),
        _ => false,
    }
}

#[derive(Clone, Debug)]
pub struct DockWorkspaceState {
    pub workspace: Workspace<PanelId>,
    pub active_name: String,
    pub saved_workspaces: BTreeMap<String, Workspace<PanelId>>,
    pub maximized: Option<PanelId>,
    pub auto_collapse_iconic: bool,
    pub flyout: Option<PanelId>,
    pub raise_next_frame: Option<PanelId>,
    pub customized: bool,
}

impl Default for DockWorkspaceState {
    fn default() -> Self {
        Self {
            workspace: workspace_from_preset("Editing"),
            active_name: "Editing".into(),
            saved_workspaces: BTreeMap::new(),
            maximized: None,
            auto_collapse_iconic: false,
            flyout: None,
            raise_next_frame: None,
            customized: false,
        }
    }
}

impl DockWorkspaceState {
    pub fn is_panel_visible(&self, panel: PanelId) -> bool {
        serde_json::to_value(&self.workspace.layout).is_ok_and(|v| json_panels_contain(&v, panel.as_id()))
    }

    pub fn hidden_panel_ids(&self) -> Vec<&'static str> {
        PanelId::ALL.into_iter().filter(|&p| !self.is_panel_visible(p)).map(PanelId::as_id).collect()
    }

    pub fn set_panel_visible(&mut self, panel: PanelId, visible: bool) {
        let _ = self.workspace.set_visible(panel, visible, Some(PanelId::Inspector), |p| p.permissions());
        if visible {
            self.raise_next_frame = Some(panel);
        } else if self.maximized == Some(panel) {
            self.maximized = None;
        }
        self.customized = true;
    }

    pub fn toggle_panel(&mut self, panel: PanelId) {
        let next = !self.is_panel_visible(panel);
        self.set_panel_visible(panel, next);
    }

    pub fn switch_workspace(&mut self, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if let Some(saved) = self.saved_workspaces.get(trimmed).cloned() {
            self.workspace = saved;
            self.active_name = trimmed.to_owned();
            self.maximized = None;
            self.customized = false;
            return Ok(());
        }
        if let Some(&canon) = PRESET_NAMES.iter().find(|p| p.eq_ignore_ascii_case(trimmed)) {
            self.workspace = workspace_from_preset(canon);
            self.active_name = canon.to_owned();
            self.maximized = None;
            self.customized = false;
            return Ok(());
        }
        Err(format!("unknown workspace `{trimmed}`"))
    }

    pub fn save_workspace(&mut self, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("workspace name cannot be empty".into());
        }
        self.saved_workspaces.insert(trimmed.to_owned(), self.workspace.clone());
        self.active_name = trimmed.to_owned();
        self.customized = false;
        Ok(())
    }

    pub fn delete_workspace(&mut self, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if PRESET_NAMES.iter().any(|p| p.eq_ignore_ascii_case(trimmed)) {
            return Err(format!("built-in workspace `{trimmed}` cannot be deleted"));
        }
        if self.saved_workspaces.remove(trimmed).is_none() {
            return Err(format!("unknown saved workspace `{trimmed}`"));
        }
        if self.active_name == trimmed {
            self.workspace = workspace_from_preset("Editing");
            self.active_name = "Editing".into();
        }
        Ok(())
    }

    pub fn reset_workspace(&mut self) {
        if let Some(saved) = self.saved_workspaces.get(&self.active_name).cloned() {
            self.workspace = saved;
        } else {
            let default_layout = preset_layout(&self.active_name);
            self.workspace.reset(&default_layout);
        }
        self.maximized = None;
        self.customized = false;
    }
}

#[derive(Default)]
pub struct DockFrameOutcome {
    pub forwarded_events: Vec<egui::Event>,
    pub forwarded_modifiers: Modifiers,
    pub dropped_files: Vec<PathBuf>,
}

struct MagicDockContent<'a, 'b> {
    ctx: &'a mut ViewCtx<'b>,
    active_view: ViewMode,
}

impl DockContent<PanelId> for MagicDockContent<'_, '_> {
    fn body(&mut self, ui: &mut Ui, panel: &PanelId) {
        match (*panel, self.active_view) {
            (PanelId::Canvas, ViewMode::Project) => views::project::draw_canvas_panel(ui, self.ctx),
            (PanelId::Canvas, ViewMode::Storyboard) => views::storyboard::draw_canvas_panel(ui, self.ctx),
            (PanelId::Canvas, ViewMode::Animation) => views::animation::draw_canvas_panel(ui, self.ctx),
            (PanelId::Canvas, ViewMode::Design) => views::lyrics::draw_canvas_panel(ui, self.ctx),
            (PanelId::Canvas, ViewMode::Motion) => views::motion::draw_canvas_panel(ui, self.ctx),
            (PanelId::Canvas, ViewMode::Studio) => views::studio::draw_canvas_panel(ui, self.ctx),
            (PanelId::Canvas, ViewMode::Home) => views::home::show(ui, self.ctx),

            (PanelId::Navigator, ViewMode::Project) => views::project::draw_navigator_panel(ui, self.ctx),
            (PanelId::Navigator, ViewMode::Storyboard) => views::storyboard::draw_navigator_panel(ui, self.ctx),
            (PanelId::Navigator, ViewMode::Animation) => views::animation::draw_navigator_panel(ui, self.ctx),
            (PanelId::Navigator, ViewMode::Design) => views::lyrics::draw_navigator_panel(ui, self.ctx),
            (PanelId::Navigator, ViewMode::Motion) => views::motion::draw_navigator_panel(ui, self.ctx),
            (PanelId::Navigator, ViewMode::Studio) => views::studio::draw_navigator_panel(ui, self.ctx),
            (PanelId::Navigator, ViewMode::Home) => views::project::draw_navigator_panel(ui, self.ctx),

            (PanelId::Inspector, ViewMode::Project) => views::project::draw_inspector_panel(ui, self.ctx),
            (PanelId::Inspector, ViewMode::Storyboard) => views::storyboard::draw_inspector_panel(ui, self.ctx),
            (PanelId::Inspector, ViewMode::Animation) => views::animation::draw_inspector_panel(ui, self.ctx),
            (PanelId::Inspector, ViewMode::Design) => views::lyrics::draw_inspector_panel(ui, self.ctx),
            (PanelId::Inspector, ViewMode::Motion) => views::motion::draw_inspector_panel(ui, self.ctx),
            (PanelId::Inspector, ViewMode::Studio) => views::studio::draw_inspector_panel(ui, self.ctx),
            (PanelId::Inspector, ViewMode::Home) => views::project::draw_inspector_panel(ui, self.ctx),

            (PanelId::Timeline, ViewMode::Project) => views::project::draw_timeline_panel(ui, self.ctx),
            (PanelId::Timeline, ViewMode::Storyboard) => views::storyboard::draw_timeline_panel(ui, self.ctx),
            (PanelId::Timeline, ViewMode::Animation) => views::animation::draw_timeline_panel(ui, self.ctx),
            (PanelId::Timeline, ViewMode::Design) => views::lyrics::draw_timeline_panel(ui, self.ctx),
            (PanelId::Timeline, ViewMode::Motion) => views::motion::draw_timeline_panel(ui, self.ctx),
            (PanelId::Timeline, ViewMode::Studio) => views::studio::draw_timeline_panel(ui, self.ctx),
            (PanelId::Timeline, ViewMode::Home) => views::project::draw_timeline_panel(ui, self.ctx),

            (PanelId::Assets, _) => crate::shell::draw_stage_assets_panel(ui, self.ctx),
            (PanelId::Scopes, _) => views::studio_finishing::draw_scopes_panel(ui, self.ctx),
            (PanelId::Fidget, _) => crate::options_modal::draw_fidget_panel_body(ui, self.ctx),
            (PanelId::History, _) => crate::modals::draw_history_panel_body(ui, self.ctx),
            (PanelId::Fonts, _) => crate::fonts_ui::draw_font_manager_body(ui, self.ctx),
        }
    }

    fn logical_label(&self, panel: &PanelId) -> String {
        panel.title(self.active_view).to_owned()
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

pub fn build_dock_style(ui: &Ui) -> DockStyle {
    let mut style = DockStyle::from_ui(ui);
    style.new_dock_zones = true;
    style.maximize_item = true;
    style.close_others_item = true;
    style.command_press_floats = true;
    style.float_hint = true;
    style.wheel_selects_tabs = true;
    style.middle_click_closes_tabs = true;
    style
}

pub fn show_dock_area(ui: &mut Ui, ctx: &mut ViewCtx<'_>, modal_open: bool) -> DockFrameOutcome {
    let active_view = ctx.state.effective_view();
    let layout = ctx.state.docking.workspace.layout.clone();
    let locked = ctx.state.docking.workspace.locked;
    let auto_collapse = ctx.state.docking.auto_collapse_iconic;
    let maximized = ctx.state.docking.maximized;
    let raise_panel = ctx.state.docking.raise_next_frame.take();
    let style = build_dock_style(ui);

    let mut content = MagicDockContent { ctx, active_view };
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

pub fn execute_dock_command(egui_ctx: &egui::Context, state: &mut crate::state::UiShellState, id: &str, params: &Value) -> Result<Value, String> {
    let canonical_id = match id {
        "window.floatPanel" => "window.panel.float",
        "window.dockPanel" => "window.panel.dock",
        "window.closePanel" => "window.panel.close",
        other => other,
    };
    let mut merged = if params.is_object() { params.clone() } else { json!({}) };
    if matches!(canonical_id, "window.panel.float" | "window.panel.dock" | "window.panel.close") && merged.get("panel").is_none() {
        let target_panel = if state.focused_panel.is_empty() { "inspector" } else { state.focused_panel.as_str() };
        merged["panel"] = json!(target_panel);
    }
    merged["command"] = json!(canonical_id);
    let cmd: DockCommand<PanelId> = serde_json::from_value(merged.clone()).map_err(|e| format!("{canonical_id}: {e}"))?;

    if cmd.is_app_owned() {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("Editing");
        match canonical_id {
            "window.workspace" => state.docking.switch_workspace(name)?,
            "window.workspace.new" => state.docking.save_workspace(name)?,
            "window.workspace.delete" => state.docking.delete_workspace(name)?,
            "window.workspace.reset" | "ui.dock.reset" => state.docking.reset_workspace(),
            other => return Err(format!("unhandled workspace command `{other}`")),
        }
        return Ok(inspect_docking(egui_ctx, state));
    }

    let mut space = float_space(egui_ctx, dock_id()).unwrap_or_else(|| FloatSpace::new(egui_ctx.content_rect()));
    for p in PanelId::ALL {
        space = space.with_panel_size(p, p.floating_size());
    }
    let closed = cmd.apply(&mut state.docking.workspace, false, &space, |panel| panel.permissions()).map_err(|e| format!("{canonical_id}: {e:?}"))?;
    state.docking.customized = true;
    if canonical_id == "window.panel.activate"
        && let Some(raw_p) = params.get("panel").and_then(Value::as_str).and_then(PanelId::parse)
    {
        state.docking.raise_next_frame = Some(raw_p);
        state.focused_panel = raw_p.as_id().to_owned();
    }
    let closed_ids: Vec<&'static str> = closed.into_iter().map(PanelId::as_id).collect();
    Ok(json!({
        "ok": true,
        "closed": closed_ids,
        "docking": inspect_docking(egui_ctx, state),
    }))
}

pub fn is_dock_command_id(id: &str) -> bool {
    DockCommand::<PanelId>::IDS.contains(&id) || matches!(id, "window.floatPanel" | "window.dockPanel" | "window.closePanel")
}

pub fn inspect_docking(egui_ctx: &egui::Context, state: &crate::state::UiShellState) -> Value {
    let hit_rects: Vec<Value> = DockArea::<PanelId>::hit_rects(egui_ctx, dock_id())
        .into_iter()
        .map(|(hit, rect)| {
            json!({
                "hit": hit,
                "rect": [rect.min.x, rect.min.y, rect.width(), rect.height()],
            })
        })
        .collect();
    let saved_names: Vec<&str> = state.docking.saved_workspaces.keys().map(String::as_str).collect();
    json!({
        "workspace": state.docking.active_name,
        "presets": PRESET_NAMES,
        "savedWorkspaces": saved_names,
        "layout": state.docking.workspace.layout,
        "locked": state.docking.workspace.locked,
        "maximized": state.docking.maximized.map(PanelId::as_id),
        "customized": state.docking.customized,
        "focused": if state.focused_panel.is_empty() { "canvas" } else { state.focused_panel.as_str() },
        "flyout": state.docking.flyout.map(PanelId::as_id),
        "autoCollapse": state.docking.auto_collapse_iconic,
        "rightToLeft": false,
        "hidden": state.docking.hidden_panel_ids(),
        "hitRects": hit_rects,
    })
}
