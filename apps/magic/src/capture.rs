//! Cross-platform `ui.screenshot {path?, panel?, id?}` powered by [`craft_ui::capture::CaptureSlot`]
//! (`wgpu` readback on macOS Metal, Windows DX12, and Linux/BSD Vulkan) and atomic private writes
//! (`craft-atomic` `NewFilePermissions::Private` when writing default temp files).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use craft_atomic::{NewFilePermissions, WriteOptions};
use craft_ui::capture::CaptureSlot;
use egui::{ColorImage, Context, Rect, ViewportCommand, ViewportId};
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use magic_ui_egui::AutomationRegistry;
use serde_json::{Value, json};

const PER_IMAGE_BOUND_BYTES: usize = 128 * 1024 * 1024;
pub const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct PendingCapture {
    pub slot: CaptureSlot,
    pub target: PathBuf,
    pub is_temp: bool,
    pub panel: Option<String>,
    pub widget_id: Option<String>,
    pub settle_frames: u8,
    pub issued: bool,
    pub deadline: Instant,
    pub reply: mpsc::Sender<Result<Value, String>>,
}

impl PendingCapture {
    pub fn start(
        ctx: &Context,
        target: PathBuf,
        is_temp: bool,
        panel: Option<String>,
        widget_id: Option<String>,
        reply: mpsc::Sender<Result<Value, String>>,
    ) -> Self {
        let (slot, callback) = CaptureSlot::new(ctx, ViewportId::ROOT, PER_IMAGE_BOUND_BYTES);
        ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Screenshot(callback));
        Self { slot, target, is_temp, panel, widget_id, settle_frames: 0, issued: true, deadline: Instant::now() + CAPTURE_TIMEOUT, reply }
    }

    pub fn poll(&mut self, pixels_per_point: f32, reg: &AutomationRegistry) -> bool {
        if let Some(outcome) = self.slot.take() {
            let reply = match outcome {
                Ok(image) => finish_capture(&image, pixels_per_point, reg, &self.target, self.is_temp, self.panel.as_deref(), self.widget_id.as_deref()),
                Err(err) => Err(format!("screenshot capture failed: {err:?}")),
            };
            let _ = self.reply.send(reply);
            return true;
        }
        if Instant::now() >= self.deadline {
            let _ = self.reply.send(Err("no frame was presented within 10 seconds (window hidden or display asleep)".into()));
            return true;
        }
        false
    }
}

fn finish_capture(
    image: &ColorImage,
    pixels_per_point: f32,
    reg: &AutomationRegistry,
    target: &Path,
    is_temp: bool,
    panel: Option<&str>,
    widget_id: Option<&str>,
) -> Result<Value, String> {
    let crop_rect = resolve_crop_rect(reg, panel, widget_id)?;
    let (rgba, width, height) = match crop_rect {
        Some(rect) => crop_rgba(image, rect, pixels_per_point)?,
        None => {
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
            let w = u32::try_from(image.size[0]).map_err(|_| "image width overflow")?;
            let h = u32::try_from(image.size[1]).map_err(|_| "image height overflow")?;
            (bytes, w, h)
        }
    };
    let mut png = Vec::new();
    PngEncoder::new(&mut png).write_image(&rgba, width, height, ColorType::Rgba8.into()).map_err(|e| format!("PNG encoding failed: {e}"))?;
    if is_temp {
        let opts = WriteOptions::default().with_new_file_permissions(NewFilePermissions::Private);
        opts.atomic_write(target, &png).map_err(|e| format!("could not write {}: {e}", target.display()))?;
    } else {
        magic::write_file(target, &png).map_err(|e| format!("could not write {}: {e}", target.display()))?;
    }
    Ok(json!({
        "ok": true,
        "path": target,
        "width": width,
        "height": height,
        "bytes": png.len(),
    }))
}

fn resolve_crop_rect(reg: &AutomationRegistry, panel: Option<&str>, widget_id: Option<&str>) -> Result<Option<Rect>, String> {
    if panel.is_some() && widget_id.is_some() {
        return Err("ui.screenshot accepts either `panel` or `id`, not both".into());
    }
    if let Some(panel_name) = panel {
        let rect = reg
            .panel_rect(panel_name)
            .ok_or_else(|| format!("unknown or off-screen panel `{panel_name}` (expected app-bar, stage-assets, central, or app-footer)"))?;
        return Ok(Some(rect));
    }
    if let Some(id) = widget_id {
        let elem = reg.get(id).ok_or_else(|| format!("widget `{id}` is not mounted in the current frame"))?;
        return Ok(Some(elem.rect));
    }
    Ok(None)
}

fn crop_rgba(image: &ColorImage, logical: Rect, ppp: f32) -> Result<(Vec<u8>, u32, u32), String> {
    let scale = if ppp.is_finite() && ppp > 0.0 { ppp } else { 1.0 };
    let img_w = image.size[0];
    let img_h = image.size[1];
    let x0 = ((logical.min.x * scale).floor().max(0.0) as usize).min(img_w);
    let y0 = ((logical.min.y * scale).floor().max(0.0) as usize).min(img_h);
    let x1 = ((logical.max.x * scale).ceil().max(0.0) as usize).min(img_w);
    let y1 = ((logical.max.y * scale).ceil().max(0.0) as usize).min(img_h);
    if x1 <= x0 || y1 <= y0 {
        return Err("requested crop region lies outside the rendered viewport".into());
    }
    let crop_w = x1 - x0;
    let crop_h = y1 - y0;
    let mut out = Vec::with_capacity(crop_w * crop_h * 4);
    for row in y0..y1 {
        for col in x0..x1 {
            out.extend_from_slice(&image.pixels[row * img_w + col].to_srgba_unmultiplied());
        }
    }
    let w = u32::try_from(crop_w).map_err(|_| "crop width overflow")?;
    let h = u32::try_from(crop_h).map_err(|_| "crop height overflow")?;
    Ok((out, w, h))
}

pub fn target_path(requested: Option<&str>) -> Result<(PathBuf, bool), String> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let Some(requested) = requested else {
        let seq = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("magic-shots-{}", std::process::id()));
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&dir).map_err(|e| format!("cannot create private screenshot directory: {e}"))?;
        return Ok((dir.join(format!("shot-{seq}.png")), true));
    };
    let path = Path::new(requested);
    if !path.is_absolute() {
        return Err("path must be absolute".into());
    }
    if path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() != Some("png") {
        return Err("path must end in .png".into());
    }
    match path.parent() {
        Some(parent) if parent.is_dir() => Ok((path.to_path_buf(), false)),
        _ => Err("the folder for path does not exist".into()),
    }
}
