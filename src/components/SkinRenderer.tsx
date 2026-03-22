interface SkinRendererProps {
  imageSrc: string;
  hueRotation: number;
}

export function SkinRenderer(props: SkinRendererProps) {
  return (
    <div
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        width: "100%",
        height: "100%",
        "pointer-events": "none",
      }}
    >
      {/* Body layer — the main device image */}
      <img
        src={props.imageSrc}
        alt=""
        draggable={false}
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          width: "100%",
          height: "100%",
          "object-fit": "contain",
          "pointer-events": "none",
          filter: props.hueRotation !== 0 ? `hue-rotate(${props.hueRotation}deg)` : "none",
          transition: "filter 0.3s ease",
        }}
      />
    </div>
  );
}
