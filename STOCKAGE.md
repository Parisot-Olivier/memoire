# Stockage SQLite

Version 0.3 — 6 novembre 2026 — schéma 2 (incréments 5.2c et 5.2d). Référence de conception de la crate memoire-store. Conception détaillée et raisons : ETAT-HISTORIQUE.md (sections 3 à 7, précisions aux sections 14 à 16), décisions D20 et D21. La version 0.2 (24 octobre 2026, schéma 1) reste décrite à la section 7, car la migration la lit encore.

## 1. Rôle et frontières

La crate `memoire-store` conserve un dépôt dans un fichier SQLite qui fait autorité (D4). Elle ne contient aucune règle métier : chaque commande est préparée, validée et appliquée par le domaine ; le stockage rend durable le résultat, et seulement lui.

| Couche | Responsabilité | Interdit |
|---|---|---|
| Domaine (`memoire-domain`) | Règles, validation, écritures d’une opération (`Writes`), format textuel et découpage en lignes | Toute entrée-sortie, toute dépendance |
| Stockage (`memoire-store`) | `SqliteHistory` (trait `History`), une transaction par commande, ouverture, vérification complète, migration | Décider d’une règle métier, produire une ligne que le domaine n’a pas produite |
| Service (`memoire-app`) | Identifiants UUID, enveloppes, motifs, fichiers (export, mise en place d’une migration) | Écrire en base sans passer par le stockage |

Dépendance admise (D14) : `rusqlite` avec SQLite intégré (`bundled`), version figée par Cargo.lock.

## 2. Principe : deux parties, une seule définition des données

- **État courant** de la tête de chaque branche non abandonnée, ligne canonique par ligne canonique (`etat_courant`) : ce que lisent `statut` et les commandes ordinaires. Une branche abandonnée (incrément 5.4a) perd son état courant ; il est reconstruit depuis l'historique, et validé, à sa réactivation.
- **Historique** : commits, avec un état de référence complet tous les cent commits le long des premiers parents et une différence avec le premier parent ailleurs ; journal des opérations ; identités employées ; branches et piles de rétablissement.

Toutes les lignes stockées sont des morceaux du texte canonique v2 (FORMAT.md, section 13), produits par le domaine (`state_lines`, `encode_active`, `encode_branch_v2`, `encode_identity`, `encode_commit_v2`, `encode_operation_v2`) ; le texte complet se rassemble depuis les tables (`Store::text`). Le format reste la seule définition des données.

**Validation (D21, qui révise le principe de la version 0.2 : « une seule voie de lecture et de validation »)** : l’ouverture lit et valide les métadonnées, les branches, la branche active et l’état courant de celle-ci ; un état ancien est validé à chaque lecture ; la vérification complète (`Store::verify`, par `verify_repository_v2`, à mémoire bornée, plus la comparaison de l’état courant de chaque branche à son historique) a lieu à l’import, à l’export, après une migration et par `memoire verifier`.

## 3. Schéma, version 2

Métadonnées : `('schema', '2')`, `('format', 'memoire-depot 2')`.

| Table | Contenu |
|---|---|
| `meta(cle, valeur)` | version du schéma et du format |
| `active(unique_ligne = 1, branche)` | branche active |
| `branches(id, tete, origine, base, etat)` | métadonnées de chaque branche ; `etat` : `active` ou `abandonnee` ; index sur `tete` |
| `retablir(branche, rang, sorte, cible, tete)` | pile de rétablissement de chaque branche |
| `identites(cle, ligne)` | identités employées : clé d’ordre canonique et ligne `identite` |
| `commits(id, parent1, parent2, motif, profondeur, nature)` | `parent2` : tête de la source d’un commit de fusion (incrément 5.4), sinon `NULL` ; profondeur et différence le long de `parent1` ; `nature` : `reference` ou `difference` |
| `lignes_reference(commit_id, cle, ligne)` | état complet d’un commit de référence |
| `lignes_difference(commit_id, rang, sens, cle, ligne)` | différence avec le premier parent : lignes retirées (`-`) puis ajoutées (`+`) |
| `etat_courant(branche, cle, ligne)` | état de la tête de chaque branche ; index sur `cle` (appartenance d’une note aux branches) |
| `operations(op, bloc)` | journal : bloc `operation` v2 |

Clés de ligne (ordre lexicographique = ordre du texte) : `1/<note>/0`, `1/<note>/1/<rang sur 20 chiffres>`, `1/<note>/2`, `2/<ancrage>`, `3/<contexte>/0`, `3/<contexte>/1/<note>`, `3/<contexte>/2/<relation>`, `3/<contexte>/3/<dépendant>/<cause>` (ETAT-HISTORIQUE.md, section 15).

Lecture d’un état : tête d’une branche non abandonnée, dans `etat_courant` ; sinon depuis la référence la plus proche et au plus 99 différences. Le nombre de lignes lues ne dépend pas de la longueur de l’historique (contrat : `reading_does_not_depend_on_the_history_length`).

Réglages : `journal_mode = WAL`, `synchronous = FULL`, posés seulement après la lecture du schéma et du format : une base refusée n’est jamais modifiée.

## 4. Transaction par commande

1. Le domaine prépare l’enveloppe (`Repository::prepare`) : refus, répétition (résultat enregistré) ou sans effet → aucune écriture.
2. Sinon, il rend les écritures (`Writes`) : commit et son état, branches modifiées, branche active, identité nouvelle, état de la tête active, opération.
3. `SqliteHistory::apply` les écrit dans une seule transaction (`BEGIN IMMEDIATE` … `COMMIT`) : ligne du commit, état de référence ou différence, branches et piles de rétablissement, différence de l’état courant de la branche dont la tête a bougé (ou retrait de l’état courant d’une branche abandonnée, reconstruction de celui d’une branche réactivée), branche active, identités, opération.
4. La partie en mémoire du dépôt (branche active, branches, état de la tête active) n’est mise à jour qu’après le `COMMIT` réussi (INV-TX-01).

## 5. Migration du schéma 1 (incrément 5.2d)

`memoire migrer` (ou la question de l’interface), jamais automatique : sauvegarde `sauvegardes/avant-migration-AAAAMMJJ-HHMMSSZ.db` ; lecture de la base de schéma 1 avec les règles de `v0.5.0-alpha.1` ; écriture d’un nouveau fichier par le chemin de l’import ; vérification complète ; renommage atomique sur l’ancienne base, sauf si un autre processus l’a ouverte. La source n’est jamais écrite. Détail : ETAT-HISTORIQUE.md, sections 7 et 16.

## 6. Invariants et tests

- INV-DUR-01 : processus fils tués pendant leurs écritures ; fichier rouvert égal à un préfixe exact de la suite de référence. Une écriture qui échoue en cours de route ne laisse aucune base partielle.
- INV-RETRY-01 : une enveloppe rejouée après réouverture rend le même résultat sans écriture.
- INV-EXPORT-01, D20 : après chaque commande, la base et l’historique en mémoire, conduits par les mêmes enveloppes, ont le même texte canonique complet (contrat et propriétés aléatoires à travers SQLite).
- INV-MIGR-01, INV-MIGR-02 : migration fidèle, jugée par l’export de l’ancienne version ; refus sans modification.
- Intégrité (D21) : fichier étranger, schéma inconnu, état courant illisible refusés à l’ouverture ; état ancien corrompu refusé à sa lecture et par la vérification complète.

## 7. Schéma 1 (versions `v0.3.0` à `v0.5.0-alpha.1`), lu par la migration

```
meta(cle TEXT PRIMARY KEY, valeur TEXT NOT NULL) STRICT
    -- ('schema', '1'), ('format', 'memoire-depot 1')
fragments(categorie TEXT NOT NULL CHECK (categorie IN ('active', 'branche', 'identite', 'commit', 'operation')),
          cle TEXT NOT NULL, texte TEXT NOT NULL,
          PRIMARY KEY (categorie, cle)) STRICT, WITHOUT ROWID
```

Chaque ligne de `fragments` contient un fragment du texte canonique v1 (`memoire_domain::Fragments`) ; le dépôt se relit en rassemblant les fragments puis par `decode_repository` ; des fragments rangés sous une autre clé que leur clé canonique sont refusés. Chaque commit y conservait son instantané complet : taille et temps d’ouverture croissaient avec le carré de l’historique (mesure de 5.2a : 33,5 s et 2,36 Go à 5 000 opérations).
