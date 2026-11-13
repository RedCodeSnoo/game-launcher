# Avancement

| Version | État | Tag |
|---|---|---|
| V0.1 – Socle | ✅ terminé | `v0.1.0` |
| V0.2 – Local et heures | ✅ terminé | `v0.2.0` |
| V0.3 – Steam en ligne | ✅ terminé (API à vérifier avec une vraie clé) | `v0.3.0` |
| V0.4 – Autres launchers | ✅ terminé | `v0.4.0` |
| V1.0 – Version complète | ✅ terminé | `v1.0.0` |
| V1.x – GOG, Ubisoft, EA, Xbox | ✅ terminé (Xbox vérifié en réel ; GOG, Ubisoft, EA testés sur données d'exemple seulement) | `v1.1.0` |

## V0.1 – Socle

- Projet Tauri 2 + React + TypeScript + Vite + Tailwind, dépôt Git.
- Base SQLite avec le schéma complet du §5 (migrations versionnées).
- Provider Steam : `libraryfolders.vdf` (multi-disques), manifestes `.acf`, exclusion des outils, lancement `steam://rungameid/<appid>`.
- Service de bibliothèque : intégration des scans, jeux désinstallés conservés, disques débranchés, changement de lettre de lecteur, dédoublonnage par chemin.
- Détection automatique au démarrage en arrière-plan, bouton « Actualiser », isolation des providers (erreur ou panique d'un module sans effet sur les autres).
- Interface : bibliothèque en grille et en liste virtualisées, recherche, barre latérale par source, notifications, journal dans un fichier.
- Vérifié sur cette machine : 19 jeux Steam sur 2 disques détectés en 82 ms (20 manifestes dont 1 outil exclu).
- Tests : 24 tests Rust, 13 tests front.

## V0.2 – Local et heures

- Provider Local : ajout manuel d'un `.exe` (sélecteur ou glisser-déposer), arguments et dossier de travail, dossiers surveillés, scan avec filtrage des installeurs et outils, validation candidat par candidat, candidats ignorés mémorisés.
- PlaytimeTracker : session écrite dès le lancement, surveillance des processus du dossier du jeu toutes les 5 s (uniquement pendant qu'un jeu est suivi), délais de grâce, écriture chaque minute, clôture des sessions orphelines au démarrage, indicateur « En cours ».
- Fiche jeu : bannière, Jouer / Installer / Indisponible, temps de jeu (source et suivi séparés), dernière session, onglets Succès (« Aucun succès »), Sessions, Infos, Paramètres du jeu (renommer, exécutable, arguments, correction manuelle du temps, masquer, retirer).
- Vérifié : test de bout en bout avec un vrai processus (`cargo test -- --ignored`) : jeu ajouté à la main, lancé, suivi, session close avec la bonne durée.
- Tests : 33 tests Rust (+1 de bout en bout), 13 tests front.

## V0.3 – Steam en ligne

- Compte Steam : clé API dans le coffre Windows (Credential Manager), SteamID64 (pré-rempli depuis `loginusers.vdf`, URL personnalisée résolue), vérification à l'enregistrement, messages clairs (profil privé, clé refusée).
- Jeux possédés non installés, heures et dernier lancement (`GetOwnedGames`), bouton « Installer » (`steam://install/<appid>`).
- Succès : progression du joueur, schéma (noms, descriptions, icônes), rareté mondiale ; succès cachés masqués avec option pour les révéler ; « Aucun succès » sinon. Synchronisation incrémentale (jeux joués depuis la dernière synchronisation) et à l'ouverture d'une fiche.
- Visuels : cache local du client Steam (fonctionne hors ligne) → CDN Steam → SteamGridDB (clé facultative) → image d'en-tête ; images copiées dans le cache de l'application. Jaquette personnalisée, association d'un jeu à une fiche par recherche du nom avec confirmation.
- Description, genres, développeur, date de sortie depuis la boutique Steam, en tâche de fond limitée en débit.
- Hors ligne : la synchronisation échoue en silence, la bibliothèque locale reste utilisable.
- Vérifié sur cette machine : 19 jeux Steam, bannières et logos pour tous, 18 jaquettes depuis le cache Steam sans réseau. Coffre Windows testé (écriture, lecture, suppression).
- Non vérifiable ici : appels authentifiés à l'API Steam (pas de clé) → `TODO.md`.
- Tests : 46 tests Rust (+1 de bout en bout), 13 tests front.

## V0.4 – Autres launchers

- Epic Games : manifestes `.item` (DLC, moteurs, greffons et installations incomplètes exclus), lancement par URI `com.epicgames.launcher://`, un manifeste corrompu n'empêche pas de lire les autres. En avance sur la V1.x : bibliothèque possédée, jaquettes, bannières et descriptions lues dans le cache local du catalogue du launcher (`catcache.bin`), sans compte ni API non officielle.
- Riot Games : `RiotClientInstalls.json` + `product_settings.yaml`, lancement `RiotClientServices.exe --launch-product=… --launch-patchline=…`.
- Battle.net : `product.db` (protobuf, schéma vérifié sur un fichier réel), table code produit → titre et code de lancement, repli sur la famille de produit et le nom du dossier pour les variantes inconnues, lancement `Battle.net.exe --exec="launch <CODE>"`.
- Heures : suivi maison ; succès : « Aucun succès ».
- Vérifié sur cette machine : Fortnite, League of Legends (live et PBE), Teamfight Tactics (live et PBE), VALORANT et World of Warcraft détectés avec leurs chemins et commandes ; 293 jeux Epic possédés ; détection complète des 4 launchers en 9 ms.
- Tests : 58 tests Rust (+1 de bout en bout), 13 tests front.

## V1.0 – Version complète

- Recherche instantanée (insensible aux accents), filtres (état, source, disque, genre), tris (nom, dernier lancement, temps de jeu, date d'ajout, progression des succès), préférences d'affichage mémorisées.
- Favoris, Récents, Masqués, collections personnalisées, proposition de fusion des doublons (même titre sur plusieurs sources).
- Statistiques : temps total, par source, top 10, histogramme par semaine et par mois, succès récents.
- Disques : surveillance des lecteurs (branchement/débranchement détecté en ~5 s), jeux grisés « Indisponible – disque non connecté », chemins retrouvés par numéro de série si la lettre change.
- Assistant de premier lancement : launchers détectés, compte Steam, dossiers surveillés.
- Paramètres : comptes, activation de chaque source, dossiers surveillés, thème clair/sombre, démarrage avec Windows, sauvegarde/restauration, journaux.
- Sauvegarde : export/restauration de la base, copie automatique quotidienne (7 conservées).
- Version finale : `npm run tauri build` produit l'exécutable et l'installeur NSIS.

### Critères d'acceptation (§11)

| Critère | État | Vérification |
|---|---|---|
| 100 % des jeux installés (Steam, Epic, Riot, Battle.net), disque externe compris | ✅ | Essai réel sur cette machine : 19 Steam (2 disques), 1 Epic, 5 Riot, 1 Battle.net, recoupés avec les fichiers des launchers. Multi-disques testé (`scans_libraries…`). Pas de disque externe branché ici : logique couverte par les tests de `library.rs`. |
| Chaque jeu installé se lance en un clic | ⚠️ à confirmer | Commandes construites et testées ; lancements réels non exécutés (D17) → `TODO.md`. |
| Jeu ajouté à la main : lancement + temps à la minute près | ✅ | Test de bout en bout avec un vrai processus (`tracks_a_real_process_end_to_end`). |
| Scan d'un dossier surveillé sans installeurs ni outils | ✅ | `scan_proposes_games_without_installers`, `excludes_installers_and_tools`. |
| Clé API Steam : non installés, heures et succès identiques à Steam | ⚠️ à confirmer | Analyseurs testés sur réponses d'exemple ; pas de clé pour un appel réel → `TODO.md`. |
| « Aucun succès » | ✅ | Interface (fiche jeu, onglet Succès) + `achievements_are_stored_counted_and_replaced`. |
| Disque débranché : grisé, non supprimé ; rebranché sous une autre lettre : jouable | ✅ | `unplugged_drive_greys_games_without_removing`, `local_exe_follows_drive_letter_change`, `letter_reused_by_another_disk_is_not_confused`. |
| Sans Internet : bibliothèque, lancement, suivi | ✅ | Aucune de ces fonctions n'utilise le réseau ; les erreurs réseau sont classées `OFFLINE` et ignorées ; visuels Steam lus dans le cache local. |
| Un module en erreur n'empêche pas les autres | ✅ | Isolation par `catch_unwind` dans `sync.rs` ; `corrupted_file_does_not_hide_other_games`, `scans_libraries_and_survives_corrupted_manifest`. |
| Démarrage < 2 s, fluide avec 500 jeux | ✅ | Détection des 4 launchers : 9 ms ; 1 000 jeux intégrés et relus en moins de 5 s (`handles_a_thousand_games_quickly`) ; grille virtualisée (27 cartes dans le DOM pour 1 000 jeux). Mesures de la version finale ci-dessous. |
| Aucune clé ni jeton en clair sur le disque | ✅ | Coffre Windows uniquement (`roundtrip_through_windows_vault`, `backup_never_contains_api_keys`), URL jamais journalisées. |

Mesures de la version finale sur cette machine : installeur de 3,2 Mo, détection des 4 launchers en 7 ms au démarrage, processus principal à ~43 Mo de mémoire.

## V1.x

- GOG : jeux installés par le registre (DLC exclus), lancement par l'exécutable ; jeux possédés, heures et dernier lancement lus dans la base locale de GOG Galaxy (lecture seule).
- Ubisoft Connect : registre `Launcher\Installs`, nom depuis la clé de désinstallation, lancement `uplay://launch/<id>/0`.
- EA app : registre `EA Games`, `installerdata.xml` (identifiant, titre, exécutable), lancement par l'exécutable ou `origin2://`.
- Xbox / Game Pass : dossiers `XboxGames` de chaque disque (ou dossier indiqué par `.GamingRoot`), `MicrosoftGame.config`, lancement `shell:AppsFolder\<famille>!<AppId>`.
- Epic « en ligne » : couvert sans API non officielle par le cache local du catalogue (jeux possédés et visuels, livré en V0.4). Heures et succès Epic : non faits (voir `TODO.md`).
- Option « réduire la fenêtre au lancement d'un jeu ».
- Vérifié sur cette machine : Xbox détecte Minecraft Launcher (`C:\XboxGames`) avec sa commande `shell:AppsFolder\Microsoft.4297127D64EC6_8wekyb3d8bbwe!Game`. GOG, Ubisoft et EA ne sont pas installés ici : testés sur données d'exemple uniquement.
- Version finale 1.1.0 : installeur de 3,2 Mo ; au repos, processus principal à ~40 Mo et ~0,2 % d'un cœur ; détection des 5 launchers présents en 23 ms.
- Tests : 76 tests Rust (+1 de bout en bout), 13 tests front.
