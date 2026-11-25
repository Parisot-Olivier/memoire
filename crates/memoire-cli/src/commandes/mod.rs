//! Table des commandes et outils communs (CLI.md, sections 6 et 7). Ajouter une commande : écrire
//! son test, créer son type ici ou dans un module voisin, l'inscrire dans `COMMANDES`.

mod base;
mod code;
mod depot;
mod echanges;
mod fusion;
mod notes;
mod recherche;
mod reprise;
mod reserves;
mod restauration;
mod voisinage;

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use memoire_app::{Applied, Fresh, IdSource, Service, StoredRepository};
use memoire_domain::{BranchId, Command, Kind, NodeId, Preview, Snapshot, DEFAULT_SCOPE, MAIN};

use crate::arguments::Arguments;
use crate::Echec;

/// Contexte d'exécution d'une commande.
pub(crate) struct Contexte<'a> {
    pub(crate) base: PathBuf,
    /// Répertoire courant : dossier du code par défaut (FICHE-REPRISE.md, section 4).
    pub(crate) repertoire: PathBuf,
    /// Source d'identités, remise au service à l'ouverture de la base.
    pub(crate) ids: Option<Box<dyn IdSource>>,
    pub(crate) sortie: &'a mut dyn Write,
}

/// Service de la ligne de commande : sa source d'identités est choisie à l'exécution.
pub(crate) type Depot = Service<Box<dyn IdSource>>;

impl Contexte<'_> {
    fn ids(&mut self) -> Box<dyn IdSource> {
        self.ids
            .take()
            .expect("une seule ouverture de base par commande")
    }

    pub(crate) fn ouvrir(&mut self) -> Result<Depot, Echec> {
        let ids = self.ids();
        Ok(Service::open(&self.base, ids)?)
    }

    pub(crate) fn ecrire(&mut self, texte: &str) -> Result<(), Echec> {
        self.sortie.write_all(texte.as_bytes())?;
        Ok(())
    }
}

/// Une commande de la ligne de commande.
pub(crate) trait Commande: Sync {
    /// Nom, d'un ou deux mots, tel qu'il se tape.
    fn nom(&self) -> &'static str;
    /// Arguments et options, pour l'aide.
    fn usage(&self) -> &'static str;
    /// Résumé d'une ligne, pour l'aide.
    fn resume(&self) -> &'static str;
    /// Exécution. Les options se lisent avant les arguments de position.
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec>;
}

/// Toutes les commandes, dans l'ordre de l'aide.
pub(crate) static COMMANDES: &[&dyn Commande] = &[
    &base::Init,
    &base::Statut,
    &recherche::Chercher,
    &notes::NoteCreer,
    &notes::NoteReviser,
    &echanges::EchangeColler,
    &echanges::EchangeQuestion,
    &echanges::Citer,
    &echanges::Source,
    &echanges::Citations,
    &voisinage::Voisinage,
    &notes::Lier,
    &notes::Restreindre,
    &notes::Contredire,
    &notes::PoserVerdict,
    &notes::PoserDecision,
    &notes::PoserImplementation,
    &notes::Acquitter,
    &notes::Retirer,
    &reserves::ReserveCreer,
    &reserves::ReserveConcerne,
    &reserves::ReserveTraiter,
    &reserves::ReserveEtat,
    &code::Ancrer,
    &code::Repere,
    &code::Indexer,
    &code::CodeNotes,
    &base::Annuler,
    &base::Retablir,
    &restauration::Historique,
    &restauration::RestaurationPlan,
    &restauration::Restaurer,
    &reprise::RepriseNoter,
    &reprise::RepriseVoir,
    &reprise::RepriseListe,
    &reprise::RepriseCode,
    &base::Branches,
    &base::Basculer,
    &base::Abandonner,
    &base::Reactiver,
    &fusion::FusionPlan,
    &fusion::Fusionner,
    &depot::Exporter,
    &depot::Importer,
    &depot::Sauvegarder,
    &depot::Verifier,
    &depot::Migrer,
    &base::Aide,
];

/// Longueur minimale d'un préfixe d'identifiant saisi.
const PREFIXE_MINIMAL: usize = 8;

/// Résout un identifiant saisi en entier ou par un préfixe d'au moins 8 caractères désignant un
/// seul candidat.
fn resoudre<T: Copy + ToString>(
    saisie: &str,
    candidats: impl Iterator<Item = T>,
    quoi: &str,
) -> Result<T, Echec> {
    if saisie.len() < PREFIXE_MINIMAL {
        return Err(Echec::Usage(format!(
            "identifiant « {saisie} » trop court : au moins {PREFIXE_MINIMAL} caractères"
        )));
    }
    let trouves: Vec<T> = candidats
        .filter(|c| c.to_string().starts_with(saisie))
        .collect();
    match trouves.as_slice() {
        [un] => Ok(*un),
        [] => Err(Echec::Usage(format!(
            "aucun(e) {quoi} ne commence par « {saisie} »"
        ))),
        plusieurs => Err(Echec::Usage(format!(
            "« {saisie} » désigne plusieurs {quoi}s : {}",
            plusieurs
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Note du contexte par défaut de la branche active.
pub(crate) fn note(depot: &Depot, saisie: &str) -> Result<NodeId, Echec> {
    let graphe = depot
        .repository()
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .map_err(Echec::Refus)?;
    resoudre(saisie, graphe.nodes().map(|(id, _)| id), "note")
}

/// Branches du dépôt, main comprise, lues dans les métadonnées en mémoire (sans le journal).
pub(crate) fn branches(depot: &StoredRepository) -> BTreeSet<BranchId> {
    depot.branches().collect()
}

pub(crate) fn branche(depot: &Depot, saisie: &str) -> Result<BranchId, Echec> {
    resoudre(saisie, branches(depot.repository()).into_iter(), "branche")
}

/// « id (main) » pour main, « id » sinon.
pub(crate) fn nom_de_branche(branche: BranchId) -> String {
    if branche == MAIN {
        format!("{branche} (main)")
    } else {
        branche.to_string()
    }
}

/// « titre (id) » d'une note de l'instantané ; un passage est désigné par son texte entre
/// guillemets (CLI.md, section 8).
pub(crate) fn designation(etat: &Snapshot, id: NodeId) -> String {
    let noeud = etat.graph(DEFAULT_SCOPE).ok().and_then(|g| g.node(id).ok());
    match noeud {
        Some(n) if n.kind() == Kind::Passage => {
            // Citation d'une note révisée depuis (INV-CIT-01).
            let revisee = etat.citation(id).is_ok_and(|c| c.source_revised);
            let signal = if revisee { " (source révisée)" } else { "" };
            format!("« {} » ({id}){signal}", n.text())
        }
        Some(n) => format!("{} ({id})", n.title()),
        None => format!("({id})"),
    }
}

/// Texte de la sortie d'une modification : ligne principale, puis branche créée s'il y a lieu.
pub(crate) fn compte_rendu(ligne: &str, applique: &Applied) -> String {
    let mut texte = format!("{ligne}\n");
    if applique.outcome.forked {
        texte.push_str(&format!("Branche créée : {}\n", applique.outcome.branch));
    }
    texte
}

/// Valeur saisie parmi une table de mots permis.
pub(crate) fn valeur<T: Copy>(saisie: &str, table: &[(&str, T)], quoi: &str) -> Result<T, Echec> {
    table
        .iter()
        .find(|(mot, _)| *mot == saisie)
        .map(|(_, v)| *v)
        .ok_or_else(|| {
            let permis: Vec<&str> = table.iter().map(|(m, _)| *m).collect();
            Echec::Usage(format!(
                "{quoi} inconnu(e) : {saisie} (permis : {})",
                permis.join(", ")
            ))
        })
}

/// Mot d'une valeur, pour l'affichage (inverse de `valeur`).
pub(crate) fn mot<T: Copy + PartialEq>(v: T, table: &[(&'static str, T)]) -> &'static str {
    table.iter().find(|(_, x)| *x == v).map_or("?", |(m, _)| m)
}

/// Une commande est critique si elle crée une branche ou modifie une cause de réexamen.
fn critique(apercu: &Preview) -> bool {
    apercu.forks || !apercu.effects.review.is_empty()
}

/// Prépare une modification, affiche son aperçu si elle est critique, et ne l'applique qu'avec
/// confirmation dans ce cas (CLI.md, section 5).
pub(crate) fn preparer_et_appliquer(
    contexte: &mut Contexte<'_>,
    depot: &mut Depot,
    construire: impl FnOnce(&mut Fresh<'_>) -> Command,
    motif: &str,
    confirme: bool,
) -> Result<Applied, Echec> {
    let prevu = depot.plan(construire)?;
    confirmer(contexte, depot, &prevu.preview, confirme)?;
    Ok(depot.apply(prevu, motif)?)
}

/// Affiche l'aperçu d'une opération critique ; sans confirmation, rien ne sera écrit (code 4).
pub(crate) fn confirmer(
    contexte: &mut Contexte<'_>,
    depot: &Depot,
    apercu: &Preview,
    confirme: bool,
) -> Result<(), Echec> {
    if !critique(apercu) {
        return Ok(());
    }
    let etat = depot.repository().snapshot();
    let mut texte = String::from("Opération critique :\n");
    if apercu.forks {
        texte.push_str("  - une branche sera créée depuis main\n");
    }
    for ((_, note), changement) in &apercu.effects.review {
        let quoi = if changement.after.is_empty() {
            "n'est plus à réexaminer"
        } else {
            "à réexaminer"
        };
        texte.push_str(&format!("  - {quoi} : {}\n", designation(etat, *note)));
    }
    contexte.ecrire(&texte)?;
    if confirme {
        Ok(())
    } else {
        Err(Echec::Confirmation)
    }
}
