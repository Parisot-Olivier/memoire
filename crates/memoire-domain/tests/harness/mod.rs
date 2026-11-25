//! Harnais de scénarios : traduit les gestes du contrat en enveloppes de commandes.
//!
//! Il fournit les identifiants (D3) à partir d'une graine et d'un compteur, prend l'état courant
//! comme état attendu, et ne consomme aucun identifiant quand une commande est refusée.
//! Il ne contient aucune règle métier : toute décision appartient au domaine.
#![allow(dead_code)] // chaque fichier de tests n'emploie qu'une partie du harnais

use memoire_domain::{
    AnchorId, BranchId, Change, Command, CommitId, Decision, EdgeId, Envelope, Error,
    Implementation, Kind, NodeId, OpId, Outcome, Part, Relation, Repository, ReserveStatus,
    ResumePoint, RevisionKind, ScopeId, Snapshot, Verdict,
};

/// Motif fourni à chaque commande : le contrat porté ne précise pas de motif, et un changement
/// de statut en exige un (INV-MOD-05). L'exigence elle-même est testée sans le harnais.
pub const REASON: &str = "motif du harnais";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Harness {
    repo: Repository,
    seed: u64,
    next: u64,
    last: Option<Outcome>,
}

impl Harness {
    pub fn new() -> Self {
        Self::with_seed(1)
    }

    /// Deux graines distinctes simulent deux dépôts indépendants.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            repo: Repository::new(),
            seed,
            next: 1,
            last: None,
        }
    }

    /// Identifiant réservé : graine sur 8 octets, compteur sur 8 octets ; jamais nul.
    fn raw(&self, offset: u64) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.seed.to_be_bytes());
        bytes[8..].copy_from_slice(&(self.next + offset).to_be_bytes());
        bytes
    }

    /// Exécute une commande dont `extra` identifiants ont été réservés après celui de l'opération.
    fn run(&mut self, command: Command, extra: u64) -> Result<Outcome, Error> {
        let envelope = Envelope {
            op: OpId::from_bytes(self.raw(0)),
            expected: self.repo.expected(),
            reason: REASON.into(),
            command,
        };
        let outcome = self.repo.execute(envelope)?;
        self.next += 1 + extra;
        self.last = Some(outcome.clone());
        Ok(outcome)
    }

    fn change(&mut self, change: Change, extra: u64) -> Result<Outcome, Error> {
        self.run(Command::Change(change), extra)
    }

    // ----- Gestes du contrat -----

    pub fn execute(&mut self, command: Command) -> Result<Outcome, Error> {
        self.run(command, 0)
    }

    pub fn create_node(
        &mut self,
        scope: ScopeId,
        kind: Kind,
        title: &str,
        text: &str,
    ) -> Result<NodeId, Error> {
        let id = NodeId::from_bytes(self.raw(1));
        let (title, text) = (title.into(), text.into());
        self.change(
            Change::CreateNode {
                scope,
                id,
                kind,
                title,
                text,
            },
            1,
        )?;
        Ok(id)
    }

    /// Geste du prototype : une édition sans précision est déclarée substantielle, choix
    /// conservateur fidèle au prototype. Le contenu étant partagé (D2), le contexte ne sert
    /// qu'à la lisibilité des tests portés.
    pub fn edit_text(&mut self, _scope: ScopeId, node: NodeId, text: &str) -> Result<(), Error> {
        self.revise(node, text, RevisionKind::Substantive)
    }

    pub fn revise(&mut self, node: NodeId, text: &str, kind: RevisionKind) -> Result<(), Error> {
        let text = text.into();
        self.change(Change::Revise { node, text, kind }, 0)
            .map(drop)
    }

    pub fn include(&mut self, scope: ScopeId, node: NodeId) -> Result<(), Error> {
        self.change(Change::Include { scope, node }, 0).map(drop)
    }

    pub fn clone_scope(&mut self, source: ScopeId) -> Result<ScopeId, Error> {
        let id = ScopeId::from_bytes(self.raw(1));
        self.change(Change::CloneScope { source, id }, 1)?;
        Ok(id)
    }

    pub fn link(
        &mut self,
        scope: ScopeId,
        from: NodeId,
        to: NodeId,
        relation: Relation,
    ) -> Result<EdgeId, Error> {
        let id = EdgeId::from_bytes(self.raw(1));
        self.change(
            Change::Link {
                scope,
                id,
                from,
                to,
                relation,
            },
            1,
        )?;
        Ok(id)
    }

    pub fn replace(
        &mut self,
        scope: ScopeId,
        replacement: NodeId,
        replaced: NodeId,
    ) -> Result<EdgeId, Error> {
        let id = EdgeId::from_bytes(self.raw(1));
        let change = Change::Replace {
            scope,
            id,
            replacement,
            replaced,
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn restrict(
        &mut self,
        scope: ScopeId,
        from: NodeId,
        to: NodeId,
        condition: &str,
    ) -> Result<EdgeId, Error> {
        let id = EdgeId::from_bytes(self.raw(1));
        let condition = condition.into();
        let change = Change::Restrict {
            scope,
            id,
            from,
            to,
            condition,
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn contradict(
        &mut self,
        scope: ScopeId,
        from: NodeId,
        to: NodeId,
        justification: &str,
    ) -> Result<EdgeId, Error> {
        let id = EdgeId::from_bytes(self.raw(1));
        let justification = justification.into();
        let change = Change::Contradict {
            scope,
            id,
            from,
            to,
            justification,
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn create_exchange(
        &mut self,
        scope: ScopeId,
        title: &str,
        question: Option<&str>,
        answer: &str,
    ) -> Result<NodeId, Error> {
        let id = NodeId::from_bytes(self.raw(1));
        let change = Change::CreateExchange {
            scope,
            id,
            title: title.into(),
            question: question.map(Into::into),
            answer: answer.into(),
            previous: None,
        };
        self.change(change, 1)?;
        Ok(id)
    }

    /// Échange saisi à la suite d'un autre dans la même conversation.
    pub fn continue_exchange(
        &mut self,
        scope: ScopeId,
        previous: NodeId,
        question: &str,
        answer: &str,
    ) -> Result<NodeId, Error> {
        let id = NodeId::from_bytes(self.raw(1));
        let change = Change::CreateExchange {
            scope,
            id,
            title: String::new(),
            question: Some(question.into()),
            answer: answer.into(),
            previous: Some(previous),
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn acknowledge(
        &mut self,
        scope: ScopeId,
        dependent: NodeId,
        cause: NodeId,
    ) -> Result<(), Error> {
        let change = Change::Acknowledge {
            scope,
            dependent,
            cause,
        };
        self.change(change, 0).map(drop)
    }

    pub fn anchor(
        &mut self,
        node: NodeId,
        path: &str,
        excerpt: &str,
        git: Option<&str>,
    ) -> Result<AnchorId, Error> {
        let id = AnchorId::from_bytes(self.raw(1));
        let change = Change::AnchorToCode {
            id,
            node,
            path: path.into(),
            excerpt: excerpt.into(),
            git: git.map(Into::into),
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn unanchor(&mut self, anchor: AnchorId) -> Result<(), Error> {
        self.change(Change::Unanchor { anchor }, 0).map(drop)
    }

    pub fn detach(&mut self, scope: ScopeId, edge: EdgeId) -> Result<(), Error> {
        self.change(Change::Detach { scope, edge }, 0).map(drop)
    }

    pub fn set_question(&mut self, exchange: NodeId, question: &str) -> Result<(), Error> {
        let question = question.into();
        self.change(Change::SetQuestion { exchange, question }, 0)
            .map(drop)
    }

    pub fn create_passage(
        &mut self,
        scope: ScopeId,
        exchange: NodeId,
        part: Part,
        start: usize,
        end: usize,
    ) -> Result<NodeId, Error> {
        let id = NodeId::from_bytes(self.raw(1));
        let change = Change::CreatePassage {
            scope,
            id,
            exchange,
            part,
            start,
            end,
        };
        self.change(change, 1)?;
        Ok(id)
    }

    /// Citation d'une note à une version (incrément 5.5a, D18).
    pub fn cite_note(
        &mut self,
        scope: ScopeId,
        note: NodeId,
        version: usize,
        start: usize,
        end: usize,
    ) -> Result<NodeId, Error> {
        let id = NodeId::from_bytes(self.raw(1));
        let change = Change::CiteNote {
            scope,
            id,
            note,
            version,
            start,
            end,
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn create_reserve(
        &mut self,
        scope: ScopeId,
        passage: NodeId,
        title: &str,
        target: Option<&str>,
    ) -> Result<NodeId, Error> {
        let id = NodeId::from_bytes(self.raw(1));
        let change = Change::CreateReserve {
            scope,
            id,
            passage,
            title: title.into(),
            target: target.map(Into::into),
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn treat(
        &mut self,
        scope: ScopeId,
        note: NodeId,
        reserve: NodeId,
    ) -> Result<EdgeId, Error> {
        let id = EdgeId::from_bytes(self.raw(1));
        let change = Change::Treat {
            scope,
            id,
            note,
            reserve,
        };
        self.change(change, 1)?;
        Ok(id)
    }

    pub fn set_reserve_status(
        &mut self,
        scope: ScopeId,
        reserve: NodeId,
        status: ReserveStatus,
    ) -> Result<(), Error> {
        let change = Change::SetReserveStatus {
            scope,
            reserve,
            status,
        };
        self.change(change, 0).map(drop)
    }

    /// Geste du prototype : réfuter.
    pub fn invalidate(&mut self, scope: ScopeId, node: NodeId) -> Result<BranchId, Error> {
        self.set_verdict(scope, node, Verdict::Refuted)
    }

    pub fn set_verdict(
        &mut self,
        scope: ScopeId,
        node: NodeId,
        verdict: Verdict,
    ) -> Result<BranchId, Error> {
        self.change(
            Change::SetVerdict {
                scope,
                node,
                verdict,
            },
            0,
        )
        .map(|o| o.branch)
    }

    pub fn set_decision(
        &mut self,
        scope: ScopeId,
        node: NodeId,
        decision: Decision,
    ) -> Result<BranchId, Error> {
        self.change(
            Change::SetDecision {
                scope,
                node,
                decision,
            },
            0,
        )
        .map(|o| o.branch)
    }

    pub fn set_implementation(
        &mut self,
        scope: ScopeId,
        node: NodeId,
        implementation: Implementation,
    ) -> Result<BranchId, Error> {
        let change = Change::SetImplementation {
            scope,
            node,
            implementation,
        };
        self.change(change, 0).map(|o| o.branch)
    }

    pub fn archive(&mut self, scope: ScopeId, node: NodeId) -> Result<BranchId, Error> {
        self.change(Change::Archive { scope, node }, 0)
            .map(|o| o.branch)
    }

    pub fn reattach(&mut self, scope: ScopeId, edge: EdgeId, to: NodeId) -> Result<EdgeId, Error> {
        let id = EdgeId::from_bytes(self.raw(1));
        self.change(
            Change::Reattach {
                scope,
                edge,
                to,
                id,
            },
            1,
        )?;
        Ok(id)
    }

    pub fn restore(&mut self, target: CommitId) -> Result<BranchId, Error> {
        self.change(Change::Restore { target }, 0).map(|o| o.branch)
    }

    /// Restauration ciblée d'une sélection de notes (incrément 6.4).
    pub fn restore_nodes(&mut self, target: CommitId, nodes: &[NodeId]) -> Result<(), Error> {
        let nodes = nodes.iter().copied().collect();
        self.change(Change::RestoreNodes { target, nodes }, 0)
            .map(drop)
    }

    /// Point de reprise (incrément 6.5) : un commit sans changement du réseau.
    pub fn resume(&mut self, point: ResumePoint) -> Result<Outcome, Error> {
        self.run(Command::Resume(Box::new(point)), 0)
    }

    pub fn undo(&mut self) -> Result<(), Error> {
        self.run(Command::Undo, 0).map(drop)
    }

    pub fn redo(&mut self) -> Result<(), Error> {
        self.run(Command::Redo, 0).map(drop)
    }

    pub fn checkout(&mut self, branch: BranchId) -> Result<(), Error> {
        self.run(Command::Checkout { branch }, 0).map(drop)
    }

    // ----- Lecture -----

    pub fn repository(&self) -> &Repository {
        &self.repo
    }

    pub fn last_outcome(&self) -> Option<&Outcome> {
        self.last.as_ref()
    }

    pub fn marker(&self, scope: ScopeId, id: NodeId) -> Result<String, Error> {
        self.repo.marker(scope, id)
    }

    pub fn snapshot(&self) -> &Snapshot {
        self.repo.snapshot()
    }

    pub fn snapshot_at(&self, commit: CommitId) -> Result<std::sync::Arc<Snapshot>, Error> {
        self.repo.snapshot_at(commit)
    }

    pub fn head(&self) -> CommitId {
        self.repo.head()
    }

    pub fn active_branch(&self) -> BranchId {
        self.repo.active_branch()
    }

    pub fn branch_head(&self, branch: BranchId) -> Result<CommitId, Error> {
        self.repo.branch_head(branch)
    }

    pub fn branch_count(&self) -> usize {
        self.repo.branch_count()
    }

    pub fn commit_count(&self) -> usize {
        self.repo.commit_count()
    }
}
