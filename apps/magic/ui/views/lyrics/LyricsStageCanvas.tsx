import React from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  BackdropImage,
  effectiveOutlineColor,
  fontStackForLettering,
  LineDesign,
  LyricLineGroup,
  STAGE_H,
  STAGE_W,
  warpSvgPathD,
  WordItem,
} from "./types";

export interface LyricsStageCanvasProps {
  stageRef: React.RefObject<HTMLDivElement | null>;
  boxRefs: React.MutableRefObject<Map<number, HTMLDivElement>>;
  mode: "design" | "motion";
  onModeChange: (mode: "design" | "motion") => void;
  stageBgMode: "scene" | "paper";
  activeBackdrop: BackdropImage | null;
  ecFrameUrl: string | null;
  renderingEcFrame: boolean;
  previewEffectCraftStage: () => Promise<void>;
  drawMode: boolean;
  setDrawMode: React.Dispatch<React.SetStateAction<boolean>>;
  brushColor: string;
  brushWidth: number;
  activeDesign: LineDesign;
  activeItemIndex: number;
  updateActiveDesign: (fn: (draft: LineDesign) => void) => void;
  activeLine: LyricLineGroup | null;
  drawingPoints: [number, number][] | null;
  onStageDrawDown: (e: React.PointerEvent<SVGSVGElement>) => void;
  onStageDrawMove: (e: React.PointerEvent<SVGSVGElement>) => void;
  onStageDrawUp: () => void;
  startBoxPointerDrag: (
    e: React.PointerEvent,
    idx: number,
    kind: "move" | "rotate" | "resize" | "motion-target",
  ) => void;
}

export const LyricsStageCanvas: React.FC<LyricsStageCanvasProps> = ({
  stageRef,
  boxRefs,
  mode,
  onModeChange,
  stageBgMode,
  activeBackdrop,
  ecFrameUrl,
  renderingEcFrame,
  previewEffectCraftStage,
  drawMode,
  setDrawMode,
  brushColor,
  brushWidth,
  activeDesign,
  activeItemIndex,
  updateActiveDesign,
  activeLine,
  drawingPoints,
  onStageDrawDown,
  onStageDrawMove,
  onStageDrawUp,
  startBoxPointerDrag,
}) => (
          <div className="lyrics-stage-viewport">
            {/* 16:9 Interactive Stage */}
            <div
              ref={stageRef}
              id="words-stage"
              className={`lyrics-stage-canvas ${stageBgMode === "paper" ? "paper-mode" : "scene-mode"}`}
            >
              {/* Backdrop image when in Scene mode */}
              {stageBgMode === "scene" && activeBackdrop && (
                <img
                  className="lyrics-stage-backdrop-img"
                  src={convertFileSrc(activeBackdrop.path)}
                  alt=""
                  draggable={false}
                />
              )}

              {/* Optional live-rendered EffectCraft frame from lettering/lyrics.ecproj */}
              {ecFrameUrl && (
                <img
                  className="lyrics-stage-ecproj-overlay"
                  src={ecFrameUrl}
                  alt="EffectCraft live frame"
                  draggable={false}
                />
              )}

              {/* Floating Top Action Pills on Stage */}
              <div className="lyrics-stage-floating-pills">
                <button
                  id="words-pill-kinetic"
                  type="button"
                  className={`stage-pill ${mode === "motion" ? "active" : ""}`}
                  onClick={() =>
                    onModeChange(mode === "motion" ? "design" : "motion")
                  }
                >
                  ✦ Kinetic
                </button>
                <button
                  id="words-pill-draw"
                  type="button"
                  className={`stage-pill ${drawMode ? "active" : ""}`}
                  onClick={() => setDrawMode((v) => !v)}
                >
                  ✎ Draw
                </button>
                <button
                  id="words-pill-warp"
                  type="button"
                  className={`stage-pill ${
                    (activeDesign.items[activeItemIndex]?.warp_path ?? "none") !==
                    "none"
                      ? "active-amber"
                      : ""
                  }`}
                  onClick={() => {
                    const order: WordItem["warp_path"][] = [
                      "none",
                      "arc_up",
                      "arc_down",
                      "wave",
                      "circle",
                    ];
                    const cur =
                      activeDesign.items[activeItemIndex]?.warp_path ?? "none";
                    const next = order[(order.indexOf(cur) + 1) % order.length];
                    updateActiveDesign((draft) => {
                      const target =
                        draft.items[activeItemIndex] ?? draft.items[0];
                      if (target) target.warp_path = next;
                    });
                  }}
                >
                  ⤴ Warp
                </button>
                <button
                  id="words-pill-ec-render"
                  type="button"
                  className={`stage-pill ${ecFrameUrl ? "active-amber" : ""}`}
                  disabled={renderingEcFrame}
                  onClick={() => void previewEffectCraftStage()}
                >
                  {renderingEcFrame ? "Rendering EC…" : "✦ EC Render"}
                </button>
              </div>

              {/* Motion Trajectory Dashed Vectors & Drawn Strokes SVG Layer */}
              <svg
                id="words-draw-svg"
                viewBox={`0 0 ${STAGE_W} ${STAGE_H}`}
                className={`lyrics-stage-svg-overlay ${drawMode ? "drawing-enabled" : ""}`}
                onPointerDown={onStageDrawDown}
                onPointerMove={onStageDrawMove}
                onPointerUp={onStageDrawUp}
              >
                {/* Motion Trajectory Arrows for every item on stage */}
                {activeDesign.items.map((it, idx) => {
                  const bw = it.w ?? 640;
                  const bh = it.h ?? 150;
                  const cx = it.x + bw * 0.5;
                  const cy = it.y + bh * 0.5;
                  const dx = it.dx ?? 0;
                  const dy = it.dy ?? -36;
                  if (Math.hypot(dx, dy) < 4) return null;
                  const tx = cx + dx;
                  const ty = cy + dy;
                  return (
                    <g key={`traj-${idx}`} className="lyrics-motion-trajectory">
                      <path
                        d={`M ${cx} ${cy} Q ${(cx + tx) * 0.5} ${Math.min(cy, ty) - 36} ${tx} ${ty}`}
                        fill="none"
                        stroke="#2AF5D6"
                        strokeWidth={3.5}
                        strokeDasharray="10 8"
                      />
                      <circle cx={tx} cy={ty} r={8} fill="#2AF5D6" />
                    </g>
                  );
                })}

                {/* Saved Vector / Brush Strokes */}
                {activeDesign.draws.map((dr, di) => {
                  const tx = dr.x ?? 0;
                  const ty = dr.y ?? 0;
                  return (
                    <g key={`draw-${di}`} transform={`translate(${tx}, ${ty})`}>
                      {dr.paths.map((p, pi) => (
                        <path
                          key={pi}
                          d={p.d}
                          fill="none"
                          stroke={p.color || "#FF9E3D"}
                          strokeWidth={p.width || 8}
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                      ))}
                    </g>
                  );
                })}

                {/* Active Freehand Stroke In Progress */}
                {drawingPoints && drawingPoints.length > 1 && (
                  <path
                    d={drawingPoints
                      .map((pt, i) => `${i === 0 ? "M" : "L"} ${pt[0]} ${pt[1]}`)
                      .join(" ")}
                    fill="none"
                    stroke={brushColor}
                    strokeWidth={brushWidth}
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                )}
              </svg>

              {/* Interactive Draggable / Rotatable / Skewable / Warpable Word Boxes */}
              {activeDesign.items.map((item, idx) => {
                const isSelected = idx === activeItemIndex;
                const text = (item.word_ids ?? [])
                  .map(
                    (wid) =>
                      activeLine?.words.find((w) => w.word_id === wid)?.text,
                  )
                  .filter(Boolean)
                  .join(" ");
                const bw = item.w ?? 640;
                const bh = item.h ?? 150;
                const leftPct = (item.x / STAGE_W) * 100;
                const topPct = (item.y / STAGE_H) * 100;
                const widthPct = (bw / STAGE_W) * 100;
                const heightPct = (bh / STAGE_H) * 100;
                const font = fontStackForLettering(item.lettering);
                const outline = effectiveOutlineColor(item);
                const strokePx = outline ? item.stroke_width ?? 6 : 0;
                const warpD = warpSvgPathD(item.warp_path, bw, bh);
                const warpId = `stage-warp-path-${idx}`;

                return (
                  <div
                    key={`${item.word_ids.join("_")}_${idx}`}
                    ref={(el) => {
                      if (el) boxRefs.current.set(idx, el);
                      else boxRefs.current.delete(idx);
                    }}
                    id={`word-box-${idx}`}
                    className={`lyrics-stage-word-box ${isSelected ? "selected" : ""}`}
                    style={{
                      left: `${leftPct}%`,
                      top: `${topPct}%`,
                      width: `${widthPct}%`,
                      height: `${heightPct}%`,
                      transform: `rotate(${item.rotate ?? 0}deg) skewX(${item.skew ?? 0}deg) scale(${(item.scale_x ?? 100) / 100}, ${(item.scale_y ?? 100) / 100})`,
                    }}
                    onPointerDown={(e) => startBoxPointerDrag(e, idx, "move")}
                  >
                    <svg
                      viewBox={`0 0 ${bw} ${bh}`}
                      className="lyrics-word-box-svg"
                    >
                      {warpD && (
                        <defs>
                          <path id={warpId} d={warpD} />
                        </defs>
                      )}
                      {warpD ? (
                        <>
                          {isSelected && (
                            <path
                              d={warpD}
                              fill="none"
                              stroke="rgba(255, 190, 92, 0.55)"
                              strokeWidth={2}
                              strokeDasharray="6 6"
                            />
                          )}
                          <text
                            fill={item.color || "#D95B38"}
                            stroke={outline ?? "none"}
                            strokeWidth={strokePx * 2}
                            letterSpacing={item.tracking ?? 0}
                            style={{
                              fontFamily: font.fontFamily,
                              fontWeight: font.fontWeight,
                              fontStyle: font.fontStyle,
                              fontSize: `${item.size ?? 88}px`,
                              paintOrder: "stroke fill",
                            }}
                          >
                            <textPath
                              href={`#${warpId}`}
                              startOffset="50%"
                              textAnchor="middle"
                            >
                              {text}
                            </textPath>
                          </text>
                        </>
                      ) : (
                        <text
                          x={bw * 0.5}
                          y={bh * 0.68}
                          textAnchor="middle"
                          fill={item.color || "#D95B38"}
                          stroke={outline ?? "none"}
                          strokeWidth={strokePx * 2}
                          letterSpacing={item.tracking ?? 0}
                          style={{
                            fontFamily: font.fontFamily,
                            fontWeight: font.fontWeight,
                            fontStyle: font.fontStyle,
                            fontSize: `${item.size ?? 88}px`,
                            paintOrder: "stroke fill",
                          }}
                        >
                          {text}
                        </text>
                      )}
                    </svg>

                    {/* Rotation Stem & Resize Handles when Selected */}
                    {isSelected && (
                      <>
                        <button
                          id={`word-box-rotate-${idx}`}
                          type="button"
                          className="word-handle-rotate"
                          title="Drag horizontally to rotate (-360°..360°)"
                          onPointerDown={(e) =>
                            startBoxPointerDrag(e, idx, "rotate")
                          }
                        />
                        <button
                          id={`word-box-resize-${idx}`}
                          type="button"
                          className="word-handle-resize se"
                          title="Drag to resize word box"
                          onPointerDown={(e) =>
                            startBoxPointerDrag(e, idx, "resize")
                          }
                        />
                        {mode === "motion" && (
                          <button
                            id={`word-box-motion-target-${idx}`}
                            type="button"
                            className="word-handle-motion"
                            title="Drag to set motion trajectory offset"
                            onPointerDown={(e) =>
                              startBoxPointerDrag(e, idx, "motion-target")
                            }
                          />
                        )}
                      </>
                    )}
                  </div>
                );
              })}

              <div className="lyrics-stage-hint-pill">
                {mode === "motion"
                  ? "Drag any word to adjust spatial trajectory · Use dials on right to keyframe in lyrics.ecproj"
                  : "Drag to position · Top circle to rotate 360° · Corner to resize"}
              </div>
            </div>
          </div>

);
