//! Settings schema for Magic Studio (`prefs.schema`, `prefs.get`, `prefs.set`).
//!
//! Scoped strictly to Magic Studio's coordinator responsibilities (AI roles via
//! `storytold/craft-ai-integrations`, appearance, project root,
//! lyrics font paths, character preview, and `.magic` autosave) — NLE playback, media-cache, and
//! track-label settings belong to `FilmCraft` and `EffectCraft`, not Magic.

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Bool,
    Int { min: f64, max: f64, unit: &'static str },
    Float { min: f64, max: f64, decimals: usize, unit: &'static str },
    Choice(&'static [(&'static str, &'static str)]),
    Text,
    Path,
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    pub wired: bool,
    pub enabled_by: Option<&'static str>,
}

pub struct Category {
    pub id: &'static str,
    pub title: &'static str,
}

pub const CATEGORIES: &[Category] = &[
    Category { id: "ai", title: "AI & LLM (craft-ai-integrations)" },
    Category { id: "appearance", title: "Appearance & Interface Style" },
    Category { id: "general", title: "General" },
    Category { id: "project", title: "Project Workspace" },
    Category { id: "lyrics", title: "Lyrics & Fonts" },
    Category { id: "animation", title: "Animation Preview" },
    Category { id: "autosave", title: "AutoSave" },
];

pub fn fields_with_cat() -> Vec<(&'static str, Field)> {
    vec![
        ("ai", Field { key: "ai.enabled", label: "Enable AI Integrations (storytold/craft-ai-integrations)", kind: Kind::Bool, wired: true, enabled_by: None }),
        (
            "ai",
            Field {
                key: "fidget.enabled",
                label: "Enable Fidget Structured Prompt Stager (text.complete)",
                kind: Kind::Bool,
                wired: true,
                enabled_by: Some("ai.enabled"),
            },
        ),
        (
            "ai",
            Field {
                key: "fidget.seamless",
                label: "Seamless Fidget -> Scribbles Handoff (skip manual staging review)",
                kind: Kind::Bool,
                wired: true,
                enabled_by: Some("fidget.enabled"),
            },
        ),
        (
            "ai",
            Field { key: "fidget.connection", label: "Fidget SharedStore Connection ID", kind: Kind::Text, wired: true, enabled_by: Some("fidget.enabled") },
        ),
        ("ai", Field { key: "fidget.model", label: "Fidget Model ID", kind: Kind::Text, wired: true, enabled_by: Some("fidget.enabled") }),
        (
            "ai",
            Field {
                key: "scribbles.enabled",
                label: "Enable Scribbles Image Generation (image.generate)",
                kind: Kind::Bool,
                wired: true,
                enabled_by: Some("ai.enabled"),
            },
        ),
        (
            "ai",
            Field {
                key: "scribbles.connection",
                label: "Scribbles SharedStore Connection ID",
                kind: Kind::Text,
                wired: true,
                enabled_by: Some("scribbles.enabled"),
            },
        ),
        ("ai", Field { key: "scribbles.model", label: "Scribbles Model ID", kind: Kind::Text, wired: true, enabled_by: Some("scribbles.enabled") }),
        (
            "ai",
            Field {
                key: "scribbles.endpoint",
                label: "Local Subscription / OpenAI-Compatible Endpoint URL",
                kind: Kind::Text,
                wired: true,
                enabled_by: Some("scribbles.enabled"),
            },
        ),
        ("ai", Field { key: "scribbles.template", label: "Default Scribbles Drawing Template", kind: Kind::Text, wired: true, enabled_by: None }),
        (
            "appearance",
            Field {
                key: "appearance.interface_style",
                label: "Interface Style",
                kind: Kind::Choice(&[
                    ("aurora", "Aurora"),
                    ("modern", "Modern"),
                    ("classic", "Classic"),
                    ("minimal", "Minimal"),
                    ("ocean", "Ocean"),
                    ("forest", "Forest"),
                    ("plum", "Plum"),
                    ("sand", "Sand"),
                ]),
                wired: true,
                enabled_by: None,
            },
        ),
        (
            "appearance",
            Field {
                key: "appearance.theme",
                label: "Colour mode",
                kind: Kind::Choice(&[("system", "System"), ("dark", "Dark"), ("light", "Light")]),
                wired: true,
                enabled_by: None,
            },
        ),
        ("appearance", Field { key: "appearance.accessible_contrast", label: "High contrast", kind: Kind::Bool, wired: true, enabled_by: None }),
        (
            "appearance",
            Field {
                key: "appearance.accent",
                label: "Accent",
                kind: Kind::Choice(&[("teal", "Teal (Mint)"), ("blue", "Blue"), ("violet", "Violet"), ("amber", "Amber"), ("graphite", "Graphite")]),
                wired: true,
                enabled_by: None,
            },
        ),
        (
            "appearance",
            Field {
                key: "appearance.ui_scale",
                label: "Interface scale",
                kind: Kind::Float { min: 0.8, max: 2.0, decimals: 2, unit: "x" },
                wired: true,
                enabled_by: None,
            },
        ),
        (
            "general",
            Field {
                key: "general.at_startup",
                label: "At Startup",
                kind: Kind::Choice(&[("showHome", "Show Home"), ("openLast", "Open Most Recent Project")]),
                wired: true,
                enabled_by: None,
            },
        ),
        (
            "general",
            Field {
                key: "general.when_opening_project",
                label: "When Opening Project",
                kind: Kind::Choice(&[("showOpenDialog", "Show File Dialog"), ("showProjectView", "Open Project Trunk")]),
                wired: true,
                enabled_by: None,
            },
        ),
        ("general", Field { key: "general.show_tooltips", label: "Show Tooltips", kind: Kind::Bool, wired: true, enabled_by: None }),
        ("general", Field { key: "general.show_rich_tooltips", label: "Show Rich Tooltips", kind: Kind::Bool, wired: true, enabled_by: None }),
        ("general", Field { key: "general.recent_projects", label: "Recent Projects", kind: Kind::Text, wired: true, enabled_by: None }),
        ("project", Field { key: "project.root", label: "Production Projects Folder (~/Magic)", kind: Kind::Path, wired: true, enabled_by: None }),
        (
            "project",
            Field {
                key: "project.storyboard_template",
                label: "System or Custom Storyboard Template (.designcraft)",
                kind: Kind::Text,
                wired: true,
                enabled_by: None,
            },
        ),
        ("project", Field { key: "project.storyboard_format", label: "Storyboard Reference Format", kind: Kind::Text, wired: true, enabled_by: None }),
        (
            "lyrics",
            Field {
                key: "lyrics.default_size",
                label: "Default Lyric Font Size",
                kind: Kind::Int { min: 16.0, max: 300.0, unit: "px" },
                wired: true,
                enabled_by: None,
            },
        ),
        ("lyrics", Field { key: "lyrics.outline", label: "Default Lyric Stroke Outline", kind: Kind::Bool, wired: true, enabled_by: None }),
        ("lyrics", Field { key: "lyrics.font_dirs", label: "Additional Font Folders", kind: Kind::Path, wired: true, enabled_by: None }),
        ("animation", Field { key: "animation.loop_preview", label: "Loop Character Preview", kind: Kind::Bool, wired: true, enabled_by: None }),
        (
            "animation",
            Field {
                key: "animation.duration",
                label: "Default Character Study Duration",
                kind: Kind::Float { min: 0.5, max: 60.0, decimals: 1, unit: "s" },
                wired: true,
                enabled_by: None,
            },
        ),
        (
            "autosave",
            Field { key: "autosave.enabled", label: "Enable Crash-Recovery Journal (.history/*.autosave)", kind: Kind::Bool, wired: true, enabled_by: None },
        ),
        (
            "autosave",
            Field {
                key: "autosave.interval_minutes",
                label: "AutoSave Interval",
                kind: Kind::Int { min: 1.0, max: 60.0, unit: "min" },
                wired: true,
                enabled_by: Some("autosave.enabled"),
            },
        ),
    ]
}
