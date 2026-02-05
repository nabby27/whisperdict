import type {
  ConfigState,
  EcoApi,
  EcoStatus,
  ModelState,
  ProgressPayload,
  StatusPayload,
  TranscriptionPayload,
} from "./ecoApi";

type Listener<T> = (payload: T) => void;

type ModelCatalogItem = {
  id: string;
  title: string;
  sizeMb: number;
};

const catalog: ModelCatalogItem[] = [
  { id: "tiny", title: "Tiny", sizeMb: 75 },
  { id: "base", title: "Base", sizeMb: 142 },
  { id: "small", title: "Small", sizeMb: 466 },
  { id: "medium", title: "Medium", sizeMb: 1460 },
  { id: "large", title: "Large", sizeMb: 2880 },
];

export function createMockEcoApi(): EcoApi {
  let status: EcoStatus = "idle";
  let config: ConfigState = {
    shortcut: "Ctrl+Alt+Space",
    activeModelId: "base",
  };

  const installed = new Set(["base"]);
  const partials = new Set<string>();

  const statusListeners = new Set<Listener<StatusPayload>>();
  const progressListeners = new Set<Listener<ProgressPayload>>();
  const transcriptionListeners = new Set<Listener<TranscriptionPayload>>();

  const emitStatus = (payload: StatusPayload) => {
    status = payload.status;
    statusListeners.forEach((listener) => listener(payload));
  };

  const emitProgress = (payload: ProgressPayload) => {
    progressListeners.forEach((listener) => listener(payload));
  };

  const emitTranscription = (payload: TranscriptionPayload) => {
    transcriptionListeners.forEach((listener) => listener(payload));
  };

  const buildModelState = (): ModelState[] =>
    catalog.map((model) => ({
      ...model,
      installed: installed.has(model.id),
      partial: partials.has(model.id),
      active: config.activeModelId === model.id,
    }));

  return {
    async getConfig() {
      return { ...config };
    },
    async setShortcut(shortcut) {
      config = { ...config, shortcut };
    },
    async listModels() {
      return buildModelState();
    },
    async downloadModel(id) {
      if (installed.has(id)) return;
      let progress = 0;
      partials.add(id);
      return new Promise<void>((resolve) => {
        const interval = setInterval(() => {
          progress = Math.min(100, progress + 12 + Math.random() * 8);
          emitProgress({
            id,
            downloaded: progress,
            total: 100,
            ratio: progress / 100,
          });
          if (progress >= 100) {
            clearInterval(interval);
            partials.delete(id);
            installed.add(id);
            config = { ...config, activeModelId: id };
            emitProgress({ id, downloaded: 100, total: 100, ratio: 1 });
            resolve();
          }
        }, 180);
      });
    },
    async deleteModel(id) {
      installed.delete(id);
      partials.delete(id);
      if (config.activeModelId === id) {
        config = { ...config, activeModelId: "base" };
        installed.add("base");
      }
    },
    async setActiveModel(id) {
      if (!installed.has(id)) return;
      config = { ...config, activeModelId: id };
    },
    async toggleRecording() {
      if (status === "idle") {
        emitStatus({ status: "recording" });
        return;
      }

      if (status === "recording") {
        emitStatus({ status: "processing" });
        setTimeout(() => {
          emitTranscription({
            text: "Transcripcion simulada: hola desde Eco.",
            modelId: config.activeModelId,
            durationMs: 680,
          });
          emitStatus({ status: "idle" });
        }, 700);
      }
    },
    onStatus(cb) {
      statusListeners.add(cb);
      cb({ status });
      return () => statusListeners.delete(cb);
    },
    onProgress(cb) {
      progressListeners.add(cb);
      return () => progressListeners.delete(cb);
    },
    onTranscription(cb) {
      transcriptionListeners.add(cb);
      return () => transcriptionListeners.delete(cb);
    },
  };
}
