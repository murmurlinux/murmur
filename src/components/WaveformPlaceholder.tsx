interface WaveformPlaceholderProps {
  zone: { x: number; y: number; width: number; height: number };
  sourceWidth: number;
  sourceHeight: number;
}

export function WaveformPlaceholder(props: WaveformPlaceholderProps) {
  const leftPct = () => (props.zone.x / props.sourceWidth) * 100;
  const topPct = () => (props.zone.y / props.sourceHeight) * 100;
  const widthPct = () => (props.zone.width / props.sourceWidth) * 100;
  const heightPct = () => (props.zone.height / props.sourceHeight) * 100;

  return (
    <div
      style={{
        position: "absolute",
        left: `${leftPct()}%`,
        top: `${topPct()}%`,
        width: `${widthPct()}%`,
        height: `${heightPct()}%`,
        "border-radius": "12px",
        background: "#080c14",
        overflow: "hidden",
        "z-index": 6,
        "pointer-events": "none",
        display: "flex",
        "align-items": "center",
        "justify-content": "center",
      }}
    >
      {/* Static flat line — will be replaced by live canvas in Slice 2 */}
      <div
        style={{
          width: "80%",
          height: "1px",
          background: "rgba(140, 235, 250, 0.2)",
          "box-shadow": "0 0 4px 1px rgba(140, 235, 250, 0.1)",
        }}
      />
    </div>
  );
}
