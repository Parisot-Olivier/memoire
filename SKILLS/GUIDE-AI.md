# Guide to using memoire, for an AI

[Français](GUIDE-IA.md) · **English**

This document is addressed to **an AI coding agent that has a terminal**. It explains how to use `memoire` to keep track of what you design and develop: *what*, *why*, *based on what*, and *in which code*.

All the output examples below were obtained by running the executable.

> **Language.** The command line is **in French**: command names (`note creer`, `ancrer`, `indexer`…), options (`--titre`, `--depend-de`, `--oui`…), enumerated values (`raisonnement`, `verifiee`…) and output messages. Type them exactly as shown. Output format is stable. This guide translates the explanations, not the commands. A short glossary of the messages you will see:
>
> | Output (French) | Meaning |
> |---|---|
> | `Opération critique :` | critical operation (needs `--oui`) |
> | `Rien n'a été écrit` | nothing was written |
> | `À réexaminer` | to be re-examined |
> | `Note créée : ID` / `Relation créée : ID` | note / relation created |
> | `Ancrage créé` / `Repère à copier` | anchor created / marker to copy |
> | `Révision enregistrée.` / `Statut enregistré.` | revision / status saved |
> | `Branche créée : ID` / `Branche active :` | branch created / active branch |
> | `Retrouvés`, `Modifiés`, `Déplacés`, `Ambigus`, `Perdus`, `Inconnus`, `Non ancrés`, `Mal formés` | markers found, modified, moved, ambiguous, lost, unknown, not anchored, malformed |

---

## 1. The seven rules

1. **Before coding**, write down the intention: one `raisonnement` note (*why*) and `implementation` notes (*what*), linked with `--depend-de`.
2. **After writing a function**, attach it to its note: `memoire ancrer`, then **paste the line `// @design: ID` yourself** above the function. memoire never writes into the code.
3. **At the end of a task**, run `memoire indexer .` **from the project root** and fix everything reported as "modifié", "perdu" or "inconnu" (modified, lost, unknown).
4. **At the start of each session**, read the state: `memoire statut`, then `memoire reprise voir`.
5. **Never add `--oui` without showing it to the human** when the command answers "Opération critique" (code 4). Show the preview and wait for approval.
6. **Never modify `.memoire/depot.db` other than through `memoire`.** Do not delete it, do not move it, do not open it with another tool in write mode.
7. **Only declare "verifiee" (verified) what you actually verified** (a passing test, a code review). The reason (`--motif`) must say how.

---

## 2. Setup

The executable is built with `make construire` (it produces `target/release/memoire`). The examples below write `memoire` for this executable.

```sh
export MEMOIRE_BASE=/path/to/project/.memoire/depot.db   # otherwise: .memoire/depot.db in the current folder
memoire init       # creates the database; code 3 "le fichier existe déjà" if it exists: harmless
memoire statut     # code 3 "unable to open database file": the database does not exist, run init first
memoire aide       # lists all commands; memoire aide COMMAND describes one of them
```

- `--base FILE` overrides `MEMOIRE_BASE`; it goes **before** the command name: `memoire --base x.db statut`.
- The human can open the same database in the GUI (`make`) and watch the graph being built.
- **Working folder**: run the commands that touch code (`ancrer`, `indexer`, `code notes`) **from the project root**. Paths are stored as you give them and compared with the current folder.

---

## 3. The model in brief

### Notes

A note has a **title**, an optional **text** and a **role**:

| Role | Used for |
|---|---|
| `categorie` | grouping (a topic, a module) |
| `raisonnement` | saying *why*: an analysis, a choice, a hypothesis |
| `implementation` | saying *what*: a piece of code, planned or written |
| `note` | everything else |

### Relations

| Type | Meaning | Use it for |
|---|---|---|
| `classe-dans` | ORIGIN is filed in DESTINATION | a note in its category |
| `depend-de` | ORIGIN **depends on the reasoning** of DESTINATION | an implementation that rests on a line of reasoning |
| `developpe-a-partir-de` | ORIGIN elaborates on DESTINATION | an idea that extends another |
| `lien` | plain link, no consequence | nothing more specific |

**Verified rules** (a refused relation returns code 1, `relation interdite entre ces rôles`, i.e. "relation forbidden between these roles"):

- `classe-dans`: the **destination must be a `categorie`** (the origin can have any role).
- `depend-de`: the origin cannot be a `categorie`.
- `developpe-a-partir-de`: neither the origin nor the destination can be a `categorie`.
- `lien`: no role restriction.
- A note cannot be linked to itself (code 1).

**Re-examination**: only `depend-de` **and** `developpe-a-partir-de` put the origin "to be re-examined" when the destination changes significantly. `classe-dans` and `lien` trigger nothing. Use `depend-de` for a real dependency: "if this reasoning changes, this code must be reviewed".

### Statuses

Every status change **requires `--motif`** (the reason; without it: code 2, nothing is written).

| Command | Values |
|---|---|
| `verdict NOTE VALUE` | `non-verifiee` (unverified, default), `verifiee` (verified), `refutee` (refuted) |
| `decision NOTE VALUE` | `proposee` (proposed, default), `retenue` (retained), `plus-retenue` (no longer retained), `remplacee` (replaced) |
| `implementation NOTE VALUE` | `non-commencee` (not started, default), `en-cours` (in progress), `implementee` (implemented) |

Nothing is ever deleted: a discarded idea becomes "plus-retenue" or "refutee" and stays readable.

### Re-examination

When a note that others **depend on** changes significantly, its dependents become "à réexaminer" (to be re-examined), with the cause. This happens with:
- `note reviser … --nature substantielle`;
- `verdict … refutee`;
- `retirer`.

Only notes linked by `depend-de` or `developpe-a-partir-de` are concerned.

`statut` lists them. To get out of that state: review the note and, if it is still right, run `acquitter`.

### Branches

An operation that triggers re-examinations **creates a branch** and **puts you on it**. The original branch (`main`, identifier `00000000-0000-0000-0000-000000000000`) stays intact. Always check the active branch in `statut`.

---

## 4. Command conventions

- **Identifiers**: the full UUID, or a **prefix of at least 8 characters** if it is unique. E.g. `83809466`.
- **Options**: anywhere after the command name. **An unknown option is refused** (code 2). **A value cannot start with `--`.**
- **Several notes** (`NOTES`): identifiers separated by commas, no spaces: `--depend-de 6ee16099,7db52096`.
- **Texts with spaces or accents**: in quotes. **Long texts** (an exchange's question or answer): write them to a file and pass `--question-fichier` / `--reponse-fichier`.
- **Getting a created identifier**: the output is `Note créée : UUID`; extract the UUID with a regular expression. To find a note by name: `memoire chercher TEXT` (case- and accent-insensitive), which returns `ID<TAB>role<TAB>title`.

### Return codes

| Code | Meaning | What to do |
|---|---|---|
| 0 | success | — |
| 1 | refused by a rule (e.g. "relation d'une note vers elle-même", "texte qui n'est pas un repère canonique") | read the message, fix the request |
| 2 | incorrect usage: missing or unknown option, forbidden value, signature not found | `memoire aide COMMAND` |
| 3 | storage or file: missing database, file not found, `init` on an existing database | check `MEMOIRE_BASE` and the file path |
| 4 | **confirmation required, nothing was written** | show the preview to the human; with their approval, rerun with `--oui` |

---

## 5. Critical operations and `--oui`

An operation is **critical** if it creates a branch or changes a cause of re-examination. Without `--oui`, it prints a preview and stops:

```
$ memoire note reviser 6ee16099 --texte "Le driver réveille directement." --nature substantielle
Opération critique :
  - une branche sera créée depuis main
  - à réexaminer : wake_task() (83809466-953d-43f1-97d9-4e1a62e5beaf)
Rien n'a été écrit : relancer avec --oui pour appliquer cette opération critique.
[code 4]
```

(*"Critical operation: a branch will be created from main; to re-examine: wake_task(). Nothing was written: rerun with --oui to apply this critical operation."*)

With `--oui`, the preview is still printed, then the operation is applied (`Révision enregistrée.` then `Branche créée : ID`).

**What to do:**
1. Run **without** `--oui` first.
2. If the code is 4, **show the preview to the human** and say what you intend to do.
3. Rerun with `--oui` only after their approval.
4. For operations that are not critical (creating a note, linking, anchoring…), `--oui` is unnecessary.

Moves in the history (`annuler`, `retablir`, `basculer`) ask for no confirmation: they are themselves reversible.

---

## 6. Reference of the useful commands

### Reading

| Command | Returns |
|---|---|
| `statut` | active branch; current resources; notes **to re-examine** with their cause; orphan notes |
| `chercher TEXT` | `ID<TAB>role<TAB>title` of the notes whose name contains TEXT |
| `voisinage NOTE [--tsv]` | the active relations of the note |
| `repere LINE` | from a line of code containing `@design: ID`: the note, its statuses, its supports, its reservations, its anchors |
| `code notes FILE [--ligne N] [--extrait T] [--tsv]` | the notes linked to this file or this place |
| `indexer FOLDER [--extensions LIST]` | state of all markers in the code (see §8); **writes nothing** |
| `historique [--nombre N]` | `ID — command — reason`, most recent first |
| `branches [--toutes] [--tsv]` | the branches; `*` marks the active one |
| `source NOTE [--partie question\|reponse] [--version N]` | the exact text of one part of an exchange or one version of a note |
| `verifier` | rereads the whole history: `Base vérifiée : 11 commits, 2 branches, état courant confirmé.` |

### Writing notes and links

```sh
memoire note creer --role raisonnement --titre "Réveil par événement" \
        --texte "Le driver signale, le noyau réveille." --classe-dans CATEGORY
# Note créée : 6ee16099-60f8-412f-866d-09c7428b55a7

memoire note creer --role implementation --titre "wake_task()" --depend-de 6ee16099
# Note créée : 83809466-...   then   Relation créée : ... (depend-de ...)

memoire lier ORIGIN TYPE DESTINATION          # TYPE: classe-dans | depend-de | developpe-a-partir-de | lien
```

`note creer` accepts `--classe-dans`, `--depend-de`, `--developpe-a-partir-de`, `--lien` (lists of identifiers): the note is created **and** linked in a single operation. `--texte` and `--motif` are optional.

### Changing, revising, removing

```sh
# Revising the text: the nature is REQUIRED and declared by you, never implicit.
memoire note reviser NOTE --texte "New text" --nature mineure         # no re-examination
memoire note reviser NOTE --texte "New text" --nature substantielle   # critical: re-examinations + branch

memoire verdict NOTE verifiee --motif "test_wake_task passes"
memoire decision NOTE retenue --motif "chosen after comparing with option B"
memoire implementation NOTE implementee --motif "code written and tested"
memoire acquitter DEPENDENT CAUSE --motif "still valid: ..."               # critical
memoire retirer NOTE --motif "replaced"                                     # archives, does not delete
```

- **Choosing `mineure` or `substantielle`**: *mineure* (minor) = rewording, clarification, typo; *substantielle* (substantial) = the meaning changes and the notes that depend on it may no longer hold. When in doubt, tell the human.
- **`acquitter DEPENDENT CAUSE`**: DEPENDENT is the note "to re-examine", CAUSE is the note whose change put it in that state (shown by `statut` after "cause :").

### Undoing

```sh
memoire annuler           # undoes the last operation of the active branch     → « Annulé. »
memoire retablir          # redoes the undone operation                        → « Rétabli. »
memoire basculer 00000000 # back to main (8-character prefix)
memoire abandonner BRANCH [--motif M]   # sets a branch aside, deleting nothing
memoire reactiver BRANCH
```

### Keeping a conversation and quoting passages

```sh
printf '%s' "The question asked…" > q.txt
printf '%s' "The AI's answer…" > rep.txt
memoire echange coller --titre "Wake-up from an IRQ" --reponse-fichier rep.txt --question-fichier q.txt
# Échange créé : 13487d77-...        (without a question, the exchange stays "incomplete": echange question EXCHANGE --fichier q.txt)

memoire citer EXCHANGE --texte "the kernel wakes the task at the next tick" --soutient NOTE
# Citation créée : 581152c0-... (octets 34 à 78)  + relations created
```

- The text is stored **byte for byte**; `citer --texte` requires the **exact** text (several occurrences: add `--occurrence N`).
- `--soutient NOTE` links the quotation to the note it supports. Other options: `--inspire`, `--classe-dans`, `--lien`, `--note TITLE` (creates a note from the quotation), `--reserve TITLE` (creates a doubt to follow up).

### Reservations (doubts to follow up)

```sh
memoire reserve creer PASSAGE --titre "The multicore case is not handled" [--cible C]
memoire reserve concerne RESERVATION NOTE            # the reservation applies to the note
memoire reserve traiter NOTE RESERVATION --motif M   # the note addresses the reservation: "traitée à vérifier" (addressed, to verify)
memoire reserve etat RESERVATION ouverte|traitee-a-verifier|resolue --motif M
```

### Other relations

```sh
memoire restreindre ORIGIN DESTINATION --condition "single-core only"
memoire contredire ORIGIN DESTINATION --justification "contrary measurement"   # acknowledged disagreement, no effect on the destination
```

### Backing up and exporting

```sh
memoire sauvegarder              # dated copy in sauvegardes/ next to the database
memoire exporter export.txt      # complete canonical text of the repository
memoire importer export.txt      # creates a database from an export (never overwrites)
```

### Restoring an earlier state

```sh
memoire restauration plan --depuis COMMIT [--notes NOTES]     # shows what would be restored
memoire restaurer --depuis COMMIT [--notes NOTES] --motif M   # restores, leaving the rest untouched
```

---

## 7. Linking code to notes

### Placing a marker

```sh
memoire ancrer NOTE --fichier src/sched.c --fonction "void wake_task(struct task *t)"
# Ancrage créé : 9b390922-...
# Repère à copier : // @design: 39eaef62-0c11-4ebf-8569-b155b939a15e
```

Then **you insert the marker line yourself**, right above the function:

```c
// @design: 39eaef62-0c11-4ebf-8569-b155b939a15e
void wake_task(struct task *t) {
    t->state = READY;
}
```

- **Two ways to designate the code**: `--fonction "SIGNATURE"` (the exact signature, as written; it must appear **only once**; the excerpt runs to the first line starting with `}`) or `--lignes A-B` (inclusive range, starting at 1).
- **Common errors**: signature not found or present several times → code 2; file does not exist → code 3. The file is **never modified** by memoire.
- **Comment syntax**: `// @design: ID`, `# @design: ID` or `/* @design: ID */` are recognised; the marker may also follow code on the same line. Adapt it to the language.
- A single note can have **several anchors** (two functions, two files): rerun `ancrer` for each; the marker to copy is the same.

### Finding them again

```sh
memoire repere "// @design: 39eaef62-0c11-4ebf-8569-b155b939a15e"   # from code to note
memoire code notes src/sched.c --ligne 42                            # which notes concern this place
```

Example output of `repere`:

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

(*Note, Role, Decision, Verification, Implementation, "Relies on", "Disagreements", "Reservations of the note", "Anchors" — "(aucune)" means none, "(actif)" active.*)

---

## 8. Checking consistency: `memoire indexer .`

Run it **from the project root** after any series of changes. It scans the code and classifies each marker:

| Category | Means | Action |
|---|---|---|
| Retrouvés (found) | marker and anchored code in place | nothing |
| **Modifiés** (modified) | the anchored code changed since the note | reread the note: still true? if not, `note reviser`; then re-anchor |
| Déplacés (moved) | the anchor's path is out of date | if you ran it from a subfolder, go back to the root; otherwise re-anchor |
| Ambigus (ambiguous) | several occurrences, none chosen | make the anchor more precise |
| **Perdus** (lost) | anchor without a marker in the code | paste the `@design:` line again or remove the note |
| **Inconnus** (unknown) | marker whose note does not exist on this branch | check the active branch (`statut`) and the identifier |
| Non ancrés (not anchored) | marker of a note that has no anchor | run `ancrer` |
| Mal formés (malformed) | unreadable identifier after `@design:` | fix the marker |

Example, after changing `READY` to `BLOCKED` in the anchored function:

```
Modifiés (le code ancré a changé) :
  - wake_task() passe la tâche à prête (39eaef62-...) — src/sched.c:3 — extrait ancré absent de src/sched.c
```

An anchor **detects** a change; it does not follow the code. If you rewrite a function, anchor it again.

---

## 9. Resuming interrupted work

```sh
memoire reprise noter --objectif "Finish the task wake-up" --action "Write wake_task()" \
        --etat "driver finished" --ouverts "multicore case" --code .
# --code FOLDER: reads the Git state of that folder; outside a Git repository, the command succeeds and records « Code : non disponible »
# --sans-code: does not record the state of the code
memoire reprise voir [--code .]     # latest resume point and what changed since
memoire reprise liste
```

Options of `reprise noter`: `--objectif` (goal) and `--action` (next action) are required; `--question`, `--etat` (state), `--resultat` (result), `--hypothese`, `--ouverts` (open points), `--notes NOTES`, `--motif` are optional. The state of the code is read from Git (read-only); outside a Git repository it is simply recorded as "non disponible" (not available).

**At the end of a session**, record a resume point: it lets the next session (yours or another AI's) start again without rereading the whole history.

---

## 10. Typical walk-through

```sh
# Start of session
memoire statut
memoire reprise voir

# 1. Intention
memoire note creer --role categorie --titre "Scheduler"
memoire note creer --role raisonnement --titre "Wake-up by event" \
       --texte "The driver signals; the kernel wakes up at the tick." --classe-dans CATEGORY
memoire note creer --role implementation --titre "wake_task()" --depend-de REASONING

# 2. Code (written with your editing tools), then attachment
memoire ancrer IMPLEMENTATION --fichier src/sched.c --fonction "void wake_task(struct task *t)"
#   → insert « // @design: <ID> » above the function
memoire implementation IMPLEMENTATION implementee --motif "written"
memoire verdict IMPLEMENTATION verifiee --motif "unit tests passed"

# 3. Check
memoire indexer .
memoire statut                # nothing under "À réexaminer"?

# 4. If the reasoning changes along the way
memoire note reviser REASONING --texte "…" --nature substantielle      # code 4: show the preview to the human
memoire note reviser REASONING --texte "…" --nature substantielle --oui   # after approval
memoire statut                # the active branch has changed; wake_task() is "à réexaminer"

# 5. End of session
memoire reprise noter --objectif "…" --action "…" --code .
```

---

## 11. Pitfalls

| Pitfall | What to do |
|---|---|
| Forgetting the active branch after a critical operation | reread `Branche active :` in `statut`; `basculer 00000000` returns to `main` |
| `init` on an existing database: code 3 | normal; do not delete it |
| `indexer` run from a subfolder: everything is "Déplacé" (moved) | rerun from the root |
| Copying an identifier by hand | an identifier truncated below 8 characters is refused; use `chercher` |
| Text starting with `--` | not supported; rephrase |
| Pasting a marker into a generated or temporary file | the marker must be in the code the human will read |
| Marking `verifiee` out of optimism | a status is a claim; the reason must name the evidence |
| Making the graph too fine-grained (one note per line) | one note per **intention** or per **function** |

---

## 12. Block to paste into `CLAUDE.md` or `AGENTS.md`

```markdown
## memoire (design memory)

This project uses `memoire` (executable: target/release/memoire, built by `make construire`).
Database: MEMOIRE_BASE=<path>/.memoire/depot.db. Read SKILLS/GUIDE-AI.md before using it.
The command line is in French: type the commands exactly as documented.

- Start of session: `memoire statut` then `memoire reprise voir`.
- Before coding: one `raisonnement` note + `implementation` notes linked with `--depend-de`.
- After each function: `memoire ancrer NOTE --fichier F --fonction "SIGNATURE"`, then paste
  `// @design: ID` above the function (memoire never writes into the code).
- End of task: `memoire indexer .` (from the root); fix "modifiés", "perdus", "inconnus".
- If a command answers "Opération critique" (code 4): show the preview to the human; add
  `--oui` only after their approval.
- Never touch .memoire/ other than through `memoire`.
- End of session: `memoire reprise noter --objectif … --action … --code .`
```
