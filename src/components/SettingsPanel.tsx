import { createSignal, onMount, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { loadSettings, saveSetting, type MurmurSettings } from "../lib/settings";
import { hexToHue, hueToHex } from "../lib/color";

// --- Styles ---

const sectionStyle = {
  "margin-bottom": "20px",
  padding: "16px",
  background: "#1a1a2e",
  "border-radius": "10px",
  border: "1px solid #2a2a3e",
};

const labelStyle = {
  display: "block",
  "font-size": "11px",
  "font-weight": "600",
  "text-transform": "uppercase" as const,
  "letter-spacing": "0.05em",
  color: "#888",
  "margin-bottom": "8px",
};

const inputStyle = {
  width: "100%",
  padding: "8px 12px",
  background: "#12121a",
  border: "1px solid #2a2a3e",
  "border-radius": "6px",
  color: "#e0e0e0",
  "font-size": "14px",
  "font-family": "'Inter', -apple-system, BlinkMacSystemFont, monospace",
  "box-sizing": "border-box" as const,
  outline: "none",
};

// --- Component ---

export function SettingsPanel() {
  const [settings, setSettings] = createSignal<MurmurSettings | null>(null);
  const [hue, setHue] = createSignal(191); // default cyan hue
  const [capturingHotkey, setCapturingHotkey] = createSignal(false);
  const [models, setModels] = createSignal<any[]>([]);
  const [downloadingModel, setDownloadingModel] = createSignal<string | null>(null);

  onMount(async () => {
    const s = await loadSettings();
    setSettings(s);
    setHue(hexToHue(s.accentColor));

    // Load model list
    try {
      const list = await invoke<any[]>("list_models");
      setModels(list);
    } catch {
      // Models command may not exist yet
    }
  });

  const updateSetting = async <K extends keyof MurmurSettings>(
    key: K,
    value: MurmurSettings[K],
  ) => {
    await saveSetting(key, value);
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
    await emit("settings-changed", { key, value });
  };

  // --- Hotkey capture ---

  function handleHotkeyKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      setCapturingHotkey(false);
      return;
    }

    // Ignore standalone modifier presses
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");

    // Normalize key name
    let key = e.key;
    if (key === " ") key = "Space";
    else if (key.length === 1) key = key.toUpperCase();

    parts.push(key);
    const combo = parts.join("+");

    setCapturingHotkey(false);
    updateSetting("hotkey", combo);

    // Try to change hotkey on the backend
    invoke("change_hotkey", { newHotkey: combo }).catch((err) =>
      console.error("Failed to change hotkey:", err),
    );
  }

  // --- Model actions ---

  async function downloadModel(filename: string) {
    setDownloadingModel(filename);
    try {
      await invoke("download_model", { modelFilename: filename });
      // Refresh model list
      const list = await invoke<any[]>("list_models");
      setModels(list);
    } catch (e) {
      console.error("Download failed:", e);
    } finally {
      setDownloadingModel(null);
    }
  }

  async function selectModel(filename: string) {
    await invoke("set_active_model", { modelFilename: filename }).catch(() => {});
    updateSetting("model", filename);
  }

  // --- Always on Top ---

  async function toggleAlwaysOnTop() {
    const s = settings();
    if (!s) return;
    const newVal = !s.alwaysOnTop;
    updateSetting("alwaysOnTop", newVal);
  }

  return (
    <div
      style={{
        background: "#12121a",
        color: "#e0e0e0",
        width: "100%",
        height: "100vh",
        padding: "24px",
        "box-sizing": "border-box",
        "font-family": "'Inter', -apple-system, BlinkMacSystemFont, sans-serif",
        "overflow-y": "auto",
      }}
    >
      <h1
        style={{
          "font-size": "20px",
          "font-weight": 600,
          margin: "0 0 20px 0",
          color: "#fff",
        }}
      >
        Settings
      </h1>

      {settings() && (
        <>
          {/* --- Skin Picker --- */}
          <div style={sectionStyle}>
            <label style={labelStyle}>Skin</label>
            <div
              style={{
                padding: "10px 14px",
                background: "#12121a",
                "border-radius": "6px",
                border: "1px solid #2a2a3e",
                color: "#e0e0e0",
                "font-size": "14px",
              }}
            >
              Gemini V1
              <span style={{ color: "#555", "margin-left": "8px", "font-size": "12px" }}>
                (default)
              </span>
            </div>
          </div>

          {/* --- Accent Colour --- */}
          <div style={sectionStyle}>
            <label style={labelStyle}>Accent Colour</label>
            <div style={{ display: "flex", "align-items": "center", gap: "14px" }}>
              <input
                type="range"
                min="0"
                max="360"
                value={hue()}
                onInput={(e) => {
                  const h = parseInt(e.currentTarget.value);
                  setHue(h);
                  updateSetting("accentColor", hueToHex(h));
                }}
                style={{
                  flex: 1,
                  height: "6px",
                  "border-radius": "3px",
                  appearance: "auto",
                  cursor: "pointer",
                  background: "linear-gradient(to right, #ff0000, #ffff00, #00ff00, #00ffff, #0000ff, #ff00ff, #ff0000)",
                }}
              />
              <div
                style={{
                  width: "32px",
                  height: "32px",
                  "border-radius": "50%",
                  background: hueToHex(hue()),
                  border: "2px solid #2a2a3e",
                  "flex-shrink": "0",
                }}
              />
            </div>
          </div>

          {/* --- Hotkey --- */}
          <div style={sectionStyle}>
            <label style={labelStyle}>Global Hotkey</label>
            <div style={{ display: "flex", gap: "8px" }}>
              <div
                tabIndex={0}
                onFocus={() => setCapturingHotkey(true)}
                onBlur={() => setCapturingHotkey(false)}
                onKeyDown={(e) => capturingHotkey() && handleHotkeyKeyDown(e)}
                style={{
                  ...inputStyle,
                  flex: "1",
                  cursor: "pointer",
                  "border-color": capturingHotkey() ? hueToHex(hue()) : "#2a2a3e",
                  "text-align": "center",
                  "user-select": "none",
                }}
              >
                {capturingHotkey() ? (
                  <span style={{ color: "#888", "font-style": "italic" }}>
                    Press a key combo...
                  </span>
                ) : (
                  settings()!.hotkey
                )}
              </div>
              <button
                onClick={() => {
                  updateSetting("hotkey", "Ctrl+Shift+Space");
                  invoke("change_hotkey", { newHotkey: "Ctrl+Shift+Space" }).catch(() => {});
                }}
                style={{
                  padding: "8px 12px",
                  background: "#2a2a3e",
                  border: "1px solid #3a3a4e",
                  "border-radius": "6px",
                  color: "#888",
                  cursor: "pointer",
                  "font-size": "12px",
                  "white-space": "nowrap",
                }}
              >
                Reset
              </button>
            </div>
          </div>

          {/* --- Model --- */}
          <div style={sectionStyle}>
            <label style={labelStyle}>Whisper Model</label>
            <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
              {models().length > 0 ? (
                <For each={models()}>
                  {(model) => (
                    <div
                      style={{
                        display: "flex",
                        "align-items": "center",
                        gap: "10px",
                        padding: "10px 12px",
                        background:
                          settings()!.model === model.filename ? "#1e1e38" : "#12121a",
                        border:
                          settings()!.model === model.filename
                            ? `1px solid ${hueToHex(hue())}44`
                            : "1px solid #2a2a3e",
                        "border-radius": "6px",
                      }}
                    >
                      <div style={{ flex: 1 }}>
                        <div style={{ "font-size": "14px", "font-weight": 500 }}>
                          {model.name}
                        </div>
                        <div style={{ "font-size": "11px", color: "#666", "margin-top": "2px" }}>
                          {model.description} — {model.size_mb}MB
                        </div>
                      </div>
                      {model.downloaded ? (
                        settings()!.model === model.filename ? (
                          <span
                            style={{
                              "font-size": "11px",
                              color: hueToHex(hue()),
                              "font-weight": 600,
                            }}
                          >
                            Active
                          </span>
                        ) : (
                          <button
                            onClick={() => selectModel(model.filename)}
                            style={{
                              padding: "4px 10px",
                              background: "#2a2a3e",
                              border: "1px solid #3a3a4e",
                              "border-radius": "4px",
                              color: "#ccc",
                              cursor: "pointer",
                              "font-size": "11px",
                            }}
                          >
                            Select
                          </button>
                        )
                      ) : (
                        <button
                          onClick={() => downloadModel(model.filename)}
                          disabled={downloadingModel() === model.filename}
                          style={{
                            padding: "4px 10px",
                            background:
                              downloadingModel() === model.filename ? "#1a1a2e" : "#2a2a3e",
                            border: "1px solid #3a3a4e",
                            "border-radius": "4px",
                            color:
                              downloadingModel() === model.filename ? "#666" : hueToHex(hue()),
                            cursor:
                              downloadingModel() === model.filename ? "wait" : "pointer",
                            "font-size": "11px",
                          }}
                        >
                          {downloadingModel() === model.filename ? "Downloading..." : "Download"}
                        </button>
                      )}
                    </div>
                  )}
                </For>
              ) : (
                <div style={{ color: "#555", "font-size": "13px" }}>
                  Model: {settings()!.model}
                  <div style={{ "font-size": "11px", "margin-top": "4px" }}>
                    Model management commands will be available after backend update.
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* --- General --- */}
          <div style={sectionStyle}>
            <label style={labelStyle}>General</label>
            <div
              style={{
                display: "flex",
                "align-items": "center",
                "justify-content": "space-between",
              }}
            >
              <span style={{ "font-size": "14px" }}>Always on Top</span>
              <button
                onClick={toggleAlwaysOnTop}
                style={{
                  width: "44px",
                  height: "24px",
                  "border-radius": "12px",
                  border: "none",
                  cursor: "pointer",
                  background: settings()!.alwaysOnTop ? hueToHex(hue()) : "#2a2a3e",
                  position: "relative",
                  transition: "background 0.2s ease",
                }}
              >
                <div
                  style={{
                    width: "18px",
                    height: "18px",
                    "border-radius": "50%",
                    background: "#fff",
                    position: "absolute",
                    top: "3px",
                    left: settings()!.alwaysOnTop ? "23px" : "3px",
                    transition: "left 0.2s ease",
                  }}
                />
              </button>
            </div>
          </div>

          {/* --- Version --- */}
          <div
            style={{
              "text-align": "center",
              "font-size": "11px",
              color: "#444",
              "margin-top": "12px",
            }}
          >
            Murmur v0.1.0
          </div>
        </>
      )}
    </div>
  );
}
