import { useEffect, useMemo, useRef, useState } from "react";

import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Progress } from "./components/ui/progress";
import { Textarea } from "./components/ui/textarea";
import { createEcoApi, type EcoStatus, type ModelState } from "./lib/ecoApi";
import { cn } from "./lib/utils";

const statusLabels: Record<EcoStatus, string> = {
  idle: "Idle",
  recording: "Recording",
  processing: "Transcribing",
  error: "Error",
};

const statusColors: Record<EcoStatus, string> = {
  idle: "text-success",
  recording: "text-danger",
  processing: "text-accent",
  error: "text-danger",
};

const formatModelSize = (sizeMb: number) => {
  if (!Number.isFinite(sizeMb)) return "--";
  if (sizeMb < 1024) return `${Math.round(sizeMb)} MB`;
  const sizeGb = sizeMb / 1024;
  return `${sizeGb.toFixed(sizeGb >= 10 ? 1 : 2)} GB`;
};

function App() {
  const api = useMemo(() => createEcoApi(), []);
  const [models, setModels] = useState<ModelState[]>([]);
  const [shortcut, setShortcut] = useState("Ctrl+Alt+Space");
  const [status, setStatus] = useState<EcoStatus>("idle");
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [downloads, setDownloads] = useState<Record<string, number>>({});
  const [testText, setTestText] = useState("");
  const [lastTranscript, setLastTranscript] = useState("");
  const [lastDurationMs, setLastDurationMs] = useState<number | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [isSavingShortcut, setIsSavingShortcut] = useState(false);

  const activeModel = models.find((model) => model.active);

  const refreshModels = async () => {
    const list = await api.listModels();
    setModels(list);
  };

  useEffect(() => {
    const loadConfig = async () => {
      try {
        const config = await api.getConfig();
        setShortcut(config.shortcut);
      } catch (error) {
        setStatus("error");
        const message =
          error instanceof Error
            ? error.message
            : "Tauri is unavailable. Open the desktop app.";
        setStatusMessage(message);
      }
    };
    const loadModels = async () => {
      try {
        await refreshModels();
      } catch (error) {
        const message =
          error instanceof Error
            ? error.message
            : "Models could not be loaded. Try again.";
        setStatusMessage(message);
      }
    };
    loadConfig();
    loadModels();

    const stopStatus = api.onStatus((payload) => {
      setStatus(payload.status);
      setStatusMessage(payload.message ?? null);
    });
    const stopProgress = api.onProgress((payload) => {
      if (payload.done) {
        setDownloads((prev) => {
          const next = { ...prev };
          delete next[payload.id];
          return next;
        });
        refreshModels().catch(() => undefined);
        return;
      }
      setDownloads((prev) => ({
        ...prev,
        [payload.id]: payload.ratio * 100,
      }));
    });
    const stopTranscription = api.onTranscription((payload) => {
      setLastTranscript(payload.text);
      setLastDurationMs(payload.durationMs ?? null);
      const active = document.activeElement;
      const isFocusedTextarea =
        active instanceof HTMLTextAreaElement && active.dataset.testid === "test-textarea";
      if (isFocusedTextarea) {
        return;
      }
      setTestText((current) => (current ? `${current}\n${payload.text}` : payload.text));
    });

    return () => {
      stopStatus();
      stopProgress();
      stopTranscription();
    };
  }, [api]);

  const handleSaveShortcut = async () => {
    setIsSavingShortcut(true);
    try {
      await api.setShortcut(shortcut.trim());
    } finally {
      setIsSavingShortcut(false);
    }
  };

  const handleDownload = async (id: string) => {
    setDownloads((prev) => ({ ...prev, [id]: 0 }));
    await api.downloadModel(id);
    await refreshModels();
  };

  const handleDelete = async (id: string) => {
    const confirmed = window.confirm(
      "Deleting this model removes its local files. Continue only if you want to free them."
    );
    if (!confirmed) return;
    await api.deleteModel(id);
    await refreshModels();
  };

  const handleUseModel = async (id: string) => {
    await api.setActiveModel(id);
    await refreshModels();
  };

  const handleToggleRecording = async () => {
    try {
      await api.toggleRecording();
    } catch (error) {
      setStatus("error");
      const message =
        error instanceof Error
          ? error.message
          : "Recording could not start. Try again.";
      setStatusMessage(message);
    }
  };

  const handleCopy = async () => {
    if (!testText.trim()) return;
    try {
      await navigator.clipboard.writeText(testText);
    } catch {
      setLastTranscript("Copy failed. Select the text and copy manually.");
    }
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <div className="geist-grid">
        <div className="mx-auto flex max-w-6xl flex-col gap-8 px-5 py-8 sm:px-6 sm:py-10">
          <header className="flex flex-col gap-6">
            <div className="flex flex-wrap items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="flex h-10 w-10 items-center justify-center rounded-md border border-border bg-card p-1">
                  <img src="/ECO-logo.svg" alt="ECO" className="h-full w-full rounded-md" />
                </div>
                <div className="min-w-0">
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Status</p>
                  <div className="flex items-center gap-2 text-sm font-medium">
                    <span
                      aria-hidden="true"
                      className={cn(
                        "status-dot",
                        statusColors[status],
                        status === "recording" ? "animate-status-pulse" : ""
                      )}
                    />
                    <span>{statusLabels[status]}</span>
                  </div>
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-3">
                <Button data-testid="dictate-toggle" onClick={handleToggleRecording}>
                  {status === "recording" ? "Stop Dictation" : "Start Dictation"}
                </Button>
                <div className="rounded-full border border-border bg-card px-3 py-1 text-xs text-muted">
                  Shortcut: <span className="font-medium text-foreground">{shortcut}</span>
                </div>
              </div>
            </div>
            <div className="flex flex-col gap-3">
              <h1 className="text-balance text-4xl font-semibold text-foreground">
                Local dictation without friction.
              </h1>
              <p className="text-pretty text-base text-muted">
                Eco transcribes in the background. Control the shortcut, review text, and manage
                models without breaking flow.
              </p>
              {statusMessage && (
                <p className="text-sm text-danger" data-testid="status-message" aria-live="polite">
                  {statusMessage}
                </p>
              )}
            </div>
          </header>

          <main id="main" className="grid gap-6 lg:grid-cols-[1.35fr_0.9fr]">
            <section className="flex flex-col gap-4">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Text</p>
                  <p className="text-sm text-muted">Edit, copy, and clear in one place.</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button data-testid="copy-button" variant="secondary" onClick={handleCopy}>
                    Copy
                  </Button>
                  <Button
                    data-testid="clear-button"
                    variant="outline"
                    onClick={() => setTestText("")}
                  >
                    Clear
                  </Button>
                </div>
              </div>
              <div className="rounded-xl border border-border bg-background-2 p-4 shadow-subtle">
                <label htmlFor="test-textarea" className="text-xs font-semibold text-foreground">
                  Draft Area
                </label>
                <Textarea
                  id="test-textarea"
                  name="transcription"
                  data-testid="test-textarea"
                  ref={textareaRef}
                  value={testText}
                  onChange={(event) => setTestText(event.currentTarget.value)}
                  placeholder="Transcribed text will appear here…"
                />
              </div>
              <div className="rounded-xl border border-border bg-background-2 px-4 py-3 text-xs text-muted">
                <span className="font-semibold text-foreground">Last dictation:</span>{" "}
                {lastTranscript || "No text yet."}{" "}
                <span className="tabular-nums">
                  {lastDurationMs !== null
                    ? `· ${(lastDurationMs / 1000).toFixed(2)} s`
                    : "· --"}
                </span>
              </div>
            </section>

            <aside className="flex flex-col gap-6">
              <div className="rounded-xl border border-border bg-background-2 p-4 shadow-subtle">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Shortcut</p>
                    <p className="text-sm text-muted">Configure your global combo.</p>
                  </div>
                  <Badge variant="default">System</Badge>
                </div>
                <div className="mt-4 space-y-3">
                  <label htmlFor="shortcut" className="text-xs font-semibold text-foreground">
                    Current Shortcut
                  </label>
                  <Input
                    id="shortcut"
                    name="shortcut"
                    data-testid="shortcut-input"
                    value={shortcut}
                    autoComplete="off"
                    spellCheck={false}
                    onChange={(event) => setShortcut(event.currentTarget.value)}
                    placeholder="e.g., Ctrl+Alt+Space…"
                  />
                  <div className="flex flex-wrap gap-2">
                    <Button
                      data-testid="shortcut-save"
                      variant="primary"
                      onClick={handleSaveShortcut}
                      disabled={!shortcut.trim() || isSavingShortcut}
                    >
                      {isSavingShortcut ? "Saving…" : "Save"}
                    </Button>
                    <Button variant="outline" onClick={() => setShortcut("Ctrl+Alt+Space")}>
                      Reset
                    </Button>
                  </div>
                </div>
              </div>

              <div className="rounded-xl border border-border bg-background-2 p-4 shadow-subtle">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Models</p>
                    <p className="text-sm text-muted">
                      Active: {activeModel ? activeModel.title : "No Model"}
                    </p>
                  </div>
                  <Badge variant="default">Whisper</Badge>
                </div>
                <div className="mt-4 grid gap-3">
                  {models.map((model) => {
                    const progress = downloads[model.id];
                    return (
                      <div
                        key={model.id}
                        data-testid={`model-${model.id}`}
                        className="rounded-lg border border-border bg-background-2 p-3"
                      >
                        <div className="flex flex-wrap items-start justify-between gap-3">
                          <div className="min-w-0">
                            <p className="truncate text-sm font-semibold text-foreground">
                              {model.title}
                            </p>
                            <p className="text-xs text-foreground/70 tabular-nums">
                              {formatModelSize(model.sizeMb)}
                            </p>
                          </div>
                          <div className="flex flex-wrap gap-2">
                            {!model.installed && (
                              <Button
                                data-testid={`model-${model.id}-download`}
                                variant="primary"
                                onClick={() => handleDownload(model.id)}
                                disabled={progress !== undefined && progress < 100}
                              >
                                Download
                              </Button>
                            )}
                            {model.installed && !model.active && (
                              <Button
                                data-testid={`model-${model.id}-use`}
                                variant="secondary"
                                onClick={() => handleUseModel(model.id)}
                              >
                                Use
                              </Button>
                            )}
                            {model.installed && (
                              <Button
                                data-testid={`model-${model.id}-delete`}
                                variant="outline"
                                onClick={() => handleDelete(model.id)}
                              >
                                Delete
                              </Button>
                            )}
                          </div>
                        </div>
                        <div className="mt-2 flex flex-wrap gap-2">
                          {model.active && <Badge variant="active">Active</Badge>}
                          {model.partial && !model.installed && <Badge>Incomplete</Badge>}
                        </div>
                        {progress !== undefined && progress < 100 && (
                          <div className="mt-3 w-full space-y-2">
                            <Progress
                              data-testid={`model-${model.id}-progress`}
                              value={progress}
                              className="w-full"
                            />
                            <p className="text-xs text-muted tabular-nums">
                              Downloading… {Math.round(progress)}%
                            </p>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            </aside>
          </main>
        </div>
      </div>
    </div>
  );
}

export default App;
