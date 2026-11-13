# Évolutions V1.1 – Launcher de jeux unifié

*Complément au cahier des charges – 13 novembre 2026*

Ce document s'ajoute à `cahier-des-charges-launcher.md`. Les règles de l'Annexe A (autonomie, `DECISIONS.md`, tests, commits) restent valables.

---

## E1. Arrêt forcé d'une session de jeu

**Problème constaté** : un jeu reste parfois affiché « En cours » alors qu'il a été quitté.

### Exigences
| Réf. | Exigence | Priorité |
|---|---|---|
| E1-01 | Sur un jeu « En cours » (fiche jeu, carte de la bibliothèque, menu de la zone de notification), bouton **« Arrêter le suivi »** : clôt la session immédiatement (heure de fin = maintenant) sans toucher aux processus | Must |
| E1-02 | Bouton **« Forcer la fermeture du jeu »** : termine les processus du jeu (et leurs enfants) après une fenêtre de confirmation, puis clôt la session | Must |
| E1-03 | Si un processus ne peut pas être terminé (droits insuffisants, anti-cheat), afficher un message clair, et proposer quand même « Arrêter le suivi » | Must |
| E1-04 | Après un arrêt manuel, la durée enregistrée peut être corrigée depuis l'historique des sessions (modifier l'heure de fin, supprimer la session) | Should |
| E1-05 | **Corriger la cause** : identifier pourquoi des sessions restent ouvertes et corriger la détection de fin. Pistes : processus annexes qui restent en vie (crash handler, overlay, launcher du jeu, anti-cheat), processus en état « suspendu », session jamais reprise après un redémarrage de l'app | Must |
| E1-06 | Au démarrage de l'app, toute session « en cours » dont aucun processus ne tourne est clôturée automatiquement, avec son dernier battement enregistré comme heure de fin | Must |
| E1-07 | Liste d'exclusion des processus annexes qui ne doivent **pas** maintenir une session ouverte : `*CrashHandler*.exe`, `*CrashReport*.exe`, `UnityCrashHandler*.exe`, `EasyAntiCheat*.exe`, `BEService*.exe`, `vgc.exe`, `vgtray.exe`, `RiotClient*.exe`, `steamwebhelper.exe`, `EpicWebHelper.exe`, `Battle.net*.exe`, `Agent.exe`, `*launcher*.exe` (sauf si c'est l'exe principal défini pour le jeu), etc. | Must |

---

## E2. Modifier l'emplacement d'un jeu (dossier et exécutable)

**Besoin** : pouvoir corriger le dossier ou l'exécutable de n'importe quel jeu (Steam, Epic, Riot… et jeux locaux) quand la détection s'est trompée.

### Exigences
| Réf. | Exigence | Priorité |
|---|---|---|
| E2-01 | Dans la fiche jeu → onglet **Paramètres du jeu**, pour toutes les sources : champ **Dossier d'installation** (sélecteur de dossier) et champ **Exécutable principal** (sélecteur de fichier `.exe`, ouvert par défaut dans le dossier d'installation) | Must |
| E2-02 | Chaque champ affiche la valeur **détectée automatiquement** et la valeur **personnalisée** si elle existe, avec un bouton **« Rétablir la valeur détectée »** | Must |
| E2-03 | Les valeurs personnalisées ne sont **jamais écrasées** par une nouvelle détection (stockage dans des champs séparés : `installPathOverride`, `executablePathOverride`) | Must |
| E2-04 | Validation : le dossier et l'exe doivent exister ; sinon, message d'erreur et pas d'enregistrement. Avertissement (non bloquant) si l'exe choisi est en dehors du dossier d'installation | Must |
| E2-05 | **Méthode de lancement** choisie par jeu : « Via le launcher d'origine » (par défaut pour Steam, Epic, Riot, Battle.net, Ubisoft, EA) ou « Exécutable direct ». Pour les jeux locaux, seule l'option exe est proposée | Must |
| E2-06 | Le dossier et l'exe personnalisés servent aussi au **suivi des heures** (E1 et E4) : ce sont eux qui identifient les processus du jeu | Must |
| E2-07 | Aide affichée sous l'option « Exécutable direct » : certains jeux exigent leur launcher (DRM, anti-cheat, Riot, Battle.net) et peuvent refuser de démarrer ou redémarrer le launcher eux-mêmes | Should |
| E2-08 | Champs **Arguments de lancement** et **Dossier de travail** disponibles pour toutes les sources en mode exe direct | Should |
| E2-09 | L'app ne modifie jamais les fichiers du launcher d'origine : la personnalisation reste interne à l'app | Must |

---

## E3. Fonctionnement en arrière-plan (zone de notification) et démarrage avec Windows

### Exigences
| Réf. | Exigence | Priorité |
|---|---|---|
| E3-01 | Icône permanente dans la **zone de notification** de Windows (à côté du son) | Must |
| E3-02 | Clic gauche sur l'icône : ouvre ou affiche la fenêtre principale. Clic droit : menu avec **Ouvrir**, **Jeux récents** (5 derniers, lançables directement), **Jeu en cours** (nom et durée, avec « Arrêter le suivi »), **Quitter** | Must |
| E3-03 | Fermer la fenêtre (croix) **réduit dans la zone de notification** au lieu de quitter. Option dans les Paramètres : « La croix ferme complètement l'application ». Seul « Quitter » arrête vraiment l'app | Must |
| E3-04 | Première fermeture : notification Windows « L'app continue de tourner en arrière-plan » (une seule fois) | Should |
| E3-05 | Option **« Lancer au démarrage de Windows »** (désactivée par défaut), via `tauri-plugin-autostart` | Must |
| E3-06 | Option **« Démarrer réduit dans la zone de notification »** (activée par défaut quand le démarrage avec Windows est actif) | Must |
| E3-07 | **Instance unique** : relancer l'app ramène la fenêtre existante au lieu d'en ouvrir une seconde (`tauri-plugin-single-instance`) | Must |
| E3-08 | En arrière-plan, l'app reste légère : moins de 100 Mo de RAM, CPU proche de 0 %. Le webview peut être déchargé quand la fenêtre est masquée si ça aide | Should |
| E3-09 | L'icône change (ou porte un point vert) quand un jeu est en cours | Could |

---

## E4. Détection des sessions de jeu lancées en dehors de l'app ⭐ (prioritaire)

**Besoin** : comme l'app tourne en arrière-plan, elle doit enregistrer le temps de jeu de **tous les jeux connus**, même lancés depuis Steam, un raccourci du Bureau, le launcher Riot, etc.

### Fonctionnement attendu
1. Un service de fond (`ProcessWatcher`, côté Rust) surveille le démarrage et l'arrêt des processus en permanence tant que l'app tourne (fenêtre ouverte ou non).
2. Pour chaque nouveau processus, on récupère le chemin complet de son exécutable et on le compare à un **index des jeux connus** :
   - exe principal (personnalisé ou détecté) → correspondance directe
   - sinon, exe situé **dans le dossier d'installation** d'un jeu (personnalisé ou détecté) et absent de la liste d'exclusion E1-07
3. En cas de correspondance, une session est ouverte pour ce jeu, avec l'origine **« Détectée »** (ou **« Lancée depuis l'app »** si elle vient de l'app).
4. La session se termine quand plus aucun processus correspondant à ce jeu ne tourne (même logique que E1, avec délai de grâce).

### Exigences
| Réf. | Exigence | Priorité |
|---|---|---|
| E4-01 | Détection de tout jeu présent dans la bibliothèque, quelle que soit la façon dont il a été lancé | Must |
| E4-02 | Délai de détection inférieur à 10 s après l'apparition du processus | Must |
| E4-03 | Méthode : interrogation périodique de la liste des processus (toutes les 3 à 5 s, crate `sysinfo` ou API Windows `CreateToolhelp32Snapshot` + `QueryFullProcessImageNameW`), ou événements WMI (`__InstanceCreationEvent` sur `Win32_Process`) **sans exiger les droits administrateur**. Choisir la solution la plus fiable et la plus légère, et noter le choix dans `DECISIONS.md` | Must |
| E4-04 | Index des jeux en mémoire (table de hachage chemin normalisé → jeu), reconstruit à chaque modification de la bibliothèque. Chemins comparés **sans tenir compte de la casse**, normalisés (séparateurs, `\\?\`, lettres de lecteur) | Must |
| E4-05 | **Processus protégés** : si le chemin d'un processus est illisible (jeu lancé en administrateur, anti-cheat), repli sur une comparaison par **nom d'exécutable** avec l'exe principal connu du jeu | Must |
| E4-06 | Pas de double session : un jeu lancé depuis l'app puis détecté par le watcher ne crée qu'une session. Plusieurs processus du même jeu = une seule session | Must |
| E4-07 | Sessions de moins de 60 s ignorées (lancement raté, fermeture immédiate) – seuil réglable dans les Paramètres | Should |
| E4-08 | Détection en cours de partie : si l'app démarre alors qu'un jeu tourne déjà, une session est ouverte dès la première analyse (heure de début = heure de création du processus si elle est disponible) | Must |
| E4-09 | Robustesse : battement enregistré toutes les 60 s en base ; en cas de plantage, d'arrêt du PC ou de mise en veille, la session est close à son dernier battement. Mise en veille de plus de 5 min = fin de session (le temps de veille n'est pas compté) | Must |
| E4-10 | Notification Windows optionnelle : « Session détectée : <jeu> » (désactivée par défaut) | Could |
| E4-11 | Option dans les Paramètres : « Suivre les jeux lancés en dehors de l'application » (activée par défaut) | Must |
| E4-12 | Dans l'historique des sessions, une colonne **Origine** indique « Lancée depuis l'app » ou « Détectée » | Should |
| E4-13 | Règle de calcul des heures (rappel de F-HEU-05) : si la source fournit un temps (Steam, GOG…), c'est ce temps qui est affiché comme total, et les sessions locales servent à l'historique et aux statistiques sans double comptage. Sinon, le total est la somme des sessions locales (lancées et détectées) | Must |
| E4-14 | Jeux sur un disque externe : ils restent dans l'index même si le disque est débranché, et la détection fonctionne dès que le disque est rebranché, même si sa lettre a changé (identification par le numéro de série du volume, F-EXT-02) | Should |

### Exes principaux connus (à préremplir pour les cas difficiles)
| Jeu | Exécutable à surveiller |
|---|---|
| League of Legends | `League of Legends.exe` (pas `LeagueClient.exe`, qui est le client) |
| VALORANT | `VALORANT-Win64-Shipping.exe` |
| Jeux Unreal Engine en général | `*-Win64-Shipping.exe` dans `Binaries\Win64` |
| Battle.net (WoW, Overwatch, Diablo…) | l'exe du jeu dans son dossier, pas `Battle.net.exe` ni `Agent.exe` |

L'agent peut compléter ce tableau.

---

## Tests à ajouter
- E1 : un faux jeu de test qui lance un processus enfant persistant → la session se clôt quand le processus principal s'arrête si l'enfant est dans la liste d'exclusion ; « Forcer la fermeture » tue l'arbre de processus.
- E2 : une valeur personnalisée survit à une nouvelle détection ; « Rétablir » revient à la valeur détectée ; un chemin invalide est refusé.
- E3 : vérification manuelle (fermeture → zone de notification, menu, démarrage avec Windows, instance unique), décrite dans `PROGRESS.md`.
- E4 : lancer l'exe de test **hors de l'app** → session « Détectée » ouverte en moins de 10 s et close à l'arrêt ; lancement depuis l'app → une seule session ; session de moins de 60 s ignorée ; chemins avec une casse différente reconnus.

## Ordre de réalisation
1. **E3** (zone de notification, démarrage avec Windows, instance unique), car E4 en dépend
2. **E4** (ProcessWatcher), avec la refonte de la détection de fin de session qui corrige E1-05 au passage
3. **E1** (boutons d'arrêt, nettoyage au démarrage)
4. **E2** (emplacement personnalisé)

Un tag Git `v1.1` à la fin, avec `PROGRESS.md` et `README.md` mis à jour.
