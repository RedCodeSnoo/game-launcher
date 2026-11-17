// Scènes de fond des thèmes : visuels SVG dessinés pour le projet (aucune image externe).
// Les éléments animés portent la classe `fx` : leurs animations sont suspendues avec les particules.
import type { ReactNode } from "react";
import type { Scene } from "./schema";

const svgProps = {
  className: "absolute inset-0 h-full w-full",
  viewBox: "0 0 1600 900",
  preserveAspectRatio: "xMidYMax slice",
  "aria-hidden": true,
} as const;

/** Fleur de cerisier à cinq pétales. */
function Blossom({ x, y, r, o = 0.85 }: { x: number; y: number; r: number; o?: number }) {
  return (
    <g transform={`translate(${x} ${y})`} opacity={o}>
      {[0, 72, 144, 216, 288].map((a) => (
        <ellipse key={a} cx={0} cy={-r * 0.62} rx={r * 0.42} ry={r * 0.62} fill="#f6b3c6" transform={`rotate(${a})`} />
      ))}
      <circle r={r * 0.22} fill="#d9577e" />
    </g>
  );
}

function Sakura() {
  return (
    <svg {...svgProps}>
      <defs>
        <radialGradient id="sk-sun" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#ffd9e1" stopOpacity="0.9" />
          <stop offset="1" stopColor="#ffd9e1" stopOpacity="0" />
        </radialGradient>
      </defs>
      <circle cx="1180" cy="300" r="260" fill="url(#sk-sun)" />
      <circle cx="1180" cy="300" r="120" fill="#fcd2dc" opacity="0.55" />
      {/* Montagne lointaine et son sommet enneigé */}
      <path d="M0 720 L360 620 L640 430 Q700 392 760 430 L1040 620 L1600 700 V900 H0 Z" fill="#e8c3cf" opacity="0.55" />
      <path d="M640 430 Q700 392 760 430 L728 455 L708 440 L690 462 L668 446 Z" fill="#ffffff" opacity="0.75" />
      <path d="M0 790 Q260 720 560 780 T1140 770 T1600 752 V900 H0 Z" fill="#e2adbe" opacity="0.5" />
      <path d="M0 850 Q420 800 860 846 T1600 830 V900 H0 Z" fill="#d898ad" opacity="0.38" />
      {/* Torii sur la colline */}
      <g transform="translate(1238 600)" fill="#8b3651" opacity="0.5">
        <path d="M-118 -6 Q0 -26 118 -6 L126 -22 Q0 -46 -126 -22 Z" />
        <rect x="-96" y="-4" width="192" height="12" />
        <rect x="-102" y="22" width="204" height="10" />
        <rect x="-82" y="-6" width="15" height="186" />
        <rect x="67" y="-6" width="15" height="186" />
        <rect x="-6" y="-4" width="12" height="36" />
      </g>
      {/* Branche de cerisier en haut à gauche */}
      <g opacity="0.75">
        <path d="M-20 70 C140 90 230 140 330 210 C380 245 430 250 500 236" stroke="#6e3346" strokeWidth="9" fill="none" strokeLinecap="round" />
        <path d="M200 118 C230 70 280 50 330 46" stroke="#6e3346" strokeWidth="5" fill="none" strokeLinecap="round" />
        <path d="M330 210 C350 270 340 310 310 350" stroke="#6e3346" strokeWidth="4" fill="none" strokeLinecap="round" />
        {[
          [120, 80, 15], [168, 98, 12], [236, 70, 14], [290, 50, 12], [334, 44, 10], [262, 160, 16], [318, 196, 13], [372, 232, 14],
          [432, 246, 12], [488, 232, 13], [342, 268, 11], [322, 318, 12], [300, 352, 10], [70, 70, 11], [214, 130, 10],
        ].map(([x, y, r], i) => (
          <Blossom key={i} x={x} y={y} r={r} o={0.7 + (i % 3) * 0.1} />
        ))}
      </g>
    </svg>
  );
}

function Lantern({ x, y }: { x: number; y: number }) {
  return (
    <g transform={`translate(${x} ${y})`}>
      <line x1="0" y1="-22" x2="0" y2="-6" stroke="#3a2a40" strokeWidth="2" />
      <rect x="-9" y="-8" width="18" height="5" rx="1" fill="#2a1d2c" />
      <ellipse rx="15" ry="20" cy="13" fill="#d23f37" />
      <ellipse rx="15" ry="20" cy="13" fill="url(#ky-lantern-light)" />
      {[3, 10, 17, 24].map((dy) => (
        <path key={dy} d={`M-14 ${dy} Q0 ${dy + 3} 14 ${dy}`} stroke="#8e2320" strokeWidth="0.9" fill="none" opacity="0.6" />
      ))}
      <rect x="-9" y="31" width="18" height="5" rx="1" fill="#2a1d2c" />
      <line x1="0" y1="36" x2="0" y2="46" stroke="#e8b04a" strokeWidth="1.5" />
    </g>
  );
}

const LANTERNS = [140, 290, 440, 590, 740].map((x) => {
  const t = (x + 20) / 880;
  return { x, y: (1 - t) * (1 - t) * 40 + 2 * (1 - t) * t * 150 + t * t * 60 + 22 };
});

/** Halos des lanternes : un calque HTML par groupe, animé en opacité (aucun redessin du SVG). */
function LanternGlows({ odd }: { odd: boolean }) {
  return (
    <div className={`fx kyoto-glow absolute inset-0 ${odd ? "kyoto-glow-2" : ""}`}>
      <svg {...svgProps}>
        <defs>
          <radialGradient id={`ky-glow-${odd ? 1 : 0}`} cx="0.5" cy="0.5" r="0.5">
            <stop offset="0" stopColor="#ff9a4d" stopOpacity="0.5" />
            <stop offset="1" stopColor="#ff9a4d" stopOpacity="0" />
          </radialGradient>
        </defs>
        {LANTERNS.filter((_, i) => i % 2 === (odd ? 1 : 0)).map(({ x, y }) => (
          <circle key={x} cx={x} cy={y + 13} r="50" fill={`url(#ky-glow-${odd ? 1 : 0})`} />
        ))}
      </svg>
    </div>
  );
}

function Kyoto() {
  return (
    <>
    <svg {...svgProps}>
      <defs>
        <radialGradient id="ky-moon" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#f7e6bd" stopOpacity="0.55" />
          <stop offset="1" stopColor="#f7e6bd" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="ky-lantern-light" cx="0.45" cy="0.4" r="0.6">
          <stop offset="0" stopColor="#ffd38a" stopOpacity="0.95" />
          <stop offset="1" stopColor="#ffd38a" stopOpacity="0" />
        </radialGradient>
      </defs>
      <circle cx="1270" cy="170" r="240" fill="url(#ky-moon)" />
      <circle cx="1270" cy="170" r="68" fill="#f6e8c6" />
      <circle cx="1248" cy="152" r="10" fill="#e9d6a8" opacity="0.5" />
      <circle cx="1290" cy="192" r="14" fill="#e9d6a8" opacity="0.4" />
      {/* Collines et pagode à cinq étages */}
      <path d="M0 700 Q300 610 640 680 T1280 640 T1600 660 V900 H0 Z" fill="#151d3f" />
      <g transform="translate(330 560)" fill="#0b1026">
        {[0, 1, 2, 3, 4].map((i) => {
          const w = 150 - i * 22;
          const y = -i * 58;
          return (
            <g key={i}>
              <path d={`M${-w} ${y} Q0 ${y - 22} ${w} ${y} L${w - 18} ${y - 14} Q0 ${y - 30} ${-w + 18} ${y - 14} Z`} />
              <rect x={-w * 0.55} y={y - 2} width={w * 1.1} height={44} />
            </g>
          );
        })}
        <rect x="-4" y="-320" width="8" height="40" />
        <rect x="-62" y="44" width="124" height="300" />
      </g>
      {/* Toits traditionnels au premier plan */}
      <path d="M0 790 L120 790 Q190 760 260 790 L420 790 Q520 748 620 790 L800 790 Q880 764 960 790 L1180 790 Q1280 740 1380 790 L1600 790 V900 H0 Z" fill="#080c1d" />
      {/* Guirlande de lanternes */}
      <path d="M-20 40 Q420 150 860 60" stroke="#2b2342" strokeWidth="2" fill="none" />
      {LANTERNS.map(({ x, y }) => (
        <Lantern key={x} x={x} y={y} />
      ))}
    </svg>
    <LanternGlows odd={false} />
    <LanternGlows odd />
    </>
  );
}

function Autumn() {
  return (
    <svg {...svgProps}>
      <defs>
        <radialGradient id="au-sun" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#ffb25c" stopOpacity="0.45" />
          <stop offset="1" stopColor="#ffb25c" stopOpacity="0" />
        </radialGradient>
      </defs>
      <circle cx="1120" cy="640" r="380" fill="url(#au-sun)" />
      <path d="M0 700 Q360 600 760 680 T1600 640 V900 H0 Z" fill="#3a1d14" opacity="0.85" />
      <path d="M0 790 Q400 720 820 780 T1600 760 V900 H0 Z" fill="#2a140e" />
      {/* Érable en silhouette */}
      <g fill="#4a1f15" opacity="0.9">
        <path d="M220 900 L232 640 Q210 600 170 590 L178 580 Q220 588 238 610 L250 560 L262 610 Q284 580 330 576 L334 588 Q290 600 268 640 L282 900 Z" />
        {[
          [250, 520, 120], [150, 560, 90], [350, 560, 95], [210, 450, 85], [310, 460, 80], [100, 610, 60], [410, 620, 60],
        ].map(([x, y, r], i) => (
          <circle key={i} cx={x} cy={y} r={r} fill={i % 2 ? "#6e2a18" : "#5a2216"} />
        ))}
      </g>
    </svg>
  );
}

function Winter() {
  const pines = [[1180, 760, 1.2], [1260, 780, 0.9], [1330, 770, 1.05], [1420, 790, 0.8], [150, 790, 0.9], [230, 770, 1.15]];
  return (
    <svg {...svgProps}>
      <path d="M0 640 L300 470 L470 560 L720 360 L980 560 L1180 450 L1600 640 V900 H0 Z" fill="#cddfec" opacity="0.8" />
      <path d="M720 360 L800 420 L760 430 L740 412 L712 436 L690 418 L650 420 Z M300 470 L350 498 L318 506 L298 494 L272 504 Z M1180 450 L1240 486 L1204 490 L1184 478 L1160 490 Z" fill="#ffffff" opacity="0.9" />
      <path d="M0 760 Q400 690 820 750 T1600 730 V900 H0 Z" fill="#e7f0f7" />
      <path d="M0 840 Q500 800 980 836 T1600 820 V900 H0 Z" fill="#f7fbfe" />
      {pines.map(([x, y, s], i) => (
        <g key={i} transform={`translate(${x} ${y}) scale(${s})`} fill="#7e98ab" opacity="0.75">
          <rect x="-5" y="0" width="10" height="26" fill="#6b7f8e" />
          <path d="M0 -150 L48 -60 L28 -62 L64 0 L-64 0 L-28 -62 L-48 -60 Z" />
          <path d="M0 -150 L22 -108 L0 -114 L-22 -108 Z M-30 -62 L0 -72 L30 -62 L14 -58 L0 -64 L-14 -58 Z" fill="#ffffff" opacity="0.85" />
        </g>
      ))}
    </svg>
  );
}

/** Vague périodique (période 800) sur 3200 de large : le calque glisse d'un quart pour boucler sans raccord. */
function wavePath(y: number, amp: number, h: number) {
  let d = `M0 ${y}`;
  for (let x = 0; x < 3200; x += 800) d += ` q200 ${-amp} 400 0 t400 0`;
  return `${d} V${h} H0 Z`;
}

function Wave({ cls, height, y, amp, fill, opacity, foam }: { cls: string; height: string; y: number; amp: number; fill: string; opacity: number; foam?: boolean }) {
  return (
    <div className={`fx ocean-wave ${cls} absolute bottom-0 left-0 w-[200%]`} style={{ height }}>
      <svg className="h-full w-full" viewBox="0 0 3200 200" preserveAspectRatio="none" aria-hidden>
        <path d={wavePath(y, amp, 200)} fill={fill} opacity={opacity} />
        {foam && <path d={wavePath(y - 2, amp, 200)} fill="none" stroke="#bff3ec" strokeOpacity="0.35" strokeWidth="2" vectorEffect="non-scaling-stroke" />}
      </svg>
    </div>
  );
}

function Ocean() {
  return (
    <>
      <svg {...svgProps}>
        <g opacity="0.07" fill="#ffffff">
          <path d="M980 0 L1080 0 L700 900 L560 900 Z" />
          <path d="M1180 0 L1240 0 L1020 900 L930 900 Z" />
        </g>
      </svg>
      <Wave cls="ocean-wave-3" height="24%" y={44} amp={34} fill="#0e4a68" opacity={0.6} />
      <Wave cls="ocean-wave-2" height="17%" y={44} amp={30} fill="#12627f" opacity={0.55} />
      <Wave cls="ocean-wave-1" height="10%" y={50} amp={30} fill="#17808d" opacity={0.5} foam />
    </>
  );
}

function Neon() {
  const stripes = [0, 1, 2, 3, 4, 5, 6];
  return (
    <>
      <svg {...svgProps}>
        <defs>
          <linearGradient id="ne-sun" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#ffd23f" />
            <stop offset="0.55" stopColor="#ff6a7a" />
            <stop offset="1" stopColor="#ff3cb8" />
          </linearGradient>
          <mask id="ne-stripes">
            <rect width="1600" height="900" fill="white" />
            {stripes.map((i) => (
              <rect key={i} x="0" y={735 + i * 14} width="1600" height={2.5 + i * 1.4} fill="black" />
            ))}
          </mask>
          <radialGradient id="ne-halo" cx="0.5" cy="0.5" r="0.5">
            <stop offset="0" stopColor="#ff3cb8" stopOpacity="0.4" />
            <stop offset="1" stopColor="#ff3cb8" stopOpacity="0" />
          </radialGradient>
        </defs>
        <circle cx="1080" cy="760" r="300" fill="url(#ne-halo)" opacity="0.6" />
        <circle cx="1080" cy="760" r="130" fill="url(#ne-sun)" mask="url(#ne-stripes)" opacity="0.5" />
        {/* Ville en silhouette sur l'horizon */}
        <path
          d="M0 770 V720 H60 V690 H110 V730 H170 V670 H230 V740 H300 V705 H360 V750 H420 V685 H470 V760 H1130 V690 H1190 V730 H1250 V660 H1300 V720 H1370 V695 H1430 V740 H1490 V680 H1550 V730 H1600 V770 Z"
          fill="#12041f"
        />
        <rect x="0" y="766" width="1600" height="3" fill="#3cf5c3" opacity="0.6" />
      </svg>
      <div className="neon-floor">
        <div className="fx neon-grid" />
      </div>
    </>
  );
}

function Retro() {
  // Collines et planète en pixels (rectangles nets).
  const hills: [number, number][] = [];
  for (let x = 0; x < 1600; x += 40) hills.push([x, Math.round((Math.sin(x / 170) * 40 + Math.sin(x / 61) * 18 + 760) / 20) * 20]);
  return (
    <svg {...svgProps} shapeRendering="crispEdges">
      <g fill="#ff55ff" opacity="0.18">
        {hills.map(([x, y]) => (
          <rect key={x} x={x} y={y - 40} width="40" height={940 - y} />
        ))}
      </g>
      <g fill="#55ffff" opacity="0.22">
        {hills.map(([x, y]) => (
          <rect key={x} x={x} y={y + 40} width="40" height={900 - y} />
        ))}
      </g>
      <g transform="translate(1260 140)">
        {[
          [-40, -60, 80, 20], [-60, -40, 120, 20], [-80, -20, 160, 60], [-60, 40, 120, 20], [-40, 60, 80, 20],
        ].map(([x, y, w, h], i) => (
          <rect key={i} x={x} y={y} width={w} height={h} fill="#ffff55" opacity="0.5" />
        ))}
        <rect x="-140" y="0" width="280" height="12" fill="#ff55ff" opacity="0.55" />
        <rect x="-40" y="-40" width="20" height="20" fill="#ffffff" opacity="0.35" />
      </g>
    </svg>
  );
}

function Aurora() {
  return (
    <svg {...svgProps} preserveAspectRatio="xMidYMid slice">
      <defs>
        <radialGradient id="au-a" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#2a63f5" stopOpacity="0.22" />
          <stop offset="1" stopColor="#2a63f5" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="au-b" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#7c4dff" stopOpacity="0.18" />
          <stop offset="1" stopColor="#7c4dff" stopOpacity="0" />
        </radialGradient>
      </defs>
      <g className="fx aurora-drift">
        <ellipse cx="1250" cy="120" rx="620" ry="360" fill="url(#au-a)" />
      </g>
      <g className="fx aurora-drift aurora-drift-2">
        <ellipse cx="220" cy="820" rx="560" ry="340" fill="url(#au-b)" />
      </g>
    </svg>
  );
}

const SCENES: Partial<Record<Scene, () => ReactNode>> = {
  aurora: Aurora,
  sakura: Sakura,
  kyoto: Kyoto,
  autumn: Autumn,
  winter: Winter,
  ocean: Ocean,
  neon: Neon,
  retro: Retro,
};

export function SceneArt({ scene }: { scene: Scene }) {
  const Art = SCENES[scene];
  return Art ? <Art /> : null;
}
