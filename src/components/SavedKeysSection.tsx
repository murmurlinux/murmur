import { createSignal, createEffect, JSX, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

const monoFont = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";

const glass: JSX.CSSProperties = {
  "margin-bottom": "16px",
  padding: "18px",
  background: "#ece4d0",
  "border-radius": "0",
  border: "1px solid #d4c9b5",
  position: "relative",
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

type StorageMode = "keyring" | "plaintext";

type ProviderId = "groq" | "anthropic" | "xai";
const PROVIDER_LABEL: Record<ProviderId, string> = {
  groq: "Groq",
  anthropic: "Anthropic",
  xai: "xAI",
};

interface SavedKey {
  provider: string;
  hint: string;
}

interface SavedKeysSectionProps {
  // Bumped by sibling components when a key is added/removed elsewhere.
  refreshTrigger: number;
  // Notify the parent (and AICleanupSection via window event) when this
  // component itself mutates storage by deleting a row.
  onKeyMutated?: () => void;
}

export function SavedKeysSection(props: SavedKeysSectionProps): JSX.Element {
  const [keys, setKeys] = createSignal<SavedKey[]>([]);
  const [storageMode, setStorageMode] = createSignal<StorageMode>("plaintext");
  const [loaded, setLoaded] = createSignal(false);

  const refresh = async () => {
    try {
      const [list, mode] = await Promise.all([
        invoke<Array<[string, string]>>("byok_list_keys"),
        invoke<StorageMode>("byok_storage_mode"),
      ]);
      setKeys(list.map(([provider, hint]) => ({ provider, hint })));
      setStorageMode(mode);
    } catch (err) {
      console.error("saved-keys refresh failed:", err);
      setKeys([]);
    } finally {
      setLoaded(true);
    }
  };

  createEffect(() => {
    // Track the prop so the effect re-runs whenever the parent bumps it.
    void props.refreshTrigger;
    void refresh();
  });

  const handleDelete = async (provider: string) => {
    try {
      await invoke("byok_clear_key", { provider });
      await refresh();
      // Tell AICleanupSection (and anyone else listening) that key
      // state for some provider has changed.
      window.dispatchEvent(new CustomEvent("byok-keys-changed"));
      props.onKeyMutated?.();
    } catch (err) {
      console.error("byok_clear_key failed:", err);
    }
  };

  const blurb = () =>
    storageMode() === "keyring"
      ? "Encrypted in your system keyring, the same store your browser uses for saved passwords. Stays on this machine. We never see them."
      : "No system keyring detected. Keys are saved to a local file readable only by you. Install gnome-keyring or kwallet (with the secret-service plug-in) to switch to encrypted storage.";

  const blurbColor = () =>
    storageMode() === "keyring" ? "#5a5140" : "#a33a2a";

  const providerDisplay = (id: string): string =>
    (PROVIDER_LABEL as Record<string, string | undefined>)[id] ?? id;

  return (
    <div style={glass}>
      <div style={{ ...label, "margin-bottom": "14px" }}>Saved keys</div>
      <p
        style={{
          "font-size": "12px",
          color: blurbColor(),
          "margin-bottom": "14px",
          "max-width": "520px",
        }}
      >
        {blurb()}
      </p>

      <Show
        when={loaded() && keys().length > 0}
        fallback={
          <Show when={loaded()}>
            <p
              style={{
                "font-size": "12px",
                color: "#6b655a",
                "font-family": monoFont,
                margin: "0",
              }}
            >
              No keys saved yet. Paste one above to get started.
            </p>
          </Show>
        }
      >
        <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
          <For each={keys()}>
            {(k) => (
              <div
                style={{
                  display: "flex",
                  "align-items": "center",
                  gap: "12px",
                  padding: "6px 10px",
                  background: "#f5f0e6",
                  border: "1px solid #d4c9b5",
                  "border-radius": "0",
                }}
              >
                <span
                  style={{
                    "font-size": "13px",
                    color: "#1a1a1a",
                    "min-width": "90px",
                  }}
                >
                  {providerDisplay(k.provider)}
                </span>
                <span
                  style={{
                    "font-size": "12px",
                    color: "#6b655a",
                    "font-family": monoFont,
                    flex: "1 1 auto",
                  }}
                >
                  {k.hint}
                </span>
                <button
                  type="button"
                  aria-label={`Delete ${providerDisplay(k.provider)} key`}
                  title={`Delete ${providerDisplay(k.provider)} key`}
                  onClick={() => {
                    void handleDelete(k.provider);
                  }}
                  style={{
                    width: "32px",
                    height: "32px",
                    display: "inline-flex",
                    "align-items": "center",
                    "justify-content": "center",
                    background: "transparent",
                    border: "1px solid #d4c9b5",
                    "border-radius": "0",
                    cursor: "pointer",
                    padding: "0",
                    "flex-shrink": "0",
                  }}
                >
                  <Icon name="trash" size={16} color="#6b655a" />
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
