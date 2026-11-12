//! Standard ArtCraft MCP tool router for Magic Studio (`crates/mcp/src/server.rs`).
//!
//! Exposes Magic Studio's own command registry and live `egui` UI inspection/input tools
//! (`command_list`, `command_run`, `command_batch`, `doc_inspect`, `render_preview`, `open_file`,
//! `save_file`, `inspect_ui`, `ui_elements`, `ui_click`, `ui_drag`, `ui_scroll`, `ui_key`,
//! `ui_type`, `screenshot`, `ui_control`) without proxying sibling ArtCraft applications.

use crate::{Backend, interpolate_batch_params, is_live_shell_method};
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{CallToolResult, ContentBlock as Content};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Server {
    backend: Arc<Mutex<Box<dyn Backend + Send>>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListParams {
    filter: Option<String>,
    enabled_only: Option<bool>,
    compact: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RunParams {
    /// Engine command ID (`command` or `id`).
    #[serde(alias = "id")]
    command: String,
    params: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BatchParams {
    steps: Vec<RunParams>,
    stop_on_error: Option<bool>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DocInspectParams {
    /// `"all"` (default), `"project"`, `"storyboard"`, or `"timeline"`.
    target: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PreviewParams {
    /// `"studio"` (default), `"lyrics"`, `"motion"`, `"animation"`, or `"storyboard"`.
    stage: Option<String>,
    time: Option<f64>,
    page: Option<usize>,
    path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FilePathParams {
    path: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ElementsParams {
    prefix: Option<String>,
    unidentified: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ModifiersParams {
    shift: Option<bool>,
    alt: Option<bool>,
    command: Option<bool>,
    ctrl: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ClickParams {
    id: Option<String>,
    fx: Option<f64>,
    fy: Option<f64>,
    x: Option<f64>,
    y: Option<f64>,
    button: Option<String>,
    count: Option<u32>,
    modifiers: Option<ModifiersParams>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PointParams {
    id: Option<String>,
    fx: Option<f64>,
    fy: Option<f64>,
    x: Option<f64>,
    y: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DragParams {
    from: PointParams,
    to: PointParams,
    button: Option<String>,
    steps: Option<u32>,
    modifiers: Option<ModifiersParams>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ScrollParams {
    id: Option<String>,
    fx: Option<f64>,
    fy: Option<f64>,
    x: Option<f64>,
    y: Option<f64>,
    dx: Option<f64>,
    dy: Option<f64>,
    unit: Option<String>,
    modifiers: Option<ModifiersParams>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct KeyParams {
    key: String,
    modifiers: Option<ModifiersParams>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TypeParams {
    text: String,
    id: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ShotParams {
    path: Option<String>,
    panel: Option<String>,
    id: Option<String>,
    window: Option<bool>,
    page: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ControlParams {
    method: String,
    params: Option<Value>,
}

fn compact(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k, compact(v))).collect()),
        other => other,
    }
}

fn output(result: Result<Value, String>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult::success(vec![Content::text(value.to_string())]),
        Err(error) => CallToolResult::error(vec![Content::text(error)]),
    }
}

impl Server {
    pub fn new(backend: impl Backend + Send + 'static) -> Self {
        Self { backend: Arc::new(Mutex::new(Box::new(backend))), tool_router: Self::tool_router() }
    }

    pub fn serve(&mut self) -> Result<(), String> {
        tokio::runtime::Runtime::new().map_err(|e| e.to_string())?.block_on(async {
            let service = ServiceExt::serve(self.clone(), rmcp::transport::stdio()).await.map_err(|e| e.to_string())?;
            service.waiting().await.map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    async fn run(&self, id: &str, params: Value) -> Result<Value, String> {
        if !params.is_object() {
            return Err("params must be an object".into());
        }
        let backend = self.backend.clone();
        let id = id.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut backend = backend.lock().map_err(|e| e.to_string())?;
            if is_live_shell_method(&id) && !backend.is_live() {
                return Err("UI tools require a live `magic --control` window (`magic-cli mcp --connect 127.0.0.1:9875`)".into());
            }
            backend.execute(&id, params)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn exec_list(&self, p: ListParams) -> CallToolResult {
        output(self.run("command.list", json!({"filter": p.filter.unwrap_or_default()})).await.map(|v| {
            let commands = v.get("commands").unwrap_or(&v).as_array().cloned().unwrap_or_default();
            let filtered: Vec<Value> = commands
                .into_iter()
                .filter(|c| !p.enabled_only.unwrap_or(false) || c["enabled"] == true)
                .map(|c| if p.compact.unwrap_or(false) { json!({"id": c["id"], "label": c["label"]}) } else { c })
                .collect();
            json!(filtered)
        }))
    }

    async fn exec_batch(&self, p: BatchParams) -> CallToolResult {
        let mut results = Vec::new();
        let mut prior_values = Vec::new();
        let mut completed = 0;
        let mut failed = 0;
        for step in p.steps {
            let cmd = step.command.trim();
            if cmd.is_empty() {
                failed += 1;
                results.push(json!({"ok": false, "error": "empty command ID"}));
                if p.stop_on_error.unwrap_or(true) {
                    break;
                }
                continue;
            }
            let raw_params = step.params.unwrap_or_else(|| json!({}));
            let resolved_params = interpolate_batch_params(&raw_params, &prior_values);
            match self.run(cmd, resolved_params).await {
                Ok(value) => {
                    completed += 1;
                    prior_values.push(value.clone());
                    results.push(json!({"ok": true, "result": value}));
                }
                Err(error) => {
                    failed += 1;
                    prior_values.push(Value::Null);
                    results.push(json!({"ok": false, "error": error}));
                    if p.stop_on_error.unwrap_or(true) {
                        break;
                    }
                }
            }
        }
        let value = json!({"completed": completed, "failed": failed, "results": results});
        if failed > 0 { CallToolResult::error(vec![Content::text(value.to_string())]) } else { output(Ok(value)) }
    }

    async fn exec_doc_inspect(&self, target: Option<&str>) -> CallToolResult {
        let res = match target.unwrap_or("all") {
            "project" => self.run("workshop.status", json!({})).await,
            "storyboard" => self.run("storyboard.check", json!({})).await,
            "timeline" | "sequence" => self.run("video.timeline", json!({})).await,
            "all" => match self.run("workshop.status", json!({})).await {
                Ok(project) => Ok(json!({
                    "project": project,
                    "storyboard": self.run("storyboard.check", json!({})).await.unwrap_or(Value::Null),
                    "timeline": self.run("video.timeline", json!({})).await.unwrap_or(Value::Null),
                })),
                Err(e) => Err(e),
            },
            other => Err(format!("unknown target `{other}` (expected all | project | storyboard | timeline)")),
        };
        output(res)
    }
}

#[tool_router]
impl Server {
    #[tool(description = "List Magic Studio engine command IDs, labels and parameter schemas (`{filter?, enabled_only?, compact?}`).")]
    async fn list_commands(&self, Parameters(p): Parameters<ListParams>) -> CallToolResult {
        self.exec_list(p).await
    }

    #[tool(description = "ArtCraft standard alias for `list_commands` (`{filter?, enabled_only?, compact?}`).")]
    async fn command_list(&self, Parameters(p): Parameters<ListParams>) -> CallToolResult {
        self.exec_list(p).await
    }

    #[tool(description = "Run one Magic Studio engine command (`{command, params?}`) in the selected backend.")]
    async fn run_command(&self, Parameters(p): Parameters<RunParams>) -> CallToolResult {
        let cmd = p.command.trim();
        if cmd.is_empty() {
            return output(Err("provide a non-empty `command` ID".into()));
        }
        output(self.run(cmd, p.params.unwrap_or_else(|| json!({}))).await)
    }

    #[tool(description = "ArtCraft standard alias for `run_command` (`{id|command, params?}`).")]
    async fn command_run(&self, Parameters(p): Parameters<RunParams>) -> CallToolResult {
        let cmd = p.command.trim();
        if cmd.is_empty() {
            return output(Err("provide a non-empty `id` or `command`".into()));
        }
        output(self.run(cmd, p.params.unwrap_or_else(|| json!({}))).await)
    }

    #[tool(description = "Run multiple Magic Studio engine commands sequentially with `$N.field` / `$last.field` interpolation.")]
    async fn run_batch(&self, Parameters(p): Parameters<BatchParams>) -> CallToolResult {
        self.exec_batch(p).await
    }

    #[tool(description = "ArtCraft standard alias for `run_batch` (`{steps: [{id|command, params?}], stop_on_error?}`).")]
    async fn command_batch(&self, Parameters(p): Parameters<BatchParams>) -> CallToolResult {
        self.exec_batch(p).await
    }

    #[tool(description = "Inspect the open `.magic` project, storyboard, or timeline (`all|project|storyboard|timeline`).")]
    async fn inspect_document(&self, Parameters(p): Parameters<DocInspectParams>) -> CallToolResult {
        self.exec_doc_inspect(p.target.as_deref()).await
    }

    #[tool(description = "ArtCraft standard alias for `inspect_document`.")]
    async fn doc_inspect(&self, Parameters(p): Parameters<DocInspectParams>) -> CallToolResult {
        self.exec_doc_inspect(p.target.as_deref()).await
    }

    #[tool(description = "Extract or render a cached stage preview PNG (`studio`, `lyrics`, `motion`, `animation`, or `storyboard`).")]
    async fn render_preview(&self, Parameters(p): Parameters<PreviewParams>) -> CallToolResult {
        let sec = p.time.unwrap_or(0.0);
        let res = match p.stage.as_deref().unwrap_or("studio") {
            "storyboard" => self.run("storyboard.thumbnail", json!({"page": p.page.unwrap_or(0), "scale": 1.0})).await,
            "lyrics" | "design" => self.run("words.preview_frame", json!({"time": sec})).await,
            "motion" => self.run("motion.preview_frame", json!({"time": sec})).await,
            "animation" | "animations" => self.run("character.frame", json!({"time": sec})).await,
            _ => self.run("video.frame", compact(json!({"seconds": sec, "path": p.path}))).await,
        };
        output(res)
    }

    #[tool(description = "Open a `.magic` project file (`file.open`).")]
    async fn open_file(&self, Parameters(p): Parameters<FilePathParams>) -> CallToolResult {
        let Some(path) = p.path.filter(|s| !s.trim().is_empty()) else {
            return output(Err("open_file requires `path`".into()));
        };
        output(self.run("file.open", json!({"path": path})).await)
    }

    #[tool(description = "Save the open `.magic` project (`file.save`), or save to `path` (`file.saveAs`).")]
    async fn save_file(&self, Parameters(p): Parameters<FilePathParams>) -> CallToolResult {
        match p.path.filter(|s| !s.trim().is_empty()) {
            Some(path) => output(self.run("file.saveAs", json!({"path": path})).await),
            None => output(self.run("file.save", json!({})).await),
        }
    }

    #[tool(description = "Inspect the live eframe/egui desktop UI state. Requires `--connect` to a running `magic --control` window.")]
    async fn inspect_ui(&self) -> CallToolResult {
        output(self.run("ui.inspect", json!({})).await)
    }

    #[tool(description = "List on-screen egui widget automation IDs, labels, enabled state, and logical-point bounds. Requires `--connect`.")]
    async fn ui_elements(&self, Parameters(p): Parameters<ElementsParams>) -> CallToolResult {
        output(self.run("ui.elements", compact(json!({"prefix": p.prefix, "unidentified": p.unidentified}))).await)
    }

    #[tool(description = "Click a visible enabled widget by automation ID (`{id, fx?, fy?}`) or logical window coordinates `{x, y}`. Requires `--connect`.")]
    async fn ui_click(&self, Parameters(p): Parameters<ClickParams>) -> CallToolResult {
        if p.count.is_some_and(|c| c > 10) {
            return output(Err("ui.click: count must be between 0 and 10".into()));
        }
        output(self.run("ui.click", compact(json!(p))).await)
    }

    #[tool(description = "Drag between `from` and `to` (`{id, fx?, fy?}` or `{x, y}`). Requires `--connect`.")]
    async fn ui_drag(&self, Parameters(p): Parameters<DragParams>) -> CallToolResult {
        if p.steps.is_some_and(|s| s > 1024) {
            return output(Err("ui.drag: steps must be between 0 and 1024".into()));
        }
        let params = json!({
            "from": compact(json!(p.from)),
            "to": compact(json!(p.to)),
            "button": p.button,
            "steps": p.steps,
            "modifiers": p.modifiers,
        });
        output(self.run("ui.drag", compact(params)).await)
    }

    #[tool(description = "Scroll a widget or logical point by `dx`, `dy`. Requires `--connect`.")]
    async fn ui_scroll(&self, Parameters(p): Parameters<ScrollParams>) -> CallToolResult {
        output(self.run("ui.scroll", compact(json!(p))).await)
    }

    #[tool(description = "Send a key or chord (`Enter`, `Tab`, `Escape`, `ArrowDown`, `Cmd+S`, `Physical+KeyZ`) through `craft-keymap`. Requires `--connect`.")]
    async fn ui_key(&self, Parameters(p): Parameters<KeyParams>) -> CallToolResult {
        output(self.run("ui.key", compact(json!(p))).await)
    }

    #[tool(description = "Type text into the focused `egui` text field, or into `id`. Requires `--connect`.")]
    async fn ui_type(&self, Parameters(p): Parameters<TypeParams>) -> CallToolResult {
        output(self.run("ui.type", compact(json!(p))).await)
    }

    #[tool(
        description = "Capture a PNG (`{window: true, path?, panel?, id?}` for the live eframe window, or `{window: false, page?}` for a storyboard page preview)."
    )]
    async fn screenshot(&self, Parameters(p): Parameters<ShotParams>) -> CallToolResult {
        if p.window == Some(false) {
            return output(self.run("storyboard.thumbnail", json!({"page": p.page.unwrap_or(0), "scale": 1.0})).await);
        }
        output(self.run("ui.screenshot", compact(json!({"path": p.path, "panel": p.panel, "id": p.id}))).await)
    }

    #[tool(
        description = "Call a live Magic shell control method (`document.inspect`, `ui.set`, `ui.viewer.locate`, `ui.viewer.hit`, `ui.timeline.locate`, `ui.timeline.hit`, `ui.menu.list`, `ui.menu.invoke`, `ui.playback`, `ui.focus`, `render.frame`, `perf.stats`, `app.quit`). Requires `--connect`."
    )]
    async fn ui_control(&self, Parameters(p): Parameters<ControlParams>) -> CallToolResult {
        if !is_live_shell_method(&p.method) {
            return output(Err("Use run_command / command_run for engine commands".into()));
        }
        output(self.run(&p.method, p.params.unwrap_or_else(|| json!({}))).await)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Server {
    fn get_info(&self) -> rmcp::model::ServerConfig {
        rmcp::model::ServerConfig::new(rmcp::model::ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new("magic", env!("CARGO_PKG_VERSION")))
            .with_instructions(
<<<<<<< HEAD
                "Magic Studio is a pure-Rust eframe/egui 0.37 project, storyboard, puppet-rig, and lyric-timing coordinator built on storytold/craft-libs. Start `magic --control 9875` and connect with `magic-cli mcp --connect 127.0.0.1:9875` to drive the live Magic window, or run `magic-cli mcp` headlessly. Connect directly to each sibling ArtCraft app's own `<app>-cli mcp` server when editing in FilmCraft, EffectCraft, DeckCraft, DesignCraft, PhotoCraft, or VectorCraft."
=======
                "Magic Studio is an eframe/egui project, storyboard, puppet-rig, and lyric-timing coordinator built on storytold/craft-libs. Start `magic --control 9875` and connect with `magic-cli mcp --connect 127.0.0.1:9875` to drive the live Magic window, or run `magic-cli mcp` headlessly. Connect directly to each sibling ArtCraft app's own `<app>-cli mcp` server when editing in DesignCraft, PhotoCraft, VectorCraft, EffectCraft, or FilmCraft."
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                    .to_string(),
            )
    }
}
