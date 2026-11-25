# Notes de reprise pour un développeur

Version du 14 novembre 2026 (incrément 8, apparence, livré : `v0.8.0-alpha.2`). Ce document permet de reprendre le projet sans l’historique des échanges qui l’ont construit. Lire d’abord la section 1, puis les sections 5 et 6.

---

## 1. Où en est le projet

**Le logiciel.** `memoire` est un outil personnel de mémoire de conception, un « Git pour les idées » : il conserve les échanges avec une IA, les notes, les décisions et leurs relations, signale ce qui doit être réexaminé quand une hypothèse tombe, relie les notes au code par des repères, et garde tout l’historique (branches, annulation, restauration). Premier usage visé : un mini-RTOS (Evalbot, Cortex-M3).

**Les branches du dépôt :**

| Branche | État | Contenu |
|---|---|---|
| `main` (étiquette `v0.8.0-alpha.2`) | **dernière version complète et vérifiée** | domaine (dont les opérations groupées, `Command::Lot`, l’abandon de branche, la fusion à trois voies, les citations de notes et les orphelines), stockage SQLite au schéma 2 (état courant et historique par différences, commandes indépendantes de la longueur de l’historique), migration du schéma 1, couche service, ligne de commande (48 commandes, dont l’indexeur de repères `indexer`, l’historique, la restauration ciblée, la fiche de reprise et la vue locale `voisinage`), interface Tkinter à l’apparence « memoire » (APPARENCE.md : rail des sujets, carte papier, barre d’état, panneaux Aperçu et Voisinage, lecture dans la carte) avec visionneur (rendu Markdown, surlignage, citation par sélection, citer et relier en un geste) et vue locale colorée, plan de travail modulaire (panneaux ancrables ou flottants) et terminal intégré, format textuel versions 1 et 2 ; mutation sans survivant sur les quatre crates (`VERIFICATION-PHASE5.txt`) |
| `inc-5.0` | documents seulement | conceptions validées de la phase 5 : `VERSION2.md`, `FUSION.md`, décision D19, précision de D4 |
| `inc-5.1` | fusionnée dans `main` | incrément 5.1 complet (section 6) |
| `inc-5.2` | fusionnée dans `main` (`v0.5.0-alpha.2`) |
| `inc-8.5` | **en cours, non livrée** (part de `inc-8.4`) | console en mosaïque (R21, CONSOLE.md, D32) : conception, contrat et code faits ; retouches 8.5b (R22, CONSOLE.md, section 11 : tuiles en cadres classiques, thème non pavé, pas du glisser de 103 à 18 ms sous Xvfb) ; essais sans affichage et graphiques (Xvfb, KWin virtuel) réussis ; reste le contrôle complet et la livraison, sur décision de l’utilisateur |
| `inc-8.4` | **non livrée** | terminal en onglets (remplacés en 8.5) et complétion par Tab (R19, R20, APPARENCE.md, section 11) |
| `inc-8.3` | fusionnée dans `main` (`v0.8.0-alpha.2`) | lecture conforme à la maquette (R18, APPARENCE.md, section 10) |
| `inc-8.1` | fusionnée dans `main` (`v0.8.0-alpha.1`) | apparence (R15 à R17, APPARENCE.md, D31) : thème dessiné en Python pur, fenêtre principale recomposée, affichage rapide (8.1c), menus, panneaux Aperçu et Voisinage, lecture dans la carte (8.2) ; essais graphiques réussis sous Xvfb et sous KWin virtuel |
| `inc-7.2` | fusionnée dans `main` (`v0.7.0-alpha.2`) | retrouver une note par son nom ou par le code (R14) |
| `inc-7.1` | fusionnée dans `main` (`v0.7.0-alpha.1`) | panneau des actions : menus par sujet, vocabulaire simple, « Et ensuite ? », réfuter et adopter une piste guidés (R13, D30) |
| `inc-6.9` | fusionnée dans `main` (`v0.6.0-alpha.9`) | plusieurs terminaux (R12, D29) |
| `inc-6.8` | fusionnée dans `main` (`v0.6.0-alpha.8`) | ordre des fenêtres flottantes ; essais graphiques sous KWin virtuel |
| `inc-6.7` | fusionnée dans `main` (`v0.6.0-alpha.7`) | plan de travail modulaire et terminal intégré (section 7, étape 5) |
| `inc-6.6` | fusionnée dans `main` (`v0.6.0-alpha.6`) | vue locale colorée (section 7, étape 5) |
| `inc-6.5` | fusionnée dans `main` (`v0.6.0-alpha.5`) | fiche de reprise ; mise à jour de sécurité de rusqlite (section 7, étape 5) |
| `inc-6.4` | fusionnée dans `main` (`v0.6.0-alpha.4`) | restauration ciblée et historique (section 7, étape 5) |
| `inc-6.3` | fusionnée dans `main` (`v0.6.0-alpha.3`) | indexeur de repères (section 7, étape 5) |
| `inc-6.2` | fusionnée dans `main` (`v0.6.0-alpha.2`) | centre de travail (section 7, étape 5) |
| `inc-6.1` | fusionnée dans `main` (`v0.6.0-alpha.1`) | visionneur (section 7, étape 5) |
| `inc-5.5` | fusionnée dans `main` (`v0.5.0-alpha.5`) | citations, échanges dans les dépendances, orphelines (section 7, étape 4) |
| `inc-5.4` | fusionnée dans `main` (`v0.5.0-alpha.4`) | abandon de branche et fusion (section 7, étape 3) |
| `inc-5.3` | fusionnée dans `main` (`v0.5.0-alpha.3`) | opérations groupées (section 7, étape 2) | historique hors mémoire, schéma SQLite 2, `memoire verifier`, migration du schéma 1 (`memoire migrer`) ; section 7, étape 1 |

**Événement à connaître.** Le 3 novembre 2026, l’environnement de développement a été réinitialisé. Tout ce qui suivait `v0.4.0-alpha.4` a été perdu, puis reconstitué à partir de l’historique des échanges : les conceptions de la phase 5 à l’identique de leur contenu validé, l’instantané version 2 en rejouant les modifications d’origine. Le reste de l’incrément 5.1, qui était presque entièrement vérifié avant la perte, a été refait le 5 novembre 2026, puis vérifié et livré (section 6). **Leçon :** archiver le travail hors de l’environnement après chaque étape validée.

---

## 2. Documents de référence

| Document | Rôle |
|---|---|
| `CAHIER_DES_CHARGES.md` | besoin fonctionnel (v0.2), critères d’acceptation CA-01 à CA-40 |
| `DECISIONS.md` | décisions D1 à D32, chacune avec contexte, options écartées et conséquences |
| `INVARIANTS.md` | registre des invariants (INV-…), avancement, résultats de chaque vérification |
| `TRACABILITE.md` | critères d’acceptation ↔ invariants ↔ tests |
| `SCENARIOS.md` | scénarios, dont S-REF-01 (scénario de référence sur `fixtures/rtos-2026-10/`) |
| `FORMAT.md` | format textuel canonique, version 1 (référence normative) |
| `VERSION2.md` | spécification **validée** de la version 2 : stockage en deux parties, format, migration |
| `ETAT-HISTORIQUE.md` | spécification technique de l’incrément 5.2 (étape 2 de VERSION2.md), version 0.2 **validée** avec D20 et D21 |
| `FUSION.md` | conception **validée** et **implémentée** de la fusion à trois voies (D19 ; précisions de la section 9, D23) |
| `PLAN-DE-TRAVAIL.md` | conception **validée** et **implémentée** du plan de travail modulaire (D28) |
| `RETROUVER.md` | conception **validée** et **implémentée** : suggestions par nom, `repere` toutes syntaxes, `code notes` |
| `APPARENCE.md` | conception **validée** (maquette du 14 novembre 2026) de l’apparence : thème, fenêtre principale recomposée (D31) |
| `ACTIONS.md` | conception **validée** et **implémentée** du panneau des actions : menus, vocabulaire de l’interface, parcours guidés (D30) |
| `CONSOLE.md` | conception **validée** (maquette interactive du 16 novembre 2026) et **implémentée** de la console en mosaïque (D32, qui remplace D29 pour la place des terminaux) |
| `TERMINAUX.md` | conception **validée** et **implémentée** de plusieurs terminaux (D29) : nommage et conservation restent, les panneaux par terminal sont remplacés par CONSOLE.md |
| `TERMINAL.md` | conception **validée** et **implémentée** du terminal intégré (D27) |
| `VOISINAGE.md` | conception **validée** et **implémentée** de la vue locale colorée (R7) |
| `FICHE-REPRISE.md` | conception **validée** et **implémentée** de la fiche de reprise (D26) |
| `RESTAURATION.md` | conception **validée** et **implémentée** de la restauration ciblée et de l’historique |
| `INDEXEUR.md` | conception **validée** et **implémentée** de l’indexeur de repères |
| `CENTRE.md` | conception **validée** et **implémentée** du centre de travail : citer et relier en un geste |
| `VISIONNEUR.md` | conception **validée** et **implémentée** du visionneur (D25) |
| `CITATIONS.md` | conception **validée** et **implémentée** des citations, des échanges dans les dépendances et des orphelines (D24) |
| `STOCKAGE.md`, `SERVICE.md`, `CLI.md`, `INTERFACE.md` | conception de chaque couche |
| `RETOURS.md` | retours d’usage de l’utilisateur (R1 à R10) et leur suite |
| `DEPENDANCES.md` | registre d’audit des dépendances (D14) |
| `VERIFICATION-PHASE*.txt` | journaux des contrôles complets |

---

## 3. Architecture

| Crate | Rôle | Contraintes |
|---|---|---|
| `memoire-domain` | toutes les règles : identités, commandes, branches, contextes, statuts, relations, réexamen, sources, ancrages, format textuel | **aucune dépendance, aucune entrée-sortie** (vérifié par `tests/architecture.rs`) ; déterministe ; identifiants fournis par l’appelant |
| `memoire-store` | stockage SQLite (`rusqlite` figé), schéma 2 : état courant de chaque branche et historique par différences, implémente le trait `History` du domaine (D20) ; une transaction par commande ; vérification complète sur demande (D21) ; migration du schéma 1 | aucune règle métier ; toutes les lignes stockées sont produites par le domaine |
| `memoire-app` | couche service : UUID v4, préparer puis appliquer, répétition sûre, export et import, sauvegarde | aucune règle métier |
| `memoire-cli` | exécutable `memoire` : 48 commandes, table extensible (`CLI.md`, section 7) | passe uniquement par la couche service |
| `gui/` | interface Tkinter : un bouton par commande, formulaires construits depuis `memoire aide` ; visionneur, vue locale ; plan de travail à panneaux et terminal intégré (moteurs sans affichage : `terminal_moteur.py`, `plan_de_travail.py`) | aucune règle métier ; pilote l’exécutable |

---

## 4. Mise en place de l’environnement

```
# Rust 1.91 (Ubuntu 24.04 : paquets versionnés)
apt-get install rustc-1.91 cargo-1.91 rust-1.91-clippy rustfmt-1.91 python3-tk xvfb
export PATH=/usr/lib/rust-1.91/bin:$HOME/.cargo/bin:$PATH
# Outils de vérification
cargo install --locked cargo-mutants
cargo install --locked cargo-audit --version 0.22.2
# Dépendances verrouillées (réseau requis la première fois)
cargo fetch --locked
```

**Commandes utiles :**

| Commande | Effet |
|---|---|
| `make` | construit l’exécutable et ouvre l’interface (`make lancer BASE=…` pour choisir la base) |
| `make tests` | tests Rust et Python |
| `./tools/check.sh` | contrôle complet : formatage, cohérence documentaire, dépendances (D14), audit RustSec, clippy, tests debug et release, essais de l’interface, mutation des quatre crates, régressions historiques |
| `python3 tools/deps.py --refresh` | réaudite les dépendances après une mise à jour de `Cargo.lock` |

**Variables de `check.sh` :** `CHECK_MUTANTS_SHARDS` (parts du domaine, 8 recommandé), `CHECK_CLI_SHARDS` (4 par défaut), `CHECK_JOBS` (1 sur une machine à un cœur), `CHECK_MUTANTS_DIR` (conserver les parts réussies hors de la copie de travail), `CHECK_GARDER_TMP=1` (ne pas nettoyer `/tmp`), `CHECK_PLAFOND_KIO` (plafond d’espace d’adressage de chaque binaire de test pendant la mutation, 4 Gio par défaut, `tools/lanceur-mutation.sh` : un mutant a déjà épuisé la mémoire de la machine), `CHECK_TMPDIR_TESTS` (répertoire temporaire des binaires de test pendant la mutation, `/tmp` par défaut). Si `/tmp` est en mémoire vive, lancer avec `TMPDIR` sur disque (`TMPDIR=~/tmp-mutants`) : les copies de cargo-mutants y vont, les bases d’essai restent dans `CHECK_TMPDIR_TESTS`. Une part réussie est consignée sous une clé calculée sur le code et le contrat ; elle n’est rejouée que si l’un d’eux change.

**Durées sur une machine à un cœur :** mutation du domaine 70 à 80 min en 8 parts ; stockage et service quelques minutes ; ligne de commande environ 45 min en 4 parts. Vérifier dans une copie figée : `git worktree add --detach ../verif <branche>`.

---

## 5. Règles de travail du projet

1. **Conception avant code.** Toute fonction nouvelle a d’abord un document validé par l’utilisateur ; toute décision structurante une fiche dans `DECISIONS.md`.
2. **Contrat avant code.** Un commit « contrat » (tests seuls, qui peuvent ne pas compiler), puis un commit de code. Un test existant n’est modifié que par un commit qui dit « Contrat modifié » et pourquoi.
3. **Mutation sans survivant.** Tout survivant est éliminé par un test (jamais en affaiblissant une assertion) ou, s’il est réellement inobservable, par une exclusion justifiée par écrit (seul cas actuel : `sync_directory`, voir `check.sh`).
4. **Exemples figés.** Les fichiers de `fixtures/format/` sont générés par le code, relus ligne à ligne contre la spécification, puis figés ; un changement est un changement de contrat.
5. **Aucune suppression (D6)**, domaine sans entrée-sortie ni dépendance (D14), motif obligatoire pour tout changement de statut (D12), criticité selon l’effet réel (D13).
6. **Dépendances (D14) :** versions publiées depuis au moins 14 jours, figées par `Cargo.lock`, auditées.
7. **Garder le contrat rapide** (environ 2 à 3 s) : chaque mutant l’exécute en entier ; un test lent multiplie la durée de la mutation.
8. **Clippy avant chaque commit**, et vérifier que chaque modification de fichier a réellement été appliquée.

---

## 6. Incrément 5.1 : format version 2 et migration — livré (`v0.5.0-alpha.1`)

Référence normative : `FORMAT.md`, section 13. Reconstitué le 5 novembre 2026 en suivant le plan de la version précédente de ce document, dans l'ordre : contrat de l'instantané v2, différences (`difference.rs`), contrat puis code du dépôt v2 et de la migration, contrat puis code de l'export et de l'import, documents, outillage, vérification.

**Ce qui existe :**
- `difference.rs` : `Cle`, `PartieNote`, `Element` (ordre dérivé = ordre du texte), `cle`, `lignes`, `assembler`, `difference`.
- `depot.rs` : `encode_repository_v2`, `decode_repository_v2`, `decode_repository_any`, `migrate_v1_to_v2` (contrôlée par `verifier_migration`, testée à part), `profondeurs`, `REPOSITORY_HEADER_V2`.
- `Service::export` écrit la v2 ; `Store::import` lit les deux versions. Le stockage SQLite reste en fragments v1.
- `check.sh` : `CHECK_MUTATION_SEULE=1`.
- Exemples figés `exemple-v2.txt` et `depot-v2.txt`, **dérivés à la main** des exemples v1 (le second par un script indépendant du code), puis comparés à l'encodeur.

**Choix faits là où le plan laissait une latitude** (tous décrits dans `FORMAT.md`, section 13) :
- différence : toutes les lignes retirées, puis toutes les lignes ajoutées, chaque groupe par clé croissante ;
- motifs : `Inconsistent("différence minimale")` pour une ligne retirée puis remise ; `NotYetSupported` nomme « opérations groupées », « fusion », « abandon de branche », « réactivation de branche », « citation d'une note » ; `parents` supérieur à 2 donne `Value("parents")` ;
- `Inconsistent("état de référence")` est situé à la ligne `reference` ou `difference` ; un parent absent ou un cycle donne `Inconsistent("parent de commit")` à la ligne 0 ;
- une erreur d'un état reconstruit par différence est située à la ligne ajoutée qui l'a introduite, sinon à la ligne `difference` ;
- `profondeurs` numérote par `zip` (exigé par clippy) : le mutant « += remplacé par *= » cité par le plan n'existe plus, mais le test à 102 opérations décroissantes est conservé ;
- la lecture d'un bloc de commit délimite ses lignes d'un seul tenant : l'ancienne boucle d'avancement donnait un mutant sans fin, compté comme dépassement de délai, donc comme échec du contrôle.

**Vérification :** contrôle complet réussi sur le commit 833b512 (`VERIFICATION-PHASE5.txt`) ; domaine 745 mutants, 659 détectés, 86 non compilables, aucun survivant (l'essai perdu donnait 758, 668, 90 : l'écart vient de la structure du code reconstitué). Essai graphique sauté faute de `xvfb-run` sur la machine de vérification.

---

## 7. Après l’incrément 5.1 (ordre validé par l’utilisateur)

1. **Étape 2 de `VERSION2.md`**, spécifiée dans `ETAT-HISTORIQUE.md` (incréments 5.2a à 5.2e ; décisions D20 et D21 ; précisions d’implémentation aux sections 14 à 16). **Livrée le 6 novembre 2026 (`v0.5.0-alpha.2`) :**
   - 5.2a (mesure de référence), 5.2b (historique derrière le trait `History`), 5.2c (schéma 2, `memoire verifier`), 5.2d (migration, `memoire migrer`, question de l’interface) : faits ; contrôle complet réussi au commit 77febbe (`VERIFICATION-PHASE5.txt`) ; oracle `memoire-reference` retiré ensuite, comme prévu.
   - Objectifs atteints (mesure de 5.2c) : à 5 000 opérations et 1 500 notes, `statut` 0,026 s et une écriture 0,098 s (33,5 s et 33,4 s avant) ; base 72 Mo (2,36 Go avant) ; à état constant, durées indépendantes de la longueur de l’historique.
   - Base réelle de l’utilisateur migrée avec son accord ; copie d’avant la migration conservée à part.
   - 5.2e : mesure finale (toutes les commandes sous 0,11 s à 5 000 opérations et 1 500 notes), `STOCKAGE.md` réécrit pour le schéma 2, contrôle complet réussi au commit 4910e01.
2. **Opérations groupées** (commande `lot`) : débloque R1 (créer et lier en un geste). **Livrée le 6 novembre 2026 (`v0.5.0-alpha.3`)**, selon `LOT.md` (D22) : `Command::Lot` dans le noyau (tout ou rien, un commit, un `annuler`, criticité sur l’effet net), bloc `lot` du format v2, `note creer` avec liens, autocomplétion à plusieurs notes ; contrôle complet réussi au commit 47ef3d5. Reporté : `memoire lot FICHIER` (D22).
3. **Fusion** selon `FUSION.md` et D19, avec sauvegarde automatique, abandon et réactivation de branche. **Livrée le 6 novembre 2026 (`v0.5.0-alpha.4`)**, selon la section 9 de `FUSION.md` (D23) : 5.4a `abandonner`, `reactiver`, `branches --toutes` ; 5.4b noyau (`Command::Merge`, `Repository::merge_plan`, base commune unique, conflits de texte, de champ et d’acquittement, conflit structurel résolu par désactivation, commit à deux parents) ; 5.4c second parent dans SQLite, `fusion plan` et `fusionner` (code du plan, sauvegarde `avant-fusion-…`) ; contrôle complet réussi au commit c928f3b. La forme de résolution `relation` a été retirée avec l’accord de l’utilisateur (aucun conflit possible sur une relation). 
4. **Citations (D18)** : citation d’une note (version citée, empreinte, signalement « source révisée »), lien automatique vers la source, échanges dans les dépendances, liste « Orphelines » (R6), commande `citer`. **Livrée le 6 novembre 2026 (`v0.5.0-alpha.5`)**, selon `CITATIONS.md` (D24) : relations vers les échanges, `Change::CiteNote`, `Graph::orphans` (les orphelines sortent des ressources actuelles ; S-REF-01 étape 9 reformulée), `citer` (remplace `passage`, crée la relation vers la source dans le même lot), `retirer`, section « Orphelines » de `statut` ; contrôle complet réussi au second passage, commit 765377c. 
5. Ensuite : visionneur avec rendu Markdown et surlignage (R2) — **livré le 6 novembre 2026 (`v0.6.0-alpha.1`)** selon `VISIONNEUR.md` (D25) : `source`, `citations`, `citer --plage/--version`, `gui/rendu.py`, `gui/visionneur.py` ; contrôle complet réussi au commit c996d96 (`VERIFICATION-PHASE6.txt`) ; centre de travail (citer et relier en un geste) **livré le 8 novembre 2026 (`v0.6.0-alpha.2`)** selon `CENTRE.md` (contrôle complet réussi au commit 6bf944b) ; indexeur de repères (CA-05, CA-30, CA-31) **livré le 8 novembre 2026 (`v0.6.0-alpha.3`)** selon `INDEXEUR.md` (contrôle complet réussi au commit c82cf22) ; restauration ciblée et historique (CA-27, CA-33) **livrés le 8 novembre 2026 (`v0.6.0-alpha.4`)** selon `RESTAURATION.md` (contrôle complet réussi au commit 298cb78) ; fiche de reprise (CA-08) **livrée le 9 novembre 2026 (`v0.6.0-alpha.5`)** selon `FICHE-REPRISE.md` (D26), avec la mise à jour de sécurité de rusqlite 0.41.0 (RUSTSEC-2026-0459) ; vue locale colorée (R7) **livrée le 10 novembre 2026 (`v0.6.0-alpha.6`)** selon `VOISINAGE.md` ; plan de travail modulaire et terminal intégré (R5, D27, D28) **livrés le 10 novembre 2026 (`v0.6.0-alpha.7`)** selon `PLAN-DE-TRAVAIL.md` et `TERMINAL.md` ; ordre des fenêtres flottantes (toujours au premier plan, mettre devant, mettre derrière ; PLAN-DE-TRAVAIL.md, section 10) **livré le 10 novembre 2026 (`v0.6.0-alpha.8`)** ; plusieurs terminaux nommés par leur commande, panneau « Terminaux » et menu « Terminaux ▾ » (R12, TERMINAUX.md, D29) **livrés le 10 novembre 2026 (`v0.6.0-alpha.9`)** ; panneau des actions (R13, ACTIONS.md, D30 : menu par sujet, vocabulaire simple — projet, piste, étape, idée, adopter une piste… —, questions courtes, cases à cocher, « Ce que ça va changer », « Et ensuite ? ») **livré le 10 novembre 2026 (`v0.7.0-alpha.1`)** ; retrouver une note par son nom (suggestions pendant la frappe, « nom · id8 ») ou par le code (`repere` toutes syntaxes, `code notes`) (R14, RETROUVER.md) **livré le 11 novembre 2026 (`v0.7.0-alpha.2`)** ; feuille de route de la section 7 achevée ; apparence (R15 à R17, APPARENCE.md, D31) **livrée le 14 novembre 2026 (`v0.8.0-alpha.1`)** : thème « memoire » dessiné en Python pur, fenêtre principale recomposée, affichage rapide (images opaques), menus en carte, panneaux Aperçu et Voisinage, lecture dans la carte ; lecture conforme à la maquette **livrée le 14 novembre 2026 (`v0.8.0-alpha.2`)** ; terminal en onglets et complétion par Tab (8.4, branche `inc-8.4`, non livrée) ; console en mosaïque (8.5, branche `inc-8.5`, R21, CONSOLE.md, D32 : une console unique sur le côté, en pleine page ou en fenêtre, mosaïque de un à quatre terminaux, point milieu, terminaux en arrière-plan en pastilles), à livrer avec 8.4 ; à venir : glisser-déposer des panneaux.

---

## 8. Limites connues

- La lecture d’une base de schéma 1 par la migration charge tout le dépôt en mémoire, comme l’ouverture de `v0.5.0-alpha.1` : coût unique.
- La ligne de commande et l’interface ne travaillent que sur le contexte par défaut ; plusieurs commandes du noyau n’y sont pas exposées (contextes, remplacer, réattacher, détacher) ; `retirer` (5.5) et `restaurer` (6.4) le sont désormais.
- Le code Python de l’interface n’est pas soumis à la mutation ; sa logique est isolée dans `gui/schema.py` et testée.
- Les versions des crates n’ont pas été réalignées (`memoire-domain` affiche 0.2.0).
- Le décodage complet d’un texte v2 (`decode_repository_v2`, import) garde en mémoire l’état de chaque commit ; la vérification complète (`verify_repository_v2`, `memoire verifier`, export) est, elle, à mémoire bornée.
- L’utilisateur lance souvent l’interface (`make`) depuis la racine du projet pendant le développement : `memoire/.memoire/` est alors sa base de session (non versionnée), et ses terminaux créent des dossiers `/tmp/memoire-terminal-*` ; ne pas les prendre pour des résidus d’essais.
- Console (D32) : quatre terminaux au plus sur la page ; pas d’échange de tuiles par glisser ; une disposition enregistrée avant 8.5 (panneaux `terminal-N`) ne retrouve pas la place des anciens terminaux, la console prend une fois sa place par défaut. Maquettes : artifacts Design du compte de l’utilisateur (apparence du 14 novembre, console du 16 novembre 2026).
- Le terminal intégré (D27) n’est pas un émulateur complet : ni `vim`, ni `less`, ni `top` ; fin de commande suivie pour bash seulement ; Linux et macOS. La disposition des panneaux est dans `~/.config/memoire/plan-de-travail.json`. Le visionneur et la vue locale restent des fenêtres, pas encore des panneaux.
- La fiche de reprise lit Git par l’exécutable `git` (D26) : `check.sh` l’exige. Le dossier du code comparé par `reprise voir` est `--code`, sinon le répertoire courant (celui de lancement pour l’interface) : lancer depuis un autre dossier compare avec un autre dépôt. Le contenu des fichiers non suivis n’est pas noté, seulement leur présence.
- Les essais graphiques tournent sous KWin virtuel (`tools/essais-sous-kwin.sh`) quand `kwin_wayland` est installé : seul un vrai gestionnaire de fenêtres permet de vérifier l’ordre des fenêtres flottantes ; sous Xvfb, cet essai est sauté.
- Sans `xvfb-run`, `check.sh` saute l'essai graphique de l'interface (il le signale) : l'installer sur la machine de vérification (`xorg-x11-server-Xvfb` sous Fedora, `xvfb` sous Debian et Ubuntu).
