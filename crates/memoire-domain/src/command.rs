//! Commandes : seule voie de modification du dépôt.
//!
//! Une [`Change`] produit un nouvel état du réseau ; les autres commandes déplacent une tête de
//! branche ou la branche active. Chaque commande voyage dans une [`Envelope`] qui porte son
//! identifiant d'opération, l'état sur lequel elle a été préparée et son motif.

use std::collections::BTreeSet;

use crate::fusion::Resolution;
use crate::id::{AnchorId, BranchId, CommitId, EdgeId, NodeId, OpId, ScopeId};
use crate::model::{
    Decision, Implementation, Kind, Part, Relation, ReserveStatus, RevisionKind, Verdict,
};
use crate::resume::ResumePoint;

/// Modification de l'état du réseau. Les identifiants des objets créés sont fournis (D3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// Crée une note.
    CreateNode {
        /// Contexte de création.
        scope: ScopeId,
        /// Identité de la nouvelle note.
        id: NodeId,
        /// Rôle.
        kind: Kind,
        /// Titre.
        title: String,
        /// Contenu.
        text: String,
    },
    /// Ajoute une révision au contenu d'une note, partagé par tous les contextes (D2, D5).
    /// Un contenu identique au contenu courant ne crée aucune révision.
    Revise {
        /// Note révisée.
        node: NodeId,
        /// Nouveau contenu.
        text: String,
        /// Nature déclarée de la révision.
        kind: RevisionKind,
    },
    /// Fait apparaître une note existante dans un contexte, sans copie de contenu (D2).
    Include {
        /// Contexte visé.
        scope: ScopeId,
        /// Note incluse.
        node: NodeId,
    },
    /// Crée un contexte à partir d'un autre.
    CloneScope {
        /// Contexte copié.
        source: ScopeId,
        /// Identité du nouveau contexte.
        id: ScopeId,
    },
    /// Crée une relation active.
    Link {
        /// Contexte visé.
        scope: ScopeId,
        /// Identité de la nouvelle relation.
        id: EdgeId,
        /// Origine.
        from: NodeId,
        /// Destination.
        to: NodeId,
        /// Type.
        relation: Relation,
    },
    /// Change l'état de vérification d'une note dans un contexte ; motif obligatoire.
    SetVerdict {
        /// Contexte visé.
        scope: ScopeId,
        /// Note visée.
        node: NodeId,
        /// Nouvel état.
        verdict: Verdict,
    },
    /// Change l'état de décision d'une note dans un contexte ; motif obligatoire.
    /// « Remplacée » est réservé à la relation « remplace » (incrément 2.4).
    SetDecision {
        /// Contexte visé.
        scope: ScopeId,
        /// Note visée.
        node: NodeId,
        /// Nouvel état.
        decision: Decision,
    },
    /// Change l'état d'implémentation d'une note dans un contexte ; motif obligatoire.
    SetImplementation {
        /// Contexte visé.
        scope: ScopeId,
        /// Note visée.
        node: NodeId,
        /// Nouvel état.
        implementation: Implementation,
    },
    /// Retire une note des ressources actuelles.
    Archive {
        /// Contexte visé.
        scope: ScopeId,
        /// Note retirée.
        node: NodeId,
    },
    /// Désactive une relation et en crée une vers une nouvelle destination.
    Reattach {
        /// Contexte visé.
        scope: ScopeId,
        /// Relation désactivée.
        edge: EdgeId,
        /// Nouvelle destination.
        to: NodeId,
        /// Identité de la nouvelle relation.
        id: EdgeId,
    },
    /// L'origine remplace la destination, qui passe « remplacée » dans la même opération ;
    /// motif obligatoire (D12). La note remplacée reste résoluble (INV-MOD-06).
    Replace {
        /// Contexte visé.
        scope: ScopeId,
        /// Identité de la nouvelle relation.
        id: EdgeId,
        /// Note qui remplace.
        replacement: NodeId,
        /// Note remplacée.
        replaced: NodeId,
    },
    /// L'origine restreint le périmètre de la destination à une condition, sans verdict ;
    /// les dépendants de la destination sont à réexaminer (INV-REL-06).
    Restrict {
        /// Contexte visé.
        scope: ScopeId,
        /// Identité de la nouvelle relation.
        id: EdgeId,
        /// Passage ou note qui restreint.
        from: NodeId,
        /// Note dont le périmètre est restreint.
        to: NodeId,
        /// Condition d'applicabilité, non vide.
        condition: String,
    },
    /// L'origine contredit la destination, avec justification ; aucune propagation (INV-REL-07).
    Contradict {
        /// Contexte visé.
        scope: ScopeId,
        /// Identité de la nouvelle relation.
        id: EdgeId,
        /// Note qui contredit.
        from: NodeId,
        /// Note contredite.
        to: NodeId,
        /// Justification du désaccord, non vide.
        justification: String,
    },
    /// Crée un échange à partir de sources séparées explicitement par l'utilisateur ; aucune
    /// frontière n'est déduite (INV-SOURCE-04). Sans question, l'échange est incomplet.
    CreateExchange {
        /// Contexte de création.
        scope: ScopeId,
        /// Identité du nouvel échange.
        id: NodeId,
        /// Titre de la conversation.
        title: String,
        /// Question, telle que collée, si l'utilisateur l'a fournie.
        question: Option<String>,
        /// Réponse, telle que collée.
        answer: String,
        /// Échange qui précède dans la conversation, donné explicitement (CA-01).
        previous: Option<NodeId>,
    },
    /// Complète un échange incomplet ; une question déjà présente ne se remplace pas.
    SetQuestion {
        /// Échange visé.
        exchange: NodeId,
        /// Question, telle que collée.
        question: String,
    },
    /// Crée un passage : plage exacte d'une partie d'un échange, en octets UTF-8.
    CreatePassage {
        /// Contexte de création ; l'échange doit y figurer.
        scope: ScopeId,
        /// Identité du nouveau passage.
        id: NodeId,
        /// Échange d'origine.
        exchange: NodeId,
        /// Partie de l'échange.
        part: Part,
        /// Début, en octets.
        start: usize,
        /// Fin exclusive, en octets.
        end: usize,
    },
    /// Cite une note (D18, D24) : plage exacte d'une version d'un raisonnement, d'une
    /// implémentation ou d'une note personnelle, en octets UTF-8. La citation garde cette version
    /// même si la note est révisée ensuite (INV-CIT-01).
    CiteNote {
        /// Contexte de création ; la note citée doit y figurer.
        scope: ScopeId,
        /// Identité de la nouvelle citation.
        id: NodeId,
        /// Note citée.
        note: NodeId,
        /// Version citée : 0 pour le texte de création, *k* pour la *k*-ième révision.
        version: usize,
        /// Début, en octets.
        start: usize,
        /// Fin exclusive, en octets.
        end: usize,
    },
    /// Crée une réserve depuis un passage, sans destination obligatoire (INV-RES-01).
    CreateReserve {
        /// Contexte de création ; le passage doit y figurer.
        scope: ScopeId,
        /// Identité de la nouvelle réserve.
        id: NodeId,
        /// Passage d'origine.
        passage: NodeId,
        /// Intitulé de la réserve.
        title: String,
        /// Cible prévue en texte, quand elle n'existe pas encore (INV-RES-04).
        target: Option<String>,
    },
    /// Une note propose de traiter une réserve, qui passe « traitée à vérifier » ; motif exigé.
    Treat {
        /// Contexte visé.
        scope: ScopeId,
        /// Identité de la nouvelle relation.
        id: EdgeId,
        /// Note qui traite.
        note: NodeId,
        /// Réserve traitée.
        reserve: NodeId,
    },
    /// Change le cycle de vie d'une réserve dans un contexte ; motif exigé (INV-RES-02).
    SetReserveStatus {
        /// Contexte visé.
        scope: ScopeId,
        /// Réserve visée.
        reserve: NodeId,
        /// Nouvel état.
        status: ReserveStatus,
    },
    /// Confirme qu'un dépendant reste pertinent malgré une cause de réexamen (INV-ACK-01) ;
    /// motif exigé. Aucune relation n'est supprimée ; la confirmation devient caduque si le sens
    /// du dépendant ou l'état de la cause change.
    Acknowledge {
        /// Contexte visé.
        scope: ScopeId,
        /// Note confirmée.
        dependent: NodeId,
        /// Cause acquittée.
        cause: NodeId,
    },
    /// Retire une relation active ; elle reste dans l'historique, inactive. Le statut posé par
    /// un remplacement ou un traitement est rétabli. Motif exigé.
    Detach {
        /// Contexte visé.
        scope: ScopeId,
        /// Relation retirée.
        edge: EdgeId,
    },
    /// Ancre une note dans le code : chemin, copie du texte ciblé et commit Git s'il existe
    /// (INV-CODE-03). Le logiciel n'écrit jamais dans le code (D9, INV-CODE-04).
    AnchorToCode {
        /// Identité du nouvel ancrage.
        id: AnchorId,
        /// Note ancrée.
        node: NodeId,
        /// Chemin du fichier, tel que connu au moment du lien.
        path: String,
        /// Copie du texte ciblé, non vide.
        excerpt: String,
        /// Commit du code au moment du lien, si le code est versionné.
        git: Option<String>,
    },
    /// Retire un ancrage ; il reste dans l'historique, inactif. Motif exigé.
    Unanchor {
        /// Ancrage retiré.
        anchor: AnchorId,
    },
    /// Publie de nouveau l'état d'un commit conservé.
    Restore {
        /// Commit dont l'état est repris.
        target: CommitId,
    },
    /// Restauration ciblée (RESTAURATION.md, section 2) : les notes choisies reprennent leur état
    /// au commit cible, avec leurs relations internes, leurs ancrages et leurs acquittements ;
    /// le reste du réseau n'est pas touché (INV-RESTORE-01, INV-RESTORE-02).
    RestoreNodes {
        /// Commit dont l'état est repris pour la sélection.
        target: CommitId,
        /// Notes choisies.
        nodes: BTreeSet<NodeId>,
    },
}

/// Identité employée par un objet du dépôt : elle ne sera jamais réutilisée (INV-ID-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UsedId {
    /// Identité d'une note.
    Node(NodeId),
    /// Identité d'une relation.
    Edge(EdgeId),
    /// Identité d'un contexte.
    Scope(ScopeId),
    /// Identité d'un ancrage.
    Anchor(AnchorId),
}

impl UsedId {
    pub(crate) fn is_nil(self) -> bool {
        match self {
            Self::Node(id) => id.is_nil(),
            Self::Edge(id) => id.is_nil(),
            Self::Scope(id) => id.is_nil(),
            Self::Anchor(id) => id.is_nil(),
        }
    }
}

impl Change {
    /// Identité créée par cette modification ; chaque modification en crée au plus une.
    pub(crate) fn new_id(&self) -> Option<UsedId> {
        match self {
            Self::CreateNode { id, .. }
            | Self::CreateExchange { id, .. }
            | Self::CreatePassage { id, .. }
            | Self::CiteNote { id, .. }
            | Self::CreateReserve { id, .. } => Some(UsedId::Node(*id)),
            Self::Link { id, .. }
            | Self::Reattach { id, .. }
            | Self::Replace { id, .. }
            | Self::Restrict { id, .. }
            | Self::Contradict { id, .. }
            | Self::Treat { id, .. } => Some(UsedId::Edge(*id)),
            Self::CloneScope { id, .. } => Some(UsedId::Scope(*id)),
            Self::AnchorToCode { id, .. } => Some(UsedId::Anchor(*id)),
            Self::Revise { .. }
            | Self::Include { .. }
            | Self::SetQuestion { .. }
            | Self::SetReserveStatus { .. }
            | Self::Acknowledge { .. }
            | Self::Detach { .. }
            | Self::Unanchor { .. }
            | Self::SetVerdict { .. }
            | Self::SetDecision { .. }
            | Self::SetImplementation { .. }
            | Self::Archive { .. }
            | Self::Restore { .. }
            | Self::RestoreNodes { .. } => None,
        }
    }
}

impl Change {
    /// Vrai pour un changement de statut, qui exige un motif non vide (INV-MOD-05, D12).
    pub(crate) fn requires_reason(&self) -> bool {
        matches!(
            self,
            Self::SetVerdict { .. }
                | Self::SetDecision { .. }
                | Self::SetImplementation { .. }
                | Self::Replace { .. }
                | Self::Treat { .. }
                | Self::SetReserveStatus { .. }
                | Self::Acknowledge { .. }
                | Self::Detach { .. }
                | Self::Unanchor { .. }
        )
    }
}

/// Commande adressée au dépôt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Modification du réseau.
    Change(Change),
    /// Ramène la branche active sur le parent de sa tête.
    Undo,
    /// Réapplique la dernière opération annulée sur la branche active.
    Redo,
    /// Change de branche active.
    Checkout {
        /// Branche à consulter.
        branch: BranchId,
    },
    /// Opération groupée (LOT.md, D22) : modifications appliquées dans l'ordre, chacune sur
    /// l'état laissé par les précédentes, toutes validées avant toute écriture ; un seul commit,
    /// une seule opération. Ni vide, ni restauration.
    Lot(Vec<Change>),
    /// Abandonne une branche (D19, D23) : rien n'est supprimé ; si elle est active, la branche
    /// active revient sur son origine. Aucun commit ; `annuler` ne le défait pas.
    Abandon {
        /// Branche abandonnée ; jamais main.
        branch: BranchId,
    },
    /// Réactive une branche abandonnée, sans la rendre active.
    Reactivate {
        /// Branche réactivée.
        branch: BranchId,
    },
    /// Fusionne une branche dans la branche active (FUSION.md ; D19, D23) : un commit à deux
    /// parents, jamais de bifurcation ; refusée s'il reste un conflit, ou si la source ou la
    /// cible ont bougé depuis le plan.
    Merge {
        /// Branche fusionnée ; elle n'est pas modifiée.
        source: BranchId,
        /// Tête de la source sur laquelle le plan a été calculé.
        source_head: CommitId,
        /// Résolutions, repérées par clé.
        resolutions: Vec<Resolution>,
    },
    /// Point de reprise (FICHE-REPRISE.md, D26) : publie un commit d'état identique à son
    /// parent, jamais de bifurcation ; ni dans un lot, ni sans objectif ou prochaine action.
    Resume(Box<ResumePoint>),
}

/// État sur lequel une commande a été préparée : branche active et tête de cette branche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    /// Branche active.
    pub branch: BranchId,
    /// Tête de la branche active.
    pub head: CommitId,
}

/// Commande identifiée, prête à exécuter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// Identifiant d'opération, fourni par l'appelant ; il identifie aussi le commit publié.
    pub op: OpId,
    /// État sur lequel la commande a été préparée (souvent la base d'un aperçu).
    pub expected: Expected,
    /// Motif, conservé avec le commit publié ; peut être vide.
    pub reason: String,
    /// Commande.
    pub command: Command,
}
