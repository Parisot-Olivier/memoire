//! Erreurs récupérables du domaine. Une erreur n'a jamais d'effet sur le dépôt (INV-TX-01).
//! Les variantes sans contexte sont conservées telles que dans le prototype pour que le contrat
//! porté reste textuellement identique ; leur enrichissement sera un changement de contrat explicite.

use std::fmt;

use crate::command::Expected;
use crate::id::{AnchorId, BranchId, CommitId, EdgeId, NodeId, OpId, ScopeId};

/// Refus d'une commande ou d'une lecture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Note absente du contexte consulté.
    MissingNode(NodeId),
    /// Relation absente du contexte consulté.
    MissingEdge(EdgeId),
    /// Contexte absent de l'état consulté.
    MissingScope(ScopeId),
    /// Commit inconnu du dépôt.
    MissingCommit(CommitId),
    /// Branche inconnue du dépôt.
    MissingBranch(BranchId),
    /// Le rôle de la note ne porte pas ce verdict ou ce statut : une catégorie n'en porte aucun
    /// (INV-SEM-01), un échange non plus, une réserve seulement son cycle de vie.
    CategoryHasNoVerdict,
    /// Une note qui n'est pas une catégorie doit porter un verdict et ses statuts.
    MissingVerdict(NodeId),
    /// Un changement de statut exige un motif non vide (INV-MOD-05).
    MissingReason,
    /// Statut réservé à une relation : « remplacée » est posé par « remplace » (incrément 2.4).
    ReservedStatus,
    /// Relation interdite entre ces rôles, ou réservée à une commande dédiée.
    InvalidRelation,
    /// Relation d'une note vers elle-même (INV-REL-01).
    SelfRelation,
    /// Relation active identique déjà présente : même origine, destination et type (INV-REL-01).
    DuplicateRelation,
    /// Cycle de remplacement ou de catégories (INV-REL-02, INV-REL-03).
    CyclicRelation,
    /// Condition de restriction ou justification de contradiction vide (INV-REL-06, INV-REL-07).
    MissingAnnotation,
    /// L'objet n'a pas le rôle attendu par la commande (échange, passage ou réserve).
    WrongKind(NodeId),
    /// Source d'échange ou passage : contenu immuable, aucune révision ni remplacement de source.
    SourceImmutable(NodeId),
    /// Question ou réponse vide.
    EmptySource,
    /// La note désignée n'est pas, dans ce contexte, une cause de réexamen du dépendant.
    NoSuchCause,
    /// Ancrage inconnu du dépôt.
    MissingAnchor(AnchorId),
    /// Identifiant d'ancrage déjà employé dans ce dépôt.
    AnchorIdInUse(AnchorId),
    /// Ancrage incomplet : chemin, copie du code ou commit vide, ou empreinte incohérente.
    InvalidAnchor,
    /// Le texte fourni n'est pas un repère canonique.
    NotAMarker,
    /// Relation déjà désactivée.
    InactiveEdge,
    /// Aucune opération à annuler sur la branche active.
    NothingToUndo,
    /// Aucune opération à rétablir sur la branche active.
    NothingToRedo,
    /// Pile de rétablissement incohérente avec la tête courante (garde défensive).
    StaleHead,
    /// Plage d'extrait vide, hors limites ou coupant un caractère UTF-8.
    InvalidExcerpt,
    /// Identifiant nul fourni : il est réservé aux objets initiaux.
    NilIdentifier,
    /// Un contexte porte un contenu de note différent du contenu canonique (INV-SCOPE-02).
    DivergentContent(NodeId),
    /// Identifiant de note déjà employé dans ce dépôt, y compris par un état abandonné (INV-ID-02).
    NodeIdInUse(NodeId),
    /// Identifiant de relation déjà employé dans ce dépôt.
    EdgeIdInUse(EdgeId),
    /// Identifiant de contexte déjà employé dans ce dépôt.
    ScopeIdInUse(ScopeId),
    /// Identifiant d'opération déjà employé pour une autre commande (INV-RETRY-01).
    OpIdInUse(OpId),
    /// L'état attendu par la commande n'est plus l'état courant (INV-PREVIEW-01).
    Stale {
        /// État sur lequel la commande a été préparée.
        expected: Expected,
        /// État courant au moment de l'exécution.
        actual: Expected,
    },
    /// Lot sans élément (LOT.md, section 2).
    EmptyLot,
    /// Restauration dans un lot : elle remplace l'état entier (LOT.md, section 2).
    NotInLot,
    /// Refus d'un élément d'un lot, qui refuse tout le lot.
    InLot {
        /// Rang de l'élément refusé, à partir de 1.
        element: usize,
        /// Refus de l'élément, tel qu'il serait seul.
        cause: Box<Error>,
    },
    /// main ne s'abandonne pas (FUSION.md, section 9.7).
    MainNotAbandonable,
    /// Branche abandonnée : ni active ni source d'une fusion tant qu'elle n'est pas réactivée
    /// (D23), et pas abandonnée une seconde fois.
    BranchAbandoned(BranchId),
    /// Seule une branche abandonnée se réactive.
    BranchNotAbandoned(BranchId),
    /// Restauration ciblée sans note choisie.
    EmptySelection,
    /// Note choisie qui n'existait pas au commit à restaurer.
    AbsentFromTarget(NodeId),
    /// Champ obligatoire vide dans un point de reprise : objectif ou prochaine action.
    EmptyResumeField(&'static str),
    /// Le commit désigné n'a pas été publié par un point de reprise.
    NotAResumePoint(CommitId),
    /// Fusion de la branche active dans elle-même.
    MergeIntoItself,
    /// Plusieurs ancêtres communs les plus proches (fusions croisées) : la fusion est refusée
    /// plutôt que d'en choisir un (D23).
    MultipleMergeBases,
    /// La source a bougé depuis le plan de fusion (D23).
    StaleSource {
        /// Tête de la source sur laquelle le plan a été calculé.
        expected: CommitId,
        /// Tête actuelle de la source.
        actual: CommitId,
    },
    /// Conflits de fusion sans résolution, en nombre.
    UnresolvedConflicts(usize),
    /// Résolution (rang à partir de 1) qui ne correspond à aucun conflit, ou désactivation d'une
    /// relation qui n'est pas candidate.
    UnmatchedResolution(usize),
    /// Résolution (rang à partir de 1) d'une clé déjà résolue.
    DuplicateResolution(usize),
    /// L'état réuni viole une règle que chaque côté respectait (INV-MERGE-03).
    StructuralConflict(Box<Error>),
    /// Une relation ou un ancrage diverge des deux côtés sans que la base l'explique : historique
    /// modifié hors du logiciel.
    IncoherentMerge,
    /// L'historique du dépôt n'a pu être lu ou écrit (disque, base illisible) ; message du
    /// stockage. Ce n'est pas un refus métier (ETAT-HISTORIQUE.md, section 4).
    Historique(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(id) => write!(f, "note absente : {id}"),
            Self::MissingEdge(id) => write!(f, "relation absente : {id}"),
            Self::MissingScope(id) => write!(f, "contexte absent : {id}"),
            Self::MissingCommit(id) => write!(f, "commit inconnu : {id}"),
            Self::MissingBranch(id) => write!(f, "branche inconnue : {id}"),
            Self::CategoryHasNoVerdict => {
                f.write_str("une catégorie ne porte ni verdict ni statut")
            }
            Self::MissingVerdict(id) => write!(f, "verdict ou statut manquant pour la note {id}"),
            Self::MissingReason => f.write_str("motif obligatoire pour un changement de statut"),
            Self::ReservedStatus => f.write_str("statut réservé à la relation « remplace »"),
            Self::InvalidRelation => f.write_str("relation interdite entre ces rôles"),
            Self::SelfRelation => f.write_str("relation d'une note vers elle-même"),
            Self::DuplicateRelation => f.write_str("relation active identique déjà présente"),
            Self::CyclicRelation => f.write_str("cycle de remplacement ou de catégories"),
            Self::MissingAnnotation => f.write_str("condition ou justification vide"),
            Self::WrongKind(id) => write!(f, "rôle inattendu pour {id}"),
            Self::SourceImmutable(id) => write!(f, "source immuable : {id}"),
            Self::EmptySource => f.write_str("question ou réponse vide"),
            Self::NoSuchCause => f.write_str("pas une cause de réexamen de ce dépendant"),
            Self::MissingAnchor(id) => write!(f, "ancrage inconnu : {id}"),
            Self::AnchorIdInUse(id) => write!(f, "identifiant d'ancrage déjà employé : {id}"),
            Self::InvalidAnchor => f.write_str("ancrage incomplet ou incohérent"),
            Self::NotAMarker => f.write_str("texte qui n'est pas un repère canonique"),
            Self::InactiveEdge => f.write_str("relation déjà inactive"),
            Self::NothingToUndo => f.write_str("rien à annuler"),
            Self::NothingToRedo => f.write_str("rien à rétablir"),
            Self::StaleHead => f.write_str("pile de rétablissement incohérente"),
            Self::InvalidExcerpt => f.write_str("plage d'extrait invalide"),
            Self::NilIdentifier => f.write_str("identifiant nul refusé"),
            Self::DivergentContent(id) => write!(f, "contenu divergent pour la note {id}"),
            Self::NodeIdInUse(id) => write!(f, "identifiant de note déjà employé : {id}"),
            Self::EdgeIdInUse(id) => write!(f, "identifiant de relation déjà employé : {id}"),
            Self::ScopeIdInUse(id) => write!(f, "identifiant de contexte déjà employé : {id}"),
            Self::OpIdInUse(id) => write!(f, "identifiant d'opération déjà employé : {id}"),
            Self::EmptyLot => f.write_str("lot vide"),
            Self::NotInLot => f.write_str("une restauration ne peut pas faire partie d'un lot"),
            Self::InLot { element, cause } => write!(f, "élément {element} du lot : {cause}"),
            Self::MainNotAbandonable => f.write_str("main ne peut pas être abandonnée"),
            Self::BranchAbandoned(id) => write!(f, "branche abandonnée : {id}"),
            Self::BranchNotAbandoned(id) => write!(f, "branche non abandonnée : {id}"),
            Self::EmptySelection => f.write_str("sélection vide"),
            Self::AbsentFromTarget(id) => write!(f, "note absente de l'état à restaurer : {id}"),
            Self::EmptyResumeField(field) => {
                write!(f, "champ obligatoire vide dans le point de reprise : {field}")
            }
            Self::NotAResumePoint(id) => write!(f, "pas un point de reprise : {id}"),
            Self::MergeIntoItself => f.write_str("une branche ne se fusionne pas dans elle-même"),
            Self::MultipleMergeBases => f.write_str("plusieurs bases communes : fusion refusée"),
            Self::StaleSource { expected, actual } => write!(
                f,
                "plan de fusion périmé : source attendue sur {expected}, tête actuelle {actual}"
            ),
            Self::UnresolvedConflicts(n) => write!(f, "{n} conflit(s) non résolu(s)"),
            Self::UnmatchedResolution(k) => {
                write!(f, "la résolution {k} ne correspond à aucun conflit")
            }
            Self::DuplicateResolution(k) => {
                write!(f, "la résolution {k} répète une clé déjà résolue")
            }
            Self::StructuralConflict(cause) => write!(f, "conflit structurel : {cause}"),
            Self::IncoherentMerge => f.write_str(
                "fusion impossible : une relation ou un ancrage diverge sans que la base l'explique",
            ),
            Self::Historique(cause) => write!(f, "historique indisponible : {cause}"),
            Self::Stale { expected, actual } => write!(
                f,
                "état périmé : attendu {} sur {}, courant {} sur {}",
                expected.head, expected.branch, actual.head, actual.branch
            ),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_object_concerned() {
        let id = NodeId::from_bytes([0xab; 16]);
        assert_eq!(
            Error::MissingNode(id).to_string(),
            "note absente : abababab-abab-abab-abab-abababababab"
        );
        assert_eq!(Error::NothingToUndo.to_string(), "rien à annuler");
    }
}
