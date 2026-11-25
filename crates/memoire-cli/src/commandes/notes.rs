//! Notes, relations et statuts.

use memoire_domain::{
    Change, Command, Decision, Implementation, Kind, NodeId, Relation, RevisionKind, Verdict,
    DEFAULT_SCOPE,
};

use super::{compte_rendu, mot, note, preparer_et_appliquer, valeur, Commande, Contexte};
use crate::arguments::Arguments;
use crate::Echec;

pub(crate) const ROLES: &[(&str, Kind)] = &[
    ("categorie", Kind::Category),
    ("raisonnement", Kind::Reasoning),
    ("implementation", Kind::Implementation),
    ("note", Kind::PersonalNote),
    ("echange", Kind::Exchange),
    ("citation", Kind::Passage),
    ("reserve", Kind::Reserve),
];

/// Rôles que `note creer` accepte ; les autres ont leur propre commande.
pub(crate) const ROLES_CREABLES: usize = 4;

pub(crate) const IMPLEMENTATIONS: &[(&str, Implementation)] = &[
    ("non-commencee", Implementation::NotStarted),
    ("en-cours", Implementation::InProgress),
    ("implementee", Implementation::Implemented),
];

const NATURES: &[(&str, RevisionKind)] = &[
    ("mineure", RevisionKind::Minor),
    ("substantielle", RevisionKind::Substantive),
];

const RELATIONS: &[(&str, Relation)] = &[
    ("classe-dans", Relation::ClassifiedIn),
    ("depend-de", Relation::DependsOn),
    ("developpe-a-partir-de", Relation::DevelopedFrom),
    ("lien", Relation::Related),
];

pub(crate) const VERDICTS: &[(&str, Verdict)] = &[
    ("non-verifiee", Verdict::Unverified),
    ("verifiee", Verdict::Verified),
    ("refutee", Verdict::Refuted),
];

pub(crate) const DECISIONS: &[(&str, Decision)] = &[
    ("proposee", Decision::Proposed),
    ("retenue", Decision::Retained),
    ("plus-retenue", Decision::NoLongerRetained),
    ("remplacee", Decision::Replaced),
];

pub(crate) struct NoteCreer;

/// Options de relation de `note creer`, dans l'ordre où leurs relations sont créées et
/// annoncées : une par type que `lier` connaît (LOT.md, section 5).
const OPTIONS_DE_LIEN: [&str; 4] = [
    "--classe-dans",
    "--depend-de",
    "--developpe-a-partir-de",
    "--lien",
];

/// Notes d'une liste séparée par des virgules, chacune par identifiant ou préfixe ; une liste
/// vide, un élément vide ou une note citée deux fois sont des erreurs d'usage.
pub(crate) fn notes(depot: &super::Depot, option: &str, liste: &str) -> Result<Vec<NodeId>, Echec> {
    let mut trouvees = Vec::new();
    for saisie in liste.split(',').map(str::trim) {
        if saisie.is_empty() {
            return Err(Echec::Usage(format!(
                "{option} : élément vide dans « {liste} »"
            )));
        }
        let id = note(depot, saisie)?;
        if trouvees.contains(&id) {
            return Err(Echec::Usage(format!(
                "{option} : note {id} citée deux fois"
            )));
        }
        trouvees.push(id);
    }
    Ok(trouvees)
}

impl Commande for NoteCreer {
    fn nom(&self) -> &'static str {
        "note creer"
    }
    fn usage(&self) -> &'static str {
        "--role categorie|raisonnement|implementation|note --titre T [--texte X] \
         [--classe-dans NOTES] [--depend-de NOTES] [--developpe-a-partir-de NOTES] \
         [--lien NOTES] [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "crée une note et affiche son identifiant ; avec des liens (NOTES : notes séparées par \
         des virgules), la crée et la relie en une seule opération"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let role = arguments.requise("--role")?;
        let titre = arguments.requise("--titre")?;
        let texte = arguments.option("--texte")?.unwrap_or_default();
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let oui = arguments.drapeau("--oui")?;
        let mut listes = Vec::new();
        for option in OPTIONS_DE_LIEN {
            if let Some(liste) = arguments.option(option)? {
                listes.push((option, liste));
            }
        }
        arguments.fin()?;
        let kind = valeur(&role, &ROLES[..ROLES_CREABLES], "rôle")?;
        let mut depot = contexte.ouvrir()?;
        let mut liens = Vec::new();
        for (option, liste) in &listes {
            let relation = valeur(&option[2..], RELATIONS, "type de relation")?;
            for destination in notes(&depot, option, liste)? {
                liens.push((relation, destination));
            }
        }
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |fresh| {
                let id = fresh.node();
                let creation = Change::CreateNode {
                    scope: DEFAULT_SCOPE,
                    id,
                    kind,
                    title: titre,
                    text: texte,
                };
                if liens.is_empty() {
                    return Command::Change(creation);
                }
                // R1 : la note et ses relations en une seule opération (D22).
                let mut elements = vec![creation];
                for (relation, to) in liens {
                    elements.push(Change::Link {
                        scope: DEFAULT_SCOPE,
                        id: fresh.edge(),
                        from: id,
                        to,
                        relation,
                    });
                }
                Command::Lot(elements)
            },
            &motif,
            oui,
        )?;
        let (id, elements) = match &applique.envelope.command {
            Command::Change(Change::CreateNode { id, .. }) => (*id, &[][..]),
            Command::Lot(elements) => match elements.first() {
                Some(Change::CreateNode { id, .. }) => (*id, &elements[1..]),
                _ => unreachable!("lot construit ci-dessus"),
            },
            _ => unreachable!("commande construite ci-dessus"),
        };
        let mut lignes = vec![format!("Note créée : {id}")];
        for element in elements {
            if let Change::Link {
                id, to, relation, ..
            } = element
            {
                lignes.push(format!(
                    "Relation créée : {id} ({} {to})",
                    mot(*relation, RELATIONS)
                ));
            }
        }
        contexte.ecrire(&compte_rendu(&lignes.join("\n"), &applique))
    }
}

pub(crate) struct Lier;

impl Commande for Lier {
    fn nom(&self) -> &'static str {
        "lier"
    }
    fn usage(&self) -> &'static str {
        "ORIGINE TYPE DESTINATION [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "relie deux notes ; TYPE : classe-dans, depend-de, developpe-a-partir-de, lien"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let oui = arguments.drapeau("--oui")?;
        let origine = arguments.positionnel("ORIGINE")?;
        let genre = arguments.positionnel("TYPE")?;
        let destination = arguments.positionnel("DESTINATION")?;
        arguments.fin()?;
        let relation = valeur(&genre, RELATIONS, "type de relation")?;
        let mut depot = contexte.ouvrir()?;
        let from = note(&depot, &origine)?;
        let to = note(&depot, &destination)?;
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |fresh| {
                Command::Change(Change::Link {
                    scope: DEFAULT_SCOPE,
                    id: fresh.edge(),
                    from,
                    to,
                    relation,
                })
            },
            &motif,
            oui,
        )?;
        let edge = match &applique.envelope.command {
            Command::Change(Change::Link { id, .. }) => *id,
            _ => unreachable!("commande construite ci-dessus"),
        };
        contexte.ecrire(&compte_rendu(
            &format!("Relation créée : {edge}"),
            &applique,
        ))
    }
}

/// Changement de statut : motif obligatoire (D12), confirmation si l'opération est critique.
pub(crate) fn statut(
    arguments: Arguments,
    contexte: &mut Contexte<'_>,
    construire: impl FnOnce(memoire_domain::NodeId, &str) -> Result<Change, Echec>,
) -> Result<(), Echec> {
    let mut arguments = arguments;
    let motif = arguments.requise("--motif")?;
    let oui = arguments.drapeau("--oui")?;
    let saisie = arguments.positionnel("NOTE")?;
    let valeur = arguments.positionnel("VALEUR")?;
    arguments.fin()?;
    let mut depot = contexte.ouvrir()?;
    let cible = note(&depot, &saisie)?;
    let change = construire(cible, &valeur)?;
    let applique = preparer_et_appliquer(
        contexte,
        &mut depot,
        |_| Command::Change(change),
        &motif,
        oui,
    )?;
    let mut texte = String::from("Statut enregistré.\n");
    if applique.outcome.forked {
        texte.push_str(&format!("Branche créée : {}\n", applique.outcome.branch));
    }
    contexte.ecrire(&texte)
}

pub(crate) struct PoserVerdict;

impl Commande for PoserVerdict {
    fn nom(&self) -> &'static str {
        "verdict"
    }
    fn usage(&self) -> &'static str {
        "NOTE non-verifiee|verifiee|refutee --motif M [--oui]"
    }
    fn resume(&self) -> &'static str {
        "pose l'état de vérification d'une note"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        statut(arguments, contexte, |node, saisie| {
            Ok(Change::SetVerdict {
                scope: DEFAULT_SCOPE,
                node,
                verdict: valeur(saisie, VERDICTS, "verdict")?,
            })
        })
    }
}

pub(crate) struct PoserDecision;

impl Commande for PoserDecision {
    fn nom(&self) -> &'static str {
        "decision"
    }
    fn usage(&self) -> &'static str {
        "NOTE proposee|retenue|plus-retenue|remplacee --motif M [--oui]"
    }
    fn resume(&self) -> &'static str {
        "pose l'état de décision d'une note"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        statut(arguments, contexte, |node, saisie| {
            Ok(Change::SetDecision {
                scope: DEFAULT_SCOPE,
                node,
                decision: valeur(saisie, DECISIONS, "décision")?,
            })
        })
    }
}

pub(crate) struct PoserImplementation;

impl Commande for PoserImplementation {
    fn nom(&self) -> &'static str {
        "implementation"
    }
    fn usage(&self) -> &'static str {
        "NOTE non-commencee|en-cours|implementee --motif M [--oui]"
    }
    fn resume(&self) -> &'static str {
        "pose l'état d'implémentation d'une note"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        statut(arguments, contexte, |node, saisie| {
            Ok(Change::SetImplementation {
                scope: DEFAULT_SCOPE,
                node,
                implementation: valeur(saisie, IMPLEMENTATIONS, "implémentation")?,
            })
        })
    }
}

pub(crate) struct NoteReviser;

impl Commande for NoteReviser {
    fn nom(&self) -> &'static str {
        "note reviser"
    }
    fn usage(&self) -> &'static str {
        "NOTE --texte X --nature mineure|substantielle [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "révise le texte d'une note ; la nature est déclarée, jamais implicite (D5)"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let texte = arguments.requise("--texte")?;
        let nature = arguments.requise("--nature")?;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let oui = arguments.drapeau("--oui")?;
        let saisie = arguments.positionnel("NOTE")?;
        arguments.fin()?;
        let kind = valeur(&nature, NATURES, "nature")?;
        let mut depot = contexte.ouvrir()?;
        let node = note(&depot, &saisie)?;
        let change = Change::Revise {
            node,
            text: texte,
            kind,
        };
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |_| Command::Change(change),
            &motif,
            oui,
        )?;
        contexte.ecrire(&compte_rendu("Révision enregistrée.", &applique))
    }
}

/// Relation annotée entre deux notes : restriction ou désaccord.
fn relation_annotee(
    arguments: Arguments,
    contexte: &mut Contexte<'_>,
    option: &str,
    construire: fn(
        memoire_domain::EdgeId,
        memoire_domain::NodeId,
        memoire_domain::NodeId,
        String,
    ) -> Change,
) -> Result<(), Echec> {
    let mut arguments = arguments;
    let annotation = arguments.requise(option)?;
    let motif = arguments.option("--motif")?.unwrap_or_default();
    let oui = arguments.drapeau("--oui")?;
    let origine = arguments.positionnel("ORIGINE")?;
    let destination = arguments.positionnel("DESTINATION")?;
    arguments.fin()?;
    let mut depot = contexte.ouvrir()?;
    let from = note(&depot, &origine)?;
    let to = note(&depot, &destination)?;
    let applique = preparer_et_appliquer(
        contexte,
        &mut depot,
        |fresh| Command::Change(construire(fresh.edge(), from, to, annotation)),
        &motif,
        oui,
    )?;
    let edge = match &applique.envelope.command {
        Command::Change(Change::Restrict { id, .. } | Change::Contradict { id, .. }) => *id,
        _ => unreachable!("commande construite ci-dessus"),
    };
    contexte.ecrire(&compte_rendu(
        &format!("Relation créée : {edge}"),
        &applique,
    ))
}

pub(crate) struct Restreindre;

impl Commande for Restreindre {
    fn nom(&self) -> &'static str {
        "restreindre"
    }
    fn usage(&self) -> &'static str {
        "ORIGINE DESTINATION --condition C [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "limite l'applicabilité de la destination à une condition, sans verdict"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        relation_annotee(
            arguments,
            contexte,
            "--condition",
            |id, from, to, condition| Change::Restrict {
                scope: DEFAULT_SCOPE,
                id,
                from,
                to,
                condition,
            },
        )
    }
}

pub(crate) struct Contredire;

impl Commande for Contredire {
    fn nom(&self) -> &'static str {
        "contredire"
    }
    fn usage(&self) -> &'static str {
        "ORIGINE DESTINATION --justification J [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "enregistre un désaccord assumé, sans effet sur la destination"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        relation_annotee(
            arguments,
            contexte,
            "--justification",
            |id, from, to, justification| Change::Contradict {
                scope: DEFAULT_SCOPE,
                id,
                from,
                to,
                justification,
            },
        )
    }
}

pub(crate) struct Acquitter;

impl Commande for Acquitter {
    fn nom(&self) -> &'static str {
        "acquitter"
    }
    fn usage(&self) -> &'static str {
        "DEPENDANT CAUSE --motif M [--oui]"
    }
    fn resume(&self) -> &'static str {
        "confirme qu'une note reste pertinente malgré une cause de réexamen"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let motif = arguments.requise("--motif")?;
        let oui = arguments.drapeau("--oui")?;
        let dependant = arguments.positionnel("DEPENDANT")?;
        let cause = arguments.positionnel("CAUSE")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let dependent = note(&depot, &dependant)?;
        let cause = note(&depot, &cause)?;
        let change = Change::Acknowledge {
            scope: DEFAULT_SCOPE,
            dependent,
            cause,
        };
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |_| Command::Change(change),
            &motif,
            oui,
        )?;
        contexte.ecrire(&compte_rendu("Acquittement enregistré.", &applique))
    }
}

pub(crate) struct Retirer;

impl Commande for Retirer {
    fn nom(&self) -> &'static str {
        "retirer"
    }
    fn usage(&self) -> &'static str {
        "NOTE [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "retire une note ou un échange des ressources actuelles ; ses dépendants passent à réexaminer"
    }
    // D18, point 4 ; D24 : archivage, sans suppression ; il se défait par « annuler ».
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let oui = arguments.drapeau("--oui")?;
        let saisie = arguments.positionnel("NOTE")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let node = note(&depot, &saisie)?;
        let change = Change::Archive {
            scope: DEFAULT_SCOPE,
            node,
        };
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |_| Command::Change(change),
            &motif,
            oui,
        )?;
        contexte.ecrire(&compte_rendu("Retrait enregistré.", &applique))
    }
}
