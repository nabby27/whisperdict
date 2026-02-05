import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { createMockEcoApi } from "./mockEcoApi";

export type EcoStatus = "idle" | "recording" | "processing" | "error";

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
};

export type StatusPayload = {
  status: EcoStatus;
  message?: string;
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

export interface EcoApi {
  getConfig(): Promise<ConfigState>;
  setShortcut(shortcut: string): Promise<void>;
  setLanguage(language: string): Promise<void>;
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

export function createEcoApi(): EcoApi {
  if (isMock) {
    return createMockEcoApi();
  }

  return {
    getConfig: () => invoke<ConfigState>("get_config"),
    setShortcut: (shortcut) => invoke("set_shortcut", { shortcut }),
    setLanguage: (language) => invoke("set_language", { language }),
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
