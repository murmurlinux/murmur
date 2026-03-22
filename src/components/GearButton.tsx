import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { CircleZone } from "../lib/skin-loader";

interface GearButtonProps {
  zone: CircleZone;
  sourceWidth: number;
  sourceHeight: number;
}

export function GearButton(props: GearButtonProps) {
  const [isHovered, setIsHovered] = createSignal(false);
  const [isPressed, setIsPressed] = createSignal(false);

  const leftPct = () => ((props.zone.cx - props.zone.radius) / props.sourceWidth) * 100;
  const topPct = () => ((props.zone.cy - props.zone.radius) / props.sourceHeight) * 100;
  const widthPct = () => ((props.zone.radius * 2) / props.sourceWidth) * 100;
  const heightPct = () => ((props.zone.radius * 2) / props.sourceHeight) * 100;

  const handleClick = () => {
    invoke("open_settings").catch((e) => console.error("Failed to open settings:", e));
  };

  return (
    <button
      data-interactive
      onClick={handleClick}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => { setIsHovered(false); setIsPressed(false); }}
      onMouseDown={() => setIsPressed(true)}
      onMouseUp={() => setIsPressed(false)}
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
        transition: "box-shadow 0.2s ease, transform 0.1s ease",
        "box-shadow": "none",
        transform: isPressed() ? "scale(0.92)" : "scale(1)",
        opacity: isHovered() ? 0.7 : 1,
        outline: "none",
        padding: 0,
        "pointer-events": "auto",
      }}
      title="Settings"
    />
  );
}
