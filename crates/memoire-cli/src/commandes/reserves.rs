//! Réserves (CLI.md, section 8).

use memoire_domain::{Change, Command, Relation, ReserveStatus, DEFAULT_SCOPE};

use super::{compte_rendu, note, preparer_et_appliquer, valeur, Commande, Contexte};
use crate::arguments::Arguments;
use crate::Echec;

pub(crate) const ETATS: &[(&str, ReserveStatus)] = &[
    ("ouverte", ReserveStatus::Open),
    (
        "traitee-a-verifier",
        ReserveStatus::TreatedPendingVerification,
    ),
    ("resolue", ReserveStatus::Resolved),
];

pub(crate) struct ReserveCreer;

impl Commande for ReserveCreer {
    fn nom(&self) -> &'static str {
        "reserve creer"
    }
    fn usage(&self) -> &'static str {
        "PASSAGE --titre T [--cible C]"
    }
    fn resume(&self) -> &'static str {
        "crée une réserve depuis un passage"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let titre = arguments.requise("--titre")?;
        let cible = arguments.option("--cible")?;
        let saisie = arguments.positionnel("PASSAGE")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let passage = note(&depot, &saisie)?;
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |fresh| {
                Command::Change(Change::CreateReserve {
                    scope: DEFAULT_SCOPE,
                    id: fresh.node(),
                    passage,
                    title: titre,
                    target: cible,
                })
            },
            "",
            false,
        )?;
        let id = match &applique.envelope.command {
            Command::Change(Change::CreateReserve { id, .. }) => *id,
            _ => unreachable!("commande construite ci-dessus"),
        };
        contexte.ecrire(&compte_rendu(&format!("Réserve créée : {id}"), &applique))
    }
}

/// Relation simple entre deux notes, lues dans l'ordre donné.
fn relier(
    arguments: Arguments,
    contexte: &mut Contexte<'_>,
    noms: [&str; 2],
    motif_requis: bool,
    construire: fn(
        memoire_domain::EdgeId,
        memoire_domain::NodeId,
        memoire_domain::NodeId,
    ) -> Change,
) -> Result<(), Echec> {
    let mut arguments = arguments;
    let motif = if motif_requis {
        arguments.requise("--motif")?
    } else {
        arguments.option("--motif")?.unwrap_or_default()
    };
    let oui = arguments.drapeau("--oui")?;
    let premier = arguments.positionnel(noms[0])?;
    let second = arguments.positionnel(noms[1])?;
    arguments.fin()?;
    let mut depot = contexte.ouvrir()?;
    let a = note(&depot, &premier)?;
    let b = note(&depot, &second)?;
    let applique = preparer_et_appliquer(
        contexte,
        &mut depot,
        |fresh| Command::Change(construire(fresh.edge(), a, b)),
        &motif,
        oui,
    )?;
    let edge = match &applique.envelope.command {
        Command::Change(Change::Link { id, .. } | Change::Treat { id, .. }) => *id,
        _ => unreachable!("commande construite ci-dessus"),
    };
    contexte.ecrire(&compte_rendu(
        &format!("Relation créée : {edge}"),
        &applique,
    ))
}

pub(crate) struct ReserveConcerne;

impl Commande for ReserveConcerne {
    fn nom(&self) -> &'static str {
        "reserve concerne"
    }
    fn usage(&self) -> &'static str {
        "RESERVE NOTE [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "indique que la réserve s'applique à la note ; elle reste ouverte"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        relier(
            arguments,
            contexte,
            ["RESERVE", "NOTE"],
            false,
            |id, from, to| Change::Link {
                scope: DEFAULT_SCOPE,
                id,
                from,
                to,
                relation: Relation::AppliesTo,
            },
        )
    }
}

pub(crate) struct ReserveTraiter;

impl Commande for ReserveTraiter {
    fn nom(&self) -> &'static str {
        "reserve traiter"
    }
    fn usage(&self) -> &'static str {
        "NOTE RESERVE --motif M [--oui]"
    }
    fn resume(&self) -> &'static str {
        "la note traite la réserve, qui passe à « traitée à vérifier »"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        // Traiter change le statut de la réserve : motif obligatoire (D12).
        relier(
            arguments,
            contexte,
            ["NOTE", "RESERVE"],
            true,
            |id, note, reserve| Change::Treat {
                scope: DEFAULT_SCOPE,
                id,
                note,
                reserve,
            },
        )
    }
}

pub(crate) struct ReserveEtat;

impl Commande for ReserveEtat {
    fn nom(&self) -> &'static str {
        "reserve etat"
    }
    fn usage(&self) -> &'static str {
        "RESERVE ouverte|traitee-a-verifier|resolue --motif M [--oui]"
    }
    fn resume(&self) -> &'static str {
        "pose l'état d'une réserve"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        super::notes::statut(arguments, contexte, |reserve, saisie| {
            Ok(Change::SetReserveStatus {
                scope: DEFAULT_SCOPE,
                reserve,
                status: valeur(saisie, ETATS, "état de réserve")?,
            })
        })
    }
}
