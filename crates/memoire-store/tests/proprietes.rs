//! Propriétés à travers SQLite (ETAT-HISTORIQUE.md, section 9 ; D20).
//!
//! Les suites aléatoires de `memoire-domain/tests/proprietes.rs` (même générateur, copié ici :
//! un test ne peut pas en importer un autre) sont appliquées à une base SQLite au schéma 2 et à
//! l'historique en mémoire, qui sert d'oracle. Après chaque commande : même résultat (ou même
//! refus), même état de tête, même texte canonique v2 complet. La base est rouverte à des
//! instants tirés au hasard, et vérifiée en entier à la fin de chaque suite.
//!
//! Hors mutation : ces tests complètent le contrat du stockage.

use std::path::{Path, PathBuf};

use memoire_domain::*;
use memoire_store::{Store, StoreError};

/// Générateur SplitMix64 : reproductible, suffisant pour engendrer des suites de commandes.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n as u64)).unwrap()
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> Option<T> {
        (!items.is_empty()).then(|| items[self.below(items.len())])
    }
}

const ANSWER: &str = "réponse d'essai accentuée 🚀 fin";

/// Identités connues, pour viser des objets existants, abandonnés ou inexistants.
#[derive(Default)]
struct Known {
    nodes: Vec<NodeId>,
    edges: Vec<EdgeId>,
    scopes: Vec<ScopeId>,
    anchors: Vec<AnchorId>,
    commits: Vec<CommitId>,
    branches: Vec<BranchId>,
}

struct World {
    random: Random,
    counter: u64,
    known: Known,
}

impl World {
    fn new(seed: u64) -> Self {
        let mut known = Known::default();
        known.scopes.push(DEFAULT_SCOPE);
        known.commits.push(ROOT);
        known.branches.push(MAIN);
        Self {
            random: Random(seed),
            counter: 0,
            known,
        }
    }

    fn fresh(&mut self) -> [u8; 16] {
        self.counter += 1;
        let mut bytes = [0u8; 16];
        bytes[0] = 0x7e;
        bytes[8..].copy_from_slice(&self.counter.to_be_bytes());
        bytes
    }

    fn node(&mut self) -> NodeId {
        // Parfois une identité inconnue, pour exercer les refus.
        if self.known.nodes.is_empty() || self.random.below(12) == 0 {
            return NodeId::from_bytes(self.fresh());
        }
        self.random.pick(&self.known.nodes).unwrap()
    }

    fn scope(&mut self) -> ScopeId {
        self.random.pick(&self.known.scopes).unwrap()
    }

    fn edge(&mut self) -> EdgeId {
        match self.random.pick(&self.known.edges) {
            Some(edge) if self.random.below(10) != 0 => edge,
            _ => EdgeId::from_bytes(self.fresh()),
        }
    }

    /// Commande suivante ; avec un dépôt, une sur quarante environ est un lot (incrément 5.3),
    /// construit comme dans `memoire-domain/tests/proprietes.rs`. Sans dépôt, la suite tirée est
    /// exactement celle d'avant 5.3. Avec `abandons`, une sur huit environ abandonne ou réactive
    /// une branche connue (incrément 5.4a) ; sans, aucun tirage de plus. Avec `merges`, une sur
    /// dix environ est une fusion résolue au hasard, construite comme dans
    /// `memoire-domain/tests/proprietes.rs` (incrément 5.4c).
    fn command(
        &mut self,
        lots: Option<&Repository>,
        abandons: bool,
        merges: Option<&Repository>,
    ) -> Command {
        if let Some(repo) = merges {
            if self.random.below(10) == 0 {
                return self.merge(repo);
            }
        }
        if abandons && self.random.below(8) == 0 {
            let branch = self.random.pick(&self.known.branches).unwrap();
            return if self.random.below(2) == 0 {
                Command::Abandon { branch }
            } else {
                Command::Reactivate { branch }
            };
        }
        let change = self.change();
        match self.random.below(10) {
            0 => Command::Undo,
            1 => Command::Redo,
            2 => Command::Checkout {
                branch: self.random.pick(&self.known.branches).unwrap(),
            },
            _ => match lots {
                Some(repo) if self.random.below(4) == 0 => self.lot(change, repo),
                _ => Command::Change(change),
            },
        }
    }

    /// Fusion d'une branche connue, résolue au hasard d'après son plan sur `repo`.
    fn merge(&mut self, repo: &Repository) -> Command {
        let source = self.random.pick(&self.known.branches).unwrap();
        let source_head = repo.branch_head(source).unwrap_or(ROOT);
        let mut resolutions = Vec::new();
        if let Ok(plan) = repo.merge_plan(source, &[]) {
            for conflict in plan.conflicts {
                let side = if self.random.below(2) == 0 {
                    Side::Target
                } else {
                    Side::Source
                };
                resolutions.push(Resolution::Choose(conflict, side));
            }
            if let Ok(Some(structural)) = repo
                .merge_plan(source, &resolutions)
                .map(|plan| plan.structural)
            {
                if let Some((scope, edge)) = self.random.pick(&structural.candidates) {
                    resolutions.push(Resolution::Deactivate { scope, edge });
                }
            }
        }
        Command::Merge {
            source,
            source_head,
            resolutions,
        }
    }

    /// Lot de 2 à 5 éléments ; le premier est le changement donné, ou un autre qui passe seul.
    /// Les éléments suivants sont gardés s'ils laissent le lot acceptable (aperçu sur `repo`),
    /// sauf un sur dix, gardé sans contrôle pour exercer le refus d'un élément.
    fn lot(&mut self, first: Change, repo: &Repository) -> Command {
        let mut first = first;
        for _ in 0..8 {
            if repo.preview(&Command::Lot(vec![first.clone()])).is_ok() {
                break;
            }
            first = self.change();
        }
        self.learn_change(&first);
        let mut elements = vec![first];
        let wanted = 2 + self.random.below(4);
        for _ in 0..12 {
            if elements.len() >= wanted {
                break;
            }
            let next = self.change();
            let unchecked = self.random.below(10) == 0;
            let mut tried = elements.clone();
            tried.push(next.clone());
            if unchecked || repo.preview(&Command::Lot(tried)).is_ok() {
                self.learn_change(&next);
                elements.push(next);
            }
        }
        Command::Lot(elements)
    }

    fn change(&mut self) -> Change {
        let r = &mut self.random;
        let kinds = [
            Kind::Reasoning,
            Kind::Implementation,
            Kind::PersonalNote,
            Kind::Category,
        ];
        let relations = [
            Relation::ClassifiedIn,
            Relation::DependsOn,
            Relation::DevelopedFrom,
            Relation::Related,
            Relation::AppliesTo,
        ];
        let choice = r.below(26);
        let kind = r.pick(&kinds).unwrap();
        let relation = r.pick(&relations).unwrap();
        let revision = if r.below(2) == 0 {
            RevisionKind::Minor
        } else {
            RevisionKind::Substantive
        };
        let verdict = r
            .pick(&[Verdict::Unverified, Verdict::Verified, Verdict::Refuted])
            .unwrap();
        let decision = r
            .pick(&[
                Decision::Proposed,
                Decision::Retained,
                Decision::NoLongerRetained,
                Decision::Replaced,
            ])
            .unwrap();
        let status = r
            .pick(&[
                ReserveStatus::Open,
                ReserveStatus::TreatedPendingVerification,
                ReserveStatus::Resolved,
            ])
            .unwrap();
        let text = format!("texte {}", r.below(4));
        let (start, end) = (r.below(ANSWER.len() + 2), r.below(ANSWER.len() + 2));
        let scope = self.scope();
        match choice {
            0..=3 => Change::CreateNode {
                scope,
                id: NodeId::from_bytes(self.fresh()),
                kind,
                title: "t".into(),
                text,
            },
            4 => Change::Revise {
                node: self.node(),
                text,
                kind: revision,
            },
            5 => Change::SetVerdict {
                scope,
                node: self.node(),
                verdict,
            },
            6 => Change::SetDecision {
                scope,
                node: self.node(),
                decision,
            },
            7 => Change::Archive {
                scope,
                node: self.node(),
            },
            8..=10 => Change::Link {
                scope,
                id: EdgeId::from_bytes(self.fresh()),
                from: self.node(),
                to: self.node(),
                relation,
            },
            11 => Change::Reattach {
                scope,
                edge: self.edge(),
                to: self.node(),
                id: EdgeId::from_bytes(self.fresh()),
            },
            12 => Change::Replace {
                scope,
                id: EdgeId::from_bytes(self.fresh()),
                replacement: self.node(),
                replaced: self.node(),
            },
            13 => Change::Restrict {
                scope,
                id: EdgeId::from_bytes(self.fresh()),
                from: self.node(),
                to: self.node(),
                condition: "condition".into(),
            },
            14 => Change::Detach {
                scope,
                edge: self.edge(),
            },
            15 => Change::Acknowledge {
                scope,
                dependent: self.node(),
                cause: self.node(),
            },
            16 => Change::CloneScope {
                source: scope,
                id: ScopeId::from_bytes(self.fresh()),
            },
            17 => Change::Include {
                scope,
                node: self.node(),
            },
            18 => Change::CreateExchange {
                scope,
                id: NodeId::from_bytes(self.fresh()),
                title: "conversation".into(),
                question: (self.random.below(3) != 0).then(|| "question ?".into()),
                answer: ANSWER.into(),
                previous: (self.random.below(2) == 0).then(|| self.node()),
            },
            19 => Change::CreatePassage {
                scope,
                id: NodeId::from_bytes(self.fresh()),
                exchange: self.node(),
                part: Part::Answer,
                start: start.min(end),
                end: start.max(end),
            },
            20 => Change::CreateReserve {
                scope,
                id: NodeId::from_bytes(self.fresh()),
                passage: self.node(),
                title: "réserve".into(),
                target: None,
            },
            21 => Change::Treat {
                scope,
                id: EdgeId::from_bytes(self.fresh()),
                note: self.node(),
                reserve: self.node(),
            },
            22 => Change::SetReserveStatus {
                scope,
                reserve: self.node(),
                status,
            },
            23 => Change::AnchorToCode {
                id: AnchorId::from_bytes(self.fresh()),
                node: self.node(),
                path: "src/driver.c".into(),
                excerpt: "void refresh() {}".into(),
                git: None,
            },
            24 => match self.random.pick(&self.known.anchors) {
                Some(anchor) => Change::Unanchor { anchor },
                None => Change::Archive {
                    scope,
                    node: self.node(),
                },
            },
            _ => Change::Restore {
                target: self.random.pick(&self.known.commits).unwrap(),
            },
        }
    }

    /// Enregistre les identités nouvelles d'une commande acceptée.
    fn learn(&mut self, command: &Command, outcome: &Outcome) {
        self.known.commits.push(outcome.head);
        if !self.known.branches.contains(&outcome.branch) {
            self.known.branches.push(outcome.branch);
        }
        match command {
            Command::Change(change) => self.learn_change(change),
            Command::Lot(elements) => elements.iter().for_each(|c| self.learn_change(c)),
            _ => {}
        }
    }

    /// Enregistre l'identité nouvelle d'une modification.
    fn learn_change(&mut self, change: &Change) {
        match change {
            Change::CreateNode { id, .. }
            | Change::CreateExchange { id, .. }
            | Change::CreatePassage { id, .. }
            | Change::CreateReserve { id, .. } => self.known.nodes.push(*id),
            Change::Link { id, .. }
            | Change::Reattach { id, .. }
            | Change::Replace { id, .. }
            | Change::Restrict { id, .. }
            | Change::Contradict { id, .. }
            | Change::Treat { id, .. } => self.known.edges.push(*id),
            Change::CloneScope { id, .. } => self.known.scopes.push(*id),
            Change::AnchorToCode { id, .. } => self.known.anchors.push(*id),
            _ => {}
        }
    }
}

fn path(seed: u64) -> PathBuf {
    std::env::temp_dir().join(format!(
        "memoire-proprietes-{}-{seed}.db",
        std::process::id()
    ))
}

fn remove(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
}

#[test]
fn random_command_sequences_behave_the_same_through_sqlite() {
    let (accepted, _, _) = run(1..=24, false, false, false);
    assert!(
        accepted > 500,
        "trop peu de commandes acceptées : {accepted}"
    );
}

// Incrément 5.3 (LOT.md, section 7) : mêmes exigences avec des lots aléatoires.
#[test]
fn random_sequences_with_lots_behave_the_same_through_sqlite() {
    let (accepted, lots, _) = run(101..=124, true, false, false);
    assert!(
        accepted > 500,
        "trop peu de commandes acceptées : {accepted}"
    );
    assert!(lots > 100, "trop peu de lots acceptés : {lots}");
}

// Incrément 5.4a (FUSION.md, section 9.7) : mêmes exigences avec des abandons et des
// réactivations ; l'état courant d'une branche abandonnée est retiré puis reconstruit, ce que la
// vérification complète de fin de suite confirme.
#[test]
fn random_sequences_with_abandons_behave_the_same_through_sqlite() {
    let (accepted, _, abandons) = run(201..=224, true, true, false);
    assert!(
        accepted > 500,
        "trop peu de commandes acceptées : {accepted}"
    );
    assert!(
        abandons > 20,
        "trop peu d'abandons et de réactivations acceptés : {abandons}"
    );
}

// Incrément 5.4c (FUSION.md, section 9.11) : mêmes exigences avec des fusions résolues au hasard ;
// les commits à deux parents sont écrits, relus et vérifiés comme en mémoire.
#[test]
fn random_sequences_with_merges_behave_the_same_through_sqlite() {
    let (accepted, _, moves) = run(301..=324, true, true, true);
    assert!(
        accepted > 500,
        "trop peu de commandes acceptées : {accepted}"
    );
    assert!(
        moves > 40,
        "trop peu de fusions, abandons et réactivations acceptés : {moves}"
    );
}

/// Suites aléatoires appliquées à SQLite et à l'oracle en mémoire ; rend le nombre de commandes
/// acceptées et, parmi elles, de lots, puis d'abandons, de réactivations et de fusions.
fn run(
    seeds: std::ops::RangeInclusive<u64>,
    lots: bool,
    abandons: bool,
    merges: bool,
) -> (usize, usize, usize) {
    let mut accepted = 0;
    let mut accepted_lots = 0;
    let mut accepted_abandons = 0;
    for seed in seeds {
        let file = path(seed);
        remove(&file);
        let mut store = Store::create(&file).unwrap();
        let mut oracle = Repository::new();
        let mut world = World::new(seed);
        for step in 0..80 {
            let command =
                world.command(lots.then_some(&oracle), abandons, merges.then_some(&oracle));
            let reason = if world.random.below(15) == 0 {
                ""
            } else {
                "motif"
            };
            let next = Envelope {
                op: OpId::from_bytes(world.fresh()),
                expected: oracle.expected(),
                reason: reason.into(),
                command: command.clone(),
            };
            let context = format!("graine {seed}, pas {step}, {command:?}");
            let stored = store.execute(next.clone());
            let expected = oracle.execute(next.clone());
            match (&stored, &expected) {
                (Ok(a), Ok(b)) => assert_eq!(a, b, "{context}"),
                (Err(StoreError::Domain(a)), Err(b)) => assert_eq!(a, b, "{context}"),
                _ => panic!("{context} : {stored:?} au lieu de {expected:?}"),
            }
            assert_eq!(
                store.repository().snapshot(),
                oracle.snapshot(),
                "{context}"
            );
            assert_eq!(
                store.repository().expected(),
                oracle.expected(),
                "{context}"
            );
            assert_eq!(
                store.text().unwrap(),
                encode_repository_v2(&oracle),
                "{context}"
            );
            // Répétition de l'enveloppe : même réponse des deux côtés.
            match (store.execute(next.clone()), oracle.execute(next)) {
                (Ok(a), Ok(b)) => assert_eq!(a, b, "{context} : répétition"),
                (Err(StoreError::Domain(a)), Err(b)) => assert_eq!(a, b, "{context}"),
                (a, b) => panic!("{context} : répétition {a:?} au lieu de {b:?}"),
            }
            if world.random.below(8) == 0 {
                drop(store);
                store = Store::open(&file).unwrap();
            }
            if let Ok(outcome) = expected {
                world.learn(&command, &outcome);
                accepted += 1;
                accepted_lots += usize::from(matches!(command, Command::Lot(_)));
                accepted_abandons += usize::from(matches!(
                    command,
                    Command::Abandon { .. } | Command::Reactivate { .. } | Command::Merge { .. }
                ));
            }
        }
        let verified = store.verify().unwrap();
        assert_eq!(verified.commits, oracle.commit_count(), "graine {seed}");
        drop(store);
        remove(&file);
    }
    (accepted, accepted_lots, accepted_abandons)
}
