//! Fiche de reprise (FICHE-REPRISE.md, sections 2 et 3 ; D26 ; INV-RESUME-01, INV-RESUME-02).
//!
//! Une fiche est une opération du journal qui publie un commit d'état identique à son parent :
//! ce commit est son identité et son contexte historique. L'état du code est une donnée fournie
//! par l'appelant, qui seul lit Git ; sa comparaison avec l'état actuel est une règle d'ici.

use std::collections::BTreeSet;

use crate::error::Error;
use crate::id::{CommitId, NodeId};
use crate::model::{bytes_digest, Snapshot};

/// Taille au-delà de laquelle le travail non enregistré n'est conservé que par son empreinte et
/// sa taille (FICHE-REPRISE.md, section 4).
pub const PATCH_LIMIT: usize = 4 * 1024 * 1024;

/// Travail non enregistré dans un commit, noté avec la fiche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Patch {
    /// Patch conservé tel quel : en UTF-8, au plus [`PATCH_LIMIT`] octets.
    Kept(String),
    /// Patch trop grand ou hors UTF-8 : seules sa taille et son empreinte sont conservées,
    /// plutôt qu'un texte corrompu.
    NotKept {
        /// Taille en octets.
        bytes: usize,
        /// Empreinte de ses octets ([`bytes_digest`]).
        digest: u64,
    },
}

impl Patch {
    /// Patch conservé s'il est en UTF-8 et d'au plus [`PATCH_LIMIT`] octets ; sinon sa taille et
    /// son empreinte.
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        match std::str::from_utf8(bytes) {
            Ok(text) if bytes.len() <= PATCH_LIMIT => Self::Kept(text.to_owned()),
            _ => Self::NotKept {
                bytes: bytes.len(),
                digest: bytes_digest(bytes),
            },
        }
    }
}

/// État du code au moment d'une fiche ou d'une lecture : commit, branche, lignes d'état de Git
/// (triées, sans doublon) et travail non enregistré.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeState {
    pub(crate) commit: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) changes: Vec<String>,
    pub(crate) patch: Patch,
}

impl CodeState {
    /// État du code ; les lignes d'état sont triées et dédoublonnées, le patch réduit à son
    /// empreinte s'il ne peut être conservé ([`Patch::new`]).
    #[must_use]
    pub fn new(
        commit: Option<String>,
        branch: Option<String>,
        changes: Vec<String>,
        diff: &[u8],
    ) -> Self {
        let changes: BTreeSet<String> = changes.into_iter().collect();
        Self {
            commit,
            branch,
            changes: changes.into_iter().collect(),
            patch: Patch::new(diff),
        }
    }

    /// Commit de la tête, s'il y en a un.
    #[must_use]
    pub fn commit(&self) -> Option<&str> {
        self.commit.as_deref()
    }

    /// Branche, sauf tête détachée.
    #[must_use]
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// Lignes d'état des fichiers changés, triées.
    #[must_use]
    pub fn changes(&self) -> &[String] {
        &self.changes
    }

    /// Travail non enregistré.
    #[must_use]
    pub fn patch(&self) -> &Patch {
        &self.patch
    }
}

/// Changement du code depuis une fiche (INV-RESUME-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeChange {
    /// La fiche n'a pas noté l'état du code.
    NotRecorded,
    /// L'état du code n'est pas disponible aujourd'hui (pas de dépôt Git).
    Unavailable,
    /// Comparaison : commit différent, travail non enregistré différent. Un simple changement
    /// de nom de branche n'est pas un changement du code.
    Compared {
        /// Le commit de la tête a changé.
        commit: bool,
        /// Les fichiers changés ou le travail non enregistré ont changé.
        work: bool,
    },
}

/// Point de reprise (FICHE-REPRISE.md, section 2). Objectif et prochaine action sont
/// obligatoires ; un champ facultatif vide est absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResumePoint {
    /// Date fournie par l'appelant (UTC, à la minute) ; le noyau ne lit pas l'horloge (D3).
    pub date: String,
    /// Objectif en cours.
    pub objective: String,
    /// Prochaine action concrète.
    pub next_action: String,
    /// Question précise en cours, blocage.
    pub question: String,
    /// État réellement atteint.
    pub reached: String,
    /// Dernier résultat de compilation, d'exécution ou de test.
    pub result: String,
    /// Hypothèse actuelle et incertitudes.
    pub hypothesis: String,
    /// Fichiers et fonctions ouverts, en texte libre.
    pub open: String,
    /// Notes ou citations ouvertes.
    pub notes: BTreeSet<NodeId>,
    /// État du code au moment de la fiche.
    pub code: Option<CodeState>,
}

impl ResumePoint {
    /// Changement du code depuis la fiche, d'après l'état actuel (`None` : indisponible).
    #[must_use]
    pub fn code_change(&self, now: Option<&CodeState>) -> CodeChange {
        let Some(then) = &self.code else {
            return CodeChange::NotRecorded;
        };
        let Some(now) = now else {
            return CodeChange::Unavailable;
        };
        CodeChange::Compared {
            commit: then.commit != now.commit,
            work: then.changes != now.changes || then.patch != now.patch,
        }
    }

    /// Champs obligatoires non vides ; chaque note existe dans `state`.
    pub(crate) fn validate(&self, state: &Snapshot) -> Result<(), Error> {
        if self.objective.trim().is_empty() {
            return Err(Error::EmptyResumeField("objectif"));
        }
        if self.next_action.trim().is_empty() {
            return Err(Error::EmptyResumeField("prochaine action"));
        }
        for &node in &self.notes {
            state.content(node)?;
        }
        Ok(())
    }
}

/// Fiche d'une branche, avec le commit qui l'identifie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumeEntry {
    /// Commit de la fiche.
    pub commit: CommitId,
    /// Fiche.
    pub point: ResumePoint,
}
