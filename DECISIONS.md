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
