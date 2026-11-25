//! Vue locale d'une note (VOISINAGE.md, section 2 ; INV-VIEW-02) : ses relations actives, chacune
//! classée selon l'origine du lien. Lecture seule.

use crate::error::Error;
use crate::id::{EdgeId, NodeId};
use crate::model::{Graph, Kind, Relation};

/// Origine d'un lien, dans l'ordre d'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinkOrigin {
    /// Relation entre deux notes entières.
    WholeNote,
    /// L'autre extrémité est une citation, d'un échange ou d'une note.
    Citation,
    /// Relation « classée dans », dans un sens ou dans l'autre.
    Category,
}

/// Sens d'une relation vu de la note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    /// La note est l'origine de la relation.
    Outgoing,
    /// La note est la destination de la relation.
    Incoming,
}

/// Lien de la vue locale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NeighborLink {
    /// Relation.
    pub edge: EdgeId,
    /// Sens, vu de la note.
    pub direction: Direction,
    /// Type.
    pub relation: Relation,
    /// Autre extrémité.
    pub other: NodeId,
    /// Origine.
    pub origin: LinkOrigin,
}

impl Graph {
    /// Relations actives qui touchent la note, chacune une fois, classées par origine, puis par
    /// sens, type et autre note (VOISINAGE.md, section 2).
    ///
    /// # Errors
    /// [`Error::MissingNode`] si la note n'est pas dans ce contexte.
    pub fn neighborhood(&self, node: NodeId) -> Result<Vec<NeighborLink>, Error> {
        self.node(node)?;
        let mut links = Vec::new();
        for (edge, data) in self.edges() {
            if !data.active() {
                continue;
            }
            let (direction, other) = if data.from() == node {
                (Direction::Outgoing, data.to())
            } else if data.to() == node {
                (Direction::Incoming, data.from())
            } else {
                continue;
            };
            let citation = self.node(other).is_ok_and(|n| n.kind() == Kind::Passage);
            let origin = match data.relation() {
                Relation::ClassifiedIn => LinkOrigin::Category,
                _ if citation => LinkOrigin::Citation,
                _ => LinkOrigin::WholeNote,
            };
            links.push(NeighborLink {
                edge,
                direction,
                relation: data.relation(),
                other,
                origin,
            });
        }
        links.sort_by_key(|l| (l.origin, l.direction, l.relation, l.other, l.edge));
        Ok(links)
    }
}
