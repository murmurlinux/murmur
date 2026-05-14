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

export type CleanupProvider = "groq" | "anthropic" | "xai";

// The API key is no longer stored in settings.json. It lives in the OS
// keyring when available (preferred) or in a per-provider plaintext
// slot under cleanup.keys.<provider> when no secret-service daemon is
// running. Both paths are managed exclusively on the Rust side via the
// byok_* Tauri commands; the frontend never holds the cleartext key
// past the moment the user pastes it.
export interface CleanupSettings {
  enabled: boolean;
  provider: CleanupProvider;
}

const DEFAULT_CLEANUP: CleanupSettings = {
  enabled: true,
  provider: "groq",
};

const CLEANUP_STORE_KEY = "cleanup";

export async function loadCleanupSettings(): Promise<CleanupSettings> {
  const store = await getStore();
  const val = await store.get<Partial<CleanupSettings>>(CLEANUP_STORE_KEY);
  if (val && typeof val === "object") {
    return {
      enabled: typeof val.enabled === "boolean" ? val.enabled : DEFAULT_CLEANUP.enabled,
      provider: (val.provider as CleanupProvider) ?? DEFAULT_CLEANUP.provider,
    };
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
