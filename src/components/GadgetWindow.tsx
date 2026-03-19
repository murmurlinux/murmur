import { getCurrentWindow } from "@tauri-apps/api/window";
import { SkinRenderer } from "./SkinRenderer";
import { MicButton } from "./MicButton";
import { GearButton } from "./GearButton";
import { LedIndicators } from "./LedIndicators";
import { Waveform } from "./Waveform";
import skinConfig from "../assets/skins/gemini-v1/skin.json";

// Scale the skin down inside the window — leaves transparent padding around edges
// so the proximity zone fits entirely within the window bounds
const SKIN_SCALE = 0.65;

export function GadgetWindow() {
  const appWindow = getCurrentWindow();

  const handleMouseDown = async (e: MouseEvent) => {
    if (e.button === 0) {
      const target = e.target as HTMLElement;
      if (!target.closest("[data-interactive]")) {
        await appWindow.startDragging();
      }
    }
  };

  const sw = skinConfig.source.width;
  const sh = skinConfig.source.height;

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
        <SkinRenderer />
        <Waveform
          zone={skinConfig.zones.waveform as any}
          sourceWidth={sw}
          sourceHeight={sh}
        />
        <MicButton
          zone={skinConfig.zones.micButton as any}
          sourceWidth={sw}
          sourceHeight={sh}
        />
        <GearButton
          zone={skinConfig.zones.gearButton as any}
          sourceWidth={sw}
          sourceHeight={sh}
        />
        <LedIndicators
          leds={[skinConfig.zones.led1, skinConfig.zones.led2, skinConfig.zones.led3] as any}
          gearZone={skinConfig.zones.gearButton as any}
          sourceWidth={sw}
          sourceHeight={sh}
        />
      </div>
    </div>
  );
}
