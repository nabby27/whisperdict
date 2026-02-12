import type {
  CheckoutSession,
  ConfigState,
  LicenseImportResult,
  LicenseState,
  ModelState,
  ProgressPayload,
  StatusPayload,
  TranscriptionPayload,
  WhisperdictApi,
  WhisperdictStatus,
} from "./whisperdictApi";

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

export function createMockWhisperdictApi(): WhisperdictApi {
  let status: WhisperdictStatus = "idle";
  let entitlement: "free" | "pro" = "free";
  let licenseStatus: "none" | "valid" | "invalid" = "none";
  let licenseFilePath: string | null = null;
  let nextLicensePath: string | null = null;
  let config: ConfigState = {
    shortcut: "Ctrl+Alt+Space",
    activeModelId: "base",
    language: "en",
    freeTranscriptionsLeft: 50,
    totalTranscriptionsCount: 0,
    entitlement,
    licenseStatus,
    licenseFilePath,
    licenseLastValidatedAt: null,
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

  const syncLicenseConfig = () => {
    config = {
      ...config,
      entitlement,
      licenseStatus,
      licenseFilePath,
      licenseLastValidatedAt: Math.floor(Date.now() / 1000),
    };
  };

  const buildError = (code: string, message: string) => {
    const error = new Error(JSON.stringify({ code, message }));
    (error as Error & { code?: string }).code = code;
    return error;
  };

  const buildLicenseState = (): LicenseState => ({
    entitlement,
    licenseStatus,
    freeTranscriptionsLeft: config.freeTranscriptionsLeft,
    totalTranscriptionsCount: config.totalTranscriptionsCount ?? 0,
    message: licenseStatus === "invalid" ? "Imported license file is invalid." : null,
  });

  if (typeof window !== "undefined") {
    (
      window as Window & {
        __WHISPERDICT_MOCK__?: {
          setFreeTranscriptionsLeft: (value: number) => void;
          setNextLicensePath: (value: string | null) => void;
          consumeNextLicensePath: () => string | null;
        };
      }
    ).__WHISPERDICT_MOCK__ = {
      setFreeTranscriptionsLeft: (value: number) => {
        const next = Number.isFinite(value) ? Math.max(0, Math.floor(value)) : 0;
        config = {
          ...config,
          freeTranscriptionsLeft: next,
        };
      },
      setNextLicensePath: (value) => {
        nextLicensePath = value;
      },
      consumeNextLicensePath: () => {
        const next = nextLicensePath;
        nextLicensePath = null;
        return next;
      },
    };
  }

  return {
    async getConfig() {
      syncLicenseConfig();
      return { ...config };
    },
    async setShortcut(shortcut) {
      config = { ...config, shortcut };
    },
    async setLanguage(language) {
      config = { ...config, language };
    },
    async createCheckoutSession(): Promise<CheckoutSession> {
      return {
        checkoutUrl: "https://polar.sh/checkout/mock-whisperdict",
        checkoutSessionId: "cs_mock_whisperdict",
      };
    },
    async importLicenseFile(path): Promise<LicenseImportResult> {
      const normalized = path.trim().toLowerCase();
      licenseFilePath = path.trim();

      const isValidLicense =
        normalized.endsWith(".json") && !normalized.includes("invalid") && normalized.length > 0;

      if (!isValidLicense) {
        entitlement = "free";
        licenseStatus = "invalid";
        syncLicenseConfig();
        throw buildError("LICENSE_INVALID", "License file is invalid");
      }

      entitlement = "pro";
      licenseStatus = "valid";
      syncLicenseConfig();
      return {
        ok: true,
        entitlement,
        licenseStatus,
      };
    },
    async getLicenseState() {
      syncLicenseConfig();
      return buildLicenseState();
    },
    async removeLicense() {
      entitlement = "free";
      licenseStatus = "none";
      licenseFilePath = null;
      syncLicenseConfig();
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
            done: false,
          });
          if (progress >= 100) {
            clearInterval(interval);
            partials.delete(id);
            installed.add(id);
            config = { ...config, activeModelId: id };
            emitProgress({ id, downloaded: 100, total: 100, ratio: 1, done: true });
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
        if (entitlement !== "pro" && config.freeTranscriptionsLeft <= 0) {
          const code = "FREE_LIMIT_REACHED";
          const message = "Free plan limit reached";
          emitStatus({ status: "error", code, message });
          throw buildError(code, message);
        }
        emitStatus({ status: "recording" });
        return;
      }

      if (status === "recording") {
        emitStatus({ status: "processing" });
        setTimeout(() => {
          emitTranscription({
            text: "Mock transcription: hello from Whisperdict.",
            modelId: config.activeModelId,
            durationMs: 680,
          });
          if (entitlement !== "pro") {
            config = {
              ...config,
              freeTranscriptionsLeft: Math.max(0, config.freeTranscriptionsLeft - 1),
            };
          }
          config = {
            ...config,
            totalTranscriptionsCount: (config.totalTranscriptionsCount ?? 0) + 1,
          };
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
