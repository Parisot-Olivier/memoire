//! Historique du dépôt, derrière un trait (ETAT-HISTORIQUE.md, section 4 ; D20).
//!
//! Le dépôt ([`crate::Repository`]) ne garde en mémoire que la branche active, la table des
//! branches et l'état de la tête active ; tout le reste (commits et leurs états, journal,
//! identités employées) est lu par le trait [`History`]. Le domaine reste sans entrée-sortie :
//! il appelle le trait, que le stockage implémente sur SQLite. [`MemoryHistory`] garde tout en
//! mémoire, un instantané complet par commit : c'est l'historique des contrats.
//!
//! Une commande acceptée produit des [`Writes`], que l'historique applique tout ou rien ; la
//! partie en mémoire du dépôt n'est mise à jour qu'après leur réussite (INV-TX-01).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::command::{Envelope, UsedId};
use crate::error::Error;
use crate::id::{BranchId, CommitId, NodeId, OpId, DEFAULT_SCOPE, MAIN, ROOT};
use crate::model::Snapshot;
use crate::repository::{Branch, Commit, Outcome, Used};

/// Parents et motif d'un commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitInfo {
    /// Parents, le premier d'abord : aucun pour la racine, deux pour une fusion (la cible, puis la
    /// source), un sinon.
    pub parents: Vec<CommitId>,
    /// Motif enregistré avec le commit.
    pub reason: String,
}

/// Commit publié par une opération.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewCommit {
    /// Identité du commit : celle de l'opération qui le publie.
    pub id: CommitId,
    /// Premier parent : la tête de la branche active avant l'opération.
    pub parent: CommitId,
    /// Second parent d'un commit de fusion : la tête de la source réunie (FUSION.md,
    /// section 9.6) ; aucun sinon.
    pub merged: Option<CommitId>,
    /// Motif de l'opération.
    pub reason: String,
    /// État complet du commit.
    pub state: Arc<Snapshot>,
}

/// Écritures d'une opération acceptée, à appliquer tout ou rien (une transaction SQLite).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Writes {
    /// Enveloppe de l'opération, conservée au journal.
    pub envelope: Envelope,
    /// Résultat de l'opération, conservé au journal.
    pub outcome: Outcome,
    /// Commit publié, s'il y en a un.
    pub commit: Option<NewCommit>,
    /// Branches créées ou modifiées, dans leur état après l'opération ; les autres sont
    /// inchangées.
    pub branches: BTreeMap<BranchId, Branch>,
    /// Branche active après l'opération.
    pub active: BranchId,
    /// Identités nouvelles employées par l'opération, dans l'ordre canonique : aucune, une, ou
    /// plusieurs pour un lot.
    pub identities: Vec<UsedId>,
    /// État de la tête de la branche active après l'opération.
    pub state: Arc<Snapshot>,
}

/// Préparation d'une enveloppe, sans rien modifier.
// Écart de taille entre variantes admis : une préparation est produite une fois par commande et
// consommée aussitôt ; mettre les écritures en boîte ajouterait une allocation sans gain.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prepared {
    /// Opération déjà appliquée avec cette même enveloppe : son résultat enregistré, rien à
    /// écrire (INV-RETRY-01).
    Repeated(Outcome),
    /// Commande sans effet métier : rien n'est publié ni journalisé (INV-MOD-04).
    NoEffect(Outcome),
    /// Écritures à appliquer.
    Write(Writes),
}

/// Historique d'un dépôt : commits, journal, identités employées, branches et état de leur tête.
/// Toute lecture peut échouer ([`Error::Historique`]) ; une écriture est tout ou rien.
pub trait History {
    /// Branche active enregistrée.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    fn active(&self) -> Result<BranchId, Error>;
    /// Toutes les branches et leurs métadonnées.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    fn branches(&self) -> Result<BTreeMap<BranchId, Branch>, Error>;
    /// Nombre de commits, abandonnés compris.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    fn commit_count(&self) -> Result<usize, Error>;
    /// État complet d'un commit.
    ///
    /// # Errors
    /// [`Error::MissingCommit`] si le commit est inconnu ; [`Error::Historique`] si l'état ne
    /// peut être lu ou reconstruit.
    fn state(&self, commit: CommitId) -> Result<Arc<Snapshot>, Error>;
    /// Parents et motif d'un commit ; `None` s'il est inconnu.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    fn commit(&self, commit: CommitId) -> Result<Option<CommitInfo>, Error>;
    /// Enveloppe et résultat d'une opération déjà appliquée.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    fn operation(&self, op: OpId) -> Result<Option<(Envelope, Outcome)>, Error>;
    /// Vrai si l'identité a déjà été employée, même par un état abandonné (INV-ID-02).
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    fn is_used(&self, id: UsedId) -> Result<bool, Error>;
    /// Branches non abandonnées dont l'état courant contient la note, par identifiant croissant.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'historique est illisible.
    fn branches_containing(&self, node: NodeId) -> Result<Vec<BranchId>, Error>;
    /// Applique les écritures d'une opération, tout ou rien.
    ///
    /// # Errors
    /// [`Error::Historique`] si l'écriture échoue ; rien n'est alors écrit.
    fn apply(&mut self, writes: &Writes) -> Result<(), Error>;
}

/// Historique entièrement en mémoire : un instantané complet par commit, le journal, les
/// identités employées. L'égalité compare tout (D20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryHistory {
    pub(crate) commits: BTreeMap<CommitId, Commit>,
    pub(crate) branches: BTreeMap<BranchId, Branch>,
    pub(crate) active: BranchId,
    pub(crate) used: Used,
    pub(crate) journal: BTreeMap<OpId, (Envelope, Outcome)>,
}

impl Default for MemoryHistory {
    /// Historique d'un dépôt neuf : commit racine, branche main, contexte par défaut.
    fn default() -> Self {
        Self {
            commits: BTreeMap::from([(
                ROOT,
                Commit {
                    state: Arc::new(Snapshot::default()),
                    parent: None,
                    merged: None,
                    reason: String::new(),
                },
            )]),
            branches: BTreeMap::from([(
                MAIN,
                Branch {
                    head: ROOT,
                    redo: Vec::new(),
                    origin: None,
                    base: ROOT,
                    abandoned: false,
                },
            )]),
            active: MAIN,
            used: Used {
                scopes: BTreeSet::from([DEFAULT_SCOPE]),
                ..Used::default()
            },
            journal: BTreeMap::new(),
        }
    }
}

impl MemoryHistory {
    /// Identités employées, dans l'ordre canonique : notes, relations, contextes, ancrages,
    /// chacune par identifiant croissant.
    pub fn used_ids(&self) -> impl Iterator<Item = UsedId> + '_ {
        let used = &self.used;
        used.nodes
            .iter()
            .map(|id| UsedId::Node(*id))
            .chain(used.edges.iter().map(|id| UsedId::Edge(*id)))
            .chain(used.scopes.iter().map(|id| UsedId::Scope(*id)))
            .chain(used.anchors.iter().map(|id| UsedId::Anchor(*id)))
    }
}

impl History for MemoryHistory {
    fn active(&self) -> Result<BranchId, Error> {
        Ok(self.active)
    }

    fn branches(&self) -> Result<BTreeMap<BranchId, Branch>, Error> {
        Ok(self.branches.clone())
    }

    fn commit_count(&self) -> Result<usize, Error> {
        Ok(self.commits.len())
    }

    fn state(&self, commit: CommitId) -> Result<Arc<Snapshot>, Error> {
        self.commits
            .get(&commit)
            .map(|c| Arc::clone(&c.state))
            .ok_or(Error::MissingCommit(commit))
    }

    fn commit(&self, commit: CommitId) -> Result<Option<CommitInfo>, Error> {
        Ok(self.commits.get(&commit).map(|c| CommitInfo {
            parents: c.parent.into_iter().chain(c.merged).collect(),
            reason: c.reason.clone(),
        }))
    }

    fn operation(&self, op: OpId) -> Result<Option<(Envelope, Outcome)>, Error> {
        Ok(self.journal.get(&op).cloned())
    }

    fn is_used(&self, id: UsedId) -> Result<bool, Error> {
        Ok(self.used.contains(id))
    }

    fn branches_containing(&self, node: NodeId) -> Result<Vec<BranchId>, Error> {
        Ok(self
            .branches
            .iter()
            // L'état courant d'une branche abandonnée n'est pas tenu (VERSION2.md, section 3).
            .filter(|(_, b)| {
                !b.abandoned
                    && self
                        .commits
                        .get(&b.head)
                        .is_some_and(|c| c.state.notes.contains_key(&node))
            })
            .map(|(id, _)| *id)
            .collect())
    }

    fn apply(&mut self, writes: &Writes) -> Result<(), Error> {
        if let Some(commit) = &writes.commit {
            self.commits.insert(
                commit.id,
                Commit {
                    state: Arc::clone(&commit.state),
                    parent: Some(commit.parent),
                    merged: commit.merged,
                    reason: commit.reason.clone(),
                },
            );
        }
        for (id, branch) in &writes.branches {
            self.branches.insert(*id, branch.clone());
        }
        self.active = writes.active;
        for id in &writes.identities {
            self.used.insert(*id);
        }
        self.journal.insert(
            writes.envelope.op,
            (writes.envelope.clone(), writes.outcome.clone()),
        );
        Ok(())
    }
}
