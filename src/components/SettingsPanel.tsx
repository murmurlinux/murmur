import { createSignal, onMount, onCleanup, For, JSX, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { loadSettings, saveSetting, type MurmurSettings, type ModelInfo } from "../lib/settings";
import logoImg from "../assets/logo.png";
import { AICleanupSection } from "./AICleanupSection";
import { SavedKeysSection } from "./SavedKeysSection";
import { Toggle } from "./Toggle";
import { Select } from "./Select";
import { RecordModeToggle } from "./RecordModeToggle";

// --- Terminal Cream Theme ---

const monoFont = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";

const glass: JSX.CSSProperties = {
  "margin-bottom": "16px",
  padding: "18px",
  background: "#ece4d0",
  "border-radius": "0",
  border: "1px solid #d4c9b5",
  transition: "border-color 0.2s ease",
};

const label: JSX.CSSProperties = {
  display: "block",
  "font-size": "11px",
  "font-weight": "700",
  "text-transform": "uppercase",
  "letter-spacing": "0.08em",
  color: "#c9482b",
  "margin-bottom": "10px",
};

const inputBase: JSX.CSSProperties = {
  width: "100%",
  padding: "8px 12px",
  background: "#f5f0e6",
  border: "1px solid #1a1a1a",
  "border-radius": "0",
  color: "#1a1a1a",
  "font-size": "13px",
  "font-family": monoFont,
  "box-sizing": "border-box",
  outline: "none",
};

function SettingRow(props: { label: string; children: JSX.Element }) {
  return (
    <div
      style={{
        display: "flex",
        "align-items": "center",
        "justify-content": "space-between",
      }}
    >
      <span style={{ "font-size": "13px", color: "#6b655a" }}>{props.label}</span>
      {props.children}
    </div>
  );
}

const LANGUAGES = [
  { value: "en", label: "English" },
  { value: "auto", label: "Auto-detect" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "ru", label: "Russian" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
  { value: "ko", label: "Korean" },
  { value: "ar", label: "Arabic" },
  { value: "hi", label: "Hindi" },
  { value: "nl", label: "Dutch" },
  { value: "pl", label: "Polish" },
  { value: "tr", label: "Turkish" },
  { value: "sv", label: "Swedish" },
  { value: "id", label: "Indonesian" },
  { value: "uk", label: "Ukrainian" },
];

// --- Component ---

export function SettingsPanel() {
  const [settings, setSettings] = createSignal<MurmurSettings | null>(null);
  const [capturingHotkey, setCapturingHotkey] = createSignal(false);
  const [models, setModels] = createSignal<ModelInfo[]>([]);
  const [downloadingModel, setDownloadingModel] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [version, setVersion] = createSignal("...");
  // Bumped whenever AICleanupSection or SavedKeysSection mutates the
  // BYOK store; both halves observe this to stay in sync.
  const [savedKeysVersion, setSavedKeysVersion] = createSignal(0);
  const bumpSavedKeys = () => setSavedKeysVersion((v) => v + 1);

  type AccountState = {
    signedIn: boolean;
    isPro: boolean;
    email: string | null;
    proExpiresAt: string | null;
  };
  const [account, setAccount] = createSignal<AccountState>({
    signedIn: false,
    isPro: false,
    email: null,
    proExpiresAt: null,
  });

  const showError = (msg: string) => {
    setError(msg);
    setTimeout(() => setError(null), 5000);
  };

  onMount(async () => {
    // Read version from Tauri config (not hardcoded)
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      setVersion(await getVersion());
    } catch { /* fallback stays as "..." */ }

    const s = await loadSettings();
    setSettings(s);

    try {
      const list = await invoke<ModelInfo[]>("list_models");
      setModels(list);
    } catch {
      // Models command may not exist yet
    }

    await refreshAccount();
    const unlistenPro = await listen<AccountState>("pro-state-changed", (event) => {
      setAccount(event.payload);
    });
    onCleanup(() => unlistenPro());
  });

  async function refreshAccount() {
    try {
      const [isPro, email, proExpiresAt] = await Promise.all([
        invoke<boolean>("pro_is_active"),
        invoke<string | null>("pro_email"),
        invoke<string | null>("pro_expires_at"),
      ]);
      setAccount({ signedIn: email !== null, isPro, email, proExpiresAt });
    } catch (e) {
      // pro commands may not be registered in older binaries; leave default state.
      console.warn("account state unavailable", e);
    }
  }

  function accountButtonLabel(a: AccountState): string {
    if (!a.signedIn) return "Sign in / Get Pro";
    if (a.isPro) return "Manage subscription";
    return "Manage account";
  }

  function formatProExpiry(iso: string | null): string | null {
    if (!iso) return null;
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return null;
    return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
  }

  const updateSetting = async <K extends keyof MurmurSettings>(
    key: K,
    value: MurmurSettings[K],
  ) => {
    await saveSetting(key, value);
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
    await emit("settings-changed", { key, value });
  };

  function handleHotkeyKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      setCapturingHotkey(false);
      return;
    }

    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");

    let key = e.key;
    if (key === " ") key = "Space";
    else if (key.length === 1) key = key.toUpperCase();

    parts.push(key);
    const combo = parts.join("+");

    setCapturingHotkey(false);
    updateSetting("hotkey", combo);
    invoke("change_hotkey", { newHotkey: combo }).catch((err) =>
      showError(`Failed to set hotkey "${combo}": ${err}`),
    );
  }

  async function downloadModel(filename: string) {
    setDownloadingModel(filename);
    try {
      await invoke("download_model", { modelFilename: filename });
      const list = await invoke<ModelInfo[]>("list_models");
      setModels(list);
    } catch (e) {
      showError(`Download failed: ${e}`);
    } finally {
      setDownloadingModel(null);
    }
  }

  async function selectModel(filename: string) {
    await invoke("set_active_model", { modelFilename: filename }).catch((err) => {
      console.error("set_active_model failed:", err);
    });
    updateSetting("model", filename);
  }

  return (
    <div
      style={{
        background: "#f5f0e6",
        "background-image": "radial-gradient(circle at 1px 1px, rgba(26,26,26,0.06) 1px, transparent 0)",
        "background-size": "14px 14px",
        color: "#1a1a1a",
        width: "100%",
        "min-height": "100vh",
        padding: "20px",
        "box-sizing": "border-box",
        "font-family": monoFont,
        position: "relative",
      }}
    >
      <div style={{ position: "relative", "z-index": 1 }}>
        {/* Header */}
        <div
          style={{
            display: "flex",
            "align-items": "center",
            "justify-content": "space-between",
            "margin-bottom": "20px",
          }}
        >
          <div style={{ display: "flex", "align-items": "center", gap: "6px" }}>
            <img src={logoImg} alt="Murmur" width={48} height={48} style={{ "border-radius": "0" }} />
            <pre
              style={{
                color: "#c9482b",
                "font-size": "10px",
                "line-height": "1.0",
                margin: "-10px 0 0 0",
                "white-space": "pre",
                "font-weight": "700",
                "font-family": monoFont,
              }}
            >{` __  __\n|  \\/  |_   _ _ __ _ __ ___  _   _ _ __\n| |\\/| | | | | '__| '_ \` _ \\| | | | '__|\n| |  | | |_| | |  | | | | | | |_| | |\n|_|  |_|\\__,_|_|  |_| |_| |_|\\__,_|_|`}</pre>
            <span style={{ "font-size": "10px", color: "#6b655a", "font-family": monoFont, "align-self": "flex-end", "margin-bottom": "2px" }}>
              v{version()}
            </span>
          </div>
        </div>

        {error() && (
          <div
            style={{
              padding: "10px 14px",
              background: "#ece4d0",
              border: "1px solid #a33a2a",
              "border-radius": "0",
              color: "#a33a2a",
              "font-size": "12px",
              "margin-bottom": "16px",
              cursor: "pointer",
            }}
            onClick={() => setError(null)}
          >
            {error()}
          </div>
        )}

        {settings() && (
          <>
            <AICleanupSection onKeyMutated={bumpSavedKeys} />
            <SavedKeysSection
              refreshTrigger={savedKeysVersion()}
              onKeyMutated={bumpSavedKeys}
            />

            {/* Hotkey */}
            <div style={glass}>
              <label style={label}>Global Hotkey</label>
              <div style={{ display: "flex", gap: "8px" }}>
                <div
                  tabIndex={0}
                  onFocus={() => setCapturingHotkey(true)}
                  onBlur={() => setCapturingHotkey(false)}
                  onKeyDown={(e) => capturingHotkey() && handleHotkeyKeyDown(e)}
                  style={{
                    ...inputBase,
                    flex: "1",
                    cursor: "pointer",
                    "border-color": capturingHotkey() ? "#c9482b" : "#1a1a1a",
                    "box-shadow": capturingHotkey() ? "3px 3px 0 #c9482b" : "none",
                    "text-align": "center",
                    "user-select": "none",
                  }}
                >
                  {capturingHotkey() ? (
                    <span style={{ color: "#6b655a", "font-style": "italic", "font-family": monoFont }}>
                      Press a key combo...
                    </span>
                  ) : (
                    settings()!.hotkey
                  )}
                </div>
                <button
                  onClick={() => {
                    updateSetting("hotkey", "Ctrl+Shift+Space");
                    invoke("change_hotkey", { newHotkey: "Ctrl+Shift+Space" }).catch((err) => {
                      console.error("change_hotkey reset failed:", err);
                    });
                  }}
                  style={{
                    padding: "8px 12px",
                    background: "#ece4d0",
                    border: "1px solid #1a1a1a",
                    "border-radius": "0",
                    color: "#1a1a1a",
                    cursor: "pointer",
                    "font-size": "11px",
                    "font-family": monoFont,
                    "white-space": "nowrap",
                    transition: "all 0.15s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "#1a1a1a";
                    e.currentTarget.style.color = "#f5f0e6";
                    e.currentTarget.style.transform = "translate(-2px, -2px)";
                    e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "#ece4d0";
                    e.currentTarget.style.color = "#1a1a1a";
                    e.currentTarget.style.transform = "none";
                    e.currentTarget.style.boxShadow = "none";
                  }}
                >
                  Reset
                </button>
              </div>
            </div>

            {/* Model */}
            <div style={glass}>
              <label style={label}>Whisper Model</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
                {models().length > 0 ? (
                  <For each={models()}>
                    {(model) => (
                      <div
                        style={{
                          display: "flex",
                          "align-items": "center",
                          gap: "10px",
                          padding: "10px 12px",
                          background: settings()!.model === model.filename
                            ? "#f5f0e6"
                            : "#ece4d0",
                          border: settings()!.model === model.filename
                            ? "1px solid #c9482b"
                            : "1px solid #d4c9b5",
                          "border-radius": "0",
                          transition: "all 0.2s ease",
                        }}
                      >
                        <div style={{ flex: 1 }}>
                          <div style={{ "font-size": "13px", "font-weight": 500, color: "#1a1a1a" }}>
                            {model.name}
                          </div>
                          <div style={{ "font-size": "10px", color: "#6b655a", "margin-top": "2px" }}>
                            {model.description} -- {model.size_mb}MB
                          </div>
                        </div>
                        {model.downloaded ? (
                          settings()!.model === model.filename ? (
                            <span style={{ "font-size": "10px", color: "#c9482b", "font-weight": 700, "text-transform": "uppercase", "letter-spacing": "0.05em", "width": "100px", "text-align": "center", display: "inline-block" }}>
                              active
                            </span>
                          ) : (
                            <button
                              onClick={() => selectModel(model.filename)}
                              style={{
                                padding: "4px 10px",
                                "width": "100px",
                                "text-align": "center",
                                background: "#ece4d0",
                                border: "1px solid #1a1a1a",
                                "border-radius": "0",
                                color: "#1a1a1a",
                                cursor: "pointer",
                                "font-size": "10px",
                                "font-family": monoFont,
                                transition: "all 0.15s ease",
                              }}
                              onMouseEnter={(e) => {
                                e.currentTarget.style.background = "#1a1a1a";
                                e.currentTarget.style.color = "#f5f0e6";
                                e.currentTarget.style.transform = "translate(-2px, -2px)";
                                e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
                              }}
                              onMouseLeave={(e) => {
                                e.currentTarget.style.background = "#ece4d0";
                                e.currentTarget.style.color = "#1a1a1a";
                                e.currentTarget.style.transform = "none";
                                e.currentTarget.style.boxShadow = "none";
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
                              "width": "100px",
                              "text-align": "center",
                              background: downloadingModel() === model.filename
                                ? "#d4c9b5"
                                : "#ece4d0",
                              border: "1px solid #1a1a1a",
                              "border-radius": "0",
                              color: downloadingModel() === model.filename
                                ? "#6b655a"
                                : "#c9482b",
                              cursor: downloadingModel() === model.filename ? "wait" : "pointer",
                              "font-size": "10px",
                              "font-family": monoFont,
                              transition: "all 0.15s ease",
                            }}
                            onMouseEnter={(e) => {
                              if (downloadingModel() !== model.filename) {
                                e.currentTarget.style.background = "#1a1a1a";
                                e.currentTarget.style.color = "#f5f0e6";
                                e.currentTarget.style.transform = "translate(-2px, -2px)";
                                e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
                              }
                            }}
                            onMouseLeave={(e) => {
                              if (downloadingModel() !== model.filename) {
                                e.currentTarget.style.background = "#ece4d0";
                                e.currentTarget.style.color = "#c9482b";
                                e.currentTarget.style.transform = "none";
                                e.currentTarget.style.boxShadow = "none";
                              }
                            }}
                          >
                            {downloadingModel() === model.filename ? "Downloading..." : "Download"}
                          </button>
                        )}
                      </div>
                    )}
                  </For>
                ) : (
                  <div style={{ color: "#6b655a", "font-size": "12px" }}>
                    Model: {settings()!.model}
                  </div>
                )}
              </div>
            </div>

            {/* Recording Mode */}
            <div style={glass}>
              <label style={label}>Recording Mode</label>
              <RecordModeToggle
                value={settings()!.recordMode}
                onChange={(mode) => updateSetting("recordMode", mode)}
                variant="settings"
              />
              {/* Auto-stop on silence (only meaningful in tap mode) */}
              <div style={{ "margin-top": "10px" }}>
                <SettingRow label="Auto-stop on Silence">
                  <Toggle
                    value={settings()!.autoStopSilence}
                    onChange={() => updateSetting("autoStopSilence", !settings()!.autoStopSilence)}
                    disabled={settings()!.recordMode === "hold"}
                  />
                </SettingRow>
                <div style={{ "font-size": "10px", color: "#6b655a", "margin-top": "4px" }}>
                  {settings()!.recordMode === "hold"
                    ? "Not applicable in hold mode (release to stop)"
                    : "Stops recording after ~2s of silence in tap mode"}
                </div>
              </div>
            </div>

            {/* Language */}
            <div style={glass}>
              <label style={label}>Language</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
                <Select
                  value={settings()!.language}
                  onChange={(v) => updateSetting("language", v)}
                  options={LANGUAGES}
                />
                {settings()!.language !== "en" && (
                  <SettingRow label="Translate to English">
                    <Toggle
                      value={settings()!.translateToEnglish}
                      onChange={() => updateSetting("translateToEnglish", !settings()!.translateToEnglish)}
                    />
                  </SettingRow>
                )}
                {settings()!.language !== "en" && settings()!.model.includes(".en.") && (
                  <div style={{
                    "font-size": "11px",
                    color: "#c9482b",
                    padding: "8px 10px",
                    background: "#f5f0e6",
                    "border-radius": "0",
                    border: "1px solid #c9482b",
                  }}>
                    Your current model is English-only. Download a multilingual model above for best results.
                  </div>
                )}
              </div>
            </div>

            {/* Account */}
            <div style={glass}>
              <label style={label}>Account</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
                <Show
                  when={account().signedIn}
                  fallback={
                    <div style={{ "font-size": "12px", color: "#6b655a" }}>
                      Not signed in.
                    </div>
                  }
                >
                  <div style={{ "font-size": "12px", color: "#1a1a1a" }}>
                    Signed in as <strong>{account().email}</strong>
                  </div>
                  <Show when={account().isPro && formatProExpiry(account().proExpiresAt)}>
                    <div style={{ "font-size": "11px", color: "#6b655a" }}>
                      Pro until {formatProExpiry(account().proExpiresAt)}
                    </div>
                  </Show>
                </Show>
                <div style={{ display: "flex", gap: "8px" }}>
                  <button
                    type="button"
                    style={{
                      padding: "6px 12px",
                      "font-size": "12px",
                      "font-family": monoFont,
                      background: "#1a1a1a",
                      color: "#f5f0e6",
                      border: "1px solid #1a1a1a",
                      "border-radius": "0",
                      cursor: "pointer",
                    }}
                    onClick={async () => {
                      try {
                        await invoke("pro_open_sign_in");
                      } catch (e) {
                        showError(`Could not open browser: ${e}`);
                      }
                    }}
                  >
                    {accountButtonLabel(account())}
                  </button>
                  <Show when={account().signedIn}>
                    <button
                      type="button"
                      style={{
                        padding: "6px 12px",
                        "font-size": "12px",
                        "font-family": monoFont,
                        background: "#f5f0e6",
                        color: "#1a1a1a",
                        border: "1px solid #1a1a1a",
                        "border-radius": "0",
                        cursor: "pointer",
                      }}
                      onClick={async () => {
                        try {
                          await invoke("pro_sign_out");
                          await refreshAccount();
                        } catch (e) {
                          showError(`Sign-out failed: ${e}`);
                        }
                      }}
                    >
                      Sign out
                    </button>
                  </Show>
                </div>
              </div>
            </div>

            {/* General */}
            <div style={glass}>
              <label style={label}>General</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "12px" }}>
                <SettingRow label="Start on Login">
                  <Toggle
                    value={settings()!.startOnLogin}
                    onChange={async () => {
                      const next = !settings()!.startOnLogin;
                      await invoke("set_start_on_login", { enabled: next });
                      updateSetting("startOnLogin", next);
                    }}
                  />
                </SettingRow>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
