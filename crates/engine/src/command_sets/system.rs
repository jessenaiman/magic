//! System, live UI shell controls, ArtCraft CLI pass-throughs, agent session, diagnostics, and
//! settings command registrations.

use super::{cmd_spec, query_spec};
use crate::agent::{
    cmd_agent_config, cmd_agent_log, cmd_agent_models, cmd_agent_new_chat, cmd_agent_persist, cmd_agent_send, cmd_agent_session, cmd_agent_use,
};
use crate::commands::{CommandSpec, always};
use crate::doctor::cmd_app_doctor;
use crate::workshop::cmd_query_list;

fn requires_ui(_: &mut crate::Session, _: &serde_json::Value) -> crate::Result<serde_json::Value> {
    Err(crate::EngineError::InvalidParams("requires a live Magic Studio window (`magic --control 9876`)".into()))
}

pub fn specs() -> Vec<CommandSpec> {
    let mut out = Vec::with_capacity(40);
    out.extend(specs_ui_and_craft_exec());
    out.extend(specs_agent());
    out.extend(specs_edit_and_diagnostics());
    out.extend(crate::settings::commands::commands());
    out
}

fn specs_ui_and_craft_exec() -> Vec<CommandSpec> {
    vec![
        query_spec!("ui.inspect", "Inspect Live Window State", "{}", requires_ui),
        query_spec!("ui.state", "Live Window State", "{}", requires_ui),
        query_spec!("ui.elements", "Visible Controls", "{\"prefix\":\"optional\"}", requires_ui),
        cmd_spec!("ui.playback", "Control or Inspect Live Audio/Video Transport", [], None, "{\"action\":\"play|stop|toggle|status\"}", always, requires_ui),
        cmd_spec!("ui.click", "Click a Control", [], None, "{\"id\":\"element ID\"}", always, requires_ui),
        cmd_spec!("ui.set", "Set a Control or Switch View", [], None, "{\"id\":\"element ID\",\"value\":\"value\"}", always, requires_ui),
        cmd_spec!("ui.key", "Press a Key", [], None, "{\"key\":\"key name\"}", always, requires_ui),
        cmd_spec!("ui.type", "Type Into Focused or Named Control", [], None, "{\"text\":\"text\",\"id\":\"optional\"}", always, requires_ui),
        cmd_spec!("ui.drag", "Drag Between Two Targets", [], None, "{\"from\":{\"id\":\"a\"},\"to\":{\"id\":\"b\"}}", always, requires_ui),
        cmd_spec!("ui.scroll", "Scroll a Control or Viewport", [], None, "{\"id\":\"element ID\",\"dy\":120}", always, requires_ui),
        cmd_spec!("ui.screenshot", "Capture Window PNG", [], None, "{\"path\":\"optional /abs/out.png\"}", always, requires_ui),
        cmd_spec!("ui.resize", "Resize Window", [], None, "{\"width\":1440,\"height\":900}", always, requires_ui),
        cmd_spec!("ui.focus", "Focus Window or Control", [], None, "{\"id\":\"optional\"}", always, requires_ui),
        cmd_spec!("clipboard.import_image", "Paste Clipboard Artwork", [], None, "{\"expected_project\":\"path\"}", always, requires_ui),
        cmd_spec!("app.quit", "Guarded Window Close", [], None, "{\"force\":false}", always, requires_ui),
        cmd_spec!(
            "workspace.open",
            "Open File in Its ArtCraft Editor (--control)",
            ["File"],
            None,
            "{\"path\":\"file path\"}",
            always,
            crate::workspace::cmd_workspace_open
        ),
        cmd_spec!(
            "photo.exec",
            "Run Native PhotoCraft CLI Command",
            ["File"],
            None,
            "{\"path\":\"image or .pcraft/.psd\",\"id\":\"layer.removeBackground\",\"params\":{},\"save\":\"optional\"}",
            always,
            crate::workspace::cmd_photo_exec
        ),
        cmd_spec!(
            "effect.exec",
            "Run Native EffectCraft CLI Command",
            ["Animation"],
            None,
            "{\"path\":\".ecproj\",\"id\":\"comp.info\",\"params\":{},\"save\":true}",
            always,
            crate::workspace::cmd_effect_exec
        ),
        cmd_spec!(
            "vector.exec",
            "Run Native VectorCraft CLI Command",
            ["File"],
            None,
            "{\"path\":\"asset path\",\"mode\":\"info|convert|run\",\"out\":\"optional\"}",
            always,
            crate::workspace::cmd_vector_exec
        ),
        cmd_spec!(
            "anicraft.exec",
            "Run Headless AniCraft CLI Command (No GUI)",
            ["Animation"],
            None,
            "{\"path\":\"optional .anicraft\",\"mode\":\"exec|render_frame|render_seq\",\"id\":\"info\",\"params\":{},\"out\":\"optional\"}",
            always,
            crate::cmd::photo_vector::cmd_anicraft_exec
        ),
    ]
}

fn specs_agent() -> Vec<CommandSpec> {
    vec![
        query_spec!("agent.session", "Current Chat Session", "{}", cmd_agent_session),
        query_spec!("agent.log", "Agent Transcript Retrieval", "{\"refresh\":false}", cmd_agent_log),
        cmd_spec!("agent.send", "Send Message to App Agent", ["Agent"], None, "{\"message\":\"text\",\"new_chat\":true}", always, cmd_agent_send),
        cmd_spec!("agent.persist", "Persist Active Chat Selection", ["Agent"], None, "{}", always, cmd_agent_persist),
        query_spec!("agent.config", "Current Chat Selection", "{}", cmd_agent_config),
        cmd_spec!("agent.use", "Select and Save Chat Agent", ["Agent"], None, "{\"agent\":\"optional\",\"model\":\"optional\"}", always, cmd_agent_use),
        query_spec!("agent.models", "Available Chat Models", "{}", cmd_agent_models),
        cmd_spec!("agent.new_chat", "New Agent Chat", ["Agent"], None, "{}", always, cmd_agent_new_chat),
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
