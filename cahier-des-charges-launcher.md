# Cahier des charges – Launcher de jeux unifié

*Version 1.0 – 13 novembre 2026*

---

## 1. Contexte et objectifs

### 1.1 Contexte
Les jeux PC sont éparpillés entre de nombreux launchers (Steam, Epic Games, Riot, Battle.net, GOG, Ubisoft Connect, EA app, Xbox…) et des jeux installés « à la main », sans aucun launcher. Il faut ouvrir plusieurs applications pour retrouver un jeu, et les statistiques (heures de jeu, succès) sont dispersées.

### 1.2 Objectif général
Développer une application de bureau Windows qui **centralise toute la bibliothèque de jeux** de l'utilisateur, quelle que soit sa provenance, et permet de lancer chaque jeu depuis un seul endroit.

### 1.3 Objectifs détaillés
| Réf. | Objectif |
|---|---|
| O1 | **Centraliser** : une seule bibliothèque pour tous les jeux installés, y compris les jeux sans launcher |
| O2 | **Lancer** : démarrer n'importe quel jeu en un clic |
| O3 | **Statistiques** : afficher les heures de jeu et les succès quand l'information existe |
| O4 | **Bibliothèque complète** : afficher aussi les jeux possédés mais non installés (avec leurs heures et succès) quand la source le permet |
| O5 | **Multi-disques** : détecter les jeux sur tous les disques, y compris les disques durs externes |

### 1.4 Utilisateur cible
Usage personnel, mono-utilisateur, sur PC Windows 10/11. Une diffusion publique n'est pas un objectif de la V1, mais l'architecture ne doit pas l'empêcher.

---

## 2. Périmètre

### 2.1 Sources supportées
| Priorité | Source | Installés | Non installés | Heures | Succès |
|---|---|---|---|---|---|
| P1 | Steam | ✅ | ✅ | ✅ | ✅ |
| P1 | Jeux locaux (sans launcher) | ✅ | – | ✅ (suivi maison) | ❌ « Aucun succès » |
| P1 | Epic Games | ✅ | ⚠️ | ⚠️ / suivi maison | ⚠️ |
| P1 | Riot Games | ✅ | – | suivi maison | ❌ |
| P2 | Battle.net | ✅ | ❌ | suivi maison | ❌ |
| P2 | GOG Galaxy | ✅ | ✅ | ✅ | ✅ |
| P3 | Ubisoft Connect | ✅ | ⚠️ | suivi maison | ⚠️ |
| P3 | EA app | ✅ | ⚠️ | suivi maison | ❌ |
| P3 | Xbox / Game Pass (PC) | ⚠️ | ❌ | suivi maison | ❌ |

✅ = disponible · ⚠️ = partiel / API non officielle · ❌ = non disponible · – = sans objet

### 2.2 Hors périmètre (V1)
- Achat de jeux, boutique intégrée
- Téléchargement / installation / mise à jour des jeux (délégués au launcher d'origine)
- Fonctions sociales (amis, chat)
- Versions macOS / Linux
- Synchronisation cloud entre plusieurs PC
- Émulateurs avec gestion de ROMs (envisageable en V3+, voir §9)

---

## 3. Exigences fonctionnelles

### 3.1 Détection des jeux installés
| Réf. | Exigence | Priorité |
|---|---|---|
| F-DET-01 | Au premier lancement, l'application détecte automatiquement les launchers installés | Must |
| F-DET-02 | Pour chaque launcher détecté, elle liste les jeux installés (nom, chemin, identifiant) | Must |
| F-DET-03 | La détection inclut les bibliothèques situées sur d'autres disques (internes ou externes) | Must |
| F-DET-04 | Un bouton « Actualiser » relance la détection à la demande | Must |
| F-DET-05 | Une détection automatique a lieu à chaque démarrage (en arrière-plan, sans bloquer l'UI) | Should |
| F-DET-06 | Un jeu désinstallé disparaît des « installés » mais reste dans l'historique (heures conservées) | Must |

### 3.2 Jeux locaux (sans launcher)
| Réf. | Exigence | Priorité |
|---|---|---|
| F-LOC-01 | Ajout manuel d'un jeu en sélectionnant un `.exe` (sélecteur de fichier ou glisser-déposer) | Must |
| F-LOC-02 | L'utilisateur peut définir des **dossiers surveillés** (ex. `D:\Jeux`, `E:\`) | Must |
| F-LOC-03 | Le scan des dossiers surveillés propose des jeux candidats en filtrant les faux positifs (`unins*.exe`, `*CrashHandler*.exe`, `vc_redist*.exe`, `setup.exe`, `dxsetup.exe`, `UE4PrereqSetup*.exe`, etc.) | Must |
| F-LOC-04 | Chaque candidat doit être validé par l'utilisateur avant d'être ajouté | Must |
| F-LOC-05 | Arguments de lancement et dossier de travail personnalisables par jeu | Should |
| F-LOC-06 | Détection optionnelle via les raccourcis du Bureau et du menu Démarrer | Could |
| F-LOC-07 | Les jeux locaux affichent « Aucun succès » | Must |
| F-LOC-08 | Un jeu détecté à la fois par un launcher et par le scan local n'apparaît qu'une fois (dédoublonnage par chemin) | Must |

### 3.3 Disques externes
| Réf. | Exigence | Priorité |
|---|---|---|
| F-EXT-01 | Tous les lecteurs montés (fixes et amovibles) sont pris en compte | Must |
| F-EXT-02 | Les chemins sont stockés avec un identifiant de volume (numéro de série) en plus de la lettre, pour qu'un disque qui change de lettre (`E:` → `F:`) soit retrouvé | Should |
| F-EXT-03 | Un jeu dont le disque est débranché est affiché « Indisponible – disque non connecté » (grisé), sans être supprimé | Must |
| F-EXT-04 | Le branchement d'un disque déclenche une détection automatique de ses jeux | Could |

### 3.4 Bibliothèque complète (jeux non installés)
| Réf. | Exigence | Priorité |
|---|---|---|
| F-BIB-01 | Affichage des jeux possédés non installés pour les sources qui le permettent (Steam, GOG, Epic si possible) | Must (Steam) / Should (autres) |
| F-BIB-02 | Un jeu non installé affiche ses heures et ses succès si disponibles | Must |
| F-BIB-03 | Un bouton « Installer » ouvre le launcher d'origine sur la page du jeu (ex. `steam://install/<appid>`) | Should |

### 3.5 Lancement
| Réf. | Exigence | Priorité |
|---|---|---|
| F-LAN-01 | Lancement d'un jeu en un clic, avec la méthode propre à sa source (URI, CLI du launcher ou exécutable direct) | Must |
| F-LAN-02 | Message d'erreur clair si le lancement échoue (launcher absent, fichier introuvable, disque débranché) | Must |
| F-LAN-03 | Indicateur « En cours » tant que le jeu tourne | Should |
| F-LAN-04 | Option : réduire le launcher dans la zone de notification pendant le jeu | Could |

### 3.6 Heures de jeu
| Réf. | Exigence | Priorité |
|---|---|---|
| F-HEU-01 | Import des heures depuis la source quand elle les fournit (Steam, GOG…) | Must |
| F-HEU-02 | **Suivi maison** : pour tout jeu lancé depuis l'application, surveillance du processus et enregistrement de chaque session (début, fin, durée) | Must |
| F-HEU-03 | Le suivi détecte le vrai processus du jeu même si le lanceur intermédiaire se ferme (ex. un `launcher.exe` qui démarre `game.exe`) : surveillance des processus dont l'exécutable est dans le dossier du jeu | Must |
| F-HEU-04 | Affichage par jeu : temps total, dernière session, date de dernier lancement | Must |
| F-HEU-05 | Quand les deux existent, le temps affiché est celui de la source, et le temps suivi localement est visible à part (pas de double comptage) | Must |
| F-HEU-06 | Statistiques globales : temps total, top jeux, temps par source, temps par semaine/mois | Should |
| F-HEU-07 | Correction manuelle du temps de jeu | Could |

### 3.7 Succès
| Réf. | Exigence | Priorité |
|---|---|---|
| F-SUC-01 | Affichage de la liste des succès (nom, description, icône, débloqué ou non, date de déblocage) | Must (Steam) |
| F-SUC-02 | Progression : X / Y succès débloqués, en pourcentage | Must |
| F-SUC-03 | Rareté (pourcentage mondial) quand la source la fournit | Could |
| F-SUC-04 | Jeux sans succès disponibles : mention « Aucun succès » | Must |
| F-SUC-05 | Succès cachés masqués jusqu'au déblocage (option pour les révéler) | Could |

### 3.8 Métadonnées et visuels
| Réf. | Exigence | Priorité |
|---|---|---|
| F-MET-01 | Récupération automatique de la jaquette, de la bannière et du logo (SteamGridDB, données Steam, IGDB) | Must |
| F-MET-02 | Récupération de la description, du genre, du développeur et de la date de sortie (IGDB) | Should |
| F-MET-03 | Les jeux locaux sont associés à leur fiche par recherche du nom, avec confirmation par l'utilisateur | Should |
| F-MET-04 | L'utilisateur peut remplacer une jaquette ou renommer un jeu | Should |
| F-MET-05 | Les images sont mises en cache localement | Must |

### 3.9 Navigation, recherche et organisation
| Réf. | Exigence | Priorité |
|---|---|---|
| F-NAV-01 | Vue grille (jaquettes) et vue liste | Must |
| F-NAV-02 | Recherche instantanée par nom | Must |
| F-NAV-03 | Filtres : source, installé / non installé, disque, genre | Must |
| F-NAV-04 | Tris : nom, dernier lancement, temps de jeu, date d'ajout, progression des succès | Must |
| F-NAV-05 | Favoris | Should |
| F-NAV-06 | Collections personnalisées (ex. « Coop », « À finir ») | Could |
| F-NAV-07 | Masquer un jeu (logiciels détectés à tort, démos…) | Should |

### 3.10 Comptes et paramètres
| Réf. | Exigence | Priorité |
|---|---|---|
| F-PAR-01 | Connexion des comptes en ligne : clé API Steam et SteamID, compte Epic, etc. | Must |
| F-PAR-02 | Activation ou désactivation de chaque source | Must |
| F-PAR-03 | Gestion des dossiers surveillés | Must |
| F-PAR-04 | Démarrage avec Windows (optionnel) | Could |
| F-PAR-05 | Thème clair / sombre | Could |
| F-PAR-06 | Export / sauvegarde de la base (heures, collections) | Should |

---

## 4. Spécifications par source

> Chemins par défaut sous Windows. Chaque module doit aussi lire les chemins personnalisés (registre ou fichiers de configuration) au lieu de supposer l'emplacement par défaut.

### 4.1 Steam
- **Installation de Steam** : registre `HKCU\Software\Valve\Steam\SteamPath`
- **Bibliothèques (multi-disques)** : `<Steam>\steamapps\libraryfolders.vdf` (format VDF/KeyValues) liste tous les dossiers de bibliothèque, disques externes compris
- **Jeux installés** : `<bibliothèque>\steamapps\appmanifest_<appid>.acf` (nom, `installdir`, état)
- **Exclusions** : outils non-jeux (Steamworks Common Redistributables, Proton, SteamVR…), à filtrer par liste d'appid
- **Lancement** : `steam://rungameid/<appid>` · Installation : `steam://install/<appid>`
- **API Steam Web** (clé gratuite + SteamID64, profil et « Détails des jeux » en public) :
  - `IPlayerService/GetOwnedGames` → jeux possédés, `playtime_forever` (minutes), dernier lancement
  - `ISteamUserStats/GetPlayerAchievements` → succès du joueur
  - `ISteamUserStats/GetSchemaForGame` → noms, descriptions, icônes des succès
  - `ISteamUserStats/GetGlobalAchievementPercentagesForApp` → rareté
- **Visuels** : CDN Steam (`library_600x900.jpg`, `header.jpg`, etc.)

### 4.2 Epic Games
- **Jeux installés** : `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests\*.item` (JSON : `DisplayName`, `InstallLocation`, `LaunchExecutable`, `AppName`, `CatalogNamespace`, `CatalogItemId`)
- **Lancement** : `com.epicgames.launcher://apps/<Namespace>%3A<CatalogItemId>%3A<AppName>?action=launch&silent=true`
- **Bibliothèque complète, heures, succès** : pas d'API publique officielle. Possible via l'API non officielle utilisée par Legendary/Heroic (connexion OAuth au compte Epic). À isoler dans un module optionnel, car elle peut changer sans préavis.
- **Repli** : suivi maison des heures

### 4.3 GOG Galaxy
- **Base locale** : `C:\ProgramData\GOG.com\Galaxy\storage\galaxy-2.0.db` (SQLite) : jeux possédés, installés, temps de jeu
- **Jeux installés (sans Galaxy)** : registre `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<id>`
- **Lancement** : `GalaxyClient.exe /command=runGame /gameId=<id>`, ou l'exécutable direct
- **Succès** : via l'API GOG (connexion au compte), en P2

### 4.4 Battle.net
- **Jeux installés** : `C:\ProgramData\Battle.net\Agent\product.db` (protobuf : code produit, chemin d'installation) ; registre de désinstallation en complément
- **Lancement** : `Battle.net.exe --exec="launch <CODE>"` (ex. `WoW`, `Pro` pour Overwatch, `OSI` pour Diablo IV, à compléter par une table de correspondance)
- **Heures et succès** : pas d'API globale → suivi maison, « Aucun succès »

### 4.5 Riot Games
- **Installations** : `C:\ProgramData\Riot Games\RiotClientInstalls.json` (chemin du Riot Client)
- **Jeux** : `C:\ProgramData\Riot Games\Metadata\<produit>.<live>\<produit>.<live>.product_settings.yaml` (`product_install_full_path`)
- **Lancement** : `RiotClientServices.exe --launch-product=<league_of_legends|valorant|bacon> --launch-patchline=live`
- **Heures et succès** : pas d'API de temps de jeu → suivi maison (processus du jeu, pas du client Riot), « Aucun succès »

### 4.6 Ubisoft Connect
- **Jeux installés** : registre `HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\<id>\InstallDir`
- **Lancement** : `uplay://launch/<id>/0`
- **Heures et succès** : suivi maison (P3 : explorer l'API non officielle)

### 4.7 EA app
- **Jeux installés** : registre de désinstallation et manifestes dans `C:\ProgramData\EA Desktop\` (à confirmer pendant le développement)
- **Lancement** : `origin2://game/launch?offerIds=<id>` ou l'exécutable direct
- **Heures et succès** : suivi maison

### 4.8 Xbox / Game Pass PC
- **Jeux installés** : paquets UWP/MSIX (`Get-AppxPackage`), dossiers `XboxGames` sur chaque disque
- **Lancement** : `shell:AppsFolder\<PackageFamilyName>!<AppId>`
- **Heures et succès** : suivi maison (P3 : API Xbox Live via OpenXBL)

### 4.9 Jeux locaux
- **Ajout** : manuel, scan de dossiers surveillés, raccourcis `.lnk` (option)
- **Heuristique de scan** : exécutables de premier niveau, ou de 2e niveau dans les dossiers `bin`, `Binaries\Win64`, `x64` ; exclusion des installeurs et outils ; préférer l'exe le plus volumineux ou celui dont le nom ressemble à celui du dossier
- **Lancement** : exécutable direct, avec arguments et dossier de travail
- **Heures** : suivi maison · **Succès** : « Aucun succès »

---

## 5. Modèle de données

### 5.1 Entités principales

**Game** (un jeu, toutes sources confondues)
| Champ | Type | Description |
|---|---|---|
| id | UUID | Identifiant interne |
| title | texte | Nom affiché |
| sortTitle | texte | Nom utilisé pour le tri |
| igdbId / steamGridId | entier, nullable | Liens vers les métadonnées |
| coverPath, heroPath, logoPath | texte | Images en cache |
| description, genres, developer, releaseDate | divers | Métadonnées |
| favorite, hidden | booléen | Préférences |
| addedAt | date | Date d'ajout |

**GameSource** (un jeu sur une source donnée ; un même jeu peut en avoir plusieurs, par ex. Steam et Epic)
| Champ | Type | Description |
|---|---|---|
| id | UUID | |
| gameId | → Game | |
| source | enum | `steam`, `epic`, `gog`, `battlenet`, `riot`, `ubisoft`, `ea`, `xbox`, `local` |
| externalId | texte | appid Steam, AppName Epic, code Battle.net… |
| installed | booléen | |
| installPath | texte, nullable | |
| volumeSerial | texte, nullable | Identifiant du disque (cf. F-EXT-02) |
| executablePath | texte, nullable | |
| launchMethod | enum | `uri`, `launcher_cli`, `exe` |
| launchTarget | texte | URI, commande ou chemin |
| launchArgs, workingDir | texte, nullable | |
| sourcePlaytimeMinutes | entier, nullable | Temps fourni par la source |
| lastPlayedSource | date, nullable | |
| available | booléen | Faux si le disque est débranché |

**PlaySession** (suivi maison)
| Champ | Type |
|---|---|
| id, gameSourceId | |
| startedAt, endedAt | date |
| durationSeconds | entier |

**Achievement**
| Champ | Type |
|---|---|
| id, gameSourceId | |
| apiName, name, description | texte |
| iconUnlocked, iconLocked | texte |
| hidden | booléen |
| unlocked | booléen |
| unlockedAt | date, nullable |
| globalPercent | réel, nullable |

**Autres** : `Account` (identifiants par source, chiffrés), `WatchedFolder` (dossiers surveillés), `Collection` et `CollectionGame`, `Settings`.

### 5.2 Règles
- Temps affiché = `sourcePlaytimeMinutes` si disponible, sinon somme des `PlaySession`
- Aucun `Achievement` pour un jeu → afficher « Aucun succès »
- Dédoublonnage : même `installPath` → une seule entrée ; même titre normalisé sur deux sources → proposer de fusionner

---

## 6. Exigences non fonctionnelles

### 6.1 Performance
- Démarrage de l'application en moins de 2 s (bibliothèque affichée depuis le cache)
- Détection complète en arrière-plan en moins de 10 s pour environ 500 jeux (hors scan profond des dossiers surveillés)
- Interface fluide (60 i/s) avec plus de 1 000 jeux : grille virtualisée
- Consommation au repos inférieure à 150 Mo de RAM et proche de 0 % de CPU
- Surveillance des processus légère (événements système ou interrogation toutes les ~5 s au maximum)

### 6.2 Sécurité et confidentialité
- Clés API et jetons stockés dans le coffre de Windows (Credential Manager / DPAPI), jamais en clair
- Aucune donnée envoyée à un serveur tiers en dehors des API des services eux-mêmes et des services de métadonnées
- **Lecture seule** sur les fichiers des launchers : l'application ne modifie jamais leurs fichiers de configuration
- Pas d'élévation de privilèges (pas d'exécution en administrateur)

### 6.3 Fiabilité
- Chaque module de source est isolé : une erreur dans un module (format changé, API indisponible) n'empêche pas les autres de fonctionner
- Fonctionnement hors ligne : la bibliothèque, le lancement et le suivi des heures marchent sans Internet ; la synchronisation en ligne reprend au retour de la connexion
- Les sessions de jeu sont enregistrées même si l'application plante (écriture au début de la session, puis mise à jour régulière)
- Journaux (logs) consultables pour le diagnostic

### 6.4 Ergonomie
- Interface en français (internationalisation prévue)
- Assistant de premier lancement : détection des launchers, connexion Steam, ajout de dossiers
- Utilisable au clavier ; navigation à la manette en option (mode « Big Picture », V3)

### 6.5 Maintenabilité
- Architecture en modules (« providers ») : ajouter une source ne doit pas toucher au reste du code
- Tests unitaires sur les parseurs (VDF, ACF, JSON Epic, YAML Riot, protobuf Battle.net) avec des fichiers d'exemple
- Code versionné sur Git

### 6.6 Compatibilité
- Windows 10 (22H2) et Windows 11, x64

---

## 7. Architecture technique

### 7.1 Stack recommandée
| Couche | Choix recommandé | Alternatives |
|---|---|---|
| Application de bureau | **Tauri 2** (backend Rust) | Electron (Node.js), C# / .NET 8 avec WPF ou Avalonia |
| Interface | **React + TypeScript + Vite** | Svelte, Vue |
| Style | Tailwind CSS | CSS Modules |
| Base de données | **SQLite** (via `sqlx` ou `rusqlite`) | – |
| Parsing | `keyvalues-parser` (VDF), `serde_json`, `serde_yaml`, `prost` (protobuf), `winreg` (registre) | |
| Processus | `sysinfo` + API Windows (WMI / ETW pour les événements de processus) | |
| Métadonnées | API SteamGridDB, API IGDB (via Twitch), CDN Steam | |

Pourquoi Tauri : l'application est légère (quelques Mo, peu de RAM), Rust gère bien l'accès système (registre, processus, fichiers), et l'interface web permet un design moderne.

### 7.2 Découpage

```
┌──────────────────────────── Interface (React) ────────────────────────────┐
│  Bibliothèque · Fiche jeu · Statistiques · Paramètres · Assistant         │
└──────────────────────────────────┬────────────────────────────────────────┘
                                   │ commandes / événements Tauri
┌──────────────────────────────────▼────────────────────────────────────────┐
│                          Cœur applicatif (Rust)                           │
│  LibraryService · LaunchService · PlaytimeTracker · MetadataService       │
│  DriveWatcher · SyncScheduler                                             │
└───────┬──────────────────────────┬──────────────────────────┬─────────────┘
        │                          │                          │
┌───────▼────────┐        ┌────────▼────────┐        ┌────────▼─────────┐
│   Providers    │        │  SQLite + cache │        │   Coffre Windows │
│ Steam · Epic · │        │     images      │        │   (identifiants) │
│ GOG · Bnet ·   │        └─────────────────┘        └──────────────────┘
│ Riot · Ubi ·   │
│ EA · Xbox ·    │
│ Local          │
└────────────────┘
```

### 7.3 Interface commune des providers
Chaque source implémente la même interface :

```rust
trait GameProvider {
    fn id(&self) -> SourceId;
    fn is_available(&self) -> bool;                      // launcher installé ?
    fn scan_installed(&self) -> Result<Vec<DetectedGame>>;
    fn fetch_owned(&self) -> Result<Vec<OwnedGame>>;     // vide si non supporté
    fn fetch_playtime(&self, g: &GameSource) -> Result<Option<Minutes>>;
    fn fetch_achievements(&self, g: &GameSource) -> Result<Option<Vec<Achievement>>>;
    fn launch(&self, g: &GameSource) -> Result<()>;
    fn capabilities(&self) -> Capabilities;              // ce que la source sait faire
}
```

### 7.4 Suivi des heures (PlaytimeTracker)
1. Au lancement, on enregistre une session « en cours »
2. On surveille les processus dont l'exécutable se trouve dans `installPath` (pas seulement le processus lancé, à cause des launchers intermédiaires et des URI)
3. Délai de grâce de 30 à 60 s au démarrage (le jeu peut mettre du temps à apparaître) et de 10 s à la fin (changement de processus)
4. On met à jour `endedAt` toutes les minutes (en cas de plantage, on perd au plus une minute)
5. La session est close quand aucun processus du dossier ne tourne plus

---

## 8. Interface utilisateur

### 8.1 Écrans
1. **Assistant de premier lancement** : launchers détectés, connexion du compte Steam (clé API et SteamID), ajout de dossiers surveillés, premier scan
2. **Bibliothèque** (écran principal)
   - Barre latérale : Tous les jeux, Installés, Favoris, Récents, une entrée par source, Collections
   - Barre du haut : recherche, filtres, tri, bascule grille/liste, bouton Actualiser
   - Grille de jaquettes avec badge de source, pastille « installé » et grisé si le disque est débranché
3. **Fiche jeu**
   - Bannière, logo, bouton **Jouer** (ou **Installer** / **Indisponible**)
   - Temps de jeu total, dernière session, progression des succès
   - Onglets : Succès, Sessions (historique), Infos (description, chemin, source, disque), Paramètres du jeu (arguments, exe, jaquette)
4. **Statistiques** : temps total, temps par source, top 10, graphique hebdomadaire et mensuel, succès débloqués récemment
5. **Paramètres** : comptes, sources actives, dossiers surveillés, apparence, démarrage, sauvegarde, journaux
6. **Ajout manuel** : sélection de l'exe, nom, recherche de la jaquette, arguments
7. **Résultats du scan** : liste des jeux candidats, à cocher pour validation

### 8.2 Principes
- Thème sombre par défaut, centré sur les jaquettes
- Retour visuel immédiat (squelettes de chargement, notifications)
- Les jeux non installés sont visuellement distincts (opacité réduite), et les jeux sur un disque débranché sont grisés avec une icône de disque

---

## 9. Planning par versions

| Version | Contenu | Estimation (solo, temps libre) |
|---|---|---|
| **V0.1 – Socle** | Projet Tauri + React, base SQLite, écran Bibliothèque simple, provider Steam (installés et lancement), lecture de `libraryfolders.vdf` (multi-disques) | 2 à 3 semaines |
| **V0.2 – Local et heures** | Provider Local (ajout manuel, dossiers surveillés, filtrage), PlaytimeTracker, fiche jeu | 2 à 3 semaines |
| **V0.3 – Steam en ligne** | Clé API Steam : jeux non installés, heures, succès, rareté ; jaquettes via SteamGridDB | 2 semaines |
| **V0.4 – Autres launchers** | Epic (installés et lancement), Riot, Battle.net | 2 à 3 semaines |
| **V1.0 – Version complète** | Recherche, filtres et tris complets, favoris, statistiques, gestion des disques débranchés, assistant, paramètres, sauvegarde | 3 semaines |
| **V1.x** | GOG, Ubisoft, EA, Xbox ; Epic en ligne (API non officielle) | au fil de l'eau |
| **V2+** | Collections, mode manette « Big Picture », détection au branchement d'un disque, émulateurs, synchronisation | à définir |

---

## 10. Risques et contraintes

| Risque | Impact | Mitigation |
|---|---|---|
| Changement du format des fichiers d'un launcher (mise à jour) | Le module concerné ne détecte plus rien | Modules isolés, tests avec fichiers d'exemple, messages d'erreur explicites, mise à jour rapide du parseur |
| API non officielles (Epic, Ubisoft, Xbox) qui changent ou bloquent l'accès | Perte des heures et succès pour ces sources | Fonctions optionnelles, repli systématique sur le suivi maison |
| Conditions d'utilisation des services | Usage des API non officielles potentiellement contraire aux CGU | Usage personnel ; lire les CGU avant toute diffusion publique ; privilégier les API officielles |
| Profil Steam privé | Pas de heures ni de succès via l'API | Le signaler dans l'assistant : rendre public le profil et les « Détails des jeux » |
| Suivi d'heures inexact (jeux avec anti-cheat, launchers intermédiaires, processus multiples) | Temps faux | Surveillance par dossier, délais de grâce, correction manuelle |
| Jeux lancés en dehors de l'application | Pas de suivi maison pour ces sessions | Option : surveillance permanente en arrière-plan de tous les jeux connus |
| Faux positifs du scan local | Logiciels non-jeux dans la bibliothèque | Liste d'exclusion, validation manuelle, fonction Masquer |
| Antivirus qui signale la surveillance des processus | Blocage de l'application | Pas d'injection ni de hooks, simple lecture de la liste des processus ; signature du code si diffusion |
| Limites de requêtes des API (Steam, IGDB) | Synchronisation lente | Cache, synchronisation incrémentale, file d'attente avec délais |

**Contraintes** : Windows uniquement ; aucune modification des fichiers des launchers ; les launchers d'origine restent nécessaires pour installer et mettre à jour les jeux, ainsi que pour lancer ceux qui l'exigent (anti-cheat, DRM).

---

## 11. Critères d'acceptation (V1.0)

- [ ] Sur un PC équipé de Steam, Epic, Riot et Battle.net, **100 % des jeux installés** apparaissent après le premier scan, y compris ceux d'un disque externe branché
- [ ] Chaque jeu installé se lance depuis l'application en un clic
- [ ] Un jeu ajouté à la main (exe) se lance et son temps de jeu est enregistré à la minute près
- [ ] Le scan d'un dossier surveillé propose les jeux sans installeurs ni outils parmi les candidats
- [ ] Avec une clé API Steam valide, les jeux Steam non installés apparaissent avec leurs heures et leurs succès, identiques à ceux affichés par Steam
- [ ] Les jeux sans succès affichent « Aucun succès »
- [ ] Débrancher le disque externe grise ses jeux sans les supprimer ; le rebrancher les rend à nouveau jouables, même si la lettre du lecteur a changé
- [ ] Sans Internet : la bibliothèque s'affiche, les jeux se lancent et le temps de jeu est suivi
- [ ] Un module en erreur (ex. fichier Epic corrompu) n'empêche pas l'affichage des autres sources
- [ ] Démarrage en moins de 2 s et interface fluide avec 500 jeux
- [ ] Aucune clé ni aucun jeton stocké en clair sur le disque
