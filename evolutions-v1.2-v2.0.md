# Grosses mises à jour V1.2 → V2.0 – Launcher de jeux unifié

*Complément au cahier des charges – 17 novembre 2026*

Les règles de l'Annexe A restent valables (autonomie, `DECISIONS.md`, tests, commits signés RedCodeSnoo). Chaque version est publiée par un tag `vX.Y.0`, ce qui déclenche la Release GitHub et la mise à jour automatique chez l'utilisateur.

| Version | Thème principal |
|---|---|
| **V1.2** | Système de thèmes et 10 thèmes |
| **V1.3** | Page Statistiques complète et streaks |
| **V1.4** | Organisation : accueil, backlog, notes, roulette, recherche rapide |
| **V1.5** | Intégrations : Discord, actus, jeux gratuits Epic, captures d'écran, durée de vie des jeux |
| **V2.0** | Mode Big Picture (manette) et « Récap de l'année » |

---

## V1.2 – Thèmes

### T1. Moteur de thèmes
| Réf. | Exigence | Priorité |
|---|---|---|
| T1-01 | Toute l'interface utilise des **variables CSS** (couleurs, rayons, ombres, polices, flou, durées d'animation). Plus aucune couleur codée en dur dans les composants | Must |
| T1-02 | Un thème est un fichier JSON avec : nom, aperçu, palette (fond, surfaces, texte, accent, succès, alerte…), polices, style des cartes (arrondi, bordure, effet verre), fond (couleur, dégradé, image ou scène animée), effet de particules, icônes de la barre latérale, sons d'interface optionnels | Must |
| T1-03 | Changement de thème **instantané**, sans redémarrage, avec une transition en fondu | Must |
| T1-04 | Page **Paramètres → Apparence** : galerie de vignettes (aperçu en direct au survol), application en un clic | Must |
| T1-05 | Option **« Réduire les animations »** (désactive particules et effets lourds) ; respecter automatiquement le réglage d'accessibilité de Windows | Must |
| T1-06 | Les particules et fonds animés se mettent **en pause** quand la fenêtre est masquée, en arrière-plan, ou quand un jeu est en cours (zéro impact sur les performances en jeu) | Must |
| T1-07 | Rendu des particules sur un `<canvas>` unique et léger (au plus 2 % de CPU en continu sur une machine moyenne), avec un nombre de particules adapté à la taille de la fenêtre | Must |
| T1-08 | **Éditeur de thème** : partir d'un thème existant, changer la couleur d'accent, le fond (image perso), l'intensité des particules et le flou, puis enregistrer comme thème perso | Should |
| T1-09 | **Import / export** de thèmes en `.json` (pour les partager) | Should |
| T1-10 | **Thème dynamique** (option) : la couleur d'accent et le fond flouté s'adaptent à la jaquette du jeu sélectionné (extraction de la couleur dominante) | Should |
| T1-11 | **Thème selon la saison** (option) : bascule automatique Printemps → Sakura, Été → Océan, Automne → Automne, Hiver → Hiver | Could |
| T1-12 | **Ressources** : uniquement des polices sous licence libre (Google Fonts / OFL), et des visuels créés pour le projet (SVG, dégradés, formes générées). Aucune image ni aucun personnage protégé (anime, jeux, marques) | Must |

### T2. Thèmes inclus
| Thème | Ambiance | Palette | Fond et effets | Polices |
|---|---|---|---|---|
| **Sombre** (défaut) | Moderne, sobre | Gris anthracite, accent bleu électrique | Dégradé subtil, effet verre sur les cartes | Inter |
| **Clair** | Propre, lumineux | Blanc cassé, accent indigo | Aucun | Inter |
| **Sakura** 🌸 | Japon au printemps, doux et poétique | Rose pâle, blanc crème, prune, accent rose vif | Silhouette SVG de torii et de montagne en fond, **pétales de cerisier qui tombent** en douceur, léger motif seigaiha (vagues japonaises) sur les surfaces | Noto Serif JP pour les titres, Noto Sans JP pour le texte |
| **Nuit à Kyoto** 🏮 | Japon de nuit | Bleu nuit, rouge laque, or | Lanternes qui brillent doucement, lune, quelques lucioles | Noto Serif JP |
| **Automne** 🍂 | Chaleureux, cosy | Orange brûlé, bordeaux, brun, or | **Feuilles d'érable qui tombent** en tournoyant, grain papier léger | Fraunces, Inter |
| **Hiver** ❄️ | Calme, givré | Bleu glacier, blanc, gris perle | **Flocons de neige**, givre sur les bords des cartes | Inter |
| **Océan** 🌊 | Frais, été | Turquoise, bleu profond, sable | Vagues animées en bas de l'écran, bulles discrètes | Inter |
| **Néon / Cyberpunk** 🌆 | Rétro-futuriste | Noir violacé, magenta, cyan | Grille en perspective animée, lueurs néon sur les bordures, scanlines légères | Orbitron pour les titres, Rajdhani |
| **Rétro Arcade** 👾 | Pixel art, années 80 | Noir, couleurs CGA vives | Étoiles en pixels qui défilent, curseur et boutons pixelisés, sons « 8 bits » optionnels | Press Start 2P pour les titres, VT323 |
| **OLED** ⚫ | Minimaliste | Noir pur, blanc, un seul accent | Aucun effet, contraste maximal (économie d'énergie sur les écrans OLED) | Inter |

---

## V1.3 – Statistiques et streaks

### S1. Règles de calcul
| Réf. | Règle | Priorité |
|---|---|---|
| S1-01 | Un **jour joué** est un jour où le temps de jeu total (tous jeux confondus) atteint un **seuil minimum**, réglable (15 min par défaut) | Must |
| S1-02 | **Heure de bascule** de la journée réglable (4 h du matin par défaut) : une partie à 1 h du matin compte pour la veille | Must |
| S1-03 | Une session à cheval sur deux jours est **répartie** entre les deux jours au prorata | Must |
| S1-04 | Seules les sessions réelles comptent (lancées ou détectées). Les heures saisies à la main n'ont pas de date, donc elles ne comptent ni dans la grille ni dans les streaks | Must |
| S1-05 | **Streak actuelle** = nombre de jours joués d'affilée jusqu'à aujourd'hui. Si aujourd'hui n'est pas encore joué, la streak d'hier reste affichée « en danger » jusqu'à la bascule | Must |
| S1-06 | **Meilleure streak** = plus longue série de l'historique, avec ses dates de début et de fin | Must |
| S1-07 | Les statistiques sont calculées en Rust, avec une table de résumés journaliers mise à jour à chaque fin de session (affichage instantané même avec des années d'historique) | Must |
| S1-08 | **Jokers de streak** (option) : un joker gagné tous les 7 jours d'affilée (2 au maximum), consommé automatiquement pour sauver la streak un jour manqué | Could |

### S2. Page Statistiques
| Réf. | Élément | Priorité |
|---|---|---|
| S2-01 | **En-tête** : 4 tuiles (temps total, streak actuelle 🔥, meilleure streak, jours joués cette année) | Must |
| S2-02 | **Grille d'activité** façon GitHub : 53 colonnes (semaines) × 7 lignes (jours), une case par jour sur 12 mois glissants. Couleur à 5 niveaux d'intensité selon le temps joué ce jour-là, dans la couleur d'accent du thème | Must |
| S2-03 | Survol d'une case : date, temps total, jeux joués ce jour-là (avec leurs jaquettes miniatures). Clic : liste des sessions du jour | Must |
| S2-04 | Sélecteur d'année (2026, 2025…) et vue « 12 derniers mois » | Must |
| S2-05 | La même grille dans la **fiche de chaque jeu** (activité de ce jeu seulement) | Should |
| S2-06 | **Temps par semaine / mois** (barres), filtrable par jeu ou par launcher | Must |
| S2-07 | **Répartition par launcher** et **par genre** (barres horizontales triées) | Must |
| S2-08 | **Top jeux** de la période, avec jaquette, temps et barre de progression | Must |
| S2-09 | **Heatmap des habitudes** : 7 jours × 24 heures, pour voir à quels moments de la semaine on joue le plus | Should |
| S2-10 | **Records** : plus longue session, plus grosse journée, jeu le plus joué en une semaine, heure de jeu la plus tardive | Should |
| S2-11 | **Succès** : total débloqué, pourcentage de complétion global, succès les plus rares obtenus, jeux « presque finis à 100 % » (≥ 80 %) | Should |
| S2-12 | Sélecteur de période global : 7 jours, 30 jours, cette année, tout | Must |
| S2-13 | **Badges du launcher** (méta-succès) : par exemple « Noctambule » (session après 3 h), « Marathonien » (session de plus de 6 h), « Fidèle » (streak de 30 jours), « Collectionneur » (100 jeux), « Touche-à-tout » (5 launchers utilisés), « Complétionniste » (un jeu à 100 %). Notification à l'obtention | Could |
| S2-14 | **Objectifs** (option) : objectif hebdomadaire de temps de jeu, ou limite quotidienne avec rappel doux | Could |
| S2-15 | Graphiques dessinés avec une bibliothèque légère (Recharts ou visx), en utilisant les variables du thème (adaptés à tous les thèmes) | Must |

### S3. Widget streak
| Réf. | Exigence | Priorité |
|---|---|---|
| S3-01 | Mini-version de la grille (4 dernières semaines) et de la streak sur l'accueil | Must |
| S3-02 | Streak affichée dans le menu de la zone de notification | Should |
| S3-03 | Notification optionnelle le soir (heure réglable) si la streak est en danger | Could |

---

## V1.4 – Organisation

| Réf. | Fonctionnalité | Priorité |
|---|---|---|
| O-01 | **Page d'accueil** : grande bannière « Continuer à jouer » (dernier jeu, avec un bouton Jouer géant sur l'artwork en fond), jeux récents, widget streak, succès récents, nouveautés de la bibliothèque | Must |
| O-02 | **Statut de chaque jeu** : À jouer, En cours, Terminé, 100 %, Abandonné, En pause. Filtre et vue dédiée « Backlog » | Must |
| O-03 | **Note personnelle** sur 10 et **notes de texte** par jeu | Should |
| O-04 | **Journal de session** : à la fin d'une session, notification optionnelle « Une note sur cette session ? » (par exemple « boss du chapitre 3 battu »), visible dans l'historique | Could |
| O-05 | **Recherche rapide** `Ctrl+K` : palette de commandes pour trouver et lancer un jeu, changer de thème ou ouvrir une page, au clavier, en moins d'une seconde | Must |
| O-06 | **« Je sais pas à quoi jouer »** 🎲 : roulette animée qui tire un jeu au hasard parmi les jeux installés, avec des filtres (backlog seulement, durée courte, genre, pas joué depuis 30 jours) | Should |
| O-07 | **Collections** personnalisées (glisser-déposer) et collections automatiques (« Pas joué depuis 6 mois », « Presque 100 % », « Ajoutés récemment ») | Should |
| O-08 | Affichage de la bibliothèque : taille des jaquettes réglable, vue grille, liste ou bandeaux | Should |

---

## V1.5 – Intégrations

| Réf. | Fonctionnalité | Priorité |
|---|---|---|
| I-01 | **Discord Rich Presence** (option) : afficher « Joue à <jeu> via <nom de l'app> » avec la jaquette et le temps écoulé | Should |
| I-02 | **Jeux gratuits Epic de la semaine** : encart sur l'accueil et notification optionnelle (endpoint public des promotions Epic) | Should |
| I-03 | **Actualités** des jeux Steam de la bibliothèque (API officielle `ISteamNews`), dans la fiche jeu et en flux sur l'accueil | Should |
| I-04 | **Galerie de captures d'écran** par jeu : lecture des dossiers de captures Steam, Xbox Game Bar (`Vidéos\Captures`), NVIDIA et dossiers perso ; plein écran, ouvrir le dossier | Could |
| I-05 | **Durée de vie des jeux** (histoire principale / complétion) affichée dans la fiche et utilisée par la roulette. Source : IGDB (`time_to_beat`) en priorité. Si une source non officielle est utilisée, elle doit être optionnelle et désactivable | Could |
| I-06 | **Sauvegarde et synchronisation** des données de l'app (heures, notes, thèmes) vers un dossier au choix (OneDrive, Google Drive, clé USB), avec restauration | Should |

---

## V2.0 – Mode Big Picture et Récap de l'année

| Réf. | Fonctionnalité | Priorité |
|---|---|---|
| B-01 | **Mode Big Picture** : interface plein écran pour la télé, navigable entièrement à la **manette** (Xbox, PlayStation, via la Gamepad API), avec de grandes jaquettes, un fond animé selon le thème et des sons de navigation | Must |
| B-02 | Raccourci : bouton Guide/PS ou `F11` pour entrer et sortir du mode | Should |
| B-03 | **Récap de l'année** (façon « Wrapped ») : à partir du 1er décembre, une suite d'écrans animés (temps total, top 5 jeux, meilleure streak, jour record, launcher préféré, badges, « ton profil de joueur » : noctambule, marathonien, touche-à-tout…) | Must |
| B-04 | Export du récap en **image PNG** à partager | Must |
| B-05 | **Profil de joueur** : page avec avatar, pseudo, jeu favori, badges, statistiques clés, exportable en image | Could |

---

## Exigences transversales
- **Performance** : la page Statistiques s'affiche en moins de 300 ms avec 3 ans d'historique ; aucun effet visuel ne tourne pendant qu'un jeu est en cours.
- **Qualité visuelle** : chaque thème est vérifié sur toutes les pages (bibliothèque, fiche jeu, stats, paramètres, Big Picture), avec des contrastes de texte lisibles (WCAG AA).
- **Migration** : chaque version met à jour la base de données automatiquement, sans perte de données.
- **Tests** : calcul des jours joués (seuil, heure de bascule, sessions à cheval sur minuit, fuseau horaire, changement d'heure été/hiver), streak actuelle et meilleure, jokers, chargement et validation des thèmes JSON.
- **Captures** : ajouter au README une capture de chaque thème et de la page Statistiques.
