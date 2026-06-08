import { useState } from "react";

interface Props {
  seed: string;
  name: string;
  size?: number;
  showStatus?: boolean;
}

function initials(name: string): string {
  const parts = name.replace(/[-_.]/g, " ").trim().split(/\s+/);
  if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
  return name.slice(0, 2).toUpperCase();
}

function hue(seed: string): number {
  let h = 0;
  for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) % 360;
  return h;
}

export function Avatar({ seed, name, size = 60, showStatus = false }: Props) {
  const [failed, setFailed] = useState(false);
  const url = `https://api.dicebear.com/10.x/glyphs/svg?seed=${encodeURIComponent(seed)}`;
  const h = hue(seed);

  return (
    <span className="avatar" style={{ width: size, height: size }}>
      <span
        className="avatar-inner"
        style={{
          background: failed
            ? `linear-gradient(135deg, hsl(${h} 70% 55%), hsl(${(h + 40) % 360} 70% 45%))`
            : "#fff",
        }}
      >
        {failed ? (
          <span className="avatar-initials" aria-hidden="true">
            {initials(name)}
          </span>
        ) : (
          <img
            className="avatar-img"
            src={url}
            width={size}
            height={size}
            alt={`${name} avatar`}
            onError={() => setFailed(true)}
            draggable={false}
          />
        )}
      </span>
      {showStatus && <span className="peer-online" />}
    </span>
  );
}
