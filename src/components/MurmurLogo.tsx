interface MurmurLogoProps {
  size?: number;
  color?: string;
}

export function MurmurLogo(props: MurmurLogoProps) {
  const s = () => props.size ?? 32;
  const c = () => props.color ?? "#14b8a6";

  return (
    <svg width={s()} height={s()} viewBox="0 0 128 128" fill="none">
      <rect x="4" y="4" width="120" height="120" rx="28"
            fill={c()} fill-opacity="0.1" stroke={c()} stroke-width="3"/>
      <rect x="26" y="30" width="9" height="68" rx="4.5" fill={c()}/>
      <rect x="43" y="42" width="9" height="56" rx="4.5" fill={c()}/>
      <rect x="60" y="30" width="9" height="68" rx="4.5" fill={c()}/>
      <rect x="77" y="42" width="9" height="56" rx="4.5" fill={c()}/>
      <rect x="94" y="30" width="9" height="68" rx="4.5" fill={c()}/>
    </svg>
  );
}
