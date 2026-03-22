import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { hexToRgba } from "../lib/color";
import type { CircleZone } from "../lib/skin-loader";

interface MicButtonProps {
  zone: CircleZone;
  sourceWidth: number;
  sourceHeight: number;
  accentColor: string;
}

export function MicButton(props: MicButtonProps) {
  const [isHeld, setIsHeld] = createSignal(false);

  const effectiveLedR = () => props.zone.ledRadius ?? props.zone.radius * 1.08;
  const effectiveTaperR = () => props.zone.taperRadius ?? props.zone.radius * 1.23;

  const leftPct = () => ((props.zone.cx - effectiveLedR()) / props.sourceWidth) * 100;
  const topPct = () => ((props.zone.cy - effectiveLedR()) / props.sourceHeight) * 100;
  const widthPct = () => ((effectiveLedR() * 2) / props.sourceWidth) * 100;
  const heightPct = () => ((effectiveLedR() * 2) / props.sourceHeight) * 100;

  const taperLeftPct = () => ((props.zone.cx - effectiveTaperR()) / props.sourceWidth) * 100;
  const taperTopPct = () => ((props.zone.cy - effectiveTaperR()) / props.sourceHeight) * 100;
  const taperWidthPct = () => ((effectiveTaperR() * 2) / props.sourceWidth) * 100;
  const taperHeightPct = () => ((effectiveTaperR() * 2) / props.sourceHeight) * 100;

  const ledRingPx = () => {
    const lr = (effectiveLedR() / props.sourceWidth) * 690;
    const btnR = (props.zone.radius / props.sourceWidth) * 690;
    return Math.max(1, Math.round(lr - btnR));
  };

  const taperSpreadPx = () => {
    const tr = (effectiveTaperR() / props.sourceWidth) * 690;
    const lr = (effectiveLedR() / props.sourceWidth) * 690;
    return Math.max(1, Math.round(tr - lr));
  };

  const handleDown = async () => {
    setIsHeld(true);
    try {
      await invoke("start_recording");
    } catch (e) {
      console.error("Failed to start recording:", e);
      setIsHeld(false);
    }
  };

  const handleUp = async () => {
    if (!isHeld()) return;
    setIsHeld(false);
    try {
      await invoke("stop_recording");
    } catch (e) {
      console.error("Failed to stop recording:", e);
    }
  };

  const c = () => props.accentColor;

  return (
    <>
      <div
        style={{
          position: "absolute",
          left: `${taperLeftPct()}%`,
          top: `${taperTopPct()}%`,
          width: `${taperWidthPct()}%`,
          height: `${taperHeightPct()}%`,
          "border-radius": "50%",
          "pointer-events": "none",
          "z-index": 5,
          "box-shadow": isHeld()
            ? `inset 0 0 12px 6px ${hexToRgba(c(), 0.2)}`
            : `inset 0 0 5px 2px ${hexToRgba(c(), 0.04)}`,
          transition: "box-shadow 0.15s ease",
        }}
      />

      <button
        data-interactive
        onMouseDown={handleDown}
        onMouseUp={handleUp}
        onMouseLeave={handleUp}
        style={{
          position: "absolute",
          left: `${leftPct()}%`,
          top: `${topPct()}%`,
          width: `${widthPct()}%`,
          height: `${heightPct()}%`,
          "border-radius": "50%",
          border: "none",
          background: "transparent",
          cursor: "pointer",
          "z-index": 10,
          transition: "all 0.15s ease",
          outline: isHeld()
            ? `${ledRingPx()}px solid ${hexToRgba(c(), 0.5)}`
            : `${ledRingPx()}px solid ${hexToRgba(c(), 0.1)}`,
          "outline-offset": "0px",
          "box-shadow": isHeld()
            ? `0 0 ${taperSpreadPx()}px ${Math.round(taperSpreadPx() / 2)}px ${hexToRgba(c(), 0.15)}`
            : "none",
          transform: isHeld() ? "scale(0.98)" : "scale(1)",
          padding: 0,
          "pointer-events": "auto",
        }}
        title="Hold to record"
      />
    </>
  );
}
