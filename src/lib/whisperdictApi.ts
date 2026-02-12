import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { createMockWhisperdictApi } from "./mockWhisperdictApi";

export type WhisperdictStatus = "idle" | "recording" | "processing" | "error";

export type ModelState = {
  id: string;
  title: string;
  sizeMb: number;
  installed: boolean;
  partial: boolean;
  active: boolean;
};

type BackendModelState = {
  id: string;
  title: string;
  size_mb: number;
  installed: boolean;
  partial: boolean;
  active: boolean;
};

export type ConfigState = {
  shortcut: string;
  activeModelId: string;
  language: string;
  freeTranscriptionsLeft: number;
  totalTranscriptionsCount?: number;
  entitlement?: LicenseEntitlement;
  licenseStatus?: LicenseStatus;
  licenseFilePath?: string | null;
  licenseLastValidatedAt?: number | null;
};

export type LicenseEntitlement = "free" | "pro";
export type LicenseStatus = "none" | "valid" | "invalid";

export type LicenseState = {
  entitlement: LicenseEntitlement;
  licenseStatus: LicenseStatus;
  freeTranscriptionsLeft: number;
  totalTranscriptionsCount: number;
  message: string | null;
};

export type LicenseImportResult = {
  ok: boolean;
  entitlement: LicenseEntitlement;
  licenseStatus: LicenseStatus;
};

export type CheckoutSession = {
  checkoutUrl: string;
  checkoutSessionId: string;
};

export type StatusPayload = {
  status: WhisperdictStatus;
  message?: string;
  code?: string;
};

export type ProgressPayload = {
  id: string;
  downloaded: number;
  total: number;
  ratio: number;
  done: boolean;
  error?: string;
};

type BackendProgressPayload = {
  model_id: string;
  downloaded: number;
  total: number | null;
  done: boolean;
  error?: string | null;
};

export type TranscriptionPayload = {
  text: string;
  modelId: string;
  durationMs?: number;
};

export type WhisperdictError = {
  code?: string;
  message: string;
  raw: unknown;
};

type ErrorPayload = {
  code?: string;
  message?: string;
};

const parseErrorPayload = (value: unknown): ErrorPayload | null => {
  if (!value || typeof value !== "object") {
    return null;
  }

  const candidate = value as { code?: unknown; message?: unknown };
  const code = typeof candidate.code === "string" ? candidate.code : undefined;
  const message = typeof candidate.message === "string" ? candidate.message : undefined;

  if (!code && !message) {
    return null;
  }

  return { code, message };
};

const parseErrorString = (value: string): ErrorPayload | null => {
  const trimmed = value.trim();
  const start = trimmed.indexOf("{");
  const end = trimmed.lastIndexOf("}");
  if (start === -1 || end === -1 || end <= start) {
    return null;
  }

  try {
    return parseErrorPayload(JSON.parse(trimmed.slice(start, end + 1)));
  } catch {
    return null;
  }
};

export const parseWhisperdictError = (error: unknown): WhisperdictError => {
  if (typeof error === "string") {
    const payload = parseErrorString(error);
    return {
      code: payload?.code,
      message: payload?.message ?? error,
      raw: error,
    };
  }

  const payload = parseErrorPayload(error);
  if (payload) {
    return {
      code: payload.code,
      message: payload.message ?? "Unknown backend error",
      raw: error,
    };
  }

  if (error instanceof Error) {
    const parsed = parseErrorString(error.message);
    return {
      code: parsed?.code,
      message: parsed?.message ?? error.message,
      raw: error,
    };
  }

  return {
    message: "Unexpected error",
    raw: error,
  };
};

export interface WhisperdictApi {
  getConfig(): Promise<ConfigState>;
  setShortcut(shortcut: string): Promise<void>;
  setLanguage(language: string): Promise<void>;
  createCheckoutSession(): Promise<CheckoutSession>;
  importLicenseFile(path: string): Promise<LicenseImportResult>;
  getLicenseState(): Promise<LicenseState>;
  removeLicense(): Promise<void>;
  listModels(): Promise<ModelState[]>;
  downloadModel(id: string): Promise<void>;
  deleteModel(id: string): Promise<void>;
  setActiveModel(id: string): Promise<void>;
  toggleRecording(): Promise<void>;
  onStatus(cb: (payload: StatusPayload) => void): () => void;
  onProgress(cb: (payload: ProgressPayload) => void): () => void;
  onTranscription(cb: (payload: TranscriptionPayload) => void): () => void;
}

const isMock = import.meta.env.VITE_E2E === "1";

export function createWhisperdictApi(): WhisperdictApi {
  if (isMock) {
    return createMockWhisperdictApi();
  }

  return {
    getConfig: () => invoke<ConfigState>("get_config"),
    setShortcut: (shortcut) => invoke("set_shortcut", { shortcut }),
    setLanguage: (language) => invoke("set_language", { language }),
    createCheckoutSession: () => invoke<CheckoutSession>("create_checkout_session"),
    importLicenseFile: (path) =>
      invoke<LicenseImportResult>("import_license_file", {
        path,
      }),
    getLicenseState: () => invoke<LicenseState>("get_license_state"),
    removeLicense: () => invoke("remove_license"),
    listModels: async () => {
      const models = await invoke<Array<ModelState | BackendModelState>>("list_models");
      return models.map((model) => ({
        id: model.id,
        title: model.title,
        sizeMb: "size_mb" in model ? model.size_mb : model.sizeMb,
        installed: model.installed,
        partial: model.partial,
        active: model.active,
      }));
    },
    downloadModel: (id) => invoke("download_model", { id }),
    deleteModel: (id) => invoke("delete_model", { id }),
    setActiveModel: (id) => invoke("set_active_model", { id }),
    toggleRecording: () => invoke("toggle_recording"),
    onStatus: (cb) => {
      const unlisten = listen<StatusPayload>("status:changed", (event) => cb(event.payload));
      return () => {
        unlisten.then((fn) => fn()).catch(() => undefined);
      };
    },
    onProgress: (cb) => {
      const unlisten = listen<BackendProgressPayload>("models:progress", (event) => {
        const payload = event.payload;
        const total = payload.total ?? 0;
        const ratio = payload.done
          ? 1
          : total > 0
          ? Math.min(1, payload.downloaded / total)
          : 0;
        cb({
          id: payload.model_id,
          downloaded: payload.downloaded,
          total,
          ratio,
          done: payload.done,
          error: payload.error ?? undefined,
        });
      });
      return () => {
        unlisten.then((fn) => fn()).catch(() => undefined);
      };
    },
    onTranscription: (cb) => {
      const unlisten = listen<TranscriptionPayload>("transcription:result", (event) =>
        cb(event.payload)
      );
      return () => {
        unlisten.then((fn) => fn()).catch(() => undefined);
      };
    },
  };
}
