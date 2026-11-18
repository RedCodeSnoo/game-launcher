// Recadrage de la photo de profil : glisser pour déplacer, molette ou curseur pour zoomer.
// Le résultat (320 × 320, JPEG) est enregistré dans les réglages : aucune copie du fichier d'origine.
import { useEffect, useRef, useState } from "react";
import { ZoomIn, ZoomOut } from "lucide-react";
import { Button } from "./ui";

const VIEW = 280; // diamètre du cadre à l'écran
const OUT = 320; // taille de l'image enregistrée

export function PhotoEditor({ src, onCancel, onSave }: { src: string; onCancel: () => void; onSave: (dataUrl: string) => void }) {
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const [zoom, setZoom] = useState(1);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const drag = useRef<{ x: number; y: number; px: number; py: number } | null>(null);

  useEffect(() => {
    const i = new Image();
    i.onload = () => setImg(i);
    i.src = src;
  }, [src]);

  // Échelle de base : l'image couvre tout le cadre au zoom 1.
  const base = img ? VIEW / Math.min(img.naturalWidth, img.naturalHeight) : 1;
  const scale = base * zoom;
  const w = (img?.naturalWidth ?? 0) * scale;
  const h = (img?.naturalHeight ?? 0) * scale;
  // L'image ne laisse jamais de vide dans le cadre.
  const clamp = (p: { x: number; y: number }) => ({ x: Math.min(0, Math.max(VIEW - w, p.x)), y: Math.min(0, Math.max(VIEW - h, p.y)) });
  useEffect(() => {
    if (img) setPos({ x: (VIEW - img.naturalWidth * base) / 2, y: (VIEW - img.naturalHeight * base) / 2 });
  }, [img, base]);

  const setZoomAround = (z: number) => {
    const nz = Math.min(4, Math.max(1, z));
    // Zoom centré sur le milieu du cadre.
    const k = nz / zoom;
    setPos((p) => {
      const nw = (img?.naturalWidth ?? 0) * base * nz;
      const nh = (img?.naturalHeight ?? 0) * base * nz;
      const nx = VIEW / 2 - (VIEW / 2 - p.x) * k;
      const ny = VIEW / 2 - (VIEW / 2 - p.y) * k;
      return { x: Math.min(0, Math.max(VIEW - nw, nx)), y: Math.min(0, Math.max(VIEW - nh, ny)) };
    });
    setZoom(nz);
  };

  const save = () => {
    if (!img) return;
    const canvas = document.createElement("canvas");
    canvas.width = OUT;
    canvas.height = OUT;
    const c = canvas.getContext("2d")!;
    const f = OUT / VIEW;
    c.imageSmoothingQuality = "high";
    c.drawImage(img, pos.x * f, pos.y * f, w * f, h * f);
    onSave(canvas.toDataURL("image/jpeg", 0.9));
  };

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-scrim/60 p-4 backdrop-blur-sm" onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div role="dialog" aria-modal="true" aria-label="Ajuster la photo" className="w-full max-w-sm rounded-2xl border border-line bg-panel p-6 shadow-2xl">
        <h2 className="text-base font-semibold">Ajuster la photo</h2>
        <p className="mt-1 text-sm text-muted">Glissez pour placer, molette ou curseur pour zoomer.</p>
        <div
          className="relative mx-auto mt-5 cursor-grab touch-none overflow-hidden rounded-full bg-panel2 ring-4 ring-accent active:cursor-grabbing"
          style={{ width: VIEW, height: VIEW }}
          onPointerDown={(e) => {
            (e.target as HTMLElement).setPointerCapture(e.pointerId);
            drag.current = { x: e.clientX, y: e.clientY, px: pos.x, py: pos.y };
          }}
          onPointerMove={(e) => {
            const d = drag.current;
            if (d) setPos(clamp({ x: d.px + e.clientX - d.x, y: d.py + e.clientY - d.y }));
          }}
          onPointerUp={() => (drag.current = null)}
          onWheel={(e) => setZoomAround(zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1))}
        >
          {img && <img src={src} alt="" draggable={false} className="pointer-events-none absolute max-w-none select-none" style={{ left: pos.x, top: pos.y, width: w, height: h }} />}
        </div>
        <div className="mt-5 flex items-center gap-3">
          <button onClick={() => setZoomAround(zoom / 1.2)} className="text-muted hover:text-fg" aria-label="Dézoomer">
            <ZoomOut size={18} />
          </button>
          <input type="range" min={1} max={4} step={0.01} value={zoom} onChange={(e) => setZoomAround(Number(e.target.value))} className="flex-1 accent-[var(--accent)]" aria-label="Zoom" />
          <button onClick={() => setZoomAround(zoom * 1.2)} className="text-muted hover:text-fg" aria-label="Zoomer">
            <ZoomIn size={18} />
          </button>
        </div>
        <div className="mt-6 flex justify-end gap-2">
          <Button onClick={onCancel}>Annuler</Button>
          <Button variant="primary" onClick={save} disabled={!img}>
            Enregistrer
          </Button>
        </div>
      </div>
    </div>
  );
}
