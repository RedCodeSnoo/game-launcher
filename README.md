# GameLauncher

Launcher de jeux unifié pour Windows : une seule bibliothèque pour **Steam, Epic Games, Riot Games, Battle.net, GOG, Ubisoft Connect, EA app, Xbox** et les jeux sans launcher, avec un suivi automatique du temps de jeu.

![Bibliothèque](docs/captures/bibliotheque.png)

## Installation

1. Téléchargez `GameLauncher_X.Y.Z_x64-setup.exe` dans la [dernière Release](https://github.com/RedCodeSnoo/game-launcher/releases/latest).
2. Lancez-le (installation pour votre session, sans droits administrateur).

Windows 10 ou 11 (64 bits) avec WebView2, présent par défaut sur Windows 11. L'installeur n'est pas signé par un certificat de développeur : Windows SmartScreen peut afficher un avertissement au premier lancement (« Informations complémentaires », puis « Exécuter quand même »).

Ensuite, l'application se met à jour toute seule (voir plus bas).

## Fonctionnalités

- **Bibliothèque unifiée** : détection automatique des jeux installés (plusieurs disques, disques externes et changement de lettre compris), jeux possédés non installés (Steam avec clé API, Epic, GOG), jeux sans launcher par simple glisser-déposer d'un `.exe` ou par dossiers surveillés.
- **Lancement** par le launcher d'origine ou par l'exécutable direct, au choix pour chaque jeu ; dossier et exécutable corrigeables quand la détection se trompe.
- **Temps de jeu suivi automatiquement**, que le jeu soit lancé depuis GameLauncher, depuis Steam, un raccourci ou un autre launcher (observation des processus toutes les 3 s, sans droits administrateur ni injection). Historique des sessions, temps de départ saisissable à la main, temps Steam importé.
- **Organisation** : accueil « Continuer à jouer », backlog en colonnes avec statuts (À jouer, En cours, Terminé, 100 %…), note sur 10 et notes, journal de session, recherche rapide `Ctrl+K`, roulette « Je sais pas à quoi jouer » 🎲, collections par glisser-déposer et collections automatiques, vues grille, liste ou bandeaux.
- **Statistiques et séries** 🔥 : série de jours joués (avec jokers et rappel du soir), grille d'activité façon GitHub, temps par semaine et par mois, par launcher, par genre, top jeux, carte des habitudes, records, succès Steam (avec rareté) et badges.
- **Jaquettes et fiches** automatiques (cache Steam, catalogue Epic, SteamGridDB en option).
- **Mode Big Picture** 📺 : plein écran pour la télé, tout à la manette (Xbox, PlayStation), `F11` ou Vue + Menu pour entrer et sortir.
- **Récap de l'année** ✨ (à partir du 1er décembre) : temps de jeu, top 5, meilleure série, jour record, launcher préféré et profil de joueur, exportable en image ; page **Profil** avec photo recadrable, pseudo, top 10 de vos jeux préférés (à classer par glisser-déposer) et badges, exportable en image.
- **Intégrations** : jeux gratuits Epic de la semaine, actualités Steam, galerie de captures d'écran (Steam, Xbox Game Bar, NVIDIA), Discord Rich Presence, durée de vie des jeux (IGDB, facultatif), sauvegarde et synchronisation vers OneDrive, Google Drive ou une clé USB.
- **Zone de notification** : GameLauncher reste actif fenêtre fermée, menu des jeux récents et du jeu en cours, démarrage avec Windows.
- Favoris, collections, filtres et tris, thème sombre ou clair, sauvegarde et restauration.

| Accueil | Backlog |
|---|---|
| ![Accueil](docs/captures/accueil.jpg) | ![Backlog](docs/captures/backlog.jpg) |

| Fiche d'un jeu | Statistiques |
|---|---|
| ![Fiche d'un jeu](docs/captures/fiche-jeu.png) | ![Statistiques](docs/captures/statistiques.jpg) |

| Big Picture | Récap de l'année |
|---|---|
| ![Big Picture](docs/captures/bigpicture.jpg) | ![Récap de l'année](docs/captures/recap.jpg) |

| Profil | Image du récap à partager |
|---|---|
| ![Profil](docs/captures/profil.jpg) | ![Image du récap](docs/captures/recap-image.jpg) |

*Captures réalisées avec le backend de démonstration (données fictives).*

Les clés API (Steam, SteamGridDB) sont rangées dans le Gestionnaire d'identification de Windows, jamais dans la base ni dans les sauvegardes. Les données locales sont dans `%APPDATA%\com.gamelauncher.app`.

## Thèmes

Dix thèmes, chacun avec son fond, ses particules, ses polices et ses cartes. Paramètres → Apparence : survolez une vignette pour l'essayer, cliquez pour l'adopter. Un éditeur permet de partir d'un thème (accent, image de fond, particules, flou) et de l'exporter en `.json` pour le partager. Options : animations réduites (suit aussi le réglage de Windows), thème dynamique d'après la jaquette du jeu affiché, thème selon la saison, sons d'interface. Les effets s'arrêtent pendant une partie et quand la fenêtre est masquée.

| | | |
|---|---|---|
| ![Sombre](docs/captures/themes/dark.jpg) **Sombre** | ![Clair](docs/captures/themes/light.jpg) **Clair** | ![Sakura](docs/captures/themes/sakura.jpg) **Sakura** 🌸 |
| ![Nuit à Kyoto](docs/captures/themes/kyoto.jpg) **Nuit à Kyoto** 🏮 | ![Automne](docs/captures/themes/autumn.jpg) **Automne** 🍂 | ![Hiver](docs/captures/themes/winter.jpg) **Hiver** ❄️ |
| ![Océan](docs/captures/themes/ocean.jpg) **Océan** 🌊 | ![Néon](docs/captures/themes/neon.jpg) **Néon** 🌆 | ![Rétro Arcade](docs/captures/themes/retro.jpg) **Rétro Arcade** 👾 |
| ![OLED](docs/captures/themes/oled.jpg) **OLED** ⚫ | ![Apparence](docs/captures/apparence.jpg) **Paramètres → Apparence** | |

Polices sous licence libre (OFL) et visuels dessinés pour le projet (SVG, dégradés, canvas).

## Mises à jour automatiques

GameLauncher vérifie au démarrage, puis toutes les 6 heures (même fenêtre fermée), si une nouvelle version est publiée dans les [Releases](https://github.com/RedCodeSnoo/game-launcher/releases). Si oui : notification Windows, puis bannière avec les notes de version et les boutons **Installer et redémarrer** et **Plus tard**. L'installation n'a jamais lieu pendant une partie : elle attend la fin de la session. Chaque mise à jour est signée, et l'application refuse une mise à jour dont la signature ne correspond pas.

Paramètres > Mises à jour : version actuelle, recherche manuelle, option « Vérifier automatiquement ».

## Développement

Prérequis : Node.js 22, Rust stable (`x86_64-pc-windows-msvc`), outils de build C++ de Visual Studio.

```bash
npm install            # dépendances
npm run tauri dev      # application en développement
npm run dev            # interface seule dans un navigateur, avec un backend simulé (?games=1000, ?update=1)
npm run check          # versions synchronisées, typage, tests de l'interface et tests Rust
npm run build:local    # installeur local, sans les fichiers de mise à jour (pas besoin de la clé de signature)
```

Tests de bout en bout du suivi avec de vrais processus (~25 s) :

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
```

### Publier une nouvelle version

Le numéro de version n'est écrit qu'à un endroit : `package.json` (`tauri.conf.json` le lit, `scripts/sync-version.mjs` le recopie dans `Cargo.toml`).

1. Ajoutez une section `## [X.Y.Z]` en tête de `CHANGELOG.md` (elle devient les notes de version) et commitez.
2. `npm version X.Y.Z` : met à jour les trois fichiers, crée le commit « Version X.Y.Z » et le tag `vX.Y.Z`.
3. `git push --follow-tags`

Le workflow [`release.yml`](.github/workflows/release.yml) compile l'installeur, le signe et publie la Release avec `latest.json` ; les applications installées la proposent ensuite d'elles-mêmes. Il a besoin des secrets `TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` du dépôt.

### Organisation

- `src/` : interface React + TypeScript + Tailwind
- `src-tauri/src/` : cœur Rust (Tauri 2) : providers par launcher, base SQLite, lancement, `tracker.rs` et `procs.rs` (suivi des processus), `playtime.rs` (calcul du temps total), `updates.rs` (mises à jour), `tray.rs` (zone de notification)
- `src-tauri/tests/fixtures/` : fichiers d'exemple pour les tests des analyseurs
- `cahier-des-charges-launcher.md`, `evolutions-v1.1.md` : spécifications ; `PROGRESS.md`, `DECISIONS.md`, `TODO.md` : avancement, décisions, reste à faire

## Licence

[MIT](LICENSE) © RedCodeSnoo
