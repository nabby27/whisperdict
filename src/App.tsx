import { useEffect, useMemo, useRef, useState } from "react";

import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Progress } from "./components/ui/progress";
import { Textarea } from "./components/ui/textarea";
import { createEcoApi, type EcoStatus, type ModelState } from "./lib/ecoApi";
import { cn } from "./lib/utils";

const statusLabels: Record<EcoStatus, string> = {
  idle: "En Reposo",
  recording: "Grabando",
  processing: "Transcribiendo",
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
    api
      .getConfig()
      .then((config) => setShortcut(config.shortcut))
      .catch((error) => {
        setStatus("error");
        const message =
          error instanceof Error
            ? error.message
            : "Tauri no está disponible. Abre la app de escritorio.";
        setStatusMessage(message);
      });
    refreshModels().catch((error) => {
      const message =
        error instanceof Error
          ? error.message
          : "No se pudieron cargar los modelos. Intenta de nuevo.";
      setStatusMessage(message);
    });

    const stopStatus = api.onStatus((payload) => {
      setStatus(payload.status);
      setStatusMessage(payload.message ?? null);
    });
    const stopProgress = api.onProgress((payload) => {
      setDownloads((prev) => ({
        ...prev,
        [payload.id]: payload.ratio * 100,
      }));
      if (payload.ratio >= 1) {
        refreshModels().catch(() => undefined);
      }
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
    setDownloads((prev) => ({ ...prev, [id]: 1 }));
    await api.downloadModel(id);
    await refreshModels();
  };

  const handleDelete = async (id: string) => {
    const confirmed = window.confirm(
      "Eliminar este modelo borrará sus archivos locales. Continúa solo si quieres liberarlos."
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
          : "No se pudo iniciar el dictado. Intenta de nuevo.";
      setStatusMessage(message);
    }
  };

  const handleCopy = async () => {
    if (!testText.trim()) return;
    try {
      await navigator.clipboard.writeText(testText);
    } catch {
      setLastTranscript("No se pudo copiar. Selecciona el texto y copia manualmente.");
    }
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <a className="skip-link" href="#main">
        Saltar al contenido
      </a>
      <div className="geist-grid">
        <div className="mx-auto flex max-w-6xl flex-col gap-8 px-5 py-8 sm:px-6 sm:py-10">
          <header className="flex flex-col gap-6">
            <div className="flex flex-wrap items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="flex h-10 w-10 items-center justify-center rounded-md border border-border bg-card text-xs font-semibold">
                  ECO
                </div>
                <div className="min-w-0">
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Estado</p>
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
                  {status === "recording" ? "Detener Dictado" : "Iniciar Dictado"}
                </Button>
                <div className="rounded-full border border-border bg-card px-3 py-1 text-xs text-muted">
                  Atajo: <span className="font-medium text-foreground">{shortcut}</span>
                </div>
              </div>
            </div>
            <div className="flex flex-col gap-3">
              <h1 className="text-balance text-4xl font-semibold text-foreground">
                Dictado local, sin fricción.
              </h1>
              <p className="text-pretty text-base text-muted">
                Eco transcribe tu voz en segundo plano. Controla el atajo, revisa el texto y gestiona
                modelos sin salir de tu flujo.
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
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Texto</p>
                  <p className="text-sm text-muted">Edita, copia y limpia en un solo lugar.</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button data-testid="copy-button" variant="secondary" onClick={handleCopy}>
                    Copiar
                  </Button>
                  <Button
                    data-testid="clear-button"
                    variant="outline"
                    onClick={() => setTestText("")}
                  >
                    Limpiar
                  </Button>
                </div>
              </div>
              <div className="rounded-xl border border-border bg-background-2 p-4 shadow-subtle">
                <label htmlFor="test-textarea" className="text-xs font-semibold text-foreground">
                  Área de Prueba
                </label>
                <Textarea
                  id="test-textarea"
                  name="transcription"
                  data-testid="test-textarea"
                  ref={textareaRef}
                  value={testText}
                  onChange={(event) => setTestText(event.currentTarget.value)}
                  placeholder="El texto transcrito aparecerá aquí…"
                />
              </div>
              <div className="rounded-xl border border-border bg-background-2 px-4 py-3 text-xs text-muted">
                <span className="font-semibold text-foreground">Último dictado:</span>{" "}
                {lastTranscript || "Aún no hay texto."}{" "}
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
                    <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Atajo</p>
                    <p className="text-sm text-muted">Configura tu combinación global.</p>
                  </div>
                  <Badge variant="default">Sistema</Badge>
                </div>
                <div className="mt-4 space-y-3">
                  <label htmlFor="shortcut" className="text-xs font-semibold text-foreground">
                    Atajo Actual
                  </label>
                  <Input
                    id="shortcut"
                    name="shortcut"
                    data-testid="shortcut-input"
                    value={shortcut}
                    autoComplete="off"
                    spellCheck={false}
                    onChange={(event) => setShortcut(event.currentTarget.value)}
                    placeholder="Ej.: Ctrl+Alt+Space…"
                  />
                  <div className="flex flex-wrap gap-2">
                    <Button
                      data-testid="shortcut-save"
                      variant="primary"
                      onClick={handleSaveShortcut}
                      disabled={!shortcut.trim() || isSavingShortcut}
                    >
                      {isSavingShortcut ? "Guardando…" : "Guardar"}
                    </Button>
                    <Button variant="outline" onClick={() => setShortcut("Ctrl+Alt+Space")}>
                      Restablecer
                    </Button>
                  </div>
                </div>
              </div>

              <div className="rounded-xl border border-border bg-background-2 p-4 shadow-subtle">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-[11px] uppercase tracking-[0.24em] text-muted">Modelos</p>
                    <p className="text-sm text-muted">
                      Activo: {activeModel ? activeModel.title : "Sin Modelo"}
                    </p>
                  </div>
                  <Badge variant="default">Whisper</Badge>
                </div>
                <div className="mt-4 grid gap-3">
                  {models.map((model) => {
                    const progress = downloads[model.id] || 0;
                    return (
                      <div
                        key={model.id}
                        data-testid={`model-${model.id}`}
                        className="rounded-lg border border-border bg-background-2 p-3"
                      >
                        <div className="flex flex-wrap items-center justify-between gap-3">
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
                                disabled={progress > 0 && progress < 100}
                              >
                                Descargar
                              </Button>
                            )}
                            {model.installed && !model.active && (
                              <Button
                                data-testid={`model-${model.id}-use`}
                                variant="secondary"
                                onClick={() => handleUseModel(model.id)}
                              >
                                Usar
                              </Button>
                            )}
                            {model.installed && (
                              <Button
                                data-testid={`model-${model.id}-delete`}
                                variant="outline"
                                onClick={() => handleDelete(model.id)}
                              >
                                Eliminar
                              </Button>
                            )}
                          </div>
                        </div>
                        {model.active && <Badge variant="active">En Uso</Badge>}
                        {model.partial && !model.installed && <Badge>Incompleto</Badge>}
                        {progress > 0 && progress < 100 && (
                          <div className="mt-3 space-y-2">
                            <Progress data-testid={`model-${model.id}-progress`} value={progress} />
                            <p className="text-xs text-muted tabular-nums">
                              Descargando… {Math.round(progress)}%
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
