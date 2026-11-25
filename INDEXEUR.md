# Indexeur de repères — conception de l’incrément 6.3

Version 0.1 — 8 novembre 2026 — **validée par l’utilisateur le 8 novembre 2026** ; aucune décision nouvelle (application de D9 et des invariants INV-CODE) ; **implémentée** (6.3a noyau, 6.3b ligne de commande et interface ; `v0.6.0-alpha.3`). Étape annoncée dès la phase 2 (INVARIANTS.md, section 11 : « indexeur de repères : occurrences, chemins périmés, états retrouvé, modifié, ambigu, perdu »). Couvre CA-05, CA-30 et CA-31, rend testable INV-CODE-01 et complète INV-CODE-05. Regroupe en section 8 les points à valider.

---

## 1. Ce que l’utilisateur obtient

```
memoire indexer ~/projets/rtos
```

L’exécutable parcourt le code et y cherche les repères `@design: <identifiant>`. Il les confronte aux notes et aux ancrages du dépôt, puis rend un rapport :

```
Repères : 7 dans 4 fichiers (31 fichiers lus, 2 ignorés)
Retrouvés :
  - Primitive commune (3f2a…) — src/driver.c:42
Modifiés (le code ancré a changé) :
  - Gestionnaire d'attente (77b0…) — src/driver.c:88 — extrait ancré absent de src/driver.c
Déplacés (chemin de l'ancrage périmé) :
  - Réveil perdu (9c01…) — src/kernel.c:10 — ancré dans src/engine.c
Ambigus (plusieurs occurrences, aucune choisie) :
  - Tick (aa12…) — src/a.c:3, src/b.c:7
Perdus (ancrage sans repère dans le code) :
  - Ancienne file (bb34…) — ancré dans src/old.c
Inconnus (repère sans note dans cette branche) :
  - cc56… — src/x.c:5 — présente sur la branche 5e1a…
Non ancrés (repère d'une note sans ancrage) :
  - Hypothèse (dd78…) — src/y.c:12
```

**Lecture seule** : rien n’est écrit, ni dans le code (INV-CODE-04), ni dans le dépôt. Un ancrage n’est jamais corrigé automatiquement (INV-CODE-05 : « un chemin périmé est signalé, jamais deviné »). Pour mettre un ancrage à jour, on ancre de nouveau avec `ancrer`.

---

## 2. Détection d’un repère

- Un **repère** est la suite `@design: ` suivie d’un identifiant canonique (UUID en minuscules, avec ses tirets), n’importe où dans une ligne. Ainsi `// @design: …` (C, Rust), `# @design: …`, `; @design: …` (assembleur), `/* @design: … */` sont tous reconnus. Le repère à copier reste `// @design: <id>` (INV-CODE-02).
- Une ligne peut porter plusieurs repères : chacun compte. Un identifiant mal formé après `@design:` n’est pas un repère : il est signalé dans une section « Mal formés », avec sa ligne.
- La détection est une fonction pure du noyau, `find_markers(texte) -> Vec<(ligne, Option<NodeId>)>`, appelée par la ligne de commande sur chaque fichier lu. Le noyau reste sans entrée-sortie.

---

## 3. Classement (noyau)

Le classement est une règle métier : il appartient au noyau. `Snapshot::code_report(scope, occurrences, fichiers) -> CodeReport` le calcule sur l’état courant :
- `occurrences` : chemin relatif, ligne et note de chaque repère trouvé ;
- `fichiers` : texte des fichiers lus.

Pour chaque **ancrage actif** du contexte, avec les occurrences du repère de sa note :

| Occurrences | Condition | État |
|---|---|---|
| plusieurs | — | **ambigu** : toutes listées, aucune choisie (INV-CODE-01) |
| une | dans un autre fichier que celui de l’ancrage | **déplacé** : nouveau et ancien chemins (INV-CODE-05) |
| une | dans le fichier de l’ancrage, extrait ancré présent tel quel | **retrouvé** |
| une | dans le fichier de l’ancrage, extrait ancré absent | **modifié** |
| aucune | — | **perdu** |

Puis, pour chaque repère :
- **inconnu** : sa note n’est pas dans l’état courant. Les branches qui la contiennent sont indiquées, comme pour `repere` (INV-CODE-05).
- **non ancré** : sa note existe mais n’a aucun ancrage actif. Aucune occurrence n’est classée deux fois.

Pour un déplacé, le rapport précise aussi si l’extrait ancré figure dans le nouveau fichier. Une note à plusieurs ancrages a une ligne par ancrage. Les ancrages retirés sont ignorés.

**Chemins** : comparés relativement au dossier indexé. Un chemin d’ancrage absolu situé sous ce dossier y est ramené ; `./` en tête est ignoré ; le séparateur est `/`.

---

## 4. Ligne de commande

| Commande | Effet |
|---|---|
| `indexer DOSSIER [--extensions LISTE]` | parcourt DOSSIER récursivement et rend le rapport de la section 1 ; `--extensions c,h,s,rs` restreint les fichiers lus |

**Fichiers lus :**
- sont ignorés les dossiers cachés (`.git`, `.memoire`…), `target` et `node_modules` ;
- sont aussi ignorés les fichiers qui ne sont pas en UTF-8 valide et ceux de plus de 4 Mo ; ils sont comptés « ignorés » ;
- l’ordre de lecture et de rapport est l’ordre des chemins, pour un rapport reproductible.

**Code de retour :** 0 même s’il y a des repères perdus ou ambigus, puisque c’est un rapport. Il vaut 2 si DOSSIER n’est pas un dossier, et 3 en cas d’erreur de lecture du dossier lui-même.

**Contrat modifié :** une ligne de plus dans l’aide figée, soit 39 commandes.

---

## 5. Interface

- La commande apparaît dans la fenêtre principale comme les autres.
- Un champ nommé `DOSSIER` reçoit un bouton « Parcourir » qui choisit un dossier (`gui/schema.py`). C’est le pendant des champs `FICHIER`.
- Le rapport s’affiche dans la zone de sortie. Pas de vue propre en 6.3.

---

## 6. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-CODE-01 | (existant, P) Un repère absent ou ambigu est signalé ; aucune occurrence n’est choisie arbitrairement |
| INV-CODE-04 | (existant) Le logiciel n’écrit jamais dans les fichiers de code |
| INV-CODE-05 | (existant) La résolution ne dépend pas du chemin ; un chemin périmé est signalé, jamais deviné |
| INV-CODE-06 | L’indexation ne modifie ni le code ni le dépôt ; deux indexations des mêmes fichiers rendent le même rapport |

---

## 7. Découpage et tests

| Étape | Contenu |
|---|---|
| 6.3a | noyau : `find_markers`, `code_report` (chaque état, chaque cas limite) ; contrat, mutation sans survivant |
| 6.3b | ligne de commande `indexer` (parcours, fichiers ignorés, chemins, rapport) ; champ `DOSSIER` dans l’interface ; documents ; contrôle complet ; livraison `v0.6.0-alpha.3` |

**Tests prévus :**
- **Noyau :**
  - repères dans plusieurs syntaxes de commentaire, plusieurs par ligne, identifiants mal formés ;
  - chaque état du tableau, inconnu et non ancré ;
  - note à plusieurs ancrages, ancrage retiré ;
  - chemins absolus et `./` ;
  - S-CODE-02 (fichier renommé : déplacé, même note).
- **Ligne de commande :**
  - arborescence d’essai avec dossiers cachés, fichier binaire, fichier non UTF-8, extensions ;
  - rapport exact ;
  - dépôt et fichiers inchangés après indexation (INV-CODE-04, INV-CODE-06) ;
  - recette sur `fixtures/rtos-2026-10/driver.c` avec l’ancrage de S-REF-01.

---

## 8. Points validés le 8 novembre 2026

1. **Détection** de `@design: <id>` n’importe où dans une ligne, quelle que soit la syntaxe de commentaire ; plusieurs repères par ligne ; section « Mal formés ».
2. **États et priorités** du tableau de la section 3 : ambigu, puis déplacé, puis retrouvé ou modifié selon l’extrait ancré ; perdu ; inconnu ; non ancré.
3. **Lecture seule** : l’indexeur ne corrige aucun ancrage. Ré-ancrer reste un geste de l’utilisateur (`ancrer`).
4. **Fichiers lus** et chemins comparés relativement au dossier indexé (section 4).
5. **Commande `indexer DOSSIER`** (39 commandes) et bouton « Parcourir » pour un dossier dans l’interface ; pas de vue propre en 6.3.
