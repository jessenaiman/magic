import React from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  BackdropImage,
  buildDefaultLineDesign,
  effectiveOutlineColor,
  fontStackForLettering,
  LineDesign,
  LyricLineGroup,
  STAGE_H,
  STAGE_W,
} from "./types";

export interface LyricsMomentRailProps {
  loading: boolean;
  allLineGroups: LyricLineGroup[];
  activeBackdrop: BackdropImage | null;
  updateTether: () => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  filteredLines: LyricLineGroup[];
  activeKey: string | null;
  draftDesigns: Record<string, LineDesign>;
  setSelectedLineKey: (k: string) => void;
  setActiveItemIndex: (i: number) => void;
  setPlayheadSec: (sec: number) => void;
  anchorRefs: React.MutableRefObject<Map<string, HTMLButtonElement>>;
  formatTime: (sec: number) => string;
}

export const LyricsMomentRail: React.FC<LyricsMomentRailProps> = ({
  loading,
  allLineGroups,
  activeBackdrop,
  updateTether,
  searchQuery,
  setSearchQuery,
  filteredLines,
  activeKey,
  draftDesigns,
  setSelectedLineKey,
  setActiveItemIndex,
  setPlayheadSec,
  anchorRefs,
  formatTime,
}) => (
        <aside id="words-lyrics" className="lyrics-moments-rail">
          {/* LEFT RAIL: Lyric Moments with 16:9 Paper/Scene Miniature Thumbnails */}
          <div className="lyrics-moments-header">
            <div className="lyrics-moments-title-row">
              <strong>Lyric moments</strong>
              <span className="muted">Artist words · read-only</span>
            </div>
            <input
              id="words-lyrics-search"
              type="search"
              placeholder="Filter lyrics…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>

          <div className="lyrics-moments-list" onScroll={updateTether}>
            {loading && !allLineGroups.length ? (
              <p className="muted" style={{ padding: "12px" }}>
                Loading lyric moments…
              </p>
            ) : null}
            {filteredLines.map((group, idx) => {
              const key = `${group.animId}::${group.lineId}`;
              const isSelected = activeKey === key;
              const lineDesign =
                draftDesigns[key] ?? buildDefaultLineDesign(group.words, "cascade");
              const prevSection = idx > 0 ? filteredLines[idx - 1].sectionName : null;
              const showSectionHead = group.sectionName !== prevSection;
              const leadItem = lineDesign.items[0];
              return (
                <React.Fragment key={key}>
                  {showSectionHead && (
                    <div className="lyrics-section-divider">
                      <span className="badge">{group.sectionName}</span>
                      <span className="muted">{formatTime(group.start)}</span>
                    </div>
                  )}
                  <article
                    id={`words-moment-${group.animId}-${group.lineId}`}
                    className={`lyrics-moment-card ${isSelected ? "active" : ""}`}
                    onClick={() => {
                      setSelectedLineKey(key);
                      setActiveItemIndex(0);
                      setPlayheadSec(group.start);
                    }}
                  >
                    <div className="lyrics-moment-top">
                      {activeBackdrop && (
                        <span className="lyrics-moment-scene-thumb">
                          <img
                            src={convertFileSrc(activeBackdrop.path)}
                            alt=""
                          />
                          <em>16:9</em>
                        </span>
                      )}
                      <div className="lyrics-moment-meta">
                        <div className="lyrics-moment-line-text">
                          <span className="time">[{formatTime(group.start)}]</span>{" "}
                          {group.text}
                        </div>
                        <div className="lyrics-moment-badges">
                          <span className="pill-amber">
                            ⚡ {leadItem?.move ?? "pop"}
                          </span>
                          {leadItem?.warp_path && leadItem.warp_path !== "none" && (
                            <span className="pill-teal">
                              ⤴ {leadItem.warp_path}
                            </span>
                          )}
                          {lineDesign.draws.length > 0 && (
                            <span className="pill-emerald">
                              ✎ {lineDesign.draws.length} stroke
                            </span>
                          )}
                        </div>
                      </div>
                      <button
                        ref={(el) => {
                          if (el) anchorRefs.current.set(key, el);
                          else anchorRefs.current.delete(key);
                        }}
                        type="button"
                        className="lyrics-moment-anchor-btn"
                        title="Tethered to 16:9 stage"
                      >
                        ⚓
                      </button>
                    </div>

                    {/* Miniature 16:9 Paper Preview */}
                    <svg
                      viewBox={`0 0 ${STAGE_W} ${STAGE_H}`}
                      className="lyrics-moment-mini-svg"
                    >
                      <rect
                        width={STAGE_W}
                        height={STAGE_H}
                        rx={32}
                        fill="#F5EFE4"
                      />
                      {lineDesign.items.map((it, i) => {
                        const txt = it.word_ids
                          .map((wid) => group.words.find((w) => w.word_id === wid)?.text)
                          .filter(Boolean)
                          .join(" ");
                        const font = fontStackForLettering(it.lettering);
                        const cx = it.x + (it.w ?? 600) * 0.5;
                        const cy = it.y + (it.h ?? 130) * 0.65;
                        const outline = effectiveOutlineColor(it);
                        return (
                          <g
                            key={i}
                            transform={`translate(${cx}, ${cy}) rotate(${it.rotate ?? 0}) skewX(${it.skew ?? 0}) scale(${(it.scale_x ?? 100) / 100}, ${(it.scale_y ?? 100) / 100})`}
                          >
                            <text
                              textAnchor="middle"
                              fill={it.color || "#173F49"}
                              stroke={outline ?? "none"}
                              strokeWidth={outline ? (it.stroke_width ?? 6) * 1.4 : 0}
                              style={{
                                fontFamily: font.fontFamily,
                                fontWeight: font.fontWeight,
                                fontStyle: font.fontStyle,
                                fontSize: `${Math.max(46, it.size ?? 84)}px`,
                                paintOrder: "stroke fill",
                              }}
                            >
                              {txt}
                            </text>
                          </g>
                        );
                      })}
                    </svg>

                    <div className="lyrics-moment-foot">
                      <span>
                        {formatTime(group.start)} – {formatTime(group.end)}
                      </span>
                      <span className="hero-tag">
                        ✎ &ldquo;
                        {(leadItem?.word_ids ?? [])
                          .map((wid) => group.words.find((w) => w.word_id === wid)?.text)
                          .filter(Boolean)
                          .join(" ") || group.words[0]?.text}
                        &rdquo; ({lineDesign.items.length})
                      </span>
                    </div>
                  </article>
                </React.Fragment>
              );
            })}
          </div>
        </aside>

);
