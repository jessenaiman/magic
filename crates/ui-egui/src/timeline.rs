//! Shared Craft-document timeline widget (`draw_craft_timeline`) used across `Animations`,
//! `Lyrics`, `Motion`, and `Studio` (`craft-shared` Phase 3).
//!
//! Renders multi-track spans directly from real `.ecproj` / `.fcproj` payloads, click-to-scrub
//! playhead rails, and an inline `"Out of sync — Sync"` action badge whenever `dirty_tracks`
//! indicates an underlying `.ecproj` changed on disk.

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use serde_json::json;

use crate::controls::{FilmbarOrientation, FilmbarStyle, paint_filmbar_frame, paint_local_image};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, status_pill};

#[derive(Clone, Debug)]
pub struct CraftTimelineSpan {
    pub start_sec: f64,
    pub end_sec: f64,
    pub label: String,
}

pub struct CraftTimelineLane<'a> {
    pub code: &'a str,
    pub title: &'a str,
    pub color: Color32,
    pub filmbar: bool,
    pub dirty: bool,
    pub spans: Vec<CraftTimelineSpan>,
    pub frames: &'a [String],
    pub empty_hint: &'a str,
}

pub fn draw_craft_timeline(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    id_prefix: &str,
    total_dur_sec: f64,
    playhead_sec: &mut f64,
    lanes: &[CraftTimelineLane<'_>],
) -> bool {
    let dur = total_dur_sec.max(1.0);
    let mut scrubbed = false;
    let any_dirty = lanes.iter().any(|l| l.dirty);

    if any_dirty {
        ui.horizontal(|ui| {
            status_pill(ui, ctx.pal, "Out of sync (.ecproj modified)", ctx.pal.accent_amber);
            let sync_id = format!("{id_prefix}-sync-links");
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new(&sync_id, "refresh", "Sync Linked Comps", "Re-render dirty .ecproj tracks and swap FilmCraft footage").primary(),
            )
            .clicked()
            {
                ctx.exec("video.sync_links", json!({"render": true}), Some("Synced modified EffectCraft tracks into FilmCraft"));
                ctx.state.dirty_tracks.clear();
            }
        });
        ui.add_space(2.0);
    }

    for lane in lanes {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(88.0, 20.0), Layout::left_to_right(Align::Center), |ui| {
                let (dr, _) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), Sense::hover());
                let dot_col = if lane.dirty { ctx.pal.accent_amber } else { lane.color };
                ui.painter().circle_filled(dr.center(), 3.0, dot_col);
                let title_txt = if lane.dirty {
                    format!("{}*", lane.title)
                } else {
                    lane.title.to_owned()
                };
                ui.label(RichText::new(title_txt).monospace().size(9.5).color(ctx.pal.text_secondary));
            });

            let bar_w = ui.available_width().max(100.0);
            let row_h = if lane.filmbar { 22.0 } else { 18.0 };
            let (lane_rect, lresp) = ui.allocate_exact_size(Vec2::new(bar_w, row_h), Sense::click_and_drag());
            if (lresp.clicked() || lresp.dragged())
                && let Some(pos) = lresp.interact_pointer_pos()
            {
                let frac = ((pos.x - lane_rect.min.x) / lane_rect.width()).clamp(0.0, 1.0) as f64;
                *playhead_sec = frac * dur;
                scrubbed = true;
            }

            if lane.filmbar {
                let inner = paint_filmbar_frame(
                    ui,
                    lane_rect,
                    ctx.pal,
                    FilmbarStyle {
                        orientation: FilmbarOrientation::Horizontal,
                        selected: false,
                        hovered: lresp.hovered(),
                        timecode: None,
                    },
                );
                if !lane.frames.is_empty() {
                    let n = lane.frames.len().clamp(1, 8);
                    let tile_w = (inner.width() / (n as f32)).max(24.0);
                    for (idx, fp) in lane.frames.iter().take(n).enumerate() {
                        let tr = Rect::from_min_size(
                            Pos2::new(inner.min.x + (idx as f32) * tile_w, inner.min.y),
                            Vec2::new(tile_w - 1.0, inner.height()),
                        );
                        let _ = paint_local_image(ui, tr, Some(fp.as_str()), ctx.pal.bg_stage, 1);
                    }
                }
            } else {
                let border_col = if lane.dirty {
                    ctx.pal.accent_amber
                } else {
                    lane.color.gamma_multiply(0.55)
                };
                ui.painter().rect(
                    lane_rect,
                    CornerRadius::same(4),
                    ctx.pal.bg_input,
                    Stroke::new(1.0, border_col),
                    StrokeKind::Inside,
                );
                if !lane.spans.is_empty() {
                    for sp in &lane.spans {
                        let x0 = lane_rect.min.x + ((sp.start_sec / dur).clamp(0.0, 1.0) as f32) * bar_w;
                        let span_len = (sp.end_sec - sp.start_sec).max(0.15);
                        let cw = ((span_len / dur) as f32 * bar_w).max(8.0);
                        let cr = Rect::from_min_size(
                            Pos2::new(x0, lane_rect.min.y + 2.0),
                            Vec2::new(cw.min((lane_rect.max.x - x0).max(4.0)), row_h - 4.0),
                        );
                        ui.painter().rect_filled(cr, CornerRadius::same(3), lane.color.gamma_multiply(0.38));
                        if cr.width() >= 28.0 && !sp.label.is_empty() {
                            ui.painter().text(
                                cr.center(),
                                Align2::CENTER_CENTER,
                                &sp.label,
                                FontId::monospace(8.5),
                                ctx.pal.text_primary,
                            );
                        }
                    }
                } else if !lane.frames.is_empty() {
                    let n = lane.frames.len().clamp(1, 5);
                    for (k, fp) in lane.frames.iter().take(n).enumerate() {
                        let kr = Rect::from_center_size(
                            Pos2::new(lane_rect.min.x + (k as f32 + 0.5) * (bar_w / (n as f32)), lane_rect.center().y),
                            Vec2::new(28.0, row_h - 4.0),
                        );
                        let _ = paint_local_image(ui, kr, Some(fp.as_str()), ctx.pal.bg_stage, 2);
                    }
                } else if !lane.empty_hint.is_empty() {
                    ui.painter().text(
                        lane_rect.center(),
                        Align2::CENTER_CENTER,
                        lane.empty_hint,
                        FontId::monospace(8.5),
                        ctx.pal.text_muted,
                    );
                }
            }

            let ph_x = lane_rect.min.x + ((*playhead_sec / dur).clamp(0.0, 1.0) as f32) * bar_w;
            ui.painter().line_segment(
                [Pos2::new(ph_x, lane_rect.min.y), Pos2::new(ph_x, lane_rect.max.y)],
                Stroke::new(1.5, ctx.pal.accent_emerald),
            );
            ctx.reg
                .record(format!("{id_prefix}-lane-{}", lane.code), lane.title, &lresp, None);
        });
    }
    scrubbed
}
