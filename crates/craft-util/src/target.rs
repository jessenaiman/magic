//! Single-responsibility catalog of ArtCraft applications and their upstream `--control`, CLI,
//! and MCP contracts.
//!
//! Upstream references:
//! - FilmCraft (`:9876`, `FILMCRAFT_CONTROL_TOKEN`): <https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md>
//! - EffectCraft (`:9877`, `mcp --autosave`): <https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md>
//! - DesignCraft (`:7979`): <https://github.com/storytold/designcraft/blob/main/docs/control-protocol.md>
//! - PDFCraft (`:7985`): <https://github.com/storytold/pdfcraft>
//! - DeckCraft (`:7990`): <https://github.com/storytold/deckcraft>
//! - PhotoCraft (`:9879`): <https://github.com/storytold/photocraft/blob/main/docs/control-protocol.md>
//! - VectorCraft (`:9880`): <https://github.com/storytold/vectorcraft/blob/main/docs/control-protocol.md>
//! - AniCraft (early alpha, CLI-only in Magic): <https://github.com/storytold/AniCraft>
//! - TextureCraft (`:9882`): <https://github.com/storytold/texturecraft/blob/main/AGENTS.md>

use std::path::Path;

use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAuth {
    None,
    InlineToken,
    FirstFrameAuth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlWire {
    MethodParams,
    /// AniCraft `anicraft-cli serve` JSON-lines protocol: `{"id": 1, "cmd": "...", "params": {...}}`.
    CmdParams,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CraftTarget {
    Film,
    Photo,
    Effect,
    Vector,
    Design,
    Pdf,
    Deck,
    Ani,
    Texture,
}

impl CraftTarget {
    pub const ALL: [CraftTarget; 9] = [
        CraftTarget::Film,
        CraftTarget::Photo,
        CraftTarget::Effect,
        CraftTarget::Vector,
        CraftTarget::Design,
        CraftTarget::Pdf,
        CraftTarget::Deck,
        CraftTarget::Ani,
        CraftTarget::Texture,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Film => "filmcraft",
            Self::Photo => "photocraft",
            Self::Effect => "effectcraft",
            Self::Vector => "vectorcraft",
            Self::Design => "designcraft",
            Self::Pdf => "pdfcraft",
            Self::Deck => "deckcraft",
            Self::Ani => "anicraft",
            Self::Texture => "texturecraft",
        }
    }

    pub const fn app_bin(self) -> &'static str {
        self.name()
    }

    /// Companion `<product>-cli` executable name verified from `storytold/*` repositories.
    pub const fn cli_name(self) -> &'static str {
        match self {
            Self::Film => "filmcraft-cli",
            Self::Photo => "photocraft-cli",
            Self::Effect => "effectcraft-cli",
            Self::Vector => "vectorcraft-cli",
            Self::Design => "designcraft-cli",
            Self::Pdf => "pdfcraft-cli",
            Self::Deck => "deckcraft-cli",
            Self::Ani => "anicraft-cli",
            Self::Texture => "texturecraft-cli",
        }
    }

    pub const fn allows_live_ui(self) -> bool {
        !matches!(self, Self::Ani)
    }

    pub fn from_name(raw: &str) -> Option<Self> {
        let key = raw.trim().to_ascii_lowercase();
        let base = key.strip_suffix("-cli").unwrap_or(&key);
        match base {
            "pdf" => Some(Self::Pdf),
            "deck" | "pptx" => Some(Self::Deck),
            "design" => Some(Self::Design),
            _ => Self::ALL.into_iter().find(|t| t.name() == base),
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        if let Ok(bytes) = std::fs::read(path)
            && let Some(kind) = craft_doc::sniff(&bytes)
        {
            match kind {
                craft_doc::Kind::PhotoCraft => return Some(Self::Photo),
                craft_doc::Kind::VectorCraft => return Some(Self::Vector),
                craft_doc::Kind::EffectCraft => return Some(Self::Effect),
                _ => {}
            }
        }
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "fcproj" | "fcgt" => Some(Self::Film),
            "pcraft" | "psd" => Some(Self::Photo),
            "ecproj" | "ectemplate" => Some(Self::Effect),
            "vectorcraft" | "vcraft" | "drawcraft" | "svg" => Some(Self::Vector),
            "designcraft" | "idml" => Some(Self::Design),
            "pdf" => Some(Self::Pdf),
            "deckcraft" | "pptx" => Some(Self::Deck),
            "anicraft" | "ani" | "fla" | "xfl" => Some(Self::Ani),
            "tcraft" => Some(Self::Texture),
            _ => None,
        }
    }

    pub const fn default_port(self) -> u16 {
        match self {
            Self::Film => 9876,
            Self::Effect => 9877,
            Self::Design => 7979,
            Self::Pdf => 7985,
            Self::Deck => 7990,
            Self::Photo => 9879,
            Self::Vector => 9880,
            Self::Ani => 9881,
            Self::Texture => 9882,
        }
    }

    pub const fn port_env(self) -> &'static str {
        match self {
            Self::Film => "FILMCRAFT_CONTROL_PORT",
            Self::Photo => "PHOTOCRAFT_CONTROL_PORT",
            Self::Effect => "EFFECTCRAFT_CONTROL_PORT",
            Self::Vector => "VECTORCRAFT_CONTROL_PORT",
            Self::Design => "DESIGNCRAFT_CONTROL_PORT",
            Self::Pdf => "PDFCRAFT_CONTROL_PORT",
            Self::Deck => "DECKCRAFT_CONTROL_PORT",
            Self::Ani => "ANICRAFT_CONTROL_PORT",
            Self::Texture => "TEXTURECRAFT_CONTROL_PORT",
        }
    }

    pub const fn token_env(self) -> &'static str {
        match self {
            Self::Film => "FILMCRAFT_CONTROL_TOKEN",
            Self::Photo => "PHOTOCRAFT_CONTROL_TOKEN",
            Self::Effect => "EFFECTCRAFT_CONTROL_TOKEN",
            Self::Vector => "VECTORCRAFT_CONTROL_TOKEN",
            Self::Design => "DESIGNCRAFT_CONTROL_TOKEN",
            Self::Pdf => "PDFCRAFT_CONTROL_TOKEN",
            Self::Deck => "DECKCRAFT_CONTROL_TOKEN",
            Self::Ani => "ANICRAFT_CONTROL_TOKEN",
            Self::Texture => "TEXTURECRAFT_CONTROL_TOKEN",
        }
    }

    pub const fn optional_in_doctor(self) -> bool {
        matches!(self, Self::Pdf | Self::Deck | Self::Ani | Self::Texture)
    }

    pub const fn auth(self) -> ControlAuth {
        match self {
            Self::Film => ControlAuth::InlineToken,
            Self::Photo | Self::Texture => ControlAuth::FirstFrameAuth,
            Self::Design | Self::Pdf | Self::Deck | Self::Effect | Self::Vector | Self::Ani => ControlAuth::None,
        }
    }

    pub const fn wire(self) -> ControlWire {
        match self {
            Self::Ani => ControlWire::CmdParams,
            _ => ControlWire::MethodParams,
        }
    }

    pub const fn uses_automation_roots(self) -> bool {
        matches!(self, Self::Photo | Self::Vector)
    }

    pub const fn uses_token_file_flag(self) -> bool {
        matches!(self, Self::Photo)
    }

    pub fn forbids_live_command(self, command: &str) -> bool {
        self == Self::Photo && command.starts_with("file.")
    }

    pub fn control_envelope(self, command: &str, params: &Value) -> (String, Value) {
        if self.wire() == ControlWire::CmdParams {
            return (command.to_string(), params.clone());
        }
        if command.starts_with("ui.")
            || command.starts_with("app.")
            || matches!(command, "session.inspect" | "render.frame" | "perf.stats")
        {
            return (command.to_string(), params.clone());
        }
        match self {
            Self::Texture => ("command.run".to_string(), json!({"id": command, "params": params})),
            Self::Vector if command == "document.inspect" => ("document.inspect".to_string(), params.clone()),
            _ => ("engine.execute".to_string(), json!({"command": command, "params": params})),
        }
    }

    pub fn live_inspect_envelope(self) -> (String, Value) {
        match self {
            Self::Film => self.control_envelope("project.inspect", &json!({})),
            Self::Effect => self.control_envelope("project.summary", &json!({})),
            Self::Design | Self::Pdf | Self::Deck => self.control_envelope("document.inspect", &json!({})),
            Self::Photo => ("ui.inspect".to_string(), json!({})),
            Self::Vector => ("document.inspect".to_string(), json!({})),
            Self::Texture => ("session.inspect".to_string(), json!({})),
            Self::Ani => self.control_envelope("doc.info", &json!({})),
        }
    }

    pub fn live_open_envelope(self, path: &Path) -> (String, Value) {
        match self {
            Self::Photo | Self::Vector => ("app.open".to_string(), json!({"path": path})),
            _ => self.control_envelope("file.open", &json!({"path": path})),
        }
    }

    pub fn live_save_envelope(self, doc: &Path) -> (String, Value) {
        match self {
            Self::Photo => ("app.save".to_string(), json!({"path": doc})),
            Self::Vector => ("app.save".to_string(), json!({"path": doc, "wait": true})),
            Self::Effect | Self::Texture | Self::Ani => self.control_envelope("file.save", &json!({"path": doc, "confirm": true})),
            Self::Film | Self::Design | Self::Pdf | Self::Deck => self.control_envelope("file.save", &json!({})),
        }
    }

    pub fn cli_control_hint(self, port: u16) -> String {
        match self {
            Self::Film => format!("filmcraft-cli --bridge 127.0.0.1:{port} exec project.inspect '{{}}'"),
            Self::Effect => format!("effectcraft-cli --bridge {port} --json exec project.summary '{{}}'"),
            Self::Design => format!("designcraft-cli app --port {port} --method document.inspect"),
            Self::Pdf => "pdfcraft-cli ui --control ~/.pdfcraft-control.json inspect".to_string(),
            Self::Deck => format!("deckcraft-cli app --port {port} --method document.inspect"),
            Self::Photo => format!("photocraft-cli mcp --bridge 127.0.0.1:{port}"),
            Self::Vector => format!("vectorcraft-cli mcp --connect 127.0.0.1:{port}"),
            Self::Texture => format!("texturecraft-cli mcp --bridge 127.0.0.1:{port}"),
            Self::Ani => "anicraft-cli serve".to_string(),
        }
    }

    pub fn mcp_bridge_hint(self, port: u16) -> String {
        match self {
            Self::Film => format!("filmcraft-cli mcp --bridge 127.0.0.1:{port}"),
            Self::Effect => format!("effectcraft-cli mcp --bridge {port}"),
            Self::Design => format!("designcraft-cli mcp --connect 127.0.0.1:{port}"),
            Self::Pdf => "pdfcraft-cli mcp --compact".to_string(),
            Self::Deck => format!("deckcraft-cli mcp --connect 127.0.0.1:{port}"),
            Self::Vector => format!("vectorcraft-cli mcp --connect 127.0.0.1:{port}"),
            Self::Photo => format!("photocraft-cli mcp --bridge 127.0.0.1:{port}"),
            Self::Texture => format!("texturecraft-cli mcp --bridge 127.0.0.1:{port}"),
            Self::Ani => "anicraft-cli mcp".to_string(),
        }
    }

    pub const fn mcp_headless_hint(self) -> &'static str {
        match self {
            Self::Film => "filmcraft-cli mcp",
            Self::Effect => "effectcraft-cli mcp --autosave",
            Self::Design => "designcraft-cli mcp",
            Self::Pdf => "pdfcraft-cli mcp --compact",
            Self::Deck => "deckcraft-cli mcp",
            Self::Vector => "vectorcraft-cli mcp",
            Self::Photo => "photocraft-cli mcp",
            Self::Texture => "texturecraft-cli mcp",
            Self::Ani => "anicraft-cli mcp",
        }
    }
}
