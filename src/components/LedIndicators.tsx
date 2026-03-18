import { createSignal, onMount, onCleanup } from "solid-js";

interface Led {
  x: number; y: number; width: number; height: number; rotation?: number;
}

interface LedIndicatorsProps {
  leds: Led[];
  gearZone: { cx: number; cy: number; proximityRadius?: number };
  sourceWidth: number;
  sourceHeight: number;
}

export function LedIndicators(props: LedIndicatorsProps) {
  const [proximity, setProximity] = createSignal(0);

  const onMouseMove = (e: MouseEvent) => {
    // Find the skin container (the inner scaled div) by data attribute
    const skinContainer = document.querySelector("[data-skin-container]") as HTMLElement;
    if (!skinContainer) return;

    const rect = skinContainer.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    // Mouse outside the window entirely — reset
    const windowRect = document.documentElement.getBoundingClientRect();
    if (e.clientX < windowRect.left || e.clientY < windowRect.top ||
        e.clientX > windowRect.right || e.clientY > windowRect.bottom) {
      setProximity(0);
      return;
    }

    // Gear button center in display coords (relative to skin container)
    const gx = (props.gearZone.cx / props.sourceWidth) * rect.width;
    const gy = (props.gearZone.cy / props.sourceHeight) * rect.height;

    const dist = Math.sqrt((mx - gx) ** 2 + (my - gy) ** 2);

    // Proximity radius scaled to current skin container size
    const proxR = ((props.gearZone.proximityRadius || 200) / props.sourceWidth) * rect.width;

    if (dist <= proxR * 0.3) setProximity(3);
    else if (dist <= proxR * 0.6) setProximity(2);
    else if (dist <= proxR) setProximity(1);
    else setProximity(0);
  };

  const onMouseLeave = () => setProximity(0);

  onMount(() => {
    window.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseleave", onMouseLeave);
  });
  onCleanup(() => {
    window.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseleave", onMouseLeave);
  });

  const lightOrder = [3, 2, 1];

  return (
    <>
      {props.leds.map((led, i) => {
        const leftPct = () => (led.x / props.sourceWidth) * 100;
        const topPct = () => (led.y / props.sourceHeight) * 100;
        const widthPct = () => (led.width / props.sourceWidth) * 100;
        const heightPct = () => (led.height / props.sourceHeight) * 100;

        const requiredProximity = lightOrder[i];
        const isLit = () => proximity() >= requiredProximity;

        return (
          <div
            style={{
              position: "absolute",
              left: `${leftPct()}%`,
              top: `${topPct()}%`,
              width: `${widthPct()}%`,
              height: `${heightPct()}%`,
              "border-radius": "3px",
              transform: led.rotation ? `rotate(${led.rotation}deg)` : "none",
              background: isLit()
                ? "rgba(140, 235, 250, 0.75)"
                : "transparent",
              "box-shadow": isLit()
                ? "0 0 6px 2px rgba(140, 235, 250, 0.5)"
                : "none",
              transition: "all 0.3s ease",
              "pointer-events": "none",
              "z-index": 8,
            }}
          />
        );
      })}
    </>
  );
}
