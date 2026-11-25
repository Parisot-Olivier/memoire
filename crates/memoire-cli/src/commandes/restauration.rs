//! Historique et restauration (RESTAURATION.md, sections 3 et 4 ; INV-RESTORE-01, INV-RESTORE-02,
//! CA-27, CA-33). Aucune règle ici : le plan et la restauration sont ceux du noyau.

use std::collections::BTreeSet;

use memoire_domain::{
    command_name, Change, Command, CommitId, EdgeId, History, NodeId, RestoreItem, RestorePlan,
    Snapshot, DEFAULT_SCOPE, ROOT,
};

use super::fusion::{nom_de_champ, valeur_de_membre, TYPES};
use super::notes::notes;
use super::{
    compte_rendu, designation, mot, preparer_et_appliquer, resoudre, Commande, Contexte, Depot,
};
use crate::arguments::Arguments;
use crate::Echec;

/// Commit désigné par un identifiant complet, ou par un préfixe d'au moins 8 caractères d'un
/// commit de l'historique de la branche active.
pub(crate) fn commit(depot: &Depot, saisie: &str) -> Result<CommitId, Echec> {
    let repo = depot.repository();
    if let Some(id) = CommitId::parse(saisie) {
        if repo.history().commit(id).map_err(Echec::Refus)?.is_some() {
            return Ok(id);
        }
    }
    let historique = repo.history_of_branch(usize::MAX).map_err(Echec::Refus)?;
    resoudre(saisie, historique.iter().map(|h| h.commit), "commit")
}

/// Sélection de `--notes`, si elle est donnée.
fn selection(depot: &Depot, liste: Option<&str>) -> Result<Option<BTreeSet<NodeId>>, Echec> {
    liste
        .map(|l| notes(depot, "--notes", l).map(|v| v.into_iter().collect()))
        .transpose()
}

pub(crate) struct Historique;

impl Commande for Historique {
    fn nom(&self) -> &'static str {
        "historique"
    }
    fn usage(&self) -> &'static str {
        "[--nombre N]"
    }
    fn resume(&self) -> &'static str {
        "liste les derniers commits de la branche active, du plus récent au plus ancien : \
         identifiant, commande, motif"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let nombre = arguments.option("--nombre")?;
        arguments.fin()?;
        let nombre = match nombre {
            None => 20,
            Some(n) => n.parse::<usize>().ok().filter(|k| *k >= 1).ok_or_else(|| {
                Echec::Usage(format!("nombre invalide : {n} (entier à partir de 1)"))
            })?,
        };
        let depot = contexte.ouvrir()?;
        let historique = depot
            .repository()
            .history_of_branch(nombre)
            .map_err(Echec::Refus)?;
        let mut texte = String::new();
        for entree in historique {
            if entree.commit == ROOT {
                texte.push_str(&format!("{} — racine\n", entree.commit));
                continue;
            }
            let commande = entree.command.as_ref().map_or("?", command_name);
            let motif = if entree.reason.is_empty() {
                "(sans motif)"
            } else {
                entree.reason.as_str()
            };
            texte.push_str(&format!("{} — {commande} — {motif}\n", entree.commit));
        }
        contexte.ecrire(&texte)
    }
}

/// Relation d'un état, désignée par ses extrémités et son type.
fn relation(etat: &Snapshot, edge: EdgeId) -> Option<(bool, String)> {
    let r = etat.graph(DEFAULT_SCOPE).ok()?.edge(edge).ok()?;
    Some((
        r.active(),
        format!(
            "{} {} {}",
            designation(etat, r.from()),
            mot(r.relation(), TYPES),
            designation(etat, r.to())
        ),
    ))
}

/// Texte du plan (RESTAURATION.md, section 1).
fn texte_du_plan(
    courant: &Snapshot,
    cible: &Snapshot,
    plan: &RestorePlan,
    ciblee: Option<usize>,
) -> String {
    let mut texte = match ciblee {
        Some(n) => format!(
            "Restauration ciblée de {} : {n} note{}\n",
            plan.target,
            if n > 1 { "s" } else { "" }
        ),
        None => format!("Restauration complète de {}\n", plan.target),
    };
    let mut retabli = Vec::new();
    let mut conserve = Vec::new();
    for item in &plan.items {
        match *item {
            RestoreItem::Text {
                node,
                target_version,
                current_version,
            } => retabli.push(format!(
                "texte de {} : version {target_version} rétablie, version {current_version} remplacée",
                designation(courant, node)
            )),
            RestoreItem::Field { scope, node, field } => retabli.push(format!(
                "{} de {} : {} → {}",
                nom_de_champ(field),
                designation(courant, node),
                valeur_de_membre(courant, scope, node, field),
                valeur_de_membre(cible, scope, node, field)
            )),
            RestoreItem::Relation { edge, .. } => {
                let (actif, nom) = relation(courant, edge).unwrap_or((false, String::new()));
                let apres = relation(cible, edge).is_some_and(|(a, _)| a);
                let quoi = match (actif, apres) {
                    (false, true) => "réactivée",
                    (true, false) => "désactivée",
                    _ => "rétablie",
                };
                retabli.push(format!("relation {edge} : {nom} — {quoi}"));
            }
            RestoreItem::Anchor { anchor } => {
                let chemin = courant.anchor(anchor).map(|a| a.path().to_owned()).unwrap_or_default();
                let actif = cible.anchor(anchor).is_ok_and(|a| a.active());
                let quoi = if actif { "rétabli" } else { "retiré" };
                retabli.push(format!("ancrage {anchor} ({chemin}) : {quoi}"));
            }
            RestoreItem::Ack {
                dependent, cause, ..
            } => retabli.push(format!(
                "acquittement de {} envers {} : rétabli tel qu'à la cible",
                designation(courant, dependent),
                designation(courant, cause)
            )),
            RestoreItem::Kept { edge, .. } => {
                let (_, nom) = relation(courant, edge).unwrap_or((false, String::new()));
                conserve.push(format!("relation {edge} : {nom}"));
            }
        }
    }
    let mut section = |titre: &str, lignes: Vec<String>, vide: &str| {
        texte.push_str(&format!("{titre} :\n"));
        if lignes.is_empty() {
            texte.push_str(&format!("  ({vide})\n"));
        }
        for l in lignes {
            texte.push_str(&format!("  - {l}\n"));
        }
    };
    section("Rétabli", retabli, "rien");
    if ciblee.is_some() {
        section(
            "Conservé (relations avec le reste du réseau, différentes depuis)",
            conserve,
            "aucune",
        );
    }
    let mut reexamens = Vec::new();
    for ((_, note), changement) in &plan.effects.review {
        let quoi = if changement.after.is_empty() {
            "n'est plus à réexaminer"
        } else {
            "à réexaminer"
        };
        reexamens.push(format!("{quoi} : {}", designation(courant, *note)));
    }
    section("Réexamens", reexamens, "aucun changement");
    texte
}

pub(crate) struct RestaurationPlan;

impl Commande for RestaurationPlan {
    fn nom(&self) -> &'static str {
        "restauration plan"
    }
    fn usage(&self) -> &'static str {
        "--depuis COMMIT [--notes NOTES]"
    }
    fn resume(&self) -> &'static str {
        "ce que rétablira la restauration de ces notes (ou de tout le réseau) depuis ce commit, et \
         les relations avec le reste du réseau conservées"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let depuis = arguments.requise("--depuis")?;
        let liste = arguments.option("--notes")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let cible = commit(&depot, &depuis)?;
        let choisies = selection(&depot, liste.as_deref())?;
        let repo = depot.repository();
        let plan = repo
            .restore_plan(cible, choisies.as_ref())
            .map_err(Echec::Refus)?;
        let ancien = repo.snapshot_at(cible).map_err(Echec::Refus)?;
        let texte = texte_du_plan(
            repo.snapshot(),
            &ancien,
            &plan,
            choisies.as_ref().map(BTreeSet::len),
        );
        contexte.ecrire(&texte)
    }
}

pub(crate) struct Restaurer;

impl Commande for Restaurer {
    fn nom(&self) -> &'static str {
        "restaurer"
    }
    fn usage(&self) -> &'static str {
        "--depuis COMMIT [--notes NOTES] [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "rétablit ces notes, avec leurs liens internes, dans leur état de ce commit, sans toucher \
         le reste ; sans --notes, tout le réseau"
    }
    // Opération critique (D13) : aperçu et --oui ; un seul « annuler » la défait (CA-33).
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let depuis = arguments.requise("--depuis")?;
        let liste = arguments.option("--notes")?;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let oui = arguments.drapeau("--oui")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let target = commit(&depot, &depuis)?;
        let choisies = selection(&depot, liste.as_deref())?;
        let change = match choisies {
            Some(nodes) => Change::RestoreNodes { target, nodes },
            None => Change::Restore { target },
        };
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |_| Command::Change(change),
            &motif,
            oui,
        )?;
        contexte.ecrire(&compte_rendu(
            &format!("Restauration enregistrée : {}", applique.outcome.head),
            &applique,
        ))
    }
}
