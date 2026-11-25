//! Vue locale d'une note (VOISINAGE.md, section 3 ; INV-VIEW-02 ; R7). Aucune règle ici : les liens
//! et leur origine sont ceux du noyau (`Graph::neighborhood`) ; lecture seule.

use std::collections::{BTreeMap, BTreeSet};

use memoire_domain::{
    CitedSource, Direction, Graph, LinkOrigin, NeighborLink, NodeId, Snapshot, DEFAULT_SCOPE,
};

use super::fusion::TYPES;
use super::notes::ROLES;
use super::{designation, mot, note, Commande, Contexte};
use crate::arguments::Arguments;
use crate::Echec;

/// État d'une note pour la vue : écartée d'abord, puis à réexaminer, puis orpheline.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Etat {
    Courante,
    Ecartee,
    AReexaminer,
    Orpheline,
}

impl Etat {
    fn mot(self) -> &'static str {
        match self {
            Etat::Courante => "courante",
            Etat::Ecartee => "ecartee",
            Etat::AReexaminer => "a-reexaminer",
            Etat::Orpheline => "orpheline",
        }
    }

    fn texte(self) -> &'static str {
        match self {
            Etat::Courante => "",
            Etat::Ecartee => "écartée",
            Etat::AReexaminer => "à réexaminer",
            Etat::Orpheline => "orpheline",
        }
    }
}

/// États des notes d'un contexte, calculés une fois.
struct Etats {
    causes: BTreeMap<NodeId, BTreeSet<NodeId>>,
    orphelines: BTreeSet<NodeId>,
}

impl Etats {
    fn new(graphe: &Graph) -> Self {
        Self {
            causes: graphe.review_causes(),
            orphelines: graphe.orphans().into_iter().collect(),
        }
    }

    fn de(&self, graphe: &Graph, id: NodeId) -> Etat {
        if graphe.node(id).is_ok_and(|n| n.withdrawn()) {
            Etat::Ecartee
        } else if self.causes.get(&id).is_some_and(|c| !c.is_empty()) {
            Etat::AReexaminer
        } else if self.orphelines.contains(&id) {
            Etat::Orpheline
        } else {
            Etat::Courante
        }
    }
}

/// Source d'une citation : la note ou l'échange cité.
fn source(etat: &Snapshot, citation: NodeId) -> Option<NodeId> {
    etat.citation(citation).ok().map(|vue| match vue.source {
        CitedSource::Exchange { exchange, .. } => exchange,
        CitedSource::Note { note, .. } => note,
    })
}

const ORIGINES: [(LinkOrigin, &str, &str); 3] = [
    (LinkOrigin::WholeNote, "Note entière", "note"),
    (LinkOrigin::Citation, "Citation", "citation"),
    (LinkOrigin::Category, "Catégorie", "categorie"),
];

/// Nom d'une note sur une ligne : titre, ou texte cité ; tabulations et retours à la ligne
/// remplacés par des espaces.
fn nom(graphe: &Graph, id: NodeId) -> String {
    graphe
        .node(id)
        .map(|n| {
            let brut = if n.kind() == memoire_domain::Kind::Passage {
                n.text()
            } else {
                n.title()
            };
            brut.replace(['\t', '\n', '\r'], " ")
        })
        .unwrap_or_default()
}

fn fleche(direction: Direction) -> (&'static str, &'static str) {
    match direction {
        Direction::Outgoing => ("→", "sortant"),
        Direction::Incoming => ("←", "entrant"),
    }
}

fn lisible(
    etat: &Snapshot,
    graphe: &Graph,
    etats: &Etats,
    id: NodeId,
    liens: &[NeighborLink],
) -> String {
    let centre = etats.de(graphe, id);
    let role = graphe.node(id).map_or("?", |n| mot(n.kind(), ROLES));
    let suffixe = match centre {
        Etat::Courante => String::new(),
        autre => format!(", {}", autre.texte()),
    };
    let mut texte = format!("Voisinage de {} — {role}{suffixe}\n", designation(etat, id));
    for (origine, titre, _) in ORIGINES {
        texte.push_str(&format!("{titre} :\n"));
        let mut aucun = true;
        for lien in liens.iter().filter(|l| l.origin == origine) {
            aucun = false;
            let mut ligne = format!(
                "  {} {} {}",
                fleche(lien.direction).0,
                mot(lien.relation, TYPES),
                designation(etat, lien.other)
            );
            let voisine = etats.de(graphe, lien.other);
            if voisine != Etat::Courante {
                ligne.push_str(&format!(" ({})", voisine.texte()));
            }
            if let Some(s) = source(etat, lien.other) {
                ligne.push_str(&format!(", citation de {}", designation(etat, s)));
            }
            texte.push_str(&ligne);
            texte.push('\n');
        }
        if aucun {
            texte.push_str("  (aucun)\n");
        }
    }
    texte
}

fn tsv(etat: &Snapshot, graphe: &Graph, etats: &Etats, liens: &[NeighborLink]) -> String {
    let mut texte =
        String::from("relation\tsens\ttype\torigine\tvoisine\trole\tnom\tsource\tetat\n");
    for lien in liens {
        let origine = ORIGINES
            .iter()
            .find(|(o, _, _)| *o == lien.origin)
            .map_or("?", |(_, _, mot)| mot);
        let role = graphe
            .node(lien.other)
            .map_or("?", |n| mot(n.kind(), ROLES));
        let source = source(etat, lien.other).map_or("-".to_owned(), |s| s.to_string());
        texte.push_str(&format!(
            "{}\t{}\t{}\t{origine}\t{}\t{role}\t{}\t{source}\t{}\n",
            lien.edge,
            fleche(lien.direction).1,
            mot(lien.relation, TYPES),
            lien.other,
            nom(graphe, lien.other),
            etats.de(graphe, lien.other).mot()
        ));
    }
    texte
}

pub(crate) struct Voisinage;

impl Commande for Voisinage {
    fn nom(&self) -> &'static str {
        "voisinage"
    }
    fn usage(&self) -> &'static str {
        "NOTE [--tsv]"
    }
    fn resume(&self) -> &'static str {
        "relations actives de la note, groupées par origine : note entière, citation, catégorie ; \
         --tsv : une ligne par lien, pour l'interface"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let en_tsv = arguments.drapeau("--tsv")?;
        let saisie = arguments.positionnel("NOTE")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let id = note(&depot, &saisie)?;
        let etat = depot.repository().snapshot();
        let graphe = etat.graph(DEFAULT_SCOPE).map_err(Echec::Refus)?;
        let liens = graphe.neighborhood(id).map_err(Echec::Refus)?;
        let etats = Etats::new(graphe);
        let texte = if en_tsv {
            tsv(etat, graphe, &etats, &liens)
        } else {
            lisible(etat, graphe, &etats, id, &liens)
        };
        contexte.ecrire(&texte)
    }
}
