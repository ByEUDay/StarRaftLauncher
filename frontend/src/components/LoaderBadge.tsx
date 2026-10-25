import type { LoaderKind } from '../api/types';
import { LOADER_NAMES } from '../api/types';

// 各加载器的识别色与简化标志（非官方 logo，仅用于区分）
const STYLE: Record<LoaderKind, { from: string; to: string; glyph: string }> = {
  vanilla: { from: '#4fd18f', to: '#1f9a63', glyph: 'M' },
  fabric: { from: '#e6c99a', to: '#b48b5c', glyph: 'F' },
  quilt: { from: '#b48bff', to: '#7c4dde', glyph: 'Q' },
  forge: { from: '#7b93ff', to: '#3f57d6', glyph: 'F' },
  neoforge: { from: '#ffaa55', to: '#e2701b', glyph: 'N' },
};

export function LoaderIcon({ loader, size = 24 }: { loader: LoaderKind; size?: number }) {
  const s = STYLE[loader] ?? STYLE.vanilla;
  return (
    <span
      className="loader-icon"
      style={{ width: size, height: size, fontSize: size * 0.46, background: `linear-gradient(135deg, ${s.from}, ${s.to})` }}
      aria-hidden
    >
      {loader === 'forge' ? (
        // 铁砧
        <svg width={size * 0.62} height={size * 0.62} viewBox="0 0 24 24" fill="#fff">
          <path d="M3 7h13c0 2.5 2 4 5 4v1h-5l-1.5 2.5L17 18H7l2.5-3.5L8 12c-2.8 0-5-2-5-5z" />
        </svg>
      ) : (
        s.glyph
      )}
    </span>
  );
}

export function LoaderChip({ loader, version, className = '' }: { loader: LoaderKind; version?: string | null; className?: string }) {
  if (loader === 'vanilla') return <span className={`chip ${className}`}>原版</span>;
  const short = version && loader === 'forge' ? version.replace(/^[^-]+-/, '') : version;
  return (
    <span className={`chip loader-chip loader-${loader} ${className}`}>
      <span className="loader-dot" />
      {LOADER_NAMES[loader]}
      {short ? ` ${short}` : ''}
    </span>
  );
}
