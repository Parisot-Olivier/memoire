# Fusion à trois voies — conception (phase 5)

Version 0.2 — 31 octobre 2026 — validée par l’utilisateur le 31 octobre 2026, avant le code (D19). Section 9 (précisions d’implémentation, incrément 5.4) : version 0.3 du 6 novembre 2026, **validée par l’utilisateur le 6 novembre 2026** (D23) ; version 0.4 du même jour, validée : la forme de résolution `relation` est retirée (section 9.3). **Implémentée** à l’incrément 5.4 (5.4a abandon et réactivation, 5.4b noyau, 5.4c stockage et ligne de commande), `v0.5.0-alpha.4`.

## 1. But

Ramener sur une branche cible (en général main) le travail fait sur une branche source, en distinguant les changements indépendants, réunis sans question, des changements concurrents, présentés comme conflits par objet et par relation (CDC, section 10 ; INV-MERGE-01 à 03 ; CA-36, CA-37). Aucune fusion n’est jamais implicite.

## 2. Base commune

La base est le plus proche ancêtre commun des têtes des deux branches dans le graphe des commits (un commit de fusion a deux parents). Pour une branche créée depuis main et jamais fusionnée, c’est son commit de base.

## 3. Ce qui se fusionne, champ par champ

Pour chaque champ, on compare base (B), cible (C) et source (S) :
- C = B : on prend S ; S = B : on garde C ; C = S : inchangé ;
- C ≠ B, S ≠ B et C ≠ S : **conflit**.

| Objet | Champs comparés |
|---|---|
| Contenu d’une note | suite des révisions (ajout seulement) : une seule branche a révisé → on prend ses révisions ; les deux ont révisé différemment → conflit de contenu |
| Membre d’un contexte | verdict, décision, implémentation, état de réserve, visibilité |
| Relation | activité, état antérieur mémorisé |
| Acquittement | présence et état de cause confirmé |
| Ancrage | activité |
| Contextes, notes, relations, ancrages nouveaux | réunis (les identités ne se recouvrent jamais) |
| Identités employées | réunies |

## 4. Conflits structurels

Après réunion champ par champ, l’état candidat est validé par les règles ordinaires (INV-MERGE-03). Une violation qui naît de la seule réunion devient un conflit structurel présenté à l’utilisateur :
- cycle de catégories ou de remplacements formé par deux relations valides séparément ;
- deux relations actives identiques (même origine, destination et type) ;
- deux remplacements actifs d’une même note ;
- statut réservé incohérent (« remplacée » sans remplacement actif).
La résolution consiste à désactiver l’une des relations en cause ; elle reste dans l’historique.

## 5. Préparation, résolution, application

1. `plan de fusion` (lecture seule) : base, changements réunis sans conflit, conflits numérotés, effets sur les ressources actuelles et les réexamens.
2. L’utilisateur choisit, pour chaque conflit, la valeur de la cible ou celle de la source ; pour un conflit structurel, la relation à désactiver.
3. `fusionner SOURCE` avec les résolutions : refusé s’il reste un conflit non résolu, ou si la cible a changé depuis le plan (état attendu de l’enveloppe : le plan est recalculé).
4. Le résultat est un commit à deux parents (tête de la cible, tête de la source), une seule opération du journal, annulable d’un seul « Annuler ».

Une fusion ne bifurque jamais : c’est l’acte explicite qui ramène le travail sur main (exception à D1, décision D19). La branche source reste consultable.

**Sauvegarde automatique.** Avant d’appliquer une fusion, une copie cohérente de la base est écrite dans `sauvegardes/avant-fusion-AAAAMMJJ-HHMMSSZ.db` (même mécanisme que `sauvegarder`, R9). Si elle échoue, la fusion n’est pas appliquée. Cette copie est un filet de sécurité : l’historique complet reste dans la base courante, et l’annulation de la fusion reste le moyen ordinaire de revenir en arrière.

**Abandon d’une branche.** Une branche peut être abandonnée : la branche active revient sur son origine, la branche disparaît de la liste ordinaire mais reste consultable et réactivable, rien n’est supprimé (D6). L’annulation de l’opération fondatrice reste possible comme aujourd’hui.

## 6. Ce qui n’est pas fusionné automatiquement

Le texte d’une note révisée des deux côtés n’est pas fusionné ligne à ligne : l’utilisateur choisit une version, puis peut réviser. Le logiciel ne prétend pas détecter les contradictions du langage naturel (CDC).

## 7. Conséquences sur le format (version 2)

- commit : liste de parents (0, 1 ou 2) ;
- journal : commande `fusionner` avec sa source et ses résolutions ;
- et, pour la même version : opérations groupées (`lot`), citations de notes avec version citée et empreinte (D18). Voir VERSION2.md.

## 8. Tests prévus

Changements indépendants réunis ; chaque type de conflit ; conflit structurel par cycle ; plan obsolète refusé ; fusion annulée d’un geste ; fusion répétée (idempotence) ; propriété : fusionner une branche dans sa propre origine non modifiée redonne exactement l’état de la branche.

## 9. Précisions d’implémentation (incrément 5.4)

Elles précisent les sections 2 à 7 sans en changer les principes. Les points qui s’en écartent ou les complètent sont regroupés en 9.8.

### 9.1 Commandes du noyau

- `Command::Merge { source, source_head, resolutions }` : fusionne la branche `source` dans la branche active (la cible). `source_head` est la tête de la source sur laquelle le plan a été calculé : si elle a bougé, la fusion est refusée comme périmée, de même que si la cible a bougé (état attendu de l’enveloppe).
- `Command::Abandon { branch }` et `Command::Reactivate { branch }` (section 5, « Abandon d’une branche »).
- `Repository::merge_plan(source, &resolutions)`, en lecture seule : base, conflits restants, conflit structurel éventuel, effets sur les réexamens. Il calcule exactement ce que la fusion appliquera, comme l’aperçu pour les autres commandes (INV-PREVIEW-01).

### 9.2 Base commune

- Les ancêtres se lisent par `History::commit` en suivant **les deux parents** d’un commit de fusion.
- La base est l’unique ancêtre commun le plus proche : un ancêtre commun qui n’est l’ancêtre d’aucun autre ancêtre commun.
- **S’il y en a plusieurs** (fusions croisées dans les deux sens), la fusion est refusée avec un motif, au lieu de choisir une base au hasard (limite assumée, section 9.8).
- Cas particuliers :
  - tête de la source déjà ancêtre de la cible (rien de nouveau, par exemple une fusion répétée) : rien n’est publié ni journalisé (INV-MOD-04) ;
  - tête de la cible égale à la base : l’état fusionné est celui de la source, avec quand même un commit à deux parents, pour garder la trace de la fusion.

### 9.3 Réunion à trois voies

Rien n’est jamais retiré d’un état (D6) : on ne fait qu’ajouter des clés ou changer des valeurs. Pour chaque clé de la réunion des trois états, et pour chaque champ :

| Élément | Clé | Champs comparés séparément | Conflit |
|---|---|---|---|
| Contenu d’une note (révisions, question d’un échange) | note | le contenu entier : il ne fait que s’allonger | `texte` |
| Membre d’un contexte | contexte, note | verdict, décision, implémentation, état de réserve, visibilité | `champ` |
| Relation | contexte, relation | activité | (impossible, voir ci-dessous) |
| Acquittement | contexte, dépendant, cause | présence et état confirmé | `acquittement` |
| Ancrage | ancrage | activité | (impossible : un ancrage ne fait que se désactiver) |

- **Règle par champ** : cible = base, on prend la source ; source = base, on garde la cible ; cible = source, inchangé ; sinon conflit.
- **Absence dans la base :** une clé absente de la base et présente des deux côtés (une même note incluse dans un contexte sur les deux branches) se compare champ par champ, sans base. Chaque champ différent est alors un conflit.
- Le contenu de chaque membre de contexte est ensuite pris dans le contenu fusionné de sa note.
- **Rien ne disparaît** (D6) : une note, un membre, une relation ou un ancrage présent d’un seul côté est repris tel quel. Seul un acquittement peut manquer d’un côté (une restauration rend l’état d’alors, sans les acquittements venus depuis) : sa présence est un champ comparé comme les autres.
- **Relations et ancrages sans conflit possible** (version 0.4) : leur seul champ modifiable est l’activité, qui n’a que deux valeurs, et tout objet présent des deux côtés existe déjà dans la base (base unique, identités jamais réemployées). Les deux côtés ne peuvent donc pas différer à la fois de la base et entre eux. Si cela arrivait malgré tout (base modifiée hors du logiciel), la fusion serait refusée avec un motif, sans résolution possible.

### 9.4 Conflits structurels

- L’état réuni, résolutions de champ appliquées, est validé par les règles ordinaires (INV-MERGE-03).
- S’il est invalide, le plan présente un conflit structurel :
  - le motif du refus (cycle, relation en double, remplacement multiple, statut réservé) ;
  - comme relations candidates, les relations actives créées ou modifiées depuis la base sur l’une ou l’autre branche.
- **Résolution :** désactiver une ou plusieurs de ces relations, avec les effets de `retirer` une relation (le statut posé par un remplacement ou un traitement est rétabli). L’état est ensuite revalidé.
- La fusion n’est appliquée que sur un état valide.

### 9.5 Résolutions

Chaque résolution est repérée par sa clé et non par un numéro, pour que le journal reste lisible sans le plan. Elles reprennent les trois formes de VERSION2.md, section 6, et en ajoutent une :
- `champ <portée> <note> <champ> <cible|source>` (champ `verdict`, `decision`, `implementation`, `reserve` ou `visibilite`) ;
- `texte <note> <cible|source>` ;
- `acquittement <portée> <dépendant> <cause> <cible|source>` (nouveau) ;
- `desactiver <portée> <relation>`.

La fusion est refusée :
- s’il reste un conflit non résolu ;
- si une résolution ne correspond à aucun conflit, ou si une relation désactivée n’est pas candidate ;
- si deux résolutions portent sur la même clé.

### 9.6 Commit, journal, annulation, format

- **Commit de fusion :** deux parents, la tête de la cible d’abord puis la tête de la source. Profondeur et différence sont calculées le long du premier parent, comme pour les autres commits.
- **Annuler** ramène la cible sur son premier parent ; **rétablir** refait la fusion. La source n’est jamais modifiée.
- **Pas de bifurcation :** une fusion ne bifurque jamais (D19). Les effets sur les réexamens sont annoncés, et la ligne de commande demande `--oui` quand il y en a, comme pour toute opération qui en a.
- **Format v2 :**
  - `parents 2 <cible> <source>` est désormais lu ;
  - `commande fusionner source <branche> tete <commit> resolutions <k>`, suivi de *k* lignes `    resolution …` ;
  - `etat abandonnee` est désormais lu, ainsi que `commande abandonner branche <id>` et `commande reactiver branche <id>`.
- **Historique :** les commits gardent leur second parent (`NewCommit` et `MemoryHistory` passent à une liste de parents ; SQLite remplit la colonne `parent2`, déjà prévue). C’est un changement de contrat du domaine, consigné.

### 9.7 Abandon et réactivation

- `abandonner BRANCHE` :
  - refusé pour main et pour une branche déjà abandonnée ;
  - si la branche est active, la branche active revient sur son origine ;
  - la branche garde sa tête et son historique, et rien n’est supprimé (D6) ;
  - SQLite retire son état courant, reconstruit à la réactivation (VERSION2.md, section 3).
- Une branche abandonnée ne peut pas devenir active (`basculer`, ou `rétablir` qui y rentrerait), ni servir de source à une fusion, tant qu’elle n’est pas réactivée.
- `reactiver BRANCHE` : la branche redevient ordinaire, mais ne devient pas active.
- **Annulation :** `annuler` ne défait ni un abandon ni une réactivation. Ce ne sont pas des commits, et chacune défait l’autre.
- `branches` n’affiche plus les branches abandonnées ; `branches --toutes` les affiche, marquées « (abandonnée) ».

### 9.8 Points validés le 6 novembre 2026

1. **Tête de la source dans la commande** (`tete <commit>` dans le format, en plus de VERSION2.md). Sans elle, une source modifiée entre le plan et la fusion serait fusionnée dans un état que l’utilisateur n’a pas vu.
2. **Une forme de résolution ajoutée** (`acquittement`) aux trois de VERSION2.md. La forme `relation`, d’abord proposée, est retirée en version 0.4 (validée) : un conflit sur une relation est impossible (section 9.3).
3. **Plusieurs bases communes : refus**, plutôt qu’une base choisie arbitrairement. Ce cas n’apparaît qu’après des fusions croisées dans les deux sens entre deux branches.
4. **Abandon et réactivation non annulables par `annuler`**, chacune défaisant l’autre ; une branche abandonnée ne peut être ni active ni source.
5. **Ligne de commande** (section 9.9).

### 9.9 Ligne de commande et interface

- **`fusion plan SOURCE`** (lecture seule) affiche, dans l’ordre :
  - la base ;
  - le nombre de changements réunis sans conflit ;
  - les conflits **numérotés**, chacun avec sa valeur dans la cible et dans la source ;
  - le conflit structurel éventuel, avec ses relations candidates numérotées ;
  - les effets sur les réexamens ;
  - un **code de plan** de 17 caractères, `<8 premiers caractères de la tête de la cible>-<8 premiers de la tête de la source>`.
- **`fusionner SOURCE --plan CODE [--cible N,…] [--source N,…] [--desactiver N,…] [--motif M] [--oui]`** :
  - recalcule le plan ;
  - le refuse si le code ne correspond plus (« plan périmé : relancer fusion plan ») ;
  - traduit les numéros en résolutions par clé ;
  - écrit la sauvegarde `sauvegardes/avant-fusion-AAAAMMJJ-HHMMSSZ.db`, et si elle échoue, rien n’est fusionné ;
  - puis applique la fusion.
- **Branches :** `abandonner BRANCHE [--motif M]`, `reactiver BRANCHE [--motif M]`, `branches [--toutes]`.
- **Aide figée :** quatre commandes de plus (35). L’interface les montre comme les autres, sans code propre : on lance `fusion plan`, on lit les conflits, puis on remplit `fusionner`.

### 9.10 Découpage

| Étape | Contenu |
|---|---|
| 5.4a | abandon et réactivation : noyau, format, stockage (état courant retiré puis reconstruit), ligne de commande |
| 5.4b | noyau de la fusion : parents multiples, base commune, réunion, conflits, résolutions, plan, commit à deux parents, format ; propriétés |
| 5.4c | stockage, service, ligne de commande (plan, code, sauvegarde), interface ; documents ; contrôle complet ; livraison `v0.5.0-alpha.4` |

### 9.11 Tests

Ceux de la section 8, plus :
- chaque forme de résolution, chaque refus de 9.5, plan périmé côté cible et côté source ;
- base unique après une fusion déjà faite, refus de plusieurs bases ;
- abandon de la branche active, branche abandonnée ni active ni source, réactivation ;
- propriétés aléatoires : fusions et abandons dans les suites, à travers l’historique en mémoire et SQLite ;
- une fusion sans conflit d’une branche dans son origine non modifiée redonne exactement l’état de la branche ;
- l’état fusionné est toujours valide.
