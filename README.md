# GameLauncher

Launcher de jeux unifié pour Windows : une seule bibliothèque pour Steam, Epic, Riot, Battle.net, GOG, Ubisoft, EA, Xbox et les jeux sans launcher. Voir `cahier-des-charges-launcher.md`.

## Prérequis

- Windows 10/11 x64, WebView2 (présent par défaut sur Windows 11)
- Node.js 20+, Rust (stable, cible `x86_64-pc-windows-msvc`), outils de build C++ de Visual Studio

## Commandes

```bash
npm install            # dépendances de l'interface
npm run tauri dev      # lancer l'application en développement
npm run tauri build    # construire l'exécutable et l'installeur
npm run check          # typage, tests de l'interface et tests Rust
npm run dev            # interface seule dans un navigateur, avec un backend simulé
```

## Organisation

- `src/` : interface React (bibliothèque, fiche jeu, statistiques, paramètres)
- `src-tauri/src/` : cœur Rust (providers, base SQLite, lancement, suivi des heures)
- `src-tauri/tests/fixtures/` : fichiers d'exemple pour les tests des parseurs
- `PROGRESS.md`, `DECISIONS.md`, `TODO.md` : avancement, décisions, reste à faire

Les données (base, images, journaux) sont dans `%APPDATA%\com.gamelauncher.app`.
