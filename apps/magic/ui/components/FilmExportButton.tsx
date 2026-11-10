import { useEffect, useRef, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { executeCommand } from "../api";
import { ExportReport, exportIconState } from "../export-state";
import { workspaceDrafts } from "../workspace-drafts";
import { Button } from "./controls";

function editorDrafts() {
  const state = workspaceDrafts();
  return state.dirty || state.saving;
}

export function FilmExportButton({
  project,
  options,
  disabled,
  onBusy,
  onToast,
}: {
  project: string;
  options: Record<string, unknown>;
  disabled: boolean;
  onBusy: (busy: boolean) => void;
  onToast: (message: string) => void;
}) {
  const [report, setReport] = useState<ExportReport | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const [drafts, setDrafts] = useState(false);
  const alive = useRef(true);
  const inFlight = useRef(false);
  const polling = useRef(false);
  const revision = useRef(0);
  const optionsKey = JSON.stringify(options);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  useEffect(() => {
    const version = ++revision.current;
    setReport(null);
    setError("");
    const refresh = async () => {
      setDrafts(editorDrafts());
      if (polling.current || inFlight.current) return;
      polling.current = true;
      try {
        const value = await executeCommand<ExportReport>("video.film", {
          ...JSON.parse(optionsKey),
          _expected_project: project,
        });
        if (alive.current && revision.current === version) setReport(value);
      } catch {
        if (alive.current && revision.current === version) setReport(null);
      } finally {
        polling.current = false;
      }
    };
    void refresh();
    const timer = window.setInterval(() => void refresh(), 3000);
    return () => {
      ++revision.current;
      window.clearInterval(timer);
    };
  }, [project, optionsKey]);

  const status = exportIconState(report, running, error, drafts);
  const tooltip =
    status === "running"
      ? "Rendering final video via FilmCraft"
      : status === "ready"
        ? "Open exported film"
        : status === "error"
          ? "Export failed · click to retry"
          : "Export final video via FilmCraft";

  const activate = async () => {
    if (inFlight.current) return;
    if (editorDrafts()) {
      onToast("Save the editor changes before exporting the final video");
      return;
    }
    if (status === "ready" && report?.export?.path) {
      try {
        await openPath(report.export.path);
      } catch (failure) {
        onToast(String(failure));
      }
      return;
    }
    const version = revision.current;
    inFlight.current = true;
    setRunning(true);
    setError("");
    onBusy(true);
    try {
      await executeCommand("video.export", {
        ...JSON.parse(optionsKey),
        _expected_project: project,
      });
      const checked = await executeCommand<ExportReport>("video.film", {
        ...JSON.parse(optionsKey),
        _expected_project: project,
      });
      if (exportIconState(checked, false, "", false) !== "ready") {
        throw new Error(
          checked.validation?.reason ||
            checked.validation?.error ||
            "Export finished but output validation did not pass",
        );
      }
      if (alive.current && revision.current === version) {
        setReport(checked);
        onToast("Export ready · click Export to open the video");
      }
    } catch (failure) {
      if (alive.current && revision.current === version) {
        setError(String(failure));
        onToast(String(failure));
      }
    } finally {
      inFlight.current = false;
      if (alive.current) {
        setRunning(false);
        onBusy(false);
      }
    }
  };

  return (
    <Button
      id="film-export"
      variant={status === "ready" ? "primary" : status === "error" ? "danger" : "ghost"}
      icon={status === "ready" ? "check" : "upload"}
      label="Export"
      disabled={disabled || running}
      aria-label={tooltip}
      aria-busy={running}
      title={error || tooltip}
      data-export-state={status}
      onClick={() => void activate()}
    />
  );
}
