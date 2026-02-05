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

export type ConfigState = {
  shortcut: string;
  activeModelId: string;
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
};

export type TranscriptionPayload = {
  text: string;
  modelId: string;
  durationMs?: number;
};

export interface EcoApi {
  getConfig(): Promise<ConfigState>;
  setShortcut(shortcut: string): Promise<void>;
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
    listModels: () => invoke<ModelState[]>("list_models"),
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
      const unlisten = listen<ProgressPayload>("models:progress", (event) => cb(event.payload));
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
