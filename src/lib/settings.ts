import { Store } from "@tauri-apps/plugin-store";

export interface MurmurSettings {
  accentColor: string;
  hotkey: string;
  model: string;
  recordMode: "hold" | "tap";
  autoStopSilence: boolean;
  startOnLogin: boolean;
  onboardingComplete: boolean;
  language: string;
  translateToEnglish: boolean;
}

const DEFAULTS: MurmurSettings = {
  accentColor: "#10b981",
  hotkey: "Ctrl+Shift+Space",
  model: "ggml-tiny.en.bin",
  recordMode: "hold",
  autoStopSilence: true,
  startOnLogin: false,
  onboardingComplete: false,
  language: "en",
  translateToEnglish: false,
};

let storeInstance: Store | null = null;

export async function getStore(): Promise<Store> {
  if (!storeInstance) {
    storeInstance = await Store.load("settings.json");
  }
  return storeInstance;
}

export async function loadSettings(): Promise<MurmurSettings> {
  const store = await getStore();
  const settings = { ...DEFAULTS };

  for (const key of Object.keys(DEFAULTS) as (keyof MurmurSettings)[]) {
    const val = await store.get<MurmurSettings[typeof key]>(key);
    if (val !== undefined && val !== null) {
      (settings as any)[key] = val;
    }
  }

  return settings;
}

export async function saveSetting<K extends keyof MurmurSettings>(
  key: K,
  value: MurmurSettings[K],
): Promise<void> {
  const store = await getStore();
  await store.set(key, value);
  await store.save();
}

export interface ModelInfo {
  name: string;
  filename: string;
  url: string;
  size_mb: number;
  description: string;
  downloaded: boolean;
}

export { DEFAULTS as SETTING_DEFAULTS };

export type CleanupProvider = "groq" | "anthropic";

export interface CleanupSettings {
  enabled: boolean;
  provider: CleanupProvider;
  apiKey: string;
}

const DEFAULT_CLEANUP: CleanupSettings = {
  enabled: true,
  provider: "groq",
  apiKey: "",
};

const CLEANUP_STORE_KEY = "cleanup";

export async function loadCleanupSettings(): Promise<CleanupSettings> {
  const store = await getStore();
  const val = await store.get<CleanupSettings>(CLEANUP_STORE_KEY);
  if (val && typeof val === "object") {
    return { ...DEFAULT_CLEANUP, ...val };
  }
  return { ...DEFAULT_CLEANUP };
}

export async function saveCleanupSetting<K extends keyof CleanupSettings>(
  key: K,
  value: CleanupSettings[K],
): Promise<void> {
  const current = await loadCleanupSettings();
  const updated: CleanupSettings = { ...current, [key]: value };
  const store = await getStore();
  await store.set(CLEANUP_STORE_KEY, updated);
  await store.save();
}
