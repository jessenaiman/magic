//! Single standard Craft timeline component (`draw_standard_timeline`) shared across all Magic
//! Studio stages in two modes:
//! - [`TimelineFlavor::FilmCraft`] (`05 Studio`): Multi-track NLE sequence (`V1`–`V4`, `C1`, `A1`),
//!   SMPTE `HH:MM:SS:FF` frame counter, icon-driven step/transport bar (`skipStart`, `stepBack`,
//!   `play`/`pause`, `stepForward`, `skipEnd`, `razor`), interactive `muteSpeaker` & `soloHeadphones`
//!   headers, In/Out range brackets, filmstrip tiles, clip selection, and real `craft_audio::peaks::Peaks`
//!   `[min, max]` waveforms.
//! - [`TimelineFlavor::EffectCraft`] (`02 Animations`, `03 Lyrics`, `04 Motion`): Composition layer
//!   stack, `prevKeyframe` / `nextKeyframe` SVG stepper, selectable layer duration bars, and diamond keyframes.

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, paint_filmbar_frame, paint_local_image};
use crate::icons::icon_image;
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, status_pill};

const TRACK_HEADER_W: f32 = 142.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineFlavor {
    FilmCraft,
    EffectCraft,
}

#[derive(Clone, Debug)]
pub struct CraftTimelineSpan {
    pub id: String,
    pub start_sec: f64,
    pub end_sec: f64,
    pub label: String,
}

impl CraftTimelineSpan {
    pub fn new(start_sec: f64, end_sec: f64, label: impl Into<String>) -> Self {
        Self { id: String::new(), start_sec, end_sec, label: label.into() }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }
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

pub struct StandardTimelineSpec<'a> {
    pub id_prefix: &'a str,
    pub flavor: TimelineFlavor,
    pub total_dur_sec: f64,
    pub fps: f64,
    pub in_out: Option<(f64, f64)>,
}

#[derive(Clone, Debug, Default)]
pub struct TimelineOutcome {
    pub scrubbed: bool,
    pub clicked_span: Option<(usize, String, f64)>,
    pub toggled_play: bool,
    pub razor_at: Option<f64>,
}

pub fn format_smpte_timecode(sec: f64, fps: f64) -> String {
    let clamped = if sec.is_finite() && sec > 0.0 { sec } else { 0.0 };
    let rate = fps.round().max(1.0) as u64;
    let total_frames = (clamped * (rate as f64)).floor() as u64;
    let ff = total_frames % rate;
    let total_secs = total_frames / rate;
    let ss = total_secs % 60;
    let mm = (total_secs / 60) % 60;
    let hh = total_secs / 3600;
    format!("{hh:02}:{mm:02}:{ss:02}:{ff:02}")
}

fn format_ruler_tick(sec: f64, dur: f64, flavor: TimelineFlavor, fps: f64) -> String {
    if flavor == TimelineFlavor::FilmCraft && dur >= 8.0 {
        let mins = (sec / 60.0).floor() as u64;
        let secs = (sec % 60.0).floor() as u64;
        let ff = ((sec.fract()) * fps.max(1.0)).floor() as u64;
        format!("{mins:02}:{secs:02}:{ff:02}")
    } else if dur <= 12.0 {
        format!("{sec:.1}s")
    } else {
        let mins = (sec / 60.0).floor() as u64;
        let rem = sec - (mins as f64) * 60.0;
        format!("{mins}:{rem:04.1}")
    }
}

fn choose_major_step(dur: f64, width_px: f32) -> f64 {
    let target_ticks = ((width_px / 92.0) as usize).clamp(4, 12) as f64;
    let raw = dur / target_ticks;
    for candidate in [0.25, 0.5, 1.0, 2.0, 2.5, 5.0, 10.0, 15.0, 30.0, 60.0] {
        if candidate >= raw {
            return candidate;
        }
    }
    60.0
}

/// Render exact `[min, max]` sample columns produced by `craft_audio::peaks::Peaks` (`song.waveform`).
fn paint_craft_audio_peaks(ui: &mut Ui, rect: Rect, color: Color32, waveform_doc: &Value, span_start_sec: f64, span_end_sec: f64, total_dur: f64) {
    let mid_y = rect.center().y;
    let Some(peaks) = waveform_doc["peaks"].as_array().filter(|a| !a.is_empty()) else {
        ui.painter().line_segment([Pos2::new(rect.min.x, mid_y), Pos2::new(rect.max.x, mid_y)], Stroke::new(1.0, color.gamma_multiply(0.45)));
        return;
    };
    let wf_dur = waveform_doc["duration"].as_f64().unwrap_or(total_dur).max(0.1);
    let n_bins = peaks.len();
    let cols = ((rect.width() / 2.6) as usize).clamp(8, 320);
    let half_h = (rect.height() * 0.46).max(2.0);
    let stroke = Stroke::new(1.5, color.gamma_multiply(0.88));

    for col in 0..cols {
        let frac = (col as f64 + 0.5) / (cols as f64);
        let t_sec = span_start_sec + frac * (span_end_sec - span_start_sec).max(0.05);
        let bin_idx = (((t_sec / wf_dur).clamp(0.0, 1.0)) * ((n_bins - 1) as f64)).round() as usize;
        let pair = &peaks[bin_idx.min(n_bins - 1)];
        let mn = pair.get(0).and_then(Value::as_f64).unwrap_or(0.0).clamp(-1.0, 1.0) as f32;
        let mx = pair.get(1).and_then(Value::as_f64).unwrap_or(0.0).clamp(-1.0, 1.0) as f32;
        let y_top = mid_y - (mx.max(0.03) * half_h);
        let y_bot = mid_y - (mn.min(-0.03) * half_h);
        let x = rect.min.x + (frac as f32) * rect.width();
        ui.painter().line_segment([Pos2::new(x, y_top), Pos2::new(x, y_bot)], stroke);
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

#[allow(clippy::cognitive_complexity)]
fn draw_timeline_header_bar(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    spec: &StandardTimelineSpec<'_>,
    playhead_sec: &mut f64,
    lanes: &[CraftTimelineLane<'_>],
    out: &mut TimelineOutcome,
) {
    let dur = spec.total_dur_sec.max(1.0);
    let fps = spec.fps.max(1.0);
    let frame_dt = 1.0 / fps;
    let any_dirty = lanes.iter().any(|l| l.dirty);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let (mode_lbl, mode_col) = match spec.flavor {
            TimelineFlavor::FilmCraft => ("FILMCRAFT", ctx.pal.accent_emerald),
            TimelineFlavor::EffectCraft => ("EFFECTCRAFT", ctx.pal.accent_sky),
        };
        status_pill(ui, ctx.pal, mode_lbl, mode_col);

        if spec.flavor == TimelineFlavor::FilmCraft {
            ui.label(
                RichText::new(format!("{} / {}", format_smpte_timecode(*playhead_sec, fps), format_smpte_timecode(dur, fps)))
                    .monospace()
                    .size(10.0)
                    .color(ctx.pal.accent_emerald),
            );
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-tl-home", spec.id_prefix), "skipStart", "Jump to sequence start (Home)"))
                .clicked()
            {
                *playhead_sec = 0.0;
                out.scrubbed = true;
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-tl-step-b", spec.id_prefix), "stepBack", "Step back 1 frame (Left)"))
                .clicked()
            {
                *playhead_sec = (*playhead_sec - frame_dt).max(0.0);
                out.scrubbed = true;
            }
            let play_ico = if ctx.state.audio_playing { "pause" } else { "play" };
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::icon_only(&format!("{}-tl-play", spec.id_prefix), play_ico, "Play / Pause sequence (Space)").primary(),
            )
            .clicked()
            {
                out.toggled_play = true;
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-tl-step-f", spec.id_prefix), "stepForward", "Step forward 1 frame (Right)"))
                .clicked()
            {
                *playhead_sec = (*playhead_sec + frame_dt).min(dur);
                out.scrubbed = true;
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-tl-end", spec.id_prefix), "skipEnd", "Jump to sequence end (End)")).clicked()
            {
                *playhead_sec = dur;
                out.scrubbed = true;
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-tl-razor", spec.id_prefix), "razor", "Razor split clip at playhead (⌘K)"))
                .clicked()
            {
                out.razor_at = Some(*playhead_sec);
            }
        } else {
            let cur_frame = ((*playhead_sec * fps).round().max(0.0)) as u64;
            ui.label(RichText::new(format!("{:.2}s · f{cur_frame:04} ({fps:.0}fps)", *playhead_sec)).monospace().size(9.8).color(ctx.pal.accent_emerald));
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-ec-prevk", spec.id_prefix), "prevKeyframe", "Step to previous keyframe"))
                .clicked()
            {
                *playhead_sec = (*playhead_sec - 0.25).max(0.0);
                out.scrubbed = true;
            }
            let play_ico = if ctx.state.audio_playing { "pause" } else { "play" };
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-ec-play", spec.id_prefix), play_ico, "Play / Pause composition").primary())
                .clicked()
            {
                out.toggled_play = true;
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only(&format!("{}-ec-nextk", spec.id_prefix), "nextKeyframe", "Step to next keyframe"))
                .clicked()
            {
                *playhead_sec = (*playhead_sec + 0.25).min(dur);
                out.scrubbed = true;
            }
        }

        if any_dirty {
            let sync_id = format!("{}-sync-links", spec.id_prefix);
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new(&sync_id, "refresh", "Sync Comps", "Re-render dirty .ecproj tracks and swap FilmCraft footage").primary(),
            )
            .clicked()
            {
                ctx.exec("video.sync_links", json!({"render": true}), Some("Synced modified EffectCraft tracks into FilmCraft"));
                ctx.state.dirty_tracks.clear();
            }
        }
    });
}

fn mute_slot_index(code: &str) -> Option<usize> {
    match code {
        "V1" => Some(0),
        "V2" => Some(1),
        "V3" => Some(2),
        "V4" => Some(3),
        "A1" => Some(4),
        _ => None,
    }
}

#[allow(clippy::cognitive_complexity)]
fn draw_lane_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>, flavor: TimelineFlavor, lane: &CraftTimelineLane<'_>, row_h: f32) {
    ui.allocate_ui_with_layout(Vec2::new(TRACK_HEADER_W, row_h), Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let swatch_col = if lane.dirty { ctx.pal.accent_amber } else { lane.color };
        if flavor == TimelineFlavor::FilmCraft {
            let (badge_r, _) = ui.allocate_exact_size(Vec2::new(21.0, (row_h - 4.0).clamp(13.0, 17.0)), Sense::hover());
            ui.painter().rect(
                badge_r,
                CornerRadius::same(3),
                swatch_col.gamma_multiply(0.22),
                Stroke::new(1.0, swatch_col.gamma_multiply(0.75)),
                StrokeKind::Inside,
            );
            ui.painter().text(badge_r.center(), Align2::CENTER_CENTER, lane.code, FontId::monospace(8.0), swatch_col);

            let m_idx = mute_slot_index(lane.code);
            let is_muted = m_idx.is_some_and(|i| ctx.state.studio.muted_tracks[i]);
            let is_solo = ctx.state.studio.solo_track.as_deref() == Some(lane.code);

            let (m_r, m_resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::click());
            ui.painter().rect_filled(m_r, CornerRadius::same(3), if is_muted { ctx.pal.danger } else { ctx.pal.bg_input });
            icon_image("muteSpeaker", 11.0, if is_muted { Color32::WHITE } else { ctx.pal.text_muted })
                .paint_at(ui, Rect::from_center_size(m_r.center(), Vec2::splat(11.0)));
            if m_resp.on_hover_text(format!("Mute track {}", lane.code)).clicked() {
                if let Some(i) = m_idx {
                    ctx.state.studio.muted_tracks[i] = !is_muted;
                }
                ctx.exec("video.finish", json!({"op": "track_patch", "track": lane.code, "muted": !is_muted, "patched": true}), None);
            }

            let (s_r, s_resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::click());
            ui.painter().rect_filled(s_r, CornerRadius::same(3), if is_solo { ctx.pal.accent_amber } else { ctx.pal.bg_input });
            icon_image("soloHeadphones", 11.0, if is_solo { Color32::from_rgb(14, 18, 24) } else { ctx.pal.text_muted })
                .paint_at(ui, Rect::from_center_size(s_r.center(), Vec2::splat(11.0)));
            if s_resp.on_hover_text(format!("Solo track {}", lane.code)).clicked() {
                ctx.state.studio.solo_track = if is_solo { None } else { Some(lane.code.into()) };
                ctx.exec("video.finish", json!({"op": "track_patch", "track": lane.code, "solo": !is_solo, "patched": true}), None);
            }
        } else {
            let (bar_r, _) = ui.allocate_exact_size(Vec2::new(4.0, (row_h - 5.0).max(10.0)), Sense::hover());
            ui.painter().rect_filled(bar_r, CornerRadius::same(2), swatch_col);
            let (eye_r, _) = ui.allocate_exact_size(Vec2::new(13.0, 13.0), Sense::hover());
            icon_image("eye", 11.0, swatch_col).paint_at(ui, Rect::from_center_size(eye_r.center(), Vec2::splat(11.0)));
        }
        let title_txt = if lane.dirty { format!("{}*", lane.title) } else { lane.title.to_owned() };
        ui.label(RichText::new(title_txt).monospace().size(8.6).color(ctx.pal.text_secondary));
    });
}

#[allow(clippy::cognitive_complexity)]
fn draw_lane_spans_and_keys(
    ui: &mut Ui,
    ctx: &ViewCtx<'_>,
    lane_idx: usize,
    lane: &CraftTimelineLane<'_>,
    lane_rect: Rect,
    dur: f64,
    out: &mut TimelineOutcome,
) {
    let bar_w = lane_rect.width();
    let row_h = lane_rect.height();
    let border_col = if lane.dirty { ctx.pal.accent_amber } else { lane.color.gamma_multiply(0.52) };
    ui.painter().rect(lane_rect, CornerRadius::same(4), ctx.pal.bg_input, Stroke::new(1.0, border_col), StrokeKind::Inside);
    ui.painter().line_segment(
        [Pos2::new(lane_rect.min.x + 2.0, lane_rect.center().y), Pos2::new(lane_rect.max.x - 2.0, lane_rect.center().y)],
        Stroke::new(0.7, ctx.pal.border_subtle),
    );

    let selected_id = ctx.state.studio.selected_clip_id.map(|id| id.to_string());

    if !lane.spans.is_empty() {
        for (sp_idx, sp) in lane.spans.iter().enumerate() {
            let x0 = lane_rect.min.x + ((sp.start_sec / dur).clamp(0.0, 1.0) as f32) * bar_w;
            let span_len = (sp.end_sec - sp.start_sec).max(0.15);
            let cw = ((span_len / dur) as f32 * bar_w).max(12.0);
            let cr = Rect::from_min_size(Pos2::new(x0, lane_rect.min.y + 2.0), Vec2::new(cw.min((lane_rect.max.x - x0).max(4.0)), row_h - 4.0));
            let sp_hit = ui.interact(cr, ui.scope_id().with(("tl_span", lane_idx, sp_idx)), Sense::click());
            let is_sel = selected_id.as_deref().is_some_and(|sid| !sp.id.is_empty() && sp.id == sid);
            let fill_alpha = if is_sel {
                0.46
            } else if sp_hit.hovered() {
                0.36
            } else {
                0.24
            };
            let stroke_col = if is_sel { ctx.pal.accent_amber } else { lane.color.gamma_multiply(0.85) };
            ui.painter().rect(
                cr,
                CornerRadius::same(3),
                lane.color.gamma_multiply(fill_alpha),
                Stroke::new(
                    if is_sel {
                        1.8
                    } else if sp_hit.hovered() {
                        1.4
                    } else {
                        1.0
                    },
                    stroke_col,
                ),
                StrokeKind::Inside,
            );
            if sp_hit.clicked() {
                let id_or_lbl = if sp.id.is_empty() { sp.label.clone() } else { sp.id.clone() };
                out.clicked_span = Some((lane_idx, id_or_lbl, sp.start_sec));
            }
            if lane.waveform && cr.width() >= 18.0 {
                paint_craft_audio_peaks(ui, cr.shrink2(Vec2::new(2.0, 1.5)), lane.color, &ctx.state.cache.song_waveform, sp.start_sec, sp.end_sec, dur);
            }
            if cr.width() >= 26.0 && !sp.label.is_empty() {
                let max_c = ((cr.width() / 5.6) as usize).max(3);
                let lbl = if sp.label.chars().count() > max_c {
                    let mut s: String = sp.label.chars().take(max_c.saturating_sub(1)).collect();
                    s.push('…');
                    s
                } else {
                    sp.label.clone()
                };
                ui.painter().text(Pos2::new(cr.min.x + 5.0, cr.center().y), Align2::LEFT_CENTER, &lbl, FontId::monospace(8.5), ctx.pal.text_primary);
            }
        }
    } else if lane.waveform {
        paint_craft_audio_peaks(ui, lane_rect.shrink2(Vec2::new(4.0, 2.5)), lane.color, &ctx.state.cache.song_waveform, 0.0, dur, dur);
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

#[allow(clippy::cognitive_complexity)]
pub fn draw_standard_timeline(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    spec: StandardTimelineSpec<'_>,
    playhead_sec: &mut f64,
    lanes: &[CraftTimelineLane<'_>],
) -> TimelineOutcome {
    let dur = spec.total_dur_sec.max(1.0);
    let fps = spec.fps.max(1.0);
    let mut out = TimelineOutcome::default();

    let start_min = ui.cursor().min;
    draw_timeline_header_bar(ui, ctx, &spec, playhead_sec, lanes, &mut out);
    ui.add_space(2.0);

    let mut ruler_track_rect = Rect::NOTHING;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(TRACK_HEADER_W, 18.0), Layout::left_to_right(Align::Center), |ui| {
            let readout = format_ruler_tick(*playhead_sec, dur, spec.flavor, fps);
            ui.label(RichText::new(readout).monospace().size(9.2).color(ctx.pal.accent_emerald));
        });
        let bar_w = (ui.available_width() - 2.0).max(100.0);
        let (rrect, rresp) = ui.allocate_exact_size(Vec2::new(bar_w, 18.0), Sense::click_and_drag());
        ruler_track_rect = rrect;
        if (rresp.clicked() || rresp.dragged())
            && let Some(pos) = rresp.interact_pointer_pos()
        {
            let frac = ((pos.x - rrect.min.x) / rrect.width().max(1.0)).clamp(0.0, 1.0) as f64;
            *playhead_sec = frac * dur;
            out.scrubbed = true;
        }
        ui.painter().rect_filled(rrect, CornerRadius::same(3), Color32::from_rgb(0x0c, 0x11, 0x18));
        if let Some((in_s, out_s)) = spec.in_out
            && out_s > in_s
        {
            let ix0 = rrect.min.x + ((in_s / dur).clamp(0.0, 1.0) as f32) * bar_w;
            let ix1 = rrect.min.x + ((out_s / dur).clamp(0.0, 1.0) as f32) * bar_w;
            let io_rect = Rect::from_min_max(Pos2::new(ix0, rrect.min.y), Pos2::new(ix1, rrect.max.y));
            ui.painter().rect_filled(io_rect, CornerRadius::same(2), ctx.pal.accent_sky.gamma_multiply(0.16));
        }
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
            if is_major && x + 32.0 <= rrect.max.x + 12.0 {
                ui.painter().text(
                    Pos2::new(x + 2.5, rrect.min.y + 6.5),
                    Align2::LEFT_CENTER,
                    format_ruler_tick(t, dur, spec.flavor, fps),
                    FontId::monospace(8.4),
                    ctx.pal.text_muted,
                );
            }
            t += minor_step;
        }
        ctx.reg.record(format!("{}-ruler", spec.id_prefix), "Timeline Ruler", &rresp, Some(format!("{:.2}", *playhead_sec)));
    });

    ui.spacing_mut().item_spacing.y = 3.0;
    let mut bottom_lane_max_y = ruler_track_rect.max.y;

    for (lane_idx, lane) in lanes.iter().enumerate() {
        ui.horizontal(|ui| {
            let row_h = if lane.filmbar { 25.0 } else { 20.0 };
            draw_lane_header(ui, ctx, spec.flavor, lane, row_h);

            let bar_w = (ui.available_width() - 2.0).max(100.0);
            let (lane_rect, lresp) = ui.allocate_exact_size(Vec2::new(bar_w, row_h), Sense::click_and_drag());
            bottom_lane_max_y = bottom_lane_max_y.max(lane_rect.max.y);

            if lane.filmbar && lane.spans.is_empty() {
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
                draw_lane_spans_and_keys(ui, ctx, lane_idx, lane, lane_rect, dur, &mut out);
            }

            if let Some((_, _, span_sec)) = &out.clicked_span {
                *playhead_sec = *span_sec;
                out.scrubbed = true;
            } else if (lresp.clicked() || lresp.dragged())
                && let Some(pos) = lresp.interact_pointer_pos()
            {
                let frac = ((pos.x - lane_rect.min.x) / lane_rect.width().max(1.0)).clamp(0.0, 1.0) as f64;
                *playhead_sec = frac * dur;
                out.scrubbed = true;
            }
            ctx.reg.record(format!("{}-lane-{}", spec.id_prefix, lane.code), lane.title, &lresp, None);
        });
    }

    if ruler_track_rect.is_positive() {
        let ph_x = ruler_track_rect.min.x + ((*playhead_sec / dur).clamp(0.0, 1.0) as f32) * ruler_track_rect.width();
        let cap_col = if spec.flavor == TimelineFlavor::FilmCraft { ctx.pal.accent_emerald } else { ctx.pal.accent_sky };
        let cap = Rect::from_center_size(Pos2::new(ph_x, ruler_track_rect.min.y + 5.5), Vec2::new(9.0, 9.0));
        ui.painter().rect_filled(cap, CornerRadius::same(3), cap_col);
        ui.painter().line_segment([Pos2::new(ph_x, cap.max.y), Pos2::new(ph_x, bottom_lane_max_y)], Stroke::new(1.6, cap_col));
        let full_tl_rect = Rect::from_min_max(start_min, Pos2::new(ruler_track_rect.max.x, bottom_lane_max_y));
        let tl_resp = ui.interact(full_tl_rect, ui.scope_id().with(("std_tl_root", spec.id_prefix)), Sense::hover());
        ctx.reg.record(format!("{}-timeline", spec.id_prefix), "Standard Craft Timeline", &tl_resp, Some(format!("{:.2}", *playhead_sec)));
    }
    out
}

pub fn draw_craft_timeline(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    id_prefix: &str,
    total_dur_sec: f64,
    playhead_sec: &mut f64,
    lanes: &[CraftTimelineLane<'_>],
) -> bool {
    let outcome = draw_standard_timeline(
        ui,
        ctx,
        StandardTimelineSpec { id_prefix, flavor: TimelineFlavor::EffectCraft, total_dur_sec, fps: 24.0, in_out: None },
        playhead_sec,
        lanes,
    );
    outcome.scrubbed
}
