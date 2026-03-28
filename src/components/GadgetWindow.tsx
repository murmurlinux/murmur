import { getCurrentWindow, cursorPosition } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { createSignal, onMount, onCleanup } from "solid-js";
import { SkinRenderer } from "./SkinRenderer";
import { MicButton } from "./MicButton";
import { GearButton } from "./GearButton";
import { LedIndicators } from "./LedIndicators";
import { Waveform } from "./Waveform";
import { StatusLed } from "./StatusLed";
import { loadSettings } from "../lib/settings";
import { hexToHue } from "../lib/color";
import { loadSkin, type LoadedSkin, type SkinConfig } from "../lib/skin-loader";
import defaultSkinConfig from "../assets/skins/comm-badge/skin.json";
import defaultBodyImage from "../assets/skins/comm-badge/body.png";

// Scale the skin down inside the window — leaves transparent padding around edges
// so the proximity zone fits entirely within the window bounds
const SKIN_SCALE = 0.65;

// Default skin as fallback
const defaultSkin: LoadedSkin = {
  config: defaultSkinConfig as SkinConfig,
  imageSrc: defaultBodyImage,
};

export function GadgetWindow() {
  const appWindow = getCurrentWindow();
  const [skin, setSkin] = createSignal<LoadedSkin>(defaultSkin);
  const [accentColor, setAccentColor] = createSignal("#10b981");
  const [recordMode, setRecordMode] = createSignal<"hold" | "tap">("hold");

  // Load settings and skin on mount
  onMount(async () => {
    const settings = await loadSettings();
    setAccentColor(settings.accentColor);
    setRecordMode(settings.recordMode);

    const loaded = loadSkin(settings.skin);
    if (loaded) setSkin(loaded);

    // Listen for live setting changes from the settings window
    const unlisten = await listen<{ key: string; value: any }>(
      "settings-changed",
      (event) => {
        const { key, value } = event.payload;
        if (key === "skin") {
          const loaded = loadSkin(value);
          if (loaded) setSkin(loaded);
        } else if (key === "accentColor") {
          setAccentColor(value);
        } else if (key === "alwaysOnTop") {
          appWindow.setAlwaysOnTop(value).catch(() => {});
        } else if (key === "recordMode") {
          setRecordMode(value);
        }
      },
    );

    onCleanup(() => unlisten());
  });

  // Click-through: transparent padding passes mouse events to apps behind
  onMount(() => {
    let ignoring = false;

    const pollId = setInterval(async () => {
      try {
        const [cursor, winPos, winSize] = await Promise.all([
          cursorPosition(),
          appWindow.outerPosition(),
          appWindow.innerSize(),
        ]);

        const padding = (1 - SKIN_SCALE) / 2;
        const skinLeft = winPos.x + winSize.width * padding;
        const skinTop = winPos.y + winSize.height * padding;
        const skinRight = winPos.x + winSize.width * (1 - padding);
        const skinBottom = winPos.y + winSize.height * (1 - padding);

        const isOverSkin =
          cursor.x >= skinLeft &&
          cursor.x <= skinRight &&
          cursor.y >= skinTop &&
          cursor.y <= skinBottom;

        const shouldIgnore = !isOverSkin;
        if (shouldIgnore !== ignoring) {
          await appWindow.setIgnoreCursorEvents(shouldIgnore);
          ignoring = shouldIgnore;
        }
      } catch {
        // Window may be closing or not yet ready
      }
    }, 150);

    onCleanup(() => clearInterval(pollId));
  });

  const handleMouseDown = async (e: MouseEvent) => {
    if (e.button === 0) {
      const target = e.target as HTMLElement;
      if (!target.closest("[data-interactive]")) {
        await appWindow.startDragging();
      }
    }
  };

  const sw = () => skin().config.source.width;
  const sh = () => skin().config.source.height;
  const zones = () => skin().config.zones;

  // Calculate hue rotation from skin base cyan (#00d4ff, hue ~191) to accent
  const hueRotation = () => {
    const defaultHue = 191;
    return hexToHue(accentColor()) - defaultHue;
  };

  return (
    <div
      onMouseDown={handleMouseDown}
      style={{
        position: "relative",
        width: "100%",
        height: "100%",
        cursor: "grab",
        display: "flex",
        "align-items": "center",
        "justify-content": "center",
      }}
    >
      {/* Inner container scaled down — all children use percentage positioning
          relative to this container, so everything stays aligned */}
      <div
        data-skin-container
        style={{
          position: "relative",
          width: `${SKIN_SCALE * 100}%`,
          height: `${SKIN_SCALE * 100}%`,
        }}
      >
        <SkinRenderer imageSrc={skin().imageSrc} hueRotation={hueRotation()} />
        <Waveform
          zone={zones().waveform}
          sourceWidth={sw()}
          sourceHeight={sh()}
          accentColor={accentColor()}
        />
        <MicButton
          zone={zones().micButton}
          sourceWidth={sw()}
          sourceHeight={sh()}
          accentColor={accentColor()}
          recordMode={recordMode()}
        />
        <GearButton
          zone={zones().gearButton}
          sourceWidth={sw()}
          sourceHeight={sh()}
        />
        <LedIndicators
          leds={[zones().led1, zones().led2, zones().led3]}
          gearZone={zones().gearButton}
          sourceWidth={sw()}
          sourceHeight={sh()}
          accentColor={accentColor()}
        />
        {zones().statusLed && (
          <StatusLed
            zone={zones().statusLed}
            sourceWidth={sw()}
            sourceHeight={sh()}
            accentColor={accentColor()}
          />
        )}
      </div>
    </div>
  );
}
