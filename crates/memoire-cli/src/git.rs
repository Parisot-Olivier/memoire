//! Lecture de l'état du code par l'exécutable `git`, en lecture seule (FICHE-REPRISE.md,
//! section 4 ; D26, D9). Rien n'est écrit, ni dans le code ni dans le dépôt Git : `status` ne
//! rafraîchit pas l'index (`--no-optional-locks`), aucun programme configuré dans le dépôt n'est
//! lancé (`core.fsmonitor=false`, `--no-ext-diff`, `--no-textconv`), aucun stash ni référence.
//! Aucune règle ici : la comparaison appartient au noyau (`ResumePoint::code_change`).

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use memoire_domain::CodeState;

use crate::Echec;

/// Dépôt Git reconnu : racine de l'arbre de travail.
pub(crate) struct Depot {
    dossier: PathBuf,
    racine: PathBuf,
}

/// Lance `git -C dossier …` ; `None` si l'exécutable est absent.
fn lancer(dossier: &Path, args: &[&str]) -> Result<Option<Output>, Echec> {
    let resultat = Command::new("git")
        .arg("-C")
        .arg(dossier)
        .args(["--no-optional-locks", "-c", "core.fsmonitor=false"])
        .args(args)
        // Un dépôt imposé par l'environnement (crochet Git, par exemple) ne doit pas remplacer
        // celui du dossier désigné.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output();
    match resultat {
        Ok(sortie) => Ok(Some(sortie)),
        Err(erreur) if erreur.kind() == ErrorKind::NotFound => Ok(None),
        Err(erreur) => Err(Echec::Stockage(format!("git : {erreur}"))),
    }
}

/// Sortie d'une commande qui doit réussir sur un dépôt reconnu ; sinon erreur de code 3.
fn exiger(dossier: &Path, args: &[&str]) -> Result<Vec<u8>, Echec> {
    match lancer(dossier, args)? {
        Some(sortie) if sortie.status.success() => Ok(sortie.stdout),
        Some(sortie) => Err(Echec::Stockage(format!(
            "git {} a échoué : {}",
            args.join(" "),
            String::from_utf8_lossy(&sortie.stderr).trim()
        ))),
        None => Err(Echec::Stockage("git : exécutable introuvable".to_owned())),
    }
}

/// Première ligne d'une sortie, si la commande a réussi (code 0) ; aucune sinon.
fn ligne_si_reussi(dossier: &Path, args: &[&str]) -> Result<Option<String>, Echec> {
    Ok(lancer(dossier, args)?
        .filter(|sortie| sortie.status.success())
        .map(|sortie| String::from_utf8_lossy(&sortie.stdout).trim().to_owned()))
}

/// Accès au dépôt Git d'un dossier.
pub(crate) enum Acces {
    /// Dépôt reconnu.
    Depot(Depot),
    /// Le dossier n'est dans aucun dépôt Git.
    HorsDepot,
    /// L'exécutable `git` est introuvable.
    Introuvable,
}

impl Acces {
    /// Dépôt Git contenant `dossier`, s'il y en a un et si `git` est disponible.
    pub(crate) fn ouvrir(dossier: &Path) -> Result<Self, Echec> {
        Ok(match lancer(dossier, &["rev-parse", "--show-toplevel"])? {
            None => Self::Introuvable,
            Some(sortie) if sortie.status.success() => Self::Depot(Depot {
                dossier: dossier.to_path_buf(),
                racine: PathBuf::from(String::from_utf8_lossy(&sortie.stdout).trim()),
            }),
            Some(_) => Self::HorsDepot,
        })
    }

    /// Motif de l'absence d'état du code, pour l'affichage.
    pub(crate) fn motif(&self, dossier: &Path) -> String {
        match self {
            Self::Depot(_) => String::new(),
            Self::HorsDepot => format!("pas de dépôt Git dans {}", dossier.display()),
            Self::Introuvable => "git introuvable".to_owned(),
        }
    }
}

impl Depot {
    /// État du code : commit, branche, lignes d'état et travail non enregistré. Les fichiers
    /// `ignores` (la base et ses fichiers associés, s'ils sont dans le dépôt) ne comptent pas.
    pub(crate) fn etat(&self, ignores: &[PathBuf]) -> Result<CodeState, Echec> {
        let d = &self.dossier;
        let commit = ligne_si_reussi(d, &["rev-parse", "--verify", "-q", "HEAD"])?;
        let branche = ligne_si_reussi(d, &["symbolic-ref", "--short", "-q", "HEAD"])?;
        let statut = exiger(
            d,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignore-submodules=none",
            ],
        )?;
        let ignores: Vec<String> = ignores
            .iter()
            .filter_map(|chemin| chemin.strip_prefix(&self.racine).ok())
            .map(|relatif| relatif.to_string_lossy().replace('\\', "/"))
            .collect();
        let changements = lignes_d_etat(&statut)
            .into_iter()
            .filter(|ligne| !ignores.iter().any(|i| ligne.get(3..) == Some(i.as_str())))
            .collect();
        // Sans commit, aucun travail ne se rapporte à une tête : les fichiers changés sont listés.
        let patch = match commit {
            Some(_) => exiger(
                d,
                &[
                    "diff",
                    "HEAD",
                    "--binary",
                    "--no-color",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--src-prefix=a/",
                    "--dst-prefix=b/",
                ],
            )?,
            None => Vec::new(),
        };
        Ok(CodeState::new(commit, branche, changements, &patch))
    }

    /// Nombre de commits depuis `ancien` jusqu'à la tête, si `ancien` en est un ancêtre ; aucun
    /// sinon (historique réécrit, autre branche, commit disparu).
    pub(crate) fn commits_depuis(&self, ancien: &str) -> Result<Option<usize>, Echec> {
        let ancetre = lancer(
            &self.dossier,
            &["merge-base", "--is-ancestor", ancien, "HEAD"],
        )?
        .is_some_and(|sortie| sortie.status.success());
        if !ancetre {
            return Ok(None);
        }
        let plage = format!("{ancien}..HEAD");
        let compte = exiger(&self.dossier, &["rev-list", "--count", &plage])?;
        String::from_utf8_lossy(&compte)
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| Echec::Stockage("git rev-list : nombre illisible".to_owned()))
    }
}

/// Lignes « XY chemin » d'un `status --porcelain=v1 -z` ; pour un renommage ou une copie,
/// l'ancien chemin, donné à part, s'ajoute : « R  nouveau <- ancien ».
fn lignes_d_etat(statut: &[u8]) -> Vec<String> {
    let mut lignes = Vec::new();
    let mut morceaux = statut
        .split(|octet| *octet == 0)
        .filter(|morceau| !morceau.is_empty())
        .map(String::from_utf8_lossy);
    while let Some(ligne) = morceaux.next() {
        // XY : un renommage ou une copie, de l'index ou de l'arbre de travail, est suivi de
        // l'ancien chemin.
        let renomme = ligne.chars().take(2).any(|c| c == 'R' || c == 'C');
        let ancien = if renomme { morceaux.next() } else { None };
        match ancien {
            Some(ancien) => lignes.push(format!("{ligne} <- {ancien}")),
            None => lignes.push(ligne.into_owned()),
        }
    }
    lignes
}

#[cfg(test)]
mod tests {
    use super::lignes_d_etat;

    // Sortie de « status --porcelain=v1 -z » : un renommage ou une copie, de l'index (X) ou de
    // l'arbre de travail (Y), est suivi de son ancien chemin ; les autres lignes non.
    #[test]
    fn status_lines_keep_the_old_path_of_a_rename() {
        let statut = b" M a.c\0R  b.c\0vieux b.c\0?? c.c\0 C d.c\0e.c\0A  f.c\0";
        assert_eq!(
            lignes_d_etat(statut),
            [
                " M a.c",
                "R  b.c <- vieux b.c",
                "?? c.c",
                " C d.c <- e.c",
                "A  f.c"
            ]
        );
        assert!(lignes_d_etat(b"").is_empty());
    }
}
