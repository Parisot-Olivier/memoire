# Fiche de reprise — conception de l’incrément 6.5

Version 0.1 — 8 novembre 2026 — **validée par l’utilisateur le 9 novembre 2026** ; une décision nouvelle (D26, section 8) ; **implémentée** (6.5a noyau, 6.5b ligne de commande et interface ; `v0.6.0-alpha.5`). Précisions apportées à la mise en œuvre : section 12.
- Couvre CA-08 (« après arrêt et réouverture, retrouver l’objectif, l’état réel du code et la prochaine action »).
- Applique le cahier des charges :
  - section 3, étapes 1 et 7 : consulter la fiche à l’ouverture, noter un point de reprise court en fin de session ;
  - section 8 : contenu de la fiche, état du code, « signaler si le code a changé depuis » ;
  - section 10 : « revenir à un point de reprise », « la fiche conserve la référence du code ; la restauration du code reste une action de l’utilisateur dans Git » (D9).
- Cible d’usage à tester : retrouver en moins de deux minutes l’objectif, le blocage et la première action.

La section 9 regroupe les points validés.

---

## 1. Ce que l’utilisateur obtient

En fin de session :

```
memoire reprise noter --objectif "isoler le réveil perdu" \
    --action "instrumenter wake_task avec un compteur" \
    --question "le tick peut-il réveiller une tâche déjà prête ?" \
    --resultat "test_wakeup échoue 1 fois sur 20" --notes N-REVEIL
Point de reprise enregistré : 3f2a91c0-…
Code : a1b2c3d4 sur rtos-dev ; 2 fichiers modifiés, travail non enregistré conservé
```

À la réouverture (et dans l’interface, dès l’ouverture du projet) :

```
memoire reprise voir
Point de reprise 3f2a91c0 — branche main — 2026-11-08 17:42 UTC
Objectif : isoler le réveil perdu
Question : le tick peut-il réveiller une tâche déjà prête ?
Prochaine action : instrumenter wake_task avec un compteur
État atteint : —
Dernier résultat : test_wakeup échoue 1 fois sur 20
Hypothèse : —
Ouverts : —
Notes : Réveil perdu (9c01…)
Code noté : a1b2c3d4 sur rtos-dev ; 2 fichiers modifiés (memoire reprise code 3f2a91c0 pour les retrouver)
Depuis : réseau, 2 opérations ; code, 1 commit (a1b2c3d4 → e5f6a7b8), travail non enregistré différent
```

- Les trois informations de la cible d’usage (objectif, question en cours, prochaine action) viennent en tête.
- **Ce qui a changé depuis** est calculé à chaque lecture : opérations du réseau depuis la fiche, commits du code, travail non enregistré.
- **Revenir à un point de reprise** côté réseau : `restaurer --depuis 3f2a91c0` (incrément 6.4), puisque la fiche est un commit de l’historique. Côté code, c’est un geste de l’utilisateur dans Git (D9). `memoire reprise code 3f2a91c0` écrit le travail non enregistré noté sur la sortie standard, sous forme de patch : `memoire reprise code 3f2a91c0 | git apply`.

---

## 2. Contenu d’une fiche

| Champ | Option | Obligatoire | Cahier des charges |
|---|---|---|---|
| Objectif | `--objectif` | oui | objectif en cours |
| Prochaine action | `--action` | oui | prochaine action concrète |
| Question | `--question` | non | question précise en cours, blocage |
| État atteint | `--etat` | non | état réellement atteint |
| Dernier résultat | `--resultat` | non | compilation, exécution ou test pertinent |
| Hypothèse | `--hypothese` | non | hypothèse actuelle et incertitudes |
| Ouverts | `--ouverts` | non | fichiers et fonctions ouverts (texte libre) |
| Notes | `--notes NOTES` | non | passages ouverts : notes ou citations du réseau, par identifiant ou préfixe (R1) |
| Date | automatique | — | « à une date choisie » |
| État du code | automatique | — | référence de version et travail non enregistré |

- Deux champs obligatoires seulement : le point de reprise doit rester **court** (section 3, étape 7). Un champ obligatoire vide est refusé.
- `--notes` : chaque note doit exister dans l’état courant ; pas de doublon. Leur titre est relu à l’affichage dans l’état courant ; une note retirée depuis est signalée comme telle.
- **Date** : fournie par la ligne de commande, en UTC, à la minute (`2026-11-08 17:42 UTC`). Le noyau ne lit pas l’horloge : la date est une donnée de la commande, comme les identifiants (D3). Les commits restent sans date.

---

## 3. Noyau

**Une fiche est une opération du journal qui ne change pas le réseau** (D26) :
- nouvelle commande `Command::Resume(ResumePoint)` ;
- elle publie un commit dont l’état est **exactement** celui de son parent. Ce commit est l’identité de la fiche, et son état est le « contexte historique » du point de reprise ;
- jamais critique, jamais de bifurcation ; elle ne fait pas partie d’un lot ;
- elle apparaît dans `historique` (`3f2a91c0 — reprise — isoler le réveil perdu`) ;
- elle est annulable comme toute opération (voir le point 1 de la section 9).

**Lectures** :
- `Repository::resume_points(limit, all_branches)` : les fiches de la branche active, par premiers parents depuis la tête, de la plus récente à la plus ancienne ; avec `all_branches`, celles de chaque branche active, groupées par branche ;
- `Repository::resume_point(commit)` : une fiche du journal, quelle que soit sa branche ; refus si le commit n’est pas une fiche ;
- `Repository::operations_since(commit)` : nombre de commits de la branche active depuis la fiche. Aucun nombre si la fiche n’est pas dans l’historique de la branche active (annulée, ou notée sur une autre branche) : c’est signalé.

**État du code** : `CodeState { commit, branch, changes, patch }`, donnée opaque fournie par la ligne de commande :
- `commit`, `branch` : facultatifs (dépôt sans commit, tête détachée) ;
- `changes` : lignes d’état de Git (fichiers modifiés, ajoutés, supprimés, non suivis), triées ;
- `patch` : le travail non enregistré dans un commit, conservé tel quel, ou son empreinte et sa taille s’il dépasse 4 Mo ou s’il n’est pas en UTF-8. Précision apportée à la mise en œuvre (6.5a) : un `git diff` peut contenir des octets hors UTF-8, par exemple un fichier en Latin-1 ; il n’est alors pas conservé, plutôt que corrompu, et `reprise voir` le dit.

**Comparaison** (règle du noyau, testée et soumise à la mutation) : `CodeState::since(&noté, &actuel) -> CodeChange`. Les cas :
- code inchangé ;
- commit différent ;
- même commit, travail non enregistré différent ;
- dépôt Git absent aujourd’hui ;
- aucun état noté.

Le nombre de commits et l’ascendance sont fournis par la ligne de commande, seule à lire Git.

**Format v2** : commande `reprise` avec ses champs, encodés comme les autres textes (FORMAT.md). Refusée en v1, comme `restaurer-notes`. Export, réimport et sauvegarde emportent les fiches avec le journal, sans rien ajouter (CdC section 10 : « points de reprise, avec leurs identifiants »).

**Stockage** : aucun changement de schéma. Le journal (`operations`) porte déjà les commandes, et un commit sans différence est une différence vide. La base réelle n’a pas besoin de migration.

---

## 4. Lecture de l’état du code (ligne de commande)

La ligne de commande lit le dépôt Git en lecture seule, par l’exécutable `git` (D26), dans le dossier `--code DOSSIER` (par défaut le dossier courant) :

| Information | Commande Git |
|---|---|
| dépôt Git ? | `git -C D rev-parse --show-toplevel` |
| commit | `git -C D rev-parse --verify -q HEAD` |
| branche | `git -C D symbolic-ref --short -q HEAD` |
| fichiers changés | `git -C D --no-optional-locks status --porcelain=v1 -z --untracked-files=all` |
| travail non enregistré | `git -C D --no-optional-locks diff HEAD --binary --no-color --no-ext-diff` |
| commits depuis (à la lecture) | `git -C D rev-list --count X..HEAD`, `git -C D merge-base --is-ancestor X HEAD` |

- **Rien n’est écrit** :
  - ni dans le code (D9) ;
  - ni dans le dépôt Git : `--no-optional-locks` évite la mise à jour de l’index par `status` ;
  - aucun `stash`, aucune référence créée ;
  - `-c core.fsmonitor=false` empêche Git de lancer un programme configuré dans le dépôt.
- **Travail non enregistré** : le patch `--binary` se réapplique par `git apply`. Les fichiers non suivis sont **listés**, pas copiés.
- **Sans Git** (exécutable absent, dossier hors dépôt) : la fiche est enregistrée avec « état du code : non disponible », sans refus. `--sans-code` permet aussi de s’en passer.
- **Code de retour** : 3 si `git` échoue de façon inattendue sur un dépôt reconnu ; la fiche n’est alors pas enregistrée.

---

## 5. Ligne de commande

| Commande | Effet |
|---|---|
| `reprise noter --objectif X --action Y [--question Q] [--etat E] [--resultat R] [--hypothese H] [--ouverts O] [--notes NOTES] [--code DOSSIER \| --sans-code] [--motif M]` | enregistre un point de reprise ; sortie : `Point de reprise enregistré : <commit>`, puis une ligne `Code : …` |
| `reprise voir [--depuis COMMIT] [--code DOSSIER]` | la fiche la plus récente de la branche active (ou celle désignée), puis ce qui a changé depuis (section 1) ; « Aucun point de reprise sur cette branche » sinon, code 0 |
| `reprise liste [--toutes] [--nombre N]` | une ligne par fiche : `ID — date — objectif` ; `--toutes` : toutes les branches actives, groupées |
| `reprise code COMMIT` | écrit sur la sortie standard le patch du travail non enregistré noté ; code 1 si la fiche n’en a pas |

- `COMMIT` : identifiant complet, ou préfixe d’au moins 8 caractères, comme pour `restaurer`.
- **Contrat modifié** : quatre lignes de plus dans l’aide figée, soit 46 commandes.

---

## 6. Interface

- À l’ouverture, la fenêtre principale exécute `reprise voir` et affiche la fiche dans la zone de sortie s’il en existe une (CdC section 3, étape 1).
- Les quatre commandes apparaissent comme les autres ; `--notes` est un champ à plusieurs notes avec autocomplétion, `--code` un champ `DOSSIER` avec « Parcourir ».
- Les champs de texte restent sur une ligne, comme aujourd’hui ; une saisie sur plusieurs lignes est hors de 6.5.

---

## 7. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-DUR-01 | (existant) Après interruption, le dépôt correspond à l’état avant ou après une transaction |
| INV-RESUME-01 | Un point de reprise ne change pas le réseau : son commit a l’état de son parent ; il conserve, avec ses identifiants, l’état du réseau et l’état du code au moment où il est noté, après fermeture, export et réimport |
| INV-RESUME-02 | À la lecture d’un point de reprise, tout changement du code depuis (commit ou travail non enregistré) est signalé ; la lecture de l’état du code n’écrit ni dans le code ni dans le dépôt Git (D9) |

---

## 8. Décision D26

**D26 — Point de reprise : opération sans changement du réseau ; état du code lu par l’exécutable Git**

- **Contexte** : le cahier des charges demande une fiche de reprise qui conserve son contexte historique, la référence du code et un moyen de retrouver le travail non enregistré, et qui signale si le code a changé.
- **Décision** :
  - une fiche est une commande du journal (`Command::Resume`) qui publie un commit d’état identique à son parent ;
  - la ligne de commande lit l’état du code par l’exécutable `git`, en lecture seule, et le transmet au noyau comme une donnée ;
  - la date de la fiche est une donnée de la commande, en UTC.
- **Options écartées** :
  - **une fiche dans l’état du réseau** : elle serait soumise à la restauration, aux branches et à la fusion, alors qu’elle décrit une session et non une idée ;
  - **un journal à part** : changement de schéma, migration de la base, et un nouveau chemin d’export et de réimport ;
  - **une bibliothèque Git** (git2, gix) : dépendance lourde au titre de D14, pour quelques lectures ;
  - **`git stash`** : crée des références, donc écrit dans le dépôt Git.
- **Conséquences** :
  - aucun changement de schéma ;
  - l’export emporte les fiches ;
  - `restaurer --depuis` revient à un point de reprise ;
  - les essais de la ligne de commande demandent l’exécutable `git`, que `check.sh` vérifiera comme ses autres outils.

---

## 9. Points validés le 9 novembre 2026

1. **Une fiche est une opération**, avec un commit sans changement du réseau (D26) : elle figure dans `historique`, sert de point de retour pour `restaurer`, et **`annuler` la défait si c’est la dernière opération**.
   - Compromis : à la session suivante, un `annuler` destiné à la dernière modification d’idée défera d’abord la fiche. La sortie le dit (`Annulé : point de reprise « isoler le réveil perdu »`), et un second `annuler` atteint la modification.
   - L’autre voie, une fiche hors de l’annulation, demanderait un journal à part (section 8, options écartées).
2. **Contenu** : objectif et prochaine action obligatoires, le reste facultatif (section 2) ; `--notes` vers le réseau, `--ouverts` en texte libre.
3. **Lecture de Git par l’exécutable `git`**, en lecture seule (section 4) :
   - le travail non enregistré est conservé sous forme de patch dans la base, jusqu’à 4 Mo ; il est donc aussi dans les exports et les sauvegardes ;
   - les fichiers non suivis sont seulement listés.
4. **Date en UTC à la minute**, fournie par la ligne de commande.
5. **Quatre commandes** (`reprise noter`, `reprise voir`, `reprise liste`, `reprise code`), soit 46 ; affichage de la dernière fiche à l’ouverture de l’interface.

---

## 10. Découpage et tests

| Étape | Contenu |
|---|---|
| 6.5a | noyau : `Command::Resume`, `ResumePoint`, `CodeState::since`, `resume_points`, `resume_point`, `operations_since`, format v2 ; contrats, propriétés (fiches dans les suites aléatoires : état inchangé, annulation exacte) ; mutation sans survivant |
| 6.5b | ligne de commande : lecture de Git, quatre commandes ; interface ; documents, D26 ; contrôle complet ; livraison `v0.6.0-alpha.5` |

**Tests prévus :**
- **Noyau :**
  - une fiche ne change pas l’état ; champs obligatoires vides refusés ; note inconnue ou en double refusée ;
  - fiche annulée puis rétablie ;
  - fiches de plusieurs branches ;
  - chaque cas de `CodeState::since` ;
  - aller-retour du format v2, refus en v1.
- **Ligne de commande** (dépôt Git d’essai créé par le test dans un dossier temporaire) :
  - fiche notée, base fermée puis rouverte, fiche relue à l’identique (CA-08, INV-RESUME-01) ;
  - commit ajouté, puis travail modifié : chacun signalé (INV-RESUME-02) ;
  - `reprise code` rend un patch que `git apply` réapplique sur une copie propre ;
  - dépôt Git et fichiers identiques avant et après (`.git/index` compris) ;
  - dossier hors Git, `--sans-code` ;
  - `restaurer --depuis` une fiche.
- **Interface** : la fiche s’affiche à l’ouverture ; rien ne s’affiche sans fiche.

---

## 11. Hors de 6.5

- Préparer le contexte à copier dans une session d’IA (CdC section 9) : la fiche en sera une source naturelle.
- Saisie sur plusieurs lignes dans l’interface.
- Copier les fichiers non suivis.
- Proposer une fiche automatiquement à la fermeture de l’interface.

---

## 12. Précisions apportées à la mise en œuvre

1. **Patch hors UTF-8** (6.5a) : non conservé, seulement sa taille et son empreinte (`Patch::NotKept`), plutôt que corrompu (section 3).
2. **`--code` et `--sans-code`** sont deux options facultatives, refusées ensemble (code 2), et non une alternative : le lecteur de l’aide de l’interface n’admet dans une alternative que des options à valeur.
3. **La base et ses fichiers associés** (`-wal`, `-shm`, `-journal`), s’ils sont dans le dépôt du code, ne comptent pas parmi ses changements : sans cela, une base placée dans `.memoire/` non ignoré ferait paraître le code changé à chaque lecture.
4. **En-tête de `reprise voir`** : `Point de reprise <commit> — <date>`, sans le nom de branche, qu’une fiche désignée par `--depuis` ne permet pas toujours de connaître ; la ligne « Depuis » dit si la fiche est dans l’historique de la branche active.
5. **`reprise code`** sur une fiche sans état du code ou sans patch conservé : code 2, avec le motif (le code 1 est réservé aux refus du noyau).
6. **Notes de la fiche** : affichées comme ailleurs (« titre (id) ») ; une note absente de l’état courant n’a que son identifiant.
7. **Fichiers non suivis** : listés par leur présence ; une modification de leur contenu n’est pas un changement du travail noté (section 4).
8. **Lecture de Git** : `GIT_DIR`, `GIT_WORK_TREE` et `GIT_INDEX_FILE` sont retirés de l’environnement de `git`, pour que le dossier désigné ne soit jamais remplacé par un dépôt imposé ; les préfixes `a/` et `b/` du patch sont imposés, quelle que soit la configuration de l’utilisateur, pour que `git apply` le relise.

