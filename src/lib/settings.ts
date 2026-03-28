import { Store } from "@tauri-apps/plugin-store";

export interface MurmurSettings {
  skin: string;
  accentColor: string;
  hotkey: string;
  model: string;
  alwaysOnTop: boolean;
  recordMode: "hold" | "tap";
  showSkin: boolean;
  autoStopSilence: boolean;
  startOnLogin: boolean;
}

const DEFAULTS: MurmurSettings = {
  skin: "comm-badge",
  accentColor: "#10b981",
  hotkey: "Ctrl+Shift+Space",
  model: "ggml-tiny.en.bin",
  alwaysOnTop: true,
  recordMode: "hold",
  showSkin: true,
  autoStopSilence: true,
  startOnLogin: false,
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

export { DEFAULTS as SETTING_DEFAULTS };
