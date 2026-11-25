//! Échanges et citations (CLI.md, section 8 ; D18, D24).

use memoire_domain::{
    Change, CitedSource, Command, NodeId, Part, Relation, Snapshot, DEFAULT_SCOPE,
};

use super::notes::{notes, ROLES, ROLES_CREABLES, VERDICTS};
use super::{compte_rendu, mot, note, preparer_et_appliquer, valeur, Commande, Contexte};
use crate::arguments::Arguments;
use crate::Echec;

fn lire(chemin: &str) -> Result<String, Echec> {
    std::fs::read_to_string(chemin).map_err(|e| Echec::Stockage(format!("{chemin} : {e}")))
}

pub(crate) struct EchangeColler;

impl Commande for EchangeColler {
    fn nom(&self) -> &'static str {
        "echange coller"
    }
    fn usage(&self) -> &'static str {
        "--titre T --reponse-fichier R [--question-fichier Q] [--precedent ECHANGE]"
    }
    fn resume(&self) -> &'static str {
        "enregistre un échange octet pour octet ; sans question, il reste incomplet"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let titre = arguments.requise("--titre")?;
        let reponse = arguments.requise("--reponse-fichier")?;
        let question = arguments.option("--question-fichier")?;
        let precedent = arguments.option("--precedent")?;
        arguments.fin()?;
        let answer = lire(&reponse)?;
        let question = question.as_deref().map(lire).transpose()?;
        let mut depot = contexte.ouvrir()?;
        let previous = precedent.as_deref().map(|p| note(&depot, p)).transpose()?;
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |fresh| {
                Command::Change(Change::CreateExchange {
                    scope: DEFAULT_SCOPE,
                    id: fresh.node(),
                    title: titre,
                    question,
                    answer,
                    previous,
                })
            },
            "",
            false,
        )?;
        let id = match &applique.envelope.command {
            Command::Change(Change::CreateExchange { id, .. }) => *id,
            _ => unreachable!("commande construite ci-dessus"),
        };
        contexte.ecrire(&compte_rendu(&format!("Échange créé : {id}"), &applique))
    }
}

pub(crate) struct EchangeQuestion;

impl Commande for EchangeQuestion {
    fn nom(&self) -> &'static str {
        "echange question"
    }
    fn usage(&self) -> &'static str {
        "ECHANGE --fichier Q"
    }
    fn resume(&self) -> &'static str {
        "complète la question d'un échange incomplet"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let fichier = arguments.requise("--fichier")?;
        let saisie = arguments.positionnel("ECHANGE")?;
        arguments.fin()?;
        let question = lire(&fichier)?;
        let mut depot = contexte.ouvrir()?;
        let exchange = note(&depot, &saisie)?;
        let change = Change::SetQuestion { exchange, question };
        let applique =
            preparer_et_appliquer(contexte, &mut depot, |_| Command::Change(change), "", false)?;
        contexte.ecrire(&compte_rendu("Question enregistrée.", &applique))
    }
}

const PARTIES: &[(&str, Part)] = &[("question", Part::Question), ("reponse", Part::Answer)];

/// Source lue : partie d'un échange, ou version d'une note.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lue {
    Partie(Part),
    Version(usize),
}

/// Texte exact d'une source (VISIONNEUR.md, section 3) : la partie d'un échange (la réponse par
/// défaut), ou une version d'une note (la courante par défaut). Les arguments sans objet sont des
/// erreurs d'usage.
fn texte_source(
    etat: &Snapshot,
    id: NodeId,
    partie: Option<&str>,
    version: Option<&str>,
) -> Result<(String, Lue), Echec> {
    if let Ok(vue) = etat.exchange(id) {
        if version.is_some() {
            return Err(Echec::Usage(
                "--version ne vaut que pour une note".to_owned(),
            ));
        }
        let part = valeur(partie.unwrap_or("reponse"), PARTIES, "partie")?;
        let texte = match part {
            Part::Question => vue.question().ok_or_else(|| {
                Echec::Usage("question absente : l'échange est incomplet".to_owned())
            })?,
            Part::Answer => vue.answer(),
        };
        return Ok((texte.to_owned(), Lue::Partie(part)));
    }
    if partie.is_some() {
        return Err(Echec::Usage(
            "--partie ne vaut que pour un échange".to_owned(),
        ));
    }
    let versions = etat.revisions(id).map_err(Echec::Refus)?;
    let derniere = versions.len().saturating_sub(1);
    let rang = match version {
        None => derniere,
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|k| *k <= derniere)
            .ok_or_else(|| {
                Echec::Usage(format!("version {v} absente : versions 0 à {derniere}"))
            })?,
    };
    let texte = versions
        .get(rang)
        .map(|r| r.text().to_owned())
        .unwrap_or_default();
    Ok((texte, Lue::Version(rang)))
}

pub(crate) struct Source;

impl Commande for Source {
    fn nom(&self) -> &'static str {
        "source"
    }
    fn usage(&self) -> &'static str {
        "NOTE [--partie question|reponse] [--version N]"
    }
    fn resume(&self) -> &'static str {
        "affiche le texte exact d'une partie d'un échange ou d'une version d'une note, octet pour \
         octet"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let partie = arguments.option("--partie")?;
        let version = arguments.option("--version")?;
        let saisie = arguments.positionnel("NOTE")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let id = note(&depot, &saisie)?;
        let (texte, _) = texte_source(
            depot.repository().snapshot(),
            id,
            partie.as_deref(),
            version.as_deref(),
        )?;
        contexte.ecrire(&texte)
    }
}

pub(crate) struct Citations;

impl Commande for Citations {
    fn nom(&self) -> &'static str {
        "citations"
    }
    fn usage(&self) -> &'static str {
        "NOTE [--partie question|reponse] [--version N]"
    }
    fn resume(&self) -> &'static str {
        "liste les citations de cette source : identifiant, début et fin en octets, statut"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let partie = arguments.option("--partie")?;
        let version = arguments.option("--version")?;
        let saisie = arguments.positionnel("NOTE")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let id = note(&depot, &saisie)?;
        let etat = depot.repository().snapshot();
        let (_, lue) = texte_source(etat, id, partie.as_deref(), version.as_deref())?;
        let graphe = etat.graph(DEFAULT_SCOPE).map_err(Echec::Refus)?;
        let mut lignes = Vec::new();
        for (citation, noeud) in graphe.nodes() {
            let Ok(vue) = etat.citation(citation) else {
                continue;
            };
            let meme = match vue.source {
                CitedSource::Exchange { exchange, part } => {
                    exchange == id && lue == Lue::Partie(part)
                }
                CitedSource::Note { note, version } => note == id && lue == Lue::Version(version),
            };
            if !meme {
                continue;
            }
            let mut statut = noeud.verdict().map_or("-", |v| mot(v, VERDICTS)).to_owned();
            if vue.source_revised {
                statut.push_str(",source-revisee");
            }
            lignes.push((vue.start, citation, vue.end, statut));
        }
        lignes.sort();
        let texte: String = lignes
            .into_iter()
            .map(|(debut, id, fin, statut)| format!("{id}\t{debut}\t{fin}\t{statut}\n"))
            .collect();
        contexte.ecrire(&texte)
    }
}

/// Plage « A-B » en octets, fin exclue.
fn plage(saisie: &str) -> Result<(usize, usize), Echec> {
    saisie
        .split_once('-')
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .ok_or_else(|| Echec::Usage(format!("plage invalide : {saisie} (forme A-B, en octets)")))
}

pub(crate) struct Citer;

/// Options de relation de `citer` (CENTRE.md, section 3) : relation, et vrai si elle part de la
/// note désignée vers la citation (sinon de la citation vers la note).
const LIENS_DE_CITATION: [(&str, Relation, bool); 4] = [
    ("--soutient", Relation::DependsOn, true),
    ("--inspire", Relation::DevelopedFrom, true),
    ("--classe-dans", Relation::ClassifiedIn, false),
    ("--lien", Relation::Related, false),
];

impl Commande for Citer {
    fn nom(&self) -> &'static str {
        "citer"
    }
    fn usage(&self) -> &'static str {
        "NOTE (--texte X | --plage A-B) [--partie question|reponse] [--version N] \
         [--occurrence N] [--soutient NOTES] [--inspire NOTES] [--classe-dans NOTES] \
         [--lien NOTES] [--note TITRE] [--role categorie|raisonnement|implementation|note] \
         [--texte-note X] [--reserve TITRE] [--cible C] [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "cite le texte exact X, ou la plage A-B en octets, d'un échange (sa réponse par défaut) \
         ou d'une note (sa version courante par défaut), et relie la citation à sa source ; avec \
         des liens, une note ou une réserve, relie la citation et crée ces objets en une seule \
         opération"
    }
    // D18, D24 : la citation et sa relation « développé à partir de » vers la source forment un
    // seul lot (D22) ; CENTRE.md : les relations, la note et la réserve demandées s'y ajoutent.
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let texte = arguments.option("--texte")?;
        let plage_saisie = arguments.option("--plage")?;
        let partie = arguments.option("--partie")?;
        let version = arguments.option("--version")?;
        let occurrence = arguments.option("--occurrence")?;
        let mut listes = Vec::new();
        for (option, relation, vers_citation) in LIENS_DE_CITATION {
            if let Some(liste) = arguments.option(option)? {
                listes.push((option, relation, vers_citation, liste));
            }
        }
        let titre_note = arguments.option("--note")?;
        let role = arguments.option("--role")?;
        let texte_note = arguments.option("--texte-note")?;
        let titre_reserve = arguments.option("--reserve")?;
        let cible_reserve = arguments.option("--cible")?;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let confirme = arguments.drapeau("--oui")?;
        let saisie = arguments.positionnel("NOTE")?;
        arguments.fin()?;
        if titre_note.is_none() && (role.is_some() || texte_note.is_some()) {
            return Err(Echec::Usage(
                "--role et --texte-note ne valent qu'avec --note".to_owned(),
            ));
        }
        if titre_reserve.is_none() && cible_reserve.is_some() {
            return Err(Echec::Usage("--cible ne vaut qu'avec --reserve".to_owned()));
        }
        let kind = valeur(
            role.as_deref().unwrap_or("raisonnement"),
            &ROLES[..ROLES_CREABLES],
            "rôle",
        )?;
        let mut depot = contexte.ouvrir()?;
        let cible = note(&depot, &saisie)?;
        let mut liens = Vec::new();
        for (option, relation, vers_citation, liste) in &listes {
            for autre in notes(&depot, option, liste)? {
                liens.push((*relation, *vers_citation, autre));
            }
        }
        let (source, lue) = texte_source(
            depot.repository().snapshot(),
            cible,
            partie.as_deref(),
            version.as_deref(),
        )?;
        let (start, end) = match (texte, plage_saisie) {
            (Some(texte), None) => {
                let occurrence = occurrence
                    .map(|n| match n.parse::<usize>() {
                        Ok(k) if k >= 1 => Ok(k),
                        _ => Err(Echec::Usage(format!(
                            "occurrence invalide : {n} (entier à partir de 1)"
                        ))),
                    })
                    .transpose()?;
                debut_du_texte(&source, &texte, occurrence)
                    .map(|debut| (debut, debut + texte.len()))?
            }
            (None, Some(saisie)) => {
                if occurrence.is_some() {
                    return Err(Echec::Usage(
                        "--occurrence ne vaut qu'avec --texte".to_owned(),
                    ));
                }
                plage(&saisie)?
            }
            _ => {
                return Err(Echec::Usage(
                    "préciser --texte ou --plage, l'un ou l'autre".to_owned(),
                ))
            }
        };
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |fresh| {
                let id = fresh.node();
                let citation = match lue {
                    Lue::Partie(part) => Change::CreatePassage {
                        scope: DEFAULT_SCOPE,
                        id,
                        exchange: cible,
                        part,
                        start,
                        end,
                    },
                    Lue::Version(version) => Change::CiteNote {
                        scope: DEFAULT_SCOPE,
                        id,
                        note: cible,
                        version,
                        start,
                        end,
                    },
                };
                let mut lien = |from, to, relation| Change::Link {
                    scope: DEFAULT_SCOPE,
                    id: fresh.edge(),
                    from,
                    to,
                    relation,
                };
                let mut elements = vec![citation, lien(id, cible, Relation::DevelopedFrom)];
                for (relation, vers_citation, autre) in liens {
                    elements.push(if vers_citation {
                        lien(autre, id, relation)
                    } else {
                        lien(id, autre, relation)
                    });
                }
                if let Some(titre) = titre_note {
                    let nouvelle = fresh.node();
                    elements.push(Change::CreateNode {
                        scope: DEFAULT_SCOPE,
                        id: nouvelle,
                        kind,
                        title: titre,
                        text: texte_note.unwrap_or_default(),
                    });
                    elements.push(Change::Link {
                        scope: DEFAULT_SCOPE,
                        id: fresh.edge(),
                        from: nouvelle,
                        to: id,
                        relation: Relation::DevelopedFrom,
                    });
                }
                if let Some(titre) = titre_reserve {
                    elements.push(Change::CreateReserve {
                        scope: DEFAULT_SCOPE,
                        id: fresh.node(),
                        passage: id,
                        title: titre,
                        target: cible_reserve,
                    });
                }
                Command::Lot(elements)
            },
            &motif,
            confirme,
        )?;
        let Command::Lot(elements) = &applique.envelope.command else {
            unreachable!("lot construit ci-dessus")
        };
        let mut lignes = Vec::new();
        // Objet créé à la ligne précédente : la citation, puis la nouvelle note.
        let mut cree = cible;
        for element in elements {
            match element {
                Change::CreatePassage { id, .. } | Change::CiteNote { id, .. } => {
                    cree = *id;
                    lignes.push(format!("Citation créée : {id} (octets {start} à {end})"));
                }
                // Deux termes quand la relation part de l'objet créé (comme pour « note creer »),
                // trois sinon (CENTRE.md, section 3).
                Change::Link {
                    id,
                    from,
                    to,
                    relation,
                    ..
                } => {
                    let type_ = mot(*relation, RELATIONS_DE_CITATION);
                    lignes.push(if *from == cree {
                        format!("Relation créée : {id} ({type_} {to})")
                    } else {
                        format!("Relation créée : {id} ({from} {type_} {to})")
                    });
                }
                Change::CreateNode { id, .. } => {
                    cree = *id;
                    lignes.push(format!("Note créée : {id}"));
                }
                Change::CreateReserve { id, .. } => lignes.push(format!("Réserve créée : {id}")),
                _ => {}
            }
        }
        contexte.ecrire(&compte_rendu(&lignes.join("\n"), &applique))
    }
}

/// Mots des relations que `citer` crée.
const RELATIONS_DE_CITATION: &[(&str, Relation)] = &[
    ("developpe-a-partir-de", Relation::DevelopedFrom),
    ("depend-de", Relation::DependsOn),
    ("classe-dans", Relation::ClassifiedIn),
    ("lien", Relation::Related),
];

/// Début de l'occurrence choisie d'un texte non vide dans la source.
fn debut_du_texte(source: &str, texte: &str, occurrence: Option<usize>) -> Result<usize, Echec> {
    if texte.is_empty() {
        return Err(Echec::Usage("texte de citation vide".to_owned()));
    }
    let positions: Vec<usize> = source.match_indices(texte).map(|(i, _)| i).collect();
    match (positions.as_slice(), occurrence) {
        ([], _) => Err(Echec::Usage(
            "texte introuvable dans cette source".to_owned(),
        )),
        ([seule], None) => Ok(*seule),
        (plusieurs, None) => Err(Echec::Usage(format!(
            "texte présent {} fois : préciser --occurrence",
            plusieurs.len()
        ))),
        (toutes, Some(k)) => toutes.get(k - 1).copied().ok_or_else(|| {
            Echec::Usage(format!(
                "occurrence {k} absente : {} occurrence(s)",
                toutes.len()
            ))
        }),
    }
}
