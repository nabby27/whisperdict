import { useEffect, useMemo, useRef, useState } from "react";

import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { Input } from "./components/ui/input";
import { Progress } from "./components/ui/progress";
import { Separator } from "./components/ui/separator";
import { Textarea } from "./components/ui/textarea";
import { createEcoApi, type EcoStatus, type ModelState } from "./lib/ecoApi";
import { cn } from "./lib/utils";

const statusLabels: Record<EcoStatus, string> = {
  idle: "Reposo",
  recording: "Grabando",
  processing: "Transcribiendo",
  error: "Error",
};

const statusColors: Record<EcoStatus, string> = {
  idle: "text-sage",
  recording: "text-ember",
  processing: "text-clay",
  error: "text-red-600",
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
        const message = error instanceof Error ? error.message : "Tauri no disponible.";
        setStatusMessage(message);
      });
    refreshModels().catch((error) => {
      const message = error instanceof Error ? error.message : "No se pudieron cargar modelos.";
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
      const message = error instanceof Error ? error.message : "No se pudo iniciar el dictado.";
      setStatusMessage(message);
    }
  };

  const handleCopy = async () => {
    if (!testText.trim()) return;
    try {
      await navigator.clipboard.writeText(testText);
    } catch {
      setLastTranscript("No se pudo copiar al portapapeles.");
    }
  };

  return (
    <div className="eco-grid min-h-screen px-6 py-10">
      <div className="mx-auto flex max-w-6xl flex-col gap-8">
        <header className="eco-panel flex flex-col gap-6 rounded-[32px] p-8 shadow-halo">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <span
                  className={cn(
                    "eco-status-ring",
                    statusColors[status],
                    status === "recording" ? "animate-pulse-glow" : ""
                  )}
                />
                <Badge
                  data-testid="status-label"
                  variant={status === "recording" ? "active" : "soft"}
                >
                  {statusLabels[status]}
                </Badge>
              </div>
              <h1 className="font-display text-4xl font-semibold text-ink">
                Eco convierte tu voz en texto donde estes trabajando.
              </h1>
              <p className="max-w-2xl text-base text-ink/70">
                Dicta, revisa y pega sin cambiar de ventana. El motor de transcripcion se ejecuta
                localmente y los modelos se descargan bajo demanda.
              </p>
              {statusMessage && (
                <p className="text-sm text-ember" data-testid="status-message">
                  {statusMessage}
                </p>
              )}
            </div>
            <div className="flex flex-col gap-3">
              <Button
                data-testid="dictate-toggle"
                className="w-full lg:w-auto"
                onClick={handleToggleRecording}
              >
                {status === "recording" ? "Detener dictado" : "Iniciar dictado"}
              </Button>
              <div
                data-testid="active-shortcut"
                className="rounded-2xl border border-ink/10 bg-white/70 px-4 py-3 text-sm text-ink/70"
              >
                Atajo activo: <span className="font-semibold text-ink">{shortcut}</span>
              </div>
            </div>
          </div>
          <Separator />
          <div className="grid gap-6 md:grid-cols-3">
            <div className="space-y-2">
              <p className="text-xs uppercase tracking-[0.3em] text-ink/50">Modelo en uso</p>
              <p className="text-2xl font-semibold text-ink">
                {activeModel ? activeModel.title : "Sin modelo"}
              </p>
              <p className="text-sm text-ink/60">
                {activeModel
                  ? `${activeModel.sizeMb} MB · Optimo para ${
                      activeModel.id === "tiny" || activeModel.id === "base"
                        ? "velocidad"
                        : "precision"
                    }`
                  : "Descarga un modelo para empezar."}
              </p>
            </div>
            <div className="space-y-2">
              <p className="text-xs uppercase tracking-[0.3em] text-ink/50">Modo actual</p>
              <p className="text-2xl font-semibold text-ink">{statusLabels[status]}</p>
              <p className="text-sm text-ink/60">
                {status === "recording"
                  ? "Grabando desde el microfono, vuelve a pulsar para transcribir."
                  : status === "processing"
                  ? "Transcribiendo audio de forma local."
                  : "Listo para dictar con tu atajo global."}
              </p>
            </div>
            <div className="space-y-2">
              <p className="text-xs uppercase tracking-[0.3em] text-ink/50">Ultima transcripcion</p>
              <p className="text-sm text-ink/70">{lastTranscript || "Aun no hay texto."}</p>
              <p className="text-xs text-ink/50">
                {lastDurationMs !== null
                  ? `Tiempo de transcripcion: ${(lastDurationMs / 1000).toFixed(2)}s`
                  : "Tiempo de transcripcion: --"}
              </p>
            </div>
          </div>
        </header>

        <section className="grid gap-6 lg:grid-cols-[1.1fr_1fr]">
          <Card className="h-full">
            <CardHeader>
              <div>
                <CardTitle>Atajo global</CardTitle>
                <p className="text-sm text-ink/60">
                  Cambia la combinacion para iniciar y detener el dictado sin perder el foco.
                </p>
              </div>
              <Badge variant="soft">Sistema</Badge>
            </CardHeader>
            <CardContent className="space-y-5">
              <Input
                data-testid="shortcut-input"
                value={shortcut}
                onChange={(event) => setShortcut(event.currentTarget.value)}
                placeholder="Ctrl+Alt+Space"
              />
              <div className="flex flex-wrap gap-3">
                <Button
                  data-testid="shortcut-save"
                  variant="primary"
                  onClick={handleSaveShortcut}
                  disabled={!shortcut.trim() || isSavingShortcut}
                >
                  {isSavingShortcut ? "Guardando..." : "Guardar atajo"}
                </Button>
                <Button variant="outline" onClick={() => setShortcut("Ctrl+Alt+Space")}>
                  Restablecer
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card className="h-full">
            <CardHeader>
              <div>
                <CardTitle>Area de prueba</CardTitle>
                <p className="text-sm text-ink/60">
                  Comprueba el texto antes de pegarlo o copialo directamente.
                </p>
              </div>
              <Badge variant="default">Entrada</Badge>
            </CardHeader>
            <CardContent>
              <Textarea
                data-testid="test-textarea"
                ref={textareaRef}
                value={testText}
                onChange={(event) => setTestText(event.currentTarget.value)}
                placeholder="El texto transcrito aparecera aqui..."
              />
              <div className="flex flex-wrap gap-3">
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
            </CardContent>
          </Card>
        </section>

        <Card>
          <CardHeader>
            <div>
              <CardTitle>Modelos locales</CardTitle>
              <p className="text-sm text-ink/60">
                Descarga el modelo adecuado para velocidad o precision. Elige uno activo para las
                transcripciones.
              </p>
            </div>
            <Badge variant="default">Whisper</Badge>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-3">
              {models.map((model) => {
                const progress = downloads[model.id] || 0;
                return (
                  <div
                    key={model.id}
                    data-testid={`model-${model.id}`}
                    className="eco-panel flex flex-col gap-4 rounded-2xl p-4 shadow-soft"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <div className="space-y-1">
                        <div className="flex items-center gap-3">
                          <p className="text-lg font-semibold text-ink">{model.title}</p>
                          {model.active && <Badge variant="active">En uso</Badge>}
                          {model.partial && !model.installed && <Badge>Incompleto</Badge>}
                        </div>
                        <p className="text-sm text-ink/60">{model.sizeMb} MB</p>
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
                        {model.partial && !model.installed && (
                          <Button variant="outline" onClick={() => handleDelete(model.id)}>
                            Limpiar
                          </Button>
                        )}
                      </div>
                    </div>
                    {progress > 0 && progress < 100 && (
                      <div className="space-y-2">
                        <Progress data-testid={`model-${model.id}-progress`} value={progress} />
                        <p className="text-xs text-ink/60">
                          Descargando... {Math.round(progress)}%
                        </p>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default App;
