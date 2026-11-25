# memoire — a Git for ideas

[Français](README.md) · **English**

> A local, Git-like memory for design work: it keeps your conversations with an AI, your notes and decisions and the relationships between them, ties them to your code with markers, and tells you what to re-examine when an idea changes. Rust core, SQLite storage, desktop GUI, and a command line that AI coding agents can drive.

**memoire** is a desktop application that keeps the memory of a design effort: conversations with an AI, notes, decisions, their relationships and their link to the code. It helps you find out *why* something was designed the way it was, and *where to resume* your work.

Everything stays **local**: one SQLite database per project, no account, no online service.

> **A note on language.** The software itself (interface, command line, command output) and the design documents are **in French**. This README gives the exact French commands and labels, with English translations in parentheses.

## What it does

- **Keeps AI conversations** byte for byte, in order: the original question and answer.
- **Quotes and annotates** a precise passage, without piling up comments.
- **Links notes**: filed in, depends on, develops from, justifies, restricts, contradicts.
- **Tracks the state of ideas**: verified or refuted, proposed or retained, implemented or not. Nothing is ever deleted: an idea that is "no longer retained" stays readable.
- **Flags what needs re-examining** when a note changes or is refuted: its dependents are listed, together with the cause.
- **Anchors a note in code** with a marker (`@design:`) and finds out what has moved.
- **Keeps a history** like Git: branches, undo, redo, merge, restore an earlier state.
- **Resumes work**: a resume point records the goal, the next action and the state of the code.

## Requirements

| Tool | Version | Note |
|---|---|---|
| `make` | — | the only entry point |
| Rust (`cargo`) | 1.91 or later | install with [rustup](https://rustup.rs) |
| Python 3 | recent | with **Tkinter** for the GUI |

Tkinter comes with your system: the `python3-tk` package on Debian and Ubuntu, `python3-tkinter` on Fedora. The first build downloads the Rust dependencies (versions pinned by `Cargo.lock`), so a network connection is needed that one time.

## Running the program

```sh
make
```

This builds the executable (release mode, pinned versions) and opens the window.

### First launch

1. In the window, open the **Projet** (*Project*) topic and choose **Créer le projet** (*Create the project*). This creates the database file `.memoire/depot.db` in the folder from which you ran `make`.
2. Then use the topics in the left-hand menu: **Écrire** (*Write*), **Relier** (*Link*), **Juger** (*Judge*), **Lire** (*Read*), **Code**, **Histoire et pistes** (*History and branches*), **Où j'en suis** (*Where I stand*). Each action opens a form and, for sensitive operations, shows a preview before writing anything.
3. **Expert mode** offers one form for every command-line command.

### Choosing or changing the database

```sh
make lancer BASE=/path/to/my-project/.memoire/depot.db
```

In the GUI, **Changer de projet…** (*Change project…*) also lets you switch from one database to another. One project is one database.

> A database contains your conversations: it must not be published. The `.memoire/` folder is excluded by `.gitignore`.

## Other `make` targets

| Command | Effect |
|---|---|
| `make` or `make lancer` | builds, then opens the GUI |
| `make construire` | only builds the executable `target/release/memoire` |
| `make tests` | Rust and Python tests, on fictional test data (see "Fictional test data"). The GUI is tested **without showing any window** if `xvfb-run` is installed (package `xorg-x11-server-Xvfb` on Fedora, `xvfb` on Debian and Ubuntu); `make tests VIRTUEL=0` forces the current display. One test, the window-stacking order test, is skipped for lack of a window manager: `MEMOIRE=$PWD/target/release/memoire tools/essais-sous-kwin.sh` runs it under a virtual KWin. |
| `make verifier` | full check (formatting, dependencies, lints, tests, mutation testing); long, needs extra tools (see `tools/check.sh`) |
| `make nettoyer` | removes build products |
| `make aide` | lists the targets |

## Using it with an AI (experimental)

**The idea.** An AI coding agent (one with access to a terminal) builds in memoire the **graph of what it is about to develop**: one note per intention, "depends on" links between them and, once the code is written, a **marker** that ties each piece of code to its note. The human then reads the graph to **understand what the AI did and why**, instead of reading all the code.

> **Complete guide for the AI: [`SKILLS/GUIDE-AI.md`](SKILLS/GUIDE-AI.md)** (English; the French original is [`SKILLS/GUIDE-IA.md`](SKILLS/GUIDE-IA.md)). See the section "The guide for the AI" below.
>
> **Status.** This is not a built-in feature: memoire does not call any AI and has no MCP server or plugin. An AI uses it the way a human does, through the command line. The flow below was **replayed by hand** on a small C example; it has not yet been tried with a real agent on a real project. Whether it is a good tool for "vibe coding" therefore remains **to be evaluated** (see below).

### Setup

```sh
make construire                                    # produces target/release/memoire
export MEMOIRE_BASE=/path/to/project/.memoire/depot.db
target/release/memoire init                        # creates the project database
target/release/memoire aide                        # full list of commands (useful to give to the AI)
```

The GUI (`make`) and the AI work on the **same database**: the human sees the graph being built.

### The flow, command by command

| Step | Command | Result |
|---|---|---|
| 1. State the intention | `memoire note creer --role raisonnement --titre "…" --texte "…" --oui` | a note; its identifier is printed |
| 2. Break it down | `memoire note creer --role implementation --titre "…" --depend-de ID --oui` | a note linked to the first one |
| 3. Tie it to the code | `memoire ancrer ID --fichier src/x.c --fonction "void f(int a)"` | prints `Repère à copier : // @design: ID` (*marker to copy*) |
| 4. Place the marker | **the AI pastes** the line `// @design: ID` above the function itself | — |
| 5. Read back | `memoire repere "// @design: ID"` | the note, what it depends on, its reservations, its anchors |
| 6. Detect drift | `memoire indexer .` | markers found, **modified**, moved, lost, unknown |
| 7. From code to note | `memoire code notes src/x.c --ligne 42` | the notes that concern this place |

Things to know:

- **memoire never writes into your code** (decision D9). In step 4, it is the AI, with its own editing tools, that inserts the marker line. The comment adapts to the language (`//`, `#`, `/* */`).
- Run `indexer` **from the project root**: anchor paths are relative to the current folder (run from `src/`, an anchor wrongly shows up as "moved").
- A **critical** operation (one that creates a branch or triggers re-examinations) first shows a preview and is only applied with `--oui` (return code 4 otherwise). To keep the human in the loop, **do not give the AI `--oui` by default** for these operations.
- Everything can be undone (`annuler`, `retablir`) and nothing is deleted: an AI mistake can be corrected without loss.

### Example instructions to give the AI

```text
You work with memoire (database: $MEMOIRE_BASE). Before coding a feature:
1. create a "raisonnement" note saying what you are going to do and why;
2. create one "implementation" note per planned piece of code, with --depend-de;
After writing the code:
3. for each function, run `memoire ancrer`, then paste the line "// @design: ID" above it;
4. run `memoire indexer .` and fix every "modifié", "perdu" or "inconnu" marker.
If you change a line of reasoning, use `memoire note reviser` and declare its nature
(mineure or substantielle): the notes that depend on it become "to re-examine".
Only use --oui after showing the preview to the human.
```

### Is it a good tool for vibe coding?

What should help (designed for it, verified by hand):

- **Understanding after the fact**: from any function, `repere` gives back the intention and its dependencies, without rereading the conversation.
- **Seeing drift**: `indexer` flags anchored code that changed since the note was written, i.e. a note that may no longer tell the truth.
- **Measuring the impact of a change**: revising or refuting a note marks its dependents "to re-examine", with the cause.
- **Control and reversibility**: history, undo, branches to try one of the AI's ideas without losing the other.

Known limits:

- **memoire records, it does not verify.** An AI can write a wrong or complacent note; the "verified" state is only set by an explicit verdict (`verdict`), never automatically.
- **The marker is placed by hand** by the AI: if it forgets, the code is not attached. `indexer` reports it ("non ancrés", *not anchored*) but does not fix it.
- **An anchor detects change, it does not follow the code**: if the AI rewrites the function, it has to be anchored again.
- **Discipline is required**: the graph is only useful if the AI keeps it up to date on every task, hence the instructions above.
- **No integration**: no editor plugin, no MCP server; the AI must be able to run commands.

To judge whether the tool deserves a place in your workflow, try it on **a single feature** and look a week later: does the graph let you find out why the code is written that way **without rereading the conversation**? Are all the markers still "found"? Is the time spent maintaining the graph less than the time saved rereading?

## The guide for the AI: `SKILLS/GUIDE-AI.md`

The `SKILLS/` folder contains the user manual of memoire written **for an AI coding agent** (one with access to a terminal), in two languages: **`GUIDE-AI.md`** (English) and **`GUIDE-IA.md`** (French original). It lets the AI:

- **use memoire on its own**, without explanations from you: set up the database, create notes and links, change their status, undo, back up;
- **build the graph of what it develops**: one note per intention, dependencies between them, then attach each function to its note with a `// @design: ID` marker;
- **check consistency** between the notes and the code (`indexer`) and report what needs a second look;
- **respect the safeguards**: not confirm a critical operation without your approval, not touch the database other than through `memoire`, not claim an idea is verified without evidence;
- **resume an interrupted session** thanks to resume points.

It also contains a typical session walk-through, the return codes, known pitfalls, and a **ready-to-paste block** for your project's `CLAUDE.md` or `AGENTS.md`. To use it, give the file to your AI (or place it wherever your tool loads its "skills" or instructions), then ask it to use it for your project.

## Evolving the software: the documents at the root

The other `.md` files at the root of the repository (`REPRISE.md`, `DECISIONS.md`, `INVARIANTS.md`, `CAHIER_DES_CHARGES.md`, `CLI.md`, `INTERFACE.md`, `FORMAT.md`, etc.) are **not for using** memoire. They are the design documents of the software: they are there **for anyone who wants to modify it or make it evolve**. They explain what was decided and why, which rules the code must follow, and how the architecture is organised (domain, storage, service, command line, interface).

**Resuming development with your own AI.** These documents are written so that a developer, human or AI, can take over the project without the history of the conversations that built it. The recommended approach:

1. Ask your AI to read **`REPRISE.md`** first (project state, reference documents, architecture, working rules, known limits), then `DECISIONS.md` and `INVARIANTS.md`.
2. Have it **do its own takeover**: it should summarise what it understood of the architecture and the rules, and report it back to you before touching any code.
3. Make it work according to the project's rules (`REPRISE.md`, section 5): **design before code**, then **contract before code** (tests first).
4. Once you are comfortable, you can use memoire to follow the development of memoire itself, with `SKILLS/GUIDE-AI.md`.

Before you start:

- **The documents are in French** and describe the state of the project at the time they were written. An AI can read them as they are; ask it to answer in English if you prefer.
- **`REPRISE.md` mentions Git branches and tags** (`inc-8.5`, `v0.8.0-alpha.2`…) that belong to the author's private history: they are **not** in this repository.
- **The tests pass** (Rust and Python) on **fictional** test data (see "Fictional test data"). Run `make tests` before any change: it is your safety net.

## Repository layout

| Folder | Role |
|---|---|
| `crates/memoire-domain` | business rules, no I/O and no dependency |
| `crates/memoire-store` | SQLite storage |
| `crates/memoire-app` | service layer |
| `crates/memoire-cli` | the `memoire` executable (command line) |
| `gui/` | Tkinter graphical interface, which drives the executable |
| `fixtures/` | reference data for the tests, **fictional** and frozen (see below) |
| `tools/` | check scripts |
| `SKILLS/` | guide to using memoire for an AI (`GUIDE-AI.md` in English, `GUIDE-IA.md` in French) |
| `*.md` at the root | design documents, for evolving the software |

## Fictional test data

The `fixtures/` folder holds the reference data of the tests, frozen byte for byte (SHA-256 digests in the `SHA256SUMS` files). The author's original data, which was personal, has been **replaced with fictional data**:

| Folder | Content |
|---|---|
| `fixtures/rtos-2026-10/` | a fictional conversation with an AI (`doc.md`, 5 exchanges about waking tasks from a GPIO driver) and the matching C code (`driver.c`): the input of the reference scenario S-REF-01 |
| `fixtures/format/` | the export-format examples (v1 and v2), **regenerated by the code**: they are byte-for-byte identical to the originals |
| `fixtures/schema1/` | an SQLite database in the old schema 1 and its export, produced by replaying scenario S-REF-01 on the fictional conversation (47 commits, 2 branches); they are used to test the migration |

The constraints the tests place on this data are met exactly: exact sizes (`doc.md` 20,392 bytes, `driver.c` 4,263 bytes), number of tabs, non-breaking spaces and special sequences, and each quoted passage appearing exactly once.

**A single contract change** was needed, in `crates/memoire-store/tests/stockage.rs`: the sizes of the schema-1 database and of its export, which depend on the content of the conversation (1,380,352 and 88,658 bytes with the original data, 1,392,640 and 88,530 bytes with the fictional data). All other assertions are unchanged.

If you replace this data with your own, recompute the `SHA256SUMS` and adjust those two sizes.

## Further reading

All of these documents are in French.

| Document | Content |
|---|---|
| `CAHIER_DES_CHARGES.md` | requirements and expected behaviour |
| `INTERFACE.md`, `ACTIONS.md` | the GUI and its action catalogue |
| `SKILLS/GUIDE-AI.md` | guide to using memoire, aimed at an AI (English) |
| `CLI.md` | the command line |
| `DECISIONS.md` | design decisions, one sheet per decision |
| `INVARIANTS.md`, `TRACABILITE.md` | rules checked by the tests |
| `FORMAT.md`, `STOCKAGE.md` | export format and storage |
| `REPRISE.md` | project progress and handover notes |
