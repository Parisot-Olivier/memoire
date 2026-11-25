//! Lecture stricte des arguments (CLI.md, section 6) : options `--nom valeur`, drapeaux `--nom`,
//! arguments de position. Toute option inconnue, répétée ou sans valeur, et tout argument en trop,
//! sont refusés.

use crate::Echec;

/// Arguments restants d'une commande, consommés au fur et à mesure.
pub(crate) struct Arguments {
    restants: Vec<String>,
}

impl Arguments {
    pub(crate) fn new(restants: Vec<String>) -> Self {
        Self { restants }
    }

    fn position(&self, nom: &str) -> Result<Option<usize>, Echec> {
        let positions: Vec<usize> = self
            .restants
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == nom)
            .map(|(i, _)| i)
            .collect();
        match positions.as_slice() {
            [] => Ok(None),
            [une] => Ok(Some(*une)),
            _ => Err(Echec::Usage(format!("option {nom} répétée"))),
        }
    }

    /// Option facultative `--nom valeur`.
    pub(crate) fn option(&mut self, nom: &str) -> Result<Option<String>, Echec> {
        let Some(i) = self.position(nom)? else {
            return Ok(None);
        };
        let valeur = self
            .restants
            .get(i + 1)
            .filter(|v| !v.starts_with("--"))
            .cloned();
        let valeur = valeur.ok_or_else(|| Echec::Usage(format!("valeur manquante pour {nom}")))?;
        self.restants.drain(i..=i + 1);
        Ok(Some(valeur))
    }

    /// Option obligatoire `--nom valeur`.
    pub(crate) fn requise(&mut self, nom: &str) -> Result<String, Echec> {
        self.option(nom)?
            .ok_or_else(|| Echec::Usage(format!("option {nom} obligatoire")))
    }

    /// Drapeau `--nom`, sans valeur.
    pub(crate) fn drapeau(&mut self, nom: &str) -> Result<bool, Echec> {
        let present = self.position(nom)?;
        if let Some(i) = present {
            self.restants.remove(i);
        }
        Ok(present.is_some())
    }

    /// Argument de position suivant ; `description` nomme ce qui est attendu.
    pub(crate) fn positionnel(&mut self, description: &str) -> Result<String, Echec> {
        let i = self
            .restants
            .iter()
            .position(|a| !a.starts_with("--"))
            .ok_or_else(|| Echec::Usage(format!("{description} attendu")))?;
        Ok(self.restants.remove(i))
    }

    /// Vérifie qu'il ne reste rien : toute option ou valeur non lue est une erreur d'usage.
    pub(crate) fn fin(self) -> Result<(), Echec> {
        match self.restants.first() {
            None => Ok(()),
            Some(reste) => Err(Echec::Usage(format!("argument inattendu : {reste}"))),
        }
    }
}
