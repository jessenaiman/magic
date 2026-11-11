import React from "react";
import {
  CharacterPart,
  CharacterViewSummary,
  executeCommand,
  fileUrl,
} from "../../api";
import { Slider } from "../../components/controls";

export interface AnimationRigPartsPanelProps {
  selectedCharId: string;
  selectedView: string;
  onSelectView: (view: string) => void;
  selectedBone: string;
  onSelectBone: (bone: string) => void;
  parts: CharacterPart[];
  activePart?: CharacterPart;
  activeViewSummary?: CharacterViewSummary;
  drawnViews: CharacterViewSummary[];
  hasMultipleDrawnViews: boolean;
  layerDepth: number;
  setLayerDepth: React.Dispatch<React.SetStateAction<number>>;
  layerReason: string;
  setLayerReason: React.Dispatch<React.SetStateAction<string>>;
  hiddenParts: Record<string, boolean>;
  setHiddenParts: React.Dispatch<React.SetStateAction<Record<string, boolean>>>;
  partsTab: "parts" | "compare";
  setPartsTab: React.Dispatch<React.SetStateAction<"parts" | "compare">>;
  isMarked: boolean;
  isCut: boolean;
  isRigged: boolean;
  runPrepareStep: (cmd: string, label: string) => Promise<void>;
  busy: boolean;
  onToast: (msg: string) => void;
}

export const AnimationRigPartsPanel: React.FC<AnimationRigPartsPanelProps> = ({
  selectedCharId,
  selectedView,
  onSelectView,
  selectedBone,
  onSelectBone,
  parts,
  activePart,
  activeViewSummary,
  drawnViews,
  hasMultipleDrawnViews,
  layerDepth,
  setLayerDepth,
  layerReason,
  setLayerReason,
  hiddenParts,
  setHiddenParts,
  partsTab,
  setPartsTab,
  isMarked,
  isCut,
  isRigged,
  runPrepareStep,
  busy,
  onToast,
}) => (
  <>
      {/* 4. LAYERS (open by default per A3) */}
      <details id="motion-layers" className="panel motion-section" data-look="depth" open>
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
              <path d="m12 2 9 5-9 5-9-5 9-5z" />
              <path d="m3 12 9 5 9-5" />
              <path d="m3 17 9 5 9-5" />
            </svg>
          </span>
          <span className="title">Layers</span>
          <span id="motion-layers-meta" className="meta">
            {parts.length}
          </span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div className="motion-coordinate-row">
            <label className="anim-field-label">
              <span>Layer</span>
              <select
                id="authoring-layer-picker"
                value={selectedBone}
                onChange={(e) => onSelectBone(e.target.value)}
              >
                {parts.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.id}
                  </option>
                ))}
              </select>
            </label>
            <label className="anim-field-label">
              <span>Reason</span>
              <input
                id="authoring-layer-reason"
                type="text"
                placeholder="e.g. arm in front"
                value={layerReason}
                onChange={(e) => setLayerReason(e.target.value)}
              />
            </label>
          </div>
          <Slider
            id="authoring-layer-depth"
            label="Depth offset"
            control="horizontal"
            value={layerDepth}
            min={-10}
            max={10}
            step={1}
            onChange={setLayerDepth}
          />
          <div className="motion-path-actions">
            <button
              id="authoring-layer-back"
              type="button"
              className="ghost"
              disabled={busy}
              onClick={async () => {
                setLayerDepth((d) => d - 1);
                try {
                  await executeCommand("character.set", {
                    character: selectedCharId,
                    view: selectedView,
                    part: selectedBone,
                    prop: "arrange",
                    value: "backward",
                  });
                  onToast(`Sent ${selectedBone} backward in EffectCraft`);
                } catch (err) {
                  onToast(String(err));
                }
              }}
            >
              Backward
            </button>
            <button
              id="authoring-layer-front"
              type="button"
              className="ghost"
              disabled={busy}
              onClick={async () => {
                setLayerDepth((d) => d + 1);
                try {
                  await executeCommand("character.set", {
                    character: selectedCharId,
                    view: selectedView,
                    part: selectedBone,
                    prop: "arrange",
                    value: "forward",
                  });
                  onToast(`Brought ${selectedBone} forward in EffectCraft`);
                } catch (err) {
                  onToast(String(err));
                }
              }}
            >
              Forward
            </button>
            <button
              id="authoring-layer-apply"
              type="button"
              className="start"
              disabled={busy}
              onClick={async () => {
                try {
                  await executeCommand("character.set", {
                    character: selectedCharId,
                    view: selectedView,
                    part: selectedBone,
                    prop: "arrange",
                    value: layerDepth < 0 ? "back" : "front",
                  });
                  onToast(`Applied layer order (${selectedBone} → ${layerDepth < 0 ? "back" : "front"})`);
                } catch (err) {
                  onToast(String(err));
                }
              }}
            >
              Apply
            </button>
          </div>

          <div className="motion-layers">
            {parts.map((p) => {
              const isSel = selectedBone === p.id;
              const isHidden = Boolean(hiddenParts[p.id]);
              return (
                <div
                  key={p.id}
                  id={`motion-layer-${p.id}`}
                  className={`motion-layer ${isSel ? "selected" : ""} ${isHidden ? "hidden-layer" : ""}`}
                >
                  {p.file ? (
                    <img src={fileUrl(p.file)} alt={p.id} loading="lazy" />
                  ) : (
                    <span className="magic-asset-placeholder">PT</span>
                  )}
                  <button
                    type="button"
                    className="motion-layer-name"
                    aria-pressed={isSel}
                    onClick={() => onSelectBone(p.id)}
                  >
                    {p.name || p.id}
                  </button>
                  <button
                    id={`motion-visible-${p.id}`}
                    type="button"
                    className="icon"
                    aria-pressed={!isHidden}
                    title={isHidden ? "Show layer" : "Hide layer"}
                    onClick={async () => {
                      const nextHidden = !isHidden;
                      setHiddenParts((prev) => ({ ...prev, [p.id]: nextHidden }));
                      try {
                        await executeCommand("character.set", {
                          character: selectedCharId,
                          view: selectedView,
                          part: p.id,
                          prop: "visible",
                          value: !nextHidden,
                        });
                      } catch {
                        // Local visibility state preserved if not rigged yet
                      }
                    }}
                  >
                    👁
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      </details>

      {/* 5. PART ARTWORK */}
      <details id="magic-parts-pane" className="panel motion-section" data-look="artwork">
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
              <path d="m12 2 9 5-9 5-9-5 9-5z" />
              <path d="m3 12 9 5 9-5" />
              <path d="m3 17 9 5 9-5" />
            </svg>
          </span>
          <span className="title">Parts</span>
          <span id="magic-parts-meta" className="meta">
            {isRigged ? "Rigged" : isCut ? "Cut" : isMarked ? "Marked" : "Cutout"}
          </span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div id="magic-parts-slot" className="magic-parts-slot">
            <div id="character-prepare" className="seq-side-block" style={{ borderTop: 0, paddingTop: 0 }}>
              {hasMultipleDrawnViews && (
                <label className="seq-field">
                  <span>View</span>
                  <select
                    id="character-prepare-view"
                    value={selectedView}
                    onChange={(e) => onSelectView(e.target.value)}
                  >
                    {drawnViews.map((vObj) => (
                      <option key={vObj.view} value={vObj.view}>
                        {vObj.view}
                      </option>
                    ))}
                  </select>
                </label>
              )}
              <p id="character-prepare-status" className="motion-help">
                {activeViewSummary?.status || "Mark parts, cut layers in PhotoCraft, and rig in EffectCraft."}
              </p>
              <div className="motion-path-actions">
                <button
                  id="character-mark-run"
                  type="button"
                  className="ghost"
                  disabled={busy}
                  onClick={() => void runPrepareStep("character.mark", "1. Mark parts")}
                >
                  Mark
                </button>
                <button
                  id="character-cut-run"
                  type="button"
                  className="ghost"
                  disabled={busy}
                  onClick={() => void runPrepareStep("character.cut", "2. Cut layers")}
                >
                  Cut
                </button>
                <button
                  id="character-rig-run"
                  type="button"
                  className="start"
                  disabled={busy}
                  onClick={() => void runPrepareStep("character.rig", "3. Rig & animate")}
                >
                  Rig
                </button>
              </div>
              <div className="motion-path-actions">
                <button
                  id="character-open-photocraft"
                  type="button"
                  className="ghost"
                  onClick={() =>
                    void executeCommand("character.open", {
                      character: selectedCharId,
                      view: selectedView,
                      app: "photocraft",
                    })
                  }
                >
                  Edit in PhotoCraft
                </button>
                <button
                  id="character-open-effectcraft"
                  type="button"
                  className="ghost"
                  onClick={() =>
                    void executeCommand("character.open", {
                      character: selectedCharId,
                      view: selectedView,
                      app: "effectcraft",
                    })
                  }
                >
                  Edit in EffectCraft
                </button>
              </div>
            </div>

            <div className="magic-parts-drawer" style={{ marginTop: "var(--space-2)" }}>
              <div className="segmented" role="tablist" aria-label="Part artwork view">
                <button
                  id="magic-parts-tab-parts"
                  type="button"
                  role="tab"
                  aria-selected={partsTab === "parts"}
                  className={partsTab === "parts" ? "active" : ""}
                  onClick={() => setPartsTab("parts")}
                >
                  Parts ({parts.length})
                </button>
                <button
                  id="magic-parts-tab-compare"
                  type="button"
                  role="tab"
                  aria-selected={partsTab === "compare"}
                  className={partsTab === "compare" ? "active" : ""}
                  onClick={() => setPartsTab("compare")}
                >
                  Compare
                </button>
              </div>

              {partsTab === "parts" ? (
                <div className="magic-part-cards" style={{ marginTop: "var(--space-2)" }}>
                  {parts.map((p) => {
                    const isSel = selectedBone === p.id;
                    return (
                      <div
                        key={p.id}
                        className={`magic-part-card ${isSel ? "active" : ""}`}
                        onClick={() => onSelectBone(p.id)}
                      >
                        <div className="magic-part-card-thumb">
                          {p.file ? (
                            <img src={fileUrl(p.file)} alt={p.id} loading="lazy" />
                          ) : (
                            <span className="magic-asset-placeholder">PT</span>
                          )}
                        </div>
                        <div className="magic-part-card-pick">
                          <span className="magic-part-card-name">{p.name || p.id}</span>
                          <span className="dot ok" />
                        </div>
                        <div className="magic-part-card-actions">
                          <span className="magic-part-card-id">{p.parent || "root"}</span>
                          <button
                            type="button"
                            className="ghost magic-part-card-open"
                            onClick={(e) => {
                              e.stopPropagation();
                              void executeCommand("character.open", {
                                character: selectedCharId,
                                view: selectedView,
                                app: "photocraft",
                              });
                            }}
                          >
                            PSD
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <div id="magic-part-compare" className="magic-part-compare" style={{ marginTop: "var(--space-2)" }}>
                  <div className="magic-parts-strip">
                    {parts.map((p) => (
                      <button
                        key={p.id}
                        type="button"
                        className={`magic-part-thumb ${selectedBone === p.id ? "active" : ""}`}
                        onClick={() => onSelectBone(p.id)}
                      >
                        {p.file ? <img src={fileUrl(p.file)} alt={p.id} /> : null}
                        <span>{p.id}</span>
                      </button>
                    ))}
                  </div>
                  <div className="magic-compare-grid">
                    <figure className="magic-compare-frame">
                      {activeViewSummary?.drawing ? (
                        <img src={fileUrl(activeViewSummary.drawing)} alt="Original drawing" />
                      ) : (
                        <span className="magic-asset-placeholder">SRC</span>
                      )}
                      <figcaption>Source drawing</figcaption>
                    </figure>
                    <figure className="magic-compare-frame">
                      {activePart?.file ? (
                        <img src={fileUrl(activePart.file)} alt={activePart.id} />
                      ) : (
                        <span className="magic-asset-placeholder">CUT</span>
                      )}
                      <figcaption>{activePart?.id || "Cut layer"}</figcaption>
                    </figure>
                  </div>
                  <div className="magic-part-actionbar">
                    <button
                      id="part-open-photocraft"
                      type="button"
                      className="ghost"
                      onClick={() =>
                        void executeCommand("character.open", {
                          character: selectedCharId,
                          view: selectedView,
                          app: "photocraft",
                        })
                      }
                    >
                      Open in PhotoCraft
                    </button>
                    <button
                      id="part-copy-path"
                      type="button"
                      className="ghost"
                      onClick={() => {
                        if (activePart?.file) {
                          void navigator.clipboard?.writeText(activePart.file);
                          onToast(`Copied path: ${activePart.file}`);
                        }
                      }}
                    >
                      Copy
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </details>

  </>
);
