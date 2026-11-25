# Visionneur : lecture rendue, surlignage, citation par sélection — conception de l’incrément 6.1

Version 0.1 — 6 novembre 2026 — **validée par l’utilisateur le 6 novembre 2026** (décision D25) ; **implémentée** (6.1a ligne de commande, 6.1b rendu, 6.1c fenêtre ; `v0.6.0-alpha.1`). Précision de syntaxe à la mise en œuvre : la plage de `citer` s’écrit `--plage A-B` (comme `ancrer --lignes A-B`), pour rester dans la grammaire que l’interface lit (INTERFACE.md, section 2) ; `--version N` vaut aussi pour `citer`. Première étape de la phase 6 (REPRISE.md, section 7, point 5). Répond à R2 (« visionneur de note ergonomique, rendu Markdown, centre de travail : surligner, relier les passages ») et termine R8 (« surligner crée une citation, née d’une sélection dans le visionneur »). Rend testable INV-SOURCE-02 (« les surlignages restent liés à la source originale après changement du rendu Markdown »). Regroupe en section 9 les points à valider, dont le choix de la bibliothèque graphique (D17 ou une remplaçante).

---

## 1. Ce que l’utilisateur obtient

- Un bouton **« Voir »** à côté de chaque champ qui désigne une note, et dans la fenêtre principale. Il ouvre un **visionneur**, une fenêtre à part.
- Le visionneur montre :
  - pour un **échange**, la question puis la réponse, chacune rendue ;
  - pour une **note**, son texte rendu à la version courante, avec une liste pour afficher une version plus ancienne.
- **Rendu Markdown** des textes collés depuis une IA : titres, gras, italique, code en ligne et blocs de code, listes, citations `>`, liens (montrés comme texte), séparateurs. Un bouton **« Brut »** montre le texte exact, octet pour octet. Tout ce que le rendu ne sait pas faire (tableaux, images, formules) s’affiche tel quel.
- Les **citations existantes** sont surlignées. Un clic sur un surlignage affiche la citation : son identifiant, son statut, « source révisée » s’il y a lieu.
- **Sélectionner** du texte (rendu ou brut), puis cliquer **« Citer la sélection »**, crée la citation de la plage exacte. Comme `citer` (D24), cela crée aussi la relation vers la source. L’opération est critique ou non selon les mêmes règles, avec la même confirmation.
- Le visionneur se met à jour après chaque citation.

---

## 2. Choix de la bibliothèque (section 9, point 1)

Évaluation honnête de Tkinter (D17) pour cet usage, avec Python 3.14 et Tk 9.0 sur la machine de développement :

| Besoin | Tkinter (`Text`, étiquettes) | PySide6 (Qt) | Page web locale |
|---|---|---|---|
| Rendu des titres, gras, italique, code, listes, citations | oui, par étiquettes de style | oui (`setMarkdown`) | oui |
| Tableaux, images | non (texte brut) | partiel | oui |
| **Correspondance exacte entre texte affiché et octets de la source** (INV-SOURCE-02) | oui : nous écrivons le rendu, donc nous tenons la table de correspondance | non : le rendu de Qt perd les positions de la source, il faudrait le réécrire | à écrire nous-mêmes en JavaScript |
| Sélection, surlignage, clic sur une zone | oui (`tag_add`, `tag_bind`, `sel`) | oui | oui |
| Dépendances (D14) | **aucune** | plus de 100 Mo, roues binaires à auditer à chaque mise à jour | serveur HTTP à écrire, navigateur |
| Essais automatiques | oui, déjà sous `xvfb-run` | oui, plus lourd | à monter |

**Recommandation : garder Tkinter**, avec un moteur de rendu écrit pour nous. Le besoin central, citer exactement ce que l’on voit, impose de toute façon de tenir la correspondance entre l’affichage et la source. Aucune bibliothèque ne la fournit ; une fois qu’on l’écrit, Tkinter suffit. Les tableaux et les images restent en texte brut, limite assumée. Si l’usage quotidien montre que c’est insuffisant, PySide6 serait l’alternative, avec une décision dédiée et l’audit D14. D17 est **confirmée** par une décision D25, sans être remplacée.

---

## 3. Ligne de commande : ce que le visionneur demande à l’exécutable

L’interface ne contient aucune règle (D17) : tout passe par l’exécutable. Trois ajouts :

| Commande | Sortie |
|---|---|
| `source NOTE [--partie question\|reponse] [--version N]` | le texte exact de la source, octet pour octet, sans rien ajouter : la partie d’un échange (réponse par défaut), ou une version d’une note (la courante par défaut) |
| `citations NOTE [--partie question\|reponse] [--version N]` | une ligne par citation de cette source à cette partie ou version : `ID<tab>DEBUT<tab>FIN<tab>STATUT`, en octets UTF-8, par début croissant ; STATUT est le verdict, suivi de `,source-revisee` le cas échéant |
| `citer NOTE (--texte X [--occurrence N] \| --debut A --fin B) …` | `citer` accepte aussi une plage en octets, l’un ou l’autre ; une plage invalide (coupant un caractère, hors du texte) est un refus du domaine (code 1) |

`--version` vaut pour une note : 0 pour le texte de création, *k* pour la *k*-ième révision ; pour un échange, il est refusé (code 2). La version d’une citation passée par `--debut/--fin` est celle demandée, et la courante par défaut. Citer une version ancienne est permis par le noyau (CITATIONS.md, section 3).

---

## 4. Rendu : correspondance entre l’affichage et la source

Le module `gui/rendu.py`, en Python pur sans Tkinter, est testé comme `schema.py`. Il prend le texte brut et rend :
- le texte affiché ;
- des **segments de style** : plages affichées et style de chacune ;
- une **table de correspondance** : pour chaque caractère affiché, sa position en octets dans la source.

**Règles :**
1. Chaque caractère affiché provient d’un caractère de la source, à la même valeur : le rendu masque des marques (`**`, `#`, `` ` ``, `>`, puces), il n’en invente aucune. Seules exceptions, les puces de liste et les séparateurs, affichés par un caractère dédié et rattachés à la position de leur marque dans la source.
2. Une **sélection affichée** devient la plus petite plage de la source qui contient tous ses caractères. Les marques internes sont comprises : sélectionner « a b c » dans « a **b** c » cite `a **b** c`. Sélectionner « b » cite `b`.
3. Une **citation** de la source se surligne sur les caractères affichés qui proviennent de sa plage, quelles que soient les règles de rendu (INV-SOURCE-02).
4. Hors du sous-ensemble reconnu, le texte s’affiche tel quel, sans perte.

**Sous-ensemble reconnu :**
- titres `#` à `######` ;
- `**gras**`, `*italique*` et `_italique_` ;
- `` `code` `` ;
- blocs de code entre clôtures de trois accents graves (police à chasse fixe, sans autre rendu) ;
- listes `-`, `*`, `1.` ;
- citations `>` ;
- liens `[texte](adresse)`, montrés comme « texte » ;
- séparateurs `---`.

**Essais du module** : correspondance vérifiée pour chaque caractère affiché, plages des sélections et des citations, textes réels de `fixtures/rtos-2026-10/`, caractères accentués et emoji, Markdown mal formé (rendu tel quel, jamais d’exception).

---

## 5. Fenêtre

- **Fenêtre principale** : bouton « Voir » à droite de chaque champ de note (y compris l’autocomplétion), et bouton « Voir une note… », qui ouvre une liste par nom.
- **Visionneur** : titre et rôle de la note ; pour un échange, deux zones (question, réponse) ; pour une note, une zone et la liste des versions ; boutons « Rendu / Brut », « Citer la sélection », « Fermer » ; zone d’état pour la citation cliquée ou pour le compte rendu de la dernière commande.
- **Confirmation** : une citation critique affiche l’aperçu et un bouton « Confirmer », comme dans la fenêtre principale (code 4, puis `--oui`).

---

## 6. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-SOURCE-02 | (existant, P) Les surlignages restent liés à la source originale après changement du rendu Markdown |
| INV-VIEW-01 | Tout caractère affiché par le visionneur provient d’une position de la source ; une sélection se cite comme la plus petite plage de la source qui la contient ; une citation se surligne exactement sur les caractères affichés issus de sa plage |

---

## 7. Tests et vérification

- **Ligne de commande** (contrat, mutation sans survivant comme d’habitude) :
  - `source` octet pour octet : échange, note, version ancienne, refus ;
  - `citations` : format, ordre, source révisée ;
  - `citer --debut/--fin` : plage exacte, refus, exclusivité avec `--texte`.
- **Rendu** (`gui/test_rendu.py`, sans affichage) : section 4.
- **Fenêtre** (`gui/test_gui.py`, sous `xvfb-run`) : ouverture d’un échange et d’une note, surlignage d’une citation existante, sélection puis citation (avec et sans confirmation), bascule rendu ou brut, version ancienne.
- **Mutation** : le Python n’y est pas soumis (limite connue) ; sa logique est concentrée dans `rendu.py` et testée en détail.

---

## 8. Découpage

| Étape | Contenu |
|---|---|
| 6.1a | ligne de commande : `source`, `citations`, `citer --debut/--fin` ; contrat, code |
| 6.1b | `gui/rendu.py` et ses essais |
| 6.1c | fenêtre du visionneur, bouton « Voir », essais sous `xvfb-run` ; documents ; contrôle complet ; livraison `v0.6.0-alpha.1` |

Hors de 6.1, à concevoir ensuite : relier une sélection à une note existante en un geste (« centre de travail »), vue locale du voisinage (R7), mini-shell (R5).

---

## 9. Points validés le 6 novembre 2026

1. **Garder Tkinter** (D17 confirmée par D25), avec un moteur de rendu écrit pour le projet ; PySide6 seulement si l’usage montre que c’est insuffisant.
2. **Sous-ensemble Markdown** de la section 4 ; tableaux, images et formules en texte brut.
3. **Trois ajouts à la ligne de commande** : `source`, `citations`, `citer --debut/--fin` (38 commandes).
4. **Règle de sélection** : la plus petite plage de la source qui contient la sélection, marques internes comprises (section 4, règle 2).
5. **Périmètre de 6.1** : voir, surligner, citer par sélection, versions d’une note. Le « centre de travail » (relier une sélection en un geste), R7 et R5 sont reportés.
