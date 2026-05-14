import { JSX } from "solid-js";

// Animated ring-then-check indicator for one-shot success
// confirmations (Save committed, Test connection passed,
// onboarding mic confirmed). Keyframes live in styles.css under
// .ac-wrap / .ac-ring / .ac-stroke; this component is just the SVG
// markup. Mounting the component plays the animation once;
// re-mount via <Show> gives a clean replay.
//
// The companion <AnimatedCross> uses the same primitive for
// failure confirmation (Test connection failed). Same ring,
// different stroke geometry, error-red colour.

interface AnimatedCheckProps {
  size?: number;
  color?: string;
}

const SUCCESS_GREEN = "#5a7a3a";
const FAILURE_RED = "#a33a2a";

export function AnimatedCheck(props: AnimatedCheckProps): JSX.Element {
  const size = () => props.size ?? 24;
  const color = () => props.color ?? SUCCESS_GREEN;
  return (
    <span class="ac-wrap" aria-hidden="true">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={size()}
        height={size()}
        viewBox="0 0 24 24"
        fill="none"
        stroke={color()}
        stroke-width="2.4"
        stroke-linecap="round"
        stroke-linejoin="round"
        style={{ display: "block" }}
      >
        <circle class="ac-ring" cx="12" cy="12" r="10" />
        <path class="ac-stroke" d="M7 12.5 L10.5 16 L16.5 9" />
      </svg>
    </span>
  );
}

export function AnimatedCross(props: AnimatedCheckProps): JSX.Element {
  const size = () => props.size ?? 24;
  const color = () => props.color ?? FAILURE_RED;
  return (
    <span class="ac-wrap" aria-hidden="true">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={size()}
        height={size()}
        viewBox="0 0 24 24"
        fill="none"
        stroke={color()}
        stroke-width="2.4"
        stroke-linecap="round"
        stroke-linejoin="round"
        style={{ display: "block" }}
      >
        <circle class="ac-ring" cx="12" cy="12" r="10" />
        <path class="ac-stroke" d="M8 8 L16 16 M16 8 L8 16" />
      </svg>
    </span>
  );
}
