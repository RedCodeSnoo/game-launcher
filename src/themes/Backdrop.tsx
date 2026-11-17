// Fond de la fenêtre : dégradé du thème, image personnelle, jaquette floutée (thème dynamique), scène SVG et
// particules. Tout est en pause (et les particules arrêtées) quand les effets ne doivent pas tourner.
import { useEffect, useRef } from "react";
import { ParticleField } from "./particles";
import { SceneArt } from "./scenes";
import { useTheme } from "./ThemeContext";

function Particles() {
  const { active, fxActive, reduced } = useTheme();
  const canvas = useRef<HTMLCanvasElement>(null);
  const field = useRef<ParticleField | null>(null);
  const kind = reduced ? "none" : active.particles.kind;
  const density = active.particles.density;

  useEffect(() => {
    if (!canvas.current || kind === "none") return;
    const f = new ParticleField(canvas.current, kind, density);
    field.current = f;
    let timer = 0;
    const onResize = () => {
      clearTimeout(timer);
      timer = window.setTimeout(() => f.resize(), 150);
    };
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      clearTimeout(timer);
      f.destroy();
      field.current = null;
    };
  }, [kind, density]);

  useEffect(() => {
    if (fxActive) field.current?.start();
    else field.current?.stop();
  }, [fxActive, kind, density]);

  if (kind === "none") return null;
  return <canvas ref={canvas} className="absolute inset-0 h-full w-full" />;
}

export function Backdrop() {
  const { active, dynamicImage } = useTheme();
  const image = active.background.image;
  return (
    <div className="backdrop pointer-events-none fixed inset-0 -z-10 overflow-hidden" aria-hidden="true">
      {image && (
        <div
          className="absolute -inset-10 bg-cover bg-center"
          style={{ backgroundImage: `url("${image}")`, filter: active.background.imageBlur ? `blur(${active.background.imageBlur}px)` : undefined }}
        />
      )}
      {dynamicImage && <div className="backdrop-dynamic absolute -inset-16 bg-cover bg-center" style={{ backgroundImage: `url("${dynamicImage}")` }} />}
      <SceneArt scene={active.background.scene} />
      <Particles />
    </div>
  );
}
