# Apparence de l’interface — incrément 8.1

Version 0.1 — 14 novembre 2026 — conception de l’apparence d’après la maquette validée par l’utilisateur le 14 novembre 2026 (« magnifique, c’est parfait ») ; décision D31.

## 1. Origine et intention

L’utilisateur a fourni son modèle de présentation (`template/`, export Figma) : un dégradé indigo → crème → rose → violet, une grande carte crème, une pastille signature en dégradé indigo ↔ corail, et de petites pastilles de verre avec un liseré blanc et une ombre courte, décalée. Il en aime l’aspect de « logiciel, technicité douce », surtout les petites pastilles. La maquette (trois planches : écran principal, lecture, pastilles et règles) a traduit ce goût pour un outil où l’on lit et écrit beaucoup ; elle a été validée telle quelle.

Ce document fixe ce qui en passe dans l’interface Tkinter, et ce qui n’y passe pas. Il ne change aucune commande, aucune règle, aucun texte de l’exécutable : seulement l’apparence et la disposition de la fenêtre principale.

## 2. Les quatre couches

1. **L’ambiance** : un dégradé à 128° (indigo `#4B4A9E` → lavande → crème `#E8DCCF` → rose → violet `#9A6BE0`) sur le fond de la fenêtre. Il ne porte que des éléments de verre ou du texte blanc là où il est sombre (la marque, en haut à gauche).
2. **Le papier** : une carte crème `#FFEDE2`, arrondie (rayon 30), posée sur une plaque de verre clair décalée vers la droite et vers le bas. Tout le travail s’y fait : liste des actions, formulaires, journal, panneaux ancrés.
3. **La signature** : une seule pastille en dégradé indigo ↔ corail par fenêtre, en haut : le projet, la recherche, la piste active.
4. **Les pastilles** : petites formes arrondies, teintées, avec une ombre courte et décalée (5, 7, flou 4, encre à 22 %). L’élément choisi devient une pastille de papier qui ressort du verre.

**Règle d’or :** pas de texte sur le dégradé nu (sauf la marque) ; le texte est sur le papier ou sur un verre givré (crème à 60 %), où l’encre reste lisible quelle que soit la couleur dessous.

## 3. Fenêtre principale

```
 ● memoire        ( RTOS-2026 ▾ │ ⌕ Chercher une note, une action…  Ctrl K │ ● piste principale ▾ )     (>_) (▦) (⚙)
 ┌───────────┐  ┌──────────────────────────────────────────────────────────────────────────────┐
 │ ✎ Écrire  │  │ ÉCRIRE               │ Nouvelle note                                        │
 │ ⇄ Relier  │  │ ▸ Nouvelle note      │ Une idée, une solution technique ou une note libre.   │
 │ ⚖ Juger   │  │   Nouvel échange…    │ [questions de l’action, sections « Si besoin »]       │
 │ ▤ Lire    │  │   Citer un passage   │ ( Faire ) ( Ne rien faire )                           │
 │ ‹› Code   │  │   …                  │ JOURNAL                                               │
 │ ⑂ Pistes  │  │                      │ $ memoire note creer …                                │
 │ ⚑ Où…  2  │  └──────────────────────────────────────────────────────────────────────────────┘
 │ ▣ Projet  │        ( ● piste principale │ 2 notes à revoir │ terminal · cargo test )   ( depot.db )
 └───────────┘
```

- **En-tête (sur le dégradé).** À gauche, la marque. Au centre, la pastille signature : le nom du projet (dossier qui contient `.memoire`, sinon le nom du fichier de base ; un clic propose « Changer de projet… »), la recherche (pastille de verre teinté, texte blanc), la piste active (un clic ouvre « Aller sur une piste »). À droite, trois boutons ronds de verre : Terminaux (le menu « Terminaux ▾ »), Panneaux (le menu « Panneaux ▾ »), Projet (Changer de projet…, Où en est le projet ?).
- **Rail des sujets (verre givré, à gauche).** Les sujets du catalogue (ACTIONS.md, section 4) avec un pictogramme, puis « Toutes les commandes — mode expert ». Le sujet choisi est une pastille de papier ; « Où j’en suis » porte le nombre de notes à revoir quand il n’est pas nul.
- **Carte papier.** À gauche, les actions du sujet choisi (la choisie : pastille blanche à ombre courte) ; à droite, le titre, l’aide, le formulaire de l’action, puis le **journal** (la zone de sortie actuelle, en chasse fixe, avec « Effacer ») ; l’action défile si elle est longue, et la séparation entre l’action et le journal se déplace (poignée visible) ; « Agrandir » donne au journal les trois quarts de la colonne, « Réduire » lui rend sa taille d’avant (8.1c, demande de l’utilisateur). Le plan de travail (PLAN-DE-TRAVAIL.md) est dans la carte : ses zones d’ancrage entourent cette partie centrale ; les panneaux flottants restent des fenêtres.
- **Barre d’état (pastilles de verre, sous la carte).** Piste active ; nombre de notes à revoir (ou « rien à revoir ») ; notes sans appui s’il y en a ; terminal en cours (nom, point qui pulse) quand une commande tourne. À droite, le fichier de base. Chaque pastille ouvre ce qu’elle nomme : « Aller sur une piste », « Où en est le projet ? », le panneau du terminal. Elle est relue après chaque commande, par `statut` et `branches --tsv` seulement (lecture).
- **Recherche (Ctrl K, ou clic sur la pastille).** Une carte s’ouvre sous l’en-tête : un champ, puis les actions dont le titre contient le texte (sans tenir compte des majuscules ni des accents) et, dès deux lettres, les notes trouvées par `memoire chercher` (« titre — rôle · id8 »). ↑ ↓ Entrée ou clic : une action s’ouvre, une note s’ouvre dans le visionneur. Échap ferme.

Ce qui disparaît de l’ancienne barre du haut, sans fonction perdue : le champ « Base » (pastille du projet, réglage par « Changer de projet… » ; la variable `base` reste), « Statut » (barre d’état et « Où en est le projet ? »), « Voir une note… » et « Voisinage d’une note… » (recherche, rail « Lire » ; les méthodes restent).

## 4. Composants (sur le papier)

| Composant | Apparence |
|---|---|
| Bouton | pastille au liseré d’encre à 16 % ; survol : blanche à ombre courte |
| Action principale (« Faire », « Confirmer », « Adopter », « Citer la sélection ») | pastille orange `#F89C39` à grain, texte encre, ombre décalée ; une seule par écran |
| Champ, liste déroulante, zone de texte | rayon 14, fond `#FFF8F4`, liseré encre à 14 % ; en saisie : liseré indigo et halo |
| Case « Si besoin » | puce : contour et « + » ; cochée : encre pleine, texte crème et « ✓ » |
| Choix (questions à choix, désaccords) | pastilles côte à côte, trois par rangée ; choisie : blanche, ombre courte, point indigo |
| Cadre (« Avant de continuer », « Et ensuite ? », « Le plan ») | carte arrondie, liseré léger, titre en petites capitales espacées |
| Barre de défilement | fine, pouce arrondi, sans flèches |
| Terminal | panneau sombre (encre indigo `#1D1A33`), texte clair, saisie et boutons sombres |

Typographie : Geist, sinon Inter, sinon Adwaita Sans (dérivée d’Inter), sinon la police par défaut ; chasse fixe : Geist Mono, JetBrains Mono, Adwaita Mono, Fira Code, sinon la police fixe par défaut. Titres 20 points gras ; texte 10 ; étiquettes 8 en capitales espacées ; identifiants en chasse fixe. Aucune police n’est installée par le logiciel.

Couleurs de sens : celles de la vue locale (Okabe-Ito : bleu `#0072B2`, vermillon `#D55E00`, vert `#009E73`) et l’ambre `#E07B00` pour « à revoir » ; elles ne servent jamais de décor, et l’état se lit aussi au mot.

## 5. Ce que Tkinter permet, et comment

- Le fond, l’en-tête, le rail et la barre d’état sont dessinés sur un canevas qui occupe la fenêtre ; la carte et ses widgets sont posés dessus. Le canevas compose les images à transparence : le verre est une image (teinte, liseré, ombre) posée sur le dégradé. Le dégradé étant lisse, le flou d’arrière-plan n’y changerait rien : il n’est pas imité.
- Les formes (pastilles, cartes, champs, pictogrammes) sont des images PNG calculées en Python pur (`gui/dessin.py`, sans Tkinter ni dépendance) : distance signée à un rectangle arrondi, bord adouci d’un pixel, ombre adoucie ; les rangées identiques ne sont calculées qu’une fois. Elles sont redessinées, en différé, quand la fenêtre change de taille.
- Les widgets ttk reçoivent un thème « memoire » (dérivé de « clam ») dont les éléments sont ces images, découpées en neuf (coins fixes, bords et centre répétés).
- **Rapidité (8.1c).** Tk mélange une image à transparence avec ce qui est dessous à chaque dessin, en relisant l’écran ; sous XWayland, un champ ainsi dessiné coûte environ huit fois plus qu’avec une image opaque. Donc : chaque image d’élément du thème est aplatie une fois sur sa surface (papier, champ, terminal) ; le fond de la fenêtre est une seule image opaque, dont la couche statique (dégradé, plaque, carte, rail, signature, pictogrammes) n’est recomposée qu’au redimensionnement et la couche d’état (sujet choisi, badge, piste, barre d’état) à chaque relecture ; survol et clics passent par des zones calculées au dessin ; les grandes formes sont étirées par Tk à partir d’une petite forme ; le redimensionnement est regroupé (120 ms). Mesures sous KWin virtuel : frappe 5,3 → 0,6 ms, survol 18 → 0,5 ms, réaffichage complet 65 → 15 ms, redimensionnement 362 → 133 ms.
- **Ce qui n’est pas repris :** le vrai flou derrière le verre (impossible en Tkinter, et inutile sur un dégradé lisse), les animations autres que le point qui pulse, les coins arrondis des fenêtres flottantes (gérés par le gestionnaire de fenêtres).

## 6. Ce qui ne change pas

- Aucune commande, aucune sortie, aucun code de retour ; l’interface reste sans règle (D17, D30).
- Le catalogue des actions, leurs questions, sections et suites (ACTIONS.md) ; le mode expert ; le visionneur, la vue locale, le plan de travail et les terminaux gardent leur comportement, et reçoivent le thème.
- Les essais graphiques existants restent valables, sauf celui qui parcourait l’arbre du menu : le menu devient le rail et la liste des actions (contrat modifié en 8.1b, même vérification : tous les sujets, toutes les actions, les 48 commandes du mode expert).

## 7. Invariants

- **INV-LOOK-01** : le dessin est déterministe et toujours valide : mêmes paramètres, mêmes octets ; toute image est un PNG RGBA bien formé, aux dimensions annoncées.
- **INV-LOOK-02** : tout couple texte / surface déclaré par le thème atteint un contraste de 4,5:1 (3:1 pour un texte de 18 points et plus) ; l’encre reste lisible sur le verre givré posé sur n’importe quel point du dégradé.
- **INV-UI-01** (prolongé) : chaque action reste atteignable, par le rail et la liste des actions, ou par le mode expert.

## 8. Incréments

- **8.1a** : `gui/dessin.py` (images), `gui/apparence.py` (couleurs, contrastes, polices, lecture de l’état ; sans Tkinter), `gui/theme.py` (polices, thème ttk, zones de texte) appliqué à toutes les fenêtres ; terminal sombre.
- **8.1b** : composition de la fenêtre principale (fond, en-tête, rail, carte, barre d’état, recherche) ; le plan de travail passe dans la carte.

## 9. Incrément 8.2 — rattraper la maquette (validé le 14 novembre 2026)

Validé par l’utilisateur (« je te laisse commencer le travail »). Les deux choix laissés ouverts prennent la proposition : l’aperçu est un **panneau** du plan de travail ; la lecture se fait **dans la carte**, la fenêtre restant en option.

L’utilisateur a relevé trois écarts avec la maquette validée, et autorise à modifier l’affichage (jamais le moteur) pour s’en rapprocher.

### 9.1 Les écarts

1. **Menus et listes déroulantes** à l’ancien style : menus Tk natifs (Panneaux, Terminaux, Projet, Plan et Zone des bandeaux), liste des `Combobox` (fenêtre ttk avec liste et barre classiques), suggestions de notes (liste posée sous le champ), réglage numérique du terminal.
2. **Colonne de droite de l’écran principal absente** : « Ce que ça va changer » n’apparaît qu’après « Faire », et seulement pour une opération critique ; « Dernières étapes » n’existe pas.
3. **Lecture** : le visionneur et la vue locale sont des fenêtres séparées, à l’ancien style (grand cadre « Relier la sélection », graphe à rectangles) ; la maquette les met dans la carte : l’article à gauche, le voisinage et le terminal à droite, une barre de sélection, une carte de citation.

### 9.2 Ce que le moteur donne (il n’est pas modifié)

- **Aperçu sans écriture** : seulement pour une opération critique (code 4 sans `--oui`). Pour les autres, rien. « Ce que ça va changer » en direct est donc la **traduction des commandes que le formulaire va lancer** (le catalogue sait déjà les construire, `commandes_de`), marquée « prévu » ; pour une opération critique, l’aperçu du moteur (nouvelle piste, notes à revoir) s’y ajoute après « Faire », comme aujourd’hui.
- **Étapes** : `historique` donne l’identifiant, la commande et le motif, sans date ni objet. « Dernières étapes » montre la commande dans le vocabulaire de l’interface, son motif et l’id8 ; pas de « il y a 2 min ».
- `source`, `citations`, `voisinage --tsv`, `chercher` suffisent pour la lecture.

### 9.3 Proposition, en trois étapes

**8.2a — Menus et listes (un composant, `gui/menus.py`).**
- **Carte de menu** : carte papier arrondie à ombre courte, entrées de 34 px, survol en pastille, coches, séparateurs. Elle offre ce que le code utilise de `tk.Menu` (`add_command`, `add_checkbutton`, `add_separator`, `delete`, `entrycget`, `index`, `type`, `tk_popup`) : plan de travail, terminaux et essais ne changent pas.
- **Réalisation** : un cadre posé dans la fenêtre, sur un fond copié de l’image du fond, donc coins arrondis et ombre réels. Limite : la carte ne peut pas déborder de la fenêtre (Tk n’a pas de fenêtre à coins transparents sous XWayland) ; si la place manque, elle s’ouvre vers le haut ou se décale.
- **Listes des `Combobox`** : même carte (clic, ↓, Entrée, Échap) ; la classe `TCombobox` reste (essais). **Suggestions de notes** (RETROUVER.md) dans la même carte : « nom — rôle · id8 », avec le point de couleur du rôle.
- Boutons « Plan » et « Zone » des bandeaux : même carte. Réglage « Notifier après » : champ et boutons − / + en pastilles.

**8.2b — Colonne « Aperçu » de l’écran principal.**
- Un panneau du plan de travail, « Aperçu », ancré à droite par défaut, au-dessus des terminaux ; masquable et déplaçable comme les autres.
- **Ce que ça va changer** : carte mise à jour pendant la saisie (« + 1 idée créée : titre », « → s’appuie sur : X », « ◇ rangée dans : Y », « ✕ réfutée : Z »…), d’après les commandes qui seront lancées ; traduction ajoutée au catalogue (`actions.py`, sans Tkinter, testée contre l’aide figée). Après « Faire » d’une opération critique : l’aperçu du moteur ; « Confirmer » et « Ne rien faire » restent dans le formulaire.
- **Dernières étapes** : les cinq dernières (`historique --nombre 5`), relues après chaque commande comme la barre d’état ; un clic ouvre « Revenir à une étape », pré-rempli.

**8.2c — La lecture dans la carte.**
- « Lire une note », la recherche, « Voir » et un clic dans le voisinage ouvrent la note **dans la carte** : la vue Lecture remplace la vue Action, le rail passe sur « Lire », un lien « ‹ Retour à l’action » ramène au formulaire. « Ouvrir dans une fenêtre » garde la fenêtre séparée (deux notes côte à côte).
- **En-tête** : pastille du genre (« Échange avec une IA »), id8 en chasse fixe, bascule « Rendu | Brut » en pastilles, versions en pastilles (0 · 1 · 2).
- **Échange** : la question dans une carte crème plus foncée (« QUESTION »), puis « RÉPONSE » ; texte plus grand, interligne large, largeur de lecture limitée (environ 70 caractères).
- **Passages cités** surlignés orange doux, soulignés vermillon ; un clic ouvre la **carte de citation** : identifiant, octets, « à jour » ou « source révisée ».
- **Barre de sélection** flottante sous le texte choisi : « Citer la sélection » (orange), « Soutient… », « Inspire… », « Classe dans… », « Lien… », « Nouvelle note », « Doute à suivre » ; chacun ouvre une petite carte avec son champ (suggestions par nom) ; « Citer » envoie le tout en une opération (CENTRE.md inchangé, INV-WORK-01). Le grand cadre « Relier la sélection » disparaît.
- **Panneau « Voisinage »** (plan de travail, droite) qui suit la note lue : nœuds en pastilles (centre en encre pleine ; voisins blancs au liseré de la couleur de leur origine ; note écartée en gris italique), traits colorés et fléchés avec leur type, légende en puces ; clic : lire la voisine ; « Agrandir » ouvre la vue locale complète (fenêtre actuelle, restylée).
- Le terminal se range sous le voisinage (déjà possible avec le plan de travail).

### 9.4 Ce qui change dans le contrat

- 8.2a : aucun essai existant ne change ; nouveaux essais : carte de menu (entrées, coche, clavier), liste d’une `Combobox`, suggestions.
- 8.2b : nouveaux essais : traduction des commandes (sans affichage, contre l’aide figée), aperçu qui suit le formulaire, étapes relues après une commande.
- 8.2c : **Contrat modifié** pour EssaiDuVisionneur : « Voir » ouvre la lecture dans la carte, plus une fenêtre (la fenêtre reste par « Ouvrir dans une fenêtre ») ; nouveaux essais : barre de sélection (citer avec liens en une opération), carte de citation, panneau Voisinage. INV-VIEW-01, INV-VIEW-02 et INV-WORK-01 restent vérifiés tels quels.

### 9.5 Ordre et livrables

**Fait le 14 novembre 2026** : 8.2a (`gui/menus.py`), 8.2b (`gui/apercu.py`, `actions.decrire`), 8.2c (`gui/visionneur.py`, panneau Voisinage dans `gui/voisinage.py`). Écarts avec la proposition : le bandeau d’un panneau tient sur une rangée (« Détacher » et un menu « ⋯ » : ordre de la fenêtre, zone, monter, descendre, masquer) ; le seuil de notification du terminal se règle par − / + ; le panneau Voisinage s’ouvre à la première lecture de la session, puis garde la place que l’utilisateur lui donne.


8.2a, puis 8.2b, puis 8.2c. Pour chacun : contrat, code, essais sous KWin virtuel, captures comparées à la maquette ; chaque étape est montrée avant la suivante.

## 10. Incrément 8.3 — la lecture conforme à la maquette (14 novembre 2026)

Demande de l’utilisateur : la lecture d’une note et de ses plus proches voisins n’est pas encore conforme à la planche « Lecture » de la maquette. Écarts et corrections (affichage seul, moteur inchangé) :

1. **Disposition** : pendant la lecture, la colonne des actions se retire (l’article prend la place) ; à droite, le panneau « Voisinage » remplace le panneau « Aperçu », qui ne concerne que les actions. Au retour à l’action, l’inverse. Le choix de l’utilisateur est respecté : un panneau qu’il a masqué dans une vue le reste dans cette vue ; la disposition enregistrée est toujours celle de la vue des actions.
2. **Voisinage** : une carte claire « VOISINAGE » avec « Agrandir » ; jusqu’à quatre voisins aux quatre coins (au-delà, sur une ellipse), libellés sur deux lignes au plus ; la note lue au centre en pastille d’encre ; légende des trois origines dans la carte.
3. **Barre de sélection** : une seule pastille, posée en bas de l’article quand du texte est choisi : « Sélection : », « Citer la sélection », « Soutient… », « Inspire… », « Nouvelle note… », « ⋯ » (classe dans, lien, doute à suivre). Elle ne couvre plus le texte.
4. **En-tête** : pastille du genre et identifiant à gauche, bascule « Rendu | Brut » groupée à droite ; « Voisinage » n’y figure plus dans la carte (le panneau le montre).
5. **Pastille signature** : pendant la lecture, la recherche affiche « Lire › titre de la note ».

Essais : contrat modifié pour la barre de sélection (ses boutons) ; nouveaux essais pour la disposition des deux vues, la carte du voisinage et la disposition aux coins, la pastille signature.

## 11. Incrément 8.4 — terminal en onglets et complétion (14 novembre 2026)

Demande de l’utilisateur : le terminal en onglets de la maquette, et la complétion par Tab (double Tab pour voir les possibilités, par exemple après `cd`).

- **Onglets.** Le modèle ne change pas : un panneau par terminal (TERMINAUX.md, D29). En haut de chaque terminal, une rangée d’onglets nomme tous les terminaux (le nom de D29 ; « ● » devant celui qui exécute une commande), puis « + ». Choisir un onglet met ce terminal à la place de celui qu’on regarde (même zone, même rang) et masque ce dernier ; « + » ouvre un terminal à cette même place. « Nouveau terminal » du menu garde son effet (un terminal de plus, sous les autres), et un terminal peut toujours être détaché.
- **Complétion.** Le terminal n’est pas un émulateur (TERMINAL.md, D27) : la ligne se tape dans un champ, le shell ne la voit qu’à l’envoi. La complétion est donc calculée par l’interface, sans rien envoyer au shell : le dernier mot de la ligne est complété d’après le dossier courant du shell (déjà suivi), ou d’après les commandes du `PATH` pour le premier mot ; après `cd`, `pushd` et `rmdir`, seuls les dossiers sont proposés ; un dossier se termine par « / », une espace s’écrit « \ ». **Tab** complète ce qui est sans ambiguïté ; un **second Tab** montre les possibilités dans une carte au-dessus de la saisie (↑ ↓ Entrée, clic, Échap). Limite : ni variables, ni alias, ni complétions programmées de bash.
- Essais : `gui/test_terminal_moteur.py` (complétion, sans affichage), `gui/test_gui.py` (onglets, Tab et double Tab).
- **Retouches (8.4b, retours de l’utilisateur).** Une seule carte de menu ouverte à la fois, sans capture des clics : un clic ailleurs la ferme et agit normalement. Chaque onglet a sa croix (fermer le terminal ; si c’est celui qu’on regarde, un voisin prend sa place) ; « Masquer » est dans la rangée d’onglets ; la rangée est mise à jour aussitôt. La carte des possibilités a le fond du terminal, des coins droits et s’ouvre de préférence au-dessus de la saisie.

## 12. Incrément 8.5 — la console en mosaïque (16 novembre 2026)

Demande de l’utilisateur : plusieurs terminaux sur une même page, un point milieu pour les redimensionner ensemble, créer ou rouvrir un terminal d’un geste. Conception dans CONSOLE.md (D32), validée sur maquette interactive. Elle remplace les onglets de la section 11 (la complétion par Tab reste) et la ligne « Terminal » de la section 4 se lit désormais : tuiles sombres arrondies sur le papier, séparées par 12 pixels de papier, liseré lavande `#8F88F2` pour la tuile active, point milieu en pastille blanche à ombre courte.

**Règle de réalisation tirée de 8.5b (CONSOLE.md, section 11), valable pour toute l’interface :** un élément image du thème posé sur une grande surface a un milieu large (ttk le répète pour remplir : un milieu de quelques pixels coûte des milliers de copies par redessin) ; un cadre qui change souvent de taille et dont le fond diffère de celui de son parent est un cadre classique à sa couleur, pas un cadre ttk (le serveur peint le fond du premier ancêtre classique sous un widget ttk découvert).
