// Eagerly discover all skin configs and images at build time
const skinConfigs = import.meta.glob<{ default: any }>(
  "../assets/skins/*/skin.json",
  { eager: true },
);

const skinImages = import.meta.glob<{ default: string }>(
  "../assets/skins/*/body.png",
  { eager: true },
);

export interface CircleZone {
  type: string;
  shape: string;
  cx: number;
  cy: number;
  radius: number;
  action?: string;
  ledRadius?: number;
  taperRadius?: number;
  proximityRadius?: number;
}

export interface RectZone {
  type: string;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation?: number;
}

export interface SkinZones {
  micButton: CircleZone;
  gearButton: CircleZone;
  waveform: RectZone;
  statusLed: CircleZone;
  led1: RectZone;
  led2: RectZone;
  led3: RectZone;
}

export interface SkinConfig {
  name: string;
  version: string;
  source: { width: number; height: number };
  accentColor: { default: string; method: string };
  zones: SkinZones;
}

export interface LoadedSkin {
  config: SkinConfig;
  imageSrc: string;
}

function extractSkinName(path: string): string {
  // Path looks like "../assets/skins/comm-badge/skin.json"
  const parts = path.split("/");
  return parts[parts.length - 2];
}

export function loadSkin(name: string): LoadedSkin | null {
  const configKey = Object.keys(skinConfigs).find(
    (k) => extractSkinName(k) === name,
  );
  const imageKey = Object.keys(skinImages).find(
    (k) => extractSkinName(k) === name,
  );

  if (!configKey || !imageKey) return null;

  return {
    config: skinConfigs[configKey].default as SkinConfig,
    imageSrc: skinImages[imageKey].default,
  };
}
