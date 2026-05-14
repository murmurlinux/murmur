import { JSX, Show } from "solid-js";

// Inline SVG icons. Paths copied verbatim from Lucide (lucide.dev),
// MIT-licensed. We inline rather than depend on lucide-solid because
// we only need a small handful of glyphs and the desktop bundle
// budget matters more than the convenience of a wrapper package.
//
// All icons share the same 24x24 viewBox, fill=none, currentColor
// stroke, 2px width, round caps and joins. Pick a glyph via the
// `name` prop; `size` and `color` are optional.

export type IconName = "save" | "trash" | "x" | "check";

interface IconProps {
  name: IconName;
  size?: number;
  color?: string;
  title?: string;
}

// Lucide v0.x stable paths. Pinning by inlining means an icon-change
// in upstream Lucide will never silently land here.
//
// Each entry is a factory function so every <Icon> instance gets its
// own freshly-created DOM nodes. A plain JSX.Element here would be
// evaluated once at module load and reused; SolidJS can only place
// the resulting DOM in one location at a time, so multiple <Icon>
// instances with the same name would result in only the last one
// rendering. The factory pattern fixes that.
const PATHS: Record<IconName, () => JSX.Element> = {
  // save: floppy-disk-arrow-in
  save: () => (
    <>
      <path d="M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z" />
      <path d="M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7" />
      <path d="M7 3v4a1 1 0 0 0 1 1h7" />
    </>
  ),
  // trash: trash-2
  trash: () => (
    <>
      <path d="M3 6h18" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <line x1="10" x2="10" y1="11" y2="17" />
      <line x1="14" x2="14" y1="11" y2="17" />
    </>
  ),
  // x: x
  x: () => (
    <>
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </>
  ),
  // check: check
  check: () => <path d="M20 6 9 17l-5-5" />,
};

export function Icon(props: IconProps): JSX.Element {
  const size = () => props.size ?? 18;
  const color = () => props.color ?? "currentColor";
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size()}
      height={size()}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color()}
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden={props.title ? undefined : "true"}
      role={props.title ? "img" : undefined}
      style={{ display: "block", "flex-shrink": 0 }}
    >
      <Show when={props.title}>
        <title>{props.title}</title>
      </Show>
      {PATHS[props.name]()}
    </svg>
  );
}
