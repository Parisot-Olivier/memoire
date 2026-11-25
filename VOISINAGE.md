# Vue locale colorée — conception de l’incrément 6.6

Version 0.1 — 10 novembre 2026 — **validée par l’utilisateur le 10 novembre 2026** ; **implémentée** (6.6a noyau et ligne de commande, 6.6b interface ; `v0.6.0-alpha.6`) ; précisions de mise en œuvre : section 9 ; aucune décision nouvelle : application de D17 (interface Tkinter qui pilote l’exécutable) et de D25 (visionneur).
- Répond à R7 : « vue locale du voisinage d’une note, couleur selon l’origine du lien (citation, note entière, catégorie) ».
- Prolonge le visionneur (VISIONNEUR.md) et le centre de travail (CENTRE.md) : après avoir cité et relié, on voit autour d’une note ce qui la relie au reste du réseau, et on s’y déplace.

La section 8 regroupe les points validés.

---

## 1. Ce que l’utilisateur obtient

Dans l’interface, un bouton **« Voisinage »** ouvre une fenêtre centrée sur une note. Il figure :
- à côté de « Voir » dans les champs de note ;
- dans la barre du haut (« Voisinage d’une note… ») ;
- dans le visionneur.

```
                 [Scheduler]                        Légende
                      ┆ classée dans                ── note entière   (bleu, trait plein)
   [Hypothèse] ──lien── (Réveil perdu) ──dépend de──▶ [Primitive commune]
                      ┆                             ┄┄ citation       (orange, pointillé)
          « le tick réveille… » ┄┄dépend de┄┄       ╌╌ catégorie      (vert, tirets)
           (citation de Échange 12)
```

- **Au centre**, la note, avec son rôle et son état : écartée, à réexaminer, orpheline.
- **Autour**, chaque note reliée par une relation active du contexte par défaut, une fois par relation. Le lien porte le type de la relation et une flèche dans son sens.
- **La couleur et le style du trait disent l’origine du lien**, avec une légende. Le style double la couleur, pour rester lisible sans elle (impression, daltonisme) :

| Origine | Quand | Couleur, trait |
|---|---|---|
| **catégorie** | relation « classée dans », dans un sens ou dans l’autre | vert, tirets |
| **citation** | l’autre extrémité est une citation (d’un échange ou d’une note) : la citation est montrée avec sa source | orange, pointillé |
| **note entière** | toute autre relation entre deux notes | bleu, trait plein |

- **Un clic** sur une note voisine recentre la vue sur elle. **« Précédent »** revient à la note d’avant.
- **Un double-clic** l’ouvre dans le visionneur.
- **Une liste** à côté du dessin reprend chaque lien en texte (origine, sens, type, note), dans les mêmes couleurs. Elle reste lisible quand les voisins sont nombreux et se copie.

**Lecture seule** : la vue ne modifie rien. Pour relier, on garde `lier`, `citer` et le panneau « Relier la sélection » du visionneur.

---

## 2. Noyau : `Graph::neighborhood`

Le classement est une règle : il appartient au noyau, testé et soumis à la mutation, comme `code_report` (INDEXEUR.md).

```rust
pub enum LinkOrigin { WholeNote, Citation, Category }
pub enum Direction { Outgoing, Incoming }
pub struct NeighborLink {
    pub edge: EdgeId,
    pub direction: Direction,   // sortant : la note est l'origine de la relation
    pub relation: Relation,
    pub other: NodeId,
    pub origin: LinkOrigin,
}
impl Graph {
    pub fn neighborhood(&self, node: NodeId) -> Result<Vec<NeighborLink>, Error>;
}
```

- **Règle d’origine**, dans cet ordre :
  1. « classée dans » donne **catégorie** ;
  2. sinon, si l’autre extrémité est une citation (rôle passage), **citation** ;
  3. sinon, **note entière**.
- **Ensemble** : toutes les relations **actives** du contexte qui touchent la note, chacune une fois. Les relations désactivées sont exclues.
- **Ordre**, pour un affichage reproductible : origine (note entière, citation, catégorie), puis sens (sortant, entrant), puis type, puis identifiant de l’autre note.
- **Refus** : `MissingNode` si la note n’est pas dans le contexte.
- La source d’une citation voisine se lit par `Snapshot::citation`, qui existe déjà (CITATIONS.md). L’état de chaque note (écartée, à réexaminer, orpheline) se lit par les fonctions existantes.

---

## 3. Ligne de commande : `voisinage`

| Commande | Effet |
|---|---|
| `voisinage NOTE [--tsv]` | relations actives de NOTE, groupées par origine ; `--tsv` : une ligne par lien, champs séparés par des tabulations, pour l’interface |

Sortie lisible :

```
Voisinage de Réveil perdu (9c01…) — raisonnement, à réexaminer
Note entière :
  → depend-de Primitive commune (3f2a…)
  ← lien Hypothèse (dd78…)
Citation :
  → depend-de « le tick réveille… » (c1d2…), citation de Échange 12 (e5f6…)
Catégorie :
  → classe-dans Scheduler (aa12…)
```

- Une section sans lien affiche « (aucun) ». Une note sans aucune relation donne les trois sections vides.
- Les notes voisines sont désignées comme ailleurs (`designation`), avec leur état entre parenthèses quand elles sont écartées ou à réexaminer.
- **Sortie `--tsv`** : un en-tête, puis une ligne par lien, dans l’ordre du noyau. Les champs sont :
  - relation ;
  - sens (`sortant` ou `entrant`) ;
  - type ;
  - origine (`note`, `citation` ou `categorie`) ;
  - note voisine ;
  - rôle ;
  - nom (titre, ou texte cité) ;
  - source de la citation (ou `-`) ;
  - état (`courante`, `ecartee`, `a-reexaminer` ou `orpheline`).

  Tabulations et retours à la ligne d’un nom sont remplacés par des espaces. Cette forme est un contrat, testé, comme la sortie de `chercher`.
- **Contrat modifié** : une ligne de plus dans l’aide figée, soit 47 commandes.

---

## 4. Interface

- **Nouveau module `gui/voisinage.py`**, la fenêtre (Tkinter, `Canvas`).
- **Partie pure, testée sans affichage** (`gui/test_voisinage.py`) :
  - lecture de la sortie `--tsv` ;
  - disposition : la note au centre, les voisins sur un cercle, groupés par origine dans l’ordre du noyau, positions déterministes ;
  - libellés tronqués à 28 caractères, nom entier dans la liste.
- **Couleurs** : palette lisible aussi par les daltoniens (bleu `#0072B2`, orange `#D55E00`, vert `#009E73`), toujours doublée du style de trait.
- **Navigation** :
  - clic : recentrer ;
  - double-clic : visionneur ;
  - « Précédent » : pile des notes visitées dans la fenêtre ;
  - « Actualiser » : relire après une modification faite ailleurs.
- **Essais sous `xvfb-run`** : la fenêtre s’ouvre sur une note ; un lien de chaque origine est dessiné avec sa couleur et son style ; un clic recentre ; « Précédent » revient.

---

## 5. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-VIEW-02 | La vue locale montre exactement les relations actives qui touchent la note dans le contexte, chacune une fois, classée par origine : catégorie (« classée dans »), citation (l’autre extrémité est une citation), note entière (sinon) ; elle ne modifie rien |

---

## 6. Découpage et tests

| Étape | Contenu |
|---|---|
| 6.6a | noyau : `Graph::neighborhood` (chaque origine, chaque sens, relations désactivées exclues, ordre, note absente) ; ligne de commande `voisinage` (sortie lisible et `--tsv`, désignation des citations et de leur source, états) ; mutation sans survivant |
| 6.6b | interface : `gui/voisinage.py`, boutons d’accès, essais ; documents ; contrôle complet ; livraison `v0.6.0-alpha.6` |

**Tests prévus :**
- **Noyau :**
  - une note reliée par chaque type de relation, dans les deux sens ;
  - citation voisine d’un échange et d’une note ;
  - catégorie dans les deux sens (la note classée dans, une note classée dans la note) ;
  - relation détachée exclue ;
  - ordre ;
  - note absente.
- **Ligne de commande :**
  - sortie lisible exacte ;
  - `--tsv` exacte, nom avec tabulation et retour à la ligne ;
  - états (écartée, à réexaminer, orpheline) ;
  - aucune écriture (texte du dépôt inchangé).
- **Interface** : la partie pure en tests unitaires ; la fenêtre sous `xvfb-run`.

---

## 7. Hors de 6.6

- Profondeur 2 ou plus dessinée d’un coup. Aujourd’hui, on avance de voisin en voisin par le clic.
- Relations désactivées montrées en grisé.
- Relier depuis la vue, par glisser-déposer.
- Autres contextes que le contexte par défaut, comme pour le reste de l’interface.

---

## 8. Points validés le 10 novembre 2026

1. **Les trois origines et leur règle** (section 2) : catégorie pour « classée dans », citation quand l’autre extrémité est une citation, note entière sinon.
2. **Profondeur 1 avec navigation** : clic pour recentrer, « Précédent », double-clic vers le visionneur ; pas de graphe plus large en 6.6.
3. **Relations actives seulement**, dans le contexte par défaut.
4. **Commande `voisinage NOTE [--tsv]`**, soit 47 commandes, et sa forme `--tsv` comme contrat pour l’interface.
5. **Couleurs doublées d’un style de trait**, avec une légende et une liste textuelle à côté du dessin.

---

## 9. Précisions apportées à la mise en œuvre

1. **Traits** : chaque trait s’arrête au bord de l’étiquette du voisin et du disque central, pour que la flèche reste visible ; l’étiquette a un fond blanc bordé de la couleur de l’origine.
2. **Note écartée** : en gris italique dans le dessin ; les autres états figurent dans la liste textuelle.
3. **Clic et double-clic** : le recentrage attend un court délai (300 ms), annulé par un double-clic, pour que le double-clic ouvre le visionneur sans recentrer d’abord.
4. **En-tête de la fenêtre** : la première ligne de la sortie lisible de `voisinage` (désignation, rôle, état) ; les liens viennent de la forme `--tsv`.

