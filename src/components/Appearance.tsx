// Paramètres → Apparence : galerie des thèmes (aperçu en direct au survol), options, éditeur, import / export.
import { useEffect, useMemo, useRef, useState } from "react";
import { Check, Download, ImagePlus, Palette as PaletteIcon, Trash2, Upload, X } from "lucide-react";
import { api, pickFile, pickSavePath } from "../api";
import { errorMessage } from "../i18n";
import { BUILTIN_THEMES } from "../themes/builtin";
import { SceneArt } from "../themes/scenes";
import { contrast, fitAccent, PARTICLE_KINDS, readableOn, validateTheme, type ParticleKind, type Theme } from "../themes/schema";
import { useTheme } from "../themes/ThemeContext";
import { placeholderGradient } from "./Cover";
import { useToasts } from "./Toasts";
import { Button, Field, Modal, TextInput } from "./ui";

/** Vignette d'un thème, dessinée avec ses propres couleurs, polices et scène. */
export function ThemeThumb({ theme, selected }: { theme: Theme; selected: boolean }) {
  const p = theme.palette;
  return (
    <div
      className="relative aspect-[16/10] w-full overflow-hidden"
      style={{ background: theme.background.gradient ?? p.bg, borderRadius: Math.min(theme.card.radius, 14), fontFamily: `"${theme.fonts.body}", system-ui` }}
    >
      {theme.background.image && <div className="absolute inset-0 bg-cover bg-center" style={{ backgroundImage: `url("${theme.background.image}")` }} />}
      <div className="absolute inset-0 opacity-90">
        <SceneArt scene={theme.background.scene} />
      </div>
      <div className="absolute inset-y-0 left-0 w-[22%] border-r" style={{ background: p.panel, borderColor: p.line }}>
        <div className="m-[10%] h-[6%] rounded-sm" style={{ background: p.accent }} />
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="mx-[10%] mt-[12%] h-[4%] rounded-sm" style={{ background: i === 0 ? p.fg : p.muted, opacity: i === 0 ? 0.8 : 0.45 }} />
        ))}
      </div>
      <div className="absolute left-[28%] right-[5%] top-[10%] flex gap-[5%]">
        {[0, 1, 2].map((i) => (
          <div
            key={i}
            className="aspect-[2/3] flex-1"
            style={{
              borderRadius: Math.min(theme.card.radius, 10) / 2,
              background: placeholderGradient((i * 97 + theme.id.length * 31) % 360).replaceAll("var(--accent)", p.accent).replaceAll("var(--scrim)", p.scrim),
              boxShadow: theme.card.border === "glow" ? `0 0 6px ${p.accent}` : theme.card.border === "pixel" ? `2px 2px 0 ${p.accent}` : undefined,
            }}
          />
        ))}
      </div>
      <div className="absolute bottom-0 left-[22%] right-0 flex items-center justify-between gap-2 px-[6%] py-[4%]" style={{ background: `linear-gradient(transparent, ${p.bg}ee)` }}>
        <span className="truncate text-sm font-semibold" style={{ color: p.fg, fontFamily: `"${theme.fonts.display}", system-ui`, fontSizeAdjust: theme.fonts.display === "Press Start 2P" ? 0.36 : undefined }}>
          {theme.name}
        </span>
        <span className="h-3 w-3 shrink-0 rounded-full" style={{ background: p.accent }} />
      </div>
      {selected && (
        <span className="absolute right-2 top-2 flex h-6 w-6 items-center justify-center rounded-full" style={{ background: p.accent, color: p.accentFg }}>
          <Check size={14} />
        </span>
      )}
    </div>
  );
}

function download(theme: Theme): string {
  const { custom: _c, ...rest } = theme;
  return JSON.stringify({ ...rest, $schema: "gamelauncher-theme-1" }, null, 2);
}

/** Éditeur : part d'un thème existant, change l'accent, le fond, les particules et le flou. */
function ThemeEditor({ from, onClose }: { from: Theme; onClose: () => void }) {
  const toasts = useToasts();
  const { themes, preview, saveCustom, setTheme } = useTheme();
  const [base, setBase] = useState<Theme>(from);
  const [name, setName] = useState(from.custom ? from.name : `${from.name} perso`);
  const [accent, setAccent] = useState(from.palette.accent);
  const [image, setImage] = useState<string | undefined>(from.background.image);
  const [imageBlur, setImageBlur] = useState(from.background.imageBlur ?? 0);
  const [kind, setKind] = useState<ParticleKind>(from.particles.kind);
  const [density, setDensity] = useState(from.particles.density);
  const [blur, setBlur] = useState(from.card.glass ? from.card.blur : 0);

  const fittedAccent = fitAccent(accent, base.palette.panel, base.palette.bg);
  const draft = useMemo<Theme>(
    () => ({
      ...base,
      id: from.custom ? from.id : `perso-${Date.now().toString(36)}`,
      name: name.trim() || "Thème perso",
      description: `Thème personnel basé sur ${themes.find((t) => t.id === (base.base ?? base.id))?.name ?? base.name}.`,
      custom: true,
      base: base.base ?? base.id,
      palette: { ...base.palette, accent: fittedAccent, accentFg: readableOn(fittedAccent) },
      background: { ...base.background, image, imageBlur },
      particles: { kind, density },
      card: { ...base.card, glass: blur > 0, blur },
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [base, name, fittedAccent, image, imageBlur, kind, density, blur],
  );

  useEffect(() => {
    preview(draft);
  }, [draft, preview]);
  useEffect(() => () => preview(null), [preview]);

  const pickBase = (id: string) => {
    const t = themes.find((x) => x.id === id);
    if (!t) return;
    setBase(t);
    setAccent(t.palette.accent);
    setKind(t.particles.kind);
    setDensity(t.particles.density);
    setBlur(t.card.glass ? t.card.blur : 0);
  };

  const save = async () => {
    const { theme, errors } = validateTheme(draft);
    if (!theme) return toasts.error(errors.join(" "));
    try {
      await saveCustom(theme);
      setTheme(theme.id);
      toasts.success(`Thème « ${theme.name} » enregistré`);
      onClose();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  return (
    <Modal
      title={from.custom ? `Modifier « ${from.name} »` : "Créer un thème"}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Annuler</Button>
          <Button variant="primary" onClick={() => void save()}>
            Enregistrer le thème
          </Button>
        </>
      }
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label="Nom">
          <TextInput value={name} maxLength={40} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field label="Thème de départ">
          <select className="h-9 rounded-md border border-line bg-bg px-2 text-sm" value={base.id} onChange={(e) => pickBase(e.target.value)}>
            {themes.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Couleur d'accent" hint={fittedAccent !== accent.toLowerCase() ? "Ajustée automatiquement pour rester lisible." : undefined}>
          <div className="flex items-center gap-2">
            <input type="color" value={accent} onChange={(e) => setAccent(e.target.value)} className="h-9 w-14 cursor-pointer rounded-md border border-line bg-bg p-1" aria-label="Couleur d'accent" />
            <span className="font-mono text-xs text-muted">{fittedAccent}</span>
            <span className="text-xs text-muted">contraste {contrast(fittedAccent, base.palette.panel, base.palette.bg).toFixed(1)}:1</span>
          </div>
        </Field>
        <Field label="Particules">
          <select className="h-9 rounded-md border border-line bg-bg px-2 text-sm" value={kind} onChange={(e) => setKind(e.target.value as ParticleKind)}>
            {PARTICLE_KINDS.map((k) => (
              <option key={k} value={k}>
                {{ none: "Aucune", petals: "Pétales de cerisier", leaves: "Feuilles d'érable", snow: "Neige", fireflies: "Lucioles", bubbles: "Bulles", pixelstars: "Étoiles en pixels" }[k]}
              </option>
            ))}
          </select>
        </Field>
        <Field label={`Intensité des particules : ${Math.round(density * 100)} %`}>
          <input type="range" min={0} max={3} step={0.1} value={density} disabled={kind === "none"} onChange={(e) => setDensity(Number(e.target.value))} className="accent-[var(--accent)]" />
        </Field>
        <Field label={`Flou des surfaces (verre) : ${blur} px`}>
          <input type="range" min={0} max={40} step={1} value={blur} onChange={(e) => setBlur(Number(e.target.value))} className="accent-[var(--accent)]" />
        </Field>
        <Field label="Image de fond">
          <div className="flex gap-2">
            <Button
              onClick={async () => {
                const path = await pickFile("Image de fond", ["png", "jpg", "jpeg", "webp"]);
                if (!path) return;
                try {
                  setImage(await api.readImageDataUrl(path));
                } catch (e) {
                  toasts.error(errorMessage(e));
                }
              }}
            >
              <ImagePlus size={15} /> Choisir…
            </Button>
            {image && (
              <Button variant="ghost" onClick={() => setImage(undefined)}>
                <X size={15} /> Retirer
              </Button>
            )}
          </div>
        </Field>
        <Field label={`Flou de l'image : ${imageBlur} px`}>
          <input type="range" min={0} max={40} step={1} value={imageBlur} disabled={!image} onChange={(e) => setImageBlur(Number(e.target.value))} className="accent-[var(--accent)]" />
        </Field>
      </div>
      <p className="mt-4 text-xs text-muted">L'aperçu est appliqué à toute la fenêtre pendant l'édition.</p>
    </Modal>
  );
}

function Option({ label, hint, checked, onChange, disabled }: { label: string; hint: string; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted">{hint}</p>
      </div>
      <button
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`relative h-6 w-11 shrink-0 rounded-full transition disabled:opacity-40 ${checked ? "bg-accent" : "bg-line"}`}
      >
        <span className={`absolute top-0.5 h-5 w-5 rounded-full bg-on-scrim shadow transition-all ${checked ? "left-[1.375rem]" : "left-0.5"}`} />
      </button>
    </div>
  );
}

export function Appearance() {
  const toasts = useToasts();
  const { themes, custom, chosenId, active, setTheme, preview, deleteCustom, saveCustom, options, setOption, reduced } = useTheme();
  const [editing, setEditing] = useState<Theme | null>(null);
  // Aperçu au survol : seulement si la souris s'arrête sur une vignette (pas à chaque vignette traversée).
  const hoverTimer = useRef(0);
  const schedulePreview = (t: Theme) => {
    window.clearTimeout(hoverTimer.current);
    hoverTimer.current = window.setTimeout(() => preview(t), 250);
  };
  const stopPreview = () => {
    window.clearTimeout(hoverTimer.current);
    preview(null);
  };
  useEffect(() => () => window.clearTimeout(hoverTimer.current), []);
  const systemReduced = reduced && !options.reduceMotion;

  const exportTheme = async (t: Theme) => {
    const path = await pickSavePath("Exporter le thème", `${t.id}.json`, ["json"]);
    if (!path) return;
    try {
      await api.writeTextFile(path, download(t));
      toasts.success(`Thème « ${t.name} » exporté`);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  };

  const importTheme = async () => {
    const path = await pickFile("Thème GameLauncher", ["json"]);
    if (!path) return;
    try {
      const { theme, errors } = validateTheme(JSON.parse(await api.readTextFile(path)));
      if (!theme) return toasts.error(`Thème invalide : ${errors.slice(0, 3).join(" ")}`);
      const taken = BUILTIN_THEMES.some((t) => t.id === theme.id);
      const imported = { ...theme, id: taken ? `perso-${theme.id}` : theme.id, custom: true };
      await saveCustom(imported);
      setTheme(imported.id);
      toasts.success(`Thème « ${imported.name} » importé`);
    } catch (e) {
      toasts.error(e instanceof SyntaxError ? "Ce fichier n'est pas un JSON valide." : errorMessage(e));
    }
  };

  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-5" onMouseLeave={stopPreview}>
        {themes.map((t) => (
          <div key={t.id} className="group relative">
            <button
              onClick={() => {
                window.clearTimeout(hoverTimer.current);
                setTheme(t.id);
              }}
              onMouseEnter={() => schedulePreview(t)}
              onMouseLeave={() => window.clearTimeout(hoverTimer.current)}
              onFocus={() => schedulePreview(t)}
              onBlur={stopPreview}
              aria-pressed={chosenId === t.id}
              aria-label={`Thème ${t.name}`}
              title={t.description}
              className={`block w-full rounded-lg p-1 text-left ring-2 transition ${chosenId === t.id ? "ring-accent" : "ring-transparent hover:ring-line"}`}
            >
              <ThemeThumb theme={t} selected={chosenId === t.id && !options.season} />
              <span className="mt-1 block truncate px-1 text-xs text-muted">
                {t.emoji} {t.custom ? "Perso" : t.description.split(" : ")[0].split(",")[0]}
              </span>
            </button>
            {t.custom && (
              <div className="absolute right-3 top-3 hidden gap-1 group-hover:flex">
                <button onClick={() => setEditing(t)} className="rounded bg-scrim/70 p-1 text-on-scrim" aria-label={`Modifier ${t.name}`} title="Modifier">
                  <PaletteIcon size={13} />
                </button>
                <button
                  onClick={() => {
                    if (window.confirm(`Supprimer le thème « ${t.name} » ?`)) void deleteCustom(t.id);
                  }}
                  className="rounded bg-scrim/70 p-1 text-on-scrim"
                  aria-label={`Supprimer ${t.name}`}
                  title="Supprimer"
                >
                  <Trash2 size={13} />
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="flex flex-wrap gap-2">
        <Button onClick={() => setEditing(active.custom ? active : themes.find((t) => t.id === chosenId) ?? active)}>
          <PaletteIcon size={15} /> {active.custom ? "Modifier ce thème…" : "Créer un thème à partir de celui-ci…"}
        </Button>
        <Button onClick={() => void importTheme()}>
          <Upload size={15} /> Importer…
        </Button>
        <Button onClick={() => void exportTheme(themes.find((t) => t.id === chosenId) ?? active)}>
          <Download size={15} /> Exporter le thème actuel…
        </Button>
        {custom.length > 0 && <span className="self-center text-xs text-muted">{custom.length} thème(s) perso</span>}
      </div>

      <div className="flex flex-col gap-4 border-t border-line pt-4">
        <Option
          label="Réduire les animations"
          hint={systemReduced ? "Windows demande déjà moins d'animations : particules et fonds animés sont désactivés." : "Désactive les particules et les fonds animés."}
          checked={options.reduceMotion || systemReduced}
          disabled={systemReduced}
          onChange={(v) => setOption("reduceMotion", v)}
        />
        <Option label="Thème dynamique" hint="Dans la fiche d'un jeu, la couleur d'accent et le fond s'inspirent de sa jaquette." checked={options.dynamic} onChange={(v) => setOption("dynamic", v)} />
        <Option
          label="Thème selon la saison"
          hint="Printemps : Sakura · Été : Océan · Automne : Automne · Hiver : Hiver. Choisir un thème désactive cette option."
          checked={options.season}
          onChange={(v) => setOption("season", v)}
        />
        <Option label="Sons d'interface" hint="Un léger son au clic, propre au thème (8 bits pour Rétro Arcade)." checked={options.sounds} onChange={(v) => setOption("sounds", v)} />
      </div>
      {editing && <ThemeEditor from={editing} onClose={() => setEditing(null)} />}
    </div>
  );
}
