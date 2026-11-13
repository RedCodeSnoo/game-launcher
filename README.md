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

## Fonctionnement en arrière-plan

- Fermer la fenêtre laisse GameLauncher actif dans la zone de notification (près de l'horloge) ; pour l'arrêter : clic droit sur l'icône, puis **Quitter**. Une seule instance tourne à la fois.
- Tant qu'il tourne, GameLauncher enregistre le temps de jeu de tous les jeux de la bibliothèque, même lancés depuis Steam, un raccourci ou un autre launcher (lecture de la liste des processus toutes les 3 s, sans droits administrateur).
- Paramètres > Arrière-plan et démarrage : démarrage avec Windows, démarrage réduit, comportement de la croix.
- Paramètres > Suivi du temps de jeu : suivi des jeux lancés hors de l'application, durée minimale d'une session, processus à ignorer.
- Fiche d'un jeu en cours : « Arrêter le suivi » (clôt la session sans toucher au jeu) et « Forcer la fermeture du jeu ».
- Fiche d'un jeu > Paramètres du jeu : dossier, exécutable principal et méthode de lancement personnalisés.

Tests de bout en bout du suivi avec de vrais processus (~25 s) :

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
```

## Organisation

- `src/` : interface React (bibliothèque, fiche jeu, statistiques, paramètres)
- `src-tauri/src/` : cœur Rust (providers, base SQLite, lancement) ; `tracker.rs` et `procs.rs` : suivi des heures par observation des processus ; `tray.rs` : zone de notification
- `evolutions-v1.1.md` : évolutions E1 à E4 (livrées sous le tag `v1.1`)
- `src-tauri/tests/fixtures/` : fichiers d'exemple pour les tests des parseurs
- `PROGRESS.md`, `DECISIONS.md`, `TODO.md` : avancement, décisions, reste à faire

Les données (base, images, journaux) sont dans `%APPDATA%\com.gamelauncher.app`.
