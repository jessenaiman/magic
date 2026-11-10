import React from "react";
import {
  CharacterPart,
  CharacterViewSummary,
  executeCommand,
} from "../../api";
import { Slider } from "../../components/controls";

export interface AnimationPuppetPanelProps {
  selectedCharId: string;
  selectedView: string;
  onSelectView: (v: string) => void;
  selectedBone: string;
  activePart?: CharacterPart;
  drawnViews: CharacterViewSummary[];
  hasMultipleDrawnViews: boolean;
  hiddenParts: Record<string, boolean>;
  setHiddenParts: React.Dispatch<React.SetStateAction<Record<string, boolean>>>;
  partPosX: number;
  setPartPosX: React.Dispatch<React.SetStateAction<number>>;
  partPosY: number;
  setPartPosY: React.Dispatch<React.SetStateAction<number>>;
  partScaleX: number;
  setPartScaleX: React.Dispatch<React.SetStateAction<number>>;
  partScaleY: number;
  setPartScaleY: React.Dispatch<React.SetStateAction<number>>;
  partPivotX: number;
  setPartPivotX: React.Dispatch<React.SetStateAction<number>>;
  partPivotY: number;
  setPartPivotY: React.Dispatch<React.SetStateAction<number>>;
  rotation: number;
  onChangeRotation: (val: number) => void;
  onCommitRotation: (val: number) => void;
  scale: number;
  onChangeScale: (val: number) => void;
  onCommitScale: (val: number) => void;
  opacity: number;
  onChangeOpacity: (val: number) => void;
  onCommitOpacity: (val: number) => void;
  onKeyTransform: (propsPayload?: Record<string, unknown>) => void;
  currentTime: number;
  pinMode: "position" | "starch" | "overlap" | "bend";
  onChangePinMode: (mode: "position" | "starch" | "overlap" | "bend") => void;
  onRefreshCharacter: () => Promise<void>;
  busy: boolean;
  onToast: (msg: string) => void;
}

export const AnimationPuppetPanel: React.FC<AnimationPuppetPanelProps> = ({
  selectedCharId,
  selectedView,
  onSelectView,
  selectedBone,
  activePart,
  drawnViews,
  hasMultipleDrawnViews,
  hiddenParts,
  setHiddenParts,
  partPosX,
  setPartPosX,
  partPosY,
  setPartPosY,
  partScaleX,
  setPartScaleX,
  partScaleY,
  setPartScaleY,
  partPivotX,
  setPartPivotX,
  partPivotY,
  setPartPivotY,
  rotation,
  onChangeRotation,
  onCommitRotation,
  scale,
  onChangeScale,
  onCommitScale,
  opacity,
  onChangeOpacity,
  onCommitOpacity,
  onKeyTransform,
  currentTime,
  pinMode,
  onChangePinMode,
  onRefreshCharacter,
  busy,
  onToast,
}) => (
  <>
      {/* 6. EXPRESSION & POSE (includes original Part Transform + tucked Pose / 2-Bone IK / Puppet Mesh / Hand & Facial Sprites) */}
      <details id="motion-pose" className="panel motion-section" data-look="pose" open>
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
              <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
          </span>
          <span className="title">Puppet</span>
          <span id="motion-pose-meta" className="meta">
            {selectedBone}
          </span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div id="anim-selected-part-inspector" className="anim-part-inspector">
            <div className="anim-bone-banner">
              <span className="anim-subhead">
                Selected Part · {selectedBone.toUpperCase()}
              </span>
              <span className="anim-engine-badge">• EffectCraft</span>
            </div>
            <div className="motion-path-actions">
              <button
                id="motion-bar-visible"
                type="button"
                className="ghost"
                onClick={async () => {
                  const nextHidden = !hiddenParts[selectedBone];
                  setHiddenParts((prev) => ({
                    ...prev,
                    [selectedBone]: nextHidden,
                  }));
                  if (selectedCharId) {
                    try {
                      await executeCommand("character.set", {
                        character: selectedCharId,
                        view: selectedView,
                        part: selectedBone,
                        prop: "visible",
                        value: !nextHidden,
                      });
                    } catch {
                      // local preview still updates if character is unrigged
                    }
                  }
                }}
              >
                {hiddenParts[selectedBone] ? "Show part" : "Hide part"}
              </button>
              <button
                id="motion-show-all"
                type="button"
                className="ghost"
                onClick={async () => {
                  const previouslyHidden = Object.keys(hiddenParts).filter((k) => hiddenParts[k]);
                  setHiddenParts({});
                  if (selectedCharId) {
                    for (const partId of previouslyHidden) {
                      try {
                        await executeCommand("character.set", {
                          character: selectedCharId,
                          view: selectedView,
                          part: partId,
                          prop: "visible",
                          value: true,
                        });
                      } catch {
                        // ignore unrigged parts
                      }
                    }
                  }
                }}
              >
                Reveal
              </button>
            </div>
          </div>

          <div className="motion-coordinate-row" style={{ gridTemplateColumns: "repeat(4, minmax(0, 1fr))" }}>
            <label className="anim-field-label">
              <span>Pos X</span>
              <input
                id="authoring-part-x"
                type="number"
                value={partPosX}
                onChange={(e) => setPartPosX(Number(e.target.value))}
              />
            </label>
            <label className="anim-field-label">
              <span>Pos Y</span>
              <input
                id="authoring-part-y"
                type="number"
                value={partPosY}
                onChange={(e) => setPartPosY(Number(e.target.value))}
              />
            </label>
            <label className="anim-field-label">
              <span>Piv X</span>
              <input
                id="authoring-pivot-x"
                type="number"
                value={partPivotX}
                onChange={(e) => setPartPivotX(Number(e.target.value))}
              />
            </label>
            <label className="anim-field-label">
              <span>Piv Y</span>
              <input
                id="authoring-pivot-y"
                type="number"
                value={partPivotY}
                onChange={(e) => setPartPivotY(Number(e.target.value))}
              />
            </label>
          </div>
          <div className="motion-coordinate-row">
            <label className="anim-field-label">
              <span>Scale X (%)</span>
              <input
                id="authoring-scale-x"
                type="number"
                value={partScaleX}
                onChange={(e) => setPartScaleX(Number(e.target.value))}
              />
            </label>
            <label className="anim-field-label">
              <span>Scale Y (%)</span>
              <input
                id="authoring-scale-y"
                type="number"
                value={partScaleY}
                onChange={(e) => setPartScaleY(Number(e.target.value))}
              />
            </label>
          </div>

          <Slider
            id="bone-rotation"
            label="Rotation"
            control="dial"
            value={rotation}
            min={-180}
            max={180}
            step={1}
            unit="°"
            onChange={onChangeRotation}
            onCommit={onCommitRotation}
          />

          <Slider
            id="bone-scale"
            label="Scale"
            value={scale}
            min={10}
            max={200}
            step={1}
            unit="%"
            onChange={onChangeScale}
            onCommit={onCommitScale}
          />

          <Slider
            id="bone-opacity"
            label="Opacity"
            value={opacity}
            min={0}
            max={100}
            step={1}
            unit="%"
            onChange={onChangeOpacity}
            onCommit={onCommitOpacity}
          />

          <div className="motion-path-actions">
            <button
              id="authoring-part-reset"
              type="button"
              className="ghost"
              disabled={busy}
              onClick={() => {
                onChangeRotation(0);
                onChangeScale(100);
                onChangeOpacity(100);
                const joint = activePart?.joint || [0, 0];
                setPartPosX(Math.round(joint[0]));
                setPartPosY(Math.round(joint[1]));
                setPartPivotX(Math.round(joint[0]));
                setPartPivotY(Math.round(joint[1]));
                setPartScaleX(100);
                setPartScaleY(100);
                onKeyTransform({
                  position: [Math.round(joint[0]), Math.round(joint[1])],
                  anchor: [Math.round(joint[0]), Math.round(joint[1])],
                  scale: [100, 100],
                  rotation: 0,
                  opacity: 100,
                });
              }}
            >
              Reset
            </button>
            <button
              id="authoring-part-apply"
              type="button"
              className="start"
              disabled={busy}
              onClick={() =>
                onKeyTransform({
                  position: [partPosX, partPosY],
                  anchor: [partPivotX, partPivotY],
                  scale: [partScaleX, partScaleY],
                  rotation,
                  opacity,
                })
              }
            >
              Key {selectedBone} Transform
            </button>
          </div>

          {/* EffectCraft Puppet Controls: 11-Pin Face Rig, Mouth Viseme Lip-Sync, Expressions & Pins */}
          <details className="seq-side-block" open>
            <summary className="muted" style={{ cursor: "pointer", fontSize: "var(--text-xs)" }}>
              EffectCraft Puppet Controls (Face, Lip-Sync &amp; {pinMode} Pins)
            </summary>
            <div className="motion-section-body" style={{ padding: "var(--space-2) 0 0" }}>
              <div className="motion-path-actions" style={{ marginBottom: "6px" }}>
                <button
                  id="authoring-puppet-rig-face"
                  type="button"
                  className="ghost"
                  disabled={busy || !selectedCharId}
                  onClick={async () => {
                    try {
                      await executeCommand("character.puppet.rig_face", {
                        character: selectedCharId,
                        view: selectedView,
                      });
                      await onRefreshCharacter();
                      onToast(`Installed 11-pin EffectCraft face & mouth puppet rig (${selectedCharId})`);
                    } catch (err) {
                      onToast(String(err));
                    }
                  }}
                >
                  Face
                </button>
                <button
                  id="authoring-puppet-lipsync"
                  type="button"
                  className="start"
                  disabled={busy || !selectedCharId}
                  onClick={async () => {
                    try {
                      const res = await executeCommand<{ keyframes_added?: number }>(
                        "character.puppet.lipsync",
                        {
                          character: selectedCharId,
                          view: selectedView,
                          start: currentTime,
                          intensity: 1.0,
                        }
                      );
                      await onRefreshCharacter();
                      onToast(
                        `Synced mouth viseme puppet pins to song audio (${res.keyframes_added ?? 0} keys)`
                      );
                    } catch (err) {
                      onToast(String(err));
                    }
                  }}
                >
                  Lipsync
                </button>
              </div>
              <div className="motion-actions-grid" style={{ marginBottom: "6px" }}>
                {(
                  [
                    "happy",
                    "joy",
                    "surprised",
                    "oh",
                    "sad",
                    "focused",
                    "blink",
                    "wink_L",
                    "wink_R",
                    "neutral",
                  ] as const
                ).map((expr) => (
                  <button
                    key={expr}
                    id={`puppet-expr-${expr}`}
                    type="button"
                    className="motion-action"
                    disabled={busy || !selectedCharId}
                    onClick={async () => {
                      try {
                        await executeCommand("character.puppet.expression", {
                          character: selectedCharId,
                          view: selectedView,
                          expression: expr,
                          at: currentTime,
                          duration: 0.9,
                        });
                        await onRefreshCharacter();
                        onToast(`Keyed ${expr} facial puppet expression at ${currentTime.toFixed(2)}s`);
                      } catch (err) {
                        onToast(String(err));
                      }
                    }}
                  >
                    {expr.replace("_", " ")}
                  </button>
                ))}
              </div>
              <div className="motion-actions-grid">
                {(["position", "starch", "overlap", "bend"] as const).map((mode) => (
                  <button
                    key={mode}
                    type="button"
                    className="motion-action"
                    aria-pressed={pinMode === mode}
                    onClick={() => onChangePinMode(mode)}
                  >
                    {mode}
                  </button>
                ))}
              </div>
              <div className="motion-path-actions">
                <button
                  id="authoring-puppet-add-pin"
                  type="button"
                  className="ghost"
                  disabled={busy}
                  onClick={async () => {
                    try {
                      await executeCommand("character.puppet.addPin", {
                        character: selectedCharId,
                        view: selectedView,
                        layer: selectedBone,
                        kind: pinMode,
                        position: [partPosX || 192, partPosY || 256],
                      });
                      onToast(`Added ${pinMode} puppet pin on ${selectedBone} at (${partPosX}, ${partPosY})`);
                    } catch (err) {
                      onToast(String(err));
                    }
                  }}
                >
                  Add {pinMode} pin
                </button>
                <button
                  id="authoring-keyframe-ease"
                  type="button"
                  className="ghost"
                  disabled={busy}
                  onClick={async () => {
                    try {
                      await executeCommand("character.keyframe.easyEase", {
                        character: selectedCharId,
                        view: selectedView,
                        layer: selectedBone,
                        prop: "rotation",
                      });
                      onToast(`Applied Ease to ${selectedBone} rotation keyframes`);
                    } catch (err) {
                      onToast(String(err));
                    }
                  }}
                >
                  Ease
                </button>
              </div>
            </div>
          </details>

          {hasMultipleDrawnViews && (
            <button
              id="btn-generate-turnaround-sheet"
              type="button"
              className="ghost"
              onClick={() => {
                const idx = drawnViews.findIndex((v) => v.view === selectedView);
                const nextView = drawnViews[(idx + 1) % drawnViews.length]?.view || "front";
                onSelectView(nextView);
              }}
            >
              Turnaround
            </button>
          )}
        </div>
      </details>

  </>
);
