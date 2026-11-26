# memoire — un Git pour les idées

**Français** · [English](README.md)

Ce logiciel a été entièrement développé avec Claude Opus 5.5 et Claude Fable 5.2, à partir d'un cahier des charges défini par mes soins avec l'aide de ChatGPT 6.1 Astra, qui a également fourni les tests de base de l'architecture. Le design a été entièrement réalisé par moi-même, avec quelques adaptations apportées par Claude Fable 5.2.

**memoire** est un logiciel de bureau pour garder la mémoire d'un travail de conception : les échanges avec une IA, les notes, les décisions, leurs relations et leur lien avec le code. Il aide à retrouver *pourquoi* une chose a été conçue ainsi, et *où reprendre* le travail.

Tout reste **local** : une base SQLite par projet, sans compte ni service en ligne.

## Ce que fait le logiciel

- **Conserver les échanges** avec une IA, octet pour octet, dans leur ordre : question et réponse d'origine.
- **Citer et annoter** un passage précis, sans accumuler de commentaires à la suite.
- **Relier les notes** : classé dans, dépend de, développé à partir de, justifie, restreint, contredit.
- **Suivre l'état des idées** : vérifiée ou réfutée, proposée ou retenue, implémentée ou non. Rien n'est jamais supprimé : une idée « plus retenue » reste consultable.
- **Signaler ce qu'il faut réexaminer** quand une note change ou est réfutée : ses dépendantes sont signalées, avec la cause.
- **Ancrer une note dans le code** par un repère (`@design:`) et retrouver ce qui a bougé.
- **Garder l'historique** comme dans Git : branches, annuler, rétablir, fusionner, restaurer un ancien état.
- **Reprendre le travail** : un point de reprise enregistre l'objectif, la prochaine action et l'état du code.

## Prérequis

| Outil | Version | Remarque |
|---|---|---|
| `make` | — | seul point d'entrée |
| Rust (`cargo`) | 1.91 ou plus | à installer avec [rustup](https://rustup.rs) |
| Python 3 | récent | avec **Tkinter** pour l’interface |

Tkinter s'installe avec le système : paquet `python3-tk` sous Debian et Ubuntu, `python3-tkinter` sous Fedora. La première construction télécharge les dépendances Rust (versions figées par `Cargo.lock`) ; il faut donc un accès réseau cette fois-là.

## Lancer le programme

```sh
make
```

Cette commande construit l'exécutable (mode release, versions figées) puis ouvre la fenêtre.

### Premier lancement

1. Dans la fenêtre, ouvrez le sujet **Projet** et choisissez **Créer le projet**. Cela crée le fichier de base `.memoire/depot.db` dans le dossier d'où vous avez lancé `make`.
2. Utilisez ensuite les sujets du menu à gauche : **Écrire**, **Relier**, **Juger**, **Lire**, **Code**, **Histoire et pistes**, **Où j'en suis**. Chaque action ouvre un formulaire et, pour les opérations sensibles, montre un aperçu avant d'écrire quoi que ce soit.
3. Le **mode expert** offre un formulaire pour chaque commande de la ligne de commande.

### Choisir ou changer la base

```sh
make lancer BASE=/chemin/vers/mon-projet/.memoire/depot.db
```

Dans l'interface, **Changer de projet…** permet aussi de passer d'une base à une autre. Un projet correspond à une base.

> Une base contient vos conversations : elle ne doit pas être publiée. Le dossier `.memoire/` est exclu par `.gitignore`.

## Autres cibles `make`

| Commande | Effet |
|---|---|
| `make` ou `make lancer` | construit puis ouvre l'interface |
| `make construire` | construit seulement l'exécutable `target/release/memoire` |
| `make tests` | tests Rust et Python, sur des données de test fictives (voir « Données de test fictives »). L'interface est testée **sans afficher de fenêtre** si `xvfb-run` est installé (paquet `xorg-x11-server-Xvfb` sous Fedora, `xvfb` sous Debian et Ubuntu) ; `make tests VIRTUEL=0` force l'écran courant. Un test, celui de l'ordre des fenêtres, est ignoré faute de gestionnaire de fenêtres : `MEMOIRE=$PWD/target/release/memoire tools/essais-sous-kwin.sh` l'exécute avec KWin virtuel. |
| `make verifier` | contrôle complet (formatage, dépendances, lints, tests, mutation) ; long, demande des outils supplémentaires (voir `tools/check.sh`) |
| `make nettoyer` | supprime les produits de compilation |
| `make aide` | liste les cibles |

## Utilisation avec une IA (expérimental)

**L'idée.** Une IA de programmation (un agent qui a accès à un terminal) construit dans memoire le **graphe de ce qu'elle va développer** : une note par intention, des liens « dépend de » entre elles, puis, une fois le code écrit, un **repère** qui rattache chaque morceau de code à sa note. L'humain relit ensuite le graphe pour **comprendre ce que l'IA a fait et pourquoi**, au lieu de relire tout le code.

> **Guide complet pour l'IA : [`SKILLS/GUIDE-IA.md`](SKILLS/GUIDE-IA.md)** (français ; version anglaise : [`SKILLS/GUIDE-AI.md`](SKILLS/GUIDE-AI.md)). Voir la section « Le guide pour l'IA » ci-dessous.
>
> **Statut.** Ce n'est pas une fonction intégrée : memoire n'appelle aucune IA et n'a ni serveur MCP ni plugin. Une IA l'utilise comme un humain, par la ligne de commande. Le flux ci-dessous a été **rejoué à la main** sur un petit exemple C ; il n'a pas encore été essayé avec un agent réel sur un vrai projet. Savoir si c'est un bon outil pour le « vibe coding » reste donc **à évaluer** (voir plus bas).

### Mise en place

```sh
make construire                                    # produit target/release/memoire
export MEMOIRE_BASE=/chemin/du/projet/.memoire/depot.db
target/release/memoire init                        # crée la base du projet
target/release/memoire aide                        # liste complète des commandes (utile à donner à l'IA)
```

L'interface graphique (`make`) et l'IA travaillent sur la **même base** : l'humain voit le graphe se construire.

### Le flux, commande par commande

| Étape | Commande | Résultat |
|---|---|---|
| 1. Poser l'intention | `memoire note creer --role raisonnement --titre "…" --texte "…" --oui` | une note, son identifiant est affiché |
| 2. Décomposer | `memoire note creer --role implementation --titre "…" --depend-de ID --oui` | une note liée à la première |
| 3. Rattacher au code | `memoire ancrer ID --fichier src/x.c --fonction "void f(int a)"` | affiche `Repère à copier : // @design: ID` |
| 4. Poser le repère | **l'IA colle elle-même** la ligne `// @design: ID` au-dessus de la fonction | — |
| 5. Relire | `memoire repere "// @design: ID"` | la note, ce dont elle dépend, ses réserves, ses ancrages |
| 6. Détecter les écarts | `memoire indexer .` | repères retrouvés, **modifiés**, déplacés, perdus, inconnus |
| 7. Du code à la note | `memoire code notes src/x.c --ligne 42` | les notes qui concernent cet endroit |

Points à connaître :

- **memoire n'écrit jamais dans votre code** (décision D9). À l'étape 4, c'est l'IA, avec ses propres outils d'édition, qui insère la ligne de repère. Le commentaire s'adapte au langage (`//`, `#`, `/* */`).
- Lancez `indexer` **depuis la racine du projet** : les chemins des ancrages sont relatifs au dossier courant (lancé depuis `src/`, un ancrage apparaît à tort comme « déplacé »).
- Une opération **critique** (qui crée une branche ou déclenche des réexamens) affiche d'abord un aperçu et ne s'applique qu'avec `--oui` (code de retour 4 sinon). Pour garder l'humain dans la boucle, **ne donnez pas `--oui` d'office** à l'IA pour ces opérations.
- Tout est annulable (`annuler`, `retablir`) et rien n'est supprimé : une erreur de l'IA se corrige sans perte.

### Exemple d'instructions à donner à l'IA

```text
Tu travailles avec memoire (base : $MEMOIRE_BASE). Avant de coder une fonctionnalité :
1. crée une note "raisonnement" qui dit ce que tu vas faire et pourquoi ;
2. crée une note "implementation" par morceau de code prévu, avec --depend-de ;
Après avoir écrit le code :
3. pour chaque fonction, lance `memoire ancrer`, puis colle la ligne "// @design: ID" au-dessus ;
4. lance `memoire indexer .` et corrige tout repère "modifié", "perdu" ou "inconnu".
Si tu changes un raisonnement, utilise `memoire note reviser` en déclarant la nature
(mineure ou substantielle) : les notes qui en dépendent passent à réexaminer.
N'utilise --oui qu'après avoir montré l'aperçu à l'humain.
```

### Est-ce un bon outil pour le vibe coding ?

Ce qui devrait aider (conçu pour, vérifié à la main) :

- **Comprendre après coup** : de n'importe quelle fonction, `repere` redonne l'intention et ses dépendances, sans relire la conversation.
- **Voir la dérive** : `indexer` signale le code ancré qui a changé depuis la note, donc une note qui ne dit peut-être plus la vérité.
- **Mesurer l'impact d'un changement** : réviser ou réfuter une note marque ses dépendantes « à réexaminer », avec la cause.
- **Contrôle et réversibilité** : historique, annulation, branches pour essayer une piste de l'IA sans perdre l'autre.

Limites connues :

- **memoire enregistre, il ne vérifie pas.** Une IA peut écrire une note fausse ou complaisante ; l'état « vérifiée » n'est posé que par un verdict explicite (`verdict`), jamais automatiquement.
- **Le repère est posé à la main** par l'IA : si elle oublie, le code n'est pas rattaché. `indexer` le signale (« non ancrés »), mais ne le corrige pas.
- **Un ancrage détecte le changement, il ne suit pas le code** : si l'IA réécrit la fonction, il faut ré-ancrer.
- **Discipline requise** : le graphe n'est utile que si l'IA le tient à jour à chaque tâche, d'où les instructions ci-dessus.
- **Aucune intégration** : pas de plugin d'éditeur ni de serveur MCP ; l'IA doit pouvoir lancer des commandes.

Pour juger si l'outil mérite sa place dans votre pratique, essayez-le sur **une seule fonctionnalité** et regardez, une semaine plus tard : le graphe vous permet-il de retrouver pourquoi le code est écrit ainsi **sans relire la conversation** ? Les repères sont-ils tous encore « retrouvés » ? Le temps passé à tenir le graphe est-il inférieur à celui gagné à relire ?

## Le guide pour l'IA : `SKILLS/GUIDE-IA.md`

Le dossier `SKILLS/` contient **`GUIDE-IA.md`** (français) et sa traduction anglaise **`GUIDE-AI.md`** : un mode d'emploi de memoire écrit **pour une IA de programmation** (un agent qui a accès à un terminal). Il lui permet de :

- **utiliser memoire seule**, sans explication de votre part : installer la base, créer des notes et des liens, changer leur statut, annuler, sauvegarder ;
- **construire le graphe de ce qu'elle développe** : une note par intention, des dépendances entre elles, puis rattacher chaque fonction à sa note par un repère `// @design: ID` ;
- **contrôler la cohérence** entre les notes et le code (`indexer`) et signaler ce qui est à revoir ;
- **respecter les garde-fous** : ne pas confirmer une opération critique sans votre accord, ne pas toucher à la base autrement que par `memoire`, ne pas affirmer qu'une idée est vérifiée sans preuve ;
- **reprendre une session interrompue** grâce aux points de reprise.

Il contient aussi un déroulé de session type, les codes de retour, les pièges connus et un **bloc prêt à coller** dans le `CLAUDE.md` ou l'`AGENTS.md` de votre projet. Pour l'utiliser, donnez le fichier à votre IA (ou placez-le là où votre outil charge ses « skills » ou instructions), puis demandez-lui de s'en servir pour votre projet.

## Faire évoluer le logiciel : les documents à la racine

Les autres fichiers `.md` à la racine du dépôt (`REPRISE.md`, `DECISIONS.md`, `INVARIANTS.md`, `CAHIER_DES_CHARGES.md`, `CLI.md`, `INTERFACE.md`, `FORMAT.md`, etc.) ne servent **pas à utiliser** memoire. Ce sont les documents de conception du logiciel : ils sont là **pour qui veut le modifier ou le faire évoluer**. Ils expliquent ce qui a été décidé, pourquoi, quelles règles le code doit respecter et comment l'architecture est organisée (domaine, stockage, service, ligne de commande, interface).

**Reprendre le développement avec votre propre IA.** Ces documents sont écrits pour qu'un développeur, humain ou IA, reprenne le projet sans l'historique des échanges qui l'ont construit. La démarche conseillée :

1. Demandez à votre IA de lire d'abord **`REPRISE.md`** (état du projet, documents de référence, architecture, règles de travail, limites connues), puis `DECISIONS.md` et `INVARIANTS.md`.
2. Faites-lui **faire sa propre reprise** : qu'elle résume ce qu'elle a compris de l'architecture et des règles, et qu'elle vous le restitue avant de toucher au code.
3. Faites-la travailler selon les règles du projet (`REPRISE.md`, section 5) : **conception avant code**, puis **contrat avant code** (les tests d'abord).
4. Une fois à l'aise, vous pouvez utiliser memoire pour suivre le développement de memoire lui-même, avec `SKILLS/GUIDE-IA.md`.

À savoir avant de commencer :

- **Les documents sont en français** et décrivent l'état du projet à la date de leur rédaction.
- **`REPRISE.md` mentionne des branches et des étiquettes Git** (`inc-8.5`, `v0.8.0-alpha.2`…) qui appartiennent à l'historique privé de l'auteur : elles ne sont **pas** dans ce dépôt.
- **Les tests passent** (Rust et Python) sur des données de test **fictives** (voir « Données de test fictives »). Lancez `make tests` avant toute modification : c'est votre filet de sécurité.

## Organisation du dépôt

| Dossier | Rôle |
|---|---|
| `crates/memoire-domain` | règles métier, sans entrée-sortie ni dépendance |
| `crates/memoire-store` | stockage SQLite |
| `crates/memoire-app` | couche service |
| `crates/memoire-cli` | exécutable `memoire` (ligne de commande) |
| `gui/` | interface graphique Tkinter, qui pilote l'exécutable |
| `fixtures/` | données de référence des tests, **fictives** et figées (voir ci-dessous) |
| `tools/` | scripts de contrôle |
| `SKILLS/` | guide d'utilisation de memoire pour une IA (`GUIDE-IA.md` en français, `GUIDE-AI.md` en anglais) |
| `*.md` à la racine | documents de conception, pour faire évoluer le logiciel |

## Données de test fictives

Le dossier `fixtures/` contient les données de référence des tests, figées octet pour octet (empreintes SHA-256 dans les `SHA256SUMS`). Les données d'origine de l'auteur, qui étaient personnelles, ont été **remplacées par des données fictives** :

| Dossier | Contenu |
|---|---|
| `fixtures/rtos-2026-10/` | une conversation fictive avec une IA (`doc.md`, 5 échanges sur le réveil de tâches par un driver GPIO) et le code C correspondant (`driver.c`) : le jeu d'entrée du scénario de référence S-REF-01 |
| `fixtures/format/` | les exemples du format d'export (v1 et v2), **régénérés par le code** : ils sont identiques octet pour octet aux originaux |
| `fixtures/schema1/` | une base SQLite de l'ancien schéma 1 et son export, produits en rejouant le scénario S-REF-01 sur la conversation fictive (47 commits, 2 branches) ; ils servent à tester la migration |

Les contraintes que les tests imposent à ces données sont respectées à l'identique : tailles exactes (`doc.md` 20 392 octets, `driver.c` 4 263 octets), nombre de tabulations, d'espaces insécables et de séquences spéciales, passages cités présents une seule fois.

**Un seul changement de contrat** a été nécessaire, dans `crates/memoire-store/tests/stockage.rs` : les tailles de la base de schéma 1 et de son export, qui dépendent du contenu de la conversation (1 380 352 et 88 658 octets avec les données d'origine, 1 392 640 et 88 530 octets avec les données fictives). Toutes les autres assertions sont inchangées.

Si vous remplacez ces données par les vôtres, recalculez les `SHA256SUMS` et ajustez ces deux tailles.

## Pour aller plus loin

| Document | Contenu |
|---|---|
| `CAHIER_DES_CHARGES.md` | besoins et fonctionnement attendu |
| `INTERFACE.md`, `ACTIONS.md` | l'interface et son catalogue d'actions |
| `SKILLS/GUIDE-IA.md`, `SKILLS/GUIDE-AI.md` | guide d'utilisation de memoire destiné à une IA (français, anglais) |
| `CLI.md` | la ligne de commande |
| `DECISIONS.md` | décisions de conception, une fiche par décision |
| `INVARIANTS.md`, `TRACABILITE.md` | règles vérifiées par les tests |
| `FORMAT.md`, `STOCKAGE.md` | format d'export et stockage |
| `REPRISE.md` | état d'avancement du projet |
