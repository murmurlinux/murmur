import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

interface MicButtonProps {
  zone: { cx: number; cy: number; radius: number; ledRadius: number; taperRadius: number };
  sourceWidth: number;
  sourceHeight: number;
}

export function MicButton(props: MicButtonProps) {
  const [isHeld, setIsHeld] = createSignal(false);

  const leftPct = () => ((props.zone.cx - props.zone.ledRadius) / props.sourceWidth) * 100;
  const topPct = () => ((props.zone.cy - props.zone.ledRadius) / props.sourceHeight) * 100;
  const widthPct = () => ((props.zone.ledRadius * 2) / props.sourceWidth) * 100;
  const heightPct = () => ((props.zone.ledRadius * 2) / props.sourceHeight) * 100;

  const taperLeftPct = () => ((props.zone.cx - props.zone.taperRadius) / props.sourceWidth) * 100;
  const taperTopPct = () => ((props.zone.cy - props.zone.taperRadius) / props.sourceHeight) * 100;
  const taperWidthPct = () => ((props.zone.taperRadius * 2) / props.sourceWidth) * 100;
  const taperHeightPct = () => ((props.zone.taperRadius * 2) / props.sourceHeight) * 100;

  const ledRingPx = () => {
    const ledR = (props.zone.ledRadius / props.sourceWidth) * 690;
    const btnR = (props.zone.radius / props.sourceWidth) * 690;
    return Math.max(1, Math.round(ledR - btnR));
  };

  const taperSpreadPx = () => {
    const taperR = (props.zone.taperRadius / props.sourceWidth) * 690;
    const ledR = (props.zone.ledRadius / props.sourceWidth) * 690;
    return Math.max(1, Math.round(taperR - ledR));
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
            ? "inset 0 0 12px 6px rgba(0, 212, 255, 0.2)"
            : "inset 0 0 5px 2px rgba(0, 212, 255, 0.04)",
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
            ? `${ledRingPx()}px solid rgba(0, 212, 255, 0.5)`
            : `${ledRingPx()}px solid rgba(0, 212, 255, 0.1)`,
          "outline-offset": "0px",
          "box-shadow": isHeld()
            ? `0 0 ${taperSpreadPx()}px ${Math.round(taperSpreadPx() / 2)}px rgba(0, 212, 255, 0.15)`
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
