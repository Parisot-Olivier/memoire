# Opérations groupées (`lot`) — conception de l’incrément 5.3

Version 0.1 — 6 novembre 2026 — **validée par l’utilisateur le 6 novembre 2026** (décision D22) ; implémentée et vérifiée (incrément 5.3, `v0.5.0-alpha.3`, VERIFICATION-PHASE5.txt). Étape 3 de VERSION2.md (section 8) ; besoin R1 (RETOURS.md) : « créer une note et la lier dans le même geste ». Exigence déjà posée (INVARIANTS.md, section 5) : « l’API de lot devra valider toutes les étapes avant publication et créer une seule branche pour l’ensemble ».

---

## 1. Ce que l’utilisateur obtient

```
memoire note creer --role raisonnement --titre "Réveil par primitive commune" \
    --classe-dans N-DRV --depend-de 3f2a91c0,77b0e2d4
```

- Une **seule opération** : la note et ses trois relations sont créées ensemble, ou rien ne l’est.
- Un **seul `annuler`** défait le tout ; un seul `retablir` le refait.
- Si l’ensemble est critique (par exemple, une dépendance envers une note réfutée crée une cause de réexamen), la confirmation `--oui` est demandée **une fois**, et une **seule branche** est créée pour l’ensemble.
- Sans option de lien, `note creer` reste exactement la commande d’aujourd’hui.
- Dans l’interface, les champs `--classe-dans`, `--depend-de`, `--developpe-a-partir-de` et `--lien` apparaissent dans le formulaire de `note creer`, avec l’autocomplétion par nom (R3).

---

## 2. Noyau : la commande `Command::Lot`

Une nouvelle variante de commande, générique : `Command::Lot(Vec<Change>)`, une suite de modifications élémentaires appliquées dans l’ordre. La ligne de commande n’en expose d’abord qu’un usage (section 5), mais le noyau accepte toute suite valide.

**Règles de préparation** (toutes contrôlées avant toute écriture, comme pour une commande seule) :

1. **Non vide.** Un lot sans élément est refusé (`Error::EmptyLot`).
2. **Pas de restauration dans un lot.** `Restore` remplace l’état entier : mêlé à d’autres modifications, son sens serait ambigu. Refusé (`Error::NotInLot`).
3. **Application dans l’ordre**, sur un état de travail : l’élément *k* voit l’état produit par les éléments 1 à *k − 1*. C’est ce qui permet de lier une note créée plus haut dans le même lot. Chaque état intermédiaire est validé.
4. **Refus localisé.** Le refus d’un élément refuse tout le lot et nomme l’élément : `Error::InLot { element, cause }` (élément numéroté à partir de 1, cause : le refus habituel de la commande seule). Exemple de message : « élément 3 du lot : note inconnue ».
5. **Identités.** Chaque identité créée doit être non nulle, jamais employée (INV-ID-02), et **distincte des autres identités du même lot**.
6. **Motif.** Exigé si au moins un élément l’exige seul (changement de statut, D12).
7. **Élément sans effet admis.** Le lot est jugé sur son état final. Si cet état final égale l’état courant, rien n’est publié ni journalisé (INV-MOD-04), comme pour une commande seule sans effet.
8. **Criticité sur l’effet net (D13).** Le lot est critique si l’état final, comparé à l’état courant, crée ou lève une cause de réexamen, ou écarte une note. Une cause créée par un élément et levée par un autre ne compte pas : seul l’effet réel compte. Un lot critique sur main crée **une** branche, celle de l’opération.
9. **Aperçu.** Mêmes effets que l’exécution (INV-PREVIEW-01) : effets nets entre l’état courant et l’état final.

**Écritures.** Un lot accepté publie **un** commit, journalise **une** opération et emploie **plusieurs** identités. Conséquence d’interface interne : le champ `Writes::identity: Option<UsedId>` devient `identities: Vec<UsedId>` (ordre canonique). Le stockage SQLite écrit une ligne d’identité par identité, dans la même transaction. C’est un changement de contrat du domaine (D20), consigné dans un commit « Contrat modifié ».

**Annuler, rétablir, répétition.** Rien de nouveau : un lot est une opération comme une autre. Annuler revient au parent du commit, rejouer la même enveloppe rend le résultat enregistré (INV-RETRY-01).

---

## 3. Format (version 2)

Forme déjà réservée par VERSION2.md (section 6) :

```
operation <op> attendu <branche> <tête> motif <ch>
  commande lot <n>
    element <commande> <champs>
    …
  resultat …
  effet …
```

- `<n>` est le nombre d’éléments (au moins 1) ; suivent exactement *n* lignes `element`, indentées de quatre espaces. Chacune a les mots et les champs de la ligne `commande` de la modification seule (FORMAT.md, section 12.2).
- Refusés à la lecture : `n` nul, nombre de lignes `element` différent de `n`, élément `restaurer`, `lot` imbriqué, identités en double dans le lot.
- **Version 1 :** un lot n’y est pas représentable. Le décodeur v1, conservé tel quel, continue de refuser `lot`. L’encodeur v1 ne sert plus qu’aux tests et à la lecture des bases de schéma 1. Les bases de schéma 1 ne contiennent aucun lot, puisqu’aucune version antérieure n’en écrivait.
- FORMAT.md, section 13, est complété ; un exemple de bloc `lot` est figé dans les tests du format.

---

## 4. Service

Aucun changement de principe : `Service::plan` reçoit déjà une fonction qui construit la commande en tirant des identités nouvelles (`fresh.node()`, `fresh.edge()`). Un lot en tire plusieurs, et l’aperçu les rapporte toutes (`Assigned`, INV-ID-04).

---

## 5. Ligne de commande et interface

**`note creer`** reçoit quatre options facultatives, une par type de relation que `lier` connaît :

```
note creer --role … --titre T [--texte X] [--classe-dans NOTES] [--depend-de NOTES]
    [--developpe-a-partir-de NOTES] [--lien NOTES] [--motif M] [--oui]
```

- `NOTES` : une ou plusieurs notes, séparées par des virgules, chacune par identifiant complet ou par préfixe d’au moins 8 caractères (comme ailleurs). La note créée est l’**origine** de chaque relation : elle « est classée dans », « dépend de », « se développe à partir de » chaque note citée.
- Une note citée deux fois pour le même type est refusée (erreur d’usage, code 2) ; une note introuvable, aussi.
- Sortie : `Note créée : <id>`, puis une ligne `Relation créée : <id> (<type> <destination>)` par relation, puis le compte rendu habituel (branche créée, causes de réexamen).
- Lot critique sans `--oui` : aperçu et code 4, rien n’est écrit, comme `lier`.
- **Contrat modifié :** la ligne de `note creer` dans l’aide figée (`tests/aide.txt`). Le nombre de commandes ne change pas (31).

**Interface :** le formulaire suit l’aide, sans code propre pour les champs. Seule l’autocomplétion change : dans un champ `NOTES`, elle porte sur le dernier élément après la dernière virgule, et le choix remplace ce seul élément.

**Hors de 5.3, à proposer plus tard si le besoin se présente :** une commande générique `memoire lot FICHIER`, qui lirait une suite de commandes. Elle demanderait une petite syntaxe pour désigner, dans le fichier, les notes créées plus haut. R1 n’en a pas besoin.

---

## 6. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-LOT-01 | Un lot est tout ou rien : un refus de n’importe quel élément ne modifie rien ; un lot accepté publie un seul commit et une seule opération, qu’un seul `annuler` défait |
| INV-LOT-02 | La criticité d’un lot se juge sur son effet net ; un lot critique sur main crée une seule branche |
| INV-LOT-03 | Les identités créées par un lot sont distinctes entre elles et jamais réemployées, même après annulation |

---

## 7. Tests et vérification

- **Contrat du domaine :** chaque règle de la section 2, refus localisés compris (élément *k*) ; égalité complète du dépôt après chaque refus (INV-TX-01) ; aperçu égal à l’exécution ; annuler et rétablir un lot ; lot critique, lot dont l’effet net n’est pas critique ; répétition d’un lot.
- **Propriétés aléatoires :** des lots aléatoires (2 à 5 éléments tirés des commandes existantes) entrent dans les suites, à travers l’historique en mémoire et à travers SQLite. Un lot accepté donne le même état final que ses éléments exécutés un à un, sauf la criticité et le nombre de commits. Un lot refusé ne change rien.
- **Format :** lecture et écriture d’un bloc `lot`, refus de la section 3, exemple figé.
- **Stockage :** plusieurs identités en une transaction ; arrêt brutal pendant l’écriture d’un lot.
- **Ligne de commande et interface :** `note creer` avec liens, critique ou non ; refus (note introuvable, doublon, préfixe ambigu) ; autocomplétion dans un champ à plusieurs notes.
- **Mutation :** sans survivant, comme d’habitude.

---

## 8. Découpage proposé

| Étape | Contenu |
|---|---|
| 5.3a | noyau : `Command::Lot`, règles, `Writes::identities`, format v2 du bloc `lot` ; contrats, propriétés |
| 5.3b | stockage (identités multiples), service ; ligne de commande `note creer` avec liens ; interface (autocomplétion à plusieurs notes) |
| 5.3c | documents, contrôle complet, livraison `v0.5.0-alpha.3` |

---

## 9. Points validés le 6 novembre 2026

1. La commande générique `Command::Lot` dans le noyau, avec un seul usage exposé pour l’instant (`note creer` avec liens), la commande `memoire lot FICHIER` étant reportée.
2. La criticité sur l’effet net (section 2, point 8) plutôt que « critique si l’un des éléments l’est seul ».
3. Les quatre options de relation sur `note creer`, avec des listes séparées par des virgules, et la note créée toujours à l’origine des relations.
