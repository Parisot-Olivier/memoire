//! Base, état, historique et aide.

use memoire_app::Service;
use std::collections::BTreeMap;

use memoire_app::StoredRepository;
use memoire_domain::{
    command_name, BranchId, Change, Command, CommitId, Decision, Error, History, NodeId, OpId,
    Snapshot, Verdict, DEFAULT_SCOPE, MAIN,
};

use super::{branche, branches, designation, nom_de_branche, Commande, Contexte, Depot, COMMANDES};
use crate::arguments::Arguments;
use crate::Echec;

pub(crate) struct Init;

impl Commande for Init {
    fn nom(&self) -> &'static str {
        "init"
    }
    fn usage(&self) -> &'static str {
        ""
    }
    fn resume(&self) -> &'static str {
        "crée la base (jamais d'écrasement)"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        arguments.fin()?;
        if let Some(parent) = contexte.base.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let ids = contexte.ids.take().expect("source d'identités");
        Service::create(&contexte.base, ids)?;
        let texte = format!("Dépôt créé : {}\n", contexte.base.display());
        contexte.ecrire(&texte)
    }
}

pub(crate) struct Statut;

impl Commande for Statut {
    fn nom(&self) -> &'static str {
        "statut"
    }
    fn usage(&self) -> &'static str {
        ""
    }
    fn resume(&self) -> &'static str {
        "branche active, ressources actuelles, notes à réexaminer et leurs causes"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let depot = depot.repository();
        let etat = depot.snapshot();
        let graphe = etat.graph(DEFAULT_SCOPE).map_err(Echec::Refus)?;
        let mut texte = format!(
            "Branche active : {}\n",
            nom_de_branche(depot.active_branch())
        );
        texte.push_str("Ressources actuelles :\n");
        let ressources = graphe.current_resources();
        if ressources.is_empty() {
            texte.push_str("  (aucune)\n");
        }
        for id in ressources {
            texte.push_str(&format!("  - {}\n", designation(etat, id)));
        }
        texte.push_str("À réexaminer :\n");
        let causes = graphe.review_causes();
        if causes.is_empty() {
            texte.push_str("  (aucune)\n");
        }
        for (id, sources) in causes {
            let liste: Vec<String> = sources.iter().map(|c| designation(etat, *c)).collect();
            texte.push_str(&format!(
                "  - {} — cause : {}\n",
                designation(etat, id),
                liste.join(", ")
            ));
        }
        // R6, D24 : appuis tous écartés, causes acquittées.
        texte.push_str("Orphelines :\n");
        let orphelines = graphe.orphans();
        if orphelines.is_empty() {
            texte.push_str("  (aucune)\n");
        }
        for id in orphelines {
            texte.push_str(&format!("  - {}\n", designation(etat, id)));
        }
        contexte.ecrire(&texte)
    }
}

/// Déplacement dans l'historique : réversible lui-même, il ne demande pas de confirmation.
/// Annule ou rétablit ; `message` est calculé sur le dépôt avant le déplacement.
fn deplacer(
    contexte: &mut Contexte<'_>,
    arguments: Arguments,
    commande: Command,
    message: impl FnOnce(&Depot) -> Result<String, Echec>,
) -> Result<(), Echec> {
    let mut arguments = arguments;
    let motif = arguments.option("--motif")?.unwrap_or_default();
    arguments.fin()?;
    let mut depot = contexte.ouvrir()?;
    let message = message(&depot)?;
    let prevu = depot.plan(|_| commande)?;
    depot.apply(prevu, &motif)?;
    contexte.ecrire(&message)
}

pub(crate) struct Annuler;

impl Commande for Annuler {
    fn nom(&self) -> &'static str {
        "annuler"
    }
    fn usage(&self) -> &'static str {
        "[--motif M]"
    }
    fn resume(&self) -> &'static str {
        "annule la dernière opération de la branche active"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        // Une fiche de reprise juste avant : « annuler » la défait, et le dit (D26).
        deplacer(contexte, arguments, Command::Undo, |depot| {
            let repo = depot.repository();
            match repo.resume_point(repo.head()) {
                Ok(point) => Ok(format!(
                    "Annulé : point de reprise « {} ».\n",
                    point.objective
                )),
                Err(Error::NotAResumePoint(_)) => Ok("Annulé.\n".to_owned()),
                Err(e) => Err(Echec::Refus(e)),
            }
        })
    }
}

pub(crate) struct Retablir;

impl Commande for Retablir {
    fn nom(&self) -> &'static str {
        "retablir"
    }
    fn usage(&self) -> &'static str {
        "[--motif M]"
    }
    fn resume(&self) -> &'static str {
        "rétablit la dernière opération annulée"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        deplacer(contexte, arguments, Command::Redo, |_| {
            Ok("Rétabli.\n".to_owned())
        })
    }
}

pub(crate) struct Branches;

impl Commande for Branches {
    fn nom(&self) -> &'static str {
        "branches"
    }
    fn usage(&self) -> &'static str {
        "[--toutes] [--tsv]"
    }
    fn resume(&self) -> &'static str {
        "liste les branches ; * marque la branche active ; --toutes montre aussi les branches \
         abandonnées ; --tsv : une ligne par branche avec un nom lisible, pour l'interface"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let toutes = arguments.drapeau("--toutes")?;
        let en_tsv = arguments.drapeau("--tsv")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let depot = depot.repository();
        if en_tsv {
            return contexte.ecrire(&branches_en_tsv(depot, toutes)?);
        }
        let mut texte = String::new();
        for id in branches(depot) {
            let abandonnee = depot.is_abandoned(id).map_err(Echec::Refus)?;
            if abandonnee && !toutes {
                continue;
            }
            let marque = if id == depot.active_branch() {
                "* "
            } else {
                "  "
            };
            let tete = depot.branch_head(id).map_err(Echec::Refus)?;
            let suffixe = if abandonnee { " (abandonnée)" } else { "" };
            texte.push_str(&format!(
                "{marque}{} — tête {tete}{suffixe}\n",
                nom_de_branche(id)
            ));
        }
        contexte.ecrire(&texte)
    }
}

pub(crate) struct Basculer;

impl Commande for Basculer {
    fn nom(&self) -> &'static str {
        "basculer"
    }
    fn usage(&self) -> &'static str {
        "BRANCHE [--motif M]"
    }
    fn resume(&self) -> &'static str {
        "change de branche active (identifiant ou préfixe d'au moins 8 caractères)"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let saisie = arguments.positionnel("BRANCHE")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let cible = branche(&depot, &saisie)?;
        let prevu = depot.plan(|_| Command::Checkout { branch: cible })?;
        depot.apply(prevu, &motif)?;
        let texte = format!("Branche active : {}\n", nom_de_branche(cible));
        contexte.ecrire(&texte)
    }
}

pub(crate) struct Abandonner;

impl Commande for Abandonner {
    fn nom(&self) -> &'static str {
        "abandonner"
    }
    fn usage(&self) -> &'static str {
        "BRANCHE [--motif M]"
    }
    fn resume(&self) -> &'static str {
        "abandonne une branche sans rien supprimer ; si elle est active, revient sur son origine"
    }
    // Comme un déplacement dans l'historique, l'abandon se défait (par « reactiver ») : il ne
    // demande pas de confirmation (FUSION.md, section 9.9).
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let saisie = arguments.positionnel("BRANCHE")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let cible = branche(&depot, &saisie)?;
        let active = depot.repository().active_branch();
        let prevu = depot.plan(|_| Command::Abandon { branch: cible })?;
        let applique = depot.apply(prevu, &motif)?;
        let mut texte = format!("Branche abandonnée : {}\n", nom_de_branche(cible));
        if applique.outcome.branch != active {
            texte.push_str(&format!(
                "Branche active : {}\n",
                nom_de_branche(applique.outcome.branch)
            ));
        }
        contexte.ecrire(&texte)
    }
}

pub(crate) struct Reactiver;

impl Commande for Reactiver {
    fn nom(&self) -> &'static str {
        "reactiver"
    }
    fn usage(&self) -> &'static str {
        "BRANCHE [--motif M]"
    }
    fn resume(&self) -> &'static str {
        "réactive une branche abandonnée, sans la rendre active"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let saisie = arguments.positionnel("BRANCHE")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let cible = branche(&depot, &saisie)?;
        let prevu = depot.plan(|_| Command::Reactivate { branch: cible })?;
        depot.apply(prevu, &motif)?;
        let texte = format!("Branche réactivée : {}\n", nom_de_branche(cible));
        contexte.ecrire(&texte)
    }
}

pub(crate) struct Aide;

impl Commande for Aide {
    fn nom(&self) -> &'static str {
        "aide"
    }
    fn usage(&self) -> &'static str {
        "[COMMANDE]"
    }
    fn resume(&self) -> &'static str {
        "liste les commandes, ou détaille l'une d'elles"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let mut mots = Vec::new();
        while let Ok(mot) = arguments.positionnel("COMMANDE") {
            mots.push(mot);
        }
        arguments.fin()?;
        let ligne = |c: &&dyn Commande| {
            let usage = if c.usage().is_empty() {
                String::new()
            } else {
                format!(" {}", c.usage())
            };
            format!("  {}{usage} — {}\n", c.nom(), c.resume())
        };
        let texte = if mots.is_empty() {
            let mut texte = String::from(
                "Usage : memoire [--base FICHIER] COMMANDE …\nBase : --base, sinon MEMOIRE_BASE, sinon .memoire/depot.db\nCommandes :\n",
            );
            for commande in COMMANDES {
                texte.push_str(&ligne(commande));
            }
            texte
        } else {
            let nom = mots.join(" ");
            let commande = COMMANDES
                .iter()
                .find(|c| c.nom() == nom)
                .ok_or_else(|| Echec::Usage(format!("commande inconnue : {nom}")))?;
            ligne(commande)
        };
        contexte.ecrire(&texte)
    }
}

/// Une ligne d'une sortie --tsv : tabulations et retours à la ligne remplacés par des espaces.
pub(crate) fn champ_tsv(texte: &str) -> String {
    texte.replace(['\t', '\n', '\r'], " ")
}

/// Titre d'une note dans un état, sinon son identifiant.
fn titre(etat: Option<&Snapshot>, note: NodeId) -> String {
    etat.and_then(|e| {
        e.graph(DEFAULT_SCOPE)
            .ok()?
            .node(note)
            .ok()
            .map(|n| n.title().to_owned())
    })
    .unwrap_or_else(|| note.to_string())
}

/// Nom lisible d'une branche et motif, d'après l'opération qui l'a créée (ACTIONS.md, section 3) ;
/// une branche partage l'identifiant de son opération fondatrice.
fn fondation(depot: &StoredRepository, branche: BranchId) -> Result<(String, String), Echec> {
    if branche == MAIN {
        return Ok(("piste principale".to_owned(), String::new()));
    }
    let operation = depot
        .history()
        .operation(OpId::from_bytes(branche.to_bytes()))
        .map_err(Echec::Refus)?;
    let Some((enveloppe, _)) = operation else {
        return Ok((branche.to_string(), String::new()));
    };
    let etat = depot
        .snapshot_at(CommitId::from_bytes(branche.to_bytes()))
        .ok();
    let titre = |note: NodeId| titre(etat.as_deref(), note);
    let nom = match &enveloppe.command {
        Command::Change(Change::SetVerdict { node, verdict, .. }) => {
            if *verdict == Verdict::Refuted {
                format!("Réfutation de « {} »", titre(*node))
            } else {
                format!("Verdict sur « {} »", titre(*node))
            }
        }
        Command::Change(Change::SetDecision { node, decision, .. }) => {
            if *decision == Decision::NoLongerRetained {
                format!("Mise de côté de « {} »", titre(*node))
            } else {
                format!("Décision sur « {} »", titre(*node))
            }
        }
        Command::Change(Change::Archive { node, .. }) => format!("Retrait de « {} »", titre(*node)),
        Command::Change(Change::Restore { target } | Change::RestoreNodes { target, .. }) => {
            let etape = target.to_string();
            format!("Retour à l'étape {}", etape.get(..8).unwrap_or(&etape))
        }
        autre => command_name(autre).to_owned(),
    };
    Ok((nom, enveloppe.reason))
}

/// « branches --tsv » : en-tête, puis une ligne par branche ; deux noms identiques sont numérotés,
/// dans l'ordre des identifiants.
fn branches_en_tsv(depot: &StoredRepository, toutes: bool) -> Result<String, Echec> {
    let mut texte = String::from("piste\tactive\tmise-de-cote\tnom\tmotif\n");
    let mut vus: BTreeMap<String, usize> = BTreeMap::new();
    for id in branches(depot) {
        let abandonnee = depot.is_abandoned(id).map_err(Echec::Refus)?;
        if abandonnee && !toutes {
            continue;
        }
        let (nom, motif) = fondation(depot, id)?;
        let rang = vus.entry(nom.clone()).or_insert(0);
        *rang += 1;
        let nom = if *rang > 1 {
            format!("{nom} ({rang})")
        } else {
            nom
        };
        let oui = |vrai: bool| if vrai { "oui" } else { "non" };
        texte.push_str(&format!(
            "{id}\t{}\t{}\t{}\t{}\n",
            oui(id == depot.active_branch()),
            oui(abandonnee),
            champ_tsv(&nom),
            champ_tsv(&motif)
        ));
    }
    Ok(texte)
}
