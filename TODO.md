# Reste à faire

## À vérifier par l'utilisateur (impossible à tester en autonomie)

- **API Steam avec une vraie clé** : les analyseurs sont testés sur des réponses d'exemple et les appels suivent la documentation, mais aucun appel authentifié n'a pu être fait (pas de clé API disponible). À vérifier : Paramètres > Compte Steam, puis comparaison des heures et des succès avec le client Steam (critère §11).
- **SteamGridDB avec une vraie clé** : même situation.

## Non fait

- **IGDB** (F-MET-02) : non intégré, car il exige un compte développeur Twitch (identifiant + secret). La description, les genres, le développeur et la date de sortie viennent de la fiche de la boutique Steam (sans clé), y compris pour les jeux locaux associés à une fiche. Voir `DECISIONS.md` D11.
- **Icônes des succès hors ligne** : elles sont chargées depuis le CDN Steam par la vue web (et conservées par son cache HTTP), pas copiées dans le cache d'images de l'application.
