# Citations, échanges dans les dépendances, notes orphelines — conception de l’incrément 5.5

Version 0.1 — 6 novembre 2026 — **validée par l’utilisateur le 6 novembre 2026** (décision D24) ; **implémentée** (5.5a noyau, 5.5b ligne de commande ; `v0.5.0-alpha.5`). Étape 4 de la feuille de route (REPRISE.md, section 7) et de VERSION2.md (section 8, point 5). Met en œuvre D18 (validée le 31 octobre 2026) et la vue « Orphelines » de R6. Les principes de D18 ne changent pas : ce document dit comment les appliquer et regroupe en section 9 les points à valider.

---

## 1. Ce que l’utilisateur obtient

```
memoire citer 3f2a91c0 --texte "le réveil peut être perdu"       # dans un échange (réponse par défaut)
memoire citer 77b0e2d4 --texte "invariant de priorité"           # dans une note
memoire lier N-DRV depend-de 9c01aa52                             # une note dépend d’un échange entier
memoire lier 9c01aa52 developpe-a-partir-de 3f2a91c0              # une conversation née d’une autre
memoire retirer 3f2a91c0 --motif "conversation périmée" --oui    # un échange sort des ressources actuelles
memoire statut                                                    # … puis « Orphelines : »
```

- **Citer** remplace `passage` (D18, point 5). On cite dans un échange (question ou réponse) **ou dans une note**. Dans les deux cas, la même opération crée un lien « développé à partir de » de la citation vers sa source (D18, point 3). Ce lien est une relation ordinaire : visible, et qu’on peut retirer.
- La citation d’une note est attachée à la **version citée**. Si la note est révisée ensuite, la citation ne bouge pas. Elle est signalée « source révisée ». Une révision substantielle de la source met aussi la citation à réexaminer, par le lien créé (règle de réexamen existante, sans code nouveau).
- **Échanges dans les dépendances** (D18, points 1, 2 et 4) :
  - une note peut dépendre d’un échange entier, ou s’en être développée ;
  - un échange peut s’être développé à partir d’un autre ;
  - un échange retiré des ressources actuelles met ses dépendants à réexaminer.
- **Orphelines** (R6) : une note dont **tous** les appuis déclarés sont écartés, et dont toutes les causes de réexamen ont été acquittées, apparaît dans une liste « Orphelines » de `statut`.

---

## 2. Noyau : règles de relation (D18, points 1, 2 et 4)

| Relation | Aujourd’hui | Après 5.5 |
|---|---|---|
| `developpe-a-partir-de` | affirmation → affirmation | en plus : affirmation → échange, échange → échange |
| `depend-de` | affirmation → affirmation | en plus : affirmation → échange |
| autres | inchangées | inchangées |

- « Affirmation » : raisonnement, implémentation, note personnelle, citation (`Kind::is_claim`).
- **Les règles s’assouplissent, aucune ne se durcit.** Tout état valide aujourd’hui le reste, et l’historique n’est jamais rejugé (INV-REL-04). Le versionnement des règles (D16) n’est pas déclenché.
- **Retrait d’un échange** : `Change::Archive` vaut déjà pour toute note. Un échange archivé est écarté, donc cause de réexamen pour ses dépendants, par la règle existante. Rien à coder dans le noyau.
- Un échange ne porte toujours ni verdict ni décision.

---

## 3. Noyau : citation d’une note (D18, point 5 ; R8 ; VERSION2.md, section 5)

- La citation (rôle `Kind::Passage` dans le code, mot `citation` dans le format v2) a désormais deux sortes de sources :
  - **un échange**, partie question ou réponse, comme aujourd’hui ;
  - **une note**, avec la version citée : 0 pour le texte de création, *k* pour la *k*-ième révision.
- **Notes citables** : raisonnement, implémentation, note personnelle (section 9, point 3). Pas une catégorie, pas un échange (on en cite une partie), pas une citation, pas une réserve.
- **Validation** :
  - la version existe ;
  - la plage est valide en UTF-8 dans le texte de cette version (INV-SOURCE-01) ;
  - l’empreinte enregistrée est celle du texte cité.
- **Commande du noyau** : nouvelle variante `Change::CiteNote { scope, id, note, version, start, end }`. Ajouter une variante plutôt que modifier `CreatePassage` laisse intacts le journal existant et les tests qui le construisent.
- **Lecture** : `Snapshot::citation(id)` rend la source (échange et partie, ou note et version), la plage et le texte cité. Le noyau répond aussi à la question « source révisée ? » : la note citée a-t-elle une révision plus récente que la version citée ?
- **Format v2** : `citation … partie note version <n>` et `commande citer … partie note version <n>` sont désormais lus. Ils étaient réservés et refusés comme « pas encore pris en charge ». Contrat modifié : les deux cas `NotYetSupported("citation d'une note")` deviennent des allers-retours.
- **Réserves** : elles ne naissent toujours que d’une citation d’échange (section 9, point 4).

---

## 4. Noyau : vue « Orphelines » (R6)

**Définition** (celle de RETOURS.md, R6, précisée). Dans un contexte, une note est orpheline si :
1. elle a au moins une relation active « dépend de » ou « développé à partir de » vers une autre note, ses **appuis déclarés** ;
2. **tous** ces appuis sont écartés : réfutés, archivés, « plus retenus » ou remplacés ;
3. elle n’a **aucune** cause de réexamen en cours : chaque cause née de ces retraits a été acquittée ;
4. elle n’est pas elle-même écartée.

`Graph::orphans()` rend ces notes, par identifiant croissant.

**Conséquence sur les ressources actuelles** (R6 : « la note sort des ressources actuelles ») : `Graph::current_resources()` exclut les orphelines. C’est un changement de contrat de D11 (section 9, point 6).

Une citation dont la source est écartée, et dont la cause est acquittée, peut être orpheline comme toute autre note.

---

## 5. Ligne de commande et interface

| Commande | Effet |
|---|---|
| `citer SOURCE --texte X [--partie question\|reponse] [--occurrence N] [--motif M] [--oui]` | remplace `passage` ; SOURCE : échange ou note. Pour une note, la version citée est la version courante et `--partie` est refusé (code 2). Sortie : `Citation créée : <id> (octets A à B)`, puis `Relation créée : <id> (developpe-a-partir-de <source>)` |
| `retirer NOTE [--motif M] [--oui]` | retire une note, un échange compris, des ressources actuelles (`Change::Archive`) ; opération critique si elle crée des causes de réexamen |
| `statut` | ajoute une section `Orphelines :` après `À réexaminer :` |
| `repere` | une citation de note y est désignée par son texte, suivi de « (source révisée) » quand c’est le cas |

- **La citation et son lien forment un lot** (`Command::Lot`, D22) construit par la ligne de commande : un seul commit, un seul `annuler` (section 9, point 2).
- **Mot de rôle** : le rôle `passage` devient `citation` dans les sorties de la ligne de commande (listes de `chercher`, complétion de l’interface). Le format v2 l’appelle déjà ainsi.
- **Contrat modifié** :
  - aide figée : `passage` remplacée par `citer`, `retirer` ajoutée, soit 36 commandes ;
  - sorties de `statut` (nouvelle section) ;
  - mot de rôle ;
  - tests de l’interface (nombre de commandes ; `PASSAGE` reste un nom de champ de `reserve creer`).
- **Interface** : formulaires générés depuis l’aide, sans code propre. L’autocomplétion de `citer SOURCE` propose échanges et notes.

---

## 6. Compatibilité

- **Base existante de l’utilisateur** : aucune migration (schéma 2 et format `memoire-depot 2` inchangés). Les passages déjà créés gardent leur forme. Ils n’ont pas de lien « développé à partir de » vers leur échange, et rien n’est récrit (D6) ; on peut créer ce lien à la main avec `lier`.
- **Ancien exécutable** : `v0.5.0-alpha.4` ouvrant une base qui contient une relation nouvelle (vers un échange) ou une citation de note la refuse, avec un motif (relation interdite, citation de note non prise en charge). Il n’écrit rien. L’en-tête du format ne change pas : l’assouplissement ne justifie pas une version 3.

---

## 7. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-CIT-01 | Une citation désigne exactement le texte d’une version de sa source (partie d’échange, ou version d’une note) ; une révision ultérieure de la note ne la déplace pas et la fait signaler « source révisée » |
| INV-CIT-02 | Une citation créée par `citer` naît avec son lien « développé à partir de » vers sa source, dans la même opération ; ce lien se retire comme toute relation |
| INV-ORPH-01 | Une note est orpheline si et seulement si elle a au moins un appui déclaré, que tous sont écartés, qu’elle n’a aucune cause de réexamen en cours et qu’elle n’est pas écartée ; une orpheline n’est pas une ressource actuelle |
| INV-REL-04 | (existant) Les règles de type sont vérifiées à l’activation ; l’assouplissement ne rejuge aucun état |

---

## 8. Découpage et tests

| Étape | Contenu |
|---|---|
| 5.5a | noyau : règles de relation assouplies ; `Change::CiteNote`, validation, `Snapshot::citation`, source révisée ; format v2 de la citation de note ; `Graph::orphans` et ressources actuelles ; contrats et propriétés (citations de notes et relations vers des échanges dans les suites aléatoires) |
| 5.5b | stockage (aucun changement de schéma attendu : vérification par équivalence) ; ligne de commande `citer`, `retirer`, `statut`, `repere`, mot de rôle ; interface |
| 5.5c | documents, contrôle complet, livraison `v0.5.0-alpha.5` |

**Tests prévus :**
- chaque relation nouvellement permise, et celles qui restent refusées (échange → affirmation, catégorie, etc.) ;
- citation d’une note à chaque version, plage invalide, version inexistante, note non citable ;
- signal « source révisée » après une révision mineure ;
- réexamen après une révision substantielle, par le lien créé ;
- orphelines : chaque condition de la définition, levée de l’état d’orpheline (un appui redevient valide, une cause n’est plus acquittée) ;
- `citer` en un seul lot, annulable d’un geste ;
- `retirer` d’un échange et réexamen de ses dépendants.

---

## 9. Points validés le 6 novembre 2026

1. **Découpage** en 5.5a, 5.5b et 5.5c, livraison `v0.5.0-alpha.5`.
2. **Le lien créé par `citer` est un lot de la ligne de commande**, non une règle du noyau. Le lien peut être retiré ensuite (D18), donc le noyau ne pourrait de toute façon pas en garantir la présence. Le journal existant reste inchangé (les anciennes opérations `citer` sans lien se rejouent à l’identique).
3. **Notes citables** : raisonnement, implémentation, note personnelle. Ni catégorie, ni échange (on en cite une partie), ni citation, ni réserve. La ligne de commande cite toujours la version courante.
4. **Réserves** : elles ne naissent que d’une citation d’échange, comme aujourd’hui. Les ouvrir aux citations de notes est reporté, faute de besoin exprimé.
5. **`passage` devient `citer`**, sans alias ; le mot de rôle `passage` devient `citation` dans les sorties.
6. **Orphelines** : la définition de la section 4. Elles sortent des ressources actuelles (changement de D11, à consigner dans une décision D24).
7. **Nouvelle commande `retirer`** (archivage d’une note ou d’un échange), nécessaire pour D18, point 4, et absente aujourd’hui de la ligne de commande. Le noyau n’a pas d’opération inverse : un retrait se défait par `annuler`, ou plus tard par une restauration, comme aujourd’hui.
