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
