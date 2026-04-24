import { JSX, Show } from "solid-js";
import { isPro } from "../lib/auth";

const PRO_URL = "https://murmurlinux.com/pro";

type ProGateProps = {
  feature: string;
  children: JSX.Element;
  title?: string;
};

/**
 * Wraps Pro-only UI. Pro tier renders children verbatim. Free tier renders
 * greyed-out children + rust "PRO" pill + "Learn more" link. Clicks on
 * disabled controls are mute (no modal). The `feature` prop is logged for
 * later analytics on gate impressions; `title` optionally names the section
 * for accessibility.
 */
export function ProGate(props: ProGateProps) {
  return (
    <Show
      when={isPro()}
      fallback={
        <div
          data-pro-gate={props.feature}
          aria-disabled="true"
          aria-label={props.title ? `${props.title} (Pro feature)` : undefined}
          style={{
            position: "relative",
            opacity: "0.55",
            "pointer-events": "none",
            "user-select": "none",
          }}
        >
          <div
            style={{
              position: "absolute",
              top: "0",
              right: "0",
              "pointer-events": "auto",
              display: "flex",
              "align-items": "center",
              gap: "8px",
              "z-index": "1",
            }}
          >
            <span
              style={{
                "font-family": "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace",
                "font-size": "10px",
                "font-weight": "700",
                "letter-spacing": "0.08em",
                color: "#f5f0e6",
                background: "#c9482b",
                padding: "2px 6px",
                "border-radius": "0",
              }}
            >
              PRO
            </span>
            <a
              href={PRO_URL}
              target="_blank"
              rel="noopener noreferrer"
              style={{
                "font-family": "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace",
                "font-size": "11px",
                color: "#c9482b",
                "text-decoration": "underline",
              }}
            >
              Learn more →
            </a>
          </div>
          {props.children}
        </div>
      }
    >
      {props.children}
    </Show>
  );
}
