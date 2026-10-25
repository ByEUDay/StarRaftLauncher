import { useMemo } from 'react';

// 程序化生成的方块风景（颜色由 CSS 变量控制，深色为星夜、浅色为白天）

export function hashString(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function mulberry32(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

interface TerrainOptions {
  from?: number;
  to: number;
  bottom: number;
  base: number;
  amp: number;
  step: number;
  block: number;
}

/** 阶梯状地形轮廓：正弦叠加后按方块尺寸量化 */
function terrain(seed: number, { from = 0, to, bottom, base, amp, step, block }: TerrainOptions): string {
  const rnd = mulberry32(seed);
  const p1 = rnd() * Math.PI * 2;
  const p2 = rnd() * Math.PI * 2;
  const f1 = 0.004 + rnd() * 0.004;
  const f2 = 0.012 + rnd() * 0.01;
  let d = `M${from} ${bottom}`;
  for (let x = from; x < to; x += step) {
    const raw = base - amp * (0.6 * Math.sin(x * f1 + p1) + 0.4 * Math.sin(x * f2 + p2));
    const y = Math.round(raw / block) * block;
    d += ` L${x} ${y} L${Math.min(x + step, to)} ${y}`;
  }
  return `${d} L${to} ${bottom} Z`;
}

function stars(seed: number, count: number, w: number, h: number) {
  const rnd = mulberry32(seed);
  return Array.from({ length: count }, () => ({
    x: rnd() * w,
    y: rnd() * h,
    r: 0.6 + rnd() * 1.3,
    delay: rnd() * 4,
  }));
}

export function Scene({ seed = 7 }: { seed?: number }) {
  const layers = useMemo(
    () => ({
      stars: stars(seed, 90, 1200, 230),
      far: terrain(seed + 1, { to: 1200, bottom: 400, base: 225, amp: 48, step: 32, block: 16 }),
      mid: terrain(seed + 2, { to: 1200, bottom: 400, base: 268, amp: 34, step: 24, block: 12 }),
      shore: terrain(seed + 3, { to: 560, bottom: 400, base: 300, amp: 18, step: 20, block: 10 }),
    }),
    [seed],
  );

  return (
    <svg className="scene" viewBox="0 0 1200 400" preserveAspectRatio="xMidYMax slice" aria-hidden>
      <defs>
        <linearGradient id="scene-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" style={{ stopColor: 'var(--sky-top)' }} />
          <stop offset="1" style={{ stopColor: 'var(--sky-bottom)' }} />
        </linearGradient>
        <radialGradient id="scene-glow">
          <stop offset="0" style={{ stopColor: 'var(--moon)', stopOpacity: 0.55 }} />
          <stop offset="1" style={{ stopColor: 'var(--moon)', stopOpacity: 0 }} />
        </radialGradient>
        <radialGradient id="scene-lantern">
          <stop offset="0" stopColor="#ffcf6b" stopOpacity="0.9" />
          <stop offset="1" stopColor="#ffcf6b" stopOpacity="0" />
        </radialGradient>
      </defs>

      <rect width="1200" height="400" fill="url(#scene-sky)" />

      <g className="scene-stars">
        {layers.stars.map((s, i) => (
          <circle key={i} cx={s.x} cy={s.y} r={s.r} style={{ animationDelay: `${s.delay}s` }} />
        ))}
      </g>

      <circle cx="760" cy="112" r="120" fill="url(#scene-glow)" />
      <rect className="scene-moon" x="730" y="82" width="60" height="60" rx="6" />

      <path className="scene-far" d={layers.far} />
      <path className="scene-mid" d={layers.mid} />

      <rect className="scene-water" x="0" y="318" width="1200" height="82" />
      <g className="scene-ripples">
        <rect x="900" y="334" width="120" height="3" rx="1.5" />
        <rect x="940" y="348" width="70" height="3" rx="1.5" />
        <rect x="610" y="340" width="90" height="2" rx="1" />
        <rect x="200" y="360" width="140" height="2" rx="1" />
      </g>

      {/* 木筏 */}
      <g className="scene-raft" transform="translate(730 296)">
        <circle cx="62" cy="-6" r="34" fill="url(#scene-lantern)" />
        <rect x="0" y="18" width="110" height="9" rx="2" fill="#8a5a33" />
        <rect x="6" y="27" width="98" height="6" rx="2" fill="#6b4426" />
        <rect x="52" y="-34" width="4" height="52" fill="#5a3a20" />
        <path d="M56 -32 L92 6 L56 6 Z" fill="#efe7d4" opacity=".92" />
        <rect x="58" y="-12" width="8" height="10" rx="2" fill="#ffcf6b" />
      </g>

      <path className="scene-shore" d={layers.shore} />
    </svg>
  );
}

/** 实例封面：根据 ID 生成色相与地形 */
export function Cover({ id }: { id: string }) {
  const hash = hashString(id);
  const hue = hash % 360;
  const d = useMemo(
    () => ({
      far: terrain(hash, { to: 320, bottom: 120, base: 70, amp: 18, step: 16, block: 8 }),
      near: terrain(hash + 9, { to: 320, bottom: 120, base: 92, amp: 12, step: 12, block: 6 }),
      stars: stars(hash, 14, 320, 60),
    }),
    [hash],
  );
  const gid = `cover-${id}`;
  return (
    <svg className="cover" viewBox="0 0 320 120" preserveAspectRatio="xMidYMax slice" aria-hidden>
      <defs>
        <linearGradient id={gid} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor={`hsl(${hue} 70% 58%)`} />
          <stop offset="1" stopColor={`hsl(${(hue + 50) % 360} 60% 30%)`} />
        </linearGradient>
      </defs>
      <rect width="320" height="120" fill={`url(#${gid})`} />
      {d.stars.map((s, i) => (
        <circle key={i} cx={s.x} cy={s.y} r={s.r * 0.7} fill="#fff" opacity="0.7" />
      ))}
      <path d={d.far} fill={`hsl(${hue} 45% 22%)`} opacity="0.55" />
      <path d={d.near} fill={`hsl(${hue} 50% 14%)`} opacity="0.85" />
    </svg>
  );
}
