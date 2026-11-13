# Avancement

| Version | État | Tag |
|---|---|---|
| V0.1 – Socle | ✅ terminé | `v0.1.0` |
| V0.2 – Local et heures | ⏳ | |
| V0.3 – Steam en ligne | ⏳ | |
| V0.4 – Autres launchers | ⏳ | |
| V1.0 – Version complète | ⏳ | |
| V1.x | ⏳ | |

## V0.1 – Socle

- Projet Tauri 2 + React + TypeScript + Vite + Tailwind, dépôt Git.
- Base SQLite avec le schéma complet du §5 (migrations versionnées).
- Provider Steam : `libraryfolders.vdf` (multi-disques), manifestes `.acf`, exclusion des outils, lancement `steam://rungameid/<appid>`.
- Service de bibliothèque : intégration des scans, jeux désinstallés conservés, disques débranchés, changement de lettre de lecteur, dédoublonnage par chemin.
- Détection automatique au démarrage en arrière-plan, bouton « Actualiser », isolation des providers (erreur ou panique d'un module sans effet sur les autres).
- Interface : bibliothèque en grille et en liste virtualisées, recherche, barre latérale par source, notifications, journal dans un fichier.
- Vérifié sur cette machine : 19 jeux Steam sur 2 disques détectés en 82 ms (20 manifestes dont 1 outil exclu).
- Tests : 24 tests Rust, 13 tests front.
