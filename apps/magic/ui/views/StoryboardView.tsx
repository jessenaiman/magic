import React, { useEffect, useRef, useState } from "react";
import {
  executeCommand,
  fileUrl,
  pickFile,
  StoryboardCheckPage,
  StoryboardData,
  StoryboardPage,
  StoryboardSlot,
} from "../api";

interface Props {
  projectPath: string;
  storyboard?: StoryboardData;
  onRefresh: () => Promise<void>;
  onNavigateStudio?: () => void;
  onToast: (message: string) => void;
}

function TrafficDot({ light }: { light?: string }) {
  const cls = light === "green" ? "green" : light === "amber" ? "amber" : "red";
  return <span className={`sb-light ${cls}`} aria-label={`${cls} status`} />;
}

export const StoryboardView: React.FC<Props> = ({
  projectPath,
  storyboard,
  onRefresh,
  onNavigateStudio,
  onToast,
}) => {
  const [pages, setPages] = useState<StoryboardPage[]>([]);
  const [check, setCheck] = useState<StoryboardData>(storyboard || {});
  const [pageIndex, setPageIndex] = useState(0);
  const [rail, setRail] = useState<string[]>([]);
  const [previews, setPreviews] = useState<Record<number, string>>({});
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [zoom, setZoom] = useState(100);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [editing, setEditing] = useState<string | null>(null);
  const [selectedSlot, setSelectedSlot] = useState<StoryboardSlot | null>(null);
  const [dragOverSlot, setDragOverSlot] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [wizStyle, setWizStyle] = useState("");
  const [wizWorld, setWizWorld] = useState("");
  const [wizChar, setWizChar] = useState("");
  const [wizMotion, setWizMotion] = useState("");
  const [newArchetype, setNewArchetype] = useState("scene");

  const request = useRef(0);
  const liveProject = useRef(projectPath);
  const cardsRef = useRef<HTMLDivElement | null>(null);
  const programmaticScrollUntil = useRef(0);
  liveProject.current = projectPath;

  const folder = projectPath.replace(/[/\\][^/\\]+$/, "");
  const page = pages[Math.min(pageIndex, Math.max(0, pages.length - 1))];
  const slotKey = (pageNum: number, slot: StoryboardSlot) => `${pageNum}:${slot.label}`;

  const load = async () => {
    const current = ++request.current;
    const project = projectPath;
    setError("");
    try {
      const data = await executeCommand<{ pages: StoryboardPage[] }>("storyboard.pages", {
        full: true,
        _expected_project: project,
      });
      const completion = await executeCommand<StoryboardData>("storyboard.check", {
        _expected_project: project,
      });
      const thumbs = await executeCommand<{ files: string[]; modified: number }>("storyboard.thumbnail", {
        rail: true,
        scale: 0.45,
        _expected_project: project,
      });
      if (current !== request.current || liveProject.current !== project) return;
      setPages(data.pages);
      setCheck(completion);
      setRail(thumbs.files.map((file) => `${fileUrl(file)}?v=${thumbs.modified}`));
      setPageIndex((index) => Math.min(index, Math.max(0, data.pages.length - 1)));
    } catch (cause) {
      if (current === request.current) {
        setError(String(cause));
      }
    }
  };

  useEffect(() => {
    setPageIndex(0);
    setDrafts({});
    setEditing(null);
    setSelectedSlot(null);
    setPreviews({});
    void load();
    return () => {
      ++request.current;
    };
  }, [projectPath]);

  useEffect(() => {
    let cancelled = false;
    if (page) {
      const targetPage = page.page;
      void executeCommand<{ out: string; modified: number }>("storyboard.thumbnail", {
        page: targetPage,
        scale: 0.75,
        _expected_project: projectPath,
      })
        .then((shot) => {
          if (!cancelled) {
            setPreviews((old) => ({ ...old, [targetPage]: `${fileUrl(shot.out)}?v=${shot.modified}` }));
          }
        })
        .catch((cause) => {
          if (!cancelled) setError(String(cause));
        });
    }
    return () => {
      cancelled = true;
    };
  }, [page, projectPath]);

  const scrollToPage = (index: number, focusLabel?: string) => {
    const clamped = Math.max(0, Math.min(pages.length - 1, index));
    setPageIndex(clamped);
    setEditing(null);
    const targetPage = pages[clamped];
    if (targetPage && focusLabel) {
      const match = targetPage.slots.find((s) => s.label === focusLabel);
      if (match) setSelectedSlot(match);
    }
    programmaticScrollUntil.current = Date.now() + 500;
    const target = document.getElementById(`sb-spread-${clamped}`);
    target?.scrollIntoView({ behavior: "smooth", block: "center" });
  };

  const handleScroll = () => {
    if (Date.now() < programmaticScrollUntil.current) return;
    const container = cardsRef.current;
    if (!container || !pages.length) return;
    const rect = container.getBoundingClientRect();
    const centerY = rect.top + rect.height / 2;
    let bestIndex = pageIndex;
    let bestDist = Infinity;
    for (let i = 0; i < pages.length; i++) {
      const el = document.getElementById(`sb-spread-${i}`);
      if (!el) continue;
      const r = el.getBoundingClientRect();
      const dist = Math.abs(r.top + r.height / 2 - centerY);
      if (dist < bestDist) {
        bestDist = dist;
        bestIndex = i;
      }
    }
    if (bestIndex !== pageIndex) {
      setPageIndex(bestIndex);
    }
  };

  const action = async (name: string, run: () => Promise<unknown>, refresh = false) => {
    if (busy) return;
    setBusy(name);
    setError("");
    try {
      await run();
      if (refresh) {
        await load();
        await onRefresh();
      }
    } catch (cause) {
      setError(String(cause));
      onToast(String(cause));
    } finally {
      setBusy("");
    }
  };

  const change = (pageNum: number, slot: StoryboardSlot, value: string) => {
    const key = slotKey(pageNum, slot);
    setDrafts((old) => {
      const next = { ...old };
      if (value === (slot.text ?? "")) delete next[key];
      else next[key] = value;
      return next;
    });
  };

  const saveTextSlot = async (pageNum: number, slot: StoryboardSlot) => {
    const key = slotKey(pageNum, slot);
    const value = drafts[key];
    if (value === undefined) return;
    await action(
      "Saving…",
      async () => {
        await executeCommand("storyboard.set", {
          page: pageNum,
          fields: { [slot.label]: value },
          _expected_project: projectPath,
        });
        setDrafts((old) => {
          const next = { ...old };
          delete next[key];
          return next;
        });
        setEditing(null);
      },
      true,
    );
  };

  const placeImageOnSlot = async (
    pageNum: number,
    slot: StoryboardSlot,
    imagePath: string,
    sourceProject?: string,
  ) => {
    await action(
      "Placing…",
      async () => {
        await executeCommand("storyboard.set", {
          page: pageNum,
          label: slot.label,
          image: imagePath,
          ...(sourceProject ? { source_project: sourceProject } : {}),
          _expected_project: projectPath,
        });
        onToast(`Placed flat image onto ${slot.label}`);
      },
      true,
    );
  };

  const pickImageForSlot = async (pageNum: number, slot: StoryboardSlot) => {
    const chosen = await pickFile({
      title: `Place flat image or Craft project on ${slot.label}`,
      extensions: ["png", "jpg", "jpeg", "webp", "svg", "ecproj", "pcraft", "psd", "vectorcraft", "fcproj"],
    });
    if (typeof chosen !== "string" || !chosen) return;
    await placeImageOnSlot(pageNum, slot, chosen);
  };

  const checkPages: StoryboardCheckPage[] = check.pages || [];
  const mainTitleChecks = checkPages.filter((p) =>
    ["cover", "world", "char", "motion"].includes(p.archetype),
  );
  const sceneChecks = checkPages.filter(
    (p) => !["cover", "world", "char", "motion"].includes(p.archetype),
  );

  const activeCli =
    selectedSlot?.cli ||
    page?.cli ||
    (page ? `magic-cli exec storyboard.pages page=${page.page}` : "magic-cli exec storyboard.new");

  return (
    <div
      id="view-storyboard"
      className="storyboard-layout sb-three-col"
      data-document-dirty={Object.keys(drafts).length > 0}
      data-document-saving={busy === "Saving…"}
    >
      {/* Left Rail: Framed Side-by-Side 2-Up Spread Preview & CRUD */}
      <aside className="sb-panel sb-index">
        <header style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 6 }}>
          <div>
            <h2 style={{ margin: 0, fontSize: 14 }}>Spreads</h2>
            <p className="muted" style={{ margin: 0, fontSize: 11 }}>
              DesignCraft · {pages.length} spreads
            </p>
          </div>
          <button
            id="storyboard-refresh"
            className="ghost magic-btn-icon"
            disabled={!!busy}
            title="Reload spreads"
            onClick={() =>
              void action("Reloading…", async () => {
                await load();
                await onRefresh();
              })
            }
          >
            <span>↻</span>
            <span>Reload</span>
          </button>
        </header>

        <div style={{ display: "flex", gap: 4, alignItems: "center", margin: "6px 0" }}>
          <select
            id="storyboard-archetype"
            aria-label="Template archetype"
            value={newArchetype}
            onChange={(e) => setNewArchetype(e.target.value)}
            style={{ flex: 1, fontSize: 11 }}
          >
            <option value="scene">E-Scene</option>
            <option value="world">B-World</option>
            <option value="char">C-Character</option>
            <option value="motion">D-Motion</option>
            <option value="cover">A-Cover</option>
          </select>
          <button
            id="storyboard-add-page"
            className="ghost magic-btn-icon"
            disabled={!!busy}
            title="Add spread from master template"
            onClick={() =>
              void action(
                "Adding…",
                () =>
                  executeCommand("storyboard.add", {
                    archetype: newArchetype,
                    at: pages.length,
                    _expected_project: projectPath,
                  }),
                true,
              )
            }
          >
            <span>+</span>
            <span>Page</span>
          </button>
          <button
            id="storyboard-remove-page"
            className="ghost magic-btn-icon"
            disabled={!!busy || pages.length <= 1}
            title="Delete selected spread"
            onClick={() =>
              page &&
              void action(
                "Deleting…",
                () =>
                  executeCommand("storyboard.remove", {
                    page: page.page,
                    _expected_project: projectPath,
                  }),
                true,
              )
            }
          >
            <span>✕</span>
            <span>Delete</span>
          </button>
        </div>

        <nav aria-label="Storyboard 2-up spreads" style={{ display: "flex", flexDirection: "column", gap: 8, overflowY: "auto", flex: 1 }}>
          {pages.map((item, index) => {
            const pCheck = checkPages.find((p) => p.page === item.page);
            const light = pCheck?.traffic_light || (pCheck?.status === "complete" ? "green" : "red");
            const leftNum = String(index * 2 + 1).padStart(2, "0");
            const rightNum = String(index * 2 + 2).padStart(2, "0");
            return (
              <button
                id={`sb-nav-${item.page + 1}`}
                key={item.page}
                type="button"
                className={`sb-spread-card ${page?.page === item.page ? "selected" : ""}`}
                aria-current={page?.page === item.page ? "page" : undefined}
                onClick={() => scrollToPage(index)}
              >
                <div className="sb-spread-frame">
                  {rail[item.page] ? (
                    <img src={rail[item.page]} alt={`Spread ${leftNum}-${rightNum}: ${item.title}`} />
                  ) : (
                    <span className="muted">{leftNum}–{rightNum}</span>
                  )}
                  <div className="sb-spread-spine" aria-hidden="true" />
                </div>
                <div className="sb-spread-meta">
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    <TrafficDot light={light} />
                    <strong>{leftNum}–{rightNum}</strong>
                    <span>{item.title}</span>
                  </span>
                  <span className="sb-section-pill">{item.section || item.archetype}</span>
                </div>
              </button>
            );
          })}
        </nav>

        <button
          id="storyboard-export"
          className="ghost magic-btn-icon"
          disabled={!!busy || !pages.length}
          onClick={() =>
            void action("Exporting…", async () => {
              await executeCommand("storyboard.export", {
                pdf: `${folder}/storyboard.pdf`,
                pptx: `${folder}/storyboard.pptx`,
                _expected_project: projectPath,
              });
              onToast("Exported storyboard.pdf & storyboard.pptx");
            })
          }
        >
          <span>⤓</span>
          <span>Export</span>
        </button>
      </aside>

      {/* Center Stage: Interactive Spread + Drag-and-Drop Flat Image Overlay + Exact CLI Bar */}
      <main className="sb-panel sb-main" style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
        <header className="sb-toolbar">
          <button
            id="sb-prev"
            className="ghost magic-btn-icon"
            disabled={pageIndex === 0}
            onClick={() => scrollToPage(pageIndex - 1)}
            title="Previous spread"
          >
            <span>←</span>
          </button>
          <strong style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {page
              ? `${page.page + 1} / ${pages.length} · ${page.title}${page.section ? ` (${page.section})` : ""}`
              : "Storyboard"}
          </strong>
          <button
            id="sb-next"
            className="ghost magic-btn-icon"
            disabled={!pages.length || pageIndex >= pages.length - 1}
            onClick={() => scrollToPage(pageIndex + 1)}
            title="Next spread"
          >
            <span>→</span>
          </button>
          <select
            id="sb-zoom"
            aria-label="Zoom"
            value={zoom}
            onChange={(event) => setZoom(Number(event.target.value))}
          >
            <option value={100}>Fit</option>
            <option value={150}>150%</option>
            <option value={200}>200%</option>
          </select>
          <button
            id="storyboard-open-editor"
            className="start magic-btn-icon"
            disabled={!!busy || !pages.length}
            onClick={() =>
              void action("Opening…", () =>
                executeCommand("storyboard.open", { _expected_project: projectPath }),
              )
            }
            title="Open live in DesignCraft (--control)"
          >
            <span>↗</span>
            <span>DesignCraft</span>
          </button>
        </header>

        <div
          id="sb-cards"
          ref={cardsRef}
          onScroll={handleScroll}
          className={`sb-spread ${zoom === 100 ? "fit" : "zoomed"}`}
          style={{ flex: 1 }}
        >
          {pages.length ? (
            pages.map((item) => {
              const imgSrc = previews[item.page] || rail[item.page];
              const [vx, vy, vx1, vy1] = item.bounds ?? [0, 0, 1600, 1000];
              const editableSlots = (item.slots || []).filter(
                (slot) => slot.editable && !slot.locked && !slot.hidden,
              );
              const isCurrent = page?.page === item.page;
              return (
                <section
                  id={`sb-spread-${item.page}`}
                  key={item.page}
                  className={`sb-page-viewport ${isCurrent ? "current" : ""}`}
                  aria-label={`Spread ${item.page + 1}: ${item.title}`}
                >
                  <div className={`sb-spread-stage sb-zoom-${zoom}`}>
                    {imgSrc ? (
                      <img src={imgSrc} alt={item.title} />
                    ) : (
                      <div className="sb-spread-loading muted">Rendering spread {item.page + 1}…</div>
                    )}
                    <svg
                      className="sb-overlay"
                      viewBox={`${vx} ${vy} ${vx1 - vx} ${vy1 - vy}`}
                      preserveAspectRatio="xMidYMid meet"
                    >
                      {editableSlots.map((slot) => {
                        const b = slot.bounds;
                        if (!b || b[2] <= b[0] || b[3] <= b[1]) return null;
                        const key = slotKey(item.page, slot);
                        const isText = slot.kind === "text";
                        const active = editing === key;
                        const value = drafts[key] ?? slot.text ?? "";
                        const dirty = drafts[key] !== undefined;

                        if (!isText) {
                          return (
                            <rect
                              key={slot.label}
                              id={isCurrent ? `sb-slot-${slot.label}` : `sb-slot-p${item.page}-${slot.label}`}
                              className={`sb-slot sb-slot-graphic ${dragOverSlot === key ? "drag-over" : ""}`}
                              x={b[0]}
                              y={b[1]}
                              width={b[2] - b[0]}
                              height={b[3] - b[1]}
                              role="button"
                              tabIndex={0}
                              aria-label={`${slot.label}: drop asset or click to place flat image`}
                              onClick={() => {
                                setPageIndex(item.page);
                                setSelectedSlot(slot);
                                void pickImageForSlot(item.page, slot);
                              }}
                              onDragOver={(e) => {
                                e.preventDefault();
                                e.dataTransfer.dropEffect = "copy";
                                setDragOverSlot(key);
                              }}
                              onDragLeave={() => setDragOverSlot((cur) => (cur === key ? null : cur))}
                              onDrop={(e) => {
                                e.preventDefault();
                                setDragOverSlot(null);
                                setPageIndex(item.page);
                                setSelectedSlot(slot);
                                const rawJson = e.dataTransfer.getData("application/x-magic-asset");
                                if (rawJson) {
                                  try {
                                    const parsed = JSON.parse(rawJson) as {
                                      path?: string;
                                      source_project?: string;
                                    };
                                    if (parsed.path) {
                                      void placeImageOnSlot(item.page, slot, parsed.path, parsed.source_project);
                                      return;
                                    }
                                  } catch {
                                    // fall through to text/files
                                  }
                                }
                                const plain = e.dataTransfer.getData("text/plain");
                                if (plain) {
                                  void placeImageOnSlot(item.page, slot, plain);
                                }
                              }}
                            >
                              <title>
                                {slot.label} — Drop image/Craft project or click ({slot.cli || ""})
                              </title>
                            </rect>
                          );
                        }

                        return (
                          <g key={slot.label}>
                            <rect
                              id={isCurrent ? `sb-slot-${slot.label}` : `sb-slot-p${item.page}-${slot.label}`}
                              className={`sb-slot sb-slot-text ${active ? "active" : ""}`}
                              x={b[0]}
                              y={b[1]}
                              width={b[2] - b[0]}
                              height={b[3] - b[1]}
                              role="button"
                              tabIndex={0}
                              aria-label={`${slot.label}: click to edit`}
                              onClick={() => {
                                setPageIndex(item.page);
                                setSelectedSlot(slot);
                                setEditing(active ? null : key);
                              }}
                            >
                              <title>{slot.cli || slot.label}</title>
                            </rect>
                            {active &&
                              (() => {
                                const w = Math.min(Math.max(b[2] - b[0], 420), vx1 - vx);
                                const h = Math.min(Math.max(b[3] - b[1], 240), vy1 - vy);
                                const x = Math.max(vx, Math.min(b[0], vx1 - w));
                                const y = Math.max(vy, Math.min(b[1], vy1 - h));
                                return (
                                  <foreignObject className="sb-canvas-editor" x={x} y={y} width={w} height={h}>
                                    <div className="sb-field-editor">
                                      <textarea
                                        id={`sb-canvas-field-${slot.label}`}
                                        aria-label={slot.label}
                                        rows={4}
                                        value={value}
                                        onChange={(event) => change(item.page, slot, event.target.value)}
                                      />
                                      <div style={{ display: "flex", gap: 6 }}>
                                        <button
                                          id={`sb-canvas-save-${slot.label}`}
                                          className="start magic-btn-icon"
                                          disabled={!!busy || !dirty}
                                          onClick={() => void saveTextSlot(item.page, slot)}
                                        >
                                          <span>✓</span>
                                          <span>Save</span>
                                        </button>
                                        <button
                                          id={`sb-canvas-cancel-${slot.label}`}
                                          className="ghost magic-btn-icon"
                                          disabled={!!busy}
                                          onClick={() => setEditing(null)}
                                        >
                                          <span>✕</span>
                                          <span>Close</span>
                                        </button>
                                      </div>
                                    </div>
                                  </foreignObject>
                                );
                              })()}
                          </g>
                        );
                      })}
                    </svg>
                  </div>
                </section>
              );
            })
          ) : (
            <div style={{ padding: 24 }}>
              <p className="muted">No DesignCraft storyboard created yet.</p>
              <button
                id="storyboard-new"
                className="start magic-btn-icon"
                disabled={!!busy}
                onClick={() =>
                  void action(
                    "Creating…",
                    () =>
                      executeCommand("storyboard.new", {
                        path: `${folder}/storyboard.designcraft`,
                        overwrite: false,
                        _expected_project: projectPath,
                      }),
                    true,
                  )
                }
              >
                <span>+</span>
                <span>Create</span>
              </button>
            </div>
          )}
        </div>

        {error && (
          <p role="alert" className="sb-error">
            {error}
          </p>
        )}

        {/* Exact magic-cli Command Bar + Craft Source Project Reference */}
        <div className="sb-cli-bar">
          <code className="sb-cli-code" title={activeCli}>
            {activeCli}
          </code>
          <div style={{ display: "flex", gap: 6 }}>
            {selectedSlot?.reference?.source_project && (
              <button
                id="sb-open-source-project"
                type="button"
                className="ghost magic-btn-icon"
                title={`Open underlying Craft project: ${selectedSlot.reference.source_project}`}
                onClick={() =>
                  void executeCommand("workspace.open", {
                    path: selectedSlot.reference?.source_project,
                  }).catch((e) => onToast(String(e)))
                }
              >
                <span>↗</span>
                <span>Source</span>
              </button>
            )}
            <button
              id="sb-copy-cli"
              type="button"
              className="ghost magic-btn-icon"
              onClick={() => {
                void navigator.clipboard?.writeText(activeCli);
                onToast("Copied magic-cli command");
              }}
            >
              <span>⎘</span>
              <span>Copy</span>
            </button>
          </div>
        </div>
      </main>

      {/* Right Rail: Traffic-Light Section Readiness & Quick-Jump Navigator */}
      <aside className="sb-panel sb-traffic-panel" aria-label="Storyboard section readiness">
        <div className="sb-traffic-group">
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <strong style={{ fontSize: 12 }}>1. Title Sections</strong>
            <button
              id="storyboard-wizard-toggle"
              type="button"
              className="ghost magic-btn-icon"
              onClick={() => setWizardOpen((v) => !v)}
              title="Guided setup wizard for Main Title Pages"
            >
              <span>✦</span>
              <span>Wizard</span>
            </button>
          </div>

          {wizardOpen && (
            <div style={{ display: "flex", flexDirection: "column", gap: 5, padding: "6px 0" }}>
              <input
                placeholder="Core visual style…"
                value={wizStyle}
                onChange={(e) => setWizStyle(e.target.value)}
              />
              <input
                placeholder="World & locations…"
                value={wizWorld}
                onChange={(e) => setWizWorld(e.target.value)}
              />
              <input
                placeholder="Lead character…"
                value={wizChar}
                onChange={(e) => setWizChar(e.target.value)}
              />
              <input
                placeholder="Lyrics & motion rules…"
                value={wizMotion}
                onChange={(e) => setWizMotion(e.target.value)}
              />
              <button
                id="storyboard-wizard-apply"
                type="button"
                className="start magic-btn-icon"
                disabled={!!busy}
                onClick={() =>
                  void action(
                    "Applying…",
                    async () => {
                      await executeCommand("storyboard.wizard", {
                        style: wizStyle,
                        world: wizWorld,
                        character: wizChar,
                        motion: wizMotion,
                        _expected_project: projectPath,
                      });
                      setWizardOpen(false);
                      onToast("Updated Main Title Pages");
                    },
                    true,
                  )
                }
              >
                <span>✓</span>
                <span>Apply</span>
              </button>
            </div>
          )}

          {mainTitleChecks.map((item) => (
            <button
              key={item.page}
              id={`sb-traffic-${item.page}`}
              type="button"
              className={`sb-traffic-row ${page?.page === item.page ? "active" : ""}`}
              onClick={() => scrollToPage(item.page, item.unfilled[0]?.label)}
              title={
                item.unfilled.length
                  ? `${item.unfilled.length} unfilled · ${item.unfilled[0]?.label}`
                  : "Complete"
              }
            >
              <span style={{ display: "inline-flex", alignItems: "center", gap: 7 }}>
                <TrafficDot light={item.traffic_light} />
                <span>{item.section || item.archetype}</span>
              </span>
              <span className="muted" style={{ fontSize: 11 }}>
                {item.filled_slots ?? 0}/{item.total_slots ?? 0}
              </span>
            </button>
          ))}
        </div>

        <div className="sb-traffic-group">
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <strong style={{ fontSize: 12 }}>2. Bulk Reference Art</strong>
            <TrafficDot light={check.main_sections_complete ? "green" : "amber"} />
          </div>
          <p className="muted" style={{ margin: 0, fontSize: 11 }}>
            {check.main_sections_complete
              ? "Title sections ready — generate reference plates & characters."
              : "Complete Title Sections above to unlock bulk art."}
          </p>
          <button
            id="storyboard-bulk-generate"
            type="button"
            className="ghost magic-btn-icon"
            disabled={!!busy || !check.main_sections_complete}
            onClick={() =>
              void action(
                "Generating…",
                async () => {
                  await executeCommand("storyboard.bulk_generate", {
                    _expected_project: projectPath,
                  });
                  onToast("Generated bulk reference images into Storyboard");
                },
                true,
              )
            }
          >
            <span>⊞</span>
            <span>Bulk</span>
          </button>
        </div>

        <div className="sb-traffic-group" style={{ borderBottom: "none", flex: 1 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <strong style={{ fontSize: 12 }}>3. Scene Spreads</strong>
            <TrafficDot light={check.traffic_light} />
          </div>
          {sceneChecks.map((item) => (
            <button
              key={item.page}
              id={`sb-traffic-scene-${item.page}`}
              type="button"
              className={`sb-traffic-row ${page?.page === item.page ? "active" : ""}`}
              onClick={() => scrollToPage(item.page, item.unfilled[0]?.label)}
            >
              <span style={{ display: "inline-flex", alignItems: "center", gap: 7 }}>
                <TrafficDot light={item.traffic_light} />
                <span>
                  P{item.page + 1} · {item.title || item.section || "Scene"}
                </span>
              </span>
              <span className="muted" style={{ fontSize: 11 }}>
                {item.unfilled.length ? `${item.unfilled.length} left` : "Ready"}
              </span>
            </button>
          ))}

          <div style={{ marginTop: "auto", paddingTop: 8 }}>
            <button
              id="storyboard-go-studio"
              type="button"
              className="start magic-btn-icon"
              style={{ width: "100%" }}
              disabled={!check.ready_for_video_production || !onNavigateStudio}
              onClick={() => onNavigateStudio?.()}
              title={
                check.ready_for_video_production
                  ? "All Storyboard sections green — proceed to Video Production"
                  : "Complete red/amber Storyboard sections before starting Video Production"
              }
            >
              <span>▶</span>
              <span>Studio</span>
            </button>
          </div>
        </div>
      </aside>
    </div>
  );
};

export function PreservedStoryboard(props: Props & { active: boolean }) {
  const [opened, setOpened] = useState(props.active);
  useEffect(() => {
    if (props.active) setOpened(true);
  }, [props.active]);
  return opened || props.active ? (
    <div className="sb-preserved" hidden={!props.active}>
      <StoryboardView {...props} />
    </div>
  ) : null;
}
