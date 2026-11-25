//! Dépôt : branches, état de la tête active, et un historique derrière un trait (D20).
//!
//! Chemin unique de toute commande : `plan` (lecture seule, faillible), puis les écritures
//! (`Writes`), appliquées tout ou rien par l'historique, puis la mise à jour de la partie en
//! mémoire, infaillible. [`Repository::preview`], [`Repository::prepare`] et
//! [`Repository::execute`] partagent `plan`.
//!
//! Branches (D1) : seule main déclenche une bifurcation ; une branche expérimentale garde son
//! origine et son commit de base. L'annulation ne franchit pas la base : annuler l'opération
//! fondatrice ramène sur l'origine, et le rétablissement y rentre de nouveau.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::command::{Change, Command, Envelope, Expected, UsedId};
use crate::error::Error;
use crate::fusion::{merge, MergePlan, Resolution};
use crate::history::{CommitInfo, History, MemoryHistory, NewCommit, Prepared, Writes};
use crate::id::{AnchorId, BranchId, CommitId, EdgeId, NodeId, OpId, ScopeId, MAIN};
use crate::model::{
    text_digest, Ack, Anchor, Body, Content, Decision, Edge, Kind, Node, Prior, Relation,
    ReserveStatus, Snapshot, Text, Visibility,
};
use crate::restore::{self, HistoryEntry, RestorePlan};
use crate::resume::{ResumeEntry, ResumePoint};

/// Préfixe du repère à copier dans le code ; il est suivi de la seule identité de la note.
pub const MARKER_PREFIX: &str = "// @design: ";

/// Identité désignée par un repère, si la ligne est exactement un repère canonique.
#[must_use]
pub fn parse_marker(line: &str) -> Option<NodeId> {
    line.strip_prefix(MARKER_PREFIX).and_then(NodeId::parse)
}

/// Variation des causes de réexamen d'une note dans un contexte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewChange {
    /// Causes avant la commande.
    pub before: BTreeSet<NodeId>,
    /// Causes après la commande.
    pub after: BTreeSet<NodeId>,
}

/// Effets calculés d'une commande ; identiques dans l'aperçu et dans le résultat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    /// Notes dont les causes de réexamen changent, par contexte.
    pub review: BTreeMap<(ScopeId, NodeId), ReviewChange>,
}

/// Aperçu d'une commande, calculé sans rien modifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    /// État sur lequel l'aperçu est calculé ; à reprendre dans l'enveloppe.
    pub base: Expected,
    /// Vrai si la commande publiera un commit.
    pub publishes: bool,
    /// Vrai si la commande créera une branche.
    pub forks: bool,
    /// Effets sur les causes de réexamen.
    pub effects: Effects,
}

/// Résultat d'une commande exécutée (INV-BR-04).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Opération exécutée.
    pub op: OpId,
    /// Branche active après la commande.
    pub branch: BranchId,
    /// Tête de la branche active après la commande.
    pub head: CommitId,
    /// Vrai si un commit a été publié.
    pub published: bool,
    /// Vrai si une branche a été créée.
    pub forked: bool,
    /// Effets sur les causes de réexamen, identiques à ceux de l'aperçu.
    pub effects: Effects,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Commit {
    pub(crate) state: Arc<Snapshot>,
    pub(crate) parent: Option<CommitId>,
    /// Second parent d'un commit de fusion : la tête de la source (FUSION.md, section 9.6).
    pub(crate) merged: Option<CommitId>,
    pub(crate) reason: String,
}

/// Métadonnées d'une branche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    /// Commit de tête.
    pub head: CommitId,
    /// Pile de rétablissement, du bas vers le haut.
    pub redo: Vec<Redo>,
    /// Branche quittée par l'annulation de l'opération fondatrice ; aucune pour main.
    pub origin: Option<BranchId>,
    /// Commit de l'origine sur lequel la branche a été créée ; jamais franchi par l'annulation.
    pub base: CommitId,
    /// Vrai si la branche est abandonnée (D19, D23) : elle garde tout, mais n'est ni active ni
    /// source d'une fusion tant qu'elle n'est pas réactivée. Jamais pour main.
    pub abandoned: bool,
}

/// Entrée de la pile de rétablissement d'une branche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redo {
    /// Commit annulé sur cette branche.
    Commit(CommitId),
    /// Branche quittée par l'annulation de son opération fondatrice, avec la tête qu'elle avait.
    Enter {
        /// Branche quittée.
        branch: BranchId,
        /// Sa tête au moment où elle a été quittée.
        head: CommitId,
    },
}

/// Identités déjà employées dans ce dépôt, y compris par des états abandonnés (INV-ID-02).
/// L'ensemble ne fait que croître.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Used {
    pub(crate) nodes: BTreeSet<NodeId>,
    pub(crate) edges: BTreeSet<EdgeId>,
    pub(crate) scopes: BTreeSet<ScopeId>,
    pub(crate) anchors: BTreeSet<AnchorId>,
}

impl Used {
    pub(crate) fn contains(&self, id: UsedId) -> bool {
        match id {
            UsedId::Node(id) => self.nodes.contains(&id),
            UsedId::Edge(id) => self.edges.contains(&id),
            UsedId::Scope(id) => self.scopes.contains(&id),
            UsedId::Anchor(id) => self.anchors.contains(&id),
        }
    }

    pub(crate) fn insert(&mut self, id: UsedId) {
        match id {
            UsedId::Node(id) => {
                self.nodes.insert(id);
            }
            UsedId::Edge(id) => {
                self.edges.insert(id);
            }
            UsedId::Scope(id) => {
                self.scopes.insert(id);
            }
            UsedId::Anchor(id) => {
                self.anchors.insert(id);
            }
        }
    }
}

/// Refus d'une identité déjà employée.
fn in_use(id: UsedId) -> Error {
    match id {
        UsedId::Node(id) => Error::NodeIdInUse(id),
        UsedId::Edge(id) => Error::EdgeIdInUse(id),
        UsedId::Scope(id) => Error::ScopeIdInUse(id),
        UsedId::Anchor(id) => Error::AnchorIdInUse(id),
    }
}

enum Transition {
    Nothing,
    /// Publication d'un commit ; identités nouvelles dans l'ordre canonique.
    Publish {
        forks: bool,
        new_ids: Vec<UsedId>,
    },
    Undo {
        from: CommitId,
        to: CommitId,
    },
    Leave {
        origin: BranchId,
        head: CommitId,
    },
    Redo {
        to: CommitId,
    },
    Reenter(BranchId),
    Checkout(BranchId),
    /// Abandon d'une branche ; si elle est active, la branche active devient `to`, son origine.
    Abandon {
        branch: BranchId,
        to: Option<BranchId>,
    },
    Reactivate(BranchId),
    /// Commit de fusion : son second parent est la tête de la source.
    Merge {
        source_head: CommitId,
    },
}

struct Plan {
    transition: Transition,
    /// État de la tête active après la transition.
    state: Arc<Snapshot>,
    effects: Effects,
}

impl Plan {
    fn publishes(&self) -> bool {
        matches!(
            self.transition,
            Transition::Publish { .. } | Transition::Merge { .. }
        )
    }

    fn forks(&self) -> bool {
        matches!(self.transition, Transition::Publish { forks: true, .. })
    }
}

/// Dépôt : branche active, table des branches et état de la tête active en mémoire ; le reste
/// est lu dans son historique (ETAT-HISTORIQUE.md, section 4). Avec l'historique en mémoire,
/// par défaut, l'égalité compare l'intégralité du dépôt : commits, branches, branche active,
/// identités employées et journal (D20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository<H: History = MemoryHistory> {
    pub(crate) history: H,
    active: BranchId,
    branches: BTreeMap<BranchId, Branch>,
    state: Arc<Snapshot>,
    commits: usize,
}

impl Default for Repository {
    fn default() -> Self {
        Self::new()
    }
}

impl Repository {
    /// Dépôt en mémoire contenant le commit racine, la branche main et le contexte par défaut.
    #[must_use]
    pub fn new() -> Self {
        let history = MemoryHistory::default();
        Self {
            active: history.active,
            branches: history.branches.clone(),
            state: Arc::new(Snapshot::default()),
            commits: history.commits.len(),
            history,
        }
    }

    /// Journal des opérations acceptées, par identifiant d'opération croissant (INV-RETRY-01).
    /// Réservé au dépôt en mémoire : un historique sur disque n'est pas chargé en entier.
    pub fn journal(&self) -> impl Iterator<Item = (&Envelope, &Outcome)> {
        self.history
            .journal
            .values()
            .map(|(envelope, outcome)| (envelope, outcome))
    }
}

impl<H: History> Repository<H> {
    /// Ouvre un dépôt sur un historique : lit la branche active, les branches et l'état de la
    /// tête active, et rien d'autre.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible ; [`Error::MissingBranch`] si la
    /// branche active est inconnue de l'historique.
    pub fn open(history: H) -> Result<Self, Error> {
        let active = history.active()?;
        let branches = history.branches()?;
        let current = branches.get(&active).ok_or(Error::MissingBranch(active))?;
        if current.abandoned {
            return Err(Error::BranchAbandoned(active));
        }
        let head = current.head;
        let state = history.state(head)?;
        let commits = history.commit_count()?;
        Ok(Self {
            history,
            active,
            branches,
            state,
            commits,
        })
    }

    /// Historique du dépôt, en lecture.
    #[must_use]
    pub fn history(&self) -> &H {
        &self.history
    }

    // ----- Lecture -----

    /// Branche active.
    #[must_use]
    pub fn active_branch(&self) -> BranchId {
        self.active
    }

    /// Tête de la branche active.
    #[must_use]
    pub fn head(&self) -> CommitId {
        // Invariant interne : la branche active existe toujours.
        self.branches[&self.active].head
    }

    /// État courant, à reprendre dans l'enveloppe d'une commande.
    #[must_use]
    pub fn expected(&self) -> Expected {
        Expected {
            branch: self.active,
            head: self.head(),
        }
    }

    /// Tête d'une branche.
    ///
    /// # Errors
    /// [`Error::MissingBranch`] si la branche est inconnue.
    pub fn branch_head(&self, branch: BranchId) -> Result<CommitId, Error> {
        self.branches
            .get(&branch)
            .map(|b| b.head)
            .ok_or(Error::MissingBranch(branch))
    }

    /// État de la tête de la branche active.
    #[must_use]
    pub fn snapshot(&self) -> &Snapshot {
        &self.state
    }

    /// État d'un commit conservé, y compris abandonné, lu dans l'historique.
    ///
    /// # Errors
    /// [`Error::MissingCommit`] si le commit est inconnu ; [`Error::Historique`] si l'état ne
    /// peut être lu.
    pub fn snapshot_at(&self, commit: CommitId) -> Result<Arc<Snapshot>, Error> {
        self.history.state(commit)
    }

    /// Motif enregistré avec un commit.
    ///
    /// # Errors
    /// [`Error::MissingCommit`] si le commit est inconnu ; [`Error::Historique`] si
    /// l'historique est illisible.
    pub fn commit_reason(&self, commit: CommitId) -> Result<String, Error> {
        self.commit_info(commit).map(|c| c.reason)
    }

    /// Vrai si la branche est abandonnée.
    ///
    /// # Errors
    /// [`Error::MissingBranch`] si la branche est inconnue.
    pub fn is_abandoned(&self, branch: BranchId) -> Result<bool, Error> {
        self.branch(branch).map(|b| b.abandoned)
    }

    /// Nombre de branches, abandonnées comprises.
    #[must_use]
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// Branches, abandonnées comprises, par identifiant croissant.
    pub fn branches(&self) -> impl Iterator<Item = BranchId> + '_ {
        self.branches.keys().copied()
    }

    /// Nombre de commits, abandonnés compris.
    #[must_use]
    pub fn commit_count(&self) -> usize {
        self.commits
    }

    /// Note désignée par un repère trouvé dans le code, quel que soit le fichier où il se trouve
    /// (INV-CODE-05).
    ///
    /// # Errors
    /// [`Error::NotAMarker`] si la ligne n'est pas un repère canonique ; contexte ou note absents
    /// de l'état courant sinon. [`Repository::branches_containing`] indique alors où chercher.
    pub fn resolve_marker(&self, scope: ScopeId, line: &str) -> Result<NodeId, Error> {
        let id = parse_marker(line.trim()).ok_or(Error::NotAMarker)?;
        self.snapshot().graph(scope)?.node(id)?;
        Ok(id)
    }

    /// Branches dont l'état courant contient la note, dans au moins un contexte.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    pub fn branches_containing(&self, node: NodeId) -> Result<Vec<BranchId>, Error> {
        self.history.branches_containing(node)
    }

    /// Repère à copier dans le code : préfixe et identité de la note, rien d'autre (INV-CODE-02).
    ///
    /// # Errors
    /// Contexte ou note absents de l'état courant.
    pub fn marker(&self, scope: ScopeId, id: NodeId) -> Result<String, Error> {
        self.snapshot().graph(scope)?.node(id)?;
        Ok(format!("{MARKER_PREFIX}{id}"))
    }

    // ----- Écriture -----

    /// Effets qu'aurait la commande sur l'état courant, sans rien modifier.
    ///
    /// # Errors
    /// Les mêmes refus que [`Repository::execute`], hors contrôles propres à l'enveloppe.
    pub fn preview(&self, command: &Command) -> Result<Preview, Error> {
        let plan = self.plan(command)?;
        Ok(Preview {
            base: self.expected(),
            publishes: plan.publishes(),
            forks: plan.forks(),
            effects: plan.effects,
        })
    }

    /// Prépare une enveloppe sans rien modifier : tout refus survient ici, et les écritures
    /// rendues sont exactement celles qu'appliquera [`Repository::execute`].
    ///
    /// # Errors
    /// [`Error::OpIdInUse`] si l'identifiant d'opération a servi pour une autre enveloppe ;
    /// [`Error::Stale`] si l'état attendu n'est plus l'état courant ; [`Error::Historique`] si
    /// une lecture de l'historique échoue ; sinon le refus métier de la commande.
    pub fn prepare(&self, envelope: &Envelope) -> Result<Prepared, Error> {
        if let Some((recorded, outcome)) = self.history.operation(envelope.op)? {
            return if recorded == *envelope {
                Ok(Prepared::Repeated(outcome))
            } else {
                Err(Error::OpIdInUse(envelope.op))
            };
        }
        // L'identifiant nul est celui du commit racine et de main : il ne peut identifier
        // ni une opération ni ce qu'elle publie.
        if envelope.op.is_nil() {
            return Err(Error::NilIdentifier);
        }
        let actual = self.expected();
        if envelope.expected != actual {
            return Err(Error::Stale {
                expected: envelope.expected,
                actual,
            });
        }
        let requires_reason = match &envelope.command {
            Command::Change(change) => change.requires_reason(),
            Command::Lot(elements) => elements.iter().any(Change::requires_reason),
            Command::Undo
            | Command::Redo
            | Command::Checkout { .. }
            | Command::Abandon { .. }
            | Command::Reactivate { .. }
            | Command::Merge { .. }
            | Command::Resume(_) => false,
        };
        if requires_reason && envelope.reason.trim().is_empty() {
            return Err(Error::MissingReason);
        }
        let plan = self.plan(&envelope.command)?;
        self.writes(envelope, plan)
    }

    /// Exécute une commande identifiée : préparation, écriture dans l'historique, puis mise à
    /// jour de la partie en mémoire.
    ///
    /// Une opération déjà exécutée avec la même enveloppe renvoie son résultat enregistré, sans
    /// rien réappliquer (INV-RETRY-01). Une commande sans effet métier ne publie rien et n'est
    /// pas journalisée (INV-MOD-04).
    ///
    /// # Errors
    /// Aucune erreur ne modifie le dépôt (INV-TX-01). Les refus de [`Repository::prepare`], et
    /// [`Error::Historique`] si l'écriture dans l'historique échoue.
    pub fn execute(&mut self, envelope: Envelope) -> Result<Outcome, Error> {
        match self.prepare(&envelope)? {
            Prepared::Repeated(outcome) | Prepared::NoEffect(outcome) => Ok(outcome),
            Prepared::Write(writes) => {
                self.history.apply(&writes)?;
                Ok(self.update(writes))
            }
        }
    }

    // ----- Interne -----

    fn commit_info(&self, commit: CommitId) -> Result<CommitInfo, Error> {
        self.history
            .commit(commit)?
            .ok_or(Error::MissingCommit(commit))
    }

    fn branch(&self, branch: BranchId) -> Result<&Branch, Error> {
        self.branches
            .get(&branch)
            .ok_or(Error::MissingBranch(branch))
    }

    /// Prépare une commande sans rien modifier : toute erreur survient ici.
    fn plan(&self, command: &Command) -> Result<Plan, Error> {
        match command {
            Command::Change(change) => return self.plan_change(change),
            Command::Lot(elements) => return self.plan_lot(elements),
            Command::Abandon { branch } => return self.plan_abandon(*branch),
            Command::Resume(point) => return self.plan_resume(point),
            Command::Merge {
                source,
                source_head,
                resolutions,
            } => return self.plan_merge(*source, *source_head, resolutions),
            Command::Reactivate { branch } => {
                if !self.branch(*branch)?.abandoned {
                    return Err(Error::BranchNotAbandoned(*branch));
                }
                return Ok(Plan {
                    transition: Transition::Reactivate(*branch),
                    state: Arc::clone(&self.state),
                    effects: Effects::default(),
                });
            }
            Command::Undo | Command::Redo | Command::Checkout { .. } => {}
        }
        let branch = self.branch(self.active)?;
        let (transition, state) = match command {
            // Traités ci-dessus.
            Command::Change(_)
            | Command::Lot(_)
            | Command::Abandon { .. }
            | Command::Reactivate { .. }
            | Command::Merge { .. }
            | Command::Resume(_) => (Transition::Nothing, Arc::clone(&self.state)),
            Command::Undo => {
                let from = branch.head;
                let to = *self
                    .commit_info(from)?
                    .parents
                    .first()
                    .ok_or(Error::NothingToUndo)?;
                let transition = match branch.origin {
                    // Opération fondatrice : la base n'est pas franchie, on rejoint l'origine.
                    Some(origin) if to == branch.base => Transition::Leave { origin, head: from },
                    Some(_) | None => Transition::Undo { from, to },
                };
                let state = match transition {
                    Transition::Leave { origin, .. } => {
                        self.history.state(self.branch(origin)?.head)?
                    }
                    _ => self.history.state(to)?,
                };
                (transition, state)
            }
            Command::Redo => match *branch.redo.last().ok_or(Error::NothingToRedo)? {
                Redo::Commit(to) => {
                    if self.commit_info(to)?.parents.first() != Some(&branch.head) {
                        return Err(Error::StaleHead);
                    }
                    (Transition::Redo { to }, self.history.state(to)?)
                }
                Redo::Enter { branch, head } => {
                    self.enterable(branch)?;
                    // Rentrer dans une branche modifiée depuis réappliquerait plus que
                    // l'opération annulée.
                    if self.branch_head(branch)? != head {
                        return Err(Error::StaleHead);
                    }
                    (Transition::Reenter(branch), self.history.state(head)?)
                }
            },
            Command::Checkout { branch } => {
                let head = self.enterable(*branch)?.head;
                if *branch == self.active {
                    (Transition::Nothing, Arc::clone(&self.state))
                } else {
                    (Transition::Checkout(*branch), self.history.state(head)?)
                }
            }
        };
        let effects = review_diff(self.snapshot(), &state);
        Ok(Plan {
            transition,
            state,
            effects,
        })
    }

    /// Branche qui peut devenir active : connue et non abandonnée (D23).
    fn enterable(&self, branch: BranchId) -> Result<&Branch, Error> {
        let found = self.branch(branch)?;
        if found.abandoned {
            return Err(Error::BranchAbandoned(branch));
        }
        Ok(found)
    }

    /// Abandon (FUSION.md, section 9.7) : jamais main ni deux fois ; la branche active revient
    /// sur son origine, avec les effets de ce retour.
    fn plan_abandon(&self, branch: BranchId) -> Result<Plan, Error> {
        let abandoned = self.branch(branch)?;
        if abandoned.abandoned {
            return Err(Error::BranchAbandoned(branch));
        }
        let origin = abandoned.origin.ok_or(Error::MainNotAbandonable)?;
        if branch != self.active {
            return Ok(Plan {
                transition: Transition::Abandon { branch, to: None },
                state: Arc::clone(&self.state),
                effects: Effects::default(),
            });
        }
        let state = self.history.state(self.enterable(origin)?.head)?;
        Ok(Plan {
            effects: review_diff(self.snapshot(), &state),
            transition: Transition::Abandon {
                branch,
                to: Some(origin),
            },
            state,
        })
    }

    /// Plan d'une fusion de `source` dans la branche active, sans rien modifier (FUSION.md,
    /// section 9.1) : base commune, conflits restants après les résolutions données, conflit
    /// structurel éventuel, effets sur les réexamens. La fusion applique exactement ce plan.
    ///
    /// # Errors
    /// [`Error::MergeIntoItself`] ; [`Error::MissingBranch`] ; [`Error::BranchAbandoned`] pour une
    /// source abandonnée ; [`Error::MultipleMergeBases`] ; [`Error::DuplicateResolution`],
    /// [`Error::UnmatchedResolution`] ; [`Error::IncoherentMerge`] ; [`Error::Historique`].
    pub fn merge_plan(
        &self,
        source: BranchId,
        resolutions: &[Resolution],
    ) -> Result<MergePlan, Error> {
        self.merge_parts(source, resolutions).map(|(plan, _)| plan)
    }

    /// Plan d'une restauration, en lecture seule (RESTAURATION.md, section 2) : ciblée sur
    /// `nodes`, ou complète sans sélection ; exactement ce que la restauration appliquera
    /// (INV-PREVIEW-01).
    ///
    /// # Errors
    /// [`Error::MissingCommit`] ; [`Error::EmptySelection`], [`Error::AbsentFromTarget`] ; la
    /// règle violée par l'état restauré ; [`Error::Historique`].
    pub fn restore_plan(
        &self,
        target: CommitId,
        nodes: Option<&BTreeSet<NodeId>>,
    ) -> Result<RestorePlan, Error> {
        let old = self.history.state(target)?;
        let state = match nodes {
            Some(nodes) => restore::restored_nodes(self.snapshot(), &old, nodes)?,
            None => self.restored(target)?,
        };
        state.validate()?;
        Ok(RestorePlan {
            target,
            items: restore::items(self.snapshot(), &old, &state, nodes),
            effects: review_diff(self.snapshot(), &state),
        })
    }

    /// Commits de la branche active, de la tête vers la racine le long des premiers parents,
    /// au plus `limit` (RESTAURATION.md, section 3).
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    pub fn history_of_branch(&self, limit: usize) -> Result<Vec<HistoryEntry>, Error> {
        let mut entries = Vec::new();
        let mut next = Some(self.head());
        while let Some(commit) = next.filter(|_| entries.len() < limit) {
            let info = self.commit_info(commit)?;
            let command = self
                .history
                .operation(OpId::from_bytes(commit.to_bytes()))?
                .map(|(envelope, _)| envelope.command);
            entries.push(HistoryEntry {
                commit,
                reason: info.reason,
                command,
            });
            next = info.parents.first().copied();
        }
        Ok(entries)
    }

    /// Fiche de reprise publiée par un commit (FICHE-REPRISE.md, section 3), quelle que soit sa
    /// branche, même annulée.
    ///
    /// # Errors
    /// [`Error::MissingCommit`] ; [`Error::NotAResumePoint`] si le commit n'a pas été publié par
    /// une fiche ; [`Error::Historique`].
    pub fn resume_point(&self, commit: CommitId) -> Result<ResumePoint, Error> {
        self.commit_info(commit)?;
        match self
            .history
            .operation(OpId::from_bytes(commit.to_bytes()))?
        {
            Some((envelope, _)) => match envelope.command {
                Command::Resume(point) => Ok(*point),
                _ => Err(Error::NotAResumePoint(commit)),
            },
            None => Err(Error::NotAResumePoint(commit)),
        }
    }

    /// Fiches d'une branche, de sa tête vers la racine le long des premiers parents, au plus
    /// `limit` (FICHE-REPRISE.md, section 3).
    ///
    /// # Errors
    /// [`Error::MissingBranch`] ; [`Error::Historique`].
    pub fn resume_points(&self, branch: BranchId, limit: usize) -> Result<Vec<ResumeEntry>, Error> {
        let mut entries = Vec::new();
        let mut next = Some(self.branch(branch)?.head);
        while let Some(commit) = next.filter(|_| entries.len() < limit) {
            let info = self.commit_info(commit)?;
            if let Some((envelope, _)) = self
                .history
                .operation(OpId::from_bytes(commit.to_bytes()))?
            {
                if let Command::Resume(point) = envelope.command {
                    entries.push(ResumeEntry {
                        commit,
                        point: *point,
                    });
                }
            }
            next = info.parents.first().copied();
        }
        Ok(entries)
    }

    /// Nombre de commits de la branche active depuis `commit`, le long des premiers parents ;
    /// aucun si `commit` n'est pas dans cet historique (fiche annulée ou d'une autre branche).
    ///
    /// # Errors
    /// [`Error::MissingCommit`] ; [`Error::Historique`].
    pub fn operations_since(&self, commit: CommitId) -> Result<Option<usize>, Error> {
        self.commit_info(commit)?;
        let mut count = 0;
        let mut next = Some(self.head());
        while let Some(current) = next {
            if current == commit {
                return Ok(Some(count));
            }
            count += 1;
            next = self.commit_info(current)?.parents.first().copied();
        }
        Ok(None)
    }

    /// Ancêtres d'un commit, lui compris, par les deux parents des commits de fusion.
    fn ancestors(&self, head: CommitId) -> Result<BTreeSet<CommitId>, Error> {
        let mut seen = BTreeSet::new();
        let mut stack = vec![head];
        while let Some(commit) = stack.pop() {
            if seen.insert(commit) {
                stack.extend(self.commit_info(commit)?.parents);
            }
        }
        Ok(seen)
    }

    /// Plan de fusion et, sauf si la source est déjà réunie, l'état réuni.
    fn merge_parts(
        &self,
        source: BranchId,
        resolutions: &[Resolution],
    ) -> Result<(MergePlan, Option<Snapshot>), Error> {
        if source == self.active {
            return Err(Error::MergeIntoItself);
        }
        let source_head = self.enterable(source)?.head;
        let target_head = self.head();
        let target_side = self.ancestors(target_head)?;
        if target_side.contains(&source_head) {
            let plan = MergePlan {
                base: source_head,
                target_head,
                source_head,
                up_to_date: true,
                from_source: 0,
                conflicts: Vec::new(),
                structural: None,
                effects: Effects::default(),
            };
            // Rien à résoudre : toute résolution est sans objet.
            return match resolutions.len() {
                0 => Ok((plan, None)),
                _ => Err(Error::UnmatchedResolution(1)),
            };
        }
        // Base : l'unique ancêtre commun qui n'est l'ancêtre d'aucun autre (FUSION.md, 9.2).
        let common: BTreeSet<CommitId> = self
            .ancestors(source_head)?
            .intersection(&target_side)
            .copied()
            .collect();
        let mut below = BTreeSet::new();
        let mut stack = Vec::new();
        for commit in &common {
            stack.extend(self.commit_info(*commit)?.parents);
        }
        while let Some(commit) = stack.pop() {
            if below.insert(commit) {
                stack.extend(self.commit_info(commit)?.parents);
            }
        }
        let mut nearest = common.difference(&below);
        let base = match (nearest.next(), nearest.next()) {
            (Some(base), None) => *base,
            _ => return Err(Error::MultipleMergeBases),
        };
        let source_state = self.history.state(source_head)?;
        let merged = merge(
            &*self.history.state(base)?,
            self.snapshot(),
            &source_state,
            resolutions,
        )?;
        let plan = MergePlan {
            base,
            target_head,
            source_head,
            up_to_date: false,
            from_source: merged.from_source,
            conflicts: merged.conflicts,
            structural: merged.structural,
            effects: review_diff(self.snapshot(), &merged.state),
        };
        Ok((plan, Some(merged.state)))
    }

    /// Fusion (FUSION.md, sections 5 et 9.6) : refusée si la source a bougé depuis le plan ou s'il
    /// reste un conflit ; sans effet si la source est déjà réunie ; jamais de bifurcation (D19).
    fn plan_merge(
        &self,
        source: BranchId,
        expected: CommitId,
        resolutions: &[Resolution],
    ) -> Result<Plan, Error> {
        if source != self.active {
            let actual = self.enterable(source)?.head;
            if actual != expected {
                return Err(Error::StaleSource { expected, actual });
            }
        }
        let (plan, state) = self.merge_parts(source, resolutions)?;
        let Some(state) = state else {
            return Ok(Plan {
                transition: Transition::Nothing,
                state: Arc::clone(&self.state),
                effects: Effects::default(),
            });
        };
        if !plan.conflicts.is_empty() {
            return Err(Error::UnresolvedConflicts(plan.conflicts.len()));
        }
        if let Some(structural) = plan.structural {
            return Err(Error::StructuralConflict(Box::new(structural.reason)));
        }
        Ok(Plan {
            transition: Transition::Merge {
                source_head: expected,
            },
            state: Arc::new(state),
            effects: plan.effects,
        })
    }

    /// Fiche de reprise (D26) : un commit d'état identique, jamais de bifurcation, aucun effet.
    fn plan_resume(&self, point: &ResumePoint) -> Result<Plan, Error> {
        point.validate(self.snapshot())?;
        Ok(Plan {
            transition: Transition::Publish {
                forks: false,
                new_ids: Vec::new(),
            },
            state: Arc::clone(&self.state),
            effects: Effects::default(),
        })
    }

    fn plan_change(&self, change: &Change) -> Result<Plan, Error> {
        let state = self.applied(self.snapshot(), change, &BTreeSet::new())?;
        let restores = matches!(change, Change::Restore { .. } | Change::RestoreNodes { .. });
        Ok(self.publication(state, restores, change.new_id().into_iter().collect()))
    }

    /// Lot (LOT.md, section 2) : chaque élément s'applique à l'état laissé par les précédents ;
    /// le refus d'un élément refuse tout le lot et le nomme. Le lot est jugé sur son effet net.
    fn plan_lot(&self, elements: &[Change]) -> Result<Plan, Error> {
        if elements.is_empty() {
            return Err(Error::EmptyLot);
        }
        if elements
            .iter()
            .any(|change| matches!(change, Change::Restore { .. } | Change::RestoreNodes { .. }))
        {
            return Err(Error::NotInLot);
        }
        let mut state = self.snapshot().clone();
        let mut taken = BTreeSet::new();
        for (index, change) in elements.iter().enumerate() {
            state = self
                .applied(&state, change, &taken)
                .map_err(|error| match error {
                    // Une panne de l'historique n'est pas le refus d'un élément.
                    Error::Historique(_) => error,
                    cause => Error::InLot {
                        element: index + 1,
                        cause: Box::new(cause),
                    },
                })?;
            taken.extend(change.new_id());
        }
        Ok(self.publication(state, false, taken.into_iter().collect()))
    }

    /// État après une modification appliquée à `base`, validé : identité nouvelle non nulle,
    /// jamais employée dans l'historique, ni déjà prise par un élément du même lot (`taken`).
    fn applied(
        &self,
        base: &Snapshot,
        change: &Change,
        taken: &BTreeSet<UsedId>,
    ) -> Result<Snapshot, Error> {
        if let Some(new) = change.new_id() {
            if new.is_nil() {
                return Err(Error::NilIdentifier);
            }
            if taken.contains(&new) || self.history.is_used(new)? {
                return Err(in_use(new));
            }
        }
        let state = self.candidate(base, change)?;
        state.validate()?;
        Ok(state)
    }

    /// Plan d'une publication vers `state` : rien si l'état ne change pas (INV-MOD-04) ; sinon
    /// effets et criticité jugés de l'état courant à `state` (D13).
    fn publication(&self, state: Snapshot, restores: bool, new_ids: Vec<UsedId>) -> Plan {
        let current = self.snapshot();
        if state == *current {
            return Plan {
                transition: Transition::Nothing,
                state: Arc::clone(&self.state),
                effects: Effects::default(),
            };
        }
        let effects = review_diff(current, &state);
        // Bifurcation (D1) : seule une modification critique engagée sur main crée une branche.
        let forks = self.active == MAIN && is_critical(restores, current, &state, &effects);
        Plan {
            transition: Transition::Publish { forks, new_ids },
            state: Arc::new(state),
            effects,
        }
    }

    /// État candidat d'une modification, à partir de `base`.
    fn candidate(&self, base: &Snapshot, change: &Change) -> Result<Snapshot, Error> {
        if let Change::Restore { target } = change {
            return self.restored(*target);
        }
        if let Change::RestoreNodes { target, nodes } = change {
            return restore::restored_nodes(self.snapshot(), &*self.history.state(*target)?, nodes);
        }
        let mut state = base.clone();
        match change {
            Change::CreateNode {
                scope,
                id,
                kind,
                title,
                text,
            } => {
                // Échanges, passages et réserves portent des données propres : commande dédiée.
                if kind.has_body() {
                    return Err(Error::WrongKind(*id));
                }
                state.insert_note(*scope, *id, Content::new(*kind, title, text))?;
            }
            Change::CreateExchange {
                scope,
                id,
                title,
                question,
                answer,
                previous,
            } => {
                if let Some(previous) = previous {
                    state.exchange(*previous)?;
                }
                let body = Body::Exchange {
                    question: question.as_deref().map(source).transpose()?,
                    answer: source(answer)?,
                    previous: *previous,
                };
                let content = Content::with_body(Kind::Exchange, title, "", body);
                state.insert_note(*scope, *id, content)?;
            }
            Change::SetQuestion { exchange, question } => {
                let mut content = (**state.content(*exchange)?).clone();
                match &mut content.body {
                    Body::Exchange {
                        question: slot @ None,
                        ..
                    } => *slot = Some(source(question)?),
                    Body::Exchange { .. } => return Err(Error::SourceImmutable(*exchange)),
                    Body::Note
                    | Body::Passage { .. }
                    | Body::Citation { .. }
                    | Body::Reserve { .. } => return Err(Error::WrongKind(*exchange)),
                }
                state.set_content(*exchange, Arc::new(content));
            }
            Change::CreatePassage {
                scope,
                id,
                exchange,
                part,
                start,
                end,
            } => {
                state.graph(*scope)?.node(*exchange)?;
                let text = state.excerpt(*exchange, *part, *start, *end)?.to_owned();
                let body = Body::Passage {
                    exchange: *exchange,
                    part: *part,
                    start: *start,
                    end: *end,
                };
                let content = Content::with_body(Kind::Passage, "", &text, body);
                state.insert_note(*scope, *id, content)?;
            }
            Change::CiteNote {
                scope,
                id,
                note,
                version,
                start,
                end,
            } => {
                state.graph(*scope)?.node(*note)?;
                let text = state
                    .note_excerpt(*note, *version, *start, *end)?
                    .to_owned();
                let body = Body::Citation {
                    note: *note,
                    version: *version,
                    start: *start,
                    end: *end,
                };
                let content = Content::with_body(Kind::Passage, "", &text, body);
                state.insert_note(*scope, *id, content)?;
            }
            Change::CreateReserve {
                scope,
                id,
                passage,
                title,
                target,
            } => {
                state.graph(*scope)?.node(*passage)?;
                state.passage(*passage)?;
                let body = Body::Reserve {
                    passage: *passage,
                    target: target.as_deref().map(annotation).transpose()?,
                };
                let content = Content::with_body(Kind::Reserve, title, "", body);
                state.insert_note(*scope, *id, content)?;
            }
            Change::Treat {
                scope,
                id,
                note,
                reserve,
            } => {
                let graph = state.graph_mut(*scope)?;
                // Si un traitement est déjà actif, l'état d'avant tout traitement est celui qu'il
                // a mémorisé : c'est lui que le dernier retrait doit rétablir (défaut S12).
                let inherited = graph
                    .edges
                    .values()
                    .find(|e| e.active && e.relation == Relation::Treats && e.to == *reserve)
                    .and_then(|e| e.prior);
                let current = graph.node(*reserve)?.reserve.map(Prior::Reserve);
                let prior = inherited.or(current);
                graph.insert_edge(*id, *note, *reserve, Relation::Treats, None, prior)?;
                // Seule l'action « traite » fait passer à « traitée à vérifier » (INV-RES-03).
                if let Some(status) = graph.node_mut(*reserve)?.reserve.as_mut() {
                    *status = ReserveStatus::TreatedPendingVerification;
                }
            }
            Change::Acknowledge {
                scope,
                dependent,
                cause,
            } => {
                let graph = state.graph_mut(*scope)?;
                let causes = graph.review_causes();
                if !causes.get(dependent).is_some_and(|c| c.contains(cause)) {
                    return Err(Error::NoSuchCause);
                }
                let ack = Ack {
                    dependent: graph.node(*dependent)?.content.substantive(),
                    cause: graph.cause_state(*cause)?,
                };
                graph.acks.insert((*dependent, *cause), ack);
            }
            Change::Detach { scope, edge } => state.graph_mut(*scope)?.detach(*edge)?,
            Change::AnchorToCode {
                id,
                node,
                path,
                excerpt,
                git,
            } => {
                state.content(*node)?;
                let anchor = Anchor {
                    node: *node,
                    path: path.clone(),
                    excerpt: excerpt.clone(),
                    digest: text_digest(excerpt),
                    git: git.clone(),
                    active: true,
                };
                // La validation refuse un chemin, une copie ou un commit vides.
                state.anchors.insert(*id, anchor);
            }
            Change::Unanchor { anchor } => {
                let found = state
                    .anchors
                    .get_mut(anchor)
                    .ok_or(Error::MissingAnchor(*anchor))?;
                // Retirer un ancrage déjà retiré n'a aucun effet (INV-MOD-04).
                found.active = false;
            }
            Change::SetReserveStatus {
                scope,
                reserve,
                status,
            } => {
                let node = state.graph_mut(*scope)?.node_mut(*reserve)?;
                let slot = node.reserve.as_mut().ok_or(Error::WrongKind(*reserve))?;
                *slot = *status;
            }
            Change::Revise { node, text, kind } => {
                let current = state.content(*node)?;
                if matches!(current.kind, Kind::Exchange | Kind::Passage) {
                    return Err(Error::SourceImmutable(*node));
                }
                // Un contenu identique ne crée pas de révision : commande sans effet (INV-MOD-04).
                if current.current().text != *text {
                    let mut revised = (**current).clone();
                    let title = revised.current().title.clone();
                    revised.revisions.push((
                        *kind,
                        Text {
                            title,
                            text: text.clone(),
                        },
                    ));
                    state.set_content(*node, Arc::new(revised));
                }
            }
            Change::Include { scope, node } => {
                let content = Arc::clone(state.content(*node)?);
                state
                    .graph_mut(*scope)?
                    .nodes
                    .entry(*node)
                    .or_insert_with(|| Node::new(content));
            }
            Change::CloneScope { source, id } => {
                let graph = state.graph(*source)?.clone();
                state.scopes.insert(*id, graph);
            }
            Change::Link {
                scope,
                id,
                from,
                to,
                relation,
            } => {
                // Les relations porteuses d'un statut ou d'un texte ont leur commande dédiée.
                if !relation.is_plain() {
                    return Err(Error::InvalidRelation);
                }
                state
                    .graph_mut(*scope)?
                    .insert_edge(*id, *from, *to, *relation, None, None)?;
            }
            Change::Replace {
                scope,
                id,
                replacement,
                replaced,
            } => {
                let graph = state.graph_mut(*scope)?;
                // Un seul remplacement actif à la fois : le statut antérieur reste rétablissable.
                let prior = graph.node(*replaced)?.decision;
                if prior == Some(Decision::Replaced) {
                    return Err(Error::ReservedStatus);
                }
                let prior = prior.map(Prior::Decision);
                graph.insert_edge(
                    *id,
                    *replacement,
                    *replaced,
                    Relation::Replaces,
                    None,
                    prior,
                )?;
                // Statut et relation dans la même opération ; une catégorie, sans statut, a déjà
                // été refusée par les règles de relation.
                if let Some(decision) = graph.node_mut(*replaced)?.decision.as_mut() {
                    *decision = Decision::Replaced;
                }
            }
            Change::Restrict {
                scope,
                id,
                from,
                to,
                condition,
            } => {
                let annotation = annotation(condition)?;
                state.graph_mut(*scope)?.insert_edge(
                    *id,
                    *from,
                    *to,
                    Relation::RestrictsScope,
                    Some(annotation),
                    None,
                )?;
            }
            Change::Contradict {
                scope,
                id,
                from,
                to,
                justification,
            } => {
                let annotation = annotation(justification)?;
                state.graph_mut(*scope)?.insert_edge(
                    *id,
                    *from,
                    *to,
                    Relation::Contradicts,
                    Some(annotation),
                    None,
                )?;
            }
            Change::SetVerdict {
                scope,
                node,
                verdict,
            } => {
                let current = state.graph_mut(*scope)?.node_mut(*node)?;
                let slot = current
                    .verdict
                    .as_mut()
                    .ok_or(Error::CategoryHasNoVerdict)?;
                *slot = *verdict;
            }
            Change::SetDecision {
                scope,
                node,
                decision,
            } => {
                if *decision == Decision::Replaced {
                    return Err(Error::ReservedStatus);
                }
                let current = state.graph_mut(*scope)?.node_mut(*node)?;
                let slot = current
                    .decision
                    .as_mut()
                    .ok_or(Error::CategoryHasNoVerdict)?;
                *slot = *decision;
            }
            Change::SetImplementation {
                scope,
                node,
                implementation,
            } => {
                let current = state.graph_mut(*scope)?.node_mut(*node)?;
                let slot = current
                    .implementation
                    .as_mut()
                    .ok_or(Error::CategoryHasNoVerdict)?;
                *slot = *implementation;
            }
            Change::Archive { scope, node } => {
                state.graph_mut(*scope)?.node_mut(*node)?.visibility = Visibility::Archived;
            }
            Change::Reattach {
                scope,
                edge,
                to,
                id,
            } => {
                let graph = state.graph_mut(*scope)?;
                let old = graph.edge(*edge)?.clone();
                if !old.active {
                    return Err(Error::InactiveEdge);
                }
                // Déplacer un remplacement, une restriction ou une contradiction changerait un
                // statut ou un texte : ces relations ne se réattachent pas.
                if !old.relation.is_plain() {
                    return Err(Error::InvalidRelation);
                }
                graph
                    .edges
                    .get_mut(edge)
                    .ok_or(Error::MissingEdge(*edge))?
                    .active = false;
                graph.insert_edge(*id, old.from, *to, old.relation, None, None)?;
            }
            // Traitées avant la copie de l'état courant.
            Change::Restore { .. } | Change::RestoreNodes { .. } => {}
        }
        Ok(state)
    }

    /// Restauration non destructive (D6, INV-RET-01, INV-RET-03) : les objets existant à la cible
    /// reprennent leur état d'alors ; ceux créés depuis restent présents, retirés des ressources
    /// actuelles, et les relations créées depuis sont désactivées. Aucune identité ne disparaît.
    fn restored(&self, target: CommitId) -> Result<Snapshot, Error> {
        let mut state = (*self.history.state(target)?).clone();
        let now = self.snapshot();
        // Le contenu d'une note existant à la cible reprend sa version d'alors ; celui d'une note
        // créée depuis est conservé. Chaque contexte référence ensuite ce contenu canonique.
        for (&id, content) in &now.notes {
            state.notes.entry(id).or_insert_with(|| Arc::clone(content));
        }
        for (&id, anchor) in &now.anchors {
            state.anchors.entry(id).or_insert_with(|| Anchor {
                active: false,
                ..anchor.clone()
            });
        }
        let canonical = state.notes.clone();
        for (&scope, current) in &now.scopes {
            let restored = state.scopes.entry(scope).or_default();
            for (&id, node) in &current.nodes {
                let content = canonical
                    .get(&id)
                    .map_or_else(|| Arc::clone(&node.content), Arc::clone);
                restored.nodes.entry(id).or_insert_with(|| Node {
                    content,
                    verdict: node.verdict,
                    decision: node.decision,
                    implementation: node.implementation,
                    reserve: node.reserve,
                    visibility: Visibility::Archived,
                });
            }
            for (&id, edge) in &current.edges {
                restored.edges.entry(id).or_insert_with(|| Edge {
                    active: false,
                    ..edge.clone()
                });
            }
        }
        Ok(state)
    }

    /// Écritures d'un plan préparé, sans rien modifier.
    fn writes(&self, envelope: &Envelope, plan: Plan) -> Result<Prepared, Error> {
        let op = envelope.op;
        let current = self.branch(self.active)?;
        let mut branches = BTreeMap::new();
        let mut commit = None;
        let mut identities = Vec::new();
        let (active, published, forked) = match plan.transition {
            Transition::Nothing => {
                return Ok(Prepared::NoEffect(Outcome {
                    op,
                    branch: self.active,
                    head: self.head(),
                    published: false,
                    forked: false,
                    effects: plan.effects,
                }));
            }
            Transition::Publish { forks, new_ids } => {
                // Le commit et l'éventuelle branche portent l'identité de l'opération : elle est
                // unique dans le journal et non nulle, donc jamais déjà employée.
                let id = CommitId::from_bytes(op.to_bytes());
                let parent = current.head;
                let (destination, branch) = if forks {
                    let branch = Branch {
                        head: id,
                        redo: Vec::new(),
                        origin: Some(self.active),
                        base: parent,
                        abandoned: false,
                    };
                    (BranchId::from_bytes(op.to_bytes()), branch)
                } else {
                    // Une nouvelle action efface la pile de rétablissement ; les commits
                    // abandonnés restent conservés (INV-UNDO-02).
                    let branch = Branch {
                        head: id,
                        redo: Vec::new(),
                        ..current.clone()
                    };
                    (self.active, branch)
                };
                branches.insert(destination, branch);
                commit = Some(NewCommit {
                    id,
                    parent,
                    merged: None,
                    reason: envelope.reason.clone(),
                    state: Arc::clone(&plan.state),
                });
                identities = new_ids;
                (destination, true, forks)
            }
            Transition::Merge { source_head } => {
                let id = CommitId::from_bytes(op.to_bytes());
                branches.insert(
                    self.active,
                    Branch {
                        head: id,
                        redo: Vec::new(),
                        ..current.clone()
                    },
                );
                commit = Some(NewCommit {
                    id,
                    parent: current.head,
                    merged: Some(source_head),
                    reason: envelope.reason.clone(),
                    state: Arc::clone(&plan.state),
                });
                (self.active, true, false)
            }
            Transition::Undo { from, to } => {
                let mut branch = current.clone();
                branch.redo.push(Redo::Commit(from));
                branch.head = to;
                branches.insert(self.active, branch);
                (self.active, false, false)
            }
            Transition::Leave { origin, head } => {
                let mut branch = self.branch(origin)?.clone();
                branch.redo.push(Redo::Enter {
                    branch: self.active,
                    head,
                });
                branches.insert(origin, branch);
                (origin, false, false)
            }
            Transition::Redo { to } => {
                let mut branch = current.clone();
                branch.redo.pop();
                branch.head = to;
                branches.insert(self.active, branch);
                (self.active, false, false)
            }
            Transition::Reenter(entered) => {
                let mut branch = current.clone();
                branch.redo.pop();
                branches.insert(self.active, branch);
                (entered, false, false)
            }
            Transition::Checkout(branch) => (branch, false, false),
            Transition::Abandon { branch, to } => {
                let mut abandoned = self.branch(branch)?.clone();
                abandoned.abandoned = true;
                branches.insert(branch, abandoned);
                (to.unwrap_or(self.active), false, false)
            }
            Transition::Reactivate(branch) => {
                let mut reactivated = self.branch(branch)?.clone();
                reactivated.abandoned = false;
                branches.insert(branch, reactivated);
                (self.active, false, false)
            }
        };
        let head = match branches.get(&active) {
            Some(branch) => branch.head,
            None => self.branch(active)?.head,
        };
        let outcome = Outcome {
            op,
            branch: active,
            head,
            published,
            forked,
            effects: plan.effects,
        };
        Ok(Prepared::Write(Writes {
            envelope: envelope.clone(),
            outcome,
            commit,
            branches,
            active,
            identities,
            state: plan.state,
        }))
    }

    /// Met à jour la partie en mémoire après l'écriture réussie dans l'historique. Infaillible.
    fn update(&mut self, writes: Writes) -> Outcome {
        if writes.commit.is_some() {
            self.commits += 1;
        }
        self.branches.extend(writes.branches);
        self.active = writes.active;
        self.state = writes.state;
        writes.outcome
    }
}

/// Critère D13 : une modification est critique si, dans l'état courant, elle crée ou lève une
/// cause de réexamen, si elle écarte une note (réfutation, « plus retenue », retrait des
/// ressources actuelles, remplacement), ou si elle restaure un état antérieur. La criticité se
/// juge sur l'effet réel, pas sur le seul type de commande.
/// Pour un lot, l'effet jugé est l'effet net, de l'état courant à l'état final (D22).
fn is_critical(restores: bool, before: &Snapshot, after: &Snapshot, effects: &Effects) -> bool {
    restores || !effects.review.is_empty() || withdraws(before, after)
}

/// Vrai si une note est écartée dans un contexte où elle ne l'était pas.
fn withdraws(before: &Snapshot, after: &Snapshot) -> bool {
    after.scopes.iter().any(|(scope, graph)| {
        graph.nodes.iter().any(|(id, node)| {
            node.withdrawn()
                && !before
                    .scopes
                    .get(scope)
                    .and_then(|g| g.nodes.get(id))
                    .is_some_and(Node::withdrawn)
        })
    })
}

/// Source d'échange : conservée octet pour octet, mais jamais vide (INV-SOURCE-03).
fn source(text: &str) -> Result<Arc<str>, Error> {
    if text.trim().is_empty() {
        return Err(Error::EmptySource);
    }
    Ok(Arc::from(text))
}

/// Texte d'une restriction ou d'une contradiction : non vide (INV-REL-06, INV-REL-07).
fn annotation(text: &str) -> Result<String, Error> {
    if text.trim().is_empty() {
        return Err(Error::MissingAnnotation);
    }
    Ok(text.to_owned())
}

/// Variations des causes de réexamen entre deux états, contexte par contexte.
fn review_diff(before: &Snapshot, after: &Snapshot) -> Effects {
    let scopes: BTreeSet<ScopeId> = before
        .scopes
        .keys()
        .chain(after.scopes.keys())
        .copied()
        .collect();
    let mut review = BTreeMap::new();
    for scope in scopes {
        let causes = |s: &Snapshot| {
            s.scopes
                .get(&scope)
                .map(|g| g.review_causes())
                .unwrap_or_default()
        };
        let (old, new) = (causes(before), causes(after));
        let nodes: BTreeSet<NodeId> = old.keys().chain(new.keys()).copied().collect();
        for node in nodes {
            let b = old.get(&node).cloned().unwrap_or_default();
            let a = new.get(&node).cloned().unwrap_or_default();
            if b != a {
                review.insert(
                    (scope, node),
                    ReviewChange {
                        before: b,
                        after: a,
                    },
                );
            }
        }
    }
    Effects { review }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Un historique dont la branche active est abandonnée est incohérent : il ne s'ouvre pas
    // (D23). Le format le refuse déjà à la lecture ; l'historique SQLite passe par ici.
    #[test]
    fn an_abandoned_active_branch_is_refused_at_opening() {
        let mut history = MemoryHistory::default();
        assert!(Repository::open(history.clone()).is_ok());
        if let Some(main) = history.branches.get_mut(&MAIN) {
            main.abandoned = true;
        }
        assert_eq!(Repository::open(history), Err(Error::BranchAbandoned(MAIN)));
    }
}
