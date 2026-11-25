//! Différences entre états (VERSION2.md, section 4). Chaque ligne canonique d'un instantané en
//! version 2 a une clé, tirée de ses premiers jetons ; une différence retire et ajoute des lignes
//! entières, repérées par leur clé. L'ordre des clés est l'ordre canonique du texte (FORMAT.md,
//! section 6) : rassembler les lignes dans l'ordre des clés redonne le texte de l'instantané.

use std::collections::BTreeMap;

use crate::format::{decode_snapshot_v2, encode_snapshot_v2, DecodeError, SNAPSHOT_HEADER_V2};
use crate::id::{AnchorId, EdgeId, NodeId, ScopeId};
use crate::model::Snapshot;

/// Partie d'une note : création, révision de rang donné (à partir de 1), données propres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PartieNote {
    /// Ligne `contenu`.
    Contenu,
    /// Ligne `revision`, par rang.
    Revision(usize),
    /// Ligne `echange`, `citation` ou `reserve`.
    Corps,
}

/// Élément d'un contexte, dans l'ordre de son bloc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Element {
    /// Ligne `contexte`.
    Entete,
    /// Ligne `membre`, par note.
    Membre(NodeId),
    /// Ligne `relation`, par identifiant.
    Relation(EdgeId),
    /// Ligne `acquittement`, par couple (dépendant, cause).
    Acquittement(NodeId, NodeId),
}

/// Clé d'une ligne canonique. L'ordre dérivé suit celui du texte : notes, ancrages, contextes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Cle {
    /// Ligne d'une note.
    Note(NodeId, PartieNote),
    /// Ligne `ancrage`.
    Ancrage(AnchorId),
    /// Ligne d'un bloc de contexte.
    Contexte(ScopeId, Element),
}

/// Lignes d'un instantané, sans en-tête ni `fin`, par clé.
pub(crate) type Lignes = BTreeMap<Cle, String>;

/// Clé d'une ligne, tirée de ses premiers jetons ; aucune pour l'en-tête, `fin` ou une ligne
/// inconnue de la version 2. Le reste de la ligne n'est pas lu : la lecture de l'instantané le
/// vérifie.
pub(crate) fn cle(ligne: &str) -> Option<Cle> {
    let mut jetons = ligne.split(' ');
    let mot = jetons.next()?;
    let mut id = || jetons.next().and_then(NodeId::parse).map(NodeId::to_bytes);
    let note = |bytes: [u8; 16]| NodeId::from_bytes(bytes);
    let cle = match mot {
        "contenu" => Cle::Note(note(id()?), PartieNote::Contenu),
        "echange" | "citation" | "reserve" => Cle::Note(note(id()?), PartieNote::Corps),
        "ancrage" => Cle::Ancrage(AnchorId::from_bytes(id()?)),
        "contexte" => Cle::Contexte(ScopeId::from_bytes(id()?), Element::Entete),
        "membre" => {
            let scope = ScopeId::from_bytes(id()?);
            Cle::Contexte(scope, Element::Membre(note(id()?)))
        }
        "relation" => {
            let scope = ScopeId::from_bytes(id()?);
            Cle::Contexte(scope, Element::Relation(EdgeId::from_bytes(id()?)))
        }
        "acquittement" => {
            let scope = ScopeId::from_bytes(id()?);
            let dependent = note(id()?);
            Cle::Contexte(scope, Element::Acquittement(dependent, note(id()?)))
        }
        "revision" => {
            let node = note(id()?);
            let rang = jetons.next()?.parse().ok()?;
            Cle::Note(node, PartieNote::Revision(rang))
        }
        _ => return None,
    };
    Some(cle)
}

/// Lignes de l'instantané, encodé en version 2.
pub(crate) fn lignes(snapshot: &Snapshot) -> Lignes {
    encode_snapshot_v2(snapshot)
        .lines()
        .filter_map(|ligne| cle(ligne).map(|cle| (cle, ligne.to_owned())))
        .collect()
}

/// Texte de l'instantané en version 2 : en-tête, lignes dans l'ordre des clés, `fin`.
pub(crate) fn assembler(lignes: &Lignes) -> String {
    let mut out = String::new();
    out.push_str(SNAPSHOT_HEADER_V2);
    out.push('\n');
    for ligne in lignes.values() {
        out.push_str(ligne);
        out.push('\n');
    }
    out.push_str("fin\n");
    out
}

/// Lignes changées, chacune avec sa clé, dans l'ordre des clés.
type Changes<'a, K> = Vec<(&'a K, &'a str)>;

/// Différence de deux tables de lignes, quelle que soit la forme de leurs clés : lignes retirées
/// puis ajoutées, chacune avec sa clé, dans l'ordre des clés.
fn keyed_difference<'a, K: Ord>(
    avant: &'a BTreeMap<K, String>,
    apres: &'a BTreeMap<K, String>,
) -> (Changes<'a, K>, Changes<'a, K>) {
    let changees = |de: &'a BTreeMap<K, String>, vers: &'a BTreeMap<K, String>| {
        de.iter()
            .filter(|(cle, ligne)| vers.get(*cle) != Some(*ligne))
            .map(|(cle, ligne)| (cle, ligne.as_str()))
            .collect()
    };
    (changees(avant, apres), changees(apres, avant))
}

// ----- Forme publique, pour le stockage (ETAT-HISTORIQUE.md, section 3) -----

/// Forme textuelle d'une clé : son ordre lexicographique est l'ordre canonique du texte. Les
/// préfixes 1, 2 et 3 rangent notes, ancrages puis contextes ; le rang d'une révision est écrit
/// sur 20 chiffres, la longueur de `u64::MAX`, pour que l'ordre reste exact quel que soit le rang.
fn key_text(cle: &Cle) -> String {
    match cle {
        Cle::Note(n, PartieNote::Contenu) => format!("1/{n}/0"),
        Cle::Note(n, PartieNote::Revision(rang)) => format!("1/{n}/1/{rang:020}"),
        Cle::Note(n, PartieNote::Corps) => format!("1/{n}/2"),
        Cle::Ancrage(a) => format!("2/{a}"),
        Cle::Contexte(s, Element::Entete) => format!("3/{s}/0"),
        Cle::Contexte(s, Element::Membre(n)) => format!("3/{s}/1/{n}"),
        Cle::Contexte(s, Element::Relation(e)) => format!("3/{s}/2/{e}"),
        Cle::Contexte(s, Element::Acquittement(d, c)) => format!("3/{s}/3/{d}/{c}"),
    }
}

/// Clé textuelle d'une ligne canonique d'un instantané en version 2 ; aucune pour l'en-tête, la
/// fin ou une ligne inconnue.
#[must_use]
pub fn line_key(line: &str) -> Option<String> {
    cle(line).map(|cle| key_text(&cle))
}

/// Lignes canoniques d'un état (version 2), par clé textuelle ; parcourues dans l'ordre des clés,
/// elles redonnent le texte de l'instantané, sans en-tête ni fin.
#[must_use]
pub fn state_lines(snapshot: &Snapshot) -> BTreeMap<String, String> {
    lignes(snapshot)
        .into_iter()
        .map(|(cle, ligne)| (key_text(&cle), ligne))
        .collect()
}

/// État relu et validé depuis ses lignes canoniques, données dans l'ordre des clés.
///
/// # Errors
/// [`DecodeError`] comme pour [`decode_snapshot_v2`] ; la ligne 1 du texte relu est l'en-tête,
/// la première ligne donnée est donc la ligne 2.
pub fn state_from_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
) -> Result<Snapshot, DecodeError> {
    let mut text = String::new();
    text.push_str(SNAPSHOT_HEADER_V2);
    text.push('\n');
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("fin\n");
    decode_snapshot_v2(&text)
}

/// Différence entre deux états, sous forme de lignes à clés textuelles ([`state_lines`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineChanges<'a> {
    /// Couples (clé, ligne) retirés, dans l'ordre des clés.
    pub removed: Vec<(&'a str, &'a str)>,
    /// Couples (clé, ligne) ajoutés, dans l'ordre des clés.
    pub added: Vec<(&'a str, &'a str)>,
}

/// Différence entre deux tables de lignes à clés textuelles ([`state_lines`]) : une ligne
/// modifiée est retirée puis ajoutée ; une ligne inchangée n'apparaît pas.
#[must_use]
pub fn state_difference<'a>(
    before: &'a BTreeMap<String, String>,
    after: &'a BTreeMap<String, String>,
) -> LineChanges<'a> {
    let (removed, added) = keyed_difference(before, after);
    let pairs = |v: Changes<'a, String>| v.into_iter().map(|(k, l)| (k.as_str(), l)).collect();
    LineChanges {
        removed: pairs(removed),
        added: pairs(added),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::decode_snapshot;

    const EXEMPLE_V1: &str = include_str!("../../../fixtures/format/exemple-v1.txt");
    const EXEMPLE_V2: &str = include_str!("../../../fixtures/format/exemple-v2.txt");

    fn id(n: u8) -> String {
        format!("{n:02x}000000-0000-0000-0000-000000000000")
    }

    // Chaque sorte de ligne a sa clé ; l'en-tête, la fin et les lignes inconnues n'en ont pas.
    #[test]
    fn every_line_kind_has_its_key() {
        let n = |k| NodeId::parse(&id(k)).unwrap();
        let cases = [
            (
                format!("contenu {} note titre \"\" texte \"\"", id(1)),
                Cle::Note(n(1), PartieNote::Contenu),
            ),
            (
                format!("revision {} 12 mineure titre \"\" texte \"\"", id(1)),
                Cle::Note(n(1), PartieNote::Revision(12)),
            ),
            (
                format!("echange {} q", id(2)),
                Cle::Note(n(2), PartieNote::Corps),
            ),
            (
                format!("citation {} s", id(3)),
                Cle::Note(n(3), PartieNote::Corps),
            ),
            (
                format!("reserve {} p", id(4)),
                Cle::Note(n(4), PartieNote::Corps),
            ),
            (
                format!("ancrage {} note", id(5)),
                Cle::Ancrage(AnchorId::from_bytes(n(5).to_bytes())),
            ),
            (
                format!("contexte {}", id(6)),
                Cle::Contexte(ScopeId::from_bytes(n(6).to_bytes()), Element::Entete),
            ),
            (
                format!("membre {} {} verdict -", id(6), id(7)),
                Cle::Contexte(ScopeId::from_bytes(n(6).to_bytes()), Element::Membre(n(7))),
            ),
            (
                format!("relation {} {} {}", id(6), id(8), id(7)),
                Cle::Contexte(
                    ScopeId::from_bytes(n(6).to_bytes()),
                    Element::Relation(EdgeId::from_bytes(n(8).to_bytes())),
                ),
            ),
            (
                format!("acquittement {} {} {} sens 0", id(6), id(7), id(9)),
                Cle::Contexte(
                    ScopeId::from_bytes(n(6).to_bytes()),
                    Element::Acquittement(n(7), n(9)),
                ),
            ),
        ];
        for (ligne, attendue) in cases {
            assert_eq!(cle(&ligne), Some(attendue), "{ligne}");
        }
        for sans_cle in [
            String::new(),
            "fin".to_owned(),
            SNAPSHOT_HEADER_V2.to_owned(),
            format!("passage {} echange", id(1)),
            "contenu".to_owned(),
            "contenu 1".to_owned(),
            format!("revision {}", id(1)),
            format!("revision {} mineure", id(1)),
            format!("membre {}", id(6)),
            format!("relation {}", id(6)),
            format!("acquittement {} {}", id(6), id(7)),
        ] {
            assert_eq!(cle(&sans_cle), None, "{sans_cle}");
        }
    }

    // L'ordre des clés est celui du texte : la sorte de ligne prime sur les identifiants.
    #[test]
    fn keys_follow_the_canonical_text_order() {
        let n = |k| NodeId::parse(&id(k)).unwrap();
        let s = |k| ScopeId::from_bytes(n(k).to_bytes());
        let canonique = [
            Cle::Note(n(2), PartieNote::Contenu),
            Cle::Note(n(2), PartieNote::Revision(1)),
            Cle::Note(n(2), PartieNote::Revision(2)),
            Cle::Note(n(2), PartieNote::Corps),
            Cle::Note(n(3), PartieNote::Contenu),
            Cle::Ancrage(AnchorId::from_bytes(n(1).to_bytes())),
            Cle::Ancrage(AnchorId::from_bytes(n(4).to_bytes())),
            Cle::Contexte(s(1), Element::Entete),
            Cle::Contexte(s(1), Element::Membre(n(9))),
            Cle::Contexte(s(1), Element::Relation(EdgeId::from_bytes(n(1).to_bytes()))),
            Cle::Contexte(s(1), Element::Acquittement(n(1), n(2))),
            Cle::Contexte(s(1), Element::Acquittement(n(2), n(1))),
            Cle::Contexte(s(2), Element::Entete),
        ];
        let mut triees = canonique;
        triees.sort();
        assert_eq!(triees, canonique);
    }

    // Les lignes d'un état, rassemblées, redonnent son texte v2.
    #[test]
    fn assembling_the_lines_gives_back_the_text() {
        let etat = decode_snapshot(EXEMPLE_V1).unwrap();
        let lignes = lignes(&etat);
        assert_eq!(lignes.len(), EXEMPLE_V2.lines().count() - 2);
        assert_eq!(assembler(&lignes), EXEMPLE_V2);
        assert_eq!(assembler(&Lignes::new()), "memoire-instantane 2\nfin\n");
    }

    /// Lignes retirées puis ajoutées, sans leurs clés.
    fn difference<'a>(avant: &'a Lignes, apres: &'a Lignes) -> (Vec<&'a str>, Vec<&'a str>) {
        let (retirees, ajoutees) = keyed_difference(avant, apres);
        let lignes = |v: Changes<'a, Cle>| v.into_iter().map(|(_, l)| l).collect();
        (lignes(retirees), lignes(ajoutees))
    }

    // Une différence ne contient que les lignes changées, retirées et ajoutées dans l'ordre des clés.
    // Contrat modifié (incrément 5.2c) : la différence à clés internes est retirée au profit de la
    // différence générique ; ce test la vérifie avec les mêmes assertions.
    #[test]
    fn a_difference_holds_only_the_changed_lines() {
        let avant = lignes(&decode_snapshot(EXEMPLE_V1).unwrap());
        assert_eq!(difference(&avant, &avant), (vec![], vec![]));
        let mut apres = avant.clone();
        let (premiere, derniere) = (
            *avant.keys().next().unwrap(),
            *avant.keys().next_back().unwrap(),
        );
        let ancienne = avant[&derniere].clone();
        apres.insert(derniere, ancienne.replace("depend-de", "lien"));
        apres.remove(&premiere);
        let nouvelle = Cle::Note(NodeId::parse(&id(1)).unwrap(), PartieNote::Contenu);
        apres.insert(nouvelle, "contenu nouvelle".to_owned());
        let (retirees, ajoutees) = difference(&avant, &apres);
        assert_eq!(retirees, [avant[&premiere].as_str(), ancienne.as_str()]);
        assert_eq!(ajoutees, ["contenu nouvelle", apres[&derniere].as_str()]);
        assert_eq!(difference(&apres, &avant), (ajoutees, retirees));
    }
}
