import React from "react";
import { Dial, Slider } from "../../components/controls";
import {
  EasingPreset,
  LineDesign,
  MotionProperty,
  MotionSpec,
  MovePreset,
  StrokePreset,
  TextPresetItem,
  WordItem,
} from "./types";

const ROTATION = "transform/rotation";

export interface LyricsMotionMarksProps {
  mode: "design" | "motion";
  item?: WordItem;
  design: LineDesign;
  activeItemIndex: number;
  patchItem: (patch: Partial<WordItem>) => void;
  onUpdateDesign: (fn: (draft: LineDesign) => void) => void;
  moves: MovePreset[];
  easings: EasingPreset[];
  textPresets: (string | TextPresetItem)[];
  strokes: StrokePreset[];
  motionProps: MotionProperty[];
  compBindingLabel: string;
  motionSpec: MotionSpec;
  setMotionSpec: React.Dispatch<React.SetStateAction<MotionSpec>>;
  defaultMotionSpec: (property: string) => MotionSpec;
  motionBusy: boolean;
  applyNativeKeyframes: () => Promise<void>;
  drawMode: boolean;
  onToggleDrawMode: () => void;
  brushColor: string;
  onBrushColorChange: (c: string) => void;
  brushWidth: number;
  onBrushWidthChange: (w: number) => void;
  onToast: (msg: string) => void;
}

export const LyricsMotionMarks: React.FC<LyricsMotionMarksProps> = ({
  mode,
  item,
  design,
  activeItemIndex,
  patchItem,
  onUpdateDesign,
  moves,
  easings,
  textPresets,
  strokes,
  motionProps,
  compBindingLabel,
  motionSpec,
  setMotionSpec,
  defaultMotionSpec,
  motionBusy,
  applyNativeKeyframes,
  drawMode,
  onToggleDrawMode,
  brushColor,
  onBrushColorChange,
  brushWidth,
  onBrushWidthChange,
  onToast,
}) => (
  <>
      {/* 3. Motion Styles & EffectCraft Keyframes (Motion Look) */}
      <details
        id="lyrics-motion-section"
        className="panel motion-section"
        data-look="motion"
        open={mode === "motion" || true}
      >
        <summary className="panel-head">
          <span className="motion-section-icon" aria-hidden="true">
            <svg viewBox="0 0 16 16">
              <path d="M2 12c3-6 5-6 8 0s5 0 4-6" />
            </svg>
          </span>
          <span className="title">Motion</span>
          <span className="meta">{item?.move ?? "pop"}</span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div className="lyrics-moves-grid">
            {moves.map((mv) => (
              <button
                key={mv.id}
                id={`words-move-${mv.id}`}
                type="button"
                className={`lyrics-move-card ${item?.move === mv.id ? "active" : ""}`}
                title={mv.does}
                onClick={() => patchItem({ move: mv.id })}
              >
                <span className="move-title">{mv.name}</span>
                <span className="move-desc">{mv.does}</span>
              </button>
            ))}
          </div>

          <div className="lyrics-sublabel">EffectCraft Text Animator Presets</div>
          <div className="lyrics-warp-pills">
            {textPresets.slice(0, 8).map((raw) => {
              const id =
                typeof raw === "string"
                  ? raw
                  : raw.id || raw.match_name || raw.name || "typewriter";
              const label =
                typeof raw === "string"
                  ? raw.replace(/_/g, " ")
                  : raw.name || id;
              return (
                <button
                  key={id}
                  id={`words-ec-preset-${id}`}
                  type="button"
                  className={`lyrics-chip ${item?.text_preset === id ? "active" : ""}`}
                  onClick={() => patchItem({ text_preset: id })}
                >
                  ✦ {label}
                </button>
              );
            })}
          </div>

          {/* Native EffectCraft Keyframe Curve Controls */}
          <div className="lyrics-ecproj-box">
            <div className="lyrics-ecproj-head">
              <strong>EffectCraft Keyframes</strong>
              <span className="muted">{compBindingLabel}</span>
            </div>
            <label className="inspector-field">
              <span>Channel</span>
              <select
                id="word-motion-property"
                value={motionSpec.property}
                onChange={(e) => setMotionSpec(defaultMotionSpec(e.target.value))}
              >
                {motionProps.map((mp) => (
                  <option key={mp.path} value={mp.path}>
                    {mp.name ?? mp.path}
                  </option>
                ))}
              </select>
            </label>
            <label className="inspector-field">
              <span>AniCraft Easing</span>
              <select
                id="word-motion-easing"
                value={motionSpec.easing}
                onChange={(e) =>
                  setMotionSpec((s) => ({ ...s, easing: e.target.value }))
                }
              >
                {easings.map((ea) => (
                  <option key={ea.id} value={ea.id}>
                    {ea.name}
                    {ea.bezier
                      ? ` (${ea.bezier.map((n) => n.toFixed(2)).join(", ")})`
                      : ""}
                  </option>
                ))}
              </select>
            </label>
            <label className="inspector-field">
              <span>Symbol Timeline</span>
              <select
                id="word-motion-loop-mode"
                value={motionSpec.loopMode}
                onChange={(e) =>
                  setMotionSpec((s) => ({
                    ...s,
                    loopMode: e.target.value as MotionSpec["loopMode"],
                  }))
                }
              >
                <option value="loop">Graphic · Loop</option>
                <option value="play_once">Graphic · Play Once</option>
                <option value="single_frame">Graphic · Single Frame</option>
              </select>
            </label>
            <div className="lyrics-dial-grid">
              <div className="craft-number" data-control="dial">
                <label className="craft-number-title" htmlFor="word-motion-delay-dial">
                  <span className="craft-control-label">Delay</span>
                  <output className="craft-number-readout">
                    {motionSpec.delay.toFixed(2)}s
                  </output>
                </label>
                <span className="craft-number-body">
                  <Dial
                    id="word-motion-delay"
                    label="Delay"
                    value={motionSpec.delay}
                    min={0}
                    max={5}
                    step={0.05}
                    unit="s"
                    onChange={(v) => setMotionSpec((s) => ({ ...s, delay: v }))}
                  />
                </span>
              </div>
              <div className="craft-number" data-control="dial">
                <label className="craft-number-title" htmlFor="word-motion-duration-dial">
                  <span className="craft-control-label">Duration</span>
                  <output className="craft-number-readout">
                    {motionSpec.duration.toFixed(2)}s
                  </output>
                </label>
                <span className="craft-number-body">
                  <Dial
                    id="word-motion-duration"
                    label="Duration"
                    value={motionSpec.duration}
                    min={0.1}
                    max={6}
                    step={0.05}
                    unit="s"
                    onChange={(v) => setMotionSpec((s) => ({ ...s, duration: v }))}
                  />
                </span>
              </div>
              {motionSpec.property === ROTATION ? (
                <div className="craft-number" data-control="dial">
                  <label className="craft-number-title" htmlFor="word-motion-speed-dial">
                    <span className="craft-control-label">Speed</span>
                    <output className="craft-number-readout">
                      {Math.round(motionSpec.speed)}°/s
                    </output>
                  </label>
                  <span className="craft-number-body">
                    <Dial
                      id="word-motion-speed"
                      label="Speed"
                      value={motionSpec.speed}
                      min={-360}
                      max={360}
                      step={5}
                      unit="°/s"
                      onChange={(v) => setMotionSpec((s) => ({ ...s, speed: v }))}
                    />
                  </span>
                </div>
              ) : null}
            </div>
            <button
              id="word-motion-apply"
              type="button"
              className="primary"
              disabled={motionBusy}
              onClick={() => void applyNativeKeyframes()}
            >
              {motionBusy ? "Writing to lyrics.ecproj…" : "Apply Keyframes to lyrics.ecproj"}
            </button>
          </div>
        </div>
      </details>

      {/* 4. Drawn Marks & Brushes (Chalk Look) */}
      <details
        id="lyrics-draw-section"
        className="panel motion-section"
        data-look="chalk"
      >
        <summary className="panel-head">
          <span className="motion-section-icon" aria-hidden="true">
            <svg viewBox="0 0 16 16">
              <path d="M12.1 2.3a1 1 0 0 1 1.4 0l.2.2a1 1 0 0 1 0 1.4L5.5 12.1 2 13l.9-3.5L11.1 1.3z" />
            </svg>
          </span>
          <span className="title">Marks</span>
          <span className="meta">{ design.draws.length } marks</span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div className="lyrics-warp-pills">
            <button
              id="words-inspector-draw-toggle"
              type="button"
              className={`lyrics-chip ${drawMode ? "active" : ""}`}
              onClick={onToggleDrawMode}
            >
              {drawMode ? "✎ Drawing on Stage (Active)" : "✎ Draw Freehand on Stage"}
            </button>
            {design.draws.length > 0 && (
              <button
                id="words-clear-draws"
                type="button"
                className="ghost"
                onClick={() =>
                  onUpdateDesign((draft) => {
                    draft.draws = [];
                  })
                }
              >
                Clear Marks ({design.draws.length})
              </button>
            )}
          </div>

          <div className="lyrics-color-row">
            <label className="lyrics-color-picker">
              <span>Ink</span>
              <input
                id="words-brush-color"
                type="color"
                value={brushColor}
                onChange={(e) => onBrushColorChange(e.target.value)}
              />
            </label>
            <div className="lyrics-flex-1">
              <Slider
                id="words-brush-width"
                label="Brush Width"
                value={brushWidth}
                min={2}
                max={28}
                step={1}
                unit="px"
                onChange={onBrushWidthChange}
              />
            </div>
          </div>

          <div className="lyrics-sublabel">Vector Trim-Path Accents</div>
          <div className="lyrics-warp-pills">
            {strokes.map((st) => (
              <button
                key={st.id}
                id={`words-stroke-preset-${st.id}`}
                type="button"
                className="lyrics-chip"
                title={st.does}
                onClick={() => {
                  onUpdateDesign((draft) => {
                    const target = draft.items[activeItemIndex] ?? draft.items[0];
                    const bx = target ? target.x : 540;
                    const by = target ? target.y + (target.h ?? 130) - 16 : 620;
                    draft.draws.push({
                      name: st.name,
                      view_box: st.view_box,
                      x: bx,
                      y: by,
                      w: st.w,
                      h: st.h,
                      paths: [
                        {
                          d: st.d,
                          color: brushColor,
                          width: brushWidth,
                          from: 0,
                          to: st.draw || 0.8,
                        },
                      ],
                    });
                  });
                  onToast(`Added ${st.name} vector trim-path stroke`);
                }}
              >
                + {st.name}
              </button>
            ))}
          </div>
        </div>
      </details>

  </>
);
