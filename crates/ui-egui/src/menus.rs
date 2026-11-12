//! Native & in-window system menu bar built on [`craft_menu`] (`File`, `Edit`, `View`,
<<<<<<< HEAD
//! `Window`, `Help`) — deduplicated so stage-specific commands live exclusively on their
//! respective view ribbons (`Storyboard`, `Animations`, `Lyrics`, `Motion`, `Studio`).
=======
//! `Window`, `Help`) wired directly to [`craft_ui::docking`] panels and workspaces.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

use craft_keymap::{CommandRef, Keymap};
use craft_menu::{Gate, Item, ItemRole, ItemState, LayoutOptions, MenuBar, MenuBarUi, MenuRole, Row, Target, apply_keymap, apply_state, from_items, layout};
use serde_json::{Value, json};

<<<<<<< HEAD
use crate::state::{UiAction, UiShellState, ViewMode};
=======
use crate::docking::{PRESET_NAMES, PanelId, is_dock_command_id};
use crate::state::{PickerTarget, UiAction, UiShellState, ViewMode};
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
use crate::theme::ThemeMode;
use crate::widgets::AutomationRegistry;

/// Clean app-level menus only — stage actions live directly on each view's ribbon & toolbar.
pub const MENU_ORDER: &[&str] = &["File", "Edit", "View", "Window", "Help"];

fn plain_row(path: &[&str], cmd: CommandRef, label: &str) -> Row {
    let kind = Item::new(cmd.clone(), label).kind;
    Row { path: path.iter().map(|s| (*s).to_owned()).collect(), command: cmd, label: label.to_owned(), shortcut: None, enabled: true, kind }
}

<<<<<<< HEAD
=======
fn check_row(path: &[&str], cmd: CommandRef, label: &str, checked: bool) -> Row {
    let kind = Item::new(cmd.clone(), label).check(checked).kind;
    Row { path: path.iter().map(|s| (*s).to_owned()).collect(), command: cmd, label: label.to_owned(), shortcut: None, enabled: true, kind }
}

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
fn radio_row(path: &[&str], cmd: CommandRef, label: &str, selected: bool) -> Row {
    let kind = Item::new(cmd.clone(), label).radio(selected).kind;
    Row { path: path.iter().map(|s| (*s).to_owned()).collect(), command: cmd, label: label.to_owned(), shortcut: None, enabled: true, kind }
}

fn sep_row(menu: &str) -> Row {
    plain_row(&[menu], CommandRef::from("---"), "---")
}

fn build_rows(state: &UiShellState) -> Vec<Row> {
<<<<<<< HEAD
    let mut rows = Vec::with_capacity(42);
    // File
    rows.push(plain_row(&["File"], CommandRef::from("project.new"), "New Song Project…"));
    rows.push(plain_row(&["File"], CommandRef::from("project.open"), "Open Project…"));
=======
    let mut rows = Vec::with_capacity(64);
    // File
    rows.push(plain_row(&["File"], CommandRef::from("project.new"), "New Song Project…"));
    rows.push(plain_row(&["File"], CommandRef::from("project.open"), "Open Project…"));
    rows.push(plain_row(&["File"], CommandRef::from("project.archive_pick"), "Archive Project Folder (.zip)…"));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    rows.push(sep_row("File"));
    rows.push(plain_row(&["File"], CommandRef::from("file.save"), "Save Project"));
    rows.push(plain_row(&["File"], CommandRef::from("file.saveAs"), "Save Project As…"));
    rows.push(plain_row(&["File"], CommandRef::from("project.import_file"), "Import Media or Asset…"));
    rows.push(sep_row("File"));
    rows.push(plain_row(&["File"], CommandRef::from("ui.preferences"), "Options…"));
    rows.push(sep_row("File"));
    rows.push(plain_row(&["File"], CommandRef::from("file.close"), "Close Project"));
    rows.push(plain_row(&["File"], CommandRef::from("app.quit"), "Quit Magic Studio"));

    // Edit
    rows.push(plain_row(&["Edit"], CommandRef::from("edit.undo"), "Undo"));
    rows.push(plain_row(&["Edit"], CommandRef::from("edit.redo"), "Redo"));
    rows.push(sep_row("Edit"));
    rows.push(plain_row(&["Edit"], CommandRef::from("clipboard.import_image"), "Paste Clipboard Artwork"));
    rows.push(sep_row("Edit"));
    rows.push(plain_row(&["Edit", "Preferences"], CommandRef::from("ui.preferences"), "Options (Style & AI Providers)…"));

    // View
    for (view, title) in [
        (ViewMode::Home, "Home Catalog"),
        (ViewMode::Project, "Project Files & Assets"),
        (ViewMode::Storyboard, "Storyboard"),
        (ViewMode::Animation, "Animations"),
        (ViewMode::Design, "Kinetic Lyrics"),
        (ViewMode::Motion, "Action Motion"),
        (ViewMode::Studio, "FilmCraft Studio"),
    ] {
        rows.push(radio_row(&["View"], CommandRef::with_params("ui.set", json!({"view": view.as_id()})), title, state.effective_view() == view));
    }
    rows.push(sep_row("View"));
<<<<<<< HEAD
    rows.push(plain_row(&["View"], CommandRef::from("ui.toggle_ribbon"), "Command Ribbon"));
    rows.push(plain_row(&["View"], CommandRef::from("ui.toggle_assets"), "Stage Assets Shelf"));
    rows.push(plain_row(&["View"], CommandRef::from("ui.toggle_fidget"), "Fidget & Scribbles Stager"));
=======
    rows.push(check_row(&["View"], CommandRef::from("ui.toggle_ribbon"), "Stage Header Bar", !state.ribbon_collapsed));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    for (mode, label) in [(ThemeMode::Dark, "Dark Workstation"), (ThemeMode::Light, "Light Paper"), (ThemeMode::System, "Match System")] {
        rows.push(radio_row(&["View", "Theme"], CommandRef::with_params("ui.set", json!({"theme": mode.as_str()})), label, state.theme_mode == mode));
    }

<<<<<<< HEAD
    // Window (ArtCraft Suite Launchers)
=======
    // Window (craft_ui::docking Panels, Workspaces & ArtCraft Suite Launchers)
    for panel in PanelId::TOGGLEABLE {
        let visible = state.docking.is_panel_visible(panel);
        rows.push(check_row(&["Window"], CommandRef::with_params("ui.panel.toggle", json!({"panel": panel.as_id()})), panel.menu_label(), visible));
    }
    rows.push(sep_row("Window"));
    for &preset in PRESET_NAMES {
        rows.push(radio_row(
            &["Window", "Workspaces"],
            CommandRef::with_params("window.workspace", json!({"name": preset})),
            preset,
            state.docking.active_name.eq_ignore_ascii_case(preset),
        ));
    }
    for saved in state.docking.saved_workspaces.keys() {
        rows.push(radio_row(
            &["Window", "Workspaces"],
            CommandRef::with_params("window.workspace", json!({"name": saved})),
            saved,
            state.docking.active_name == *saved,
        ));
    }
    rows.push(plain_row(&["Window", "Workspaces"], CommandRef::from("window.workspace.reset"), "Reset Workspace"));
    rows.push(check_row(&["Window", "Workspaces"], CommandRef::from("window.workspace.lock"), "Lock Workspace", state.docking.workspace.locked));
    rows.push(plain_row(&["Window"], CommandRef::from("window.floatPanel"), "Undock Active Panel"));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    rows.push(plain_row(&["Window"], CommandRef::from("window.snap"), "Snap Window Left / Right"));
    rows.push(sep_row("Window"));
    for (app, title) in [
        ("photocraft", "Launch PhotoCraft"),
        ("vectorcraft", "Launch VectorCraft"),
        ("designcraft", "Launch DesignCraft"),
        ("effectcraft", "Launch EffectCraft"),
        ("filmcraft", "Launch FilmCraft"),
    ] {
        rows.push(plain_row(&["Window"], CommandRef::with_params("workspace.open", json!({"app": app})), title));
    }

    // Help
    rows.push(plain_row(&["Help"], CommandRef::from("ui.doctor"), "ArtCraft Suite Doctor…"));
    rows.push(plain_row(&["Help"], CommandRef::from("ui.copy_diagnostics"), "Copy Diagnostics"));
    rows.push(plain_row(&["Help"], CommandRef::from("ui.report_problem"), "Report a Problem…"));
    rows.push(plain_row(&["Help"], CommandRef::from("app.update"), "Check for Updates"));
    rows.push(sep_row("Help"));
    rows.push(plain_row(&["Help"], CommandRef::from("help.about"), "About Magic Studio"));
    rows
}

pub fn build_menu_bar(state: &UiShellState, keymap: &Keymap, is_enabled: impl Fn(&str) -> bool) -> MenuBar {
    let rows = build_rows(state);
    let mut bar = from_items(MENU_ORDER, &rows);
    bar.tag_menu("File", MenuRole::File);
    bar.tag_menu("Edit", MenuRole::Edit);
    bar.tag_menu("View", MenuRole::View);
    bar.tag_menu("Window", MenuRole::Window);
    bar.tag_menu("Help", MenuRole::Help);
    bar.tag_item(&CommandRef::from("help.about"), ItemRole::About);
    bar.tag_item(&CommandRef::from("app.quit"), ItemRole::Quit);
    bar.tag_submenu(MenuRole::Edit, "Preferences", ItemRole::Settings);
    bar.tag_submenu(MenuRole::View, "Theme", ItemRole::Appearance);

    apply_keymap(&mut bar, keymap);
    apply_state(&mut bar, |cmd| {
        let checked = match cmd.id.as_str() {
            "ui.toggle_ribbon" => Some(!state.ribbon_collapsed),
<<<<<<< HEAD
            "ui.toggle_assets" => Some(state.assets_drawer_open),
            "ui.toggle_fidget" => Some(state.fidget_open),
=======
            "ui.panel.toggle" => cmd.params.get("panel").and_then(Value::as_str).and_then(PanelId::parse).map(|p| state.docking.is_panel_visible(p)),
            "window.workspace" => cmd.params.get("name").and_then(Value::as_str).map(|n| state.docking.active_name.eq_ignore_ascii_case(n)),
            "window.workspace.lock" => Some(state.docking.workspace.locked),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            "ui.set" => {
                if let Some(v) = cmd.params.get("view").and_then(Value::as_str).and_then(ViewMode::parse) {
                    Some(state.effective_view() == v)
                } else {
                    cmd.params.get("theme").and_then(Value::as_str).and_then(ThemeMode::parse).map(|m| state.theme_mode == m)
                }
            }
            _ => None,
        };
        let enabled = command_enabled(state, &cmd.id, &is_enabled);
        ItemState { enabled, checked }
    });
    bar
}

fn command_enabled(state: &UiShellState, id: &str, engine_enabled: &impl Fn(&str) -> bool) -> bool {
    let has_project = state.project_path.is_some();
<<<<<<< HEAD
    match id {
        "project.new"
        | "project.open"
        | "app.quit"
        | "ui.preferences"
        | "ui.set"
        | "ui.toggle_ribbon"
        | "ui.toggle_fidget"
=======
    if is_dock_command_id(id) || id == "ui.panel.toggle" {
        return true;
    }
    match id {
        "project.new"
        | "project.open"
        | "project.archive_pick"
        | "app.quit"
        | "ui.preferences"
        | "ui.font_manager"
        | "ui.set"
        | "ui.toggle_ribbon"
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        | "ui.copy_diagnostics"
        | "ui.report_problem"
        | "window.snap"
        | "workspace.open"
        | "ui.doctor"
        | "app.update"
        | "help.about" => true,
<<<<<<< HEAD
        "ui.toggle_assets" | "file.save" | "file.saveAs" | "file.close" => has_project,
=======
        "file.save" | "file.saveAs" | "file.close" => has_project,
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        other => engine_enabled(other),
    }
}

pub fn draw_in_window_bar(ui: &mut egui::Ui, bar: &MenuBar, modal_open: bool, reg: &mut AutomationRegistry) -> Vec<CommandRef> {
    let laid_out = layout(bar, Target::current(), &LayoutOptions::new("Magic Studio")).bar;
    let gate = Gate { modal: modal_open, recording_shortcut: false, text_editing: ui.ctx().egui_wants_keyboard_input() };
    let bar_resp = ui.scope_builder(egui::UiBuilder::new(), |ui| MenuBarUi::new(&laid_out).gate(gate).show(ui));
    reg.record("menu-bar", "System Menu Bar", &bar_resp.response, None);
    bar_resp.inner
}

pub fn serialize_menu_list(state: &UiShellState, is_enabled: impl Fn(&str) -> bool) -> Value {
    let rows = build_rows(state);
    let items: Vec<Value> = rows
        .into_iter()
        .filter(|r| r.label != "---")
        .map(|r| {
            let enabled = command_enabled(state, &r.command.id, &is_enabled);
            json!({
                "path": r.path,
                "id": r.command.id,
                "label": r.label,
                "params": r.command.params,
                "enabled": enabled,
            })
        })
        .collect();
    json!({
        "menus": MENU_ORDER,
        "count": items.len(),
        "items": items,
    })
}

pub fn translate_menu_command(cmd: &CommandRef, state: &mut UiShellState) -> Option<UiAction> {
<<<<<<< HEAD
=======
    if is_dock_command_id(&cmd.id) {
        return Some(UiAction::DockCommand { id: cmd.id.clone(), params: cmd.params.clone() });
    }
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    match cmd.id.as_str() {
        "project.new" => {
            state.view = ViewMode::Home;
            state.home.show_new_form = true;
            None
        }
        "project.open" => Some(UiAction::PickFile {
<<<<<<< HEAD
            target: crate::state::PickerTarget::OpenProject,
            title: "Open Magic Studio Project".into(),
=======
            target: PickerTarget::OpenProject,
            title: "Open Magic Studio Project (.magic)".into(),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            extensions: vec!["magic".into()],
            directory: false,
            save: false,
        }),
<<<<<<< HEAD
        "file.saveAs" => Some(UiAction::PickFile {
            target: crate::state::PickerTarget::SaveProjectAs,
=======
        "project.archive_pick" => Some(UiAction::PickFile {
            target: PickerTarget::ArchiveProjectFolder,
            title: "Select Project Folder to Archive (.zip)".into(),
            extensions: Vec::new(),
            directory: true,
            save: false,
        }),
        "file.saveAs" => Some(UiAction::PickFile {
            target: PickerTarget::SaveProjectAs,
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            title: "Save Magic Studio Project As".into(),
            extensions: vec!["magic".into()],
            directory: false,
            save: true,
        }),
        "project.import_file" => Some(UiAction::PickFile {
<<<<<<< HEAD
            target: crate::state::PickerTarget::ImportTrunkFile,
=======
            target: PickerTarget::ImportTrunkFile,
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            title: "Import Media or Asset".into(),
            extensions: Vec::new(),
            directory: false,
            save: false,
        }),
        "file.close" => Some(UiAction::CloseProject),
        "app.quit" => Some(UiAction::RequestQuit),
        "ui.preferences" => Some(UiAction::OpenPreferences),
<<<<<<< HEAD
=======
        "ui.font_manager" => Some(UiAction::OpenFontManager),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        "ui.doctor" => Some(UiAction::OpenDoctor),
        "ui.copy_diagnostics" => Some(UiAction::CopyDiagnostics),
        "ui.report_problem" => Some(UiAction::OpenReportProblem),
        "help.about" => Some(UiAction::OpenAbout),
        "window.snap" => Some(UiAction::SnapWindow),
<<<<<<< HEAD
=======
        "ui.panel.toggle" => {
            if let Some(p) = cmd.params.get("panel").and_then(Value::as_str).and_then(PanelId::parse) {
                if p == PanelId::Fonts && state.project_path.is_none() {
                    return Some(UiAction::OpenFontManager);
                }
                state.docking.toggle_panel(p);
            }
            None
        }
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        "ui.toggle_ribbon" => {
            state.ribbon_collapsed = !state.ribbon_collapsed;
            None
        }
<<<<<<< HEAD
        "ui.toggle_assets" => {
            state.assets_drawer_open = !state.assets_drawer_open;
            None
        }
        "ui.toggle_fidget" => {
            state.fidget_open = !state.fidget_open;
            None
        }
=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        "ui.set" => {
            if let Some(v) = cmd.params.get("view").and_then(Value::as_str).and_then(ViewMode::parse) {
                state.view = v;
            }
            if let Some(t) = cmd.params.get("theme").and_then(Value::as_str).and_then(ThemeMode::parse) {
                state.theme_mode = t;
            }
            None
        }
        other => Some(UiAction::Execute { id: other.to_owned(), params: cmd.params.clone(), toast: None }),
    }
}
