//! Fusion à trois voies (FUSION.md, sections 3 à 5 et 9.3 à 9.5 ; D19, D23).
//!
//! La réunion compare, pour chaque clé et chaque champ, la base commune (B), la cible (C) et la
//! source (S) : C = S, on garde ; C = B, on prend S ; S = B, on garde C ; sinon, conflit. Rien
//! ne disparaît (D6) : un objet présent d'un seul côté est repris tel quel ; seule la présence d'un
//! acquittement se compare comme un champ, une restauration pouvant l'avoir retiré.
//!
//! Les relations et les ancrages n'ont que leur activité de modifiable, à deux valeurs, et tout
//! objet présent des deux côtés est dans la base unique : aucun conflit n'y est possible. Un écart
//! (base modifiée hors du logiciel) refuse la fusion ([`Error::IncoherentMerge`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::error::Error;
use crate::id::{CommitId, EdgeId, NodeId, ScopeId};
use crate::model::{Ack, Edge, Graph, Node, Snapshot};
use crate::repository::Effects;

/// Côté retenu pour résoudre un conflit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    /// La branche active, dans laquelle on fusionne.
    Target,
    /// La branche fusionnée.
    Source,
}

/// Champ d'un membre de contexte comparé séparément.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberField {
    /// État de vérification.
    Verdict,
    /// État de décision.
    Decision,
    /// État d'implémentation.
    Implementation,
    /// État d'une réserve.
    Reserve,
    /// Ressources actuelles ou archivée.
    Visibility,
}

/// Conflit d'une fusion, repéré par sa clé ; l'ordre est celui du texte canonique (notes, puis
/// contextes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Conflict {
    /// Contenu d'une note révisé différemment des deux côtés : une version entière est choisie.
    Text {
        /// Note.
        node: NodeId,
    },
    /// Champ d'un membre de contexte changé différemment des deux côtés.
    Field {
        /// Contexte.
        scope: ScopeId,
        /// Note.
        node: NodeId,
        /// Champ.
        field: MemberField,
    },
    /// Acquittement d'une même cause différent des deux côtés (présence ou état confirmé).
    Ack {
        /// Contexte.
        scope: ScopeId,
        /// Note dépendante.
        dependent: NodeId,
        /// Cause acquittée.
        cause: NodeId,
    },
}

/// Résolution d'un conflit, repérée par sa clé pour rester lisible dans le journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Valeur d'un côté pour un conflit de texte, de champ ou d'acquittement.
    Choose(Conflict, Side),
    /// Relation candidate d'un conflit structurel à désactiver, avec les effets de son retrait.
    Deactivate {
        /// Contexte.
        scope: ScopeId,
        /// Relation.
        edge: EdgeId,
    },
}

/// Conflit structurel : l'état réuni viole une règle que chaque côté respectait (INV-MERGE-03).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Structural {
    /// Règle violée.
    pub reason: Error,
    /// Relations actives créées ou modifiées depuis la base, d'un côté ou de l'autre, dont la
    /// désactivation peut lever le conflit.
    pub candidates: Vec<(ScopeId, EdgeId)>,
}

/// Plan d'une fusion, calculé sans rien modifier ; la fusion applique exactement ce plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergePlan {
    /// Base commune : l'unique ancêtre commun le plus proche des deux têtes.
    pub base: CommitId,
    /// Tête de la cible (branche active).
    pub target_head: CommitId,
    /// Tête de la source, à reprendre dans la commande de fusion.
    pub source_head: CommitId,
    /// Vrai si la source est déjà réunie : rien à fusionner.
    pub up_to_date: bool,
    /// Nombre d'objets (notes, membres, relations, acquittements, ancrages) pris à la source
    /// sans conflit.
    pub from_source: usize,
    /// Conflits restant à résoudre, dans l'ordre canonique.
    pub conflicts: Vec<Conflict>,
    /// Conflit structurel restant, après les désactivations demandées.
    pub structural: Option<Structural>,
    /// Effets de la fusion sur les causes de réexamen de la cible.
    pub effects: Effects,
}

/// État réuni et ce qu'il reste à résoudre.
pub(crate) struct Merged {
    pub(crate) state: Snapshot,
    pub(crate) from_source: usize,
    pub(crate) conflicts: Vec<Conflict>,
    pub(crate) structural: Option<Structural>,
}

/// Résolutions indexées par clé, avec leur rang (à partir de 1).
struct Choices {
    sides: BTreeMap<Conflict, (usize, Side)>,
    deactivations: BTreeMap<(ScopeId, EdgeId), usize>,
}

impl Choices {
    /// Refuse deux résolutions de même clé, en nommant la seconde.
    fn of(resolutions: &[Resolution]) -> Result<Self, Error> {
        let mut choices = Self {
            sides: BTreeMap::new(),
            deactivations: BTreeMap::new(),
        };
        for (index, resolution) in resolutions.iter().enumerate() {
            let rank = index + 1;
            let fresh = match resolution {
                Resolution::Choose(conflict, side) => {
                    choices.sides.insert(*conflict, (rank, *side)).is_none()
                }
                Resolution::Deactivate { scope, edge } => choices
                    .deactivations
                    .insert((*scope, *edge), rank)
                    .is_none(),
            };
            if !fresh {
                return Err(Error::DuplicateResolution(rank));
            }
        }
        Ok(choices)
    }
}

/// Réunion en cours : résolutions, conflits relevés, objets pris à la source.
struct Merger<'a> {
    choices: &'a Choices,
    used: BTreeSet<Conflict>,
    conflicts: Vec<Conflict>,
    from_source: usize,
}

impl Merger<'_> {
    /// Règle par champ ; un conflit prend la valeur résolue, sinon il est relevé et la valeur de
    /// la cible tient lieu de résultat provisoire. Rend la valeur et vrai si elle vient de la
    /// source sans conflit.
    fn field<T: PartialEq + Clone>(
        &mut self,
        conflict: Conflict,
        base: Option<&T>,
        target: &T,
        source: &T,
    ) -> (T, bool) {
        if target == source || base == Some(source) {
            return (target.clone(), false);
        }
        if base == Some(target) {
            return (source.clone(), true);
        }
        match self.choices.sides.get(&conflict) {
            Some((_, side)) => {
                self.used.insert(conflict);
                match side {
                    Side::Target => (target.clone(), false),
                    Side::Source => (source.clone(), false),
                }
            }
            None => {
                self.conflicts.push(conflict);
                (target.clone(), false)
            }
        }
    }

    fn member(
        &mut self,
        scope: ScopeId,
        id: NodeId,
        base: Option<&Node>,
        t: &Node,
        s: &Node,
    ) -> Node {
        let conflict = |field| Conflict::Field {
            scope,
            node: id,
            field,
        };
        let mut merged = t.clone();
        let (verdict, a) = self.field(
            conflict(MemberField::Verdict),
            base.map(|b| &b.verdict),
            &t.verdict,
            &s.verdict,
        );
        let (decision, b) = self.field(
            conflict(MemberField::Decision),
            base.map(|b| &b.decision),
            &t.decision,
            &s.decision,
        );
        let (implementation, c) = self.field(
            conflict(MemberField::Implementation),
            base.map(|b| &b.implementation),
            &t.implementation,
            &s.implementation,
        );
        let (reserve, d) = self.field(
            conflict(MemberField::Reserve),
            base.map(|b| &b.reserve),
            &t.reserve,
            &s.reserve,
        );
        let (visibility, e) = self.field(
            conflict(MemberField::Visibility),
            base.map(|b| &b.visibility),
            &t.visibility,
            &s.visibility,
        );
        self.from_source += usize::from(a || b || c || d || e);
        merged.verdict = verdict;
        merged.decision = decision;
        merged.implementation = implementation;
        merged.reserve = reserve;
        merged.visibility = visibility;
        merged
    }

    fn graph(
        &mut self,
        scope: ScopeId,
        base: Option<&Graph>,
        t: &Graph,
        s: &Graph,
    ) -> Result<Graph, Error> {
        let mut merged = Graph::default();
        for id in union(t.nodes.keys(), s.nodes.keys()) {
            let node = match (t.nodes.get(&id), s.nodes.get(&id)) {
                (Some(a), Some(b)) => {
                    self.member(scope, id, base.and_then(|g| g.nodes.get(&id)), a, b)
                }
                (Some(a), None) => a.clone(),
                (None, Some(b)) => {
                    self.from_source += 1;
                    b.clone()
                }
                (None, None) => continue,
            };
            merged.nodes.insert(id, node);
        }
        for id in union(t.edges.keys(), s.edges.keys()) {
            let edge = match (t.edges.get(&id), s.edges.get(&id)) {
                (Some(a), Some(b)) => self.edge(base.and_then(|g| g.edges.get(&id)), a, b)?,
                (Some(a), None) => a.clone(),
                (None, Some(b)) => {
                    self.from_source += 1;
                    b.clone()
                }
                (None, None) => continue,
            };
            merged.edges.insert(id, edge);
        }
        let empty = BTreeMap::new();
        let base_acks = base.map_or(&empty, |g| &g.acks);
        for key in union(union(base_acks.keys(), t.acks.keys()).iter(), s.acks.keys()) {
            let (dependent, cause) = key;
            let conflict = Conflict::Ack {
                scope,
                dependent,
                cause,
            };
            let base = Some(base_acks.get(&key).copied());
            let (ack, taken): (Option<Ack>, bool) = self.field(
                conflict,
                base.as_ref(),
                &t.acks.get(&key).copied(),
                &s.acks.get(&key).copied(),
            );
            self.from_source += usize::from(taken);
            if let Some(ack) = ack {
                merged.acks.insert(key, ack);
            }
        }
        Ok(merged)
    }

    /// Relation présente des deux côtés : seule l'activité peut différer, et jamais en conflit.
    fn edge(&mut self, base: Option<&Edge>, t: &Edge, s: &Edge) -> Result<Edge, Error> {
        let fixed = |e: &Edge| Edge {
            active: true,
            ..e.clone()
        };
        if fixed(t) != fixed(s) {
            return Err(Error::IncoherentMerge);
        }
        let active = base.map(|b| &b.active);
        if t.active == s.active || active == Some(&s.active) {
            return Ok(t.clone());
        }
        if active == Some(&t.active) {
            self.from_source += 1;
            return Ok(s.clone());
        }
        Err(Error::IncoherentMerge)
    }
}

/// Clés de deux suites croissantes, réunies et triées.
fn union<'a, K: Ord + Copy + 'a>(
    a: impl IntoIterator<Item = &'a K>,
    b: impl IntoIterator<Item = &'a K>,
) -> BTreeSet<K> {
    a.into_iter().chain(b).copied().collect()
}

/// Réunit `target` et `source` depuis `base`, résolutions appliquées ; relève les conflits
/// restants et le conflit structurel éventuel.
///
/// # Errors
/// [`Error::DuplicateResolution`], [`Error::UnmatchedResolution`] ; [`Error::IncoherentMerge`]
/// si une relation ou un ancrage diverge sans que la base l'explique.
pub(crate) fn merge(
    base: &Snapshot,
    target: &Snapshot,
    source: &Snapshot,
    resolutions: &[Resolution],
) -> Result<Merged, Error> {
    let choices = Choices::of(resolutions)?;
    let mut merger = Merger {
        choices: &choices,
        used: BTreeSet::new(),
        conflicts: Vec::new(),
        from_source: 0,
    };
    let mut state = Snapshot {
        notes: BTreeMap::new(),
        scopes: BTreeMap::new(),
        anchors: BTreeMap::new(),
    };
    for id in union(target.notes.keys(), source.notes.keys()) {
        let content = match (target.notes.get(&id), source.notes.get(&id)) {
            (Some(a), Some(b)) => {
                let (content, taken) =
                    merger.field(Conflict::Text { node: id }, base.notes.get(&id), a, b);
                merger.from_source += usize::from(taken);
                content
            }
            (Some(a), None) => a.clone(),
            (None, Some(b)) => {
                merger.from_source += 1;
                b.clone()
            }
            (None, None) => continue,
        };
        state.notes.insert(id, content);
    }
    for id in union(target.scopes.keys(), source.scopes.keys()) {
        let graph = match (target.scopes.get(&id), source.scopes.get(&id)) {
            (Some(a), Some(b)) => merger.graph(id, base.scopes.get(&id), a, b)?,
            (Some(a), None) => a.clone(),
            (None, Some(b)) => {
                merger.from_source += b.nodes.len() + b.edges.len() + b.acks.len();
                b.clone()
            }
            (None, None) => continue,
        };
        state.scopes.insert(id, graph);
    }
    for id in union(target.anchors.keys(), source.anchors.keys()) {
        let anchor = match (target.anchors.get(&id), source.anchors.get(&id)) {
            (Some(a), Some(b)) => {
                let before = base.anchors.get(&id);
                if a == b || before == Some(b) {
                    a.clone()
                } else if before == Some(a) {
                    merger.from_source += 1;
                    b.clone()
                } else {
                    return Err(Error::IncoherentMerge);
                }
            }
            (Some(a), None) => a.clone(),
            (None, Some(b)) => {
                merger.from_source += 1;
                b.clone()
            }
            (None, None) => continue,
        };
        state.anchors.insert(id, anchor);
    }
    // Chaque membre porte le contenu réuni de sa note (INV-SCOPE-02).
    for graph in state.scopes.values_mut() {
        for (id, node) in &mut graph.nodes {
            if let Some(content) = state.notes.get(id) {
                node.content = content.clone();
            }
        }
    }
    let Merger {
        used,
        mut conflicts,
        from_source,
        ..
    } = merger;
    conflicts.sort_unstable();
    let mut unused: Vec<usize> = choices
        .sides
        .iter()
        .filter(|(conflict, _)| !used.contains(conflict))
        .map(|(_, (rank, _))| *rank)
        .collect();
    // Conflit structurel (FUSION.md, section 9.4) : les relations candidates sont celles de l'état
    // réuni avant toute désactivation ; sans conflit structurel, aucune désactivation ne vaut.
    let structural = if state.validate().is_ok() {
        unused.extend(choices.deactivations.values().copied());
        None
    } else {
        let candidates = candidates(base, target, source, &state);
        for (&(scope, edge), &rank) in &choices.deactivations {
            if candidates.contains(&(scope, edge)) {
                state.graph_mut(scope)?.detach(edge)?;
            } else {
                unused.push(rank);
            }
        }
        state
            .validate()
            .err()
            .map(|reason| Structural { reason, candidates })
    };
    if let Some(rank) = unused.into_iter().min() {
        return Err(Error::UnmatchedResolution(rank));
    }
    Ok(Merged {
        state,
        from_source,
        conflicts,
        structural,
    })
}

/// Relations actives de l'état réuni créées ou modifiées depuis la base d'un côté ou de l'autre.
fn candidates(
    base: &Snapshot,
    target: &Snapshot,
    source: &Snapshot,
    merged: &Snapshot,
) -> Vec<(ScopeId, EdgeId)> {
    let edge = |s: &Snapshot, scope: &ScopeId, id: &EdgeId| {
        s.scopes.get(scope).and_then(|g| g.edges.get(id)).cloned()
    };
    let mut found = Vec::new();
    for (scope, graph) in &merged.scopes {
        for (id, _) in graph.edges.iter().filter(|(_, e)| e.active) {
            let before = edge(base, scope, id);
            let changed =
                |s: &Snapshot| edge(s, scope, id).is_some_and(|x| Some(&x) != before.as_ref());
            // Une relation absente de la base a changé du côté qui la porte.
            if changed(target) || changed(source) {
                found.push((*scope, *id));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::id::{AnchorId, DEFAULT_SCOPE};
    use crate::model::{
        text_digest, Anchor, CauseState, Content, Fingerprint, Kind, Relation, Verdict, Visibility,
    };
    use crate::{Decision, Implementation, ReserveStatus};

    const S: ScopeId = DEFAULT_SCOPE;

    fn bytes(n: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[15] = n;
        b
    }

    fn note(n: u8) -> NodeId {
        NodeId::from_bytes(bytes(n))
    }

    fn edge(n: u8) -> EdgeId {
        EdgeId::from_bytes(bytes(n))
    }

    /// Note 1 (raisonnement) ; note 2 (implémentation) qui dépend de la note 1 (relation 3).
    fn base() -> Snapshot {
        let mut s = Snapshot::default();
        s.insert_note(S, note(1), Content::new(Kind::Reasoning, "a", "a"))
            .unwrap();
        s.insert_note(S, note(2), Content::new(Kind::Implementation, "b", "b"))
            .unwrap();
        s.graph_mut(S)
            .unwrap()
            .insert_edge(edge(3), note(2), note(1), Relation::DependsOn, None, None)
            .unwrap();
        s
    }

    fn merged(base: &Snapshot, target: &Snapshot, source: &Snapshot) -> Merged {
        merge(base, target, source, &[]).unwrap()
    }

    fn graph(s: &mut Snapshot) -> &mut Graph {
        s.graph_mut(S).unwrap()
    }

    // Un objet dont un seul champ vient de la source compte une fois, quel que soit ce champ.
    #[test]
    fn each_member_field_taken_from_the_source_counts_once() {
        let b = base();
        let changes: [fn(&mut Node); 5] = [
            |n| n.verdict = Some(Verdict::Verified),
            |n| n.decision = Some(Decision::Retained),
            |n| n.implementation = Some(Implementation::InProgress),
            |n| n.reserve = Some(ReserveStatus::Open),
            |n| n.visibility = Visibility::Archived,
        ];
        for change in changes {
            let mut source = b.clone();
            change(graph(&mut source).nodes.get_mut(&note(2)).unwrap());
            let m = merged(&b, &b, &source);
            assert_eq!(m.from_source, 1);
            assert_eq!(
                m.state.scopes[&S].nodes[&note(2)],
                source.scopes[&S].nodes[&note(2)]
            );
            // Le même changement du côté de la cible ne vient pas de la source.
            assert_eq!(merged(&b, &source, &b).from_source, 0);
        }
    }

    // Une relation présente des deux côtés ne diffère que par son activité, sans conflit
    // possible ; une relation nouvelle d'un côté est reprise ; tout autre écart est refusé.
    #[test]
    fn relations_merge_by_activity_without_conflict() {
        let b = base();
        let mut detached = b.clone();
        graph(&mut detached).edges.get_mut(&edge(3)).unwrap().active = false;
        let m = merged(&b, &b, &detached);
        assert!(!m.state.scopes[&S].edges[&edge(3)].active);
        assert_eq!(m.from_source, 1);
        let m = merged(&b, &detached, &b);
        assert!(!m.state.scopes[&S].edges[&edge(3)].active);
        assert_eq!(m.from_source, 0);
        let m = merged(&b, &detached, &detached);
        assert!(!m.state.scopes[&S].edges[&edge(3)].active);
        assert_eq!(m.from_source, 0);
        // Relation nouvelle d'un seul côté.
        let mut linked = b.clone();
        graph(&mut linked)
            .insert_edge(edge(4), note(2), note(1), Relation::Related, None, None)
            .unwrap();
        assert_eq!(merged(&b, &b, &linked).from_source, 1);
        assert_eq!(merged(&b, &linked, &b).from_source, 0);
        assert!(merged(&b, &b, &linked).state.scopes[&S].edges[&edge(4)].active);
        // Écarts inexplicables : autre définition, ou activité différente sans base.
        let mut annotated = b.clone();
        graph(&mut annotated)
            .edges
            .get_mut(&edge(3))
            .unwrap()
            .annotation = Some("x".into());
        assert_eq!(
            merge(&b, &b, &annotated, &[]).map(|_| ()),
            Err(Error::IncoherentMerge)
        );
        let mut bare = b.clone();
        graph(&mut bare).edges.clear();
        assert_eq!(
            merge(&bare, &b, &detached, &[]).map(|_| ()),
            Err(Error::IncoherentMerge)
        );
    }

    fn ack(dependent: usize) -> Ack {
        Ack {
            dependent,
            cause: CauseState {
                fingerprint: Fingerprint::default(),
                verdict: Some(Verdict::Refuted),
                decision: Some(Decision::Proposed),
                visibility: Visibility::Current,
            },
        }
    }

    // La présence d'un acquittement se compare comme un champ.
    #[test]
    fn acknowledgement_presence_is_a_field() {
        let b = base();
        let mut acked = b.clone();
        graph(&mut acked).acks.insert((note(2), note(1)), ack(0));
        let m = merged(&b, &b, &acked);
        assert_eq!(m.state.scopes[&S].acks.len(), 1);
        assert_eq!(m.from_source, 1);
        let m = merged(&b, &acked, &b);
        assert_eq!((m.state.scopes[&S].acks.len(), m.from_source), (1, 0));
        // Retiré d'un côté depuis la base (restauration), inchangé de l'autre : il disparaît.
        let m = merged(&acked, &b, &acked);
        assert_eq!((m.state.scopes[&S].acks.len(), m.from_source), (0, 0));
        let m = merged(&acked, &acked, &b);
        assert_eq!((m.state.scopes[&S].acks.len(), m.from_source), (0, 1));
    }

    // Notes : contenu pris à la source, ou note nouvelle de la source, comptés une fois.
    #[test]
    fn notes_taken_from_the_source_are_counted() {
        let b = base();
        let mut revised = b.clone();
        revised.set_content(
            note(1),
            Arc::new(Content::new(Kind::Reasoning, "a", "révisé")),
        );
        let m = merged(&b, &b, &revised);
        assert_eq!(m.from_source, 1);
        assert_eq!(m.state.notes[&note(1)], revised.notes[&note(1)]);
        assert_eq!(
            m.state.scopes[&S].nodes[&note(1)].content,
            revised.notes[&note(1)]
        );
        let mut created = b.clone();
        created
            .insert_note(S, note(5), Content::new(Kind::Category, "c", "c"))
            .unwrap();
        // Contenu et membre de la note nouvelle.
        assert_eq!(merged(&b, &b, &created).from_source, 2);
        assert_eq!(merged(&b, &created, &b).from_source, 0);
    }

    // Un contexte présent d'un seul côté est repris en entier ; ses objets comptent.
    #[test]
    fn a_scope_of_the_source_alone_is_taken_whole() {
        let b = base();
        let mut source = b.clone();
        let mut copied = b.scopes[&S].clone();
        copied.acks.insert((note(2), note(1)), ack(0));
        let other = ScopeId::from_bytes(bytes(9));
        source.scopes.insert(other, copied.clone());
        let m = merged(&b, &b, &source);
        assert_eq!(m.state.scopes[&other], copied);
        // Deux membres, une relation, un acquittement.
        assert_eq!(m.from_source, 4);
        assert_eq!(merged(&b, &source, &b).from_source, 0);
    }

    fn anchor(active: bool) -> Anchor {
        Anchor {
            node: note(2),
            path: "f.c".into(),
            excerpt: "x".into(),
            digest: text_digest("x"),
            git: None,
            active,
        }
    }

    // Un ancrage ne fait que se désactiver : la désactivation d'un côté est reprise ; un écart
    // que la base n'explique pas est refusé.
    #[test]
    fn anchors_merge_by_activity() {
        let a = AnchorId::from_bytes(bytes(7));
        let mut b = base();
        b.anchors.insert(a, anchor(true));
        let mut off = b.clone();
        off.anchors.insert(a, anchor(false));
        let m = merged(&b, &b, &off);
        assert_eq!((m.state.anchors[&a].active, m.from_source), (false, 1));
        let m = merged(&b, &off, &b);
        assert_eq!((m.state.anchors[&a].active, m.from_source), (false, 0));
        let m = merged(&b, &off, &off);
        assert_eq!((m.state.anchors[&a].active, m.from_source), (false, 0));
        let without = base();
        assert_eq!(merged(&without, &without, &b).from_source, 1);
        assert_eq!(merged(&without, &b, &without).from_source, 0);
        assert_eq!(
            merge(&without, &b, &off, &[]).map(|_| ()),
            Err(Error::IncoherentMerge)
        );
    }

    // Candidates d'un conflit structurel : relations actives de l'état réuni, nouvelles ou
    // modifiées depuis la base d'un côté ou de l'autre ; une relation inchangée ne l'est pas.
    #[test]
    fn structural_candidates_are_the_relations_changed_since_the_base() {
        let mut b = base();
        graph(&mut b).edges.get_mut(&edge(3)).unwrap().active = false;
        // Source : la relation 3 redevient active ; cible : une relation identique, nouvelle.
        let mut source = b.clone();
        graph(&mut source).edges.get_mut(&edge(3)).unwrap().active = true;
        let mut target = b.clone();
        graph(&mut target)
            .insert_edge(edge(4), note(2), note(1), Relation::DependsOn, None, None)
            .unwrap();
        let m = merged(&b, &target, &source);
        assert_eq!(
            m.structural,
            Some(Structural {
                reason: Error::DuplicateRelation,
                candidates: vec![(S, edge(3)), (S, edge(4))],
            })
        );
        // La même relation, réactivée côté cible et nouvelle côté source.
        let m = merged(&b, &source, &target);
        assert_eq!(
            m.structural.map(|s| s.candidates),
            Some(vec![(S, edge(3)), (S, edge(4))])
        );
    }
}
