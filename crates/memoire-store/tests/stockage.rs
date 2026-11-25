//! Contrat du stockage (STOCKAGE.md, section 5 ; ETAT-HISTORIQUE.md, sections 3 à 9) : relecture
//! fidèle, écriture de la seule différence, refus sans écriture, répétition sans double effet,
//! fichiers corrompus refusés, arrêt brutal d'un processus pendant les écritures (INV-DUR-01),
//! équivalence avec l'historique en mémoire (D20), lectures indépendantes de la longueur de
//! l'historique, vérification complète sur demande (D21).
//!
//! Contrat modifié (incrément 5.2c) : la base passe au schéma 2 (état courant et historique par
//! différences). Un dépôt SQLite n'a ni `Clone` ni `PartialEq` (D20) : les tests qui comparaient
//! deux dépôts comparent désormais leurs textes canoniques v2 complets, ou un dépôt SQLite à
//! l'oracle en mémoire construit par les mêmes commandes ; les tests qui lisaient la table des
//! fragments du schéma 1 lisent les tables du schéma 2. Aucune exigence n'est retirée.

use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use memoire_domain::*;
use memoire_store::{Store, StoreError, SCHEMA_VERSION};
use rusqlite::Connection;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Fichier temporaire propre au test, supprimé avec ses fichiers WAL.
struct TempFile(PathBuf);

impl TempFile {
    fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("memoire-{name}-{}-{n}.db", std::process::id()));
        remove(&path);
        Self(path)
    }
}

fn remove(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        remove(&self.0);
    }
}

fn raw(n: u64) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = 0x5e;
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    bytes
}

fn envelope<H: History>(repo: &Repository<H>, n: u64, command: Command) -> Envelope {
    Envelope {
        op: OpId::from_bytes(raw(n)),
        expected: repo.expected(),
        reason: "motif d'essai".into(),
        command,
    }
}

fn node(n: u64) -> NodeId {
    NodeId::from_bytes(raw(n))
}

/// Suite déterministe de commandes, critiques, ordinaires et déplacements de tête compris. Le
/// numéro de chaque commande sert aussi d'identifiant d'opération.
fn scenario() -> Vec<Command> {
    let s = DEFAULT_SCOPE;
    let create = |n, kind| {
        Command::Change(Change::CreateNode {
            scope: s,
            id: node(n),
            kind,
            title: format!("note {n}"),
            text: "texte « accentué » 🚀".into(),
        })
    };
    vec![
        create(1, Kind::Reasoning),
        create(2, Kind::Implementation),
        Command::Change(Change::Link {
            scope: s,
            id: EdgeId::from_bytes(raw(3)),
            from: node(2),
            to: node(1),
            relation: Relation::DependsOn,
        }),
        Command::Change(Change::SetVerdict {
            scope: s,
            node: node(1),
            verdict: Verdict::Refuted,
        }),
        Command::Undo,
        Command::Redo,
        Command::Checkout { branch: MAIN },
        create(8, Kind::Category),
        Command::Change(Change::AnchorToCode {
            id: AnchorId::from_bytes(raw(9)),
            node: node(2),
            path: "src/driver.c".into(),
            excerpt: "void refresh() {}".into(),
            git: None,
        }),
        Command::Change(Change::Restore { target: ROOT }),
        Command::Undo,
    ]
}

/// Texte canonique v2 complet de la base, rassemblé depuis ses tables.
fn text(store: &Store) -> String {
    store.text().unwrap()
}

/// Contenu brut de toutes les tables, ligne par ligne, pour constater ce qu'une commande écrit.
fn rows(path: &Path) -> Vec<(String, String)> {
    let connection = Connection::open(path).unwrap();
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut all = Vec::new();
    for table in tables {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap();
        let columns = statement.column_count();
        let mut found: Vec<(String, String)> = statement
            .query_map([], |r| {
                let values: Vec<String> = (0..columns)
                    .map(|i| format!("{:?}", r.get::<_, rusqlite::types::Value>(i).unwrap()))
                    .collect();
                Ok((table.clone(), values.join(" | ")))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        found.sort();
        all.extend(found);
    }
    all
}

fn tamper(path: &Path, sql: &str) {
    Connection::open(path).unwrap().execute_batch(sql).unwrap();
}

/// Dépôt linéaire sur main : une note d'implémentation, puis des changements alternés de son état
/// d'implémentation, jusqu'à `commits` commits racine comprise. L'état garde une taille constante.
fn linear(commits: u64) -> Repository {
    let mut repo = Repository::new();
    let first = envelope(
        &repo,
        1,
        Command::Change(Change::CreateNode {
            scope: DEFAULT_SCOPE,
            id: node(1_000_000),
            kind: Kind::Implementation,
            title: "suivie".into(),
            text: "texte".into(),
        }),
    );
    repo.execute(first).unwrap();
    for k in 2..commits {
        let implementation = if k % 2 == 0 {
            Implementation::InProgress
        } else {
            Implementation::Implemented
        };
        let next = envelope(
            &repo,
            k,
            Command::Change(Change::SetImplementation {
                scope: DEFAULT_SCOPE,
                node: node(1_000_000),
                implementation,
            }),
        );
        repo.execute(next).unwrap();
    }
    assert_eq!(repo.commit_count(), usize::try_from(commits).unwrap());
    repo
}

// INV-EXPORT-01, D4, D20 : après chaque commande, la base SQLite et l'historique en mémoire,
// conduits par les mêmes enveloppes, rendent les mêmes résultats et le même dépôt complet ; la
// base rouverte aussi. Contrat modifié (5.2c) : comparaison au texte de l'oracle en mémoire.
#[test]
fn reopening_gives_back_exactly_the_repository() {
    let file = TempFile::new("relecture");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    assert_eq!(text(&store), encode_repository_v2(&oracle));
    for (n, command) in (10..).zip(scenario()) {
        let next = envelope(store.repository(), n, command);
        let outcome = store.execute(next.clone()).unwrap();
        assert_eq!(oracle.execute(next).unwrap(), outcome, "commande {n}");
        assert_eq!(text(&store), encode_repository_v2(&oracle), "commande {n}");
        assert_eq!(
            store.repository().snapshot(),
            oracle.snapshot(),
            "commande {n}"
        );
        let reopened = Store::open(&file.0).unwrap();
        assert_eq!(text(&reopened), text(&store), "commande {n}");
        assert_eq!(reopened.repository().snapshot(), oracle.snapshot());
        assert_eq!(reopened.repository().expected(), oracle.expected());
        assert_eq!(reopened.repository().commit_count(), oracle.commit_count());
    }
}

// INV-ID-02, D20 : les identités employées et l'appartenance des notes aux branches se lisent
// dans la base rouverte, comme dans l'oracle en mémoire conduit par les mêmes commandes.
#[test]
fn identities_and_branch_membership_are_read_from_the_base() {
    let file = TempFile::new("identites");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    for (n, command) in (10..).zip(scenario()) {
        let next = envelope(store.repository(), n, command);
        store.execute(next.clone()).unwrap();
        oracle.execute(next).unwrap();
    }
    drop(store);
    let store = Store::open(&file.0).unwrap();
    let history = store.repository().history();
    let used: Vec<UsedId> = oracle.history().used_ids().collect();
    assert!(used.len() > 3);
    for id in &used {
        assert_eq!(history.is_used(*id), Ok(true), "{id:?}");
    }
    let mut containing = 0;
    for n in 0..200 {
        let id = UsedId::Node(node(n));
        assert_eq!(history.is_used(id), Ok(used.contains(&id)), "note {n}");
        let branches = store.repository().branches_containing(node(n)).unwrap();
        assert_eq!(
            branches,
            oracle.branches_containing(node(n)).unwrap(),
            "note {n}"
        );
        containing += branches.len();
    }
    assert!(containing > 1, "aucune note sur deux branches");
}

// Lecture d'un état ancien : chaque commit d'un historique qui franchit des états de référence
// se relit à l'identique, tête ou non ; ses parents et son motif aussi.
#[test]
fn every_commit_state_is_read_back_from_the_history() {
    let oracle = linear(230);
    let file = TempFile::new("etats-anciens");
    let store = Store::import(&file.0, &encode_repository_v2(&oracle)).unwrap();
    drop(store);
    let store = Store::open(&file.0).unwrap();
    for record in commit_records(&oracle) {
        assert_eq!(
            store.repository().snapshot_at(record.id).unwrap(),
            oracle.snapshot_at(record.id).unwrap(),
            "profondeur {}",
            record.depth
        );
        assert_eq!(
            store.repository().history().commit(record.id).unwrap(),
            oracle.history().commit(record.id).unwrap()
        );
    }
    let unknown = CommitId::from_bytes(raw(999_999));
    assert_eq!(
        store.repository().snapshot_at(unknown).map(|_| ()),
        Err(Error::MissingCommit(unknown))
    );
    assert_eq!(store.repository().history().commit(unknown), Ok(None));
}

// Annuler, rétablir, basculer et restaurer à travers des états de référence, en lockstep avec
// l'oracle en mémoire, base rouverte à chaque pas.
#[test]
fn moving_through_the_history_matches_memory() {
    let mut oracle = linear(205);
    let file = TempFile::new("deplacements");
    drop(Store::import(&file.0, &encode_repository_v2(&oracle)).unwrap());
    let mut commands = vec![Command::Undo; 8];
    commands.extend(vec![Command::Redo; 3]);
    commands.push(Command::Change(Change::Restore {
        target: CommitId::from_bytes(raw(50)),
    }));
    commands.push(Command::Change(Change::SetVerdict {
        scope: DEFAULT_SCOPE,
        node: node(1_000_000),
        verdict: Verdict::Refuted,
    }));
    commands.push(Command::Checkout { branch: MAIN });
    commands.push(Command::Undo);
    for (n, command) in (500..).zip(commands) {
        let mut store = Store::open(&file.0).unwrap();
        let next = envelope(store.repository(), n, command.clone());
        let stored = store.execute(next.clone()).map_err(|e| e.to_string());
        let expected = oracle
            .execute(next)
            .map_err(|e| StoreError::Domain(e).to_string());
        assert_eq!(stored, expected, "{command:?}");
        assert_eq!(text(&store), encode_repository_v2(&oracle), "{command:?}");
        assert_eq!(store.repository().snapshot(), oracle.snapshot());
    }
}

// VERSION2.md, section 7 : l'import lit le texte dans la version qu'annonce son en-tête, et la
// base créée redonne exactement le dépôt. Contrat modifié (5.2c) : comparaison des textes.
#[test]
fn import_reads_both_format_versions() {
    let mut repo = Repository::new();
    for (n, command) in (10..).zip(scenario()) {
        let next = envelope(&repo, n, command);
        repo.execute(next).unwrap();
    }
    for (name, source) in [
        ("import-v1", encode_repository(&repo)),
        ("import-v2", encode_repository_v2(&repo)),
    ] {
        let file = TempFile::new(name);
        let imported = Store::import(&file.0, &source).unwrap();
        assert_eq!(text(&imported), encode_repository_v2(&repo), "{name}");
        drop(imported);
        let reopened = Store::open(&file.0).unwrap();
        assert_eq!(text(&reopened), encode_repository_v2(&repo), "{name}");
        assert_eq!(reopened.repository().snapshot(), repo.snapshot());
    }
    let file = TempFile::new("import-inconnu");
    let unknown = encode_repository(&repo).replacen("memoire-depot 1", "memoire-depot 9", 1);
    assert!(matches!(
        Store::import(&file.0, &unknown),
        Err(StoreError::Decode(DecodeError { line: 1, .. }))
    ));
    assert!(!file.0.exists());
}

// Un fichier existant n'est jamais écrasé.
#[test]
fn create_never_overwrites_a_file() {
    let file = TempFile::new("existant");
    drop(Store::create(&file.0).unwrap());
    assert!(matches!(
        Store::create(&file.0),
        Err(StoreError::AlreadyExists)
    ));
    assert!(matches!(
        Store::import(&file.0, &encode_repository_v2(&Repository::new())),
        Err(StoreError::AlreadyExists)
    ));
    assert!(Store::open(&file.0).is_ok());
}

// INV-TX-01 : une commande refusée n'écrit rien ; une commande ordinaire n'écrit que sa
// différence. Contrat modifié (5.2c) : tables du schéma 2.
#[test]
fn refusals_write_nothing_and_changes_write_only_the_difference() {
    let file = TempFile::new("difference");
    let mut store = Store::create(&file.0).unwrap();
    for (n, command) in (10..).zip(scenario().into_iter().take(3)) {
        let next = envelope(store.repository(), n, command);
        store.execute(next).unwrap();
    }
    let before_rows = rows(&file.0);
    let before = text(&store);
    let refused = envelope(
        store.repository(),
        99,
        Command::Change(Change::Link {
            scope: DEFAULT_SCOPE,
            id: EdgeId::from_bytes(raw(98)),
            from: node(1),
            to: node(1),
            relation: Relation::DependsOn,
        }),
    );
    assert!(matches!(
        store.execute(refused),
        Err(StoreError::Domain(Error::SelfRelation))
    ));
    assert_eq!(rows(&file.0), before_rows);
    assert_eq!(text(&store), before);
    // Une création ordinaire sur main : un commit, sa différence (contenu et membre), la tête de
    // main, les deux lignes de l'état courant, une identité, une opération ; rien d'autre.
    let create = envelope(
        store.repository(),
        100,
        Command::Change(Change::CreateNode {
            scope: DEFAULT_SCOPE,
            id: node(100),
            kind: Kind::PersonalNote,
            title: "t".into(),
            text: "x".into(),
        }),
    );
    store.execute(create).unwrap();
    let after_rows = rows(&file.0);
    let added: Vec<&str> = after_rows
        .iter()
        .filter(|r| !before_rows.contains(r))
        .map(|r| r.0.as_str())
        .collect();
    let gone: Vec<&str> = before_rows
        .iter()
        .filter(|r| !after_rows.contains(r))
        .map(|r| r.0.as_str())
        .collect();
    assert_eq!(
        added,
        [
            "branches",
            "commits",
            "etat_courant",
            "etat_courant",
            "identites",
            "lignes_difference",
            "lignes_difference",
            "operations"
        ]
    );
    assert_eq!(gone, ["branches"]);
}

// INV-RETRY-01 : rejouer une enveloppe après réouverture rend le même résultat, sans écriture.
// Contrat modifié (5.2c) : contenu des tables et texte comparés.
#[test]
fn a_retried_envelope_has_no_second_effect_after_reopening() {
    let file = TempFile::new("repetition");
    let mut store = Store::create(&file.0).unwrap();
    let first = envelope(store.repository(), 10, scenario().remove(0));
    let outcome = store.execute(first.clone()).unwrap();
    drop(store);
    let mut store = Store::open(&file.0).unwrap();
    let before_rows = rows(&file.0);
    let before = text(&store);
    assert_eq!(store.execute(first).unwrap(), outcome);
    assert_eq!(rows(&file.0), before_rows);
    assert_eq!(text(&store), before);
}

fn scenario_base(name: &str, commands: usize) -> TempFile {
    let file = TempFile::new(name);
    let mut store = Store::create(&file.0).unwrap();
    for (n, command) in (10..).zip(scenario().into_iter().take(commands)) {
        let next = envelope(store.repository(), n, command);
        store.execute(next).unwrap();
    }
    file
}

/// Base au schéma 1, telle que l'écrivaient les versions antérieures (STOCKAGE.md, section 3).
fn schema_1_base(name: &str) -> TempFile {
    schema_1_of(name, &Repository::new())
}

/// Base au schéma 1 d'un dépôt donné, écrite comme `v0.5.0-alpha.1` l'écrivait : métadonnées,
/// puis les fragments canoniques du dépôt.
fn schema_1_of(name: &str, repository: &Repository) -> TempFile {
    let file = TempFile::new(name);
    let connection = Connection::open(&file.0).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE meta (cle TEXT PRIMARY KEY, valeur TEXT NOT NULL) STRICT;
             CREATE TABLE fragments (categorie TEXT NOT NULL, cle TEXT NOT NULL, texte TEXT NOT NULL,
                                     PRIMARY KEY (categorie, cle)) STRICT, WITHOUT ROWID;
             INSERT INTO meta (cle, valeur) VALUES ('schema', '1'), ('format', 'memoire-depot 1');",
        )
        .unwrap();
    for (category, parts) in Fragments::of(repository).parts {
        for (key, text) in parts {
            connection
                .execute(
                    "INSERT INTO fragments (categorie, cle, texte) VALUES (?1, ?2, ?3)",
                    (category.word(), key, text),
                )
                .unwrap();
        }
    }
    file
}

// Intégrité (D21) : un fichier d'un autre schéma, mal formé ou modifié hors du logiciel est refusé
// avec un motif ; l'ouverture vérifie les métadonnées et l'état courant de la branche active ; un
// état ancien corrompu est refusé à sa lecture et par la vérification complète. Contrat modifié
// (5.2c) : tables du schéma 2, et une base de schéma 1 est refusée avec l'indication de la
// migration.
#[test]
fn corrupted_or_foreign_files_are_refused() {
    assert_eq!(SCHEMA_VERSION, "2");
    let file = scenario_base("schema", 4);
    tamper(&file.0, "UPDATE meta SET valeur = '9' WHERE cle = 'schema'");
    assert!(matches!(Store::open(&file.0), Err(StoreError::Schema(m)) if m.contains("schéma 9")));
    let file = schema_1_base("schema-1");
    assert!(matches!(
        Store::open(&file.0),
        Err(StoreError::Schema(m)) if m.contains("schéma 1") && m.contains("memoire migrer")
    ));
    let file = scenario_base("format", 4);
    tamper(
        &file.0,
        "UPDATE meta SET valeur = 'memoire-depot 1' WHERE cle = 'format'",
    );
    assert!(matches!(Store::open(&file.0), Err(StoreError::Schema(m)) if m.contains("format")));
    let file = scenario_base("sans-meta", 4);
    tamper(&file.0, "DELETE FROM meta WHERE cle = 'format'");
    assert!(matches!(Store::open(&file.0), Err(StoreError::Schema(m)) if m.contains("absente")));
    let file = scenario_base("active", 4);
    tamper(&file.0, "DELETE FROM active");
    assert!(matches!(
        Store::open(&file.0),
        Err(StoreError::Historique(_))
    ));
    // État courant de la branche active illisible : refusé à l'ouverture.
    let file = scenario_base("etat-actif", 4);
    let active = Store::open(&file.0).unwrap().repository().active_branch();
    tamper(
        &file.0,
        &format!(
            "UPDATE etat_courant SET ligne = replace(ligne, 'titre', 'title') WHERE branche = '{active}'"
        ),
    );
    assert!(matches!(
        Store::open(&file.0),
        Err(StoreError::Historique(_))
    ));
    // Différence d'un commit ancien corrompue : ouverture acceptée (D21), lecture de cet état
    // refusée, vérification complète et export refusés.
    let file = scenario_base("ancien", 9);
    let old = CommitId::from_bytes(raw(10));
    tamper(
        &file.0,
        &format!(
            "UPDATE lignes_difference SET ligne = replace(ligne, 'titre', 'title') WHERE commit_id = '{old}'"
        ),
    );
    let store = Store::open(&file.0).unwrap();
    assert!(matches!(
        store.repository().snapshot_at(old),
        Err(Error::Historique(_))
    ));
    assert!(store.verify().is_err());
    assert!(store.export().is_err());
    // État courant d'une autre branche différent de son historique : ouverture acceptée,
    // vérification complète refusée.
    let file = scenario_base("autre-branche", 4);
    let store = Store::open(&file.0).unwrap();
    assert_eq!(store.repository().branch_count(), 2);
    let other = store
        .repository()
        .branches()
        .find(|b| *b != store.repository().active_branch())
        .unwrap();
    drop(store);
    tamper(
        &file.0,
        &format!("DELETE FROM etat_courant WHERE branche = '{other}' AND cle LIKE '3/%/1/%'"),
    );
    let store = Store::open(&file.0).unwrap();
    assert!(matches!(store.verify(), Err(StoreError::Schema(m)) if m.contains("état courant")));
    let file = TempFile::new("etranger");
    std::fs::write(
        &file.0,
        "ceci n'est pas une base SQLite, mais un texte assez long pour être lu",
    )
    .unwrap();
    assert!(matches!(Store::open(&file.0), Err(StoreError::Sqlite(_))));
}

// D21 : la vérification complète relit tout l'historique et confirme l'état courant de chaque
// branche ; l'export est le texte vérifié.
#[test]
fn full_verification_confirms_the_whole_history() {
    let file = scenario_base("verification", 11);
    let store = Store::open(&file.0).unwrap();
    let verified = store.verify().unwrap();
    assert_eq!(verified.commits, store.repository().commit_count());
    assert_eq!(verified.heads.len(), store.repository().branch_count());
    assert_eq!(store.export().unwrap(), text(&store));
    assert_eq!(verify_repository_v2(&store.export().unwrap()), Ok(verified));
}

// Indépendance de la longueur de l'historique (ETAT-HISTORIQUE.md, section 9, point 8) :
// ouvrir la base et exécuter une commande ordinaire lisent le même nombre de lignes à 150, 1 050
// et 2 050 commits. Le commit écrit n'est jamais à une profondeur multiple de 100 : c'est
// toujours une différence, et les lectures se comparent.
#[test]
fn reading_does_not_depend_on_the_history_length() {
    let mut counts = Vec::new();
    for commits in [150, 1_050, 2_050] {
        let oracle = linear(commits);
        let file = TempFile::new("lectures");
        drop(Store::import(&file.0, &encode_repository_v2(&oracle)).unwrap());
        let mut store = Store::open(&file.0).unwrap();
        let at_open = store.lines_read();
        let next = envelope(
            store.repository(),
            commits + 10,
            Command::Change(Change::SetDecision {
                scope: DEFAULT_SCOPE,
                node: node(1_000_000),
                decision: Decision::Retained,
            }),
        );
        store.execute(next).unwrap();
        counts.push((at_open, store.lines_read() - at_open));
    }
    assert!(counts[0].0 > 0 && counts[0].1 > 0, "{counts:?}");
    assert!(counts.windows(2).all(|w| w[0] == w[1]), "{counts:?}");
}

/// Position dans la suite de référence : la commande qui suit la dernière opération journalisée.
/// Une commande refusée n'a laissé aucune trace ; la rejouer redonne le même refus.
fn position(path: &Path) -> u64 {
    let connection = Connection::open(path).unwrap();
    let last: Option<String> = connection
        .query_row("SELECT max(op) FROM operations", [], |r| r.get(0))
        .unwrap();
    last.map_or(0, |op| {
        let op = OpId::parse(&op).unwrap();
        u64::from_be_bytes(op.to_bytes()[8..].try_into().unwrap()) - 9
    })
}

/// Processus fils : reprend la suite de référence là où le fichier s'est arrêté et l'exécute sans
/// fin, jusqu'à son arrêt brutal par le parent. Sans la variable d'environnement, il ne fait rien.
#[test]
fn child_writer() {
    let Ok(path) = std::env::var("MEMOIRE_ENFANT") else {
        return;
    };
    let start = position(Path::new(&path));
    let mut store = Store::open(Path::new(&path)).unwrap();
    for (n, command) in (10..)
        .zip(scenario().into_iter().cycle())
        .skip(usize::try_from(start).unwrap())
    {
        let next = envelope(store.repository(), n, command);
        let _ = store.execute(next);
    }
}

/// Dépôt de référence : les `count` premières commandes de la suite, en mémoire.
fn reference(count: u64) -> Repository {
    let mut repo = Repository::new();
    for (n, command) in (10..)
        .zip(scenario().into_iter().cycle())
        .take(usize::try_from(count).unwrap())
    {
        let next = envelope(&repo, n, command);
        let _ = repo.execute(next);
    }
    repo
}

// INV-DUR-01 : un processus tué brutalement à des instants variés laisse un fichier égal à l'état
// d'avant ou d'après une commande, jamais à un mélange. Limite : un arrêt du processus n'est pas
// une coupure d'alimentation ; le noyau du système conserve ses tampons. Contrat modifié (5.2c) :
// position lue dans la table des opérations, texte vérifié comparé à celui de l'oracle.
#[test]
fn a_killed_process_leaves_a_whole_state() {
    let file = TempFile::new("arret");
    drop(Store::create(&file.0).unwrap());
    let executable = std::env::current_exe().unwrap();
    let mut positions = Vec::new();
    for delay in [40u64, 90, 150, 230, 320, 450] {
        let mut child = Process::new(&executable)
            .args(["--exact", "child_writer", "--nocapture", "--test-threads=1"])
            .env("MEMOIRE_ENFANT", &file.0)
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(delay));
        child.kill().unwrap();
        child.wait().unwrap();
        // Le fichier rouvert est exactement l'état après un préfixe de la suite de référence.
        let store = Store::open(&file.0).unwrap();
        let reached = position(&file.0);
        assert_eq!(
            store.export().unwrap(),
            encode_repository_v2(&reference(reached)),
            "arrêt après {delay} ms"
        );
        positions.push(reached);
    }
    assert!(
        positions.windows(2).all(|w| w[0] <= w[1]) && positions.last() > positions.first(),
        "les écritures doivent progresser d'un fils à l'autre : {positions:?}"
    );
}

// Chaque erreur du stockage a un message qui nomme sa cause et reprend le détail sous-jacent.
#[test]
fn errors_explain_their_cause() {
    let decode = decode_repository("").unwrap_err();
    let cases = [
        (
            StoreError::AlreadyExists,
            "le fichier existe déjà".to_owned(),
        ),
        (
            StoreError::Domain(Error::NilIdentifier),
            format!("commande refusée : {}", Error::NilIdentifier),
        ),
        (
            StoreError::Decode(decode.clone()),
            format!("contenu illisible : {decode}"),
        ),
        (
            StoreError::Schema("schéma 9 inconnu".into()),
            "base non conforme : schéma 9 inconnu".to_owned(),
        ),
        (
            StoreError::Historique("ligne absente".into()),
            "historique illisible ou non écrit : ligne absente".to_owned(),
        ),
        (
            StoreError::Sqlite(rusqlite::Error::InvalidQuery),
            format!("erreur SQLite : {}", rusqlite::Error::InvalidQuery),
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

// R9 : une sauvegarde est une copie cohérente, qui se rouvre à l'identique ; une copie existante
// n'est jamais écrasée ; la base d'origine continue normalement. Contrat modifié (5.2c) :
// comparaison des textes.
#[test]
fn a_backup_is_a_consistent_copy() {
    let file = TempFile::new("sauvegarde-source");
    let mut store = Store::create(&file.0).unwrap();
    let commandes = scenario();
    for (n, command) in (10..).zip(commandes.iter().take(5).cloned()) {
        let next = envelope(store.repository(), n, command);
        store.execute(next).unwrap();
    }
    let copie = TempFile::new("sauvegarde-copie");
    store.backup(&copie.0).unwrap();
    assert_eq!(text(&Store::open(&copie.0).unwrap()), text(&store));
    assert!(matches!(
        store.backup(&copie.0),
        Err(StoreError::AlreadyExists)
    ));
    let next = envelope(store.repository(), 50, commandes[7].clone());
    store.execute(next).unwrap();
    assert_eq!(text(&Store::open(&file.0).unwrap()), text(&store));
    assert_ne!(text(&Store::open(&copie.0).unwrap()), text(&store));
}

// ---------------------------------------------------------------------------------------------
// Migration du schéma 1 vers le schéma 2 (incrément 5.2d ; ETAT-HISTORIQUE.md, sections 7 et 16).
// ---------------------------------------------------------------------------------------------

/// Base de schéma 1 écrite par l'exécutable de `v0.5.0-alpha.1` (S-REF-01 rejoué par la ligne de
/// commande), et l'export v2 que cette même version en donnait (fixtures/schema1/SHA256SUMS).
const SCHEMA_1_DB: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/schema1/sref01-schema1.db"
);
const SCHEMA_1_EXPORT: &str = include_str!("../../../fixtures/schema1/sref01-export-v2.txt");

fn bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

/// Copie de travail de la base figée de `v0.5.0-alpha.1`.
fn frozen_schema_1(name: &str) -> TempFile {
    let file = TempFile::new(name);
    std::fs::copy(SCHEMA_1_DB, &file.0).unwrap();
    file
}

/// Contenu des fragments d'une base de schéma 1, pour constater qu'une copie lui est fidèle.
fn fragment_rows(path: &Path) -> Vec<(String, String, String)> {
    let connection = Connection::open(path).unwrap();
    let mut statement = connection
        .prepare("SELECT categorie, cle, texte FROM fragments ORDER BY categorie, cle")
        .unwrap();
    statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

// INV-MIGR-01 : une base écrite par `v0.5.0-alpha.1` migre au schéma 2 et redonne exactement
// l'export de cette version ; la base d'origine n'est pas modifiée, la sauvegarde lui est fidèle,
// la base migrée est vérifiée en entier.
#[test]
fn a_base_written_by_the_previous_version_migrates_to_its_own_export() {
    // Contrat modifié (fixtures fictives) : tailles de la base et de l'export régénérés à partir de
    // la conversation fictive (fixtures/rtos-2026-10) ; elles valaient 1_380_352 et 88_658 avec les
    // données d'origine, retirées de la publication. Le reste du contrat est inchangé : 47 commits,
    // 2 branches, mêmes identifiants de têtes.
    assert_eq!(bytes(Path::new(SCHEMA_1_DB)).len(), 1_392_640);
    assert_eq!(SCHEMA_1_EXPORT.len(), 88_530);
    let source = frozen_schema_1("migration-source");
    let backup = TempFile::new("migration-sauvegarde");
    let target = TempFile::new("migration-cible");
    let before = bytes(&source.0);
    assert_eq!(Store::schema(&source.0).unwrap(), "1");
    let verified = Store::migrate(&source.0, &backup.0, &target.0).unwrap();
    assert_eq!(bytes(&source.0), before);
    assert_eq!(fragment_rows(&backup.0), fragment_rows(&source.0));
    assert_eq!(Store::schema(&backup.0).unwrap(), "1");
    assert_eq!(Store::schema(&target.0).unwrap(), "2");
    let store = Store::open(&target.0).unwrap();
    assert_eq!(text(&store), SCHEMA_1_EXPORT);
    assert_eq!(verified, store.verify().unwrap());
    assert_eq!(verified.commits, 47);
    assert_eq!(verified.heads.len(), 2);
}

// INV-MIGR-01 : des dépôts quelconques (branches, annulations, pile de rétablissement, historique
// qui franchit un état de référence) migrent au même dépôt, commit par commit.
#[test]
fn schema_1_bases_migrate_to_the_same_repository() {
    let mut repositories = vec![Repository::new(), linear(150)];
    let mut repo = Repository::new();
    for (n, command) in (10..).zip(scenario()) {
        let next = envelope(&repo, n, command);
        repo.execute(next).unwrap();
        repositories.push(repo.clone());
    }
    for (k, repository) in repositories.iter().enumerate() {
        let source = schema_1_of(&format!("migration-{k}"), repository);
        let backup = TempFile::new(&format!("migration-{k}-sauvegarde"));
        let target = TempFile::new(&format!("migration-{k}-cible"));
        let verified = Store::migrate(&source.0, &backup.0, &target.0).unwrap();
        assert_eq!(verified.commits, repository.commit_count(), "dépôt {k}");
        let store = Store::open(&target.0).unwrap();
        assert_eq!(text(&store), encode_repository_v2(repository), "dépôt {k}");
        assert_eq!(store.repository().expected(), repository.expected());
        for record in commit_records(repository) {
            assert_eq!(
                store.repository().snapshot_at(record.id).unwrap(),
                repository.snapshot_at(record.id).unwrap(),
                "dépôt {k}"
            );
        }
    }
}

// INV-MIGR-02 : une migration refusée ne modifie pas la base d'origine et ne laisse aucune
// base cible ; les refus constatés avant la sauvegarde (schéma, fichiers existants) ne créent
// aucun fichier ; une lecture refusée après la sauvegarde la laisse, fidèle à l'origine.
#[test]
fn a_refused_migration_leaves_the_source_intact() {
    let cases: [(&str, &str); 3] = [
        // Clé d'un commit non canonique : lisible, mais refusée comme par v0.5.0-alpha.1.
        (
            "cle",
            "UPDATE fragments SET cle = cle || 'x' WHERE categorie = 'commit' \
             AND cle = (SELECT max(cle) FROM fragments WHERE categorie = 'commit')",
        ),
        (
            "texte",
            "UPDATE fragments SET texte = replace(texte, 'titre', 'title') WHERE categorie = 'commit'",
        ),
        ("vide", "DELETE FROM fragments WHERE categorie = 'active'"),
    ];
    for (name, sql) in cases {
        let source = frozen_schema_1(&format!("refus-{name}"));
        tamper(&source.0, sql);
        let before = bytes(&source.0);
        let backup = TempFile::new(&format!("refus-{name}-sauvegarde"));
        let target = TempFile::new(&format!("refus-{name}-cible"));
        let refused = Store::migrate(&source.0, &backup.0, &target.0);
        assert!(
            matches!(refused, Err(StoreError::Schema(_) | StoreError::Decode(_))),
            "{name} : {refused:?}"
        );
        assert_eq!(bytes(&source.0), before, "{name}");
        assert!(!target.0.exists(), "{name}");
        assert_eq!(fragment_rows(&backup.0), fragment_rows(&source.0), "{name}");
    }
    // Catégorie inconnue : impossible dans une base écrite par v0.5.0-alpha.1 (contrainte CHECK
    // de la table), refusée tout de même si la table a été modifiée hors du logiciel.
    let source = schema_1_of("refus-categorie", &linear(5));
    tamper(
        &source.0,
        "INSERT INTO fragments VALUES ('inconnue', 'k', 't')",
    );
    let before = bytes(&source.0);
    let backup = TempFile::new("refus-categorie-sauvegarde");
    let target = TempFile::new("refus-categorie-cible");
    assert!(matches!(
        Store::migrate(&source.0, &backup.0, &target.0),
        Err(StoreError::Schema(m)) if m.contains("inconnue")
    ));
    assert_eq!(bytes(&source.0), before);
    assert!(!target.0.exists());
    // Refus avant toute écriture : aucun fichier créé.
    let current = scenario_base("refus-schema-2", 4);
    let unknown = frozen_schema_1("refus-schema-9");
    tamper(
        &unknown.0,
        "UPDATE meta SET valeur = '9' WHERE cle = 'schema'",
    );
    let format = frozen_schema_1("refus-format");
    tamper(
        &format.0,
        "UPDATE meta SET valeur = 'memoire-depot 2' WHERE cle = 'format'",
    );
    for (source, word) in [
        (&current, "schéma 2"),
        (&unknown, "schéma 9"),
        (&format, "format"),
    ] {
        let before = bytes(&source.0);
        let backup = TempFile::new("refus-tot-sauvegarde");
        let target = TempFile::new("refus-tot-cible");
        let refused = Store::migrate(&source.0, &backup.0, &target.0);
        assert!(
            matches!(&refused, Err(StoreError::Schema(m)) if m.contains(word)),
            "{word} : {refused:?}"
        );
        assert_eq!(bytes(&source.0), before);
        assert!(!backup.0.exists() && !target.0.exists(), "{word}");
    }
    let source = frozen_schema_1("refus-existants");
    let before = bytes(&source.0);
    let existing = TempFile::new("refus-existant");
    std::fs::write(&existing.0, "déjà là").unwrap();
    let absent = TempFile::new("refus-absent");
    assert!(matches!(
        Store::migrate(&source.0, &existing.0, &absent.0),
        Err(StoreError::AlreadyExists)
    ));
    assert!(!absent.0.exists());
    assert!(matches!(
        Store::migrate(&source.0, &absent.0, &existing.0),
        Err(StoreError::AlreadyExists)
    ));
    assert!(!absent.0.exists());
    assert_eq!(bytes(&existing.0), "déjà là".as_bytes());
    assert_eq!(bytes(&source.0), before);
}

// La source d'une migration n'est jamais écrite et ne garde aucun fichier annexe : une base en
// mode WAL fermée proprement reste seule et identique octet pour octet. Sa sauvegarde et la
// cible se rouvrent sans elle.
#[test]
fn migrating_a_wal_base_leaves_no_side_file() {
    let source = frozen_schema_1("wal-source");
    Connection::open(&source.0)
        .unwrap()
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    let before = bytes(&source.0);
    let backup = TempFile::new("wal-sauvegarde");
    let target = TempFile::new("wal-cible");
    Store::migrate(&source.0, &backup.0, &target.0).unwrap();
    assert_eq!(bytes(&source.0), before);
    for path in [&source.0, &backup.0, &target.0] {
        for suffix in ["-wal", "-shm"] {
            let side = PathBuf::from(format!("{}{suffix}", path.display()));
            assert!(!side.exists(), "{}", side.display());
        }
    }
    assert_eq!(text(&Store::open(&target.0).unwrap()), SCHEMA_1_EXPORT);
}

// Store::schema lit la version du schéma sans rien créer ni modifier ; un fichier absent ou
// étranger est une erreur.
#[test]
fn the_schema_version_is_read_without_writing() {
    let current = scenario_base("version-2", 3);
    assert_eq!(Store::schema(&current.0).unwrap(), "2");
    let old = frozen_schema_1("version-1");
    let before = bytes(&old.0);
    assert_eq!(Store::schema(&old.0).unwrap(), "1");
    assert_eq!(bytes(&old.0), before);
    let absent = TempFile::new("version-absent");
    assert!(Store::schema(&absent.0).is_err());
    assert!(!absent.0.exists());
    let foreign = TempFile::new("version-etranger");
    std::fs::write(
        &foreign.0,
        "ceci n'est pas une base SQLite, mais un texte assez long pour être lu",
    )
    .unwrap();
    assert!(Store::schema(&foreign.0).is_err());
}

// ETAT-HISTORIQUE.md, section 7, point 1 : une base de schéma 1 ou d'un schéma inconnu est
// refusée à l'ouverture sans être modifiée, même dans son mode de journal.
#[test]
fn opening_a_base_of_another_schema_writes_nothing() {
    let old = frozen_schema_1("ouverture-schema-1");
    let unknown = frozen_schema_1("ouverture-schema-9");
    tamper(
        &unknown.0,
        "UPDATE meta SET valeur = '9' WHERE cle = 'schema'",
    );
    for file in [&old, &unknown] {
        let before = bytes(&file.0);
        assert!(matches!(Store::open(&file.0), Err(StoreError::Schema(_))));
        assert_eq!(bytes(&file.0), before);
    }
}

/// Mode de journal enregistré dans le fichier d'une base.
fn journal_mode(path: &Path) -> String {
    Connection::open(path)
        .unwrap()
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap()
}

// Toute base écrite par le stockage (créée, importée, migrée) est en mode WAL : une écriture
// interrompue laisse l'état d'avant ou d'après (INV-DUR-01).
#[test]
fn bases_are_written_in_wal_mode() {
    let created = TempFile::new("wal-creee");
    drop(Store::create(&created.0).unwrap());
    let imported = TempFile::new("wal-importee");
    drop(Store::import(&imported.0, &encode_repository_v2(&linear(5))).unwrap());
    let source = frozen_schema_1("wal-migree-source");
    let backup = TempFile::new("wal-migree-sauvegarde");
    let migrated = TempFile::new("wal-migree");
    Store::migrate(&source.0, &backup.0, &migrated.0).unwrap();
    for file in [&created, &imported, &migrated] {
        assert_eq!(journal_mode(&file.0), "wal", "{}", file.0.display());
    }
    assert_eq!(journal_mode(&source.0), "delete");
}

/// Processus fils : importe un long dépôt sous une limite de taille de fichier, que le parent
/// pose ; l'écriture échoue en cours de route. Sans la variable d'environnement, il ne fait rien.
#[test]
fn child_importer() {
    let Ok(path) = std::env::var("MEMOIRE_IMPORT_LIMITE") else {
        return;
    };
    let path = PathBuf::from(path);
    let refused = Store::import(&path, &encode_repository_v2(&linear(1500)));
    assert!(refused.is_err(), "l'import aurait dû échouer");
    for suffix in ["", "-wal", "-shm"] {
        let side = PathBuf::from(format!("{}{suffix}", path.display()));
        assert!(!side.exists(), "{} reste après l'échec", side.display());
    }
    println!("écriture refusée constatée : {}", refused.err().unwrap());
}

// Une écriture qui échoue après la création du fichier (ici, limite de taille de fichier) ne
// laisse aucune base partielle : le fichier et ses annexes sont retirés.
#[cfg(unix)]
#[test]
fn a_failed_write_leaves_no_file() {
    let file = TempFile::new("limite");
    let executable = std::env::current_exe().unwrap();
    // SIGXFSZ ignoré par le shell, et donc par le fils : l'écriture rend une erreur au lieu de
    // tuer le processus. Limite de 64 blocs, bien en deçà de la base du dépôt importé.
    let output = Process::new("sh")
        .args([
            "-c",
            "trap '' XFSZ; ulimit -f 64; exec \"$0\" --exact child_importer --nocapture --test-threads=1",
        ])
        .arg(&executable)
        .env("MEMOIRE_IMPORT_LIMITE", &file.0)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("écriture refusée constatée"), "{stdout}");
    assert!(!file.0.exists());
}

// INV-LOT-01, INV-LOT-03 (incrément 5.3) : un lot s'écrit en une transaction (un commit, une
// opération, toutes ses identités) ; après réouverture, le texte est celui de l'oracle, un
// « annuler » le défait en entier, et ses identités restent employées.
#[test]
fn a_lot_is_written_in_one_transaction() {
    let file = TempFile::new("lot");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    let s = DEFAULT_SCOPE;
    let create = |n, kind| Change::CreateNode {
        scope: s,
        id: node(n),
        kind,
        title: format!("note {n}"),
        text: "texte".into(),
    };
    let link = |e, from, to, relation| Change::Link {
        scope: s,
        id: EdgeId::from_bytes(raw(e)),
        from: node(from),
        to: node(to),
        relation,
    };
    let run = |store: &mut Store, oracle: &mut Repository, n: u64, command: Command| {
        let next = envelope(oracle, n, command);
        let stored = store.execute(next.clone());
        let expected = oracle.execute(next);
        match (&stored, &expected) {
            (Ok(a), Ok(b)) => assert_eq!(a, b),
            (Err(StoreError::Domain(a)), Err(b)) => assert_eq!(a, b),
            _ => panic!("{stored:?} au lieu de {expected:?}"),
        }
        stored
    };
    run(
        &mut store,
        &mut oracle,
        10,
        Command::Change(create(1, Kind::Category)),
    )
    .unwrap();
    let before_rows = rows(&file.0);
    let before = text(&store);
    let lot = Command::Lot(vec![
        create(2, Kind::Reasoning),
        link(3, 2, 1, Relation::ClassifiedIn),
        create(4, Kind::Implementation),
        link(5, 4, 2, Relation::DependsOn),
    ]);
    // Refusé au dernier élément : rien n'est écrit.
    let refused = Command::Lot(vec![
        create(2, Kind::Reasoning),
        link(3, 2, 99, Relation::ClassifiedIn),
    ]);
    assert!(matches!(
        run(&mut store, &mut oracle, 11, refused),
        Err(StoreError::Domain(Error::InLot { element: 2, .. }))
    ));
    assert_eq!(rows(&file.0), before_rows);
    run(&mut store, &mut oracle, 12, lot).unwrap();
    drop(store);
    let mut store = Store::open(&file.0).unwrap();
    assert_eq!(text(&store), encode_repository_v2(&oracle));
    let count = |table: &str| {
        Connection::open(&file.0)
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
    };
    // Contexte par défaut, catégorie, puis les quatre identités du lot.
    assert_eq!(count("identites"), 1 + 1 + 4);
    assert_eq!(count("commits"), 3);
    assert_eq!(count("operations"), 2);
    run(&mut store, &mut oracle, 13, Command::Undo).unwrap();
    drop(store);
    let store = Store::open(&file.0).unwrap();
    assert_eq!(text(&store), encode_repository_v2(&oracle));
    let after_undo = decode_repository_v2(&text(&store)).unwrap();
    assert_eq!(
        after_undo.snapshot(),
        decode_repository_v2(&before).unwrap().snapshot()
    );
    for id in [
        UsedId::Node(node(2)),
        UsedId::Edge(EdgeId::from_bytes(raw(3))),
        UsedId::Node(node(4)),
        UsedId::Edge(EdgeId::from_bytes(raw(5))),
    ] {
        assert_eq!(store.repository().history().is_used(id), Ok(true), "{id:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// Abandon et réactivation d'une branche (incrément 5.4a ; FUSION.md, section 9.7 ; VERSION2.md,
// section 3 ; INV-BR-05).
// ---------------------------------------------------------------------------------------------

/// Lignes de l'état courant tenues pour une branche, et son état enregistré.
fn branch_rows(path: &Path, branch: BranchId) -> (i64, String) {
    let connection = Connection::open(path).unwrap();
    let id = branch.to_string();
    let lines = connection
        .query_row(
            "SELECT count(*) FROM etat_courant WHERE branche = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    let state = connection
        .query_row("SELECT etat FROM branches WHERE id = ?1", [&id], |r| {
            r.get(0)
        })
        .unwrap();
    (lines, state)
}

// VERSION2.md, section 3 : une branche abandonnée perd son état courant, reconstruit à sa
// réactivation ; son historique reste lisible. À chaque pas, la base, rouverte, rend les mêmes
// résultats et le même dépôt que l'historique en mémoire, et se vérifie en entier.
#[test]
fn abandon_and_reactivation_are_stored_like_in_memory() {
    let file = TempFile::new("abandon");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    let s = DEFAULT_SCOPE;
    let x = BranchId::from_bytes(raw(13));
    let create = |n, kind| {
        Command::Change(Change::CreateNode {
            scope: s,
            id: node(n),
            kind,
            title: format!("note {n}"),
            text: "texte".into(),
        })
    };
    let steps: Vec<(u64, Command)> = vec![
        (10, create(1, Kind::Reasoning)),
        (11, create(2, Kind::Implementation)),
        (
            12,
            Command::Change(Change::Link {
                scope: s,
                id: EdgeId::from_bytes(raw(3)),
                from: node(2),
                to: node(1),
                relation: Relation::DependsOn,
            }),
        ),
        (
            13,
            Command::Change(Change::SetVerdict {
                scope: s,
                node: node(1),
                verdict: Verdict::Refuted,
            }),
        ),
        (14, create(4, Kind::Category)),
        (15, Command::Abandon { branch: x }),
        (16, Command::Checkout { branch: x }),
        (17, Command::Reactivate { branch: x }),
        (18, Command::Checkout { branch: x }),
        (19, Command::Checkout { branch: MAIN }),
        (20, Command::Abandon { branch: x }),
        (21, Command::Reactivate { branch: x }),
        (22, Command::Abandon { branch: x }),
    ];
    for (n, command) in steps {
        let next = envelope(store.repository(), n, command.clone());
        let stored = store.execute(next.clone()).map_err(|e| e.to_string());
        let expected = oracle
            .execute(next)
            .map_err(|e| StoreError::Domain(e).to_string());
        assert_eq!(stored, expected, "{command:?}");
        drop(store);
        store = Store::open(&file.0).unwrap();
        assert_eq!(text(&store), encode_repository_v2(&oracle), "{command:?}");
        assert_eq!(store.repository().snapshot(), oracle.snapshot());
        assert_eq!(store.repository().expected(), oracle.expected());
        assert_eq!(
            store.repository().branches_containing(node(4)),
            oracle.branches_containing(node(4)),
            "{command:?}"
        );
        assert!(store.verify().is_ok(), "{command:?}");
        if oracle.branch_count() > 1 {
            let head = oracle.branch_head(x).unwrap();
            assert_eq!(
                store.repository().snapshot_at(head).unwrap(),
                oracle.snapshot_at(head).unwrap(),
                "{command:?}"
            );
            let held =
                i64::try_from(state_lines(&oracle.snapshot_at(head).unwrap()).len()).unwrap();
            let expected_rows = if oracle.is_abandoned(x).unwrap() {
                (0, "abandonnee".to_owned())
            } else {
                (held, "active".to_owned())
            };
            assert_eq!(branch_rows(&file.0, x), expected_rows, "{command:?}");
        }
    }
    assert_eq!(store.repository().is_abandoned(x), Ok(true));
}

// L'import d'un texte qui contient une branche abandonnée ne lui tient aucun état courant ; la
// vérification complète refuse un état courant tenu pour une branche abandonnée, et l'ouverture
// refuse une base dont la branche active est abandonnée.
#[test]
fn abandoned_branches_are_imported_and_checked() {
    let mut oracle = Repository::new();
    for (n, command) in (10..).zip(scenario().into_iter().take(4)) {
        let next = envelope(&oracle, n, command);
        oracle.execute(next).unwrap();
    }
    let x = oracle.active_branch();
    assert_ne!(x, MAIN);
    let abandon = envelope(&oracle, 30, Command::Abandon { branch: x });
    oracle.execute(abandon).unwrap();
    let file = TempFile::new("import-abandon");
    let store = Store::import(&file.0, &encode_repository_v2(&oracle)).unwrap();
    assert_eq!(text(&store), encode_repository_v2(&oracle));
    assert!(store.verify().is_ok());
    drop(store);
    assert_eq!(branch_rows(&file.0, x), (0, "abandonnee".to_owned()));
    // Un état courant tenu pour la branche abandonnée diverge de ce que demande le schéma.
    tamper(
        &file.0,
        &format!(
            "INSERT INTO etat_courant (branche, cle, ligne)
             SELECT '{x}', cle, ligne FROM etat_courant WHERE branche = '{MAIN}'"
        ),
    );
    let store = Store::open(&file.0).unwrap();
    assert!(matches!(store.verify(), Err(StoreError::Schema(m)) if m.contains("état courant")));
    drop(store);
    // Branche active abandonnée : refusée à l'ouverture, avec la branche nommée.
    let file = TempFile::new("active-abandonnee");
    drop(Store::import(&file.0, &encode_repository_v2(&oracle)).unwrap());
    tamper(
        &file.0,
        &format!("UPDATE branches SET etat = 'abandonnee' WHERE id = '{MAIN}'"),
    );
    assert!(matches!(
        Store::open(&file.0),
        Err(StoreError::Historique(m)) if m.contains("branche abandonnée")
    ));
}

// ---------------------------------------------------------------------------------------------
// Fusion (incrément 5.4c ; FUSION.md, section 9.6 ; INV-MERGE-01 à 03) : le second parent d'un
// commit de fusion est écrit, relu, importé et vérifié comme en mémoire.
// ---------------------------------------------------------------------------------------------

// Fusions en équivalence pas à pas avec l'historique en mémoire, base rouverte et vérifiée à
// chaque pas : avance d'une cible restée à la base, fusion avec conflit résolu, annulation et
// rétablissement d'une fusion, fusion sans effet.
#[test]
fn merges_are_stored_like_in_memory() {
    let file = TempFile::new("fusion");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    let s = DEFAULT_SCOPE;
    let x = BranchId::from_bytes(raw(13));
    let create = |n, kind| {
        Command::Change(Change::CreateNode {
            scope: s,
            id: node(n),
            kind,
            title: format!("note {n}"),
            text: "texte".into(),
        })
    };
    let verdict = |n, verdict| {
        Command::Change(Change::SetVerdict {
            scope: s,
            node: node(n),
            verdict,
        })
    };
    let merge = |head: u64, resolutions| Command::Merge {
        source: x,
        source_head: CommitId::from_bytes(raw(head)),
        resolutions,
    };
    let conflict = Conflict::Field {
        scope: s,
        node: node(1),
        field: MemberField::Verdict,
    };
    let steps: Vec<(u64, Command)> = vec![
        (10, create(1, Kind::Reasoning)),
        (11, create(2, Kind::Implementation)),
        (
            12,
            Command::Change(Change::Link {
                scope: s,
                id: EdgeId::from_bytes(raw(3)),
                from: node(2),
                to: node(1),
                relation: Relation::DependsOn,
            }),
        ),
        (13, verdict(1, Verdict::Refuted)),
        (14, create(4, Kind::Category)),
        (15, Command::Checkout { branch: MAIN }),
        // Cible restée à la base : avance sur l'état de la source.
        (16, merge(14, vec![])),
        (17, Command::Undo),
        (18, Command::Redo),
        // Conflit de verdict, résolu côté source.
        (19, Command::Checkout { branch: x }),
        (20, verdict(1, Verdict::Unverified)),
        (21, Command::Checkout { branch: MAIN }),
        (22, verdict(1, Verdict::Verified)),
        (23, merge(20, vec![])),
        (
            24,
            merge(20, vec![Resolution::Choose(conflict, Side::Source)]),
        ),
        // Source déjà réunie : rien n'est écrit.
        (25, merge(20, vec![])),
    ];
    for (n, command) in steps {
        let next = envelope(store.repository(), n, command.clone());
        let stored = store.execute(next.clone()).map_err(|e| e.to_string());
        let expected = oracle
            .execute(next)
            .map_err(|e| StoreError::Domain(e).to_string());
        assert_eq!(stored, expected, "{command:?}");
        drop(store);
        store = Store::open(&file.0).unwrap();
        assert_eq!(text(&store), encode_repository_v2(&oracle), "{command:?}");
        assert_eq!(store.repository().snapshot(), oracle.snapshot());
        assert_eq!(store.repository().expected(), oracle.expected());
        assert!(store.verify().is_ok(), "{command:?}");
    }
    for merged in [16, 24] {
        let commit = CommitId::from_bytes(raw(merged));
        let info = store.repository().history().commit(commit).unwrap();
        assert_eq!(info, oracle.history().commit(commit).unwrap());
        assert_eq!(info.map(|i| i.parents.len()), Some(2));
        assert_eq!(
            store.repository().snapshot_at(commit).unwrap(),
            oracle.snapshot_at(commit).unwrap()
        );
    }
    let commits: i64 = Connection::open(&file.0)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM commits WHERE parent2 IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(commits, 2);
    // Import du texte : mêmes commits de fusion, même texte.
    let copy = TempFile::new("fusion-import");
    let imported = Store::import(&copy.0, &encode_repository_v2(&oracle)).unwrap();
    assert_eq!(text(&imported), encode_repository_v2(&oracle));
    assert!(imported.verify().is_ok());
}

// Citations de notes et relations vers les échanges (incrément 5.5 ; CITATIONS.md ; D18, D24) :
// écrites, relues et vérifiées comme en mémoire, sans changement de schéma.
#[test]
fn note_citations_and_exchange_links_are_stored_like_in_memory() {
    let file = TempFile::new("citations");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    let s = DEFAULT_SCOPE;
    let steps: Vec<(u64, Command)> = vec![
        (
            10,
            Command::Change(Change::CreateNode {
                scope: s,
                id: node(1),
                kind: Kind::Reasoning,
                title: "invariant".into(),
                text: "la priorité est conservée".into(),
            }),
        ),
        (
            11,
            Command::Change(Change::CreateExchange {
                scope: s,
                id: node(2),
                title: "conversation".into(),
                question: Some("question".into()),
                answer: "réponse".into(),
                previous: None,
            }),
        ),
        (
            12,
            Command::Lot(vec![
                Change::CiteNote {
                    scope: s,
                    id: node(3),
                    note: node(1),
                    version: 0,
                    start: 3,
                    end: 12,
                },
                Change::Link {
                    scope: s,
                    id: EdgeId::from_bytes(raw(4)),
                    from: node(3),
                    to: node(1),
                    relation: Relation::DevelopedFrom,
                },
            ]),
        ),
        (
            13,
            Command::Change(Change::Link {
                scope: s,
                id: EdgeId::from_bytes(raw(5)),
                from: node(1),
                to: node(2),
                relation: Relation::DependsOn,
            }),
        ),
        (
            14,
            Command::Change(Change::Revise {
                node: node(1),
                text: "la priorité change".into(),
                kind: RevisionKind::Minor,
            }),
        ),
        (
            15,
            Command::Change(Change::Archive {
                scope: s,
                node: node(2),
            }),
        ),
    ];
    for (n, command) in steps {
        let next = envelope(store.repository(), n, command.clone());
        let stored = store.execute(next.clone()).map_err(|e| e.to_string());
        let expected = oracle
            .execute(next)
            .map_err(|e| StoreError::Domain(e).to_string());
        assert_eq!(stored, expected, "{command:?}");
        drop(store);
        store = Store::open(&file.0).unwrap();
        assert_eq!(text(&store), encode_repository_v2(&oracle), "{command:?}");
        assert_eq!(store.repository().snapshot(), oracle.snapshot());
        assert!(store.verify().is_ok(), "{command:?}");
    }
    let view = store.repository().snapshot().citation(node(3)).unwrap();
    assert!(view.source_revised);
    let graph = store.repository().snapshot().graph(s).unwrap();
    assert_eq!(
        graph.review_causes().get(&node(1)),
        Some(&std::collections::BTreeSet::from([node(2)]))
    );
}

// CA-27 (RESTAURATION.md, incrément 6.4) : après réouverture d'une base SQLite, une idée écartée
// est restaurée avec ses liens internes sans revenir sur le travail indépendant ; plan, résultat
// et historique sont ceux de l'historique en mémoire ; un « annuler » défait la restauration.
#[test]
fn a_targeted_restore_after_reopening_is_the_same_as_in_memory() {
    let file = TempFile::new("restauration");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    let s = DEFAULT_SCOPE;
    let create = |n, kind| {
        Command::Change(Change::CreateNode {
            scope: s,
            id: node(n),
            kind,
            title: format!("note {n}"),
            text: "texte".into(),
        })
    };
    let mut steps: Vec<(u64, Command)> = vec![
        (10, create(1, Kind::Reasoning)),
        (11, create(2, Kind::Implementation)),
        (
            12,
            Command::Change(Change::Link {
                scope: s,
                id: EdgeId::from_bytes(raw(3)),
                from: node(2),
                to: node(1),
                relation: Relation::DependsOn,
            }),
        ),
        (
            13,
            Command::Change(Change::SetDecision {
                scope: s,
                node: node(1),
                decision: Decision::NoLongerRetained,
            }),
        ),
        (14, create(4, Kind::PersonalNote)),
    ];
    let target = CommitId::from_bytes(raw(12));
    let selection: std::collections::BTreeSet<NodeId> = [node(1), node(2)].into();
    for (n, command) in steps.drain(..) {
        let next = envelope(store.repository(), n, command);
        store.execute(next.clone()).unwrap();
        oracle.execute(next).unwrap();
    }
    drop(store);
    let mut store = Store::open(&file.0).unwrap();
    assert_eq!(
        store.repository().restore_plan(target, Some(&selection)),
        oracle.restore_plan(target, Some(&selection))
    );
    assert_eq!(
        store.repository().history_of_branch(20),
        oracle.history_of_branch(20)
    );
    for (n, command) in [
        (
            15,
            Command::Change(Change::RestoreNodes {
                target,
                nodes: selection.clone(),
            }),
        ),
        (16, Command::Undo),
        (17, Command::Redo),
    ] {
        let next = envelope(store.repository(), n, command.clone());
        assert_eq!(
            store.execute(next.clone()).map_err(|e| e.to_string()),
            oracle
                .execute(next)
                .map_err(|e| StoreError::Domain(e).to_string()),
            "{command:?}"
        );
        drop(store);
        store = Store::open(&file.0).unwrap();
        assert_eq!(text(&store), encode_repository_v2(&oracle), "{command:?}");
        assert!(store.verify().is_ok());
    }
    let g = store.repository().snapshot().graph(s).unwrap();
    assert_eq!(
        g.node(node(1)).unwrap().decision(),
        Some(Decision::Proposed)
    );
    assert!(g.node(node(4)).is_ok());
}

// Incrément 6.5b (FICHE-REPRISE.md, section 3 ; INV-RESUME-01, CA-08) : une fiche de reprise est un
// commit sans différence ; après fermeture et réouverture de la base, elle se relit à l'identique,
// avec son état du code, comme dans l'historique en mémoire ; annulation et rétablissement aussi.
#[test]
fn resume_points_are_stored_like_in_memory() {
    let file = TempFile::new("reprise");
    let mut store = Store::create(&file.0).unwrap();
    let mut oracle = Repository::new();
    let point = ResumePoint {
        date: "2026-11-09 08:30 UTC".into(),
        objective: "isoler le réveil perdu".into(),
        next_action: "instrumenter wake_task".into(),
        notes: [node(1)].into(),
        code: Some(CodeState::new(
            Some("a1b2c3d4".into()),
            Some("principale".into()),
            vec![" M src/a.c".into(), "?? b.c".into()],
            b"diff --git a/src/a.c b/src/a.c\n",
        )),
        ..ResumePoint::default()
    };
    let steps = [
        (
            10,
            Command::Change(Change::CreateNode {
                scope: DEFAULT_SCOPE,
                id: node(1),
                kind: Kind::Reasoning,
                title: "réveil perdu".into(),
                text: "texte".into(),
            }),
        ),
        (11, Command::Resume(Box::new(point.clone()))),
        (12, Command::Undo),
        (13, Command::Redo),
    ];
    for (n, command) in steps {
        let next = envelope(store.repository(), n, command.clone());
        assert_eq!(
            store.execute(next.clone()).map_err(|e| e.to_string()),
            oracle
                .execute(next)
                .map_err(|e| StoreError::Domain(e).to_string()),
            "{command:?}"
        );
        drop(store);
        store = Store::open(&file.0).unwrap();
        assert_eq!(text(&store), encode_repository_v2(&oracle), "{command:?}");
        assert!(store.verify().is_ok());
    }
    let fiche = CommitId::from_bytes(raw(11));
    assert_eq!(store.repository().resume_point(fiche), Ok(point.clone()));
    assert_eq!(
        store.repository().resume_points(MAIN, 10),
        Ok(vec![ResumeEntry {
            commit: fiche,
            point
        }])
    );
    assert_eq!(store.repository().operations_since(fiche), Ok(Some(0)));
    assert_eq!(
        *store.repository().snapshot_at(fiche).unwrap(),
        *store
            .repository()
            .snapshot_at(CommitId::from_bytes(raw(10)))
            .unwrap()
    );
}
