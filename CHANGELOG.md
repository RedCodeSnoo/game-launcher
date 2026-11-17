# Notes de version

Chaque section `## [X.Y.Z]` devient le texte de la Release GitHub et de la bannière de mise à jour dans l'application.

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
