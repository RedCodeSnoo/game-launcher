"""Génère les thèmes intégrés (src/themes/builtin/*.json). Relancer après modification d'une palette :
    python scripts/make-themes.py
Les contrastes (WCAG AA) sont vérifiés par src/themes/themes.test.ts."""
import json, os

ROOT = os.path.join(os.path.dirname(__file__), "..", "src", "themes", "builtin")

def theme(id, name, emoji, description, scheme, palette, fonts, card, background, particles=("none", 1), icons=(1.75, "line"), sounds="soft", motion=160):
    return {
        "id": id,
        "name": name,
        "emoji": emoji,
        "description": description,
        "scheme": scheme,
        "palette": palette,
        "fonts": fonts,
        "card": card,
        "background": background,
        "particles": {"kind": particles[0], "density": particles[1]},
        "icons": {"stroke": icons[0], "style": icons[1]},
        "sounds": sounds,
        "motion": motion,
    }

def pal(bg, panel, panel2, line, fg, muted, accent, accentFg, ok, warn, danger, scrim, onScrim="#ffffff"):
    return dict(bg=bg, panel=panel, panel2=panel2, line=line, fg=fg, muted=muted, accent=accent, accentFg=accentFg, ok=ok, warn=warn, danger=danger, scrim=scrim, onScrim=onScrim)

THEMES = [
    theme("dark", "Sombre", "🌑", "Moderne et sobre : anthracite, accent bleu électrique, cartes en verre.", "dark",
          pal("#0d0f15", "#161922d9", "#1f2330", "#2a3042", "#e9ecf3", "#9aa2b6", "#2a63f5", "#ffffff", "#34c27e", "#e6a93c", "#f05a6e", "#05070b"),
          {"body": "Inter", "display": "Inter", "scale": 1},
          {"radius": 12, "border": "solid", "glass": True, "blur": 14, "shadow": "soft", "pattern": "none"},
          {"gradient": "radial-gradient(1200px 700px at 88% -12%, #1a2a5c 0%, transparent 62%), radial-gradient(900px 620px at -8% 112%, #26194a 0%, transparent 58%), #0d0f15", "scene": "aurora"}),
    theme("light", "Clair", "☀️", "Propre et lumineux : blanc cassé, accent indigo.", "light",
          pal("#f4f3f0", "#ffffff", "#ebeae6", "#d9d7d1", "#1b1c22", "#5d6070", "#4f46e5", "#ffffff", "#18804b", "#9a6100", "#c6283f", "#0b0c10"),
          {"body": "Inter", "display": "Inter", "scale": 1},
          {"radius": 10, "border": "solid", "glass": False, "blur": 0, "shadow": "soft", "pattern": "none"},
          {"scene": "none"}),
    theme("sakura", "Sakura", "🌸", "Le Japon au printemps : rose pâle, crème et prune, pétales de cerisier, torii et vagues seigaiha.", "light",
          pal("#fbf3f1", "#fffaf8e0", "#f6e4e6", "#ecd0d6", "#3b1f2b", "#77505f", "#c42a60", "#ffffff", "#2b7d52", "#9a5a08", "#b82337", "#2a1220"),
          {"body": "Noto Sans JP", "display": "Noto Serif JP", "scale": 1},
          {"radius": 16, "border": "solid", "glass": True, "blur": 12, "shadow": "soft", "pattern": "seigaiha"},
          {"gradient": "linear-gradient(180deg, #fde4ea 0%, #fbf1ef 42%, #f8e7ea 100%)", "scene": "sakura"},
          particles=("petals", 1), icons=(1.5, "line"), motion=220),
    theme("kyoto", "Nuit à Kyoto", "🏮", "Kyoto la nuit : bleu nuit, laque rouge et or, lanternes, lune et lucioles.", "dark",
          pal("#0b1022", "#121a33e0", "#1b2546", "#2c3760", "#f3ebdb", "#aba6bb", "#e3b44c", "#1a1205", "#5cc28d", "#f0a64a", "#ec5464", "#050814"),
          {"body": "Noto Serif JP", "display": "Noto Serif JP", "scale": 1},
          {"radius": 10, "border": "glow", "glass": True, "blur": 12, "shadow": "deep", "pattern": "asanoha"},
          {"gradient": "radial-gradient(900px 520px at 80% 6%, #1e2c5e 0%, transparent 62%), linear-gradient(180deg, #090e22 0%, #101938 58%, #1a1430 100%)", "scene": "kyoto"},
          particles=("fireflies", 0.7), icons=(1.6, "glow"), motion=240),
    theme("autumn", "Automne", "🍂", "Chaleureux et cosy : orange brûlé, bordeaux et or, feuilles d'érable et grain de papier.", "dark",
          pal("#1e130f", "#2a1b15e6", "#36241c", "#4c3327", "#f7e9d9", "#c9ad97", "#ec8a3c", "#1a0c04", "#9ccb63", "#ecb84e", "#ef6a6a", "#110906"),
          {"body": "Inter", "display": "Fraunces", "scale": 1},
          {"radius": 14, "border": "solid", "glass": True, "blur": 10, "shadow": "deep", "pattern": "paper"},
          {"gradient": "radial-gradient(1000px 620px at 12% -12%, #50261a 0%, transparent 62%), radial-gradient(820px 520px at 104% 104%, #3d1622 0%, transparent 60%), #1e130f", "scene": "autumn"},
          particles=("leaves", 0.9), motion=200),
    theme("winter", "Hiver", "❄️", "Calme et givré : bleu glacier, blanc et gris perle, flocons et givre sur les cartes.", "light",
          pal("#eef4f8", "#ffffffd6", "#e2ebf2", "#c7d7e3", "#13202b", "#4b6070", "#1d6fa8", "#ffffff", "#167a55", "#925e00", "#bf2b41", "#0b1620"),
          {"body": "Inter", "display": "Inter", "scale": 1},
          {"radius": 14, "border": "frost", "glass": True, "blur": 16, "shadow": "soft", "pattern": "frost"},
          {"gradient": "linear-gradient(180deg, #d6e8f4 0%, #eaf2f8 48%, #f6f9fc 100%)", "scene": "winter"},
          particles=("snow", 1), icons=(1.6, "line"), motion=200),
    theme("ocean", "Océan", "🌊", "Frais comme l'été : turquoise, bleu profond et sable, vagues et bulles.", "dark",
          pal("#062033", "#0b2c44e0", "#103a57", "#1f5274", "#eaf6f8", "#9fc2ce", "#2bc8b9", "#032b29", "#62d6a0", "#f2c36b", "#ff7180", "#021320"),
          {"body": "Inter", "display": "Inter", "scale": 1},
          {"radius": 16, "border": "solid", "glass": True, "blur": 14, "shadow": "soft", "pattern": "none"},
          {"gradient": "linear-gradient(180deg, #0b3a57 0%, #072538 52%, #04192a 100%)", "scene": "ocean"},
          particles=("bubbles", 0.7), motion=200),
    theme("neon", "Néon", "🌆", "Rétro-futuriste : noir violacé, magenta et cyan, grille en perspective, lueurs et scanlines.", "dark",
          pal("#0d0614", "#160b24e6", "#22113a", "#3d1f60", "#f4ecff", "#b9a8d6", "#ff3cb8", "#1a0012", "#3cf5c3", "#ffd23f", "#ff5c7a", "#07020c"),
          {"body": "Rajdhani", "display": "Orbitron", "scale": 1.08},
          {"radius": 6, "border": "glow", "glass": True, "blur": 10, "shadow": "glow", "pattern": "scanlines"},
          {"gradient": "linear-gradient(180deg, #10041c 0%, #1b0732 52%, #2a0a3c 100%)", "scene": "neon"},
          icons=(1.75, "glow"), motion=140),
    theme("retro", "Rétro Arcade", "👾", "Pixel art des années 80 : noir et couleurs CGA, étoiles en pixels, sons 8 bits.", "dark",
          pal("#05050c", "#0d0d1a", "#191930", "#3c3c66", "#f4f4f4", "#acacd0", "#55ffff", "#000000", "#55ff55", "#ffff55", "#ff6b6b", "#000000"),
          {"body": "VT323", "display": "Press Start 2P", "scale": 1.18},
          {"radius": 0, "border": "pixel", "glass": False, "blur": 0, "shadow": "hard", "pattern": "pixel"},
          {"gradient": "#05050c", "scene": "retro"},
          particles=("pixelstars", 1), icons=(2.5, "pixel"), sounds="8bit", motion=0),
    theme("oled", "OLED", "⚫", "Minimaliste : noir pur, blanc et un seul accent, sans effet. Économe sur les écrans OLED.", "dark",
          pal("#000000", "#000000", "#0e0e0e", "#262626", "#ffffff", "#a6a6a6", "#0a84ff", "#000000", "#30d158", "#ffd60a", "#ff453a", "#000000"),
          {"body": "Inter", "display": "Inter", "scale": 1},
          {"radius": 10, "border": "solid", "glass": False, "blur": 0, "shadow": "none", "pattern": "none"},
          {"gradient": "#000000", "scene": "none"},
          sounds="none", motion=120),
]

os.makedirs(ROOT, exist_ok=True)
for i, t in enumerate(THEMES):
    with open(os.path.join(ROOT, f"{t['id']}.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump({"order": i, **t}, f, ensure_ascii=False, indent=2)
        f.write("\n")
print(f"{len(THEMES)} thèmes écrits dans {os.path.normpath(ROOT)}")
