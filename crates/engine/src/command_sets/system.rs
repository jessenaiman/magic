//! System, live UI shell controls, ArtCraft CLI pass-throughs, `craft-ai-integrations` settings,
//! diagnostics, and preference command registrations.

use super::{cmd_spec, query_spec};
use crate::commands::{CommandSpec, always};
use crate::craft_ai::{cmd_ai_configure, cmd_ai_consent, cmd_ai_status};
use crate::doctor::cmd_app_doctor;
use crate::workshop::cmd_query_list;

fn requires_ui(_: &mut crate::Session, _: &serde_json::Value) -> crate::Result<serde_json::Value> {
    Err(crate::EngineError::InvalidParams("requires a live Magic Studio window (`magic --control`)".into()))
}

pub fn specs() -> Vec<CommandSpec> {
    let mut out = Vec::with_capacity(48);
    out.extend(specs_ui_and_craft_exec());
    out.extend(specs_ai());
    out.extend(specs_edit_and_diagnostics());
    out.extend(crate::settings::commands::commands());
    out
}

fn specs_ui_and_craft_exec() -> Vec<CommandSpec> {
    vec![
        query_spec!("ui.inspect", "Inspect Live Window State", "{}", requires_ui),
        query_spec!("ui.state", "Live Window State", "{}", requires_ui),
        query_spec!("ui.drafts", "Unsaved UI Drafts Report", "{}", requires_ui),
        query_spec!("ui.elements", "Visible Controls", "{\"prefix\":\"optional\"}", requires_ui),
        query_spec!("ui.viewer.locate", "Locate Stage Coordinates in Window Pixels", "{\"x\":960,\"y\":540,\"stage\":\"optional\"}", requires_ui),
        query_spec!("ui.viewer.hit", "Hit-Test Window Coordinates on Active Stage", "{\"x\":640,\"y\":360}", requires_ui),
        query_spec!("ui.timeline.locate", "Locate Timeline Time/Track in Window Pixels", "{\"time\":2.5,\"track\":\"optional V3\"}", requires_ui),
        query_spec!("ui.timeline.hit", "Hit-Test Window Coordinates on Timeline", "{\"x\":500,\"y\":780}", requires_ui),
        query_spec!("perf.stats", "Live eframe/wgpu Frame & Render Telemetry", "{}", requires_ui),
        cmd_spec!("render.frame", "Render Stage Frame PNG Headless or Over --control", [], None, "{\"stage\":\"studio|lyrics|motion|animation|storyboard\",\"time\":0.0,\"path\":\"optional /abs/out.png\"}", always, requires_ui),
        cmd_spec!("ui.playback", "Control or Inspect Live Audio/Video Transport", [], None, "{\"action\":\"play|stop|toggle|status\"}", always, requires_ui),
        cmd_spec!("ui.click", "Click a Control", [], None, "{\"id\":\"element ID\"}", always, requires_ui),
        cmd_spec!("ui.set", "Set a Control, Stage Tab, Studio Workspace, or View", [], None, "{\"view\":\"optional\",\"studioWorkspace\":\"optional matrix|edit|color|fx|audio|titles|export\",\"id\":\"optional\",\"value\":\"optional\"}", always, requires_ui),
        cmd_spec!("ui.key", "Press a Key", [], None, "{\"key\":\"key name\"}", always, requires_ui),
        cmd_spec!("ui.keymap.press", "Trigger a craft-keymap Chord", [], None, "{\"chord\":\"Cmd+Z\"}", always, requires_ui),
        cmd_spec!("ui.type", "Type Into Focused or Named Control", [], None, "{\"text\":\"text\",\"id\":\"optional\"}", always, requires_ui),
        cmd_spec!("ui.drag", "Drag Between Two Targets", [], None, "{\"from\":{\"id\":\"a\"},\"to\":{\"id\":\"b\"}}", always, requires_ui),
        cmd_spec!("ui.scroll", "Scroll a Control or Viewport", [], None, "{\"id\":\"element ID\",\"dy\":120}", always, requires_ui),
        cmd_spec!("ui.screenshot", "Capture Window or Panel PNG", [], None, "{\"path\":\"optional /abs/out.png\",\"panel\":\"optional\",\"id\":\"optional\"}", always, requires_ui),
        cmd_spec!("ui.resize", "Resize Window", [], None, "{\"width\":1440,\"height\":900}", always, requires_ui),
        cmd_spec!("ui.focus", "Focus Window or Control", [], None, "{\"id\":\"optional\"}", always, requires_ui),
        cmd_spec!("clipboard.import_image", "Paste Clipboard Artwork", [], None, "{\"expected_project\":\"path\"}", always, requires_ui),
        cmd_spec!("app.quit", "Guarded Window Close", [], None, "{\"force\":false}", always, requires_ui),
        cmd_spec!(
            "workspace.open",
            "Open File in Native Desktop Editor",
            ["File"],
            None,
            "{\"path\":\"file path\"}",
            always,
            crate::workspace::cmd_workspace_open
        ),
    ]
}

fn specs_ai() -> Vec<CommandSpec> {
    vec![
        query_spec!("ai.status", "Craft AI Status, Roles & SharedStore Connections", "{}", cmd_ai_status),
        cmd_spec!(
            "ai.configure",
            "Configure AI Master Toggle, Fidget Seamless Mode & SharedStore Role Bindings",
            ["Edit"],
            None,
            "{\"enabled\":true,\"fidget_enabled\":true,\"fidget_seamless\":false,\"scribbles_enabled\":true}",
            always,
            cmd_ai_configure
        ),
        cmd_spec!(
            "ai.consent",
            "Grant or Revoke Outbound Craft AI Consent",
            ["Edit"],
            None,
            "{\"agree\":true,\"term\":\"one_hour|one_day|until_revoked\",\"revoke\":false}",
            always,
            cmd_ai_consent
        ),
    ]
}

fn specs_edit_and_diagnostics() -> Vec<CommandSpec> {
    vec![
        cmd_spec!("edit.undo", "Undo", ["Edit"], Some("Cmd+Z"), "{}", always, crate::session::undo::cmd_edit_undo),
        cmd_spec!("edit.redo", "Redo", ["Edit"], Some("Cmd+Shift+Z"), "{}", always, crate::session::undo::cmd_edit_redo),
        query_spec!("backstage.changes", "Saved-Project Change Notifications", "{}", crate::change_marker::status),
        query_spec!("backstage.events", "Recent Activity Receipts", "{\"after\":0,\"limit\":20}", crate::activity::events),
        query_spec!("app.doctor", "Doctor: ArtCraft Suite & Environment Check", "{\"updates\":true}", cmd_app_doctor),
        query_spec!("app.update", "Check for Newer Release", "{}", crate::update::cmd_app_update),
        query_spec!("app.errors", "Recorded Runtime Errors", "{\"after\":0,\"limit\":50}", crate::activity::errors),
        query_spec!("command.list", "List Engine Commands", "{\"filter\":\"optional\"}", cmd_query_list),
    ]
}
