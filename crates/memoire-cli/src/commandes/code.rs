//! Ancrages dans le code et reprise depuis un repère (CLI.md, section 8). Le logiciel n'écrit
//! jamais dans le code (D9) : il lit le fichier pour en copier l'extrait.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use memoire_domain::{
    find_markers, relative_path, AnchorMatch, AnchorState, Change, Command, Focus, Kind, NodeId,
    Occurrence, Relation, DEFAULT_SCOPE, MARKER_PREFIX,
};

use super::base::champ_tsv;

use super::notes::{DECISIONS, IMPLEMENTATIONS, ROLES, VERDICTS};
use super::reserves::ETATS;
use super::{compte_rendu, designation, mot, note, preparer_et_appliquer, Commande, Contexte};
use crate::arguments::Arguments;
use crate::Echec;

/// Extrait d'une fonction : de la seule occurrence de la signature jusqu'à la première ligne qui
/// commence par « } », incluse.
fn fonction(code: &str, signature: &str) -> Result<String, Echec> {
    let debuts: Vec<usize> = code.match_indices(signature).map(|(i, _)| i).collect();
    let debut = match debuts.as_slice() {
        [un] => *un,
        [] => return Err(Echec::Usage(format!("signature introuvable : {signature}"))),
        plusieurs => {
            return Err(Echec::Usage(format!(
                "signature présente {} fois : préciser --lignes",
                plusieurs.len()
            )))
        }
    };
    let fin = code[debut..]
        .find("\n}")
        .map(|i| debut + i + 2)
        .ok_or_else(|| Echec::Usage(format!("fin de fonction introuvable après : {signature}")))?;
    Ok(code[debut..fin].to_owned())
}

/// Extrait d'une plage « A-B » de lignes, inclusive, numérotée à partir de 1.
fn lignes(code: &str, plage: &str) -> Result<String, Echec> {
    let invalide = || Echec::Usage(format!("plage invalide : {plage} (forme A-B, 1 ≤ A ≤ B)"));
    let (a, b) = plage.split_once('-').ok_or_else(invalide)?;
    let (a, b): (usize, usize) = (
        a.parse().map_err(|_| invalide())?,
        b.parse().map_err(|_| invalide())?,
    );
    let toutes: Vec<&str> = code.split_inclusive('\n').collect();
    if a == 0 || a > b || b > toutes.len() {
        return Err(Echec::Usage(format!(
            "plage {plage} hors du fichier ({} lignes)",
            toutes.len()
        )));
    }
    Ok(toutes[a - 1..b].concat())
}

pub(crate) struct Ancrer;

impl Commande for Ancrer {
    fn nom(&self) -> &'static str {
        "ancrer"
    }
    fn usage(&self) -> &'static str {
        "NOTE --fichier F (--fonction SIGNATURE | --lignes A-B) [--git COMMIT]"
    }
    fn resume(&self) -> &'static str {
        "ancre une note dans le code et affiche le repère à copier ; le fichier n'est pas modifié"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let fichier = arguments.requise("--fichier")?;
        let signature = arguments.option("--fonction")?;
        let plage = arguments.option("--lignes")?;
        let git = arguments.option("--git")?;
        let saisie = arguments.positionnel("NOTE")?;
        arguments.fin()?;
        let code = std::fs::read_to_string(&fichier)
            .map_err(|e| Echec::Stockage(format!("{fichier} : {e}")))?;
        let extrait = match (signature, plage) {
            (Some(s), None) => fonction(&code, &s)?,
            (None, Some(p)) => lignes(&code, &p)?,
            _ => {
                return Err(Echec::Usage(
                    "préciser exactement l'un de --fonction et --lignes".to_owned(),
                ))
            }
        };
        let mut depot = contexte.ouvrir()?;
        let node = note(&depot, &saisie)?;
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |fresh| {
                Command::Change(Change::AnchorToCode {
                    id: fresh.anchor(),
                    node,
                    path: fichier,
                    excerpt: extrait,
                    git,
                })
            },
            "",
            false,
        )?;
        let id = match &applique.envelope.command {
            Command::Change(Change::AnchorToCode { id, .. }) => *id,
            _ => unreachable!("commande construite ci-dessus"),
        };
        let repere = depot
            .repository()
            .marker(DEFAULT_SCOPE, node)
            .map_err(Echec::Refus)?;
        let texte = compte_rendu(&format!("Ancrage créé : {id}"), &applique);
        contexte.ecrire(&format!("{texte}Repère à copier : {repere}\n"))
    }
}

pub(crate) struct Repere;

impl Commande for Repere {
    fn nom(&self) -> &'static str {
        "repere"
    }
    fn usage(&self) -> &'static str {
        "LIGNE"
    }
    fn resume(&self) -> &'static str {
        "depuis une ligne de code contenant un repère : note, décision, sources, réserves, ancrages"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let ligne = arguments.positionnel("LIGNE")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let depot = depot.repository();
        // Toute ligne qui contient une marque, quelle que soit la syntaxe de commentaire
        // (RETROUVER.md, section 2) : la détection est celle de l'indexeur.
        let notes: Vec<NodeId> = find_markers(&ligne)
            .into_iter()
            .filter_map(|m| m.node)
            .collect();
        if notes.is_empty() {
            return Err(Echec::Refus(memoire_domain::Error::NotAMarker));
        }
        let mut fiches = Vec::new();
        for note in notes {
            let id = depot
                .resolve_marker(DEFAULT_SCOPE, &format!("{MARKER_PREFIX}{note}"))
                .map_err(Echec::Refus)?;
            fiches.push(fiche_de_note(depot, id)?);
        }
        contexte.ecrire(&fiches.join("\n"))
    }
}

/// Fiche d'une note atteinte par une marque : rôle, statuts, appuis, désaccords, réserves,
/// ancrages.
fn fiche_de_note(depot: &memoire_app::StoredRepository, id: NodeId) -> Result<String, Echec> {
    let etat = depot.snapshot();
    let graphe = etat.graph(DEFAULT_SCOPE).map_err(Echec::Refus)?;
    let noeud = graphe.node(id).map_err(Echec::Refus)?;
    let actives = |relation: Relation| {
        graphe
            .edges()
            .filter(move |(_, e)| e.active() && e.relation() == relation)
    };
    let mut texte = format!(
        "Note : {} ({id})\nRôle : {}\n",
        noeud.title(),
        mot(noeud.kind(), ROLES)
    );
    if let Some(d) = noeud.decision() {
        texte.push_str(&format!("Décision : {}\n", mot(d, DECISIONS)));
    }
    if let Some(v) = noeud.verdict() {
        texte.push_str(&format!("Vérification : {}\n", mot(v, VERDICTS)));
    }
    if let Some(i) = noeud.implementation() {
        texte.push_str(&format!("Implémentation : {}\n", mot(i, IMPLEMENTATIONS)));
    }
    let section = |texte: &mut String, titre: &str, lignes: Vec<String>| {
        texte.push_str(&format!("{titre} :\n"));
        if lignes.is_empty() {
            texte.push_str("  (aucune)\n");
        }
        for l in lignes {
            texte.push_str(&format!("  - {l}\n"));
        }
    };
    let sources = actives(Relation::DependsOn)
        .filter(|(_, e)| e.from() == id)
        .map(|(_, e)| designation(etat, e.to()))
        .collect();
    section(&mut texte, "S'appuie sur", sources);
    let desaccords = actives(Relation::Contradicts)
        .filter(|(_, e)| e.from() == id)
        .map(|(_, e)| {
            format!(
                "{} — {}",
                designation(etat, e.to()),
                e.annotation().unwrap_or_default()
            )
        })
        .collect();
    section(&mut texte, "Désaccords", desaccords);
    let reserve = |r: NodeId| {
        let statut = graphe
            .node(r)
            .ok()
            .and_then(|n| n.reserve_status())
            .map_or("?", |s| mot(s, ETATS));
        let statut = statut
            .replace("traitee-a-verifier", "traitée à vérifier")
            .replace("resolue", "résolue");
        let cible = etat
            .reserve(r)
            .ok()
            .and_then(|v| v.target)
            .map(|c| format!(" — cible : {c}"))
            .unwrap_or_default();
        format!("{} — {statut}{cible}", designation(etat, r))
    };
    let directes: BTreeSet<NodeId> = actives(Relation::AppliesTo)
        .filter(|(_, e)| e.to() == id)
        .map(|(_, e)| e.from())
        .collect();
    section(
        &mut texte,
        "Réserves de la note",
        directes.iter().map(|r| reserve(*r)).collect(),
    );
    let categories: BTreeSet<NodeId> = actives(Relation::ClassifiedIn)
        .filter(|(_, e)| e.from() == id)
        .map(|(_, e)| e.to())
        .collect();
    let membres: BTreeSet<NodeId> = actives(Relation::ClassifiedIn)
        .filter(|(_, e)| categories.contains(&e.to()))
        .map(|(_, e)| e.from())
        .collect();
    let voisines: Vec<String> = graphe
        .nodes()
        .filter(|(r, n)| n.kind() == Kind::Reserve && !directes.contains(r))
        .filter(|(r, _)| {
            let origine = etat.reserve(*r).map(|v| v.exchange).ok();
            let vise_un_membre = actives(Relation::AppliesTo)
                .any(|(_, e)| e.from() == *r && membres.contains(&e.to()));
            origine.is_some_and(|o| membres.contains(&o)) || vise_un_membre
        })
        .map(|(r, _)| reserve(r))
        .collect();
    section(&mut texte, "Réserves de ses catégories", voisines);
    let ancrages = etat
        .anchors_of(id)
        .into_iter()
        .map(|(_, a)| {
            format!(
                "{} ({})",
                a.path(),
                if a.active() { "actif" } else { "retiré" }
            )
        })
        .collect();
    section(&mut texte, "Ancrages", ancrages);
    Ok(texte)
}

/// Taille au-delà de laquelle un fichier n'est pas lu (INDEXEUR.md, section 4).
const TAILLE_MAXIMALE: u64 = 4 * 1024 * 1024;

/// Dossiers jamais parcourus, en plus des dossiers cachés.
const DOSSIERS_SAUTES: [&str; 2] = ["target", "node_modules"];

/// Fichiers lus sous `racine`, par chemin relatif séparé par `/`, et nombre de fichiers ignorés
/// (illisibles, non UTF-8, trop gros, liens). L'ordre est celui des chemins.
fn parcourir(
    racine: &Path,
    dossier: &Path,
    extensions: Option<&BTreeSet<String>>,
    fichiers: &mut BTreeMap<String, String>,
    ignores: &mut usize,
) -> std::io::Result<()> {
    let mut entrees = std::fs::read_dir(dossier)?.collect::<Result<Vec<_>, _>>()?;
    entrees.sort_by_key(std::fs::DirEntry::file_name);
    for entree in entrees {
        let nom = entree.file_name().to_string_lossy().into_owned();
        let chemin = entree.path();
        // Les liens ne sont pas suivis : un lien n'est ni un dossier ni un fichier ordinaire.
        let meta = std::fs::symlink_metadata(&chemin)?;
        if meta.is_dir() {
            if !nom.starts_with('.') && !DOSSIERS_SAUTES.contains(&nom.as_str()) {
                // Un sous-dossier illisible compte comme un fichier ignoré.
                if parcourir(racine, &chemin, extensions, fichiers, ignores).is_err() {
                    *ignores += 1;
                }
            }
            continue;
        }
        let retenu = extensions.is_none_or(|liste| {
            chemin
                .extension()
                .is_some_and(|e| liste.contains(&*e.to_string_lossy()))
        });
        if !retenu {
            continue;
        }
        let texte = (meta.is_file() && meta.len() <= TAILLE_MAXIMALE)
            .then(|| std::fs::read(&chemin).ok())
            .flatten()
            .and_then(|octets| String::from_utf8(octets).ok());
        let Some(texte) = texte else {
            *ignores += 1;
            continue;
        };
        let relatif = chemin
            .strip_prefix(racine)
            .unwrap_or(&chemin)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        fichiers.insert(relatif, texte);
    }
    Ok(())
}

pub(crate) struct Indexer;

impl Commande for Indexer {
    fn nom(&self) -> &'static str {
        "indexer"
    }
    fn usage(&self) -> &'static str {
        "DOSSIER [--extensions LISTE]"
    }
    fn resume(&self) -> &'static str {
        "parcourt le code de DOSSIER et rend l'état des repères et des ancrages : retrouvés, \
         modifiés, déplacés, ambigus, perdus, inconnus, non ancrés ; n'écrit rien"
    }
    // INDEXEUR.md : lecture seule (INV-CODE-04, INV-CODE-06) ; le classement est celui du noyau.
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let extensions = arguments.option("--extensions")?.map(|liste| {
            liste
                .split(',')
                .map(|e| e.trim().trim_start_matches('.').to_owned())
                .filter(|e| !e.is_empty())
                .collect::<BTreeSet<String>>()
        });
        let saisie = arguments.positionnel("DOSSIER")?;
        arguments.fin()?;
        let racine = Path::new(&saisie);
        if !racine.is_dir() {
            return Err(Echec::Usage(format!("{saisie} n'est pas un dossier")));
        }
        let racine = std::path::absolute(racine)?;
        let mut fichiers = BTreeMap::new();
        let mut ignores = 0;
        parcourir(
            &racine,
            &racine,
            extensions.as_ref(),
            &mut fichiers,
            &mut ignores,
        )?;
        let depot = contexte.ouvrir()?;
        let repo = depot.repository();
        let etat = repo.snapshot();
        let mut occurrences = Vec::new();
        let mut mal_formes = Vec::new();
        let mut porteurs = 0;
        for (chemin, texte) in &fichiers {
            let trouves = find_markers(texte);
            porteurs += usize::from(!trouves.is_empty());
            for repere in trouves {
                match repere.node {
                    Some(node) => occurrences.push(Occurrence {
                        path: chemin.clone(),
                        line: repere.line,
                        node,
                    }),
                    None => mal_formes.push(format!("{chemin}:{}", repere.line)),
                }
            }
        }
        let rapport = etat
            .code_report(
                DEFAULT_SCOPE,
                &racine.to_string_lossy(),
                &occurrences,
                &fichiers,
            )
            .map_err(Echec::Refus)?;
        let lieu = |o: &Occurrence| format!("{}:{}", o.path, o.line);
        let mut sections: Vec<(&str, Vec<String>)> = vec![
            ("Retrouvés", Vec::new()),
            ("Modifiés (le code ancré a changé)", Vec::new()),
            ("Déplacés (chemin de l'ancrage périmé)", Vec::new()),
            (
                "Ambigus (plusieurs occurrences, aucune choisie)",
                Vec::new(),
            ),
            ("Perdus (ancrage sans repère dans le code)", Vec::new()),
        ];
        for ligne in &rapport.anchors {
            let note = designation(etat, ligne.node);
            let (rang, texte) = match &ligne.state {
                AnchorState::Found(o) => (0, format!("{note} — {}", lieu(o))),
                AnchorState::Modified(o) => (
                    1,
                    format!(
                        "{note} — {} — extrait ancré absent de {}",
                        lieu(o),
                        ligne.path
                    ),
                ),
                AnchorState::Moved { at, excerpt_found } => (
                    2,
                    format!(
                        "{note} — {} — ancré dans {}, extrait {}",
                        lieu(at),
                        ligne.path,
                        if *excerpt_found {
                            "retrouvé"
                        } else {
                            "absent"
                        }
                    ),
                ),
                AnchorState::Ambiguous(toutes) => (
                    3,
                    format!(
                        "{note} — {}",
                        toutes.iter().map(lieu).collect::<Vec<_>>().join(", ")
                    ),
                ),
                AnchorState::Lost => (4, format!("{note} — ancré dans {}", ligne.path)),
            };
            sections[rang].1.push(texte);
        }
        let mut inconnus = Vec::new();
        for o in &rapport.unknown {
            let ailleurs = repo.branches_containing(o.node).map_err(Echec::Refus)?;
            let indication = if ailleurs.is_empty() {
                String::new()
            } else {
                let noms: Vec<String> = ailleurs.iter().map(ToString::to_string).collect();
                format!(" — présente sur la branche {}", noms.join(", "))
            };
            inconnus.push(format!("{} — {}{indication}", o.node, lieu(o)));
        }
        sections.push(("Inconnus (repère sans note dans cette branche)", inconnus));
        sections.push((
            "Non ancrés (repère d'une note sans ancrage)",
            rapport
                .unanchored
                .iter()
                .map(|o| format!("{} — {}", designation(etat, o.node), lieu(o)))
                .collect(),
        ));
        sections.push((
            "Mal formés (identifiant illisible après « @design: »)",
            mal_formes,
        ));
        let mut texte = format!(
            "Repères : {} dans {porteurs} fichiers (fichiers lus : {}, ignorés : {ignores})\n",
            occurrences.len() + sections[sections.len() - 1].1.len(),
            fichiers.len()
        );
        for (titre, lignes) in sections {
            texte.push_str(&format!("{titre} :\n"));
            if lignes.is_empty() {
                texte.push_str("  (aucun)\n");
            }
            for l in lignes {
                texte.push_str(&format!("  - {l}\n"));
            }
        }
        contexte.ecrire(&texte)
    }
}

pub(crate) struct CodeNotes;

impl Commande for CodeNotes {
    fn nom(&self) -> &'static str {
        "code notes"
    }
    fn usage(&self) -> &'static str {
        "FICHIER [--ligne N] [--extrait TEXTE] [--tsv]"
    }
    fn resume(&self) -> &'static str {
        "notes liées à ce fichier de code et marques qu'il contient ; avec une ligne ou un extrait, \
         celles qui les concernent d'abord ; n'écrit rien"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let ligne = arguments.option("--ligne")?;
        let extrait = arguments.option("--extrait")?;
        let en_tsv = arguments.drapeau("--tsv")?;
        let fichier = arguments.positionnel("FICHIER")?;
        arguments.fin()?;
        if ligne.is_some() && extrait.is_some() {
            return Err(Echec::Usage("--ligne et --extrait s'excluent".to_owned()));
        }
        let chemin = contexte.repertoire.join(&fichier);
        if !chemin.is_file() {
            return Err(Echec::Usage(format!("{fichier} n'est pas un fichier")));
        }
        let octets = std::fs::read(&chemin)?;
        let texte = String::from_utf8_lossy(&octets);
        let focus = match (ligne, extrait) {
            (Some(n), _) => {
                let total = texte.lines().count();
                let rang = n.parse::<usize>().ok().filter(|k| (1..=total).contains(k));
                Focus::Line(rang.ok_or_else(|| {
                    Echec::Usage(format!(
                        "ligne invalide : {n} (le fichier a {total} lignes)"
                    ))
                })?)
            }
            (None, Some(x)) => Focus::Excerpt(x),
            (None, None) => Focus::File,
        };
        let racine = contexte.repertoire.display().to_string();
        let chemin = chemin.display().to_string();
        let depot = contexte.ouvrir()?;
        let etat = depot.repository().snapshot();
        let notes = etat
            .code_notes(DEFAULT_SCOPE, &racine, &chemin, &texte, &focus)
            .map_err(Echec::Refus)?;
        let titre = |note: NodeId| {
            etat.graph(DEFAULT_SCOPE)
                .ok()
                .and_then(|g| g.node(note).ok().map(|n| champ_tsv(n.title())))
                .unwrap_or_else(|| "-".to_owned())
        };
        let debut = |a: memoire_domain::AnchorId| {
            etat.anchor(a)
                .map(|a| a.excerpt().lines().next().unwrap_or("").trim().to_owned())
                .unwrap_or_default()
        };
        let texte = if en_tsv {
            let mut sortie = String::new();
            for (genre, liste) in [("ancrage", &notes.anchors), ("proche", &notes.near)] {
                for a in liste {
                    let correspondance = match a.matched {
                        AnchorMatch::ContainsLine => "ligne",
                        AnchorMatch::ContainsExcerpt => "extrait",
                        AnchorMatch::FileOnly => "fichier",
                    };
                    sortie.push_str(&format!(
                        "{genre}\t{}\t{}\t{correspondance}\t{}\t{}\n",
                        a.node,
                        titre(a.node),
                        champ_tsv(&a.path),
                        champ_tsv(&debut(a.anchor))
                    ));
                }
            }
            for m in &notes.markers {
                let (note, nom, etat_de_la_marque) = match m.node {
                    Some(n) if m.known => (n.to_string(), titre(n), "connue"),
                    Some(n) => (n.to_string(), "-".to_owned(), "inconnue"),
                    None => ("-".to_owned(), "-".to_owned(), "mal-formee"),
                };
                let cible = if m.in_focus { "oui" } else { "non" };
                sortie.push_str(&format!(
                    "marque\t{}\t{note}\t{nom}\t{etat_de_la_marque}\t{cible}\n",
                    m.line
                ));
            }
            sortie
        } else {
            let mut sortie = format!("Notes liées à {} :\n", relative_path(&racine, &chemin));
            if notes.anchors.is_empty() {
                sortie.push_str("  (aucune)\n");
            }
            for a in &notes.anchors {
                let suite = match (&a.matched, &focus) {
                    (AnchorMatch::ContainsLine, Focus::Line(n)) => {
                        format!(" — contient la ligne {n}")
                    }
                    (AnchorMatch::ContainsExcerpt, _) => " — contient l'extrait".to_owned(),
                    _ => String::new(),
                };
                sortie.push_str(&format!(
                    "  - {} — « {} »{suite}\n",
                    designation(etat, a.node),
                    debut(a.anchor)
                ));
            }
            if !notes.near.is_empty() {
                sortie.push_str("Chemin proche :\n");
                for a in &notes.near {
                    sortie.push_str(&format!(
                        "  - {} — « {} » — ancré dans {}\n",
                        designation(etat, a.node),
                        debut(a.anchor),
                        a.path
                    ));
                }
            }
            sortie.push_str("Marques dans le fichier :\n");
            if notes.markers.is_empty() {
                sortie.push_str("  (aucune)\n");
            }
            for m in &notes.markers {
                let quoi = match m.node {
                    Some(n) if m.known => designation(etat, n),
                    Some(n) => format!("note absente de cette piste ({n})"),
                    None => "marque mal formée".to_owned(),
                };
                let suite = match (&focus, m.in_focus) {
                    (Focus::Line(n), true) => {
                        format!(" — la plus proche au-dessus de la ligne {n}")
                    }
                    (Focus::Excerpt(_), true) => " — dans l'extrait".to_owned(),
                    _ => String::new(),
                };
                sortie.push_str(&format!("  - ligne {} : {quoi}{suite}\n", m.line));
            }
            sortie
        };
        contexte.ecrire(&texte)
    }
}
