//! UI-thread `--control` RPC dispatch (`ui.*`, `engine.*`, `window.snap`, `perf.stats`, `app.*`).

use std::sync::mpsc;

use craft_keymap::CommandRef;
use egui::{Event, ViewportCommand};
use magic_ui_egui::{ThemeMode, UiAction, ViewMode};
use serde_json::{Value, json};

use crate::{MagicDesktopApp, capture, input, keymap, lifecycle, only_keys, studio};

pub(crate) struct ControlEnvelope {
    pub(crate) method: String,
    pub(crate) params: Value,
    pub(crate) reply: mpsc::Sender<Result<Value, String>>,
}

pub(crate) struct DeferredReply {
    pub(crate) frames_left: u8,
    pub(crate) result: Value,
    pub(crate) reply: mpsc::Sender<Result<Value, String>>,
}

impl MagicDesktopApp {
    pub(crate) fn drain_control(&mut self, ctx: &egui::Context) {
        let mut incoming = Vec::new();
        if let Some(rx) = &self.control_rx {
            while let Ok(env) = rx.try_recv() {
                incoming.push(env);
            }
        }
        for env in incoming {
            self.dispatch_control_envelope(ctx, env);
        }
    }

    fn dispatch_control_envelope(&mut self, ctx: &egui::Context, env: ControlEnvelope) {
        match env.method.as_str() {
            "ui.screenshot" => {
                if let Err(err) = self.begin_screenshot(ctx, &env.params, env.reply.clone()) {
                    let _ = env.reply.send(Err(err));
                }
            }
            "ui.click" | "ui.move" | "ui.drag" | "ui.scroll" | "ui.key" | "ui.type" | "ui.text" => {
                match self.enqueue_synthetic_input(ctx, &env.method, &env.params) {
                    Ok(summary) => {
                        self.deferred_replies.push(DeferredReply { frames_left: 1, result: summary, reply: env.reply });
                        ctx.request_repaint();
                    }
                    Err(err) => {
                        let _ = env.reply.send(Err(err));
                    }
                }
            }
            _ => {
                let res = self.dispatch_immediate_control(ctx, &env.method, env.params);
                let _ = env.reply.send(res);
            }
        }
    }

    fn begin_screenshot(&mut self, ctx: &egui::Context, params: &Value, reply: mpsc::Sender<Result<Value, String>>) -> Result<(), String> {
        only_keys(params, "ui.screenshot", &["path", "panel", "id"])?;
        if self.pending_captures.len() >= 4 {
            return Err("too many concurrent screenshot requests (limit: 4)".into());
        }
        let path_str = match params.get("path") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.as_str()),
            Some(_) => return Err("path must be a string".into()),
        };
        let panel = params.get("panel").and_then(Value::as_str).map(str::to_owned);
        let widget_id = params.get("id").and_then(Value::as_str).map(str::to_owned);
        if panel.is_some() && widget_id.is_some() {
            return Err("ui.screenshot accepts either `panel` or `id`, not both".into());
        }
        let (target, is_temp) = capture::target_path(path_str)?;
        if !is_temp && let Some(roots) = &self.session.automation_roots {
            roots.check_write(&target).map_err(|e| e.to_string())?;
        }
        self.pending_captures.push(capture::PendingCapture::start(ctx, target, is_temp, panel, widget_id, reply));
        ctx.request_repaint();
        Ok(())
    }

    fn enqueue_synthetic_input(&mut self, ctx: &egui::Context, method: &str, params: &Value) -> Result<Value, String> {
        let (events, summary) = match method {
            "ui.move" => input::build_move(&self.registry, params)?,
            "ui.click" => input::build_click(&self.registry, params)?,
            "ui.drag" => input::build_drag(&self.registry, params)?,
            "ui.scroll" => input::build_scroll(&self.registry, params)?,
            "ui.key" => {
                if let Some(chord) = params.get("key").and_then(Value::as_str)
                    && let Ok(Some(cmd)) = keymap::lookup_chord(chord, ctx.egui_wants_keyboard_input())
                {
                    if let Some(act) = magic_ui_egui::menus::translate_menu_command(&cmd, &mut self.ui_state) {
                        self.handle_ui_action(ctx, act);
                    }
                    return Ok(json!({"ok": true, "handled": true, "command": cmd.id}));
                }
                input::build_key(params)?
            }
            "ui.type" | "ui.text" => {
                only_keys(params, method, &["text", "id"])?;
                let text = params.get("text").and_then(Value::as_str).ok_or("ui.type requires `text` string")?;
                if let Some(id) = params.get("id").and_then(Value::as_str) {
                    let elem = self.registry.get(id).ok_or_else(|| format!("ui.type: widget `{id}` is missing or off-screen"))?;
                    if !elem.enabled {
                        return Err(format!("ui.type: widget `{id}` is disabled"));
                    }
                    ctx.memory_mut(|m| m.request_focus(elem.egui_id));
                }
                (vec![Event::Text(text.to_owned())], json!({"ok": true, "typed": text.len()}))
            }
            other => return Err(format!("unsupported synthetic input `{other}`")),
        };
        self.synthetic_events.extend(events);
        Ok(summary)
    }

    fn dispatch_immediate_control(&mut self, ctx: &egui::Context, id: &str, params: Value) -> Result<Value, String> {
        match id {
            "engine.execute" => {
                let cmd = params["command"].as_str().or_else(|| params["id"].as_str()).ok_or("engine.execute requires `command`")?;
                if matches!(cmd, "engine.execute" | "ui.menu.invoke") {
                    return Err("nested dispatch is not supported".into());
                }
                let inner = params.get("params").cloned().unwrap_or_else(|| json!({}));
                self.dispatch_immediate_control(ctx, cmd, inner)
            }
            "ui.menu.invoke" => {
                let cmd_id = params["id"].as_str().or_else(|| params["command"].as_str()).ok_or("ui.menu.invoke requires `id`")?;
                let inner = params.get("params").cloned().unwrap_or_else(|| json!({}));
                let cref = CommandRef::with_params(cmd_id, inner);
                if let Some(act) = magic_ui_egui::menus::translate_menu_command(&cref, &mut self.ui_state) {
                    self.handle_ui_action(ctx, act);
                }
                Ok(json!({"ok": true, "invoked": cmd_id}))
            }
            "engine.commands" => {
                only_keys(&params, id, &["filter", "enabledOnly", "enabled_only"])?;
                let enabled_only = params.get("enabledOnly").or_else(|| params.get("enabled_only")).and_then(Value::as_bool).unwrap_or(false);
                let mut listed = self.execute_engine(ctx, "command.list", json!({"filter": params.get("filter").and_then(Value::as_str).unwrap_or("")}))?;
                if enabled_only && let Some(arr) = listed["commands"].as_array_mut() {
                    arr.retain(|c| c["enabled"] == true);
                    listed["count"] = json!(arr.len());
                }
                Ok(listed)
            }
            "ui.menu.list" => {
                only_keys(&params, id, &[])?;
                let session_ref = &self.session;
                Ok(magic_ui_egui::menus::serialize_menu_list(&self.ui_state, |cid| {
                    magic::find_command(cid).is_some_and(|spec| (spec.enabled)(session_ref).is_ok())
                }))
            }
            "ui.inspect" | "ui.state" => {
                only_keys(&params, id, &[])?;
                Ok(self.inspect_state(ctx))
            }
            "ui.drafts" => {
                only_keys(&params, id, &[])?;
                Ok(self.ui_state.drafts_report())
            }
            "ui.elements" => {
                only_keys(&params, id, &["prefix", "unidentified"])?;
                if params.get("unidentified") == Some(&json!(true)) {
                    return Ok(json!({"unidentified": []}));
                }
                let prefix = params.get("prefix").and_then(Value::as_str);
                Ok(json!({"elements": self.registry.list(prefix)}))
            }
            "ui.set" | "ui.set_view" => self.control_ui_set(ctx, &params),
            "ui.playback" => self.control_playback(&params),
            "ui.focus" => self.control_focus(ctx, &params),
            "ui.resize" => self.control_resize(ctx, &params),
            "window.snap" => {
                only_keys(&params, id, &[])?;
                self.handle_ui_action(ctx, UiAction::SnapWindow);
                Ok(json!({"ok": true, "snappedRight": self.ui_state.window_snapped_right}))
            }
            "ui.dialog.state" => {
                only_keys(&params, id, &[])?;
                Ok(self.dialogs.state_json())
            }
            "ui.dialog.override" => self.dialogs.apply_override(&params),
            "ui.keymap.press" => {
                only_keys(&params, id, &["chord", "typing"])?;
                let chord = params["chord"].as_str().ok_or("ui.keymap.press needs a chord")?;
                let typing = params["typing"] == true;
                match keymap::lookup_chord(chord, typing)? {
                    Some(cmd) => {
                        if let Some(act) = magic_ui_egui::menus::translate_menu_command(&cmd, &mut self.ui_state) {
                            self.handle_ui_action(ctx, act);
                        }
                        Ok(json!({"handled": true, "command": cmd.id}))
                    }
                    None => Ok(json!({"handled": false})),
                }
            }
            "ui.filmcraft" => studio::control(&params),
            "ui.effectcraft" => studio::control_effect(&params),
            "ui.craft" => studio::control_any(&params),
            "perf.stats" => {
                only_keys(&params, id, &[])?;
                let fps = if self.last_render_ms > 0.01 { 1000.0 / self.last_render_ms } else { 60.0 };
                Ok(json!({
                    "ok": true,
                    "renderer": "eframe-wgpu",
                    "fps": fps,
                    "lastRenderMs": self.last_render_ms,
                    "pixelsPerPoint": ctx.pixels_per_point(),
                }))
            }
            "app.hide" => {
                only_keys(&params, id, &[])?;
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                Ok(json!({"ok": true, "hidden": true}))
            }
            "app.quit" | "app.restart" => {
                only_keys(&params, id, &["force"])?;
                let force = params.get("force").and_then(Value::as_bool).unwrap_or(false);
                let restart = id == "app.restart";
                if !force {
                    lifecycle::verify_clean(&self.session, &self.ui_state)?;
                }
                self.lifecycle.approve_exit(restart);
                ctx.send_viewport_cmd(ViewportCommand::Close);
                Ok(json!({"ok": true, "quitting": !restart, "restarting": restart, "forced": force}))
            }
            other if other.starts_with("ui.") => Err(format!("unsupported UI control method `{other}`")),
            engine_cmd => self.execute_engine(ctx, engine_cmd, params),
        }
    }

    fn inspect_state(&self, ctx: &egui::Context) -> Value {
        let screen = ctx.input(|i| i.content_rect());
        let win_focused = ctx.input(|i| i.focused);
        let focused_egui = ctx.memory(|m| m.focused());
        let focus_info = focused_egui.and_then(|fid| {
            self.registry.find_by_egui_id(fid).map(|elem| {
                json!({
                    "automationId": elem.id,
                    "label": elem.label,
                    "rect": [elem.rect.min.x, elem.rect.min.y, elem.rect.width(), elem.rect.height()],
                })
            })
        });
        let fps = if self.last_render_ms > 0.01 { 1000.0 / self.last_render_ms } else { 60.0 };
        json!({
            "ok": true,
            "renderer": "eframe",
            "ready": true,
            "view": self.ui_state.effective_view().as_id(),
            "requestedView": self.ui_state.view.as_id(),
            "theme": self.ui_state.theme_mode.as_str(),
            "project": self.ui_state.project_path,
            "busy": self.ui_state.busy,
            "status": self.ui_state.toast_message,
            "viewport": {
                "width": screen.width(),
                "height": screen.height(),
                "pixelsPerPoint": ctx.pixels_per_point(),
            },
            "fps": fps,
            "lastRenderMs": self.last_render_ms,
            "nativeMenuBar": !self.in_window_menus,
            "windowFocused": win_focused,
            "keyboardFocus": focus_info,
            "dialog": self.dialogs.state_json(),
            "drafts": self.ui_state.drafts_report(),
            "playback": {
                "available": self.ui_state.song_audio_path().is_some(),
                "playing": self.ui_state.audio_playing,
                "currentTime": self.ui_state.audio_time_sec,
                "duration": self.ui_state.audio_duration_sec,
            },
        })
    }

    fn control_ui_set(&mut self, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
        if let Some(id) = params.get("id").and_then(Value::as_str) {
            if let Some(checked) = params.get("checked").and_then(Value::as_bool) {
                only_keys(params, "ui.set", &["id", "checked"])?;
                self.registry.queue_set_checked(id.to_owned(), checked);
                ctx.request_repaint();
                return Ok(json!({"ok": true, "id": id, "checked": checked}));
            }
            only_keys(params, "ui.set", &["id", "value"])?;
            let val_str = match params.get("value") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                _ => return Err("ui.set: `value` must be a string or number".into()),
            };
            self.registry.queue_set_value(id.to_owned(), val_str.clone());
            ctx.request_repaint();
            return Ok(json!({"ok": true, "id": id, "value": val_str}));
        }
        only_keys(params, "ui.set", &["view", "time", "theme", "focused"])?;
        if let Some(raw_theme) = params.get("theme").and_then(Value::as_str) {
            let mode = ThemeMode::parse(raw_theme).ok_or_else(|| format!("unknown theme `{raw_theme}`"))?;
            self.ui_state.theme_mode = mode;
        }
        if let Some(focused) = params.get("focused").and_then(Value::as_str) {
            self.ui_state.focused_panel = focused.to_owned();
        }
        if let Some(raw_view) = params.get("view").and_then(Value::as_str) {
            let view = ViewMode::parse(raw_view).ok_or_else(|| format!("unknown view `{raw_view}`"))?;
            if let Some(sec) = params.get("time").and_then(Value::as_f64) {
                if !sec.is_finite() || sec < 0.0 || !matches!(view, ViewMode::Design | ViewMode::Motion) {
                    return Err("Authoring time requires Lyrics (`design`) or Motion and nonnegative seconds".into());
                }
                if self.ui_state.project_path.is_none() {
                    return Err("Open a project before selecting an authoring moment".into());
                }
                self.handle_ui_action(ctx, UiAction::NavigateAuthoring { view, seconds: sec });
            } else {
                self.handle_ui_action(ctx, UiAction::SwitchView(view));
            }
        }
        ctx.request_repaint();
        Ok(json!({
            "ok": true,
            "requestedView": self.ui_state.view.as_id(),
            "theme": self.ui_state.theme_mode.as_str(),
        }))
    }

    fn control_playback(&mut self, params: &Value) -> Result<Value, String> {
        only_keys(params, "ui.playback", &["action"])?;
        let action = params.get("action").and_then(Value::as_str).unwrap_or("status");
        if !matches!(action, "play" | "stop" | "toggle" | "status") {
            return Err("ui.playback: action must be \"play\", \"stop\", \"toggle\", or \"status\"".into());
        }
        let has_song = self.ui_state.song_audio_path().is_some();
        if !has_song && action != "status" {
            return Err("No active song audio is linked in the current project".into());
        }
        match action {
            "play" => self.ui_state.audio_playing = true,
            "stop" => self.ui_state.audio_playing = false,
            "toggle" => self.ui_state.audio_playing = !self.ui_state.audio_playing,
            _ => {}
        }
        Ok(json!({
            "ok": true,
            "available": has_song,
            "playing": self.ui_state.audio_playing,
            "currentTime": self.ui_state.audio_time_sec,
            "duration": self.ui_state.audio_duration_sec,
        }))
    }

    fn control_focus(&mut self, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
        only_keys(params, "ui.focus", &["id"])?;
        let Some(id) = params.get("id").and_then(Value::as_str) else {
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            return Ok(json!({"ok": true}));
        };
        let elem = self.registry.get(id).ok_or_else(|| format!("Control `{id}` is missing or hidden"))?;
        if !elem.enabled {
            return Err(format!("Control `{id}` is disabled"));
        }
        let eid = elem.egui_id;
        ctx.memory_mut(|m| m.request_focus(eid));
        ctx.request_repaint();
        Ok(json!({"ok": true, "id": id}))
    }

    fn control_resize(&mut self, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
        only_keys(params, "ui.resize", &["width", "height"])?;
        let side = |key: &str, range: std::ops::RangeInclusive<u64>| -> Result<f32, String> {
            let value = params.get(key).and_then(Value::as_u64).filter(|v| range.contains(v));
            value.map(|v| v as f32).ok_or_else(|| format!("{key} must be an integer from {} to {}", range.start(), range.end()))
        };
        let width = side("width", 320..=7680)?;
        let height = side("height", 240..=4320)?;
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(width, height)));
        ctx.request_repaint();
        Ok(json!({"ok": true, "width": width, "height": height}))
    }
}
