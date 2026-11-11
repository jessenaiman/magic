import React, { useEffect, useRef, useState } from "react";
import { StudioFilmstrips } from "../components/StudioFilmstrips";
import { FilmExportButton } from "../components/FilmExportButton";
import {
  Button,
  ControlGroup,
  IconButton,
  InputControl,
  SelectControl,
  Toolbar,
} from "../components/controls";
import {
  executeCommand,
  fileUrl,
  FilmStatusResponse,
  TimelineResponse,
} from "../api";

interface FilmViewProps {
  projectPath: string;
  onToast: (msg: string) => void;
}

export const FilmView: React.FC<FilmViewProps> = ({ projectPath, onToast }) => {
  const [film, setFilm] = useState<FilmStatusResponse | null>(null);
  const [timeline, setTimeline] = useState<TimelineResponse | null>(null);
  const [statusText, setStatusText] = useState<string>("Loading film…");
  const [busy, setBusy] = useState<boolean>(false);
  const [settingsOpen, setSettingsOpen] = useState<boolean>(false);
  const [exportPreset, setExportPreset] = useState<string>("");
  const [exportPresets, setExportPresets] = useState<Array<{ id: string; name?: string }>>([
    { id: "youtube_1080p", name: "YouTube 1080p (H.264)" },
    { id: "h265_web", name: "Web 4K / HEVC" },
    { id: "prores_master", name: "Apple ProRes 422 HQ Master" },
    { id: "dnxhr_hq", name: "Avid DNxHR HQ Master" },
    { id: "audio_wav", name: "Broadcast WAV Master" },
  ]);
  const [exportFormat, setExportFormat] = useState<string>("h264");
  const [exportScale, setExportScale] = useState<number>(1);
  const [exportQuality, setExportQuality] = useState<number>(85);
  const [gpuRendering, setGpuRendering] = useState<string>("auto");
  const [includeAudio, setIncludeAudio] = useState<boolean>(true);
  const [allowOffline, setAllowOffline] = useState<boolean>(false);
  const [burnCaptions, setBurnCaptions] = useState<boolean>(false);
  const [captionSidecar, setCaptionSidecar] = useState<string>("");
  const [exportRange, setExportRange] = useState<string>("entire");
  const [customStart, setCustomStart] = useState<number>(0);
  const [customEnd, setCustomEnd] = useState<number>(30);
  const [queueExport, setQueueExport] = useState<boolean>(false);
  const [exportSettingsEdited, setExportSettingsEdited] = useState(false);
  const [playhead, setPlayhead] = useState<number>(0);
  const videoRef = useRef<HTMLVideoElement | null>(null);

  const refreshFilm = async () => {
    try {
      const f = await executeCommand<FilmStatusResponse>("video.film");
      setFilm(f);
      setStatusText(
        f.preview?.exists
          ? f.preview_stale
            ? "Preview loaded (timeline updated since last render — click Preview to refresh)"
            : "Preview ready"
          : f.export?.exists
          ? "Export ready"
          : f.timeline?.exists
          ? "Timeline ready — scrubbing live sequence frames (click Preview for full audio/video draft)"
          : "No film timeline yet — click Assemble to build .fcproj",
      );
      if (f.timeline?.exists) {
        const tl = await executeCommand<TimelineResponse>("video.timeline");
        setTimeline(tl);
        try {
          const presetsRes = await executeCommand<{ result?: Array<{ id?: string; name?: string }> }>("video.exec", {
            id: "export.presets.list",
            params: {},
            save: false,
          });
          if (Array.isArray(presetsRes?.result) && presetsRes.result.length > 0) {
            setExportPresets(
              presetsRes.result
                .filter((item) => typeof item?.id === "string" && item.id.length > 0)
                .map((item) => ({ id: String(item.id), name: item.name ? String(item.name) : String(item.id) })),
            );
          }
        } catch {
          // Keep default preset list if sequence preset query is unavailable.
        }
      } else {
        setTimeline(null);
      }
    } catch (err) {
      setStatusText(String(err));
    }
  };

  useEffect(() => {
    setExportSettingsEdited(false);
    void refreshFilm();
  }, [projectPath]);

  const runFilmAction = async (label: string, fn: () => Promise<unknown>) => {
    if (busy) return;
    setBusy(true);
    setStatusText(`${label}…`);
    try {
      await fn();
      await refreshFilm();
      setStatusText(`${label} complete`);
      onToast(`${label} complete`);
    } catch (err) {
      const msg = String(err);
      setStatusText(msg);
      onToast(msg);
    } finally {
      setBusy(false);
    }
  };

  const handleExportInterchange = async (fmt: string) => {
    if (!film?.timeline?.path) return;
    const outPath = film.timeline.path.replace(/\.fcproj$/, `.${fmt}`);
    await runFilmAction(`Export ${fmt.toUpperCase()}`, () =>
      executeCommand("video.exec", {
        id: "file.exportInterchange",
        params: { format: fmt, path: outPath },
        save: false,
      }),
    );
  };

  const videoSrc = film?.preview?.exists
    ? `${fileUrl(film.preview.path)}?v=${film.preview.modified || 0}`
    : film?.export?.exists
      ? `${fileUrl(film.export.path)}?v=${film.export.modified || 0}`
      : "";

  return (
    <div id="view-studio" className="film-view film-view-layout">
      <Toolbar id="film-head" align="between" className="film-head">
        <div>
          <strong className="film-title">FilmCraft Studio</strong>
          <p id="film-status" className="muted film-status" role="status">
            {statusText}
          </p>
        </div>

        <ControlGroup className="film-actions">
          <Button
            id="film-assemble"
            variant="ghost"
            icon="grid"
            label="Assemble"
            title="Assemble FilmCraft timeline (A1 Song, V1 Scenes, V2 Character, V3 Lyrics, V4 Motion)"
            disabled={busy}
            onClick={() =>
              void runFilmAction("Assemble", () =>
                executeCommand("video.assemble", { export: false }),
              )
            }
          />
          <Button
            id="film-preview"
            variant="primary"
            icon="play"
            label="Preview"
            title="Render half-scale FilmCraft sequence preview"
            disabled={busy}
            onClick={() =>
              void runFilmAction("Preview", () => executeCommand("video.preview"))
            }
          />
          <Button
            id="film-frame"
            variant="ghost"
            icon="camera"
            label="Frame"
            title={`Export frame at ${playhead.toFixed(1)}s via FilmCraft`}
            disabled={busy || !film?.timeline?.exists}
            onClick={() =>
              void runFilmAction(`Export Frame @ ${playhead.toFixed(1)}s`, () =>
                executeCommand("video.frame", { seconds: playhead }),
              )
            }
          />
          <Button
            id="film-open"
            variant="ghost"
            icon="external"
            label="FilmCraft"
            title="Open linked .fcproj in live FilmCraft (--control)"
            disabled={busy || !film?.timeline?.exists}
            onClick={() =>
              void runFilmAction("Open in FilmCraft", () => executeCommand("video.open"))
            }
          />
          <FilmExportButton
            project={projectPath}
            disabled={busy || !film?.timeline?.exists}
            onBusy={setBusy}
            onToast={onToast}
            options={
              exportSettingsEdited
                ? {
                    ...(exportPreset ? { preset: exportPreset } : {}),
                    format: exportFormat,
                    range: exportRange === "custom" ? "entire" : exportRange,
                    ...(exportRange === "custom"
                      ? { startSeconds: customStart, endSeconds: customEnd }
                      : {}),
                    scale: exportScale,
                    quality: exportQuality,
                    gpuRendering,
                    audio: includeAudio,
                    allowOffline,
                    burnCaptions,
                    queue: queueExport,
                    captionSidecar: captionSidecar || null,
                  }
                : {}
            }
          />
          <IconButton
            id="film-export-settings-toggle"
            icon="sliders"
            label="Options"
            variant="ghost"
            active={settingsOpen}
            aria-expanded={settingsOpen}
            title="Export settings and NLE interchange formats"
            onClick={() => setSettingsOpen((v) => !v)}
          />
        </ControlGroup>
      </Toolbar>

      {settingsOpen && (
        <div id="film-export-settings" className="film-export-settings">
          <label className="film-inline-field">
            <span>Preset</span>
            <SelectControl
              id="film-export-preset"
              controlSize="sm"
              value={exportPreset}
              onChange={(e) => {
                setExportPreset(e.target.value);
                setExportSettingsEdited(true);
              }}
            >
              <option value="">Custom</option>
              {exportPresets.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.name || preset.id}
                </option>
              ))}
            </SelectControl>
          </label>

          <label className="film-inline-field">
            <span>Format</span>
            <SelectControl
              id="film-export-format"
              controlSize="sm"
              value={exportFormat}
              onChange={(e) => {
                setExportFormat(e.target.value);
                setExportSettingsEdited(true);
              }}
            >
              <option value="h264">H.264 (.mp4)</option>
              <option value="hevc">HEVC / H.265 (.mp4)</option>
              <option value="prores">Apple ProRes (.mov)</option>
              <option value="dnxhr">Avid DNxHR (.mov)</option>
              <option value="apv">APV (.mov)</option>
              <option value="mjpeg">Motion JPEG (.mov)</option>
              <option value="mxf-op1a">MXF OP1a (.mxf)</option>
              <option value="mxf-opatom">MXF OP-Atom (.mxf)</option>
              <option value="png">PNG Sequence (.png)</option>
              <option value="tiff">TIFF Sequence (.tiff)</option>
              <option value="bmp">BMP Sequence (.bmp)</option>
              <option value="gif">Animated GIF (.gif)</option>
              <option value="wav">Audio Master (.wav)</option>
              <option value="aiff">Audio AIFF (.aiff)</option>
              <option value="flac">Lossless Audio (.flac)</option>
              <option value="aac">AAC Audio (.aac)</option>
              <option value="mp3">MP3 Audio (.mp3)</option>
            </SelectControl>
          </label>

          <label className="film-inline-field">
            <span>Range</span>
            <SelectControl
              id="film-export-range"
              controlSize="sm"
              value={exportRange}
              onChange={(e) => {
                setExportRange(e.target.value);
                setExportSettingsEdited(true);
              }}
            >
              <option value="entire">Entire Sequence</option>
              <option value="inOut">In / Out Range</option>
              <option value="workArea">Work Area</option>
              <option value="custom">Custom Seconds</option>
            </SelectControl>
          </label>

          {exportRange === "custom" && (
            <>
              <label className="film-inline-field">
                <span>Start</span>
                <InputControl
                  id="film-export-start"
                  controlSize="sm"
                  type="number"
                  min={0}
                  step={0.1}
                  value={customStart}
                  onChange={(e) => {
                    setCustomStart(Number(e.target.value));
                    setExportSettingsEdited(true);
                  }}
                  style={{ width: "4.5rem" }}
                />
              </label>
              <label className="film-inline-field">
                <span>End</span>
                <InputControl
                  id="film-export-end"
                  controlSize="sm"
                  type="number"
                  min={0.1}
                  step={0.1}
                  value={customEnd}
                  onChange={(e) => {
                    setCustomEnd(Number(e.target.value));
                    setExportSettingsEdited(true);
                  }}
                  style={{ width: "4.5rem" }}
                />
              </label>
            </>
          )}

          <label className="film-inline-field">
            <span>Scale</span>
            <SelectControl
              id="film-export-scale"
              controlSize="sm"
              value={String(exportScale)}
              onChange={(e) => {
                setExportScale(Number(e.target.value));
                setExportSettingsEdited(true);
              }}
            >
              <option value="1">100%</option>
              <option value="0.5">50%</option>
              <option value="0.25">25%</option>
            </SelectControl>
          </label>

          <label className="film-inline-field">
            <span>Quality</span>
            <InputControl
              id="film-export-quality"
              controlSize="sm"
              type="number"
              min={0}
              max={100}
              value={exportQuality}
              onChange={(e) => {
                setExportQuality(Number(e.target.value));
                setExportSettingsEdited(true);
              }}
              style={{ width: "4.5rem" }}
            />
          </label>

          <label className="film-inline-field">
            <span>GPU</span>
            <SelectControl
              id="film-export-gpu"
              controlSize="sm"
              value={gpuRendering}
              onChange={(e) => {
                setGpuRendering(e.target.value);
                setExportSettingsEdited(true);
              }}
            >
              <option value="auto">Auto (wgpu)</option>
              <option value="off">CPU Reference</option>
            </SelectControl>
          </label>

          <label className="film-inline-field">
            <input
              id="film-export-audio"
              type="checkbox"
              checked={includeAudio}
              onChange={(e) => {
                setIncludeAudio(e.target.checked);
                setExportSettingsEdited(true);
              }}
            />
            <span>Audio</span>
          </label>

          <label className="film-inline-field">
            <input
              id="film-allow-offline"
              type="checkbox"
              checked={allowOffline}
              onChange={(e) => {
                setAllowOffline(e.target.checked);
                setExportSettingsEdited(true);
              }}
            />
            <span>Offline</span>
          </label>

          <label className="film-inline-field">
            <input
              id="film-burn-captions"
              type="checkbox"
              checked={burnCaptions}
              onChange={(e) => {
                setBurnCaptions(e.target.checked);
                setExportSettingsEdited(true);
              }}
            />
            <span>Captions</span>
          </label>

          <label className="film-inline-field">
            <span>Sidecar</span>
            <SelectControl
              id="film-caption-sidecar"
              controlSize="sm"
              value={captionSidecar}
              onChange={(e) => {
                setCaptionSidecar(e.target.value);
                setExportSettingsEdited(true);
              }}
            >
              <option value="">None</option>
              <option value="srt">.srt</option>
              <option value="vtt">.vtt</option>
            </SelectControl>
          </label>

          <label className="film-inline-field" title="Add to FilmCraft export queue">
            <input
              id="film-export-queue"
              type="checkbox"
              checked={queueExport}
              onChange={(e) => {
                setQueueExport(e.target.checked);
                setExportSettingsEdited(true);
              }}
            />
            <span>Queue</span>
          </label>

          <ControlGroup className="film-export-actions">
            {(
              [
                ["fcpxml", "FCPXML", "Export Final Cut Pro FCPXML via FilmCraft"],
                ["xml", "XML", "Export Final Cut Pro 7 XML (xmeml) via FilmCraft"],
                ["otio", "OTIO", "Export OpenTimelineIO via FilmCraft"],
                ["edl", "EDL", "Export CMX3600 EDL via FilmCraft"],
                ["aaf", "AAF", "Export AAF via FilmCraft"],
                ["omf", "OMF", "Export OMF 2.0 Audio via FilmCraft"],
              ] as const
            ).map(([fmt, lbl, tip]) => (
              <Button
                key={fmt}
                id={`film-export-${fmt}`}
                size="sm"
                variant="ghost"
                icon="download"
                label={lbl}
                title={tip}
                disabled={busy || !film?.timeline?.exists}
                onClick={() => void handleExportInterchange(fmt)}
              />
            ))}
          </ControlGroup>
        </div>
      )}

      <StudioFilmstrips
        project={projectPath}
        timeline={timeline}
        videoSrc={videoSrc}
        revision={film?.timeline?.modified || 0}
        playhead={playhead}
        onTime={setPlayhead}
        videoRef={videoRef}
      />
    </div>
  );
};
