import { createSignal } from "solid-js";

interface GearButtonProps {
  zone: { cx: number; cy: number; radius: number };
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
    console.log("Settings clicked — settings panel coming in Slice 4");
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
