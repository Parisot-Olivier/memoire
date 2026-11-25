//! Recherche d'une note par son nom (R3), pour les interfaces : une ligne par note, identifiant,
//! rôle et désignation séparés par des tabulations.

use memoire_domain::{Kind, DEFAULT_SCOPE};

use super::notes::ROLES;
use super::{mot, Commande, Contexte};
use crate::arguments::Arguments;
use crate::Echec;

/// Forme de comparaison : minuscules, sans accents ni ligatures usuelles du français.
pub(crate) fn replier(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => sortie.push('a'),
            'é' | 'è' | 'ê' | 'ë' => sortie.push('e'),
            'î' | 'ï' | 'í' | 'ì' => sortie.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => sortie.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => sortie.push('u'),
            'ÿ' => sortie.push('y'),
            'ç' => sortie.push('c'),
            'ñ' => sortie.push('n'),
            'œ' => sortie.push_str("oe"),
            'æ' => sortie.push_str("ae"),
            autre => sortie.push(autre),
        }
    }
    sortie
}

pub(crate) struct Chercher;

impl Commande for Chercher {
    fn nom(&self) -> &'static str {
        "chercher"
    }
    fn usage(&self) -> &'static str {
        "TEXTE"
    }
    fn resume(&self) -> &'static str {
        "notes dont le nom contient TEXTE, sans tenir compte de la casse ni des accents"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let texte = arguments.positionnel("TEXTE")?;
        arguments.fin()?;
        let cherche = replier(&texte);
        let depot = contexte.ouvrir()?;
        let graphe = depot
            .repository()
            .snapshot()
            .graph(DEFAULT_SCOPE)
            .map_err(Echec::Refus)?;
        let mut trouves: Vec<(bool, String, String)> = graphe
            .nodes()
            .filter_map(|(id, n)| {
                let (nom, designation) = if n.kind() == Kind::Passage {
                    (n.text().to_owned(), format!("« {} »", n.text()))
                } else {
                    (n.title().to_owned(), n.title().to_owned())
                };
                let replie = replier(&nom);
                replie.contains(&cherche).then(|| {
                    let ligne = format!("{id}\t{}\t{designation}\n", mot(n.kind(), ROLES));
                    (!replie.starts_with(&cherche), replie, ligne)
                })
            })
            .collect();
        trouves.sort();
        let texte: String = trouves.into_iter().map(|(_, _, ligne)| ligne).collect();
        contexte.ecrire(&texte)
    }
}

#[cfg(test)]
mod tests {
    use super::replier;

    #[test]
    fn folding_ignores_case_accents_and_ligatures() {
        assert_eq!(replier("Réveil des TÂCHES"), "reveil des taches");
        assert_eq!(
            replier("àâäáã éèêë îïíì ôöóòõ ùûüú ÿ ç ñ"),
            "aaaaa eeee iiii ooooo uuuu y c n"
        );
        assert_eq!(replier("Œuvre et Æther"), "oeuvre et aether");
        assert_eq!(replier("GPIOICR 1"), "gpioicr 1");
    }
}
