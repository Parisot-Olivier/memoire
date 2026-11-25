# Interface graphique minimale — incrément 4.3

Version 0.1 — 25 octobre 2026 — rédigée avant le code (D17).

## 1. Rôle et limites

Une fenêtre qui transcrit les commandes de la ligne de commande : un bouton par commande, un formulaire pour ses arguments, une zone qui affiche la sortie. Elle ne contient aucune règle : elle construit une ligne de commande, lance l’exécutable `memoire` et affiche ce qu’il répond. Tout ce que la ligne de commande refuse, l’interface le refuse de la même façon, avec le même message.

Pas de mise en page élaborée, pas de graphe, pas d’édition intégrée : c’est un panneau de commandes.

## 2. Une seule source : l’aide de l’exécutable

Au démarrage, l’interface lance `memoire aide` et construit ses boutons et formulaires à partir de la liste des commandes. Ajouter une commande à la ligne de commande (CLI.md, section 7) l’ajoute à l’interface sans autre modification.

Lecture de l’usage d’une commande :

| Forme dans l’usage | Champ de formulaire |
|---|---|
| `NOM` en majuscules | texte obligatoire (identifiant, ligne, fichier…) |
| `[NOM]` | texte facultatif |
| `a\|b\|c` | liste de choix obligatoire |
| `--option VALEUR` | texte obligatoire |
| `--option a\|b` | liste de choix obligatoire |
| `[--option VALEUR]`, `[--option a\|b]` | facultatif |
| `[--drapeau]` | case à cocher |
| `(--a X \| --b Y)` | deux champs facultatifs, l’un ou l’autre |

- Un champ dont le nom contient `FICHIER` ou dont l’option contient `fichier` a un bouton « Parcourir ». Un champ `DOSSIER` (`indexer`, incrément 6.3) et l’option `--code` (`reprise noter`, `reprise voir`, incrément 6.5) ont un bouton « Parcourir… » qui choisit un dossier.
- Un argument de position dont le résumé donne la liste « NOM : v1, v2, … » devient une liste de choix (cas de `lier`).
- Les tests de `gui/test_schema.py` lisent l’aide figée de la ligne de commande (`crates/memoire-cli/tests/aide.txt`) et vérifient que chaque commande se transcrit sans exception : une commande dont l’usage sortirait de cette grammaire fait échouer le contrôle complet.

## 3. Comportement

- **Base** : préremplie par `MEMOIRE_BASE`, sinon `.memoire/depot.db` du répertoire de lancement ; elle est passée par `--base`. Depuis 8.1, elle se change par « Changer de projet… » (pastille du projet, bouton Projet, pastille du fichier dans la barre d’état).
- **Exécuter** : un champ obligatoire vide est signalé avant tout lancement ; sinon la commande est lancée et sa sortie, ses erreurs et son code de retour s’affichent.
- **Opération critique** (code 4) : l’aperçu s’affiche, et un bouton « Confirmer » relance la même commande avec `--oui`. Rien n’est écrit sans ce second clic.
- **Base de schéma 1** (code 3, indication « memoire migrer », incrément 5.2d) : l’interface demande s’il faut migrer la base ; « oui » lance `migrer` (sauvegarde, conversion, vérification complète, remplacement) ; « non » ne fait rien. Jamais automatique.
- **Statut** : la barre d’état (8.1) montre en permanence la piste active et les notes à revoir ; « Où en est le projet ? » affiche le détail.
- **Fiche de reprise** (incrément 6.5, FICHE-REPRISE.md, section 6) : à l’ouverture, l’interface lance `reprise voir` et affiche la dernière fiche de la branche active dans la zone de sortie ; rien ne s’affiche sans base, sans fiche ou en cas d’erreur. Le dossier du code comparé est le répertoire de lancement.
- La zone de sortie est sélectionnable : les identifiants se copient pour les commandes suivantes.
- **Autocomplétion (R3)** : un champ qui désigne une note (`NOTE`, `ORIGINE`, `DESTINATION`, `DEPENDANT`, `CAUSE`, `PASSAGE`, `RESERVE`, `ECHANGE`, `--precedent`) est une liste : on tape une partie du nom, on ouvre la liste, qui propose les notes correspondantes (`memoire chercher`, sans tenir compte de la casse ni des accents) avec leur rôle et leur identifiant ; le choix remplace le texte par l’identifiant complet. Dans un champ à plusieurs notes (`NOTES` : `--classe-dans`, `--depend-de`, `--developpe-a-partir-de`, `--lien` de `note creer`, incrément 5.3), les notes sont séparées par des virgules : la liste porte sur la dernière, et le choix ne remplace qu’elle.
- **Sauvegarde (R9)** : le bouton `sauvegarder` écrit, sans argument, une copie horodatée dans `sauvegardes/` à côté de la base.

## 2 bis. Panneau des actions (incrément 7.1, ACTIONS.md, D30)

- **Menu par sujet** à gauche : Écrire, Relier, Juger, Lire, Code, Histoire et pistes, Où j’en suis, Projet, puis « Toutes les commandes (mode expert) » (les formulaires générés depuis l’aide, décrits à la section 2).
- **Une action** : son titre, sa phrase d’aide, ses questions minimales (« Quelle note ? », « Pourquoi ? ») ; « Si besoin : » des cases à cocher qui ajoutent leurs questions ; « Faire ».
- **Avant une opération importante** : « Ce que ça va changer » (nouvelle piste, notes à revoir), puis « Confirmer » ou « Ne rien faire ».
- **« Et ensuite ? »** : des boutons vers les actions qui suivent, pré-remplies avec l’objet créé ou modifié.
- **Vocabulaire** : projet, piste, étape, idée, solution technique, note libre, thème, s’appuie sur, vient de, garder quand même, adopter une piste… (ACTIONS.md, section 3).
- **Adopter une piste** : la piste à adopter et celle qui l’adopte par leur nom, la phrase qui résume, un choix par désaccord.
- **Retrouver une note** (incrément 7.2, RETROUVER.md) : dans tout champ de note, des suggestions s’ouvrent pendant la frappe (↑ ↓ Entrée ou clic) et la note choisie s’affiche « nom · id8 » ; « Voir une note… » et « Voisinage d’une note… » ouvrent les actions correspondantes ; « Quelles notes concernent ce code ? » donne, pour un fichier (et une ligne ou un extrait), les notes trouvées avec « Lire » et « Voisinage ».
- Fichiers : `gui/actions.py` (catalogue, sans Tkinter), `gui/formulaire.py`. Essais : `gui/test_actions.py` (sans affichage, dont la cohérence avec l’aide figée), `gui/test_gui.py` (EssaiDuPanneauDesActions).

## 3 bis. Visionneur (incrément 6.1, VISIONNEUR.md, D25)

- **Ouverture** : bouton « Voir » à côté de chaque champ qui désigne une note, et « Voir une note… » en haut de la fenêtre (identifiant ou préfixe).
- **Lecture** : un échange en deux zones (question, réponse), une note en une zone avec la liste de ses versions (0 pour le texte de création). Le texte vient de `memoire source`, rendu par `gui/rendu.py` (titres, gras, italique, code, blocs de code, listes, citations `>`, liens, séparateurs ; le reste tel quel) ; « Brut » montre le texte exact.
- **Citations** : celles de la source affichée (`memoire citations`) sont surlignées ; un clic en montre l’identifiant, la plage, le statut et « source révisée ».
- **Citer la sélection** : la plus petite plage de la source qui contient la sélection est citée par `memoire citer --plage`, avec sa relation vers la source ; une opération critique se confirme par « Confirmer » ; le compte rendu s’affiche dans la fenêtre principale et le visionneur se recharge.
- **Correspondance** (INV-VIEW-01, INV-SOURCE-02) : chaque caractère affiché provient d’une position de la source ; le surlignage porte sur les mêmes octets en rendu et en brut. Essais : `gui/test_rendu.py` (sans affichage), `gui/test_gui.py` (sous `xvfb-run`).

## 3 ter. Relier la sélection (incrément 6.2, CENTRE.md)

Sous les zones de lecture, le panneau « Relier la sélection » : « Soutient », « Inspire », « Classe dans », « Lien » (plusieurs notes, autocomplétion par nom, partagée avec la fenêtre principale par `gui/champs.py`), « Nouvelle note » (titre et rôle), « Réserve » (titre ; seulement pour un échange). « Citer la sélection » passe à `memoire citer` ce qui est rempli : une seule opération ; le panneau se vide après réussite.

## 3 quater. Vue locale (incrément 6.6, VOISINAGE.md ; R7)

- **Ouverture** : bouton « Voisinage » à côté de « Voir » dans chaque champ qui désigne une note, « Voisinage d’une note… » en haut de la fenêtre principale, « Voisinage » dans le visionneur.
- **Dessin** (`gui/voisinage.py`) : la note au centre, ses voisins sur un cercle, dans l’ordre de `memoire voisinage --tsv` ; chaque lien porte son type et une flèche dans son sens, s’arrête au bord de l’étiquette, et a la couleur et le trait de son origine : note entière (bleu `#0072B2`, plein), citation (orange `#D55E00`, pointillé), catégorie (vert `#009E73`, tirets) ; une note écartée est en gris italique. Une légende et une liste textuelle, dans les mêmes couleurs, accompagnent le dessin.
- **Navigation** : clic sur un voisin pour recentrer (après un court délai, pour distinguer le double-clic), « Précédent », « Actualiser » ; double-clic pour ouvrir le visionneur. Lecture seule.
- Essais : `gui/test_voisinage.py` (sans affichage : lecture, disposition, bords, libellés, styles), `gui/test_gui.py` (sous `xvfb-run`).

## 3 quinquies. Plan de travail et terminal (incrément 6.7, PLAN-DE-TRAVAIL.md, TERMINAL.md ; D27, D28 ; R5)

- **Panneaux** : autour de la zone centrale (commandes, formulaire, sortie), quatre zones (haut, bas, gauche, droite) reçoivent des panneaux redimensionnables ; chaque panneau peut flotter dans sa propre fenêtre, ou être masqué, sans que son contenu soit recréé. Bandeau : « Détacher » ou « Ancrer », « Plan » pour une fenêtre flottante (« Toujours au premier plan », « Mettre devant », « Mettre derrière », incrément 6.8), « Zone », ▲ ▼, « Masquer » ; menu « Panneaux » ; disposition conservée dans `~/.config/memoire/plan-de-travail.json` (`gui/plan_de_travail.py` pour le modèle, `gui/panneaux.py` pour la réalisation).
- **Console en mosaïque** (incrément 8.5, CONSOLE.md, D32 ; remplace un panneau par terminal, D29) : tous les terminaux sont les tuiles d’une console, panneau unique du plan de travail. Le bouton rond `>_` de l’en-tête la montre ou la masque ; elle est sur le côté (un terminal à la fois, les autres en pastilles), en pleine page (« Pleine page », « Réduire ») ou en fenêtre (« Détacher »). En pleine page et en fenêtre, une mosaïque montre de un à quatre terminaux ; le point milieu, au croisement des séparations, les redimensionne ensemble (double-clic : parts égales) ; « agrandir » donne les deux tiers à une tuile. « + Nouveau terminal » crée un terminal ; une pastille remet sur la page un terminal en arrière-plan ; page pleine, l’arrivant prend la place du terminal actif. Chaque terminal est nommé par sa commande en cours, sinon `bash — dossier` ; terminaux, mosaïque et seuil de notification se retrouvent à la réouverture (`~/.config/memoire/terminaux.json`). Modèle sans affichage : `gui/mosaique.py` ; réalisation : `gui/console.py`, `gui/gestion_des_terminaux.py`.
- **Terminal** (chaque tuile de la console) : votre shell sous pseudo-terminal ; `memoire` y vise la base de la fenêtre ; saisie avec historique (↑ ↓), Ctrl+C ou « Interrompre » ; saisie masquée quand l’écho est coupé ; taille de la tuile transmise au shell ; pas d’émulateur complet (`TERM=dumb`, pagers désactivés, couleurs retirées). Une commande d’au moins N secondes (10 par défaut, « Notifier après », un seul réglage pour la console) terminée quand le terminal n’est pas regardé déclenche une notification du bureau et un marqueur dans l’en-tête de la tuile (`gui/terminal_moteur.py`, `gui/notification.py`, `gui/terminal.py`).
- Essais : `gui/test_terminal_moteur.py`, `gui/test_notification.py`, `gui/test_plan_de_travail.py`, `gui/test_terminaux.py`, `gui/test_mosaique.py` (sans affichage), `gui/test_gui.py` (sous KWin virtuel ou `xvfb-run`).

## 3 sexies. Apparence (incrément 8.1, APPARENCE.md, D31 ; R15)

- **Fenêtre principale** : un canevas de fond (dégradé) porte l’en-tête (marque ; pastille signature : projet, recherche, piste active ; boutons ronds Terminaux, Panneaux, Projet), le rail des sujets en verre givré et la barre d’état en pastilles ; la carte papier posée dessus reçoit le plan de travail. Au centre de la carte : la liste des actions du sujet choisi, puis l’action (qui défile si elle est longue) et le journal (la zone de sortie), séparés par une séparation déplaçable à poignée visible ; « Agrandir » / « Réduire » dans l’en-tête du journal (8.1c).
- **Rapidité** (8.1c, APPARENCE.md, section 5) : images du thème aplaties sur leur surface, fond en une seule image opaque, survol par zones, grandes formes étirées par Tk.
- **Barre d’état** : piste active, notes à revoir, notes sans appui, terminal en cours (point qui pulse), fichier de base ; relue par `statut` et `branches --tsv` après chaque commande, et toutes les secondes pour les terminaux.
- **Recherche** (Ctrl K ou la pastille) : actions dont le titre contient le texte (sans majuscules ni accents), puis notes trouvées par `memoire chercher` ; Entrée ou clic ouvre l’action, ou la note dans le visionneur.
- **Thème** « memoire » (dérivé de « clam ») : éléments ttk en images de `gui/dessin.py` (Python pur, aucune dépendance) ; action principale orange à grain ; puces « Si besoin » et choix en pastilles qui passent à la ligne (`theme.Flux`) ; zones de texte arrondies (`theme.ZoneDeTexte`) ; terminal sombre arrondi. Polices : Geist, Inter ou Adwaita Sans s’il y en a une ; rien n’est installé.
- **Menus et listes** (8.2a, `gui/menus.py`) : menus, listes déroulantes (`ttk::combobox::Post` remplacée) et suggestions de notes dans une carte arrondie posée dans la fenêtre.
- **Panneau « Aperçu »** (8.2b, `gui/apercu.py`) : « Ce que ça va changer » traduit les commandes que le formulaire lancera (`actions.decrire`), puis l’aperçu du moteur d’une opération critique ; « Dernières étapes » lit `historique` ; montré au premier lancement, à droite.
- **Lecture dans la carte** (8.2c) : « Voir », « Lire », la recherche et le voisinage ouvrent la note dans la carte (« ‹ Retour à l’action » ; « Ouvrir dans une fenêtre ») ; barre de sélection, carte « Relier la sélection », carte de citation ; panneau « Voisinage » qui suit la note lue.
- **Lecture conforme à la maquette** (8.3, APPARENCE.md, section 10) : pendant la lecture, la liste des actions se retire, le panneau Voisinage remplace l’Aperçu et la colonne de droite a au moins 380 pixels ; voisinage dans un encadré clair, quatre voisins aux coins ; barre de sélection en une pastille au bas de l’article ; la pastille signature affiche « Lire › titre ».
- **Complétion** (8.4, APPARENCE.md, section 11 ; les onglets de 8.4 sont remplacés par la console en mosaïque) : Tab complète le dernier mot d’après le dossier du shell ou le `PATH` (dossiers seuls après `cd`), un second Tab montre les possibilités (`terminal_moteur.completer`).
- Fichiers : `gui/dessin.py`, `gui/apparence.py` (sans Tkinter), `gui/theme.py`, `gui/habillage.py`, `gui/menus.py`, `gui/apercu.py`. Essais : `gui/test_dessin.py`, `gui/test_apparence.py` (sans affichage ; INV-LOOK-01, INV-LOOK-02), `gui/test_gui.py` (EssaiDeLHabillage).

## 4. Lancement

`make` (ou `make lancer`) construit l’exécutable en mode release, avec les versions figées par `Cargo.lock` (`--locked`, D14), puis ouvre la fenêtre. `make lancer BASE=/chemin/projet/.memoire/depot.db` choisit la base. Prérequis : Rust (cargo), Python 3 avec Tkinter (paquet `python3-tk` sur Debian ou Ubuntu).
