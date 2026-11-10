import React from "react";
import { LyricLineGroup } from "./types";

export interface TimelineMarkerItem {
  id?: number | null;
  ec_index?: number;
  index?: number;
  time_sec: number;
  duration_sec: number;
  name: string;
  comment: string;
  color: string;
  kind: string;
  source?: string;
}

export interface SceneTrackClipItem {
  clip: number;
  name: string;
  start_sec: number;
  end_sec: number;
  duration_sec?: number;
  label?: string;
}

export interface LyricsTimelineDockProps {
  totalSongDuration: number;
  playheadSec: number;
  setPlayheadSec: (sec: number) => void;
  timelineMarkers: TimelineMarkerItem[];
  sceneTrackClips: SceneTrackClipItem[];
  waveformPeaks: number[];
  allLineGroups: LyricLineGroup[];
  activeKey: string | null;
  setSelectedLineKey: (key: string) => void;
  setActiveItemIndex: (idx: number) => void;
  activeLine: LyricLineGroup | null;
  formatTime: (sec: number) => string;
}

export const LyricsTimelineDock: React.FC<LyricsTimelineDockProps> = ({
  totalSongDuration,
  playheadSec,
  setPlayheadSec,
  timelineMarkers,
  sceneTrackClips,
  waveformPeaks,
  allLineGroups,
  activeKey,
  setSelectedLineKey,
  setActiveItemIndex,
  activeLine,
  formatTime,
}) => (
          <section id="authoring-timeline" className="lyrics-timeline-dock">
            {/* Bottom Multi-Lane Timeline with Cues/Markers Ruler, Grouped Tracks & FilmCraft Audio Waveform */}
            <div className="lyrics-timeline-tracks">
              {/* Cues / Markers Ruler Lane (Synced FilmCraft + EffectCraft) */}
              <div className="lyrics-timeline-row markers-row">
                <span className="lane-label">Cues</span>
                <div
                  className="lane-track"
                  onClick={(e) => {
                    const r = e.currentTarget.getBoundingClientRect();
                    const pct = Math.max(
                      0,
                      Math.min(1, (e.clientX - r.left) / Math.max(1, r.width)),
                    );
                    setPlayheadSec(pct * totalSongDuration);
                  }}
                >
                  {timelineMarkers.map((mk, i) => {
                    const left = Math.max(
                      0,
                      Math.min(98.5, (mk.time_sec / totalSongDuration) * 100),
                    );
                    const width =
                      mk.duration_sec > 0
                        ? Math.max(
                            1.5,
                            Math.min(
                              100 - left,
                              (mk.duration_sec / totalSongDuration) * 100,
                            ),
                          )
                        : undefined;
                    const colorClass = (mk.color || "Iris").toLowerCase();
                    return (
                      <button
                        key={`mk-${mk.id ?? i}-${mk.time_sec}`}
                        type="button"
                        className={`timeline-marker-pin color-${colorClass} ${
                          width ? "is-range" : "is-cue"
                        }`}
                        style={{
                          left: `${left}%`,
                          width: width ? `${width}%` : undefined,
                        }}
                        title={`[${mk.source ?? "sync"}] ${mk.name}: ${mk.comment} (${formatTime(mk.time_sec)})`}
                        onClick={(ev) => {
                          ev.stopPropagation();
                          setPlayheadSec(mk.time_sec);
                        }}
                      >
                        {width ? `◆ ${mk.name || mk.comment}` : "◆"}
                      </button>
                    );
                  })}
                  <div
                    className="timeline-playhead"
                    style={{
                      left: `${Math.min(100, (playheadSec / totalSongDuration) * 100)}%`,
                    }}
                  />
                </div>
              </div>

              {/* Scene / V1 Grouped Track Lane */}
              {sceneTrackClips.length > 0 && (
                <div className="lyrics-timeline-row">
                  <span className="lane-label">Scenes</span>
                  <div className="lane-track">
                    {sceneTrackClips.map((sc) => {
                      const left = (sc.start_sec / totalSongDuration) * 100;
                      const width = Math.max(
                        1.5,
                        ((sc.end_sec - sc.start_sec) / totalSongDuration) * 100,
                      );
                      return (
                        <button
                          key={`sc-${sc.clip}`}
                          type="button"
                          className="timeline-scene-pill"
                          style={{ left: `${left}%`, width: `${width}%` }}
                          title={`${sc.name} (${formatTime(sc.start_sec)} – ${formatTime(sc.end_sec)})`}
                          onClick={() => setPlayheadSec(sc.start_sec)}
                        >
                          {sc.name}
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}

              {/* Waveform Lane */}
              <div className="lyrics-timeline-row waveform-row">
                <span className="lane-label">Audio</span>
                <div
                  className="lane-track"
                  onClick={(e) => {
                    const r = e.currentTarget.getBoundingClientRect();
                    const pct = Math.max(
                      0,
                      Math.min(1, (e.clientX - r.left) / Math.max(1, r.width)),
                    );
                    setPlayheadSec(pct * totalSongDuration);
                  }}
                >
                  <svg
                    viewBox="0 0 1000 36"
                    preserveAspectRatio="none"
                    className="lyrics-waveform-svg"
                  >
                    {(waveformPeaks.length > 0
                      ? waveformPeaks
                      : Array.from({ length: 120 }, (_, i) =>
                          0.2 + 0.6 * Math.abs(Math.sin(i * 0.23)),
                        )
                    ).map((pk, i, arr) => {
                      const x = (i / arr.length) * 1000;
                      const h = Math.max(3, Math.min(34, pk * 34));
                      return (
                        <rect
                          key={i}
                          x={x}
                          y={18 - h * 0.5}
                          width={Math.max(2, 800 / arr.length)}
                          height={h}
                          rx={1}
                          fill="rgba(72, 219, 198, 0.55)"
                        />
                      );
                    })}
                  </svg>
                  <div
                    className="timeline-playhead"
                    style={{
                      left: `${Math.min(100, (playheadSec / totalSongDuration) * 100)}%`,
                    }}
                  />
                </div>
              </div>

              {/* Lyric Lines Lane */}
              <div className="lyrics-timeline-row">
                <span className="lane-label">Lyrics</span>
                <div className="lane-track">
                  {allLineGroups.map((grp) => {
                    const key = `${grp.animId}::${grp.lineId}`;
                    const left = (grp.start / totalSongDuration) * 100;
                    const width = Math.max(
                      1.2,
                      ((grp.end - grp.start) / totalSongDuration) * 100,
                    );
                    return (
                      <button
                        key={key}
                        type="button"
                        className={`timeline-clip-pill ${activeKey === key ? "active" : ""}`}
                        style={{ left: `${left}%`, width: `${width}%` }}
                        title={`${grp.sectionName}: ${grp.text}`}
                        onClick={() => {
                          setSelectedLineKey(key);
                          setActiveItemIndex(0);
                          setPlayheadSec(grp.start);
                        }}
                      >
                        {grp.text}
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Active Line Word Tokens Lane */}
              <div className="lyrics-timeline-row">
                <span className="lane-label">Words</span>
                <div className="lane-track">
                  {(activeLine?.words ?? []).map((w) => {
                    const lineStart = activeLine?.start ?? 0;
                    const lineDur = Math.max(
                      0.5,
                      (activeLine?.end ?? lineStart + 3) - lineStart,
                    );
                    const left = Math.max(
                      0,
                      Math.min(96, ((w.start - lineStart) / lineDur) * 100),
                    );
                    const width = Math.max(
                      4,
                      Math.min(100 - left, ((w.end - w.start) / lineDur) * 100),
                    );
                    return (
                      <span
                        key={w.word_id}
                        className="timeline-word-token"
                        style={{ left: `${left}%`, width: `${width}%` }}
                      >
                        {w.text}
                      </span>
                    );
                  })}
                </div>
              </div>
            </div>
          </section>
);
