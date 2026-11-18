# Décisions

Décisions prises en autonomie pendant le développement, avec leur justification.

## D0 – L'Annexe A est absente du cahier des charges

La mission renvoie à une « Annexe A » (règles A.2, A.3, A.5) mais `cahier-des-charges-launcher.md` s'arrête au §11. Aucune question ne pouvant être posée, les règles suivantes la remplacent, déduites de la mission elle-même :

- **Tests (≈ A.2)** : chaque version ajoute des tests unitaires sur ce qu'elle introduit (parseurs avec fichiers d'exemple, logique de bibliothèque sur SQLite en mémoire, logique d'interface pure). `cargo test`, `tsc`, `vitest` et `vite build` doivent être verts avant le commit.
- **Commits et tags (≈ A.3)** : au moins un commit par version du §9, tag `v0.1.0`, `v0.2.0`, `v0.3.0`, `v0.4.0`, `v1.0.0`, puis `v1.1.0`… pour la V1.x. `PROGRESS.md` est mis à jour à chaque version.
- **Définition de « terminé » (≈ A.5)** : versions V0.1 à V1.0 livrées et taguées ; chaque critère du §11 soit vérifié (test automatique ou essai réel sur cette machine), soit explicitement listé dans `TODO.md` avec la raison ; tests verts ; l'application se compile en version finale (`npm run tauri build`) ; V1.x avancée autant que possible ; ce qui n'est pas fait ou pas vérifiable est dans `TODO.md`.

## D1 – Stack

Stack recommandée du §7.1 : Tauri 2, React 19 + TypeScript + Vite, Tailwind CSS 4, SQLite via `rusqlite` (SQLite embarqué, pas de dépendance système).

## D2 – Parseur VDF écrit à la main

Le format KeyValues est simple ; un parseur maison de ~150 lignes (`vdf.rs`) est entièrement testé et évite une dépendance dont l'API varie. `keyvalues-parser` n'était qu'une suggestion.

## D3 – Erreurs = code + détail

Le backend renvoie `{ code, detail }` ; l'interface traduit le code (`i18n.ts`). Cela garde le backend indépendant de la langue (internationalisation prévue, §6.4).

## D4 – Backend simulé dans le navigateur

Hors Tauri (`npm run dev` dans un navigateur), `src/mock.ts` simule le backend avec des jeux générés (`?games=1000`). Sert à vérifier l'interface et sa fluidité sans compiler le Rust. Il n'est chargé que hors Tauri (import dynamique).

## D5 – Disponibilité des disques

`volumeSerial` (numéro de série du volume) est enregistré pour chaque jeu installé. Un jeu non retrouvé par un scan devient « indisponible » si son volume n'est monté sous aucune lettre, sinon « non installé » (il reste dans l'historique). Si le volume réapparaît sous une autre lettre, les chemins sont réécrits. Seuls les lecteurs fixes et amovibles sont pris en compte (pas les lecteurs réseau ni optiques).

## D6 – Dédoublonnage

Un jeu local dont le dossier est ensuite détecté par un launcher est remplacé par la source du launcher (ses sessions sont conservées). Deux jeux de même titre normalisé sur deux sources ne sont pas fusionnés d'office : une fusion est proposée (§5.2).

## D7 – Outils Steam exclus

Liste d'appid (redistribuables, Proton, Steam Linux Runtime, SteamVR) complétée par des préfixes de nom. Les logiciels achetés sur Steam (ex. Wallpaper Engine, Blender) restent visibles : l'utilisateur peut les masquer (F-NAV-07).

## D8 – Suivi des heures

- Interrogation de la liste des processus toutes les 5 s, seulement pendant qu'un jeu lancé depuis l'application est suivi (le fil dort sinon : 0 % de CPU au repos). Pas d'ETW/WMI : plus simple, sans privilèges, et suffisant pour une précision de quelques secondes.
- Délai d'apparition : 60 s pour un exécutable direct, 180 s quand un launcher sert d'intermédiaire (mise à jour, connexion). Le cahier des charges indique 30 à 60 s ; 180 s évite de perdre la session quand Steam ou Riot met du temps à démarrer le jeu. Délai de fin : 10 s.
- La session démarre au clic pour un lancement direct, et à l'apparition du processus quand un launcher sert d'intermédiaire.
- Un jeu qui n'apparaît jamais : la session provisoire est supprimée.

## D9 – Dossier d'un jeu local

Le dossier du jeu sert au suivi (tout processus du dossier compte) et au dédoublonnage. Pour un exécutable posé dans un dossier générique (racine de disque, Bureau, Téléchargements, dossier surveillé lui-même), aucun dossier n'est retenu : seul l'exécutable exact est suivi.

## D10 – Retirer un jeu

Seuls les jeux locaux peuvent être retirés de la bibliothèque. Un jeu de launcher réapparaîtrait au scan suivant : il faut le masquer (F-NAV-07).

## D11 – Métadonnées sans IGDB

IGDB impose un compte développeur Twitch. Pour un usage personnel sans configuration, la fiche de la boutique Steam (API publique, sans clé) fournit description, genres, développeur et date de sortie. Un jeu non-Steam est associé automatiquement à une fiche si le titre normalisé est identique, sinon l'utilisateur choisit la fiche (F-MET-03). IGDB reste possible plus tard : le champ `igdbId` existe déjà.

## D12 – Ordre des sources d'images

Cache local du client Steam d'abord (aucun réseau, visuels exacts de la bibliothèque Steam), puis CDN Steam, puis SteamGridDB si une clé est fournie, puis image d'en-tête de la boutique. Les fichiers du client Steam sont seulement lus et copiés.

## D13 – Secrets

Les clés (API Steam, SteamGridDB) ne transitent que de l'interface vers le backend, qui les écrit dans le Gestionnaire d'identification Windows (`keyring`). Elles ne sont ni dans la base, ni dans les journaux (les messages d'erreur HTTP n'incluent jamais l'URL), ni renvoyées à l'interface (seulement « clé enregistrée : oui/non »). Le SteamID, public, est dans la table `settings`.

## D14 – Synchronisation des succès

Trois appels par jeu (progression, schéma, rareté). En tâche de fond, seuls les jeux installés ou déjà joués sont synchronisés, puis uniquement ceux rejoués depuis. Les autres le sont à l'ouverture de leur fiche. Un profil privé ou une clé refusée arrête la boucle avec un message unique.

## D15 – Epic : cache local plutôt qu'API non officielle

Le launcher Epic garde le catalogue du compte dans `Data/Catalog/catcache.bin` (JSON en base64). On y lit, en lecture seule, les jeux possédés et leurs visuels. Cela couvre « Epic non installés » sans connexion OAuth ni API non officielle (risques du §10). Limite : le cache n'est à jour que si le launcher Epic a été ouvert récemment ; il ne contient ni heures ni succès (suivi maison).

## D16 – Battle.net : codes de lancement

Le cahier des charges cite `OSI` pour Diablo IV ; d'après les codes connus, `OSI` est Diablo II: Resurrected et Diablo IV est `Fen`. La table de `battlenet.rs` suit ces codes. Pour un produit inconnu, on utilise le code de sa famille (`product_family`) ou, à défaut, son code en majuscules, et le nom de son dossier comme titre.

## D17 – Lancements réels non exécutés pendant le développement

Les commandes de lancement sont construites et testées, mais aucun jeu réel n'a été démarré sur cette machine pendant le développement (ouvrir Steam, Riot ou Battle.net et lancer des jeux sans personne devant l'écran serait intrusif). Le lancement d'un exécutable et son suivi sont couverts par un test de bout en bout avec un processus réel.

## D18 – Sources désactivées

Désactiver une source arrête sa détection et masque ses jeux, sans rien supprimer : la réactiver les fait réapparaître avec leurs heures.

## D19 – Sauvegarde

La sauvegarde est une copie de la base SQLite (`VACUUM INTO`). Les images n'y sont pas (elles se récupèrent à nouveau), ni les clés API (coffre Windows). La restauration refuse tout fichier qui n'est pas une base du launcher et est bloquée pendant qu'un jeu tourne.

## D20 – Statistiques datées

Les graphiques par semaine et par mois reposent sur les sessions suivies par l'application : les heures importées d'une source (Steam, GOG) sont un cumul sans date. Les totaux, le top 10 et le temps par source utilisent la règle du §5.2 (temps de la source s'il existe, sinon sessions).

## D21 – Collections dès la V1.0

Les collections étaient prévues en V2+ ; le schéma et le filtre existant déjà, elles ont été ajoutées en V1.0 (créer, renommer, supprimer, cocher depuis la fiche d'un jeu).

## D22 – Surveillance des disques

Interrogation de `GetLogicalDrives` et des numéros de série toutes les 5 s (coût négligeable) plutôt qu'un abonnement aux messages `WM_DEVICECHANGE`, qui demande une fenêtre dédiée.

## D23 – V1.x : sources locales uniquement

Pour GOG, Ubisoft, EA et Xbox, seules les sources locales et officielles sont utilisées (registre, fichiers de configuration, base de Galaxy en lecture seule). Les API non officielles (Epic, Ubisoft, OpenXBL) et l'API GOG demandent une connexion interactive de l'utilisateur et ne peuvent pas être validées en autonomie : elles restent dans `TODO.md`. Le repli prévu par le cahier des charges s'applique : suivi maison des heures, « Aucun succès ».

## D24 – Lancement GOG et EA par l'exécutable

GOG : l'exécutable direct fonctionne sans Galaxy (jeux sans DRM). EA : l'exécutable du jeu démarre lui-même l'EA app ; l'URI `origin2://` n'est qu'un repli, car `installerdata.xml` fournit un identifiant de contenu et non un identifiant d'offre.

## D25 – Réduction de la fenêtre

F-LAN-04 demande une réduction dans la zone de notification. La version livrée réduit la fenêtre dans la barre des tâches (aucune icône de zone de notification à gérer) ; la différence est notée dans `TODO.md`.

---

# Évolutions V1.1 (`evolutions-v1.1.md`)

## D26 – Version et tag

Le tag demandé est `v1.1`. Comme `v1.1.0` désignait déjà la livraison précédente (providers GOG, Ubisoft, EA, Xbox), l'application passe en version **1.1.1** et le tag `v1.1` pointe sur cette livraison.

## D27 – Cause des sessions restées « En cours » (E1-05)

Constat dans le code et sur la machine de développement : une session ne se fermait que lorsque **plus aucun processus ne tournait dans le dossier du jeu**. Tout processus annexe qui survit au jeu la maintenait donc ouverte : `crashpad_handler.exe`, `EpicWebHelper.exe` et `CheckEnv.exe` dans le dossier de Phantom Blade Zero, `LeagueClient*.exe` dans celui de League of Legends, `wallpaper32.exe` (permanent) dans celui de Wallpaper Engine. Deuxième cause : l'état du suivi n'existait qu'en mémoire, donc une session n'était jamais reprise après un redémarrage de l'application.

Correction de la cause, pas du symptôme : un processus n'appartient à un jeu que s'il est son exécutable principal, ou s'il est dans son dossier **et** absent de la liste d'exclusion (E1-07, `procs.rs`). Les processus exclus ne peuvent ni ouvrir ni maintenir une session. Les coquilles de processus terminés (aucun fil d'exécution) sont ignorées. Les processus « suspendus » ne sont pas distingués : aucun cas réel n'a été observé, et l'arrêt manuel (E1-01) couvre ce cas.

## D28 – Méthode de détection des processus (E4-03)

Interrogation périodique toutes les **3 s** par `CreateToolhelp32Snapshot` (un appel système, aucun droit particulier), puis `QueryFullProcessImageNameW` et `GetProcessTimes` avec le droit minimal `PROCESS_QUERY_LIMITED_INFORMATION`, **une seule fois par processus** à son apparition (cache par identifiant de processus).

- Préféré à `sysinfo` (retiré du projet) : `sysinfo` rouvre et relit chaque processus à chaque passage.
- Préféré aux événements WMI : ils demandent COM, peuvent perdre des événements sous charge, et ne dispensent pas d'un balayage initial (jeu déjà ouvert au démarrage, E4-08).
- Délai de détection : 3 s au plus (exigence : moins de 10 s).

## D29 – Règles de correspondance processus → jeu (E4)

1. Chemin égal à l'exécutable principal (personnalisé, sinon détecté) : c'est le jeu, même si le nom contient « launcher ».
2. Sinon, chemin dans le dossier d'installation (personnalisé, sinon détecté) et nom absent de la liste d'exclusion. En cas de dossiers imbriqués, le plus précis l'emporte.
3. Chemin illisible (jeu en administrateur, anti-triche) : comparaison par nom avec l'exécutable principal connu, complété par un tableau interne (`League of Legends.exe`, `VALORANT-Win64-Shipping.exe`, `Wow.exe`, `Overwatch.exe`, `Diablo IV.exe`…).

`LeagueClient*.exe` est dans la liste d'exclusion : le client de League of Legends n'est pas une partie en cours. Ajouts à la liste demandée : `*crashpad*`, `*WebHelper*`, `BlizzardError.exe`, `BlizzardBrowser.exe`, `WowVoiceProxy.exe`, sous-processus CEF/Qt, installeurs et redistribuables, `wallpaper32/64.exe`. L'utilisateur peut ajouter ses propres motifs (Paramètres > Suivi du temps de jeu) et exclure un jeu entier du suivi (fiche jeu).

## D30 – Règles des sessions (E4)

- Début = heure de création du processus (exacte, y compris pour un jeu déjà ouvert au démarrage), sans jamais chevaucher la session précédente du même jeu.
- Fin = dernière observation d'un processus du jeu, après un délai de grâce de 10 s (changement de processus).
- Le seuil de 60 s (réglable) s'applique à toutes les sessions, lancées depuis l'application ou détectées ; un arrêt manuel conserve toujours la session.
- Un jeu lancé depuis l'application est « En cours » dès le clic, mais sa session n'est créée qu'à l'apparition du processus : plus de session fantôme à supprimer si le jeu ne démarre pas.
- Un trou de plus de 5 min entre deux observations (mise en veille) clôt les sessions à leur dernière observation ; un jeu encore ouvert au réveil ouvre une nouvelle session.
- Au démarrage, une session restée « en cours » est **reprise** si son jeu tourne encore et que son dernier battement a moins de 5 min ; sinon elle est close à son dernier battement (E1-06).
- Si l'observation voit le processus quelques millisecondes avant que le clic sur « Jouer » soit enregistré, la session toute récente (moins de 15 s) est requalifiée « lancée depuis l'app ».
- « Arrêter le suivi » met le jeu en sourdine tant que ses processus tournent, sinon une nouvelle session s'ouvrirait 3 s plus tard.
- Avec « Suivre les jeux lancés en dehors de l'application » désactivé, le fil d'observation dort tant qu'aucun jeu lancé depuis l'application n'est suivi.

## D31 – « Forcer la fermeture » (E1-02)

Processus terminés : ceux du jeu, tout ce qui tourne depuis son dossier (y compris les processus annexes), et leurs descendants dont le chemin est illisible ou situé dans le dossier. Un descendant situé ailleurs (un navigateur ouvert par le jeu, par exemple) n'est **pas** terminé : fermer le navigateur de l'utilisateur serait pire que laisser un processus orphelin. Si un processus résiste, la session reste ouverte et l'interface propose « Arrêter le suivi » (E1-03).

## D32 – Fenêtre fermée = vue web déchargée (E3-03, E3-08)

La croix détruit la fenêtre au lieu de la masquer : les processus WebView2 s'arrêtent et il ne reste que le processus principal. La fenêtre est recréée à la demande (clic sur l'icône, « Ouvrir », seconde instance) ; l'état d'affichage (page ouverte) n'est pas conservé. L'option « Réduire au lancement d'un jeu » ferme désormais la fenêtre vers la zone de notification (F-LAN-04, auparavant simple réduction dans la barre des tâches).

## D33 – Démarrage avec Windows (E3-05, E3-06)

`tauri-plugin-autostart` écrit l'entrée de démarrage de l'utilisateur avec l'argument `--autostart`. « Démarrer réduit » ne s'applique qu'à ce démarrage-là : un lancement manuel ouvre toujours la fenêtre. Une entrée créée par la version précédente (sans l'argument) est réécrite au démarrage.

## D34 – Emplacement personnalisé (E2)

- Colonnes séparées `install_path_override`, `executable_path_override`, `launch_mode`, `direct_args`, `direct_working_dir` : la détection n'y écrit jamais.
- Les arguments du mode « exécutable direct » sont distincts de ceux du launcher (Riot et Battle.net en exigent), pour qu'un retour au launcher d'origine fonctionne sans rien ressaisir.
- « Exécutable direct » n'est proposé que si un exécutable est connu ; retirer l'exécutable personnalisé d'un jeu sans exécutable détecté repasse au launcher d'origine.
- Jeu local : l'exécutable d'origine reste la valeur « détectée », l'exécutable choisi ensuite est la valeur personnalisée.
- Les valeurs personnalisées situées sur un disque externe suivent son changement de lettre.

## D35 – Total = temps de la source + sessions depuis la dernière synchronisation

Demande de l'utilisateur (13 novembre 2026) : après l'import des heures Steam, le total doit augmenter quand il joue, sans attendre une synchronisation ; et une synchronisation doit ramener le total au chiffre de Steam en gardant l'historique des sessions.

- La date de lecture du temps chez la source est enregistrée (`playtime_synced_at`).
- Total affiché = temps de la source + durée des sessions jouées **après** cette date (une session à cheval ne compte que pour sa partie postérieure).
- Une synchronisation remplace le temps de la source et avance la date : les sessions déjà comptées par la source sortent du calcul. Rien n'est compté deux fois, aucune session n'est supprimée.
- Pour les temps importés avant cette version, la date retenue est celle de la migration : les anciennes sessions ne sont pas ajoutées au total.
- Limite connue : si la source n'a pas encore pris en compte une partie au moment de la synchronisation, le total peut reculer de la durée de cette partie jusqu'à la synchronisation suivante.
- Sans temps fourni par la source (Riot, Battle.net, Epic, jeux locaux), le total reste la somme des sessions. Une correction manuelle prime toujours.

## D36 – Correction manuelle = base datée (bug « le compteur ne bouge plus »)

Cause : le total affiché était `manual.unwrap_or(calculé)` ; une saisie manuelle remplaçait donc le total et les sessions suivantes ne s'y ajoutaient jamais.

Correction : la saisie est une base (`manual_baseline_minutes`) datée (`manual_baseline_at`). Total = base + sessions (lancées ou détectées) qui commencent après la saisie. Une nouvelle saisie remplace la base et repart de maintenant ; vider le champ revient au calcul automatique.

Migration (schéma v4) : la colonne est renommée, les valeurs sont conservées. Leur date de saisie n'était pas enregistrée : elle est retrouvée dans les sauvegardes automatiques quotidiennes, en prenant la **première sauvegarde qui contient déjà la valeur** (borne haute). Ce choix ne compte jamais une session qui aurait pu être incluse dans le chiffre saisi ; il peut au pire oublier une session jouée entre la saisie et la sauvegarde suivante. Sans sauvegarde correspondante, la date retenue est celle de la migration.

## D37 – Une seule fonction de calcul du temps total

Cause du bug « heures saisies absentes des stats par launcher » : les statistiques par launcher additionnaient la part de chaque source (sans la base manuelle) alors que le total général utilisait le temps du jeu.

Correction : `playtime::game_total_minutes` est la seule implémentation ; `GameView::playtime_minutes` en est le résultat et sert partout (fiche, cartes, tri, statistiques, top jeux, jeux récents de la zone de notification). Temps par launcher = somme des totaux des jeux, chaque jeu étant compté une fois pour sa source principale (la première de sa fiche : installée et disponible, celle qui est lancée). Les graphiques par semaine et par mois ne comptent que les sessions réelles (une base manuelle ou un temps importé n'a pas de date), ce qu'indique une note sous le graphique.

## D38 – Mises à jour automatiques côté Rust

`tauri-plugin-updater` est piloté depuis le Rust, pas depuis l'interface : fenêtre fermée, la vue web est déchargée (D32), or la vérification doit continuer en arrière-plan. Vérification 30 s après le démarrage puis toutes les 6 h ; option « Vérifier automatiquement » (activée par défaut). Une version trouvée déclenche une notification Windows (une fois par version) et une bannière. « Installer et redémarrer » attend la fin de la partie en cours avant de télécharger ; un dernier battement des sessions est écrit juste avant que l'installeur ferme l'application. Sans connexion, l'échec est seulement journalisé. `tauri-plugin-process` est enregistré ; le redémarrage utilise `AppHandle::restart` (sous Windows, l'installeur relance l'application lui-même).

Nom du dépôt : `game-launcher` (kebab-case de GameLauncher). Adresse : `https://github.com/RedCodeSnoo/game-launcher/releases/latest/download/latest.json`.

## D39 – Clé de signature

Paire générée par `tauri signer generate` avec un mot de passe aléatoire. Clé privée et mot de passe hors du dépôt, dans `%USERPROFILE%\.tauri\` (`game-launcher.key`, `game-launcher.key.password.txt`) ; clé publique dans `tauri.conf.json`. Les deux secrets GitHub sont `TAURI_SIGNING_PRIVATE_KEY` (contenu de la clé) et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Le `.gitignore` exclut `*.key`, `.tauri/`, `.env`, les bases SQLite, journaux et sauvegardes. Sans cette clé, aucune mise à jour ne peut plus être publiée pour les versions déjà installées.

Compilation locale sans la clé : `npm run build:local` (désactive les artefacts de mise à jour).

## D40 – Une seule source pour le numéro de version

`package.json` fait foi. `tauri.conf.json` le lit directement (`"version": "../package.json"`) ; `scripts/sync-version.mjs` le recopie dans `Cargo.toml` et `Cargo.lock`, et vérifie la cohérence (`--check`, lancé par `npm run check` et par le workflow, qui compare aussi au tag). `npm version X.Y.Z` appelle le script, crée le commit « Version X.Y.Z » et le tag `vX.Y.Z`. Les notes de version viennent de `CHANGELOG.md`.

## D41 – Workflow de publication

`.github/workflows/release.yml`, déclenché par un tag `v*`, sur `windows-latest` : contrôle des versions, tests de l'interface, puis `tauri-apps/tauri-action` (compilation, signature, Release publiée avec l'installeur, sa signature et `latest.json`). Les tests Rust ne tournent pas en CI : plusieurs dépendent de la machine (coffre Windows, processus réels, disques) ; ils sont lancés localement avant chaque commit (`npm run check`).

GitHub ne déclenche aucun workflow quand plus de trois tags sont poussés en une fois : lors de la première publication (8 tags), le tag `v1.1.1` a dû être repoussé seul. Pour une nouvelle version, pousser un seul tag (`git push --follow-tags` après `npm version`).

---

# V1.2 → V2.0 (`evolutions-v1.2-v2.0.md`)

## D42 – Moteur de thèmes

Un thème est un fichier JSON (`src/themes/builtin/*.json`, générés par `scripts/make-themes.py`) validé par `src/themes/schema.ts` : palette (13 couleurs, dont un voile et le texte lisible dessus), polices, cartes (arrondi, bordure, verre, ombre, motif), fond (dégradé, scène, image perso), particules, icônes, sons, durée des transitions. Le thème est converti en variables CSS posées sur `<html>`, avec des attributs (`data-pattern`, `data-border`, `data-icons`…) qui activent les détails propres au thème dans `index.css`. Les couleurs, arrondis, polices et tailles de Tailwind sont branchés sur ces variables : plus aucune couleur en dur dans les composants (les couleurs des launchers sont des données, dans `i18n.ts`). Les vignettes de remplacement mêlent la teinte du jeu à l'accent du thème.

Changement de thème : `document.startViewTransition` (fondu enchaîné de 320 ms), sans redémarrage. Le dernier thème est gardé en cache local pour s'afficher dès le premier rendu, sans flash.

## D43 – Ressources visuelles

Polices : uniquement des polices OFL embarquées via Fontsource (Inter, Noto Sans JP, Noto Serif JP, Fraunces, Orbitron, Rajdhani, Press Start 2P, VT323), sous-ensemble latin seulement (l'application marche hors ligne, l'installeur reste léger). Scènes : SVG dessinés pour le projet (`src/themes/scenes.tsx`) : torii et branche de cerisier, pagode et lanternes, érable, sapins enneigés, vagues, soleil rayé et ville néon, paysage en pixels. Motifs (seigaiha, asanoha, grain de papier, scanlines, givre) en SVG ou dégradés CSS. Curseur pixel dessiné pour le projet. Aucune image, personnage ni marque protégés. Les sons sont synthétisés (Web Audio).

## D44 – Effets et performances

- Particules sur un `<canvas>` unique, 30 images/s au plus, résolution 1:1, nombre proportionnel à la surface de la fenêtre et plafonné à 70, halos pré-rendus en sprites.
- Les scènes animées n'animent que `transform` et `opacity` sur des calques HTML (vagues, grille néon, halos des lanternes) : composition GPU, pas de redessin du SVG.
- Pause automatique (boucle `requestAnimationFrame` arrêtée, animations CSS suspendues) quand la fenêtre est masquée, quand elle n'a pas le focus (« en arrière-plan ») et pendant une partie. « Réduire les animations » et le réglage « Afficher les animations » de Windows (`prefers-reduced-motion`) suppriment particules et animations.
- Mesures (Edge sans interface, 1600×900, temps du moteur de rendu sur 20 s, en % d'un cœur) : Sombre 0,2 %, OLED 0 %, Néon 0,3 % (11 % avant le passage aux calques composés), Sakura 2,7 %, Automne 2,8 %, Hiver 2,4 %, Rétro 3,1 %, Kyoto 4 à 5 %, Océan 5 %. Rapporté à une machine moyenne à 8 fils, le pire cas est sous 1 % : dans le budget de 2 %. Dans l'application réelle, fenêtre en arrière-plan (effets en pause) : 0,03 % de la machine.
- Un voile translucide (40 % du fond) sépare la scène du contenu ; les contrastes des 10 thèmes sont vérifiés par des tests (WCAG AA : 4,5:1 pour le texte, 3:1 pour l'accent et les états).

## D45 – Thème dynamique, saisons, thèmes perso

- Thème dynamique : la couleur dominante de la jaquette (histogramme pondéré par la saturation, gris ignorés) devient l'accent, en gardant la teinte et en ajustant la luminosité pour rester à 3:1 sur la surface du thème ; la jaquette floutée s'ajoute au fond. Lecture des pixels possible car le protocole d'images de Tauri renvoie l'en-tête CORS de la fenêtre.
- Saison (hémisphère nord) : mars-mai Sakura, juin-août Océan, septembre-novembre Automne, décembre-février Hiver. Choisir un thème à la main désactive l'option.
- Thèmes perso et importés : stockés dans le réglage `custom_themes` ; l'image de fond est intégrée au thème (data URL, 4 Mo au plus) pour qu'un export `.json` soit autonome. À l'import, tout est revalidé : couleurs hexadécimales, énumérations, dégradés sans `url()`, images PNG/JPEG/WebP uniquement.

## D46 – Correctif : jaquettes au premier lancement

La portée du protocole d'images (`$APPDATA/**`) est calculée par Tauri avant la création du dossier de données ; au tout premier lancement, elle ne correspondait donc à aucun fichier (réponse 403) et les jaquettes ne s'affichaient qu'au redémarrage suivant. Le dossier de données est désormais autorisé explicitement au démarrage (`asset_protocol_scope().allow_directory`), ce qui couvre aussi un dossier choisi par `GAMELAUNCHER_DATA_DIR`. Les erreurs de Tauri (niveau avertissement et plus) sont maintenant écrites dans le journal.

## D47 – Vérification visuelle

Chaque thème est capturé sur la bibliothèque, la fiche jeu, les statistiques et les paramètres (Edge sans interface, backend de démonstration), puis examiné en planches contact. Corrigé grâce à cela : le fond passait devant le contenu non positionné (ordre d'empilement), le soleil Néon traversait les tableaux, la barre d'outils Rétro passait sur deux lignes, les entrées de menu Rétro avaient l'ombre des boutons. Contrôles dans la vraie fenêtre : polices chargées, thème enregistré, thème dynamique sur de vraies jaquettes, mise en pause pendant une partie (un jeu détecté en cours a bien suspendu les effets).

## D48 – Jours joués et séries (V1.3)

- Jour de jeu = date locale décalée de l'heure de bascule (4 h par défaut, réglable de 0 à 12 h). Les débuts de journée sont calculés dans le fuseau de la machine avec les règles d'heure d'été (une journée peut durer 23 ou 25 h ; une bascule tombant dans le trou du passage à l'heure d'été est repoussée à l'heure suivante qui existe).
- Une session est répartie entre les jours qu'elle touche au prorata du temps réel passé de chaque côté (sa durée enregistrée est conservée au total).
- Seuil d'un jour joué : 15 min par défaut, tous jeux confondus. Seules les sessions comptent ; les bases saisies à la main et les temps importés n'ont pas de date.
- Série en cours : jours joués d'affilée jusqu'à aujourd'hui ; si aujourd'hui n'est pas encore joué, la série d'hier reste affichée « en danger » jusqu'à la bascule.
- Jokers (option, désactivée par défaut) : un joker tous les 7 jours joués d'affilée, 2 au plus ; un jour manqué est sauvé automatiquement mais ne compte pas comme joué ; les jokers sont perdus avec la série.
- Résumés journaliers : table `activity_days` (jour, jeu, secondes, sessions), reconstruite quand la signature des sessions change (nombre, somme des durées, dernière fin, jeux distincts, heure de bascule, décalage du fuseau). Sur 3 ans (4 380 sessions), la page Statistiques complète se calcule en 16 ms en release, la grille en 4 ms.

## D49 – Page Statistiques

- Périodes : 7 et 30 derniers jours de jeu, année civile en cours, tout. Pour « tout », le temps total, la répartition par launcher et le top jeux reprennent le total de la fiche jeu (bases manuelles et temps importés compris) ; pour les autres périodes, seules les sessions de la période comptent (c'est indiqué sous chaque carte).
- Filtre par jeu ou par launcher (source principale du jeu) : il s'applique à la grille, aux barres, aux répartitions, aux habitudes et aux records. Les séries restent globales.
- Genres : le temps d'un jeu est partagé à parts égales entre ses genres (« Non classé » sans genre).
- Records : plus longue session, plus grosse journée, jeu le plus joué en une semaine (lundi → dimanche), heure de fin la plus tardive comptée depuis l'heure de bascule (3 h 40 est plus tard que 23 h).
- Graphiques avec visx (scales, shapes, axes), couleurs et polices du thème par variables CSS ; grille et carte des habitudes en SVG et CSS.
- Badges calculés en Rust ; ceux obtenus sont mémorisés (`badges_earned`) et notifiés à la fin d'une session. Au premier calcul (mise à jour depuis 1.2), les badges déjà mérités sont enregistrés sans notification.
- Rappel du soir (option) : vérification toutes les 5 min à partir de l'heure choisie, une notification par jour au plus, jamais pendant une partie.
- Repoussé (« Could ») : objectifs hebdomadaires et limite quotidienne (S2-14), voir `TODO.md`.

## D50 – Organisation (V1.4)

- Statut : colonne `games.status` (`backlog`, `playing`, `paused`, `finished`, `completed`, `abandoned`, ou aucun), avec sa date de changement pour trier les colonnes du backlog ; valeurs vérifiées côté Rust. Note personnelle de 1 à 10 (dix pastilles, un second clic l'efface) et notes texte (20 000 caractères au plus, enregistrées automatiquement). Note de session (500 caractères) dans `play_sessions.note`. Migration v6.
- Accueil, page par défaut au démarrage. Le jeu de la bannière est le dernier joué encore installé ; l'illustration (« hero ») sert de fond, avec un voile du thème pour la lisibilité.
- Backlog : six colonnes, glisser-déposer HTML5 (type `application/x-game-id`, aussi utilisé pour déposer une jaquette sur une collection ou sur « Backlog » dans la barre latérale).
- Recherche rapide : score flou maison (début du titre, initiales « ffvii », début de mot, sous-chaîne, puis lettres dans l'ordre), petit bonus pour les jeux installés et favoris. Entrée lance un jeu installé, Maj+Entrée ouvre sa fiche. Les pages, les thèmes et la roulette sont dans la même liste.
- Roulette : tirage uniforme parmi les jeux installés et disponibles, filtres combinables ; la bande de jaquettes s'arrête en 3,2 s (sans animation si les animations sont réduites). Le filtre « durée courte » attend la durée de vie des jeux (V1.5).
- Journal de session : l'invitation apparaît dans la fenêtre à la fin d'une session conservée (événement `session-ended`), seulement si l'option est activée ; une notification Windows ne permettrait pas de saisir la note.
- Collections automatiques calculées dans l'interface : pas joué depuis 182 jours, 80 % des succès ou plus sans être à 100 %, ajouté depuis 30 jours.
- Bibliothèque : taille des jaquettes de 110 à 240 px (réglage `cover_size`), vues grille, liste et bandeaux (illustration du jeu en fond), toutes virtualisées.

## D51 – Intégrations (V1.5)

- **Jeux gratuits Epic** : endpoint public des promotions (`freeGamesPromotions`, locale fr, pays FR). Un jeu compte s'il a une offre à 100 % de réduction en cours ou à venir ; le lien utilise le `pageSlug` du catalogue. Vérification 1 min après le démarrage puis toutes les 6 h ; la liste est gardée en cache (hors ligne : dernière liste connue, offres terminées retirées). Notification (désactivée par défaut) uniquement pour les jeux devenus gratuits depuis la vérification précédente, jamais au premier passage.
- **Actualités Steam** : `ISteamNews/GetNewsForApp` (sans clé), cache d'une heure en mémoire ; le contenu est nettoyé (BBCode, HTML, liens d'images) et tronqué. Fil de l'accueil : les 8 jeux Steam joués le plus récemment, 3 actualités chacun, 12 au total. Désactivable.
- **Discord Rich Presence** : protocole IPC local de Discord écrit à la main (tube nommé, trames binaires), sans dépendance. Discord exige l'identifiant d'une « application » : l'utilisateur en crée une gratuitement et colle son identifiant (le nom de l'application est affiché par Discord, d'où « Joue à GameLauncher » s'il la nomme ainsi). Image : jaquette publique du CDN Steam pour les jeux Steam (une jaquette locale n'est pas accessible à Discord), sinon texte seul. Mise à jour toutes les 8 s sans réseau ; reconnexion au plus une fois par minute si Discord est fermé.
- **Captures d'écran** : Steam (`userdata/*/760/remote/<appid>/screenshots`, sans les miniatures), Xbox Game Bar (`Vidéos\Captures`, fichiers « <titre> <date> »), NVIDIA (`Vidéos\<titre>` et `Vidéos\NVIDIA\<titre>`), dossiers ajoutés par l'utilisateur. Correspondance stricte : le nom doit être le titre, suivi seulement d'une date ou d'un numéro (« Hades II 2026… » n'est pas une capture de « Hades »), accents ignorés. Les dossiers trouvés sont autorisés en lecture pour l'affichage (protocole d'images de Tauri), à la demande.
- **Durée de vie** : uniquement IGDB (`game_time_to_beats`, « normally » = histoire principale, « completely » = 100 %), source officielle de Twitch ; identifiant et secret d'application vérifiés puis rangés dans le coffre Windows ; recherche à l'ouverture d'une fiche, au plus une fois par mois et par jeu. Aucune source non officielle (HowLongToBeat) n'est utilisée.
- **Synchronisation** : copie cohérente de la base (`VACUUM INTO`, écrite à côté puis renommée) dans `<dossier>/GameLauncher/library.db` avec `sync.json` (date, nom du PC, nombre de jeux et de sessions). Automatique au plus toutes les 10 min si l'empreinte des données a changé. Restauration = import de cette copie (impossible pendant une partie). Les clés restent dans le coffre Windows. Pas de fusion entre PC : la restauration remplace la bibliothèque locale (dit clairement dans l'interface).

## D52 – Clignotement des thèmes (retour de l'utilisateur)

Cause : chaque survol d'une vignette appliquait le thème avec un fondu enchaîné (`startViewTransition`) sur toute la fenêtre, puis un second en sortant ; traverser la galerie enchaînait les fondus. L'accent du thème dynamique déclenchait aussi un fondu à l'ouverture d'une fiche. Correction : l'aperçu ne s'applique qu'après 250 ms d'arrêt sur une vignette, sans fondu ; en sortant de la galerie, retour immédiat sans fondu ; le fondu est réservé à un changement d'identifiant de thème qui ne vient pas d'un aperçu. Vérifié en comptant les fondus : 0 pour un survol rapide de 6 vignettes, 0 pour l'aperçu puis le clic, 1 pour un changement direct.

## D53 – Big Picture (V2.0)

- Composant plein écran au-dessus de l'application (pas une seconde fenêtre) : il reprend le fond animé du thème déjà en place et ses règles de pause. La fenêtre passe en plein écran à l'entrée et revient à la normale à la sortie (permissions `set-fullscreen` et `is-fullscreen`).
- Rangées : Continuer à jouer (16 derniers jeux installés joués), Favoris, À jouer (statuts À jouer et En cours), Tous les jeux installés. Seules la rangée active et la suivante sont affichées, la rangée active en clair : rien ne passe sous le titre du jeu.
- Manette : Gamepad API lue à chaque image (`requestAnimationFrame`) uniquement dans ce mode, fenêtre visible et **aucun jeu en cours** (la manette appartient alors au jeu, et rien ne doit tourner pendant une partie). Disposition standard : A/Croix = valider, B/Rond = retour, Y/Triangle = détails, croix directionnelle ou stick gauche (zone morte 0,5) ; répétition après 400 ms puis toutes les 120 ms. Logique pure dans `src/lib/gamepad.ts`, testée.
- Raccourci : `F11`, bouton Guide (bouton 16 de la disposition standard, transmis par Windows pour certaines manettes seulement), ou Vue + Menu ensemble (toujours disponible, d'où ce second raccourci). Hors Big Picture, la manette n'est lue que 4 fois par seconde, seulement si une manette est branchée et la fenêtre au premier plan.
- Sons : ceux du thème (jeu « doux » si le thème n'en a pas), activés par défaut dans ce mode ; réglage `bigpicture_sounds`.

## D54 – Récap de l'année (V2.0)

- Calcul en Rust (`recap.rs`) à partir des résumés journaliers de la V1.3 (mêmes règles de jour joué, de bascule et de fuseau) : temps total, jours joués, sessions, jeux, temps par mois, top 5, meilleure série de l'année, jour record (avec ses jeux), plus longue session, launcher préféré, badges.
- Disponible à partir du 1er décembre pour l'année en cours, toujours pour les années passées. Bannière d'accueil en décembre (année en cours) et en janvier (année précédente) ; bouton dans les Statistiques le reste de l'année (récap de l'année précédente).
- Profil de joueur : chaque trait reçoit un score = valeur / seuil (part du temps après 22 h ≥ 30 %, session moyenne ≥ 2,5 h, 15 jeux, série de 21 jours, 55 % le week-end, 60 % sur un seul jeu, 200 jours joués) ; le plus fort l'emporte, les deux suivants au-delà du seuil sont cités. Moins d'une heure de jeu, ou aucun trait à mi-seuil : « Joueur tranquille ».
- Écrans : 8 écrans de 7 s qui s'enchaînent (pause en maintenant le clic, flèches, Échap), animations en `transform`/`opacity`, supprimées si les animations sont réduites.
- Image : dessinée sur un canvas 1080 × 1350 (format portrait des réseaux sociaux) avec les couleurs et la police d'affichage du thème actif et les vraies jaquettes ; enregistrée par une commande Rust qui vérifie la signature PNG. Aucun visuel extérieur au projet.

## D55 – Profil (V2.0, « Could »)

- Avatar choisi parmi des emojis (aucune image importée, donc rien à stocker ni à protéger), pseudo libre de 32 caractères, jeu favori (par défaut le plus joué). Réglages `profile_name`, `profile_avatar`, `profile_favorite`, sauvegardés et synchronisés avec les autres réglages.
- Statistiques sur toute la période (page Statistiques « Tout »), profil de joueur de l'année en cours, export en image avec le même moteur que le récap.

## D56 – Correctif 2.0.1 : blocage de la base, poids des images

Symptômes signalés : performances désastreuses, bibliothèque vide après la mise à jour, images qui ne s'affichent pas ou mal à chaque changement d'onglet, plantage en cliquant sur la croix. Le journal Windows montrait des blocages (« AppHangB1 »), pas des plantages.

- **Cause principale** : `get_news_feed` (fil d'actualités de l'accueil, V1.5) appelait `list_visible_games(&state.conn(), &state.disabled_sources())`. Le verrou de l'argument était encore tenu quand `disabled_sources()` le redemandait : le fil se bloquait pour toujours en tenant la base. Toutes les commandes suivantes attendaient (bibliothèque vide), elles occupaient les fils d'exécution partagés avec le protocole des images (images figées), et la croix, qui lit un réglage sur le fil principal, figeait la fenêtre jusqu'à ce que Windows la ferme. Reproduit sur une copie des données réelles (0 jeu affiché, `get_games` sans réponse). Aucune donnée perdue : la base contenait tous les jeux et toutes les sessions.
- **Correction** : réglages lus avant de prendre le verrou ; `AppState::conn()` renvoie désormais un garde qui détecte une seconde demande sur le même fil et échoue (erreur journalisée) au lieu de bloquer. Toute la suite de tests tourne avec ce garde (aucun autre cas trouvé) ; tests dédiés.
- **Croix** : son traitement (lecture d'un réglage) quitte le fil principal ; la fenêtre ne peut plus attendre la base.
- **Images** : sous Windows, chaque image affichée transite par le fil principal (protocole `asset:` de WebView2). Le cache pesait 910 Mo pour ~420 jeux (jaquettes jusqu'à 4 Mo, illustrations 8 Mo, logos 14 Mo). Chaque image est ramenée à sa taille d'affichage (crate `image`, sans dépendance système) : jaquettes 600 × 900 et illustrations 1920 × 1080 au plus en JPEG qualité 85, logos 800 × 400 en PNG (transparence). Les images déjà légères sont gardées telles quelles. Le cache existant est allégé une fois, en arrière-plan, 3 s après le démarrage, les plus lourdes d'abord, le verrou n'étant pris que pour remplacer un chemin (nouveau nom horodaté).
- **Changement d'onglet** : la bibliothèque reste montée (cachée) quand une autre page ou une fiche est affichée ; elle n'est plus reconstruite ni ses jaquettes rechargées, et le défilement est conservé. Ses raccourcis clavier sont ignorés tant qu'elle est cachée.

## D57 – Glisser-déposer, profil et barre de titre (2.1.0)

- **Glisser-déposer** : sous Windows, Tauri intercepte le glisser-déposer natif pour recevoir les .exe déposés dans la fenêtre (`onDragDropEvent`) ; l'événement `drop` HTML5 n'arrive alors jamais. Le Backlog et les collections n'étaient donc pas utilisables dans l'application, alors que le navigateur de développement les acceptait. Remplacé par un glisser-déposer maison aux événements de pointeur (`src/lib/drag.ts`) : seuil de 6 px (en dessous, c'est un clic), aperçu qui suit le pointeur, zones `[data-drop]` qui reçoivent un événement `game-drop`, `Échap` pour annuler, le clic qui suit un glissement n'ouvre pas la fiche. Le dépôt de .exe garde le mécanisme de Tauri. Vérifié par de vrais mouvements de souris (CDP) et dans la fenêtre native.
- **Photo de profil** : l'image choisie est recadrée dans l'application (cercle de 280 px, glisser + zoom de 1 à 4, l'image couvre toujours le cadre) et enregistrée en JPEG 320 × 320 dans le réglage `profile_photo` (≈ 20 à 40 Ko) ; le fichier d'origine n'est ni copié ni gardé. L'emoji reste possible.
- **Top 10** : réglage `profile_top10`, tableau de dix emplacements (identifiant ou vide) ; jeux inconnus et doublons ignorés à la lecture. Ajouter = choisir dans la bibliothèque (jeux déjà classés exclus) ; glisser un jeu sur un emplacement le déplace et décale les autres ; placer un jeu déjà classé le retire de son ancienne place. Les trois premiers sont mis en avant.
- **Image du profil** : 1080 × 1350 (comme le récap), bandeau d'accent, photo dans un anneau, statistiques, top 10 en deux rangées de cinq, badges obtenus ; couleurs et polices du thème actif.
- **Barre de titre** : fenêtre sans cadre (`decorations: false`, ombre conservée), barre de 32 px dessinée par l'application avec les boutons Windows habituels (fermer rouge au survol) ; zone de déplacement `data-tauri-drag-region` (double-clic = agrandir), redimensionnement par les bords et ancrage Windows fournis par Tauri. Le nom de l'application n'y est pas répété (il est en haut de la barre latérale). « Fermer » suit le réglage existant (zone de notification ou quitter).
- **Infobulle d'activité** : rendue au niveau de la page (portail, position fixe) au lieu de l'intérieur de la zone qui défile, retournée sous la case quand il manque de la place au-dessus, retirée si la page défile.
