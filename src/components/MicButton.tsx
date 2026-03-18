import { createSignal } from "solid-js";

interface MicButtonProps {
  zone: { cx: number; cy: number; radius: number; ledRadius: number; taperRadius: number };
  sourceWidth: number;
  sourceHeight: number;
}

export function MicButton(props: MicButtonProps) {
  const [isHeld, setIsHeld] = createSignal(false);

  // The clickable zone covers the full button including the LED ring
  const leftPct = () => ((props.zone.cx - props.zone.ledRadius) / props.sourceWidth) * 100;
  const topPct = () => ((props.zone.cy - props.zone.ledRadius) / props.sourceHeight) * 100;
  const widthPct = () => ((props.zone.ledRadius * 2) / props.sourceWidth) * 100;
  const heightPct = () => ((props.zone.ledRadius * 2) / props.sourceHeight) * 100;

  // Taper ring (outermost glow boundary)
  const taperLeftPct = () => ((props.zone.cx - props.zone.taperRadius) / props.sourceWidth) * 100;
  const taperTopPct = () => ((props.zone.cy - props.zone.taperRadius) / props.sourceHeight) * 100;
  const taperWidthPct = () => ((props.zone.taperRadius * 2) / props.sourceWidth) * 100;
  const taperHeightPct = () => ((props.zone.taperRadius * 2) / props.sourceHeight) * 100;

  // LED ring width in px (gap between knob edge and LED outer edge)
  const ledRingPx = () => {
    const ledR = (props.zone.ledRadius / props.sourceWidth) * 690;
    const btnR = (props.zone.radius / props.sourceWidth) * 690;
    return Math.max(1, Math.round(ledR - btnR));
  };

  // Taper glow spread in px (gap between LED outer edge and taper edge)
  const taperSpreadPx = () => {
    const taperR = (props.zone.taperRadius / props.sourceWidth) * 690;
    const ledR = (props.zone.ledRadius / props.sourceWidth) * 690;
    return Math.max(1, Math.round(taperR - ledR));
  };

  const handleDown = () => {
    setIsHeld(true);
    console.log("Mic: RECORDING (held)");
  };
  const handleUp = () => {
    setIsHeld(false);
    console.log("Mic: IDLE (released)");
  };

  return (
    <>
      {/* Taper glow — fades outward from LED ring edge */}
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

      {/* Clickable zone — sized to ledRadius.
          The LED ring is rendered as an outline (goes OUTSIDE the element edge).
          This means the ring sits between the element edge and outward — no inward bleed. */}
      <button
        data-interactive
        onMouseDown={handleDown}
        onMouseUp={handleUp}
        onMouseLeave={() => { if (isHeld()) handleUp(); }}
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
          // Use outline for the LED ring — outline renders OUTSIDE the border box
          // so it never bleeds inward onto the knob surface
          outline: isHeld()
            ? `${ledRingPx()}px solid rgba(0, 212, 255, 0.5)`
            : `${ledRingPx()}px solid rgba(0, 212, 255, 0.1)`,
          "outline-offset": "0px",
          // Outer glow from the outline edge
          "box-shadow": isHeld()
            ? `0 0 ${taperSpreadPx()}px ${Math.round(taperSpreadPx()/2)}px rgba(0, 212, 255, 0.15)`
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
