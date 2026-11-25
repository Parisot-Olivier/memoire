//! Fiche de reprise (FICHE-REPRISE.md, sections 1, 4 et 5 ; D26 ; INV-RESUME-01, INV-RESUME-02 ;
//! CA-08). Aucune règle ici : la fiche, sa validité et la comparaison de l'état du code sont celles
//! du noyau ; l'état du code est lu par `crate::git`, en lecture seule.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use memoire_domain::{
    CodeChange, CodeState, Command, CommitId, Patch, ResumeEntry, ResumePoint, MAIN,
};

use super::notes::notes;
use super::restauration::commit;
use super::{designation, nom_de_branche, preparer_et_appliquer, Commande, Contexte, Depot};
use crate::arguments::Arguments;
use crate::git;
use crate::horodatage::date_a_la_minute;
use crate::Echec;

/// Dossier du code : `--code`, relatif au répertoire courant, sinon le répertoire courant.
fn dossier(contexte: &Contexte<'_>, saisie: Option<String>) -> Result<PathBuf, Echec> {
    let Some(saisie) = saisie else {
        return Ok(contexte.repertoire.clone());
    };
    let chemin = contexte.repertoire.join(&saisie);
    if !chemin.is_dir() {
        return Err(Echec::Usage(format!("{saisie} n'est pas un dossier")));
    }
    Ok(chemin)
}

/// La base et ses fichiers associés : s'ils sont dans le dépôt du code, ils ne comptent pas parmi
/// ses changements.
fn fichiers_de_la_base(contexte: &Contexte<'_>) -> Vec<PathBuf> {
    let Ok(base) = std::fs::canonicalize(&contexte.base) else {
        return Vec::new();
    };
    ["", "-wal", "-shm", "-journal"]
        .iter()
        .map(|suffixe| PathBuf::from(format!("{}{suffixe}", base.display())))
        .collect()
}

/// Huit premiers caractères d'un commit Git.
fn court(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}

fn pluriel(n: usize) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

/// Résumé d'un état du code : tête, fichiers changés, travail non enregistré.
fn resume_du_code(code: &CodeState) -> String {
    let mut texte = match (code.commit(), code.branch()) {
        (Some(c), Some(b)) => format!("{} sur {b}", court(c)),
        (Some(c), None) => format!("{} (tête détachée)", court(c)),
        (None, Some(b)) => format!("aucun commit sur {b}"),
        (None, None) => "aucun commit".to_owned(),
    };
    let n = code.changes().len();
    let travail = match code.patch() {
        Patch::Kept(patch) if patch.is_empty() => None,
        Patch::Kept(_) => Some("travail non enregistré conservé"),
        Patch::NotKept { .. } => {
            Some("travail non enregistré non conservé (trop grand ou hors UTF-8)")
        }
    };
    let mut parties = Vec::new();
    if n > 0 {
        let s = pluriel(n);
        let mut fichiers = format!("{n} fichier{s} changé{s}");
        let non_suivis = code
            .changes()
            .iter()
            .filter(|l| l.starts_with("??"))
            .count();
        if non_suivis > 0 {
            let s = pluriel(non_suivis);
            fichiers.push_str(&format!(
                " dont {non_suivis} non suivi{s} (listé{s} seulement)"
            ));
        }
        parties.push(fichiers);
    }
    parties.extend(travail.map(str::to_owned));
    if parties.is_empty() {
        parties.push("aucun changement non enregistré".to_owned());
    }
    texte.push_str(&format!(" ; {}", parties.join(" ; ")));
    texte
}

/// Vide affiché « — ».
fn champ(texte: &str) -> &str {
    if texte.is_empty() {
        "—"
    } else {
        texte
    }
}

pub(crate) struct RepriseNoter;

impl Commande for RepriseNoter {
    fn nom(&self) -> &'static str {
        "reprise noter"
    }
    fn usage(&self) -> &'static str {
        "--objectif TEXTE --action TEXTE [--question TEXTE] [--etat TEXTE] [--resultat TEXTE] \
         [--hypothese TEXTE] [--ouverts TEXTE] [--notes NOTES] [--code DOSSIER] [--sans-code] \
         [--motif M]"
    }
    fn resume(&self) -> &'static str {
        "enregistre un point de reprise : objectif, prochaine action, et l'état du code lu dans \
         Git sans rien y écrire ; le réseau ne change pas"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let objective = arguments.requise("--objectif")?;
        let next_action = arguments.requise("--action")?;
        let mut facultatif =
            |nom: &str| -> Result<String, Echec> { Ok(arguments.option(nom)?.unwrap_or_default()) };
        let question = facultatif("--question")?;
        let reached = facultatif("--etat")?;
        let result = facultatif("--resultat")?;
        let hypothesis = facultatif("--hypothese")?;
        let open = facultatif("--ouverts")?;
        let motif = facultatif("--motif")?;
        let liste = arguments.option("--notes")?;
        let code = arguments.option("--code")?;
        let sans_code = arguments.drapeau("--sans-code")?;
        arguments.fin()?;
        if sans_code && code.is_some() {
            return Err(Echec::Usage("--code et --sans-code s'excluent".to_owned()));
        }
        let dossier = dossier(contexte, code)?;
        let mut depot = contexte.ouvrir()?;
        let notes: BTreeSet<_> = match liste {
            Some(liste) => notes(&depot, "--notes", &liste)?.into_iter().collect(),
            None => BTreeSet::new(),
        };
        let (code, ligne) = if sans_code {
            (None, "Code : non noté (--sans-code)".to_owned())
        } else {
            match git::Acces::ouvrir(&dossier)? {
                git::Acces::Depot(git) => {
                    let etat = git.etat(&fichiers_de_la_base(contexte))?;
                    let ligne = format!("Code : {}", resume_du_code(&etat));
                    (Some(etat), ligne)
                }
                autre => (
                    None,
                    format!("Code : non disponible ({})", autre.motif(&dossier)),
                ),
            }
        };
        let secondes = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let point = ResumePoint {
            date: date_a_la_minute(secondes),
            objective,
            next_action,
            question,
            reached,
            result,
            hypothesis,
            open,
            notes,
            code,
        };
        let applique = preparer_et_appliquer(
            contexte,
            &mut depot,
            |_| Command::Resume(Box::new(point)),
            &motif,
            false,
        )?;
        contexte.ecrire(&format!(
            "Point de reprise enregistré : {}\n{ligne}\n",
            applique.outcome.head
        ))
    }
}

/// Ce qui a changé dans le code depuis la fiche (INV-RESUME-02).
fn code_depuis(
    point: &ResumePoint,
    dossier: &std::path::Path,
    contexte: &Contexte<'_>,
) -> Result<String, Echec> {
    if point.code.is_none() {
        return Ok("code non noté".to_owned());
    }
    let acces = git::Acces::ouvrir(dossier)?;
    let (git, actuel) = match &acces {
        git::Acces::Depot(git) => (Some(git), Some(git.etat(&fichiers_de_la_base(contexte))?)),
        _ => (None, None),
    };
    let (commit, work) = match point.code_change(actuel.as_ref()) {
        CodeChange::NotRecorded => return Ok("code non noté".to_owned()),
        CodeChange::Unavailable => {
            return Ok(format!("code non disponible ({})", acces.motif(dossier)))
        }
        CodeChange::Compared {
            commit: false,
            work: false,
        } => return Ok("code inchangé".to_owned()),
        CodeChange::Compared { commit, work } => (commit, work),
    };
    let mut parties = Vec::new();
    if commit {
        let avant = point.code.as_ref().and_then(CodeState::commit);
        let apres = actuel.as_ref().and_then(CodeState::commit);
        let nom = |c: Option<&str>| c.map_or("aucun".to_owned(), |c| court(c).to_owned());
        // Tête sans commit aujourd'hui : « merge-base » échoue, aucun nombre.
        let depuis = match (avant, git) {
            (Some(avant), Some(git)) => git.commits_depuis(avant)?,
            _ => None,
        };
        parties.push(match depuis {
            Some(k) => format!(
                "{k} commit{} depuis ({} → {})",
                pluriel(k),
                nom(avant),
                nom(apres)
            ),
            None if avant.is_some() && apres.is_some() => format!(
                "commit {} → {}, qui ne descend pas du commit noté",
                nom(avant),
                nom(apres)
            ),
            None => format!("commit {} → {}", nom(avant), nom(apres)),
        });
    }
    if work {
        parties.push("travail non enregistré différent".to_owned());
    }
    Ok(format!("code, {}", parties.join(", ")))
}

/// Texte de « reprise voir » (FICHE-REPRISE.md, section 1).
fn texte_de_la_fiche(
    depot: &Depot,
    id: CommitId,
    point: &ResumePoint,
    depuis_code: &str,
) -> Result<String, Echec> {
    let repo = depot.repository();
    let etat = repo.snapshot();
    let notes: Vec<String> = point.notes.iter().map(|n| designation(etat, *n)).collect();
    let notes = if notes.is_empty() {
        "—".to_owned()
    } else {
        notes.join(", ")
    };
    let code = match &point.code {
        None => "—".to_owned(),
        Some(code) => {
            let mut texte = resume_du_code(code);
            if matches!(code.patch(), Patch::Kept(p) if !p.is_empty()) {
                texte.push_str(&format!(
                    " (memoire reprise code {})",
                    court(&id.to_string())
                ));
            }
            texte
        }
    };
    let reseau = match repo.operations_since(id).map_err(Echec::Refus)? {
        Some(k) => format!("réseau, {k} opération{}", pluriel(k)),
        None => "réseau, fiche hors de l'historique de la branche active".to_owned(),
    };
    Ok(format!(
        "Point de reprise {id} — {}\n\
         Objectif : {}\n\
         Question : {}\n\
         Prochaine action : {}\n\
         État atteint : {}\n\
         Dernier résultat : {}\n\
         Hypothèse : {}\n\
         Ouverts : {}\n\
         Notes : {notes}\n\
         Code noté : {code}\n\
         Depuis : {reseau} ; {depuis_code}\n",
        champ(&point.date),
        point.objective,
        champ(&point.question),
        point.next_action,
        champ(&point.reached),
        champ(&point.result),
        champ(&point.hypothesis),
        champ(&point.open),
    ))
}

const AUCUNE_FICHE: &str = "Aucun point de reprise sur cette branche.\n";

pub(crate) struct RepriseVoir;

impl Commande for RepriseVoir {
    fn nom(&self) -> &'static str {
        "reprise voir"
    }
    fn usage(&self) -> &'static str {
        "[--depuis COMMIT] [--code DOSSIER]"
    }
    fn resume(&self) -> &'static str {
        "affiche le dernier point de reprise de la branche active (ou celui de ce commit) et ce \
         qui a changé depuis, réseau et code"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let depuis = arguments.option("--depuis")?;
        let code = arguments.option("--code")?;
        arguments.fin()?;
        let dossier = dossier(contexte, code)?;
        let depot = contexte.ouvrir()?;
        let repo = depot.repository();
        let (id, point) = match depuis {
            Some(saisie) => {
                let id = commit(&depot, &saisie)?;
                (id, repo.resume_point(id).map_err(Echec::Refus)?)
            }
            None => match repo
                .resume_points(repo.active_branch(), 1)
                .map_err(Echec::Refus)?
                .pop()
            {
                Some(ResumeEntry { commit, point }) => (commit, point),
                None => return contexte.ecrire(AUCUNE_FICHE),
            },
        };
        let depuis_code = code_depuis(&point, &dossier, contexte)?;
        let texte = texte_de_la_fiche(&depot, id, &point, &depuis_code)?;
        contexte.ecrire(&texte)
    }
}

pub(crate) struct RepriseListe;

impl Commande for RepriseListe {
    fn nom(&self) -> &'static str {
        "reprise liste"
    }
    fn usage(&self) -> &'static str {
        "[--toutes] [--nombre N]"
    }
    fn resume(&self) -> &'static str {
        "liste les points de reprise de la branche active, du plus récent au plus ancien ; \
         --toutes : de chaque branche active"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let toutes = arguments.drapeau("--toutes")?;
        let nombre = arguments.option("--nombre")?;
        arguments.fin()?;
        let nombre = match nombre {
            None => 20,
            Some(n) => n.parse::<usize>().ok().filter(|k| *k >= 1).ok_or_else(|| {
                Echec::Usage(format!("nombre invalide : {n} (entier à partir de 1)"))
            })?,
        };
        let depot = contexte.ouvrir()?;
        let repo = depot.repository();
        let ligne =
            |e: &ResumeEntry| format!("{} — {} — {}\n", e.commit, e.point.date, e.point.objective);
        if !toutes {
            let fiches = repo
                .resume_points(repo.active_branch(), nombre)
                .map_err(Echec::Refus)?;
            if fiches.is_empty() {
                return contexte.ecrire(AUCUNE_FICHE);
            }
            let texte: String = fiches.iter().map(ligne).collect();
            return contexte.ecrire(&texte);
        }
        // Chaque branche ne montre que les fiches notées depuis qu'elle a quitté main (D1 : toute
        // branche naît de main).
        let de_main: BTreeSet<CommitId> = repo
            .resume_points(MAIN, usize::MAX)
            .map_err(Echec::Refus)?
            .into_iter()
            .map(|e| e.commit)
            .collect();
        let mut texte = String::new();
        for branche in repo.branches() {
            if repo.is_abandoned(branche).map_err(Echec::Refus)? {
                continue;
            }
            let fiches: Vec<ResumeEntry> = repo
                .resume_points(branche, usize::MAX)
                .map_err(Echec::Refus)?
                .into_iter()
                .filter(|e| branche == MAIN || !de_main.contains(&e.commit))
                .take(nombre)
                .collect();
            texte.push_str(&format!("Branche {} :\n", nom_de_branche(branche)));
            if fiches.is_empty() {
                texte.push_str("  (aucun)\n");
            }
            for fiche in &fiches {
                texte.push_str(&format!("  {}", ligne(fiche)));
            }
        }
        contexte.ecrire(&texte)
    }
}

pub(crate) struct RepriseCode;

impl Commande for RepriseCode {
    fn nom(&self) -> &'static str {
        "reprise code"
    }
    fn usage(&self) -> &'static str {
        "COMMIT"
    }
    fn resume(&self) -> &'static str {
        "écrit le travail non enregistré noté avec ce point de reprise, sous forme de patch à \
         réappliquer par git apply"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let saisie = arguments.positionnel("COMMIT")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let id = commit(&depot, &saisie)?;
        let point = depot.repository().resume_point(id).map_err(Echec::Refus)?;
        let Some(code) = point.code else {
            return Err(Echec::Usage(
                "ce point de reprise n'a aucun état du code noté".to_owned(),
            ));
        };
        match code.patch() {
            Patch::Kept(patch) => contexte.ecrire(patch),
            Patch::NotKept { bytes, .. } => Err(Echec::Usage(format!(
                "travail non enregistré non conservé avec ce point de reprise ({bytes} octets, \
                 trop grand ou hors UTF-8)"
            ))),
        }
    }
}
