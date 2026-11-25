# Restauration ciblée et historique — conception de l’incrément 6.4

Version 0.1 — 8 novembre 2026 — **validée par l’utilisateur le 8 novembre 2026** ; aucune décision nouvelle (application de D1, D6, D13, D22) ; **implémentée** (6.4a noyau, 6.4b ligne de commande ; `v0.6.0-alpha.4`).
- Couvre CA-27 (« après réouverture, restaurer une idée écartée avec ses liens sans revenir sur les changements indépendants du reste du projet ») et CA-33 (« annuler une restauration »).
- Rend testable INV-RESTORE-01 (« une restauration ciblée conserve le travail indépendant et explicite les conflits ultérieurs »).
- Suit le cahier des charges, section « Annulation et restauration simples » : présenter les éléments concernés, ne pas écraser silencieusement une version plus récente.
- Expose aussi la restauration complète, qui existe dans le noyau depuis l’incrément 2.2 (D6, INV-RET-03) mais n’a pas de commande.

La section 8 regroupe les points à valider.

---

## 1. Ce que l’utilisateur obtient

```
memoire historique                                    # où en étais-je ? commits de la branche active
memoire restauration plan --depuis 3f2a91c0 --notes N-DRV-1
memoire restaurer --depuis 3f2a91c0 --notes N-DRV-1 --motif "piste reprise" --oui
memoire restaurer --depuis 3f2a91c0 --motif "retour au jalon" --oui      # tout le réseau
```

- **Historique** : la liste des commits de la branche active, du plus récent au plus ancien. Pour chacun : son identifiant, la commande qui l’a publié et son motif. C’est de là qu’on choisit l’état à restaurer.
- **Restauration ciblée** : seules les notes désignées reprennent leur état d’alors, avec leurs liens internes. Tout le reste du réseau garde son état courant.
- **Plan** : avant d’agir, `restauration plan` montre ce qui sera rétabli et ce qui ne le sera pas :
  - une version plus récente du texte qui sera remplacée ;
  - un statut qui changera ;
  - une relation avec le reste du réseau, différente depuis, qui sera conservée.
- Une restauration est **une opération ordinaire** : enregistrée, annulable d’un seul `annuler` (CA-33). Elle est **critique** (D13) : aperçu et `--oui` ; sur main, elle crée une branche (D1). Rien n’est supprimé (D6) : les versions remplacées restent dans l’historique.

---

## 2. Ce qu’une restauration ciblée rétablit (noyau)

Nouvelle modification `Change::RestoreNodes { target, nodes }`, avec une sélection de notes. Pour chaque note de la sélection, elle prend la valeur qu’avait le commit cible pour :

| Objet | Rétabli |
|---|---|
| Contenu de la note (texte, révisions) | oui, la version de la cible ; les révisions venues depuis restent dans l’historique, pas dans l’état courant |
| Statuts de la note dans chaque contexte où elle figurait à la cible | oui (verdict, décision, implémentation, réserve, visibilité) |
| Relations **entre deux notes de la sélection** | oui ; une relation créée depuis entre elles est désactivée |
| Ancrages de la note | oui ; un ancrage créé depuis est retiré, comme dans la restauration complète |
| Acquittements dont le dépendant est dans la sélection | oui, présence comprise |
| Relations entre une note de la sélection et une note **hors sélection** | **jamais** : c’est du travail indépendant (INV-RESTORE-01) |
| Tout ce qui ne touche pas la sélection | jamais |

**Refus :**
- une note créée après la cible : il n’y a rien à rétablir ;
- une sélection vide ;
- un état résultant invalide, par exemple un statut « remplacée » dont la relation « remplace » vient de l’extérieur. Le refus nomme la règle violée ; on élargit alors la sélection au groupe cohérent.

**Plan** : `Repository::restore_plan(target, nodes)` rend, en lecture seule et exactement comme l’appliquera la restauration (INV-PREVIEW-01) :
- les **objets rétablis**, avec ce qui change :
  - « texte : version 2 rétablie, version 4 remplacée » ;
  - « décision : retenue → plus retenue » ;
  - relations internes, ancrages, acquittements ;
- les **relations avec l’extérieur différentes depuis la cible, conservées**. Ce sont les « conflits ultérieurs » d’INV-RESTORE-01, présentés et jamais tranchés en silence : les inclure revient à ajouter leur autre note à la sélection ;
- les **effets sur les réexamens**.

**Restauration complète** : `Change::Restore`, inchangée (INV-RET-03).

**Format v2** :
- commande `restaurer-notes cible <commit> notes <k> <id>…`, sélection triée et sans doublon ;
- une restauration ciblée ne peut pas faire partie d’un lot, pas plus que la restauration complète (D22).

---

## 3. Historique (noyau et ligne de commande)

- `Repository::history_of_branch(limit)` remonte les **premiers parents** de la tête de la branche active. Pour chaque commit, il rend son identifiant, son motif et l’opération qui l’a publié (journal : la commande).
- Les commits n’ont pas de date : le logiciel est déterministe (D3), et l’ordre suffit.

| Commande | Sortie |
|---|---|
| `historique [--nombre N]` | les N derniers commits de la branche active (20 par défaut), un par ligne : `ID — commande — motif` ; la racine est notée `racine` |

---

## 4. Ligne de commande

| Commande | Effet |
|---|---|
| `restauration plan --depuis COMMIT [--notes NOTES]` | plan de la section 2, en lecture seule ; sans `--notes`, aperçu de la restauration complète (effets sur les réexamens, nombre d’objets rétablis) |
| `restaurer --depuis COMMIT [--notes NOTES] [--motif M] [--oui]` | restauration ciblée, ou complète sans `--notes` ; opération critique : aperçu, puis `--oui` |

- `COMMIT` : identifiant complet, ou préfixe d’au moins 8 caractères d’un commit de l’historique de la branche active.
- `NOTES` : liste séparée par des virgules, comme pour `note creer` (R1).
- Sortie de `restaurer` : `Restauration enregistrée : <commit>`, puis le compte rendu habituel (branche créée).
- **Contrat modifié** : trois lignes de plus dans l’aide figée, soit 42 commandes. Dans l’interface, `--notes` est un champ à plusieurs notes avec autocomplétion.

---

## 5. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-RESTORE-01 | (existant, P) Une restauration ciblée conserve le travail indépendant et explicite les conflits ultérieurs |
| INV-RET-01, INV-RET-03 | (existants) Aucune identité ne disparaît ; ce qui est créé depuis reste résoluble |
| INV-RESTORE-02 | Une restauration ciblée ne change que la sélection, ses relations internes, ses ancrages et ses acquittements ; une seule opération, qu’un seul `annuler` défait |

---

## 6. Découpage et tests

| Étape | Contenu |
|---|---|
| 6.4a | noyau : `RestoreNodes`, `restore_plan`, `history_of_branch`, format v2 ; contrats ; propriétés (restaurations ciblées aléatoires) ; mutation sans survivant |
| 6.4b | ligne de commande `historique`, `restauration plan`, `restaurer` ; interface ; documents ; contrôle complet ; livraison `v0.6.0-alpha.4` |

**Tests prévus :**
- S-RET-02 ciblée : une idée écartée puis rétablie, avec ses liens internes, sans toucher une note indépendante modifiée depuis.
- Une version plus récente du texte est signalée dans le plan.
- Une relation avec l’extérieur créée depuis : conservée et listée.
- Une note créée depuis est refusée ; un état invalide est refusé, avec son motif.
- `annuler` d’une restauration (CA-33).
- Après réouverture d’une base SQLite (CA-27).
- Propriétés :
  - ce qui est hors sélection est identique avant et après ;
  - restaurer toutes les notes existantes à la cible, avec leurs relations internes, équivaut à la restauration complète sur ces objets.

---

## 7. Hors de 6.4

- Proposer automatiquement le « groupe cohérent » à partir des relations ; aujourd’hui, l’utilisateur l’élargit en lisant le plan.
- Restaurer depuis le visionneur.
- CA-32 (afficher les ancrages devenus incompatibles après une restauration) : `indexer` le fait déjà sur demande.

---

## 8. Points validés le 8 novembre 2026

1. **Ce que rétablit une restauration ciblée** (tableau de la section 2), en particulier : les relations avec l’extérieur ne sont **jamais** touchées, seulement listées dans le plan.
2. **Une version plus récente** du texte ou d’un statut est rétablie à sa valeur de la cible, signalée dans le plan, avec confirmation (`--oui`) ; rien n’est supprimé de l’historique.
3. **Exposition de la restauration complète** : `restaurer --depuis COMMIT`, sans `--notes`.
4. **Nouvelle commande `historique`** pour choisir l’état ; pas de date (D3).
5. **Trois commandes** (`historique`, `restauration plan`, `restaurer`), soit 42 ; l’interface les montre comme les autres.
