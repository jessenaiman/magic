//! Shared Craft-document timeline widget (`draw_craft_timeline`) used across `Animations`,
//! `Lyrics`, `Motion`, and `Studio` (`craft-shared` Phase 3).
//!
//! Renders a legible top time ruler with major/minor ticks, a glowing playhead cap + spanning line,
//! audio waveform detail, diamond keyframes, filmstrip scene tiles, and click/drag scrubbing.

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use serde_json::json;

use crate::controls::{FilmbarOrientation, FilmbarStyle, paint_filmbar_frame, paint_local_image};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, status_pill};

const TRACK_HEADER_W: f32 = 96.0;

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
    pub waveform: bool,
    pub dirty: bool,
    pub spans: Vec<CraftTimelineSpan>,
    pub keyframes: &'a [f64],
    pub frames: &'a [String],
    pub empty_hint: &'a str,
}

fn format_ruler_tick(sec: f64, dur: f64) -> String {
    if dur <= 12.0 {
        format!("{sec:.1}s")
    } else {
        let mins = (sec / 60.0).floor() as u64;
        let rem = sec - (mins as f64) * 60.0;
        format!("{mins}:{rem:04.1}")
    }
}

fn choose_major_step(dur: f64, width_px: f32) -> f64 {
    let target_ticks = ((width_px / 88.0) as usize).clamp(4, 12) as f64;
    let raw = dur / target_ticks;
    for candidate in [0.25, 0.5, 1.0, 2.0, 2.5, 5.0, 10.0, 15.0, 30.0, 60.0] {
        if candidate >= raw {
            return candidate;
        }
    }
    60.0
}

fn paint_waveform_bars(ui: &mut Ui, rect: Rect, color: Color32, seed_tag: &str) {
    let mut seed: u32 = 0x6D2B_79F5;
    for b in seed_tag.bytes() {
        seed = seed.wrapping_mul(33).wrapping_add(u32::from(b));
    }
    let count = ((rect.width() / 3.2) as usize).clamp(8, 220);
    let mid_y = rect.center().y;
    let max_half = (rect.height() * 0.42).max(2.0);
    let stroke = Stroke::new(1.5, color.gamma_multiply(0.82));
    for i in 0..count {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let frac = (i as f32 + 0.5) / (count as f32);
        let env = (frac * std::f32::consts::PI * 3.0).sin().abs() * 0.55 + 0.45;
        let rand = ((seed >> 16) as f32) / 65535.0;
        let h = (max_half * (0.22 + 0.78 * rand * env)).max(1.5);
        let x = rect.min.x + frac * rect.width();
        ui.painter().line_segment([Pos2::new(x, mid_y - h), Pos2::new(x, mid_y + h)], stroke);
    }
}

fn paint_diamond(painter: &egui::Painter, center: Pos2, radius: f32, fill: Color32, border: Color32) {
    painter.add(Shape::convex_polygon(
        vec![
            Pos2::new(center.x, center.y - radius),
            Pos2::new(center.x + radius, center.y),
            Pos2::new(center.x, center.y + radius),
            Pos2::new(center.x - radius, center.y),
        ],
        fill,
        Stroke::new(1.0, border),
    ));
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

    // 1. Time Ruler Row with major/minor ticks & clickable playhead scrub
    let mut ruler_track_rect = Rect::NOTHING;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(TRACK_HEADER_W, 18.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new(format_ruler_tick(*playhead_sec, dur)).monospace().size(9.5).color(ctx.pal.accent_emerald));
        });
        let bar_w = (ui.available_width() - 2.0).max(100.0);
        let (rrect, rresp) = ui.allocate_exact_size(Vec2::new(bar_w, 18.0), Sense::click_and_drag());
        ruler_track_rect = rrect;
        if (rresp.clicked() || rresp.dragged())
            && let Some(pos) = rresp.interact_pointer_pos()
        {
            let frac = ((pos.x - rrect.min.x) / rrect.width().max(1.0)).clamp(0.0, 1.0) as f64;
            *playhead_sec = frac * dur;
            scrubbed = true;
        }
        ui.painter().rect_filled(rrect, CornerRadius::same(3), Color32::from_rgb(0x0c, 0x11, 0x18));
        ui.painter().line_segment([Pos2::new(rrect.min.x, rrect.max.y - 1.0), Pos2::new(rrect.max.x, rrect.max.y - 1.0)], Stroke::new(1.0, ctx.pal.border));

        let major_step = choose_major_step(dur, bar_w);
        let minor_step = (major_step / 5.0).max(0.05);
        let mut t = 0.0_f64;
        while t <= dur + 1e-4 {
            let x = rrect.min.x + ((t / dur).clamp(0.0, 1.0) as f32) * bar_w;
            let rem = (t / major_step) - (t / major_step).round();
            let is_major = rem.abs() < 0.04;
            let tick_h = if is_major { 6.5 } else { 3.2 };
            let tick_col = if is_major { ctx.pal.text_muted } else { ctx.pal.border };
            ui.painter().line_segment([Pos2::new(x, rrect.max.y - tick_h), Pos2::new(x, rrect.max.y - 1.0)], Stroke::new(1.0, tick_col));
            if is_major && x + 28.0 <= rrect.max.x + 12.0 {
                ui.painter().text(
                    Pos2::new(x + 2.5, rrect.min.y + 6.5),
                    Align2::LEFT_CENTER,
                    format_ruler_tick(t, dur),
                    FontId::monospace(8.5),
                    ctx.pal.text_muted,
                );
            }
            t += minor_step;
        }
        ctx.reg.record(format!("{id_prefix}-ruler"), "Timeline Ruler", &rresp, Some(format!("{:.2}", *playhead_sec)));
    });

    ui.spacing_mut().item_spacing.y = 3.0;
    let mut bottom_lane_max_y = ruler_track_rect.max.y;

    for lane in lanes {
        ui.horizontal(|ui| {
            let row_h = if lane.filmbar { 26.0 } else { 21.0 };
            ui.allocate_ui_with_layout(Vec2::new(TRACK_HEADER_W, row_h), Layout::left_to_right(Align::Center), |ui| {
                let (dr, _) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), Sense::hover());
                let dot_col = if lane.dirty { ctx.pal.accent_amber } else { lane.color };
                ui.painter().circle_filled(dr.center(), 3.0, dot_col);
                let title_txt = if lane.dirty { format!("{}*", lane.title) } else { lane.title.to_owned() };
                ui.label(RichText::new(title_txt).monospace().size(9.5).color(ctx.pal.text_secondary));
            });

            let bar_w = (ui.available_width() - 2.0).max(100.0);
            let (lane_rect, lresp) = ui.allocate_exact_size(Vec2::new(bar_w, row_h), Sense::click_and_drag());
            bottom_lane_max_y = bottom_lane_max_y.max(lane_rect.max.y);
            if (lresp.clicked() || lresp.dragged())
                && let Some(pos) = lresp.interact_pointer_pos()
            {
                let frac = ((pos.x - lane_rect.min.x) / lane_rect.width().max(1.0)).clamp(0.0, 1.0) as f64;
                *playhead_sec = frac * dur;
                scrubbed = true;
            }

            if lane.filmbar {
                let inner = paint_filmbar_frame(
                    ui,
                    lane_rect,
                    ctx.pal,
                    FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: lresp.hovered(), timecode: None },
                );
                if !lane.frames.is_empty() {
                    let slots = ((inner.width() / 46.0).round() as usize).clamp(4, 18);
                    let tile_w = inner.width() / (slots as f32);
                    for slot_i in 0..slots {
                        let fp = &lane.frames[slot_i % lane.frames.len()];
                        let tr = Rect::from_min_size(
                            Pos2::new(inner.min.x + (slot_i as f32) * tile_w, inner.min.y),
                            Vec2::new((tile_w - 1.5).max(8.0), inner.height()),
                        );
                        let _ = paint_local_image(ui, tr, Some(fp.as_str()), ctx.pal.bg_stage, 1);
                    }
                } else if !lane.empty_hint.is_empty() {
                    ui.painter().text(inner.center(), Align2::CENTER_CENTER, lane.empty_hint, FontId::monospace(8.5), ctx.pal.text_muted);
                }
            } else {
                let border_col = if lane.dirty { ctx.pal.accent_amber } else { lane.color.gamma_multiply(0.55) };
                ui.painter().rect(lane_rect, CornerRadius::same(4), ctx.pal.bg_input, Stroke::new(1.0, border_col), StrokeKind::Inside);
                ui.painter().line_segment(
                    [Pos2::new(lane_rect.min.x + 2.0, lane_rect.center().y), Pos2::new(lane_rect.max.x - 2.0, lane_rect.center().y)],
                    Stroke::new(0.7, ctx.pal.border_subtle),
                );

                if !lane.spans.is_empty() {
                    for (sp_idx, sp) in lane.spans.iter().enumerate() {
                        let x0 = lane_rect.min.x + ((sp.start_sec / dur).clamp(0.0, 1.0) as f32) * bar_w;
                        let span_len = (sp.end_sec - sp.start_sec).max(0.15);
                        let cw = ((span_len / dur) as f32 * bar_w).max(10.0);
                        let cr = Rect::from_min_size(Pos2::new(x0, lane_rect.min.y + 2.0), Vec2::new(cw.min((lane_rect.max.x - x0).max(4.0)), row_h - 4.0));
                        ui.painter().rect(
                            cr,
                            CornerRadius::same(3),
                            lane.color.gamma_multiply(0.24),
                            Stroke::new(1.0, lane.color.gamma_multiply(0.78)),
                            StrokeKind::Inside,
                        );
                        if lane.waveform && cr.width() >= 18.0 {
                            paint_waveform_bars(ui, cr.shrink2(Vec2::new(2.0, 1.5)), lane.color, &format!("{}-{sp_idx}", lane.code));
                        }
                        if cr.width() >= 28.0 && !sp.label.is_empty() {
                            let max_c = ((cr.width() / 5.6) as usize).max(3);
                            let lbl = if sp.label.chars().count() > max_c {
                                let mut s: String = sp.label.chars().take(max_c.saturating_sub(1)).collect();
                                s.push('…');
                                s
                            } else {
                                sp.label.clone()
                            };
                            ui.painter().text(
                                Pos2::new(cr.min.x + 5.0, cr.center().y),
                                Align2::LEFT_CENTER,
                                &lbl,
                                FontId::monospace(8.5),
                                ctx.pal.text_primary,
                            );
                        }
                    }
                } else if lane.waveform {
                    paint_waveform_bars(ui, lane_rect.shrink2(Vec2::new(4.0, 2.5)), lane.color, lane.code);
                } else if !lane.frames.is_empty() {
                    let n = lane.frames.len().clamp(1, 6);
                    for (k, fp) in lane.frames.iter().take(n).enumerate() {
                        let kr = Rect::from_center_size(
                            Pos2::new(lane_rect.min.x + (k as f32 + 0.5) * (bar_w / (n as f32)), lane_rect.center().y),
                            Vec2::new(32.0, row_h - 4.0),
                        );
                        let _ = paint_local_image(ui, kr, Some(fp.as_str()), ctx.pal.bg_stage, 2);
                    }
                } else if !lane.empty_hint.is_empty() && lane.keyframes.is_empty() {
                    ui.painter().text(lane_rect.center(), Align2::CENTER_CENTER, lane.empty_hint, FontId::monospace(8.5), ctx.pal.text_muted);
                }

                for &kt in lane.keyframes {
                    if kt >= 0.0 && kt <= dur {
                        let kx = lane_rect.min.x + ((kt / dur) as f32) * bar_w;
                        paint_diamond(ui.painter(), Pos2::new(kx, lane_rect.center().y), 4.2, lane.color, Color32::from_rgb(0x09, 0x0e, 0x15));
                    }
                }
            }

            ctx.reg.record(format!("{id_prefix}-lane-{}", lane.code), lane.title, &lresp, None);
        });
    }

    // 2. Continuous Playhead Cap + Spanning Line across ruler and all lanes
    if ruler_track_rect.is_positive() {
        let ph_x = ruler_track_rect.min.x + ((*playhead_sec / dur).clamp(0.0, 1.0) as f32) * ruler_track_rect.width();
        let cap = Rect::from_center_size(Pos2::new(ph_x, ruler_track_rect.min.y + 5.5), Vec2::new(9.0, 9.0));
        ui.painter().rect_filled(cap, CornerRadius::same(3), ctx.pal.accent_emerald);
        ui.painter().line_segment([Pos2::new(ph_x, cap.max.y), Pos2::new(ph_x, bottom_lane_max_y)], Stroke::new(1.6, ctx.pal.accent_emerald));
    }
    scrubbed
}
