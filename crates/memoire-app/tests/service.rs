//! Contrat de la couche service (SERVICE.md, section 4). Écrit d'après SERVICE.md ; une première
//! version du code existait déjà, ce qui est consigné dans l'historique.
//!
//! Contrat modifié (incrément 5.2c, D20) : le dépôt du service est adossé à l'historique SQLite,
//! qui n'a ni `Clone` ni `PartialEq` ; les comparaisons de dépôts entiers comparent leurs textes
//! canoniques v2 complets, de même force.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use memoire_app::{
    IdSource, Migration, MigrationError, RandomIds, SequenceIds, Service, StoreError,
};
use memoire_domain::*;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Texte canonique v2 complet du dépôt d'un service (D20 : un dépôt SQLite se compare par son
/// texte).
fn text<I: IdSource>(service: &Service<I>) -> String {
    service.text().unwrap()
}

/// Dépôt en mémoire égal à celui d'un service, relu depuis son texte.
fn memory<I: IdSource>(service: &Service<I>) -> Repository {
    decode_repository_v2(&text(service)).unwrap()
}

struct TempFile(PathBuf);

impl TempFile {
    fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("memoire-app-{name}-{}-{n}.db", std::process::id()));
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

fn create_note(
    kind: Kind,
    title: &'static str,
) -> impl FnOnce(&mut memoire_app::Fresh<'_>) -> Command {
    move |fresh| {
        Command::Change(Change::CreateNode {
            scope: DEFAULT_SCOPE,
            id: fresh.node(),
            kind,
            title: title.into(),
            text: "texte".into(),
        })
    }
}

// Identités : jamais nulles, jamais répétées ; suite déterministe en test, version 4 en usage réel.
#[test]
fn identities_are_never_nil_nor_repeated() {
    let mut sequence = SequenceIds::new(0);
    let first = sequence.next();
    assert_ne!(first, [0u8; 16]);
    assert_eq!(first[15], 1);
    let mut seen = BTreeSet::from([first]);
    for _ in 0..999 {
        assert!(seen.insert(sequence.next()));
    }
    let mut prefixed = SequenceIds::new(7);
    assert_eq!(&prefixed.next()[..8], &7u64.to_be_bytes());
    let mut random = RandomIds;
    let mut drawn = BTreeSet::new();
    for _ in 0..1000 {
        let id = random.next();
        assert_eq!(id[6] >> 4, 4, "version 4");
        assert_eq!(id[8] >> 6, 0b10, "variante RFC 4122");
        assert!(drawn.insert(id));
    }
}

// Préparer n'écrit rien ; appliquer publie exactement l'aperçu annoncé, avec les identités rendues.
#[test]
fn applying_a_plan_publishes_its_preview() {
    let file = TempFile::new("apercu");
    let mut service = Service::create(&file.0, SequenceIds::new(1)).unwrap();
    let plan = service_plan(&mut service, create_note(Kind::Reasoning, "a"));
    let a = service.apply(plan, "création").unwrap();
    let a_id = match &a.envelope.command {
        Command::Change(Change::CreateNode { id, .. }) => *id,
        other => panic!("{other:?}"),
    };
    // Préparation d'une note et d'une dépendance : identités rendues dans l'ordre d'attribution.
    let planned = service
        .plan(|fresh| {
            let id = fresh.node();
            let edge = fresh.edge();
            let _ = (fresh.scope(), fresh.anchor());
            Command::Change(Change::Link {
                scope: DEFAULT_SCOPE,
                id: edge,
                from: id,
                to: a_id,
                relation: Relation::DependsOn,
            })
        })
        .map(|p| p.assigned);
    // La note n'existe pas : le domaine refuse dès la préparation.
    assert!(matches!(
        planned,
        Err(StoreError::Domain(Error::MissingNode(_)))
    ));
    let before = text(&service);
    let refute = service
        .plan(|_| {
            Command::Change(Change::SetVerdict {
                scope: DEFAULT_SCOPE,
                node: a_id,
                verdict: Verdict::Refuted,
            })
        })
        .unwrap();
    assert_eq!(text(&service), before);
    assert_eq!(
        text(&Service::open(&file.0, SequenceIds::new(9)).unwrap()),
        before
    );
    assert!(refute.preview.publishes && refute.preview.forks);
    assert_eq!(
        refute.expected,
        decode_repository_v2(&before).unwrap().expected()
    );
    let preview = refute.preview.clone();
    let applied = service.apply(refute, "hypothèse réfutée").unwrap();
    assert_eq!(
        (
            applied.outcome.published,
            applied.outcome.forked,
            &applied.outcome.effects
        ),
        (preview.publishes, preview.forks, &preview.effects)
    );
    assert_eq!(applied.envelope.reason, "hypothèse réfutée");
    assert_eq!(
        text(&Service::open(&file.0, SequenceIds::new(9)).unwrap()),
        text(&service)
    );
}

fn service_plan(
    service: &mut Service<SequenceIds>,
    build: impl FnOnce(&mut memoire_app::Fresh<'_>) -> Command,
) -> memoire_app::Planned {
    service.plan(build).unwrap()
}

// Identités attribuées pendant la préparation, toutes sortes confondues, dans l'ordre.
#[test]
fn a_plan_reports_every_assigned_identity() {
    let file = TempFile::new("identites");
    let mut service = Service::create(&file.0, SequenceIds::new(3)).unwrap();
    let planned = service
        .plan(|fresh| {
            let node = fresh.node();
            let edge = fresh.edge();
            let scope = fresh.scope();
            let anchor = fresh.anchor();
            let second = fresh.node();
            let _ = (edge, scope, anchor, second);
            Command::Change(Change::CreateNode {
                scope: DEFAULT_SCOPE,
                id: node,
                kind: Kind::Category,
                title: "c".into(),
                text: "x".into(),
            })
        })
        .unwrap();
    let id = |n: u64| {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&3u64.to_be_bytes());
        bytes[8..].copy_from_slice(&n.to_be_bytes());
        bytes
    };
    assert_eq!(
        planned.assigned.nodes,
        vec![NodeId::from_bytes(id(1)), NodeId::from_bytes(id(5))]
    );
    assert_eq!(planned.assigned.edges, vec![EdgeId::from_bytes(id(2))]);
    assert_eq!(planned.assigned.scopes, vec![ScopeId::from_bytes(id(3))]);
    assert_eq!(planned.assigned.anchors, vec![AnchorId::from_bytes(id(4))]);
    assert_eq!(planned.op, OpId::from_bytes(id(6)));
}

// Un plan périmé est refusé sans écriture : l'interface doit replanifier.
#[test]
fn a_stale_plan_is_refused_without_writing() {
    let file = TempFile::new("perime");
    let mut service = Service::create(&file.0, SequenceIds::new(2)).unwrap();
    let first = service
        .plan(create_note(Kind::Reasoning, "premier"))
        .unwrap();
    let second = service
        .plan(create_note(Kind::Reasoning, "second"))
        .unwrap();
    service.apply(second, "création").unwrap();
    let after = text(&service);
    assert!(matches!(
        service.apply(first, "création"),
        Err(StoreError::Domain(Error::Stale { .. }))
    ));
    assert_eq!(text(&service), after);
    assert_eq!(
        text(&Service::open(&file.0, SequenceIds::new(9)).unwrap()),
        after
    );
}

// Répétition : l'enveloppe rendue par l'application, rejouée après réouverture, rend le même
// résultat sans double effet (INV-RETRY-01).
#[test]
fn a_returned_envelope_can_be_retried_after_reopening() {
    let file = TempFile::new("repetition");
    let mut service = Service::create(&file.0, SequenceIds::new(4)).unwrap();
    let plan = service.plan(create_note(Kind::Category, "c")).unwrap();
    let applied = service.apply(plan, "création").unwrap();
    drop(service);
    let mut service = Service::open(&file.0, SequenceIds::new(4)).unwrap();
    let before = text(&service);
    assert_eq!(service.retry(applied.envelope).unwrap(), applied.outcome);
    assert_eq!(text(&service), before);
}

// D12 : un changement de statut sans motif est refusé, sans écriture.
#[test]
fn a_status_change_requires_a_reason() {
    let file = TempFile::new("motif");
    let mut service = Service::create(&file.0, SequenceIds::new(5)).unwrap();
    let plan = service.plan(create_note(Kind::Reasoning, "a")).unwrap();
    let node = plan.assigned.nodes[0];
    service.apply(plan, "création").unwrap();
    let before = text(&service);
    let refute = service
        .plan(|_| {
            Command::Change(Change::SetVerdict {
                scope: DEFAULT_SCOPE,
                node,
                verdict: Verdict::Refuted,
            })
        })
        .unwrap();
    assert!(matches!(
        service.apply(refute, "  "),
        Err(StoreError::Domain(Error::MissingReason))
    ));
    assert_eq!(text(&service), before);
    assert_eq!(
        text(&Service::open(&file.0, SequenceIds::new(9)).unwrap()),
        before
    );
}

// ---------------------------------------------------------------------------
// Incrément 3.4 — contrat ajouté avant le code : export et import (SERVICE.md, section 5).
// ---------------------------------------------------------------------------

/// Service avec un historique varié : créations, dépendance, réfutation qui bifurque, annulation,
/// ancrage. Rend aussi une enveloppe appliquée, pour vérifier la répétition après import.
fn busy_service(file: &TempFile) -> (Service<SequenceIds>, Envelope, Outcome) {
    let mut service = Service::create(&file.0, SequenceIds::new(6)).unwrap();
    let a = service
        .plan(create_note(Kind::Reasoning, "hypothèse"))
        .unwrap();
    let a_id = a.assigned.nodes[0];
    let first = service.apply(a, "création").unwrap();
    let b = service
        .plan(create_note(Kind::Implementation, "s'appuie"))
        .unwrap();
    let b_id = b.assigned.nodes[0];
    service.apply(b, "création").unwrap();
    let link = service
        .plan(|fresh| {
            Command::Change(Change::Link {
                scope: DEFAULT_SCOPE,
                id: fresh.edge(),
                from: b_id,
                to: a_id,
                relation: Relation::DependsOn,
            })
        })
        .unwrap();
    service.apply(link, "dépendance").unwrap();
    let refute = service
        .plan(|_| {
            Command::Change(Change::SetVerdict {
                scope: DEFAULT_SCOPE,
                node: a_id,
                verdict: Verdict::Refuted,
            })
        })
        .unwrap();
    service.apply(refute, "réfutée").unwrap();
    let undo = service.plan(|_| Command::Undo).unwrap();
    service.apply(undo, "retour").unwrap();
    let anchor = service
        .plan(|fresh| {
            Command::Change(Change::AnchorToCode {
                id: fresh.anchor(),
                node: b_id,
                path: "src/driver.c".into(),
                excerpt: "void refresh() {}".into(),
                git: Some("a1b2c3d".into()),
            })
        })
        .unwrap();
    service.apply(anchor, "ancrage").unwrap();
    (service, first.envelope, first.outcome)
}

// INV-EXPORT-01, CA-09 : export puis import redonnent le dépôt, historique et journal compris.
// Contrat modifié (incrément 5.1) : l'export est écrit au format version 2 (VERSION2.md, section 6).
#[test]
fn export_then_import_gives_back_the_repository() {
    let base = TempFile::new("source");
    let (service, envelope, outcome) = busy_service(&base);
    let text_file = TempFile::new("export");
    service.export(&text_file.0).unwrap();
    let text = std::fs::read_to_string(&text_file.0).unwrap();
    assert_eq!(text, self::text(&service));
    let copy = TempFile::new("copie");
    let mut imported = Service::import(&text, &copy.0, SequenceIds::new(8)).unwrap();
    assert_eq!(self::text(&imported), self::text(&service));
    // Le journal suit : rejouer une enveloppe dans le dépôt importé rend le même résultat.
    assert_eq!(imported.retry(envelope).unwrap(), outcome);
    assert_eq!(self::text(&imported), self::text(&service));
    // Et la base importée se rouvre à l'identique.
    drop(imported);
    let reopened = Service::open(&copy.0, SequenceIds::new(9)).unwrap();
    assert_eq!(self::text(&reopened), self::text(&service));
}

// Deux exports d'un même dépôt sont identiques ; un nouvel export remplace l'ancien, sans laisser
// de fichier temporaire. Contrat modifié (incrément 5.1) : export en version 2.
#[test]
fn exports_are_deterministic_and_replace_atomically() {
    let base = TempFile::new("deterministe");
    let (mut service, _, _) = busy_service(&base);
    let text_file = TempFile::new("export-double");
    service.export(&text_file.0).unwrap();
    let first = std::fs::read(&text_file.0).unwrap();
    service.export(&text_file.0).unwrap();
    assert_eq!(std::fs::read(&text_file.0).unwrap(), first);
    let more = service
        .plan(create_note(Kind::Category, "nouvelle"))
        .unwrap();
    service.apply(more, "création").unwrap();
    service.export(&text_file.0).unwrap();
    let replaced = std::fs::read_to_string(&text_file.0).unwrap();
    assert_eq!(replaced, text(&service));
    assert_ne!(replaced.as_bytes(), first.as_slice());
    let directory = text_file.0.parent().unwrap();
    let stem = text_file
        .0
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let leftovers: Vec<String> = std::fs::read_dir(directory)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(&stem) && *name != stem)
        .collect();
    assert!(
        leftovers.is_empty(),
        "fichiers temporaires restants : {leftovers:?}"
    );
}

// L'import ne remplace jamais une base existante ; un texte invalide est refusé avec sa ligne,
// sans qu'aucun fichier de base soit créé. Contrat modifié (incrément 5.1) : l'en-tête invalide
// devient « memoire-depot 9 », car « memoire-depot 2 » est désormais lu.
#[test]
fn import_refuses_existing_bases_and_invalid_texts() {
    let base = TempFile::new("existante");
    let (service, _, _) = busy_service(&base);
    let text = encode_repository(&memory(&service));
    assert!(matches!(
        Service::import(&text, &base.0, SequenceIds::new(1)),
        Err(StoreError::AlreadyExists)
    ));
    let target = TempFile::new("jamais-creee");
    let broken = text.replacen("memoire-depot 1", "memoire-depot 9", 1);
    match Service::import(&broken, &target.0, SequenceIds::new(1)) {
        Err(StoreError::Decode(error)) => assert_eq!(error.line, 1),
        other => panic!("refus attendu, obtenu {other:?}", other = other.map(|_| ())),
    }
    assert!(!target.0.exists());
}

// VERSION2.md, section 7 : un export en version 1, écrit par une version antérieure, s'importe
// toujours ; un export en version 2 aussi.
#[test]
fn a_v1_export_can_still_be_imported() {
    let base = TempFile::new("export-v1");
    let (service, _, _) = busy_service(&base);
    for (name, text) in [
        ("import-v1", encode_repository(&memory(&service))),
        ("import-v2", text(&service)),
    ] {
        let copy = TempFile::new(name);
        let imported = Service::import(&text, &copy.0, SequenceIds::new(1)).unwrap();
        assert_eq!(self::text(&imported), self::text(&service), "{name}");
    }
}

// Une source en boîte délègue à la source qu'elle contient (ligne de commande).
#[test]
fn a_boxed_source_delegates() {
    let mut boxed: Box<dyn IdSource> = Box::new(SequenceIds::new(5));
    let mut plain = SequenceIds::new(5);
    for _ in 0..3 {
        assert_eq!(boxed.next(), plain.next());
    }
}

// R9 : la sauvegarde passe par le service.
#[test]
fn the_service_writes_backups() {
    let base = TempFile::new("sauvegarde");
    let (service, _, _) = busy_service(&base);
    let copie = TempFile::new("sauvegarde-copie");
    service.backup(&copie.0).unwrap();
    let relue = Service::open(&copie.0, SequenceIds::new(9)).unwrap();
    assert_eq!(text(&relue), text(&service));
}

// ---------------------------------------------------------------------------------------------
// Migration du schéma 1 vers le schéma 2 (incrément 5.2d ; ETAT-HISTORIQUE.md, sections 7 et 16).
// ---------------------------------------------------------------------------------------------

/// Base de schéma 1 écrite par l'exécutable de `v0.5.0-alpha.1`, et son export v2.
const SCHEMA_1_DB: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/schema1/sref01-schema1.db"
);
const SCHEMA_1_EXPORT: &str = include_str!("../../../fixtures/schema1/sref01-export-v2.txt");

/// Répertoire propre au test, supprimé avec son contenu.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("memoire-app-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    /// Noms des fichiers du répertoire, triés.
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Copie de la base figée en mode WAL, comme une base réelle, dans le répertoire.
fn schema_1_in(dir: &TempDir) -> PathBuf {
    let base = dir.0.join("depot.db");
    std::fs::copy(SCHEMA_1_DB, &base).unwrap();
    rusqlite::Connection::open(&base)
        .unwrap()
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    base
}

// INV-MIGR-01 : la base est remplacée par sa migration, de façon atomique ; elle s'ouvre ensuite
// et redonne l'export de la version qui l'a écrite ; la sauvegarde est faite avant ; aucun
// fichier temporaire ne reste ; une seconde migration ne fait rien.
#[test]
fn the_service_migrates_a_base_in_place() {
    let dir = TempDir::new("migration");
    let base = schema_1_in(&dir);
    let original = std::fs::read(&base).unwrap();
    assert!(matches!(
        Service::open(&base, SequenceIds::new(1)),
        Err(StoreError::Schema(m)) if m.contains("memoire migrer")
    ));
    let backup = dir.0.join("avant-migration.db");
    let migrated = Service::<SequenceIds>::migrate(&base, &backup).unwrap();
    let Migration::Migrated(verified) = migrated else {
        panic!("migration attendue : {migrated:?}");
    };
    assert_eq!(verified.commits, 47);
    assert_eq!(verified.heads.len(), 2);
    assert_eq!(dir.names(), ["avant-migration.db", "depot.db"]);
    assert!(!std::fs::read(&backup).unwrap().is_empty());
    assert_ne!(std::fs::read(&base).unwrap(), original);
    let mut service = Service::open(&base, SequenceIds::new(1)).unwrap();
    assert_eq!(text(&service), SCHEMA_1_EXPORT);
    assert_eq!(service.verify().unwrap(), verified);
    // Une commande ordinaire s'applique sur la base migrée et y reste.
    let planned = service
        .plan(create_note(Kind::Reasoning, "après la migration"))
        .unwrap();
    service.apply(planned, "essai").unwrap();
    let after = text(&service);
    drop(service);
    assert_eq!(
        text(&Service::open(&base, SequenceIds::new(2)).unwrap()),
        after
    );
    // Seconde migration : rien à faire, aucune sauvegarde.
    let again = dir.0.join("seconde.db");
    assert!(matches!(
        Service::<SequenceIds>::migrate(&base, &again),
        Ok(Migration::AlreadyCurrent)
    ));
    assert!(!again.exists());
    assert_eq!(
        text(&Service::open(&base, SequenceIds::new(3)).unwrap()),
        after
    );
}

// INV-MIGR-02 : une migration refusée laisse la base d'origine intacte et aucun fichier
// temporaire ; une base ouverte par un autre processus n'est pas remplacée.
#[test]
fn a_refused_migration_leaves_the_base_intact() {
    let dir = TempDir::new("migration-refus");
    let base = schema_1_in(&dir);
    rusqlite::Connection::open(&base)
        .unwrap()
        .execute_batch("UPDATE fragments SET texte = replace(texte, 'titre', 'title') WHERE categorie = 'commit'")
        .unwrap();
    let before = std::fs::read(&base).unwrap();
    let backup = dir.0.join("avant-migration.db");
    assert!(matches!(
        Service::<SequenceIds>::migrate(&base, &backup),
        Err(MigrationError::Store(StoreError::Decode(_)))
    ));
    assert_eq!(std::fs::read(&base).unwrap(), before);
    assert_eq!(dir.names(), ["avant-migration.db", "depot.db"]);

    // Base gardée ouverte ailleurs : la migration est vérifiée, mais pas mise en place.
    let dir = TempDir::new("migration-ouverte");
    let base = schema_1_in(&dir);
    let other = rusqlite::Connection::open(&base).unwrap();
    let count: i64 = other
        .query_row("SELECT count(*) FROM fragments", [], |r| r.get(0))
        .unwrap();
    assert!(count > 0);
    let before = std::fs::read(&base).unwrap();
    let backup = dir.0.join("avant-migration.db");
    let refused = Service::<SequenceIds>::migrate(&base, &backup);
    assert!(matches!(refused, Err(MigrationError::InUse)), "{refused:?}");
    assert!(refused.unwrap_err().to_string().contains("autre processus"));
    drop(other);
    assert_eq!(std::fs::read(&base).unwrap(), before);
    assert!(
        dir.names().iter().all(|n| !n.ends_with(".tmp")),
        "{:?}",
        dir.names()
    );
    assert!(backup.exists());
    // Une fois l'autre processus fermé, la migration réussit.
    let backup = dir.0.join("avant-migration-2.db");
    assert!(matches!(
        Service::<SequenceIds>::migrate(&base, &backup),
        Ok(Migration::Migrated(_))
    ));
    assert_eq!(
        text(&Service::open(&base, SequenceIds::new(1)).unwrap()),
        SCHEMA_1_EXPORT
    );

    // Base absente : erreur, aucun fichier créé.
    let dir = TempDir::new("migration-absente");
    let refused = Service::<SequenceIds>::migrate(&dir.0.join("depot.db"), &dir.0.join("s.db"));
    assert!(refused.is_err());
    assert!(dir.names().is_empty());
}

// La sauvegarde de la migration peut aller dans un répertoire encore absent : il est créé.
#[test]
fn the_backup_directory_of_a_migration_is_created() {
    let dir = TempDir::new("migration-repertoire");
    let base = schema_1_in(&dir);
    let backup = dir.0.join("sauvegardes").join("avant-migration.db");
    assert!(matches!(
        Service::<SequenceIds>::migrate(&base, &backup),
        Ok(Migration::Migrated(_))
    ));
    assert_eq!(dir.names(), ["depot.db", "sauvegardes"]);
    assert!(backup.is_file());
}

// INV-LOT-01, INV-ID-04 (incrément 5.3, LOT.md, section 4) : un lot se prépare et s'applique par
// le service comme une commande seule ; l'aperçu rapporte toutes les identités tirées, et un
// seul commit est publié.
#[test]
fn a_lot_is_planned_and_applied_through_the_service() {
    let file = TempFile::new("lot");
    let mut service = Service::create(&file.0, SequenceIds::new(7)).unwrap();
    let category = service.plan(create_note(Kind::Category, "pilote")).unwrap();
    let category = match &service.apply(category, "").unwrap().envelope.command {
        Command::Change(Change::CreateNode { id, .. }) => *id,
        other => panic!("{other:?}"),
    };
    let commits = service.repository().commit_count();
    let planned = service
        .plan(|fresh| {
            let id = fresh.node();
            Command::Lot(vec![
                Change::CreateNode {
                    scope: DEFAULT_SCOPE,
                    id,
                    kind: Kind::Reasoning,
                    title: "réveil".into(),
                    text: "texte".into(),
                },
                Change::Link {
                    scope: DEFAULT_SCOPE,
                    id: fresh.edge(),
                    from: id,
                    to: category,
                    relation: Relation::ClassifiedIn,
                },
                Change::Link {
                    scope: DEFAULT_SCOPE,
                    id: fresh.edge(),
                    from: id,
                    to: category,
                    relation: Relation::Related,
                },
            ])
        })
        .unwrap();
    assert_eq!(planned.assigned.nodes.len(), 1);
    assert_eq!(planned.assigned.edges.len(), 2);
    assert!(planned.preview.publishes && !planned.preview.forks);
    let applied = service.apply(planned, "").unwrap();
    assert!(applied.outcome.published);
    assert_eq!(service.repository().commit_count(), commits + 1);
    let mut oracle = Repository::new();
    for (envelope, _) in memory(&service).journal() {
        oracle.execute(envelope.clone()).unwrap();
    }
    assert_eq!(encode_repository_v2(&oracle), text(&service));
}
