import type { SVGProps } from 'react';

// 线性图标（24×24，描边继承 currentColor）
type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function base({ size = 18, ...rest }: IconProps) {
  return {
    width: size,
    height: size,
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.8,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    'aria-hidden': true,
    ...rest,
  };
}

export const IconPlay = (p: IconProps) => (
  <svg {...base(p)}><path d="M7 4.5v15l12.5-7.5z" fill="currentColor" stroke="none" /></svg>
);
export const IconStop = (p: IconProps) => (
  <svg {...base(p)}><rect x="6" y="6" width="12" height="12" rx="2.5" fill="currentColor" stroke="none" /></svg>
);
export const IconGrid = (p: IconProps) => (
  <svg {...base(p)}>
    <rect x="3.5" y="3.5" width="7" height="7" rx="1.8" />
    <rect x="13.5" y="3.5" width="7" height="7" rx="1.8" />
    <rect x="3.5" y="13.5" width="7" height="7" rx="1.8" />
    <rect x="13.5" y="13.5" width="7" height="7" rx="1.8" />
  </svg>
);
export const IconUser = (p: IconProps) => (
  <svg {...base(p)}><circle cx="12" cy="8" r="4" /><path d="M4.5 20.5c.8-3.6 3.9-5.5 7.5-5.5s6.7 1.9 7.5 5.5" /></svg>
);
export const IconDownload = (p: IconProps) => (
  <svg {...base(p)}><path d="M12 4v11M7 10.5l5 5 5-5M5 20h14" /></svg>
);
export const IconSparkle = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M12 3.5l1.9 5.6 5.6 1.9-5.6 1.9L12 18.5l-1.9-5.6L4.5 11l5.6-1.9z" />
    <path d="M19 16.5v4M17 18.5h4" />
  </svg>
);
export const IconSliders = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
    <circle cx="15" cy="7" r="2.2" />
    <circle cx="9" cy="17" r="2.2" />
  </svg>
);
export const IconHome = (p: IconProps) => (
  <svg {...base(p)}><path d="M4 10.5L12 4l8 6.5V19a1.5 1.5 0 0 1-1.5 1.5H15v-6h-6v6H5.5A1.5 1.5 0 0 1 4 19z" /></svg>
);
export const IconPlus = (p: IconProps) => (
  <svg {...base(p)}><path d="M12 5v14M5 12h14" /></svg>
);
export const IconFolder = (p: IconProps) => (
  <svg {...base(p)}><path d="M3.5 7.5A2 2 0 0 1 5.5 5.5h3.6l2 2h7.4a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" /></svg>
);
export const IconEdit = (p: IconProps) => (
  <svg {...base(p)}><path d="M4.5 19.5h3.8L19 8.8 15.2 5 4.5 15.7zM13.5 6.7l3.8 3.8" /></svg>
);
export const IconTrash = (p: IconProps) => (
  <svg {...base(p)}><path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l.9 12.5h9.2L17.5 7M10 11v5M14 11v5" /></svg>
);
export const IconClock = (p: IconProps) => (
  <svg {...base(p)}><circle cx="12" cy="12" r="8.5" /><path d="M12 7.5V12l3 2" /></svg>
);
export const IconRepeat = (p: IconProps) => (
  <svg {...base(p)}><path d="M17 3.5l3 3-3 3M4 11.5v-1a4 4 0 0 1 4-4h12M7 20.5l-3-3 3-3M20 12.5v1a4 4 0 0 1-4 4H4" /></svg>
);
export const IconCalendar = (p: IconProps) => (
  <svg {...base(p)}><rect x="3.5" y="5" width="17" height="15.5" rx="2" /><path d="M3.5 10h17M8 3v4M16 3v4" /></svg>
);
export const IconTerminal = (p: IconProps) => (
  <svg {...base(p)}><rect x="3" y="4.5" width="18" height="15" rx="2.5" /><path d="M7 9.5l3 2.5-3 2.5M12.5 15h4.5" /></svg>
);
export const IconCup = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M5 9h11v5a5 5 0 0 1-5 5h-1a5 5 0 0 1-5-5zM16 10.5h1.5a2.5 2.5 0 0 1 0 5H16M8.5 3.5v2.5M12 3.5v2.5" />
  </svg>
);
export const IconChip = (p: IconProps) => (
  <svg {...base(p)}>
    <rect x="6" y="6" width="12" height="12" rx="2" />
    <path d="M9.5 3v3M14.5 3v3M9.5 18v3M14.5 18v3M3 9.5h3M3 14.5h3M18 9.5h3M18 14.5h3" />
  </svg>
);
export const IconGlobe = (p: IconProps) => (
  <svg {...base(p)}><circle cx="12" cy="12" r="8.5" /><path d="M3.5 12h17M12 3.5c2.5 2.6 3.5 5.5 3.5 8.5s-1 5.9-3.5 8.5c-2.5-2.6-3.5-5.5-3.5-8.5s1-5.9 3.5-8.5z" /></svg>
);
export const IconMonitor = (p: IconProps) => (
  <svg {...base(p)}><rect x="3" y="4.5" width="18" height="12" rx="2" /><path d="M9 20.5h6M12 16.5v4" /></svg>
);
export const IconKey = (p: IconProps) => (
  <svg {...base(p)}><circle cx="8" cy="15" r="4" /><path d="M11 12l8.5-8.5M16.5 6.5l2.5 2.5M14 9l2 2" /></svg>
);
export const IconInfo = (p: IconProps) => (
  <svg {...base(p)}><circle cx="12" cy="12" r="8.5" /><path d="M12 11v5.5M12 7.8v.2" /></svg>
);
export const IconCheck = (p: IconProps) => (
  <svg {...base(p)}><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
);
export const IconAlert = (p: IconProps) => (
  <svg {...base(p)}><path d="M12 4l9 15.5H3zM12 10v4.5M12 17.2v.2" /></svg>
);
export const IconX = (p: IconProps) => (
  <svg {...base(p)}><path d="M6 6l12 12M18 6L6 18" /></svg>
);
export const IconChevronDown = (p: IconProps) => (
  <svg {...base(p)}><path d="M6.5 9.5l5.5 5.5 5.5-5.5" /></svg>
);
export const IconStar = (p: IconProps) => (
  <svg {...base(p)}><path d="M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.3-4.1 5.9-.8z" /></svg>
);
export const IconLogout = (p: IconProps) => (
  <svg {...base(p)}><path d="M14.5 4.5h3a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2h-3M10 16.5L5.5 12 10 7.5M5.5 12h10" /></svg>
);

export function MicrosoftLogo({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 22 22" aria-hidden>
      <rect x="1" y="1" width="9.5" height="9.5" fill="#f25022" />
      <rect x="11.5" y="1" width="9.5" height="9.5" fill="#7fba00" />
      <rect x="1" y="11.5" width="9.5" height="9.5" fill="#00a4ef" />
      <rect x="11.5" y="11.5" width="9.5" height="9.5" fill="#ffb900" />
    </svg>
  );
}

/** 品牌标志：方形星辰 + 木筏 */
export function LogoMark({ size = 34 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden>
      <defs>
        <linearGradient id="logo-bg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#8b93ff" />
          <stop offset="1" stopColor="#b26bff" />
        </linearGradient>
      </defs>
      <rect width="64" height="64" rx="16" fill="url(#logo-bg)" />
      <path d="M32 11l4.2 11.8L48 27l-11.8 4.2L32 43l-4.2-11.8L16 27l11.8-4.2z" fill="#fff" />
      <rect x="15" y="46" width="34" height="4" rx="1.5" fill="#fff" opacity=".9" />
      <rect x="19" y="51.5" width="26" height="3.5" rx="1.5" fill="#fff" opacity=".55" />
    </svg>
  );
}
