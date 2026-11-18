# Notes de version

Chaque section `## [X.Y.Z]` devient le texte de la Release GitHub et de la bannière de mise à jour dans l'application.

## [2.0.0] – 2026-11-18

**Big Picture et Récap de l'année** 📺✨

- **Mode Big Picture** pour jouer depuis le canapé : plein écran, grandes jaquettes en rangées (Continuer à jouer, Favoris, À jouer, Tous les jeux installés), l'illustration du jeu sélectionné en fond, sons de navigation. Tout se fait à la **manette** (Xbox, PlayStation via la Gamepad API : stick ou croix pour se déplacer, A jouer, Y détails, B quitter), au clavier ou à la souris.
- Entrée et sortie avec **`F11`**, le bouton **Guide** de la manette, ou **Vue + Menu** ensemble.
- **Récap de l'année** 🎁 : à partir du 1er décembre (et toute l'année pour les années passées), une suite d'écrans animés : temps total et courbe des mois, top 5 des jeux, meilleure série, jour record, launcher préféré, badges et votre **profil de joueur** (Noctambule, Marathonien, Touche-à-tout, Fidèle, Guerrier du week-end…). Bannière sur l'accueil en décembre et janvier, bouton dans les Statistiques.
- **Export du récap en image PNG** (1080 × 1350, aux couleurs de votre thème) à partager.
- Nouvelle page **Profil** : avatar, pseudo, jeu favori, badges, statistiques clés, jeux les plus joués, exportable en image.
- La manette n'est jamais lue pendant une partie : elle reste au jeu.

## [1.5.0] – 2026-11-18

**Intégrations** 🔌

- **Jeux gratuits Epic de la semaine** sur l'accueil (en ce moment et à venir), avec une notification optionnelle quand de nouveaux jeux deviennent gratuits.
- **Actualités** de vos jeux Steam (API officielle de Steam) : fil sur l'accueil et onglet Actualités dans la fiche.
- **Galerie de captures d'écran** dans la fiche de chaque jeu : Steam, Xbox Game Bar, NVIDIA et vos propres dossiers, en plein écran.
- **Discord Rich Presence** (option) : « Joue à <jeu> » avec la jaquette et le temps écoulé.
- **Durée de vie des jeux** (histoire principale, 100 %) depuis IGDB, avec vos identifiants Twitch (facultatif) ; utilisée par le filtre « durée courte » de la roulette.
- **Sauvegarde et synchronisation** vers un dossier au choix (OneDrive, Google Drive, clé USB…) : heures, notes, thèmes perso et réglages, automatique ou à la demande, avec restauration sur un autre PC.
- Correction : choisir ou survoler un thème ne fait plus clignoter toute la fenêtre (aperçu instantané après un court arrêt de la souris, fondu seulement pour un vrai changement).
- La page Backlog explique à quoi elle sert et comment y ajouter des jeux.

## [1.4.0] – 2026-11-18

**Organisation** 🗂️

- **Accueil** : grande bannière « Continuer à jouer » avec un bouton Jouer géant sur l'illustration du jeu, jeux récents, série, succès récents, jeux « à jouer ensuite » et nouveautés de la bibliothèque.
- **Statut de chaque jeu** (À jouer, En cours, En pause, Terminé, 100 %, Abandonné) et page **Backlog** en colonnes : glissez une carte pour changer son statut. Filtre par statut dans la bibliothèque.
- **Note sur 10** et **notes personnelles** dans la fiche de chaque jeu ; **journal de session** optionnel (« boss du chapitre 3 battu ») visible dans l'historique.
- **Recherche rapide `Ctrl+K`** : trouver et lancer un jeu, ouvrir une page, changer de thème, au clavier.
- **« Je sais pas à quoi jouer » 🎲** : roulette animée parmi les jeux installés (backlog seulement, pas joué depuis 30 jours, genre).
- **Collections** : glissez une jaquette sur une collection de la barre latérale ; collections automatiques « Pas joué depuis 6 mois », « Presque 100 % » et « Ajoutés récemment ».
- Bibliothèque : taille des jaquettes réglable et nouvelle vue en bandeaux.

## [1.3.0] – 2026-11-18

**Statistiques et séries** 🔥

- **Séries de jours joués** : série actuelle (signalée « en danger » tant que vous n'avez pas joué aujourd'hui), meilleure série avec ses dates, jokers optionnels (un tous les 7 jours, 2 au plus).
- **Grille d'activité** façon GitHub sur 12 mois ou par année : survolez un jour pour voir les jeux joués, cliquez pour ses sessions. La même grille dans la fiche de chaque jeu.
- Nouvelle page **Statistiques** : périodes 7 jours / 30 jours / cette année / tout, filtre par jeu ou par launcher, temps par semaine et par mois, répartition par launcher et par genre, top jeux, carte de vos habitudes (jour × heure), records, succès (les plus rares, ceux presque à 100 %).
- **Badges** : Noctambule, Marathonien, Fidèle, Collectionneur, Touche-à-tout, Complétionniste, avec une notification à l'obtention.
- Widget de série dans la barre latérale, série dans le menu de la zone de notification, rappel du soir optionnel si la série est en danger.
- Réglages : seuil d'un jour joué (15 min par défaut) et heure de bascule (4 h : une partie à 1 h du matin compte pour la veille).

## [1.2.0] – 2026-11-18

**Thèmes** : dix ambiances complètes, chacune avec son fond, ses particules, ses polices et ses cartes.

- **Sombre**, **Clair**, **Sakura** 🌸 (pétales de cerisier, torii, vagues seigaiha), **Nuit à Kyoto** 🏮 (lanternes, lune, lucioles), **Automne** 🍂 (feuilles d'érable, grain de papier), **Hiver** ❄️ (neige, givre sur les cartes), **Océan** 🌊 (vagues, bulles), **Néon** 🌆 (grille en perspective, lueurs, scanlines), **Rétro Arcade** 👾 (étoiles en pixels, sons 8 bits) et **OLED** ⚫.
- Paramètres → Apparence : galerie avec aperçu en direct au survol, changement instantané en fondu.
- Éditeur de thème (accent, image de fond, particules, flou), import et export en `.json`.
- Options : réduire les animations (suit aussi le réglage de Windows), thème dynamique d'après la jaquette du jeu affiché, thème selon la saison, sons d'interface.
- Les effets s'arrêtent quand la fenêtre est masquée ou en arrière-plan, et pendant une partie.
- Correction : au tout premier lancement, les jaquettes ne s'affichaient qu'après un redémarrage.

## [1.1.1] – 2026-11-17

Première version publique, avec mises à jour automatiques.

- **Mises à jour automatiques** : GameLauncher vérifie au démarrage puis toutes les 6 h s'il existe une nouvelle version, affiche ses notes et l'installe en un clic (jamais pendant une partie).
- **Temps de jeu saisi à la main** : c'est maintenant une base à laquelle s'ajoutent les parties suivantes (le compteur ne reste plus bloqué). Les saisies existantes sont conservées.
- **Statistiques** : les heures saisies à la main comptent dans le temps de leur launcher ; la fiche jeu, la bibliothèque et les statistiques affichent le même total.
- **Jeux Steam** : le total affiché est le temps Steam plus les parties jouées depuis la dernière synchronisation.
- Suivi des jeux lancés en dehors de l'application, zone de notification, démarrage avec Windows, arrêt et fermeture forcée d'un jeu, emplacement personnalisé (évolutions V1.1).
