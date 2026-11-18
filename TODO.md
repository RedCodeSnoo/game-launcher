# Reste à faire

## À vérifier par l'utilisateur (impossible à tester en autonomie)

- **API Steam avec une vraie clé** : les analyseurs sont testés sur des réponses d'exemple et les appels suivent la documentation, mais aucun appel authentifié n'a pu être fait (pas de clé API disponible). À vérifier : Paramètres > Compte Steam, puis comparaison des heures et des succès avec le client Steam (critère §11).
- **SteamGridDB avec une vraie clé** : même situation.
- **Lancement réel de chaque jeu** (Steam, Epic, Riot, Battle.net) : les commandes sont construites selon le §4 mais n'ont pas été exécutées pendant le développement (voir `DECISIONS.md` D17). À essayer : un jeu par source.
- **Battle.net** : `--exec="launch <CODE>"` ne lance le jeu que si Battle.net est déjà ouvert et connecté ; sinon il ouvre seulement le launcher. Le code de lancement de la variante `wowf` détectée ici (dossier `_forever_`) est supposé être celui de la famille (`WoW`).
- **Riot** : Teamfight Tactics apparaît comme un produit à part (il possède son propre dossier), alors qu'il se lance en pratique depuis le client League of Legends. À masquer si gênant.
- **GOG, Ubisoft Connect, EA app** : aucun de ces launchers n'est installé sur cette machine. Leurs modules sont testés sur des données d'exemple (registre simulé, base Galaxy de test, `installerdata.xml`) mais jamais sur une installation réelle. Points les plus incertains : le schéma de la base GOG Galaxy et le lancement EA par URI `origin2://` quand l'exécutable est introuvable.
- **Xbox** : la détection est vérifiée sur cette machine (Minecraft Launcher dans `C:\XboxGames`, nom de famille du paquet résolu) ; le lancement `shell:AppsFolder\…` n'a pas été exécuté.
- **Disque externe réel** : la logique (grisé, lettre qui change) est testée avec des lecteurs simulés ; aucun disque externe n'a été branché pendant le développement.
- **Mémoire au repos** : le processus principal occupe ~40 Mo et consomme ~0,2 % d'un cœur au repos (47 ms de CPU sur 20 s) ; la part des processus WebView2 n'a pas pu être isolée des autres applications WebView2 ouvertes sur la machine. L'objectif de 150 Mo au total reste à mesurer dans le Gestionnaire des tâches.
- **Interface dans la fenêtre native** : l'interface a été vérifiée dans un navigateur avec un backend simulé (y compris avec 1 000 jeux) ; l'application finale a été démarrée et ses journaux contrôlés, mais sa fenêtre n'a pas été inspectée visuellement.

### Évolutions V1.1 (E1 à E4)

- **Menu et icône de la zone de notification** : l'icône est créée et l'application reste active fenêtre fermée (vérifié), mais le menu (Jeux récents, Jeu en cours, Quitter), le clic gauche et le point vert n'ont pas pu être regardés à l'écran. À essayer : clic droit sur l'icône près de l'horloge.
- **Détection de vrais jeux lancés hors de l'application** : vérifiée avec un faux jeu réel (vrais processus), pas avec Steam, Riot ou Battle.net. À essayer : lancer un jeu depuis Steam, GameLauncher étant dans la zone de notification, puis regarder l'onglet Sessions (origine « Détectée »).
- **Jeux protégés par un anti-triche** (VALORANT, etc.) : si le chemin du processus est illisible, le jeu est reconnu par son nom d'exécutable (testé avec un processus simulé seulement).
- **Jeux dont le vrai processus tourne hors de leur dossier** : Minecraft Launcher (le jeu Java tourne depuis `%APPDATA%\.minecraft`) ne sera pas détecté tel quel. Contournement : fiche jeu > Paramètres du jeu > Dossier d'installation.
- **Démarrage avec Windows après un vrai redémarrage** : l'entrée de démarrage est bien écrite et retirée, et le démarrage réduit fonctionne avec l'argument `--autostart` ; le redémarrage du PC lui-même n'a pas été fait.
- **Notifications Windows** (première fermeture, « Session détectée ») : envoyées sans erreur, mais leur affichage n'a pas été vu.
- **« Forcer la fermeture » sur un jeu protégé** : le cas d'échec (droits insuffisants, anti-triche) affiche un message et propose d'arrêter le suivi ; il n'a été vérifié que dans l'interface simulée.
- **Processus « suspendus »** (piste d'E1-05) : non distingués des processus actifs ; aucun cas réel observé. « Arrêter le suivi » couvre ce cas.
- **Teamfight Tactics** : son exécutable de partie n'est pas connu avec certitude ; s'il se joue par `League of Legends.exe`, ses parties seront comptées sur League of Legends.

### Intégrations (V1.5)

- **Discord Rich Presence** : le protocole est testé (trames, activité) mais pas avec un vrai client Discord, faute d'identifiant d'application. À essayer : créer une application sur discord.com/developers, coller son identifiant dans Paramètres → Intégrations, lancer un jeu Discord ouvert.
- **Durée de vie (IGDB)** : analyse testée sur des réponses d'exemple ; aucun appel authentifié fait (pas d'identifiants Twitch).
- **Captures Xbox Game Bar et NVIDIA** : logique testée sur des dossiers simulés ; sur ce PC seules des captures Steam ont été trouvées en réel.

### Big Picture (V2.0)

- **Vraie manette** : la lecture de la manette est testée sur des états simulés et le mode a été parcouru au clavier ; aucune manette n'était branchée. À essayer : brancher une manette Xbox ou PlayStation, appuyer sur Vue + Menu.
- **Bouton Guide / PS** : Windows ne le transmet pas pour toutes les manettes (souvent capturé par la Xbox Game Bar ou Steam) ; Vue + Menu et `F11` fonctionnent dans tous les cas.

## Non fait

- **Fusion de bibliothèques entre PC** : la synchronisation par dossier remplace la bibliothèque à la restauration ; elle ne fusionne pas deux historiques modifiés en parallèle.

- **Objectifs de temps de jeu** (S2-14, « Could ») : objectif hebdomadaire ou limite quotidienne avec rappel doux. Non fait en V1.3.

- **IGDB** (F-MET-02) : non intégré, car il exige un compte développeur Twitch (identifiant + secret). La description, les genres, le développeur et la date de sortie viennent de la fiche de la boutique Steam (sans clé) et du catalogue Epic. Voir `DECISIONS.md` D11.
- **Epic : heures et succès en ligne** (API non officielle de Legendary/Heroic) : non fait. Elle exige une connexion OAuth interactive au compte Epic, impossible à tester sans l'utilisateur, et son usage peut être contraire aux CGU (§10). Les jeux Epic possédés et leurs visuels viennent du cache local du launcher ; les heures viennent du suivi maison.
- **Succès GOG** (API GOG avec connexion au compte), **API non officielles Ubisoft et Xbox (OpenXBL)** : non faits, pour les mêmes raisons. Ces sources affichent « Aucun succès ».
- **Détection par les raccourcis du Bureau et du menu Démarrer** (F-LOC-06, « Could ») : non fait.
- **Icônes des succès hors ligne** : elles sont chargées depuis le CDN Steam par la vue web (et conservées par son cache HTTP), pas copiées dans le cache d'images de l'application.
- **Signature du code** de l'installeur (utile seulement en cas de diffusion publique).
- **Après V2.0** : émulateurs, fusion de bibliothèques entre PC.
