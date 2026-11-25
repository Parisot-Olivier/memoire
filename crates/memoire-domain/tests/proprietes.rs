//! Tests de propriétés (incrément 2.7) : suites de commandes engendrées au hasard, graines fixes,
//! sans dépendance externe. Après chaque commande, les lois du noyau sont vérifiées :
//! - INV-TX-01 : une commande refusée laisse le dépôt strictement identique ;
//! - INV-PREVIEW-01 : l'aperçu annonce exactement la publication, la bifurcation et les effets ;
//! - INV-RET-01 : une publication conserve toutes les identités de l'état précédent ;
//! - INV-UNDO-01 : annuler une publication rend l'état précédent, rétablir rend l'état suivant ;
//! - INV-BR-02 : seule une commande engagée sur main peut bifurquer ;
//! - tout état publié est valide, et aucune note n'est sa propre cause de réexamen ;
//! - INV-ID-04 : rejouer les mêmes enveloppes sur un dépôt neuf redonne le même dépôt ;
//! - FORMAT.md : chaque état publié s'encode et se relit à l'identique (incrément 3.1), et
//!   chaque dépôt final aussi, historique et journal compris (incrément 3.1b) ; de même en
//!   version 2 (incrément 5.1).
//! - INV-BR-05 (incrément 5.4a) : un abandon ou une réactivation ne publie rien et ne déplace
//!   aucune tête ; la branche active n'est jamais abandonnée.
//! - INV-RESTORE-02 (incrément 6.4a) : une restauration ciblée ne change rien hors de la sélection.
//! - INV-ORPH-01 et INV-CIT-01 (incrément 5.5a) : une orpheline n'est ni à réexaminer ni
//!   ressource actuelle ; une citation de note acceptée désigne sa source et sa version.
//! - INV-MERGE-01 à 03 (incrément 5.4b) : une fusion a deux parents, ne bifurque pas, laisse la
//!   source intacte, publie un état valide, et redonne la source dans une cible restée à la base.
//!
//! Ces tests complètent le contrat ; ils ne comptent pas dans le test de mutation, trop long.

use std::collections::BTreeSet;

use memoire_domain::*;

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

    /// Commande suivante ; avec `lots`, une sur quarante environ est un lot de 2 à 5 éléments
    /// (incrément 5.3). Avec `abandons`, une sur huit environ abandonne ou réactive une branche
    /// connue (incrément 5.4a). Avec `merges`, une sur dix environ fusionne une branche connue
    /// dans la branche active, chaque conflit résolu d'un côté tiré au hasard et, s'il reste un
    /// conflit structurel, une relation candidate désactivée (incrément 5.4b). Sans aucun des
    /// trois, la suite tirée est exactement celle d'avant 5.3.
    fn command(
        &mut self,
        lots: Option<&Repository>,
        abandons: bool,
        merges: Option<&Repository>,
        citations: bool,
        restores: bool,
        resumes: bool,
    ) -> Command {
        // Incrément 6.5a : une commande sur dix environ note un point de reprise, parfois sans
        // objectif ou avec une note inconnue (refusé), parfois avec un état du code.
        if resumes && self.random.below(10) == 0 {
            let mut notes = BTreeSet::new();
            for _ in 0..self.random.below(3) {
                notes.insert(self.node());
            }
            let n = self.counter;
            let code = (self.random.below(2) == 0).then(|| {
                CodeState::new(
                    Some(format!("c{}", self.random.below(3))),
                    None,
                    vec![format!(" M f{}.c", self.random.below(2))],
                    format!("patch {n}").as_bytes(),
                )
            });
            return Command::Resume(Box::new(ResumePoint {
                date: format!("pas {n}"),
                objective: if self.random.below(12) == 0 {
                    " ".into()
                } else {
                    format!("objectif {n}")
                },
                next_action: "action".into(),
                notes,
                code,
                ..ResumePoint::default()
            }));
        }
        // Incrément 6.4a : une commande sur dix environ restaure une à trois notes connues depuis
        // un commit connu.
        if restores && self.random.below(10) == 0 {
            let mut nodes = std::collections::BTreeSet::new();
            for _ in 0..=self.random.below(3) {
                nodes.insert(self.node());
            }
            return Command::Change(Change::RestoreNodes {
                target: self.random.pick(&self.known.commits).unwrap(),
                nodes,
            });
        }
        // Incrément 5.5a : une commande sur huit environ cite une note connue, à une version et
        // une plage tirées au hasard (souvent refusées, pour exercer les refus).
        if citations && self.random.below(8) == 0 {
            let note = self.node();
            return Command::Change(Change::CiteNote {
                scope: self.scope(),
                id: NodeId::from_bytes(self.fresh()),
                note,
                // Surtout la version de création et des plages valides dans « texte n » ; parfois
                // une version ou une plage hors de la note.
                version: if self.random.below(4) == 0 {
                    1 + self.random.below(2)
                } else {
                    0
                },
                start: self.random.below(4),
                end: if self.random.below(6) == 0 {
                    9
                } else {
                    4 + self.random.below(3)
                },
            });
        }
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

    /// Lot de 2 à 5 éléments ; le premier est le changement donné, ou un autre qui passe seul. Les éléments suivants sont gardés s'ils
    /// laissent le lot acceptable (aperçu sur `repo`, sans rien modifier), sauf un sur dix,
    /// gardé sans contrôle pour exercer le refus d'un élément au-delà du premier. Les identités
    /// créées par un élément sont aussitôt connues, pour que les suivants puissent les viser.
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
            | Change::CreateReserve { id, .. }
            | Change::CiteNote { id, .. } => self.known.nodes.push(*id),
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

/// Identités observables d'un état, sur les contextes et notes connus.
fn identities(s: &Snapshot, known: &Known) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for &scope in &known.scopes {
        if let Ok(g) = s.graph(scope) {
            set.insert(format!("contexte {scope}"));
            set.extend(g.nodes().map(|(id, _)| format!("{scope} note {id}")));
            set.extend(g.edges().map(|(id, _)| format!("{scope} relation {id}")));
        }
    }
    for &node in &known.nodes {
        if s.revisions(node).is_ok() {
            set.insert(format!("contenu {node}"));
        }
    }
    for &anchor in &known.anchors {
        if s.anchor(anchor).is_ok() {
            set.insert(format!("ancrage {anchor}"));
        }
    }
    set
}

/// Rien hors de la sélection ne change : contenus, membres, relations sans extrémité choisie,
/// acquittements d'un dépendant non choisi, ancrages d'une note non choisie.
fn outside_unchanged(
    before: &Snapshot,
    after: &Snapshot,
    nodes: &std::collections::BTreeSet<NodeId>,
    context: &str,
) {
    let encoded = |s: &Snapshot| -> Vec<String> {
        state_lines(s)
            .into_values()
            .filter(|line| !nodes.iter().any(|n| line.contains(&n.to_string())))
            .collect()
    };
    assert_eq!(
        encoded(before),
        encoded(after),
        "{context} : hors sélection modifié"
    );
}

fn check_state(s: &Snapshot, known: &Known, context: &str) {
    assert_eq!(s.validate(), Ok(()), "{context}");
    for &scope in &known.scopes {
        if let Ok(g) = s.graph(scope) {
            // INV-ORPH-01 : une orpheline n'est ni écartée, ni à réexaminer, ni ressource actuelle.
            let causes = g.review_causes();
            let current = g.current_resources();
            for orphan in g.orphans() {
                assert!(!current.contains(&orphan), "{context} : orpheline courante");
                assert!(
                    !causes.contains_key(&orphan),
                    "{context} : orpheline à réexaminer"
                );
            }
            for (node, causes) in g.review_causes() {
                assert!(
                    !causes.contains(&node),
                    "{context} : {node} cause d'elle-même"
                );
            }
        }
    }
}

// Une famille de commandes par drapeau, comme depuis 5.3 ; le huitième (6.5a) dépasse le seuil de
// clippy sans rendre les appels moins lisibles.
#[allow(clippy::too_many_arguments)]
fn run(
    seed: u64,
    steps: usize,
    lots: bool,
    abandons: bool,
    merges: bool,
    citations: bool,
    restores: bool,
    resumes: bool,
) -> (Repository, Vec<Envelope>) {
    let mut world = World::new(seed);
    let mut repo = Repository::new();
    let mut accepted = Vec::new();
    for step in 0..steps {
        let command = world.command(
            lots.then_some(&repo),
            abandons,
            merges.then_some(&repo),
            citations,
            restores,
            resumes,
        );
        let context = format!("graine {seed}, pas {step}, {command:?}");
        let before = repo.clone();
        let merge_plan = match &command {
            Command::Merge {
                source,
                resolutions,
                ..
            } => repo.merge_plan(*source, resolutions).ok(),
            _ => None,
        };
        let preview = repo.preview(&command);
        let reason = if world.random.below(15) == 0 {
            ""
        } else {
            "motif"
        };
        let envelope = Envelope {
            op: OpId::from_bytes(world.fresh()),
            expected: repo.expected(),
            reason: reason.into(),
            command: command.clone(),
        };
        match repo.execute(envelope.clone()) {
            Err(_) => assert_eq!(repo, before, "{context} : refus avec effet"),
            Ok(outcome) => {
                let preview = preview.unwrap_or_else(|e| panic!("{context} : aperçu {e:?}"));
                assert_eq!(
                    (preview.publishes, preview.forks, &preview.effects),
                    (outcome.published, outcome.forked, &outcome.effects),
                    "{context} : aperçu différent de l'exécution"
                );
                if outcome.forked {
                    assert_eq!(before.active_branch(), MAIN, "{context}");
                }
                // INV-LOT-01 : un lot accepté donne l'état de ses éléments exécutés un à un, en
                // une seule opération et au plus un commit.
                if let Command::Lot(elements) = &command {
                    let mut one_by_one = before.clone();
                    for (k, change) in elements.iter().enumerate() {
                        let single = Envelope {
                            op: OpId::from_bytes([
                                0xfd,
                                u8::try_from(k).unwrap(),
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                0,
                                1,
                            ]),
                            expected: one_by_one.expected(),
                            reason: "motif".into(),
                            command: Command::Change(change.clone()),
                        };
                        one_by_one
                            .execute(single)
                            .unwrap_or_else(|e| panic!("{context} : élément {k} refusé seul, {e}"));
                    }
                    assert_eq!(
                        one_by_one.snapshot(),
                        repo.snapshot(),
                        "{context} : lot différent de ses éléments"
                    );
                    assert!(
                        repo.commit_count() <= before.commit_count() + 1,
                        "{context} : plus d'un commit"
                    );
                }
                // INV-BR-05 : un abandon ou une réactivation ne publie rien, ne change aucune
                // tête ; la branche abandonnée n'est plus active ; la réactivée ne le devient pas.
                if let Command::Abandon { branch } | Command::Reactivate { branch } = &command {
                    let abandons = matches!(command, Command::Abandon { .. });
                    assert_eq!(repo.is_abandoned(*branch), Ok(abandons), "{context}");
                    assert_eq!(repo.commit_count(), before.commit_count(), "{context}");
                    for b in before.branches() {
                        assert_eq!(repo.branch_head(b), before.branch_head(b), "{context}");
                    }
                    if abandons {
                        assert_ne!(repo.active_branch(), *branch, "{context}");
                    } else {
                        assert_eq!(repo.active_branch(), before.active_branch(), "{context}");
                    }
                }
                assert_eq!(
                    repo.is_abandoned(repo.active_branch()),
                    Ok(false),
                    "{context} : branche active abandonnée"
                );
                // INV-MERGE-01 à 03 : une fusion publiée a deux parents, la cible d'abord ; elle
                // ne bifurque pas, ne modifie pas la source ; dans une cible restée à la base, elle
                // redonne exactement l'état de la source. L'état réuni est vérifié plus bas.
                if let Command::Merge {
                    source,
                    source_head,
                    ..
                } = &command
                {
                    let plan = merge_plan.unwrap_or_else(|| panic!("{context} : plan refusé"));
                    assert_eq!(repo.branch_head(*source), Ok(*source_head), "{context}");
                    assert!(!outcome.forked, "{context}");
                    assert_eq!(outcome.published, !plan.up_to_date, "{context}");
                    assert_eq!(outcome.effects, plan.effects, "{context}");
                    if outcome.published {
                        assert_eq!(
                            repo.history().commit(repo.head()).unwrap().unwrap().parents,
                            vec![before.head(), *source_head],
                            "{context}"
                        );
                        if plan.base == before.head() {
                            assert_eq!(
                                repo.snapshot(),
                                &*before.snapshot_at(*source_head).unwrap(),
                                "{context} : avance de la cible différente de la source"
                            );
                        }
                    }
                }
                // INV-RESTORE-02 : une restauration ciblée ne touche rien hors de la sélection.
                if let Command::Change(Change::RestoreNodes { nodes, .. }) = &command {
                    outside_unchanged(before.snapshot(), repo.snapshot(), nodes, &context);
                }
                // INV-RESUME-01 : une fiche acceptée publie un commit sans changer le réseau, ne
                // bifurque pas, reste sur la branche active et se relit dans le journal.
                if let Command::Resume(point) = &command {
                    assert!(outcome.published && !outcome.forked, "{context}");
                    assert_eq!(repo.snapshot(), before.snapshot(), "{context}");
                    assert_eq!(repo.active_branch(), before.active_branch(), "{context}");
                    assert_eq!(
                        repo.resume_point(repo.head()).as_ref(),
                        Ok(&**point),
                        "{context}"
                    );
                    assert_eq!(repo.operations_since(repo.head()), Ok(Some(0)), "{context}");
                }
                // INV-CIT-01 : une citation de note acceptée désigne sa source et sa version.
                if let Command::Change(Change::CiteNote {
                    id, note, version, ..
                }) = command
                {
                    assert_eq!(
                        repo.snapshot().citation(id).map(|v| v.source),
                        Ok(CitedSource::Note { note, version }),
                        "{context}"
                    );
                }
                if outcome.published {
                    let (old, new) = (before.snapshot(), repo.snapshot());
                    assert!(
                        identities(old, &world.known).is_subset(&identities(new, &world.known)),
                        "{context} : identité perdue"
                    );
                    check_state(new, &world.known, &context);
                    // Format textuel (FORMAT.md) : aller-retour exact et forme canonique unique.
                    // La version 1 ne représente pas la citation d'une note (CITATIONS.md,
                    // section 3) : la famille qui en crée ne relit que la version 2.
                    if !citations {
                        let text = encode_snapshot(new);
                        let decoded = decode_snapshot(&text)
                            .unwrap_or_else(|e| panic!("{context} : relecture refusée, {e}"));
                        assert_eq!(&decoded, new, "{context} : relecture infidèle");
                        assert_eq!(
                            encode_snapshot(&decoded),
                            text,
                            "{context} : forme non canonique"
                        );
                    }
                    // Version 2 (VERSION2.md, section 5) : même exigence.
                    let text = encode_snapshot_v2(new);
                    let decoded = decode_snapshot_v2(&text)
                        .unwrap_or_else(|e| panic!("{context} : relecture v2 refusée, {e}"));
                    assert_eq!(&decoded, new, "{context} : relecture v2 infidèle");
                    assert_eq!(
                        encode_snapshot_v2(&decoded),
                        text,
                        "{context} : forme v2 non canonique"
                    );
                    let mut probe = repo.clone();
                    let undo = Envelope {
                        op: OpId::from_bytes([0xff; 16]),
                        expected: probe.expected(),
                        reason: String::new(),
                        command: Command::Undo,
                    };
                    probe.execute(undo).unwrap();
                    assert_eq!(probe.snapshot(), old, "{context} : annulation inexacte");
                    let redo = Envelope {
                        op: OpId::from_bytes([0xfe; 16]),
                        expected: probe.expected(),
                        reason: String::new(),
                        command: Command::Redo,
                    };
                    probe.execute(redo).unwrap();
                    assert_eq!(probe.snapshot(), new, "{context} : rétablissement inexact");
                }
                world.learn(&command, &outcome);
                accepted.push(envelope);
            }
        }
    }
    (repo, accepted)
}

#[test]
fn random_command_sequences_respect_the_kernel_laws() {
    let mut published = 0;
    for seed in 1..=48 {
        let (repo, accepted) = run(seed, 80, false, false, false, false, false, false);
        published += accepted.len();
        // INV-ID-04 : mêmes enveloppes, même dépôt.
        let mut replay = Repository::new();
        for envelope in accepted {
            replay.execute(envelope).unwrap();
        }
        assert_eq!(replay, repo, "graine {seed} : rejeu différent");
        // FORMAT.md, section 12 : le dépôt complet s'exporte et se relit à l'identique.
        let text = encode_repository(&repo);
        let decoded = decode_repository(&text)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt relu infidèle");
        assert_eq!(
            encode_repository(&decoded),
            text,
            "graine {seed} : forme non canonique"
        );
        // VERSION2.md, sections 6 et 7 : relecture en v2, et la migration de la v1 donne la v2.
        let v2 = encode_repository_v2(&repo);
        let decoded = decode_repository_v2(&v2)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt v2 relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt v2 relu infidèle");
        assert_eq!(
            migrate_v1_to_v2(&text),
            Ok(v2),
            "graine {seed} : migration différente de l'encodage v2"
        );
    }
    // Garde-fou : les suites doivent réellement exercer le noyau.
    assert!(
        published > 1_000,
        "trop peu de commandes acceptées : {published}"
    );
}

// Incrément 5.3 (LOT.md, section 7) : mêmes lois avec des lots aléatoires ; le dépôt se relit en
// version 2, où les lots sont représentables (la version 1 ne les connaît pas).
#[test]
fn random_sequences_with_lots_respect_the_kernel_laws() {
    let mut lots = 0;
    for seed in 101..=148 {
        let (repo, accepted) = run(seed, 80, true, false, false, false, false, false);
        lots += accepted
            .iter()
            .filter(|e| matches!(e.command, Command::Lot(_)))
            .count();
        let mut replay = Repository::new();
        for envelope in accepted {
            replay.execute(envelope).unwrap();
        }
        assert_eq!(replay, repo, "graine {seed} : rejeu différent");
        let v2 = encode_repository_v2(&repo);
        let decoded = decode_repository_v2(&v2)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt v2 relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt v2 relu infidèle");
        assert_eq!(encode_repository_v2(&decoded), v2, "graine {seed}");
    }
    // Garde-fou : des lots doivent réellement être acceptés.
    assert!(lots > 250, "trop peu de lots acceptés : {lots}");
}

// Incrément 5.4a (FUSION.md, section 9.7) : mêmes lois avec des abandons et des réactivations,
// lots compris ; le dépôt se relit en version 2, seule à représenter l'abandon.
#[test]
fn random_sequences_with_abandons_respect_the_kernel_laws() {
    let (mut abandons, mut reactivations) = (0, 0);
    for seed in 201..=248 {
        let (repo, accepted) = run(seed, 80, true, true, false, false, false, false);
        for envelope in &accepted {
            match envelope.command {
                Command::Abandon { .. } => abandons += 1,
                Command::Reactivate { .. } => reactivations += 1,
                _ => {}
            }
        }
        let mut replay = Repository::new();
        for envelope in accepted {
            replay.execute(envelope).unwrap();
        }
        assert_eq!(replay, repo, "graine {seed} : rejeu différent");
        let v2 = encode_repository_v2(&repo);
        let decoded = decode_repository_v2(&v2)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt v2 relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt v2 relu infidèle");
        assert_eq!(encode_repository_v2(&decoded), v2, "graine {seed}");
        assert_eq!(
            verify_repository_v2(&v2).map(|v| v.commits),
            Ok(repo.commit_count()),
            "graine {seed}"
        );
    }
    // Garde-fou : abandons et réactivations doivent réellement être acceptés.
    assert!(abandons > 40, "trop peu d'abandons acceptés : {abandons}");
    assert!(
        reactivations > 20,
        "trop peu de réactivations acceptées : {reactivations}"
    );
}

// Incrément 5.4b (FUSION.md, section 9.11) : mêmes lois avec des fusions résolues au hasard,
// abandons et lots compris ; le dépôt se relit en version 2, seule à représenter la fusion.
#[test]
fn random_sequences_with_merges_respect_the_kernel_laws() {
    let (mut merges, mut resolved) = (0, 0);
    for seed in 301..=348 {
        // Suites plus longues : les conflits n'y naissent qu'au hasard de deux branches qui
        // changent le même champ.
        let (repo, accepted) = run(seed, 200, true, true, true, false, false, false);
        for envelope in &accepted {
            if let Command::Merge { resolutions, .. } = &envelope.command {
                merges += 1;
                resolved += usize::from(!resolutions.is_empty());
            }
        }
        let mut replay = Repository::new();
        for envelope in accepted {
            replay.execute(envelope).unwrap();
        }
        assert_eq!(replay, repo, "graine {seed} : rejeu différent");
        let v2 = encode_repository_v2(&repo);
        let decoded = decode_repository_v2(&v2)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt v2 relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt v2 relu infidèle");
        assert_eq!(encode_repository_v2(&decoded), v2, "graine {seed}");
        assert_eq!(
            verify_repository_v2(&v2).map(|v| v.commits),
            Ok(repo.commit_count()),
            "graine {seed}"
        );
    }
    // Garde-fou : des fusions, dont certaines résolues, doivent réellement être acceptées
    // (mesuré : 296 fusions, dont 7 résolues). Chaque forme de résolution est par ailleurs
    // couverte par le contrat.
    assert!(merges > 150, "trop peu de fusions acceptées : {merges}");
    assert!(resolved > 3, "trop peu de fusions résolues : {resolved}");
}

// Incrément 5.5a (CITATIONS.md, section 8) : mêmes lois avec des citations de notes à des
// versions et plages tirées au hasard, en plus des relations vers les échanges désormais permises ;
// lots, abandons et fusions compris. Relecture en version 2.
#[test]
fn random_sequences_with_citations_respect_the_kernel_laws() {
    let mut citations = 0;
    for seed in 401..=448 {
        let (repo, accepted) = run(seed, 120, true, true, true, true, false, false);
        citations += accepted
            .iter()
            .filter(|e| matches!(e.command, Command::Change(Change::CiteNote { .. })))
            .count();
        let mut replay = Repository::new();
        for envelope in accepted {
            replay.execute(envelope).unwrap();
        }
        assert_eq!(replay, repo, "graine {seed} : rejeu différent");
        let v2 = encode_repository_v2(&repo);
        let decoded = decode_repository_v2(&v2)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt v2 relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt v2 relu infidèle");
        assert_eq!(
            verify_repository_v2(&v2).map(|v| v.commits),
            Ok(repo.commit_count()),
            "graine {seed}"
        );
    }
    assert!(
        citations > 30,
        "trop peu de citations acceptées : {citations}"
    );
}

// Incrément 6.4a (RESTAURATION.md, section 6) : mêmes lois avec des restaurations ciblées
// aléatoires ; rien hors de la sélection ne change.
#[test]
fn random_sequences_with_targeted_restores_respect_the_kernel_laws() {
    let mut restores = 0;
    for seed in 501..=548 {
        let (repo, accepted) = run(seed, 120, true, true, false, true, true, false);
        restores += accepted
            .iter()
            .filter(|e| matches!(e.command, Command::Change(Change::RestoreNodes { .. })))
            .count();
        let mut replay = Repository::new();
        for envelope in accepted {
            replay.execute(envelope).unwrap();
        }
        assert_eq!(replay, repo, "graine {seed} : rejeu différent");
        let v2 = encode_repository_v2(&repo);
        let decoded = decode_repository_v2(&v2)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt v2 relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt v2 relu infidèle");
    }
    assert!(
        restores > 60,
        "trop peu de restaurations ciblées acceptées : {restores}"
    );
}

// Incrément 6.5a (FICHE-REPRISE.md, section 10) : mêmes lois avec des points de reprise ; une
// fiche ne change pas le réseau, s'annule et se rétablit exactement, et se relit en version 2.
#[test]
fn random_sequences_with_resume_points_respect_the_kernel_laws() {
    let mut resumes = 0;
    for seed in 601..=648 {
        let (repo, accepted) = run(seed, 120, true, true, true, true, true, true);
        resumes += accepted
            .iter()
            .filter(|e| matches!(e.command, Command::Resume(_)))
            .count();
        let mut replay = Repository::new();
        for envelope in accepted {
            replay.execute(envelope).unwrap();
        }
        assert_eq!(replay, repo, "graine {seed} : rejeu différent");
        let v2 = encode_repository_v2(&repo);
        let decoded = decode_repository_v2(&v2)
            .unwrap_or_else(|e| panic!("graine {seed} : dépôt v2 relu refusé, {e}"));
        assert_eq!(decoded, repo, "graine {seed} : dépôt v2 relu infidèle");
    }
    assert!(resumes > 60, "trop peu de fiches acceptées : {resumes}");
}
