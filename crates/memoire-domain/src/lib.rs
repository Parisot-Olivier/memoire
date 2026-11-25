//! Domaine du logiciel de mémoire de conception — phase 2.
//!
//! Garanties de structure :
//! - aucune dépendance externe ; aucun identifiant n'est généré ici : l'appelant les fournit (D3) ;
//! - toute modification passe par [`Repository::execute`]. La préparation d'une commande ne
//!   reçoit qu'une référence partagée et l'application est infaillible : une commande refusée
//!   ne peut donc rien modifier (INV-TX-01) ;
//! - l'aperçu et l'exécution partagent le même calcul d'effets (INV-PREVIEW-01) ;
//! - une opération identifiée n'est appliquée qu'une fois (INV-RETRY-01).
//!
//! Modèle : contenu partagé entre contextes et révisions (2.3a) ; statuts séparés en 2.3b,
//! relations v0.2 en 2.4, sources et réserves en 2.5, acquittements et ancrages en 2.6.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Erreur de domaine = Result, jamais panique : interdit dans le code livré, autorisé dans les tests.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod command;
mod depot;
mod difference;
mod error;
mod format;
mod fusion;
mod history;
mod id;
mod markers;
mod model;
mod neighborhood;
mod repository;
mod restore;
mod resume;
mod source;

pub use command::UsedId;
pub use command::{Change, Command, Envelope, Expected};
pub use depot::{
    command_name, commit_records, decode_operation_v2, decode_repository, decode_repository_any,
    decode_repository_v2, encode_active, encode_branch_v2, encode_commit_parents_v2,
    encode_commit_v2, encode_identity, encode_operation_v2, encode_repository,
    encode_repository_v2, holds_reference, identity_key, migrate_v1_to_v2, verify_repository_v2,
    Category, CommitBlock, CommitContent, CommitRecord, FragmentChange, Fragments, Verified,
    REPOSITORY_HEADER, REPOSITORY_HEADER_V2,
};
pub use difference::{line_key, state_difference, state_from_lines, state_lines, LineChanges};
pub use error::Error;
pub use format::{
    decode_snapshot, decode_snapshot_v2, encode_snapshot, encode_snapshot_v2, DecodeError,
    DecodeReason, SNAPSHOT_HEADER, SNAPSHOT_HEADER_V2,
};
pub use fusion::{Conflict, MemberField, MergePlan, Resolution, Side, Structural};
pub use history::{CommitInfo, History, MemoryHistory, NewCommit, Prepared, Writes};
pub use id::{
    AnchorId, BranchId, CommitId, EdgeId, NodeId, OpId, ScopeId, DEFAULT_SCOPE, MAIN, ROOT,
};
pub use markers::{
    find_markers, relative_path, AnchorMatch, AnchorReport, AnchorState, AnchoredNote, CodeNotes,
    CodeReport, Focus, FoundMarker, MarkerHit, Occurrence, MARKER_TAG,
};
pub use model::{
    bytes_digest, text_digest, Anchor, CitationView, CitedSource, Decision, Edge, ExchangeView,
    Graph, Implementation, Kind, Node, Part, PassageView, Relation, ReserveStatus, ReserveView,
    Revision, RevisionKind, Snapshot, Verdict, Visibility,
};
pub use neighborhood::{Direction, LinkOrigin, NeighborLink};
pub use repository::{
    parse_marker, Branch, Effects, Outcome, Preview, Redo, Repository, ReviewChange, MARKER_PREFIX,
};
pub use restore::{HistoryEntry, RestoreItem, RestorePlan};
pub use resume::{CodeChange, CodeState, Patch, ResumeEntry, ResumePoint, PATCH_LIMIT};
pub use source::{Excerpt, SourceText};
