//! Embedded `24x24` monochrome workstation icon catalog and full-color companion poses
//! (`Fidget` dog and `Scribbles` cat) prepared once via [`craft_ui::icons::SvgIconSet`].

use std::sync::OnceLock;

use craft_ui::icons::{SvgIconSet, SvgStyle};
use egui::{Color32, Image, Vec2};

macro_rules! stroke_svg {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            r#"</svg>"#
        )
        .as_bytes()
    };
}

macro_rules! fill_svg {
    ($body:literal) => {
        concat!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="none">"#, $body, r#"</svg>"#).as_bytes()
    };
}

static MONO_ICONS: &[(&str, &[u8])] = &[
    ("square", stroke_svg!(r#"<rect x="4" y="4" width="16" height="16" rx="2"/>"#)),
    (
        "brand",
        fill_svg!(
            r#"<path d="M12 2l2.2 6.3L20.5 10.5l-6.3 2.2L12 19l-2.2-6.3L3.5 10.5l6.3-2.2L12 2zm6.5 12l1 2.8 2.8 1-2.8 1-1 2.8-1-2.8-2.8-1 2.8-1 1-2.8z"/>"#
        ),
    ),
    ("play", fill_svg!(r#"<polygon points="6 4 20 12 6 20 6 4"/>"#)),
    ("pause", fill_svg!(r#"<rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/>"#)),
    ("plus", stroke_svg!(r#"<path d="M12 5v14M5 12h14"/>"#)),
    ("folder", stroke_svg!(r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z"/>"#)),
    ("save", stroke_svg!(r#"<path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><path d="M17 21v-8H7v8M7 3v5h8"/>"#)),
    ("refresh", stroke_svg!(r#"<path d="M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6"/>"#)),
    ("external", stroke_svg!(r#"<path d="M14 4h6v6M20 4l-9 9M10 6H6a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-4"/>"#)),
    (
        "sparkles",
        stroke_svg!(r#"<path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9L12 3zm6 12l.9 2.1L21 18l-2.1.9L18 21l-.9-2.1L15 18l2.1-.9L18 15z"/>"#),
    ),
    ("scissors", stroke_svg!(r#"<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M20 4L8.12 15.88M14.47 14.48L20 20M8.12 8.12L12 12"/>"#)),
    ("layers", stroke_svg!(r#"<polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/>"#)),
    ("wand", stroke_svg!(r#"<path d="M15 4V2m0 14v-2M8 9h2m10 0h2m-4.2-3.8L19 4m-3.2 9.8L19 15M3 21l9-9m.2-6.2L11 4"/>"#)),
    ("film", stroke_svg!(r#"<rect x="2" y="3" width="20" height="18" rx="2"/><path d="M7 3v18M17 3v18M2 8h5M17 8h5M2 12h20M2 16h5M17 16h5"/>"#)),
    ("music", stroke_svg!(r#"<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>"#)),
    ("type", stroke_svg!(r#"<path d="M4 7V4h16v3M9 20h6M12 4v16"/>"#)),
    ("pen", stroke_svg!(r#"<path d="M12 20h9M16.5 3.5a2.12 2.12 0 1 1 3 3L7 19l-4 1 1-4 12.5-12.5z"/>"#)),
    ("trash", stroke_svg!(r#"<path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6h14"/>"#)),
    ("copy", stroke_svg!(r#"<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>"#)),
    (
        "paste",
        stroke_svg!(r#"<path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1"/>"#),
    ),
    ("check", stroke_svg!(r#"<polyline points="20 6 9 17 4 12"/>"#)),
    ("close", stroke_svg!(r#"<path d="M18 6L6 18M6 6l12 12"/>"#)),
    ("chevronLeft", stroke_svg!(r#"<polyline points="15 18 9 12 15 6"/>"#)),
    ("chevronRight", stroke_svg!(r#"<polyline points="9 18 15 12 9 6"/>"#)),
    ("chevronDown", stroke_svg!(r#"<polyline points="6 9 12 15 18 9"/>"#)),
    ("chevronUp", stroke_svg!(r#"<polyline points="18 15 12 9 6 15"/>"#)),
    ("sliders", stroke_svg!(r#"<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3"/><path d="M2 14h4M10 8h4M18 16h4"/>"#)),
    ("eye", stroke_svg!(r#"<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z"/><circle cx="12" cy="12" r="3"/>"#)),
    ("upload", stroke_svg!(r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"/>"#)),
    ("download", stroke_svg!(r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3"/>"#)),
    (
        "grid",
        stroke_svg!(
            r#"<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/>"#
        ),
    ),
    ("pin", stroke_svg!(r#"<path d="M12 17v5M9 3h6l1 7 3 3v2H5v-2l3-3 1-7z"/>"#)),
    ("undo", stroke_svg!(r#"<path d="M9 14L4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3"/>"#)),
    ("redo", stroke_svg!(r#"<path d="M15 14l5-5-5-5M20 9H9a5 5 0 0 0 0 10h3"/>"#)),
    ("camera", stroke_svg!(r#"<path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/>"#)),
    (
        "cube",
        stroke_svg!(
            r#"<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/>"#
        ),
    ),
    ("user", stroke_svg!(r#"<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>"#)),
    ("image", stroke_svg!(r#"<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>"#)),
    ("search", stroke_svg!(r#"<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.35-4.35"/>"#)),
    ("zoomIn", stroke_svg!(r#"<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.35-4.35M11 8v6M8 11h6"/>"#)),
    ("zoomOut", stroke_svg!(r#"<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.35-4.35M8 11h6"/>"#)),
    ("book", stroke_svg!(r#"<path d="M2 4h7a3 3 0 0 1 3 3v13a2 2 0 0 0-2-2H2z"/><path d="M22 4h-7a3 3 0 0 0-3 3v13a2 2 0 0 1 2-2h8z"/>"#)),
    ("send", stroke_svg!(r#"<path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/>"#)),
    (
        "link",
        stroke_svg!(r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"#),
    ),
];

static COMPANION_ICONS: &[(&str, &[u8])] = &[
    (
        "fidget_work",
        br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 160"><g stroke="#503329" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><g transform="rotate(-12 106 119)"><path d="M105 119 Q147 126 132 91" fill="none" stroke="#eab25b" stroke-width="15"/></g><g><path d="M46 95 Q73 80 102 98 L110 137 Q78 151 40 137Z" fill="#eab25b"/><ellipse cx="74" cy="122" rx="20" ry="23" fill="#fff0ce" stroke="none"/></g><g fill="#eab25b"><path d="M45 126 Q27 132 33 143 L59 143 L59 133 M91 133 L91 143 L118 143 Q124 130 105 127"/></g><g transform="rotate(-10 76 94)"><path d="M43 47 Q16 43 21 78 Q27 98 40 79Z M101 47 Q129 40 124 78 Q115 97 104 77Z" fill="#754225"/><path d="M36 62 Q35 34 74 34 Q113 33 114 62 L111 83 Q105 103 76 103 Q42 103 37 83Z" fill="#eab25b"/><ellipse cx="75" cy="81" rx="25" ry="17" fill="#fff0ce" stroke="none"/><ellipse cx="57" cy="64" rx="7" ry="9" fill="#352c37"/><ellipse cx="90" cy="64" rx="7" ry="9" fill="#352c37"/><circle cx="59" cy="61" r="2.5" fill="#fff5dd"/><circle cx="92" cy="61" r="2.5" fill="#fff5dd"/><path d="M68 76 Q75 70 82 76 L75 83Z" fill="#503329"/><path d="M64 84 Q74 102 84 84" fill="#67343d"/><path d="M69 91 Q74 102 79 91" fill="#f38691"/><path d="M43 80 L31 77 M104 80 L117 77" stroke="#754225"/></g><g><path d="M45 98 Q74 111 104 98 L102 109 Q73 121 47 108Z" fill="#53bfa9"/><path d="M91 109 L109 125 L108 106Z" fill="#53bfa9"/></g></g></svg>"##,
    ),
    (
        "scribbles_work",
        br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 160"><g stroke="#503329" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><g transform="rotate(-12 106 119)"><path d="M105 119 Q147 126 132 91" fill="none" stroke="#ee8241" stroke-width="15"/></g><g><path d="M46 95 Q73 80 102 98 L110 137 Q78 151 40 137Z" fill="#ee8241"/><ellipse cx="74" cy="122" rx="20" ry="23" fill="#fff0ce" stroke="none"/></g><g fill="#ee8241"><path d="M45 126 Q27 132 33 143 L59 143 L59 133 M91 133 L91 143 L118 143 Q124 130 105 127"/></g><g transform="rotate(-10 76 94)"><path d="M38 59 L35 19 Q58 25 62 41 M83 41 Q99 22 113 20 L110 64" fill="#ee8241"/><path d="M43 45 L42 30 L56 43 M91 43 L106 30 L105 48" fill="#f9b7a0"/><path d="M36 62 Q35 34 74 34 Q113 33 114 62 L111 83 Q105 103 76 103 Q42 103 37 83Z" fill="#ee8241"/><path d="M62 38 L65 47 M75 35 L75 46 M88 38 L85 47" stroke="#a7472c" stroke-width="5"/><ellipse cx="75" cy="81" rx="25" ry="17" fill="#fff0ce" stroke="none"/><ellipse cx="57" cy="64" rx="7" ry="9" fill="#352c37"/><ellipse cx="90" cy="64" rx="7" ry="9" fill="#352c37"/><circle cx="59" cy="61" r="2.5" fill="#fff5dd"/><circle cx="92" cy="61" r="2.5" fill="#fff5dd"/><path d="M68 76 Q75 70 82 76 L75 83Z" fill="#503329"/><path d="M64 84 Q74 102 84 84" fill="#67343d"/><path d="M69 91 Q74 102 79 91" fill="#f38691"/><path d="M43 80 L31 77 M104 80 L117 77" stroke="#a7472c"/></g><g><path d="M45 98 Q74 111 104 98 L102 109 Q73 121 47 108Z" fill="#9c83d9"/><path d="M91 109 L109 125 L108 106Z" fill="#9c83d9"/></g></g></svg>"##,
    ),
    (
        "scribbles_rest",
        br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 160"><g stroke="#503329" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><g><path d="M105 119 Q147 126 132 91" fill="none" stroke="#ee8241" stroke-width="15"/></g><g><path d="M46 95 Q73 80 102 98 L110 137 Q78 151 40 137Z" fill="#ee8241"/><ellipse cx="74" cy="122" rx="20" ry="23" fill="#fff0ce" stroke="none"/></g><g fill="#ee8241"><path d="M45 126 Q27 132 33 143 L59 143 L59 133 M91 133 L91 143 L118 143 Q124 130 105 127"/></g><g><path d="M38 59 L35 19 Q58 25 62 41 M83 41 Q99 22 113 20 L110 64" fill="#ee8241"/><path d="M43 45 L42 30 L56 43 M91 43 L106 30 L105 48" fill="#f9b7a0"/><path d="M36 62 Q35 34 74 34 Q113 33 114 62 L111 83 Q105 103 76 103 Q42 103 37 83Z" fill="#ee8241"/><path d="M62 38 L65 47 M75 35 L75 46 M88 38 L85 47" stroke="#a7472c" stroke-width="5"/><ellipse cx="75" cy="81" rx="25" ry="17" fill="#fff0ce" stroke="none"/><ellipse cx="57" cy="64" rx="7" ry="9" fill="#352c37"/><ellipse cx="90" cy="64" rx="7" ry="9" fill="#352c37"/><circle cx="59" cy="61" r="2.5" fill="#fff5dd"/><circle cx="92" cy="61" r="2.5" fill="#fff5dd"/><path d="M68 76 Q75 70 82 76 L75 83Z" fill="#503329"/><path d="M62 83 Q74 92 86 83" fill="none"/><path d="M43 80 L31 77 M104 80 L117 77" stroke="#a7472c"/></g><g><path d="M45 98 Q74 111 104 98 L102 109 Q73 121 47 108Z" fill="#9c83d9"/><path d="M91 109 L109 125 L108 106Z" fill="#9c83d9"/></g></g></svg>"##,
    ),
];

pub fn catalog() -> &'static SvgIconSet {
    static SET: OnceLock<SvgIconSet> = OnceLock::new();
    SET.get_or_init(|| SvgIconSet::new(MONO_ICONS, SvgStyle { current_color: "#ffffff", stroke_width: Some("1.85") }, "bytes://magic/icons/", "square"))
}

pub fn companion_catalog() -> &'static SvgIconSet {
    static SET: OnceLock<SvgIconSet> = OnceLock::new();
    SET.get_or_init(|| {
        SvgIconSet::new(COMPANION_ICONS, SvgStyle { current_color: "#ffffff", stroke_width: None }, "bytes://magic/companions/", "scribbles_work")
    })
}

pub fn icon_image(name: &str, size: f32, tint: Color32) -> Image<'static> {
    let resolved = if catalog().contains(name) { name } else { "square" };
    Image::new(catalog().source(resolved)).fit_to_exact_size(Vec2::splat(size)).tint(tint)
}

pub fn companion_image(name: &str, size: f32) -> Image<'static> {
    let resolved = if companion_catalog().contains(name) { name } else { "scribbles_work" };
    Image::new(companion_catalog().source(resolved)).fit_to_exact_size(Vec2::splat(size))
}
