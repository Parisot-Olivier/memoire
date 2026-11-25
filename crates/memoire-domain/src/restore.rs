//! Restauration ciblée et historique (RESTAURATION.md, sections 2 et 3 ; INV-RESTORE-01,
//! INV-RESTORE-02, INV-RET-01, INV-RET-03).
//!
//! Les notes choisies reprennent leur état au commit cible, avec leurs relations internes (entre
//! deux notes choisies), leurs ancrages et les acquittements dont elles sont le dépendant. Rien
//! d'autre ne change : une relation entre une note choisie et une note qui ne l'est pas est du
//! travail indépendant, jamais touché, seulement signalé dans le plan quand elle a changé depuis.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::command::Command;
use crate::error::Error;
use crate::fusion::MemberField;
use crate::id::{AnchorId, CommitId, EdgeId, NodeId, ScopeId};
use crate::model::{Node, Snapshot};
use crate::repository::Effects;

/// Changement que rétablira une restauration, ou relation extérieure qu'elle conservera.
/// L'ordre est celui du plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RestoreItem {
    /// Contenu d'une note : la version de la cible remplace la version courante.
    Text {
        /// Note.
        node: NodeId,
        /// Version rétablie (nombre de révisions à la cible).
        target_version: usize,
        /// Version remplacée (nombre de révisions courant).
        current_version: usize,
    },
    /// Statut d'une note dans un contexte.
    Field {
        /// Contexte.
        scope: ScopeId,
        /// Note.
        node: NodeId,
        /// Champ rétabli.
        field: MemberField,
    },
    /// Relation rétablie, ou désactivée si elle a été créée depuis la cible.
    Relation {
        /// Contexte.
        scope: ScopeId,
        /// Relation.
        edge: EdgeId,
    },
    /// Ancrage rétabli, ou retiré s'il a été créé depuis la cible.
    Anchor {
        /// Ancrage.
        anchor: AnchorId,
    },
    /// Acquittement rétabli, présence comprise.
    Ack {
        /// Contexte.
        scope: ScopeId,
        /// Note dépendante.
        dependent: NodeId,
        /// Cause.
        cause: NodeId,
    },
    /// Relation entre une note choisie et une note qui ne l'est pas, différente depuis la cible :
    /// conservée telle quelle (INV-RESTORE-01).
    Kept {
        /// Contexte.
        scope: ScopeId,
        /// Relation.
        edge: EdgeId,
    },
}

/// Plan d'une restauration, exactement ce qu'elle appliquera.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestorePlan {
    /// Commit restauré.
    pub target: CommitId,
    /// Changements et relations conservées, dans l'ordre.
    pub items: Vec<RestoreItem>,
    /// Effets sur les causes de réexamen.
    pub effects: Effects,
}

/// Commit de l'historique d'une branche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    /// Commit.
    pub commit: CommitId,
    /// Motif enregistré.
    pub reason: String,
    /// Commande qui l'a publié ; aucune pour la racine.
    pub command: Option<Command>,
}

/// État où les notes choisies reprennent leur état dans `old` (section 2 du document).
pub(crate) fn restored_nodes(
    now: &Snapshot,
    old: &Snapshot,
    nodes: &BTreeSet<NodeId>,
) -> Result<Snapshot, Error> {
    if nodes.is_empty() {
        return Err(Error::EmptySelection);
    }
    let mut state = now.clone();
    for &node in nodes {
        now.content(node)?;
        let content = old.notes.get(&node).ok_or(Error::AbsentFromTarget(node))?;
        state.notes.insert(node, Arc::clone(content));
    }
    let notes = state.notes.clone();
    for (scope, graph) in &mut state.scopes {
        let before = old.scopes.get(scope);
        for node in nodes {
            if let Some(member) = before.and_then(|g| g.nodes.get(node)) {
                graph.nodes.insert(*node, member.clone());
            }
        }
        for (id, edge) in &mut graph.edges {
            if nodes.contains(&edge.from) && nodes.contains(&edge.to) {
                match before.and_then(|g| g.edges.get(id)) {
                    Some(then) => *edge = then.clone(),
                    None => edge.active = false,
                }
            }
        }
        graph
            .acks
            .retain(|(dependent, _), _| !nodes.contains(dependent));
        for (key, ack) in before.map(|g| &g.acks).into_iter().flatten() {
            if nodes.contains(&key.0) {
                graph.acks.insert(*key, *ack);
            }
        }
        // Chaque membre porte le contenu canonique de sa note (INV-SCOPE-02).
        for (id, member) in &mut graph.nodes {
            if let Some(content) = notes.get(id) {
                member.content = Arc::clone(content);
            }
        }
    }
    for (id, anchor) in &mut state.anchors {
        if nodes.contains(&anchor.node) {
            match old.anchors.get(id) {
                Some(then) => *anchor = then.clone(),
                None => anchor.active = false,
            }
        }
    }
    Ok(state)
}

/// Champs d'un membre qui diffèrent entre deux états.
fn fields(now: &Node, then: &Node) -> Vec<MemberField> {
    [
        (MemberField::Verdict, now.verdict != then.verdict),
        (MemberField::Decision, now.decision != then.decision),
        (
            MemberField::Implementation,
            now.implementation != then.implementation,
        ),
        (MemberField::Reserve, now.reserve != then.reserve),
        (MemberField::Visibility, now.visibility != then.visibility),
    ]
    .into_iter()
    .filter(|(_, differs)| *differs)
    .map(|(field, _)| field)
    .collect()
}

/// Plan : ce qui diffère entre l'état courant et l'état restauré, puis, pour une restauration
/// ciblée, les relations extérieures différentes depuis la cible, conservées.
pub(crate) fn items(
    now: &Snapshot,
    old: &Snapshot,
    restored: &Snapshot,
    nodes: Option<&BTreeSet<NodeId>>,
) -> Vec<RestoreItem> {
    let mut items = Vec::new();
    for (&node, content) in &restored.notes {
        if let Some(current) = now.notes.get(&node).filter(|c| *c != content) {
            items.push(RestoreItem::Text {
                node,
                target_version: content.revisions.len(),
                current_version: current.revisions.len(),
            });
        }
    }
    for (&scope, graph) in &restored.scopes {
        let Some(current) = now.scopes.get(&scope) else {
            continue;
        };
        for (&node, member) in &graph.nodes {
            if let Some(was) = current.nodes.get(&node) {
                for field in fields(was, member) {
                    items.push(RestoreItem::Field { scope, node, field });
                }
            }
        }
        for (&edge, value) in &graph.edges {
            if current.edges.get(&edge) != Some(value) {
                items.push(RestoreItem::Relation { scope, edge });
            }
        }
        let keys: BTreeSet<&(NodeId, NodeId)> =
            graph.acks.keys().chain(current.acks.keys()).collect();
        for &&(dependent, cause) in &keys {
            if graph.acks.get(&(dependent, cause)) != current.acks.get(&(dependent, cause)) {
                items.push(RestoreItem::Ack {
                    scope,
                    dependent,
                    cause,
                });
            }
        }
        if let Some(nodes) = nodes {
            let then = old.scopes.get(&scope).map(|g| &g.edges);
            let empty = BTreeMap::new();
            let then = then.unwrap_or(&empty);
            for (&edge, value) in &current.edges {
                let outside = nodes.contains(&value.from) != nodes.contains(&value.to);
                if outside && then.get(&edge) != Some(value) {
                    items.push(RestoreItem::Kept { scope, edge });
                }
            }
        }
    }
    for (&anchor, value) in &restored.anchors {
        if now.anchors.get(&anchor) != Some(value) {
            items.push(RestoreItem::Anchor { anchor });
        }
    }
    items.sort_unstable();
    items
}
