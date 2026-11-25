# Guide d'utilisation de memoire pour une IA

**Français** · [English](GUIDE-AI.md)

Ce document s'adresse à **une IA de programmation qui dispose d'un terminal**. Il explique comment utiliser `memoire` pour garder la trace de ce qu'elle conçoit et développe : *quoi*, *pourquoi*, *à partir de quoi*, et *dans quel code*.

Tous les exemples de sortie ci-dessous ont été obtenus en exécutant l'exécutable. Les sorties sont en français et leur forme est stable.

---

## 1. Les sept règles à suivre

1. **Avant de coder**, note l'intention : une note `raisonnement` (pourquoi) et des notes `implementation` (quoi), reliées par `--depend-de`.
2. **Après avoir écrit une fonction**, rattache-la à sa note : `memoire ancrer`, puis **colle toi-même** la ligne `// @design: ID` au-dessus de la fonction. memoire n'écrit jamais dans le code.
3. **En fin de tâche**, lance `memoire indexer .` **depuis la racine du projet** et corrige tout ce qui est « modifié », « perdu » ou « inconnu ».
4. **Au début de chaque session**, lis l'état : `memoire statut`, puis `memoire reprise voir`.
5. **Ne mets jamais `--oui` sans l'avoir montré à l'humain** quand la commande répond « Opération critique » (code 4). Montre l'aperçu, attends son accord.
6. **Ne modifie jamais `.memoire/depot.db` autrement que par `memoire`.** Ne le supprime pas, ne le déplace pas, ne l'ouvre pas avec un autre outil en écriture.
7. **Ne déclare « vérifiée » que ce que tu as réellement vérifié** (test passé, relecture du code). Le motif doit dire comment.

---

## 2. Mise en place

L'exécutable se construit avec `make construire` (produit `target/release/memoire`). Les exemples ci-dessous écrivent `memoire` pour cet exécutable.

```sh
export MEMOIRE_BASE=/chemin/du/projet/.memoire/depot.db   # sinon : .memoire/depot.db du dossier courant
memoire init       # crée la base ; code 3 « le fichier existe déjà » si elle existe : ce n'est pas grave
memoire statut     # code 3 « unable to open database file » : la base n'existe pas, lance d'abord init
memoire aide       # liste toutes les commandes ; memoire aide COMMANDE détaille l'une d'elles
```

- `--base FICHIER` remplace `MEMOIRE_BASE` ; il se place **avant** le nom de la commande : `memoire --base x.db statut`.
- L'humain peut ouvrir la même base dans l'interface graphique (`make`) et voir le graphe se construire.
- **Dossier de travail** : lance les commandes qui touchent au code (`ancrer`, `indexer`, `code notes`) **depuis la racine du projet**. Les chemins sont enregistrés tels que tu les donnes et comparés au dossier courant.

---

## 3. Le modèle en bref

### Notes

Une note a un **titre**, un **texte** facultatif et un **rôle** :

| Rôle | Sert à |
|---|---|
| `categorie` | regrouper (un sujet, un module) |
| `raisonnement` | dire *pourquoi* : une analyse, un choix, une hypothèse |
| `implementation` | dire *quoi* : un morceau de code prévu ou écrit |
| `note` | tout le reste |

### Relations

| Type | Sens | À employer pour |
|---|---|---|
| `classe-dans` | ORIGINE est rangée dans DESTINATION | une note dans sa catégorie |
| `depend-de` | ORIGINE **dépend du raisonnement** de DESTINATION | une implémentation qui repose sur un raisonnement |
| `developpe-a-partir-de` | ORIGINE approfondit DESTINATION | une idée qui prolonge une autre |
| `lien` | simple lien, sans conséquence | rien de plus précis |

**Règles vérifiées** (une relation refusée donne le code 1, `relation interdite entre ces rôles`) :

- `classe-dans` : la **destination doit être une `categorie`** (l'origine peut avoir n'importe quel rôle).
- `depend-de` : l'origine ne peut pas être une `categorie`.
- `developpe-a-partir-de` : ni l'origine ni la destination ne peuvent être une `categorie`.
- `lien` : aucune restriction de rôle.
- Une note ne peut pas être reliée à elle-même (code 1).

**Réexamen** : seules `depend-de` **et** `developpe-a-partir-de` mettent l'origine « à réexaminer » quand la destination change de façon importante. `classe-dans` et `lien` ne déclenchent rien. Emploie `depend-de` pour une vraie dépendance : « si ce raisonnement change, ce code est à revoir ».

### Statuts

Chaque changement de statut **exige `--motif`** (sans lui : code 2, rien n'est écrit).

| Commande | Valeurs |
|---|---|
| `verdict NOTE VALEUR` | `non-verifiee` (défaut), `verifiee`, `refutee` |
| `decision NOTE VALEUR` | `proposee` (défaut), `retenue`, `plus-retenue`, `remplacee` |
| `implementation NOTE VALEUR` | `non-commencee` (défaut), `en-cours`, `implementee` |

Rien n'est jamais supprimé : une idée écartée devient « plus retenue » ou « réfutée » et reste consultable.

### Réexamen

Quand une note dont d'autres **dépendent** change de façon importante, ses dépendantes passent « à réexaminer », avec la cause. Cela arrive avec :
- `note reviser … --nature substantielle` ;
- `verdict … refutee` ;
- `retirer`.

Seules les notes reliées par `depend-de` ou `developpe-a-partir-de` sont concernées.

`statut` les liste. Pour en sortir : revois la note, puis, si elle reste juste, `acquitter`.

### Branches

Une opération qui déclenche des réexamens **crée une branche** et **t'y place**. La branche d'origine (`main`, identifiant `00000000-0000-0000-0000-000000000000`) reste intacte. Vérifie toujours la branche active dans `statut`.

---

## 4. Conventions des commandes

- **Identifiants** : UUID complet, ou **préfixe d'au moins 8 caractères** s'il est unique. Ex. `83809466`.
- **Options** : n'importe où après le nom de la commande. **Une option inconnue est refusée** (code 2). **Une valeur ne peut pas commencer par `--`.**
- **Plusieurs notes** (`NOTES`) : identifiants séparés par des virgules, sans espace : `--depend-de 6ee16099,7db52096`.
- **Textes avec espaces ou accents** : entre guillemets. **Textes longs** (question, réponse d'un échange) : écris-les dans un fichier et passe `--question-fichier` / `--reponse-fichier`.
- **Récupérer un identifiant créé** : la sortie est `Note créée : UUID` ; extrais l'UUID avec une expression régulière. Pour retrouver une note par son nom : `memoire chercher TEXTE` (sans casse ni accents), qui rend `ID<TAB>rôle<TAB>titre`.

### Codes de retour

| Code | Sens | Que faire |
|---|---|---|
| 0 | réussi | — |
| 1 | refus par une règle (ex. : « relation d'une note vers elle-même », « texte qui n'est pas un repère canonique ») | lis le message, corrige la demande |
| 2 | usage incorrect : option manquante, inconnue, valeur interdite, signature introuvable | `memoire aide COMMANDE` |
| 3 | stockage ou fichier : base absente, fichier introuvable, `init` sur une base existante | vérifie `MEMOIRE_BASE`, le chemin du fichier |
| 4 | **confirmation requise, rien n'a été écrit** | montre l'aperçu à l'humain ; avec son accord, relance avec `--oui` |

---

## 5. Opérations critiques et `--oui`

Une opération est **critique** si elle crée une branche ou modifie une cause de réexamen. Sans `--oui`, elle affiche un aperçu et s'arrête :

```
$ memoire note reviser 6ee16099 --texte "Le driver réveille directement." --nature substantielle
Opération critique :
  - une branche sera créée depuis main
  - à réexaminer : wake_task() (83809466-953d-43f1-97d9-4e1a62e5beaf)
Rien n'a été écrit : relancer avec --oui pour appliquer cette opération critique.
[code 4]
```

Avec `--oui`, l'aperçu est quand même affiché, puis l'opération s'applique (`Révision enregistrée.` puis `Branche créée : ID`).

**Conduite à tenir :**
1. Lance d'abord **sans** `--oui`.
2. Si le code est 4, **montre l'aperçu à l'humain** et dis ce que tu comptes faire.
3. Relance avec `--oui` seulement après son accord.
4. Pour les opérations qui ne sont pas critiques (créer une note, relier, ancrer…), `--oui` est inutile.

Les déplacements dans l'historique (`annuler`, `retablir`, `basculer`) ne demandent pas de confirmation : ils sont eux-mêmes réversibles.

---

## 6. Référence des commandes utiles

### Lire

| Commande | Rend |
|---|---|
| `statut` | branche active ; ressources actuelles ; notes **à réexaminer** avec leur cause ; notes orphelines |
| `chercher TEXTE` | `ID<TAB>rôle<TAB>titre` des notes dont le nom contient TEXTE |
| `voisinage NOTE [--tsv]` | les relations actives de la note |
| `repere LIGNE` | depuis une ligne de code contenant `@design: ID` : la note, ses statuts, ses appuis, ses réserves, ses ancrages |
| `code notes FICHIER [--ligne N] [--extrait T] [--tsv]` | les notes liées à ce fichier ou à cet endroit |
| `indexer DOSSIER [--extensions LISTE]` | état de tous les repères du code (voir §8) ; **n'écrit rien** |
| `historique [--nombre N]` | `ID — commande — motif`, du plus récent au plus ancien |
| `branches [--toutes] [--tsv]` | les branches ; `*` marque l'active |
| `source NOTE [--partie question\|reponse] [--version N]` | le texte exact d'une partie d'un échange ou d'une version de note |
| `verifier` | relit tout l'historique : `Base vérifiée : 11 commits, 2 branches, état courant confirmé.` |

### Écrire des notes et des liens

```sh
memoire note creer --role raisonnement --titre "Réveil par événement" \
        --texte "Le driver signale, le noyau réveille." --classe-dans CATEGORIE
# Note créée : 6ee16099-60f8-412f-866d-09c7428b55a7

memoire note creer --role implementation --titre "wake_task()" --depend-de 6ee16099
# Note créée : 83809466-...   puis   Relation créée : ... (depend-de ...)

memoire lier ORIGINE TYPE DESTINATION          # TYPE : classe-dans | depend-de | developpe-a-partir-de | lien
```

`note creer` accepte `--classe-dans`, `--depend-de`, `--developpe-a-partir-de`, `--lien` (listes d'identifiants) : la note est créée **et** reliée en une seule opération. `--texte` et `--motif` sont facultatifs.

### Changer, réviser, retirer

```sh
# Réviser le texte : la nature est OBLIGATOIRE et déclarée par toi, jamais implicite.
memoire note reviser NOTE --texte "Nouveau texte" --nature mineure         # pas de réexamen
memoire note reviser NOTE --texte "Nouveau texte" --nature substantielle   # critique : réexamens + branche

memoire verdict NOTE verifiee --motif "test_wake_task passe"
memoire decision NOTE retenue --motif "choisi après comparaison avec l'option B"
memoire implementation NOTE implementee --motif "code écrit et testé"
memoire acquitter DEPENDANT CAUSE --motif "reste valable : ..."            # critique
memoire retirer NOTE --motif "remplacée"                                    # archive, ne supprime pas
```

- **Choisir `mineure` ou `substantielle`** : *mineure* = reformulation, précision, faute ; *substantielle* = le sens change et les notes qui en dépendent pourraient ne plus tenir. Dans le doute, dis-le à l'humain.
- **`acquitter DEPENDANT CAUSE`** : DEPENDANT est la note « à réexaminer », CAUSE la note dont le changement l'a mise dans cet état (affichée par `statut` après « cause : »).

### Annuler

```sh
memoire annuler           # annule la dernière opération de la branche active  → « Annulé. »
memoire retablir          # rétablit l'opération annulée                       → « Rétabli. »
memoire basculer 00000000 # revient à main (préfixe de 8 caractères)
memoire abandonner BRANCHE [--motif M]   # met une branche de côté, sans rien supprimer
memoire reactiver BRANCHE
```

### Conserver une conversation et en citer des passages

```sh
printf '%s' "La question posée…" > q.txt
printf '%s' "La réponse de l'IA…" > rep.txt
memoire echange coller --titre "Réveil depuis une IRQ" --reponse-fichier rep.txt --question-fichier q.txt
# Échange créé : 13487d77-...        (sans question, l'échange reste « incomplet » : echange question ECHANGE --fichier q.txt)

memoire citer ECHANGE --texte "le noyau réveille la tâche au tick suivant" --soutient NOTE
# Citation créée : 581152c0-... (octets 34 à 78)  + relations créées
```

- Le texte est enregistré **octet pour octet** ; `citer --texte` exige le texte **exact** (plusieurs occurrences : ajoute `--occurrence N`).
- `--soutient NOTE` relie la citation à la note qu'elle appuie. D'autres options : `--inspire`, `--classe-dans`, `--lien`, `--note TITRE` (crée une note depuis la citation), `--reserve TITRE` (crée un doute à suivre).

### Réserves (doutes à suivre)

```sh
memoire reserve creer PASSAGE --titre "Le cas multicœur n'est pas traité" [--cible C]
memoire reserve concerne RESERVE NOTE            # la réserve s'applique à la note
memoire reserve traiter NOTE RESERVE --motif M   # la note traite la réserve : « traitée à vérifier »
memoire reserve etat RESERVE ouverte|traitee-a-verifier|resolue --motif M
```

### Autres relations

```sh
memoire restreindre ORIGINE DESTINATION --condition "uniquement en monocœur"
memoire contredire ORIGINE DESTINATION --justification "mesure contraire"   # désaccord assumé, sans effet sur la destination
```

### Sauvegarder et exporter

```sh
memoire sauvegarder              # copie datée dans sauvegardes/ à côté de la base
memoire exporter export.txt      # texte canonique complet du dépôt
memoire importer export.txt      # crée une base depuis un export (n'écrase jamais)
```

### Restaurer un ancien état

```sh
memoire restauration plan --depuis COMMIT [--notes NOTES]     # montre ce qui serait rétabli
memoire restaurer --depuis COMMIT [--notes NOTES] --motif M   # rétablit, sans toucher le reste
```

---

## 7. Relier le code aux notes

### Poser un repère

```sh
memoire ancrer NOTE --fichier src/sched.c --fonction "void wake_task(struct task *t)"
# Ancrage créé : 9b390922-...
# Repère à copier : // @design: 39eaef62-0c11-4ebf-8569-b155b939a15e
```

Puis **tu insères toi-même** la ligne de repère juste au-dessus de la fonction :

```c
// @design: 39eaef62-0c11-4ebf-8569-b155b939a15e
void wake_task(struct task *t) {
    t->state = READY;
}
```

- **Deux façons de désigner le code** : `--fonction "SIGNATURE"` (la signature exacte, telle qu'elle est écrite ; elle doit apparaître **une seule fois** ; l'extrait va jusqu'à la première ligne commençant par `}`) ou `--lignes A-B` (plage inclusive, à partir de 1).
- **Erreurs fréquentes** : signature introuvable ou présente plusieurs fois → code 2 ; fichier inexistant → code 3. Le fichier n'est **jamais modifié** par memoire.
- **Syntaxe du commentaire** : `// @design: ID`, `# @design: ID` ou `/* @design: ID */` sont reconnus ; le repère peut aussi suivre du code sur la même ligne. Adapte-le au langage.
- Une même note peut avoir **plusieurs ancrages** (deux fonctions, deux fichiers) : relance `ancrer` pour chacun ; le repère à copier est le même.

### Retrouver

```sh
memoire repere "// @design: 39eaef62-0c11-4ebf-8569-b155b939a15e"   # du code vers la note
memoire code notes src/sched.c --ligne 42                            # quelles notes concernent cet endroit
```

Exemple de `repere` :

```
Note : wake_task() passe la tâche à prête (39eaef62-...)
Rôle : implementation
Décision : proposee
Vérification : non-verifiee
Implémentation : non-commencee
S'appuie sur :
  - Réveil des tâches par événement (72e3f3a4-...)
Désaccords :
  (aucune)
Réserves de la note :
  (aucune)
Ancrages :
  - src/sched.c (actif)
```

---

## 8. Contrôler la cohérence : `memoire indexer .`

Lance-le **depuis la racine du projet** après toute série de modifications. Il parcourt le code et classe chaque repère :

| Catégorie | Signifie | Action |
|---|---|---|
| Retrouvés | repère et code ancré en place | rien |
| **Modifiés** | le code ancré a changé depuis la note | relis la note : toujours vraie ? sinon `note reviser` ; puis ré-ancre |
| Déplacés | le chemin de l'ancrage est périmé | si tu lances depuis un sous-dossier, remonte à la racine ; sinon ré-ancre |
| Ambigus | plusieurs occurrences, aucune choisie | précise l'ancrage |
| **Perdus** | ancrage sans repère dans le code | recolle la ligne `@design:` ou retire la note |
| **Inconnus** | repère dont la note n'existe pas sur cette branche | vérifie la branche active (`statut`) et l'identifiant |
| Non ancrés | repère d'une note qui n'a pas d'ancrage | lance `ancrer` |
| Mal formés | identifiant illisible après `@design:` | corrige le repère |

Exemple, après avoir changé `READY` en `BLOCKED` dans la fonction ancrée :

```
Modifiés (le code ancré a changé) :
  - wake_task() passe la tâche à prête (39eaef62-...) — src/sched.c:3 — extrait ancré absent de src/sched.c
```

Un ancrage **détecte** un changement ; il ne suit pas le code. Si tu réécris une fonction, ancre-la de nouveau.

---

## 9. Reprendre un travail interrompu

```sh
memoire reprise noter --objectif "Finir le réveil des tâches" --action "Écrire wake_task()" \
        --etat "driver terminé" --ouverts "cas multicœur" --code .
# --code DOSSIER : lit l'état Git de ce dossier ; hors dépôt Git, la commande réussit et note « Code : non disponible »
# --sans-code : n'enregistre pas l'état du code
memoire reprise voir [--code .]     # dernier point de reprise et ce qui a changé depuis
memoire reprise liste
```

Options de `reprise noter` : `--objectif` et `--action` sont obligatoires ; `--question`, `--etat`, `--resultat`, `--hypothese`, `--ouverts`, `--notes NOTES`, `--motif` sont facultatives. L'état du code est lu dans Git (lecture seule) ; hors dépôt Git, il est simplement noté « non disponible ».

**À la fin d'une session**, enregistre un point de reprise : il permettra à la session suivante (la tienne ou celle d'une autre IA) de repartir sans relire toute l'histoire.

---

## 10. Déroulé type

```sh
# Début de session
memoire statut
memoire reprise voir

# 1. Intention
memoire note creer --role categorie --titre "Ordonnanceur"
memoire note creer --role raisonnement --titre "Réveil par événement" \
       --texte "Le driver signale ; le noyau réveille au tick." --classe-dans CATEGORIE
memoire note creer --role implementation --titre "wake_task()" --depend-de RAISONNEMENT

# 2. Code (écrit avec tes outils d'édition), puis rattachement
memoire ancrer IMPLEMENTATION --fichier src/sched.c --fonction "void wake_task(struct task *t)"
#   → insérer « // @design: <ID> » au-dessus de la fonction
memoire implementation IMPLEMENTATION implementee --motif "écrit"
memoire verdict IMPLEMENTATION verifiee --motif "tests unitaires passés"

# 3. Contrôle
memoire indexer .
memoire statut                # rien dans « À réexaminer » ?

# 4. Si le raisonnement change en cours de route
memoire note reviser RAISONNEMENT --texte "…" --nature substantielle      # code 4 : montrer l'aperçu à l'humain
memoire note reviser RAISONNEMENT --texte "…" --nature substantielle --oui   # après accord
memoire statut                # la branche active a changé ; wake_task() est « à réexaminer »

# 5. Fin de session
memoire reprise noter --objectif "…" --action "…" --code .
```

---

## 11. Pièges

| Piège | Conduite |
|---|---|
| Oublier la branche active après une opération critique | relis `Branche active :` dans `statut` ; `basculer 00000000` revient à `main` |
| `init` sur une base existante : code 3 | normal ; ne la supprime pas |
| `indexer` lancé depuis un sous-dossier : tout est « Déplacé » | relance depuis la racine |
| Copier un identifiant à la main | un identifiant tronqué à moins de 8 caractères est refusé ; utilise `chercher` |
| Texte commençant par `--` | non pris en charge ; reformule |
| Coller un repère dans un fichier généré ou temporaire | le repère doit être dans le code que l'humain lira |
| Marquer `verifiee` par optimisme | un statut est une affirmation ; le motif doit nommer la preuve |
| Créer un graphe trop fin (une note par ligne) | une note par **intention** ou par **fonction** |

---

## 12. Bloc à coller dans `CLAUDE.md` ou `AGENTS.md`

```markdown
## memoire (mémoire de conception)

Ce projet utilise `memoire` (exécutable : target/release/memoire, construit par `make construire`).
Base : MEMOIRE_BASE=<chemin>/.memoire/depot.db. Lis SKILLS/GUIDE-IA.md avant de l'utiliser.

- Début de session : `memoire statut` puis `memoire reprise voir`.
- Avant de coder : une note `raisonnement` + des notes `implementation` reliées par `--depend-de`.
- Après chaque fonction : `memoire ancrer NOTE --fichier F --fonction "SIGNATURE"`, puis colle
  `// @design: ID` au-dessus de la fonction (memoire n'écrit jamais dans le code).
- Fin de tâche : `memoire indexer .` (depuis la racine) ; corrige « modifiés », « perdus », « inconnus ».
- Si une commande répond « Opération critique » (code 4) : montre l'aperçu à l'humain ; n'ajoute
  `--oui` qu'après son accord.
- Ne touche jamais à .memoire/ autrement que par `memoire`.
- Fin de session : `memoire reprise noter --objectif … --action … --code .`
```
