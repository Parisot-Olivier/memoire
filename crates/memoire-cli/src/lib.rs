//! Ligne de commande `memoire` (CLI.md). Aucune règle métier : chaque écriture passe par la couche
//! service, qui passe par le stockage.

mod arguments;
mod commandes;
mod git;
mod horodatage;

use std::io::Write;
use std::path::PathBuf;

use memoire_app::{IdSource, StoreError};
use memoire_domain::Error;

use arguments::Arguments;
use commandes::{Contexte, COMMANDES};

/// Ce que le programme lit de son environnement.
#[derive(Clone, Debug)]
pub struct Environnement {
    /// Valeur de `MEMOIRE_BASE`, si elle est définie.
    pub base: Option<PathBuf>,
    /// Répertoire courant, pour la base par défaut `.memoire/depot.db`.
    pub repertoire: PathBuf,
}

/// Échec d'une commande, traduit en code de retour (CLI.md, section 5).
#[derive(Debug)]
pub(crate) enum Echec {
    /// Commande, option ou argument incorrects : code 2.
    Usage(String),
    /// Refus du domaine : code 1.
    Refus(Error),
    /// Stockage ou entrée-sortie : code 3.
    Stockage(String),
    /// Opération critique non confirmée, rien d'écrit : code 4.
    Confirmation,
}

impl From<StoreError> for Echec {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Domain(e) => Echec::Refus(e),
            other => Echec::Stockage(other.to_string()),
        }
    }
}

impl From<std::io::Error> for Echec {
    fn from(error: std::io::Error) -> Self {
        Echec::Stockage(error.to_string())
    }
}

/// Exécute une ligne de commande et rend son code de retour. `arguments` ne contient pas le nom du
/// programme. Les sorties vont dans `sortie` ; les erreurs, préfixées de « erreur : », dans
/// `erreurs`.
pub fn executer(
    arguments: &[String],
    environnement: &Environnement,
    ids: Box<dyn IdSource>,
    sortie: &mut dyn Write,
    erreurs: &mut dyn Write,
) -> i32 {
    let resultat = lancer(arguments, environnement, ids, sortie);
    let (code, message) = match resultat {
        Ok(()) => return 0,
        // Une lecture impossible de l'historique n'est pas un refus métier : erreur de stockage.
        Err(Echec::Refus(Error::Historique(m))) => (
            3,
            format!("erreur : historique illisible ou non écrit : {m}"),
        ),
        Err(Echec::Refus(e)) => (1, format!("erreur : refusé : {e}")),
        Err(Echec::Usage(m)) => (2, format!("erreur : {m} (voir « memoire aide »)")),
        Err(Echec::Stockage(m)) => (3, format!("erreur : {m}")),
        Err(Echec::Confirmation) => (
            4,
            "Rien n'a été écrit : relancer avec --oui pour appliquer cette opération critique."
                .to_owned(),
        ),
    };
    let _ = writeln!(erreurs, "{message}");
    code
}

fn lancer(
    arguments: &[String],
    environnement: &Environnement,
    ids: Box<dyn IdSource>,
    sortie: &mut dyn Write,
) -> Result<(), Echec> {
    let mut mots = arguments.to_vec();
    let base_option = match mots.first().map(String::as_str) {
        Some("--base") => {
            if mots.len() < 2 {
                return Err(Echec::Usage("valeur manquante pour --base".to_owned()));
            }
            let base = mots.remove(1);
            mots.remove(0);
            Some(PathBuf::from(base))
        }
        _ => None,
    };
    let base = base_option
        .or_else(|| environnement.base.clone())
        .unwrap_or_else(|| environnement.repertoire.join(".memoire").join("depot.db"));
    let commande = COMMANDES
        .iter()
        .filter(|c| {
            let nom: Vec<&str> = c.nom().split(' ').collect();
            mots.len() >= nom.len() && mots.iter().zip(&nom).all(|(m, n)| m == n)
        })
        .max_by_key(|c| c.nom().split(' ').count())
        .ok_or_else(|| match mots.first() {
            Some(mot) => Echec::Usage(format!("commande inconnue : {mot}")),
            None => Echec::Usage("commande attendue".to_owned()),
        })?;
    let restants = mots.split_off(commande.nom().split(' ').count());
    let mut contexte = Contexte {
        base,
        repertoire: environnement.repertoire.clone(),
        ids: Some(ids),
        sortie,
    };
    commande.executer(Arguments::new(restants), &mut contexte)
}
