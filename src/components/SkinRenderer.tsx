import bodyImage from "../assets/skins/gemini-v1/body.png";

export function SkinRenderer() {
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
        src={bodyImage}
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
        }}
      />
    </div>
  );
}
