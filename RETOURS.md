# Retours d’usage

Registre des retours de l’utilisateur après usage réel, et de ce qu’ils deviennent. Chaque développement qui en découle renvoie à son numéro.

## Retours du 25 octobre 2026 (interface v0.4.0-alpha.3)

| N° | Retour | Suite |
|---|---|---|
| R1 | Créer une note et la lier dans le même geste | Fait et vérifié (incrément 5.3, v0.5.0-alpha.3) : `note creer` avec `--classe-dans`, `--depend-de`, `--developpe-a-partir-de`, `--lien` (LOT.md, D22) ; une seule opération, un seul `annuler` ; autocomplétion à plusieurs notes dans l’interface |
| R2 | Visionneur de note ergonomique, rendu Markdown, centre de travail : surligner, relier les passages | Visionneur fait (incrément 6.1, v0.6.0-alpha.1, VISIONNEUR.md, D25) : rendu Markdown, surlignage des citations, citation par sélection, versions d’une note ; centre de travail fait (incrément 6.2, v0.6.0-alpha.2, CENTRE.md) : citer et relier une sélection en un geste (soutient, inspire, classe dans, lien, note nouvelle, réserve) |
| R3 | Choisir une note par son nom, avec autocomplétion, plutôt que par son identifiant | Fait et vérifié (incrément 4.4, v0.4.0-alpha.4) : commande `chercher`, autocomplétion des champs de note |
| R4 | Notes rangées dans des répertoires par catégorie | Abandonné par l’utilisateur le 25 octobre 2026 |
| R5 | Mini-shell intégré (cd, ls, cat, commandes du logiciel), logiciel tout-en-un | Précisé le 10 novembre 2026 : le shell du système dans le logiciel, flottant ou incrusté, redimensionnable, avec notification de fin de compilation, dans un plan de travail modulaire (à la manière de Vivado). Fait (incrément 6.7, v0.6.0-alpha.7, TERMINAL.md, PLAN-DE-TRAVAIL.md, D27, D28) : panneau Terminal (bash sous pseudo-terminal, `memoire` sur la base de la fenêtre, notification du bureau), panneaux ancrables ou flottants, disposition conservée ; apparence (direction Liquid Glass) à concevoir avec un designer d’interface |
| R6 | Note devenue orpheline après une réfutation puis un acquittement : la ranger dans une liste au même titre que « à réexaminer » | Vue calculée « Orphelines », pas une catégorie au sens de D7 ; définition retenue : tous les appuis déclarés sont écartés et les causes acquittées ; la note sort des ressources actuelles. Fait (incrément 5.5, v0.5.0-alpha.5, D24) : `Graph::orphans`, section « Orphelines » de `statut` |
| R7 | Vue locale du voisinage d’une note, couleur selon l’origine du lien (citation, note entière, catégorie) | Fait (incrément 6.6, v0.6.0-alpha.6, VOISINAGE.md) : commande `voisinage`, fenêtre « Voisinage » (couleur et trait par origine, recentrer d’un clic, visionneur d’un double-clic) |
| R8 | Surligner crée une « citation », rôle à part entière, née d’une sélection dans le visionneur et non de `note creer` | Modèle fait (incrément 5.5, v0.5.0-alpha.5, D18, D24) : rôle citation, `citer` dans un échange ou une note, version citée, « (source révisée) » ; citation par sélection dans le visionneur faite (incrément 6.1) |
| R9 | Bouton de sauvegarde manuelle de la base | Fait et vérifié (incrément 4.4, v0.4.0-alpha.4) : commande et bouton `sauvegarder` |
| R10 | Classer un échange, lui rattacher des notes, faire dépendre un échange d’un autre, la citation d’un échange créant un lien de dépendance avec lui | Classement : déjà possible. Le reste : décision D18, faite (incrément 5.5, v0.5.0-alpha.5) : dépendance d’une note envers un échange, échange développé d’un autre, citation reliée à sa source, `retirer` un échange |

## Retours du 10 novembre 2026 (terminal intégré, v0.6.0-alpha.7)

| N° | Retour | Suite |
|---|---|---|
| R11 | Détaché, le terminal passe derrière le logiciel dès qu’on y clique ; « que ce soit moi qui décide (mettre devant, mettre derrière) » | Fait (incrément 6.8, v0.6.0-alpha.8, PLAN-DE-TRAVAIL.md, section 10) : menu « Plan » d’une fenêtre flottante (toujours au premier plan, mettre devant, mettre derrière), vérifié sous KWin virtuel |
| R12 | Plusieurs terminaux, un menu et un panneau plaçable qui les liste, nommés par la commande qu’ils exécutent, mis devant ou derrière facilement | Fait (incrément 6.9, v0.6.0-alpha.9, TERMINAUX.md, D29) : un panneau par terminal, noms en direct, panneau « Terminaux », menu « Terminaux ▾ », conservation entre deux sessions |
| R13 | Menus difficiles à suivre et à réapprendre : vocabulaire simple, création par étapes à cocher, système en graphe, surtout réfuter et fusionner ; « adopter une piste » plutôt qu’« intégrer » | Fait (incrément 7.1, v0.7.0-alpha.1, ACTIONS.md, D30) : menu par sujet, questions courtes, sections à cocher, « Ce que ça va changer », « Et ensuite ? », parcours « Réfuter » et « Adopter une piste », mode expert |
| R14 | Retrouver une note par son nom plutôt que par son identifiant (boutons du haut) ; donner le code et trouver la note associée | Fait (incrément 7.2, v0.7.0-alpha.2, RETROUVER.md) : suggestions pendant la frappe, notes affichées « nom · id8 », boutons du haut vers la recherche par nom ; « Que désigne cette marque ? » toutes syntaxes ; « Quelles notes concernent ce code ? » (fichier, ligne ou extrait) |

## Retours du 14 novembre 2026 (passage au design, v0.7.0-alpha.2)

| N° | Retour | Suite |
|---|---|---|
| R15 | Retrouver dans le logiciel l’aspect « logiciel, technicité douce » du modèle de présentation de l’utilisateur (`template/`), surtout les petites pastilles de verre à ombre ; maquette HTML validée (« magnifique, c’est parfait ») | Fait (incrément 8.1, APPARENCE.md, D31) : thème « memoire » dessiné en Python pur, fenêtre principale recomposée (en-tête et pastille signature, rail des sujets, carte papier, barre d’état en pastilles, recherche Ctrl K), terminal sombre arrondi |
| R16 | Menus déroulants restés à l’ancien style ; il manque « Ce que ça va changer » et « Dernières étapes » à droite, et la lecture et le graphe local ne suivent pas la maquette | Fait (incrément 8.2, APPARENCE.md, section 9) : carte de menu commune, panneau Aperçu, lecture dans la carte avec barre de sélection, carte de citation et panneau Voisinage |
| R17 | Pouvoir agrandir le journal ; interface lente depuis l’apparence | Fait (8.1c) : poignée visible et « Agrandir » ; images aplaties, fond en une image opaque : « super fluide maintenant » (retour de l’utilisateur) |
| R18 | La lecture d’une note et de ses plus proches voisins n’est pas encore conforme à la maquette | Fait (incrément 8.3, APPARENCE.md, section 10) ; reste : le terminal en onglets de la maquette |
| R19 | Terminal en onglets comme la maquette ; complétion par Tab et double Tab (par exemple après `cd`) | Fait (incrément 8.4, APPARENCE.md, section 11) : onglets sur le modèle d’un panneau par terminal ; complétion calculée par l’interface (fichiers, dossiers, commandes du PATH), sans variables ni alias |
| R20 | Possibilités de `cd` sur fond beige au lieu du fond du terminal ; fermer un onglet est hasardeux ; les menus se superposent ; masquer un terminal plus facilement | Fait (8.4b, APPARENCE.md, section 11) |
| R21 | Améliorer la console : plusieurs terminaux sur une même page, créer ou rouvrir un terminal d’un geste, les agrandir facilement (point milieu qui redimensionne les autres) ; garder la console sur le côté pour taper vite et la pleine page pour travailler ; une action reste simple, sans combinaison de menus | Fait (incrément 8.5, CONSOLE.md, D32) : console unique en mosaïque (un à quatre terminaux, point milieu, agrandir), sur le côté, en pleine page ou en fenêtre ; pastilles des terminaux en arrière-plan ; un clic par action |
| R22 | Redimensionner les terminaux fait des éclairs beiges sur les icônes des tuiles, et le geste est saccadé ; de petits détails ne sont pas parfaits | Fait (8.5b, CONSOLE.md, section 11) : tuiles en cadres classiques au fond du terminal, grandes surfaces du thème non pavées (pas du glisser mesuré avant et après), détails de la console |
