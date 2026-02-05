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

const LANGUAGE_OPTIONS = [
  { code: "af", label: "Afrikaans" },
  { code: "am", label: "Amharic" },
  { code: "ar", label: "Arabic" },
  { code: "as", label: "Assamese" },
  { code: "az", label: "Azerbaijani" },
  { code: "ba", label: "Bashkir" },
  { code: "be", label: "Belarusian" },
  { code: "bg", label: "Bulgarian" },
  { code: "bn", label: "Bengali" },
  { code: "bo", label: "Tibetan" },
  { code: "br", label: "Breton" },
  { code: "bs", label: "Bosnian" },
  { code: "ca", label: "Catalan" },
  { code: "cs", label: "Czech" },
  { code: "cy", label: "Welsh" },
  { code: "da", label: "Danish" },
  { code: "de", label: "German" },
  { code: "el", label: "Greek" },
  { code: "en", label: "English" },
  { code: "es", label: "Spanish" },
  { code: "et", label: "Estonian" },
  { code: "eu", label: "Basque" },
  { code: "fa", label: "Persian" },
  { code: "fi", label: "Finnish" },
  { code: "fo", label: "Faroese" },
  { code: "fr", label: "French" },
  { code: "gl", label: "Galician" },
  { code: "gu", label: "Gujarati" },
  { code: "ha", label: "Hausa" },
  { code: "haw", label: "Hawaiian" },
  { code: "he", label: "Hebrew" },
  { code: "hi", label: "Hindi" },
  { code: "hr", label: "Croatian" },
  { code: "ht", label: "Haitian Creole" },
  { code: "hu", label: "Hungarian" },
  { code: "hy", label: "Armenian" },
  { code: "id", label: "Indonesian" },
  { code: "is", label: "Icelandic" },
  { code: "it", label: "Italian" },
  { code: "ja", label: "Japanese" },
  { code: "jw", label: "Javanese" },
  { code: "ka", label: "Georgian" },
  { code: "kk", label: "Kazakh" },
  { code: "km", label: "Khmer" },
  { code: "kn", label: "Kannada" },
  { code: "ko", label: "Korean" },
  { code: "la", label: "Latin" },
  { code: "lb", label: "Luxembourgish" },
  { code: "ln", label: "Lingala" },
  { code: "lo", label: "Lao" },
  { code: "lt", label: "Lithuanian" },
  { code: "lv", label: "Latvian" },
  { code: "mg", label: "Malagasy" },
  { code: "mi", label: "Maori" },
  { code: "mk", label: "Macedonian" },
  { code: "ml", label: "Malayalam" },
  { code: "mn", label: "Mongolian" },
  { code: "mr", label: "Marathi" },
  { code: "ms", label: "Malay" },
  { code: "mt", label: "Maltese" },
  { code: "my", label: "Myanmar" },
  { code: "ne", label: "Nepali" },
  { code: "nl", label: "Dutch" },
  { code: "nn", label: "Norwegian Nynorsk" },
  { code: "no", label: "Norwegian" },
  { code: "oc", label: "Occitan" },
  { code: "pa", label: "Punjabi" },
  { code: "pl", label: "Polish" },
  { code: "ps", label: "Pashto" },
  { code: "pt", label: "Portuguese" },
  { code: "ro", label: "Romanian" },
  { code: "ru", label: "Russian" },
  { code: "sa", label: "Sanskrit" },
  { code: "sd", label: "Sindhi" },
  { code: "si", label: "Sinhala" },
  { code: "sk", label: "Slovak" },
  { code: "sl", label: "Slovenian" },
  { code: "sn", label: "Shona" },
  { code: "so", label: "Somali" },
  { code: "sq", label: "Albanian" },
  { code: "sr", label: "Serbian" },
  { code: "su", label: "Sundanese" },
  { code: "sv", label: "Swedish" },
  { code: "sw", label: "Swahili" },
  { code: "ta", label: "Tamil" },
  { code: "te", label: "Telugu" },
  { code: "tg", label: "Tajik" },
  { code: "th", label: "Thai" },
  { code: "tk", label: "Turkmen" },
  { code: "tl", label: "Tagalog" },
  { code: "tr", label: "Turkish" },
  { code: "tt", label: "Tatar" },
  { code: "uk", label: "Ukrainian" },
  { code: "ur", label: "Urdu" },
  { code: "uz", label: "Uzbek" },
  { code: "vi", label: "Vietnamese" },
  { code: "yi", label: "Yiddish" },
  { code: "yo", label: "Yoruba" },
  { code: "zh", label: "Chinese" },
];

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
  const [language, setLanguage] = useState("en");
  const [languageInput, setLanguageInput] = useState("English (en)");
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
        const code = config.language || "en";
        setLanguage(code);
        setLanguageInput(getLanguageLabel(code));
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
      await api.setLanguage(language);
    } finally {
      setIsSavingShortcut(false);
    }
  };

  const handleLanguageChange = async (value: string) => {
    setLanguageInput(value);
    const match = resolveLanguageCode(value);
    if (match) {
      setLanguage(match);
      await api.setLanguage(match);
    }
  };

  const getLanguageLabel = (code: string) => {
    const option = LANGUAGE_OPTIONS.find((item) => item.code === code);
    return option ? `${option.label} (${option.code})` : code;
  };

  const resolveLanguageCode = (value: string) => {
    const normalized = value.trim().toLowerCase();
    const direct = LANGUAGE_OPTIONS.find((item) => item.code === normalized);
    if (direct) return direct.code;
    const fromLabel = LANGUAGE_OPTIONS.find(
      (item) => `${item.label} (${item.code})`.toLowerCase() === normalized
    );
    if (fromLabel) return fromLabel.code;
    const loose = LANGUAGE_OPTIONS.find((item) => item.label.toLowerCase() === normalized);
    return loose?.code;
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
                  <div className="space-y-2">
                    <label htmlFor="language" className="text-xs font-semibold text-foreground">
                      Language
                    </label>
                    <Input
                      id="language"
                      name="language"
                      list="language-options"
                      value={languageInput}
                      onChange={(event) => handleLanguageChange(event.currentTarget.value)}
                      placeholder="English (en)"
                    />
                    <datalist id="language-options">
                      {LANGUAGE_OPTIONS.map((option) => (
                        <option
                          key={option.code}
                          value={`${option.label} (${option.code})`}
                        />
                      ))}
                    </datalist>
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
                            <div className="flex flex-wrap items-center gap-2">
                              <div>
                                <p className="truncate text-sm font-semibold text-foreground">
                                  {model.title}
                                </p>
                                <p className="text-xs text-foreground/70 tabular-nums">
                                  {formatModelSize(model.sizeMb)}
                                </p>
                              </div>
                              <div className="flex flex-wrap items-center gap-2">
                                {model.active && <Badge variant="active">Active</Badge>}
                                {model.partial && !model.installed && <Badge>Incomplete</Badge>}
                              </div>
                            </div>
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
