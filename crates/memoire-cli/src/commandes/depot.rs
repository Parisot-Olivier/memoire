//! Export et import du texte canonique (SERVICE.md, section 5).

use std::path::{Path, PathBuf};

use memoire_app::{IdSource, Migration, Service};

use super::{Commande, Contexte};
use crate::arguments::Arguments;
use crate::Echec;

pub(crate) struct Exporter;

impl Commande for Exporter {
    fn nom(&self) -> &'static str {
        "exporter"
    }
    fn usage(&self) -> &'static str {
        "FICHIER"
    }
    fn resume(&self) -> &'static str {
        "écrit le texte canonique du dépôt complet, de façon atomique"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let fichier = PathBuf::from(arguments.positionnel("FICHIER")?);
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        depot.export(&fichier)?;
        contexte.ecrire(&format!("Exporté : {}\n", fichier.display()))
    }
}

pub(crate) struct Importer;

impl Commande for Importer {
    fn nom(&self) -> &'static str {
        "importer"
    }
    fn usage(&self) -> &'static str {
        "FICHIER"
    }
    fn resume(&self) -> &'static str {
        "crée la base à partir d'un export, validé en entier avant toute création"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let fichier = PathBuf::from(arguments.positionnel("FICHIER")?);
        arguments.fin()?;
        let texte = std::fs::read_to_string(&fichier)?;
        if let Some(parent) = contexte.base.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let ids = contexte.ids.take().expect("source d'identités");
        Service::import(&texte, &contexte.base, ids)?;
        let message = format!("Importé dans : {}\n", contexte.base.display());
        contexte.ecrire(&message)
    }
}

pub(crate) struct Sauvegarder;

impl Commande for Sauvegarder {
    fn nom(&self) -> &'static str {
        "sauvegarder"
    }
    fn usage(&self) -> &'static str {
        "[FICHIER]"
    }
    fn resume(&self) -> &'static str {
        "copie cohérente de la base ; par défaut, horodatée dans sauvegardes/ à côté de la base"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let fichier = arguments.positionnel("FICHIER").ok();
        arguments.fin()?;
        let cible = match fichier {
            Some(f) => PathBuf::from(f),
            None => {
                let cible = sauvegarde_horodatee(&contexte.base, "depot");
                if let Some(dossier) = cible.parent() {
                    std::fs::create_dir_all(dossier)?;
                }
                cible
            }
        };
        let depot = contexte.ouvrir()?;
        depot.backup(&cible)?;
        contexte.ecrire(&format!("Sauvegarde écrite : {}\n", cible.display()))
    }
}

/// Chemin d'une sauvegarde horodatée dans sauvegardes/ à côté de la base, sans le créer.
pub(crate) fn sauvegarde_horodatee(base: &Path, prefixe: &str) -> PathBuf {
    let secondes = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let dossier = base
        .parent()
        .map_or_else(|| PathBuf::from("sauvegardes"), |p| p.join("sauvegardes"));
    dossier.join(format!(
        "{prefixe}-{}.db",
        crate::horodatage::horodatage(secondes)
    ))
}

pub(crate) struct Verifier;

/// « 1 nom » ou « n noms ».
fn compte(n: usize, nom: &str) -> String {
    if n == 1 {
        format!("1 {nom}")
    } else {
        format!("{n} {nom}s")
    }
}

impl Commande for Verifier {
    fn nom(&self) -> &'static str {
        "verifier"
    }
    fn usage(&self) -> &'static str {
        ""
    }
    fn resume(&self) -> &'static str {
        "relit tout l'historique et confirme l'état courant de chaque branche"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let verifie = depot.verify()?;
        contexte.ecrire(&format!(
            "Base vérifiée : {}, {}, état courant confirmé.\n",
            compte(verifie.commits, "commit"),
            compte(verifie.heads.len(), "branche")
        ))
    }
}

pub(crate) struct Migrer;

impl Commande for Migrer {
    fn nom(&self) -> &'static str {
        "migrer"
    }
    fn usage(&self) -> &'static str {
        ""
    }
    fn resume(&self) -> &'static str {
        "convertit une base de schéma 1 au schéma 2, après une sauvegarde dans sauvegardes/, et la vérifie en entier"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        arguments.fin()?;
        // La commande est la confirmation (ETAT-HISTORIQUE.md, section 7) ; la sauvegarde n'est
        // écrite que si la base est bien à migrer.
        let sauvegarde = sauvegarde_horodatee(&contexte.base, "avant-migration");
        match Service::<Box<dyn IdSource>>::migrate(&contexte.base, &sauvegarde)
            .map_err(|e| Echec::Stockage(e.to_string()))?
        {
            Migration::AlreadyCurrent => {
                contexte.ecrire("Base déjà au schéma 2 : rien à migrer.\n")
            }
            Migration::Migrated(verifie) => contexte.ecrire(&format!(
                "Sauvegarde écrite : {}\nBase migrée au schéma 2 : {}, {}, vérifiée.\n",
                sauvegarde.display(),
                compte(verifie.commits, "commit"),
                compte(verifie.heads.len(), "branche")
            )),
        }
    }
}
