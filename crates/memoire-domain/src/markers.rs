//! Indexeur de repères (INDEXEUR.md, sections 2 et 3 ; INV-CODE-01, INV-CODE-05, INV-CODE-06) :
//! détection des repères dans un texte et classement des ancrages d'après les occurrences
//! trouvées. Aucune lecture de fichier ici : la ligne de commande parcourt le code et transmet les
//! textes lus.

use std::collections::BTreeMap;

use crate::error::Error;
use crate::id::{AnchorId, NodeId, ScopeId};
use crate::model::Snapshot;

/// Marque d'un repère, suivie d'un identifiant canonique, n'importe où dans une ligne.
pub const MARKER_TAG: &str = "@design: ";

/// Longueur d'un identifiant canonique.
const ID_LEN: usize = 36;

/// Repère trouvé dans un texte : ligne (à partir de 1) et note ; aucune pour un identifiant mal
/// formé, à signaler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundMarker {
    /// Ligne, à partir de 1.
    pub line: usize,
    /// Note désignée, si l'identifiant est canonique.
    pub node: Option<NodeId>,
}

/// Repères d'un texte, dans l'ordre (INDEXEUR.md, section 2) : `@design: ` suivi d'un identifiant
/// canonique, qui ne se prolonge ni par une lettre, ni par un chiffre, ni par un tiret.
#[must_use]
pub fn find_markers(text: &str) -> Vec<FoundMarker> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(at) = rest.find(MARKER_TAG) {
            let after = &rest[at + MARKER_TAG.len()..];
            let prolonged = after
                .get(ID_LEN..)
                .and_then(|tail| tail.chars().next())
                .is_some_and(|c| c.is_alphanumeric() || c == '-');
            let node = after
                .get(..ID_LEN)
                .and_then(NodeId::parse)
                .filter(|_| !prolonged);
            found.push(FoundMarker {
                line: index + 1,
                node,
            });
            rest = after;
        }
    }
    found
}

/// Repère trouvé dans un fichier du dossier indexé.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Occurrence {
    /// Chemin relatif au dossier indexé, séparé par `/`.
    pub path: String,
    /// Ligne, à partir de 1.
    pub line: usize,
    /// Note désignée.
    pub node: NodeId,
}

/// État d'un ancrage actif d'après les occurrences du repère de sa note (INDEXEUR.md, section 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchorState {
    /// Une occurrence, dans le fichier de l'ancrage, dont l'extrait ancré est intact.
    Found(Occurrence),
    /// Une occurrence, dans le fichier de l'ancrage, dont l'extrait ancré est absent.
    Modified(Occurrence),
    /// Une occurrence, dans un autre fichier : le chemin de l'ancrage est périmé (INV-CODE-05).
    Moved {
        /// Occurrence trouvée.
        at: Occurrence,
        /// Vrai si l'extrait ancré figure dans ce fichier.
        excerpt_found: bool,
    },
    /// Plusieurs occurrences, toutes listées, aucune choisie (INV-CODE-01).
    Ambiguous(Vec<Occurrence>),
    /// Aucune occurrence.
    Lost,
}

/// Ligne du rapport pour un ancrage actif.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorReport {
    /// Ancrage.
    pub anchor: AnchorId,
    /// Note ancrée.
    pub node: NodeId,
    /// Chemin de l'ancrage, relatif au dossier indexé.
    pub path: String,
    /// État.
    pub state: AnchorState,
}

/// Rapport d'indexation (INV-CODE-06 : mêmes entrées, même rapport).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeReport {
    /// Un ancrage actif par ligne, par identifiant croissant.
    pub anchors: Vec<AnchorReport>,
    /// Repères dont la note n'est pas dans le contexte, par chemin puis ligne.
    pub unknown: Vec<Occurrence>,
    /// Repères d'une note sans ancrage actif, par chemin puis ligne.
    pub unanchored: Vec<Occurrence>,
}

/// Chemin relatif au dossier indexé : séparateur `/`, sans `./` en tête, ramené sous le dossier
/// s'il y est (INDEXEUR.md, section 3).
#[must_use]
pub fn relative_path(root: &str, path: &str) -> String {
    let path = path.replace('\\', "/");
    let root = root.trim_end_matches('/');
    let path = match path.strip_prefix(root).and_then(|p| p.strip_prefix('/')) {
        Some(inside) if !root.is_empty() => inside.to_owned(),
        _ => path,
    };
    let mut path = path.as_str();
    while let Some(rest) = path.strip_prefix("./") {
        path = rest;
    }
    path.to_owned()
}

impl Snapshot {
    /// Classement des ancrages actifs du contexte et des repères trouvés (INDEXEUR.md,
    /// section 3). `files` donne le texte des fichiers lus, par chemin relatif ; `root` sert à
    /// ramener les chemins d'ancrage sous le dossier indexé.
    ///
    /// # Errors
    /// [`Error::MissingScope`] si le contexte n'existe pas.
    pub fn code_report(
        &self,
        scope: ScopeId,
        root: &str,
        occurrences: &[Occurrence],
        files: &BTreeMap<String, String>,
    ) -> Result<CodeReport, Error> {
        let graph = self.graph(scope)?;
        let mut sorted = occurrences.to_vec();
        sorted.sort();
        let mut by_node: BTreeMap<NodeId, Vec<Occurrence>> = BTreeMap::new();
        for occurrence in &sorted {
            by_node
                .entry(occurrence.node)
                .or_default()
                .push(occurrence.clone());
        }
        let mut anchors = Vec::new();
        for (&anchor, data) in &self.anchors {
            if !data.active || graph.node(data.node).is_err() {
                continue;
            }
            let path = relative_path(root, &data.path);
            let found = by_node.get(&data.node).map_or(&[][..], Vec::as_slice);
            let contains = |file: &str| files.get(file).is_some_and(|t| t.contains(&data.excerpt));
            let state = match found {
                [] => AnchorState::Lost,
                [one] if one.path != path => AnchorState::Moved {
                    at: one.clone(),
                    excerpt_found: contains(&one.path),
                },
                [one] if contains(&path) => AnchorState::Found(one.clone()),
                [one] => AnchorState::Modified(one.clone()),
                several => AnchorState::Ambiguous(several.to_vec()),
            };
            anchors.push(AnchorReport {
                anchor,
                node: data.node,
                path,
                state,
            });
        }
        let anchored = |node: NodeId| self.anchors.values().any(|a| a.active && a.node == node);
        let (mut unknown, mut unanchored) = (Vec::new(), Vec::new());
        for occurrence in sorted {
            if graph.node(occurrence.node).is_err() {
                unknown.push(occurrence);
            } else if !anchored(occurrence.node) {
                unanchored.push(occurrence);
            }
        }
        Ok(CodeReport {
            anchors,
            unknown,
            unanchored,
        })
    }
}

/// Ce que l'on cherche dans un fichier de code (RETROUVER.md, section 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    /// Le fichier entier.
    File,
    /// Une ligne, à partir de 1.
    Line(usize),
    /// Un extrait collé.
    Excerpt(String),
}

/// Correspondance d'un ancrage avec la cible, dans l'ordre d'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnchorMatch {
    /// L'extrait ancré contient la ligne visée.
    ContainsLine,
    /// L'extrait ancré contient l'extrait collé, ou y est contenu.
    ContainsExcerpt,
    /// Seulement le même fichier.
    FileOnly,
}

/// Note d'un ancrage actif du fichier (ou d'un chemin proche).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredNote {
    /// Ancrage.
    pub anchor: AnchorId,
    /// Note ancrée.
    pub node: NodeId,
    /// Correspondance avec la cible.
    pub matched: AnchorMatch,
    /// Chemin de l'ancrage, relatif au dossier.
    pub path: String,
}

/// Marque trouvée dans le texte du fichier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkerHit {
    /// Ligne, à partir de 1.
    pub line: usize,
    /// Note désignée ; aucune pour une marque mal formée.
    pub node: Option<NodeId>,
    /// Vrai si la note est dans le contexte.
    pub known: bool,
    /// Vrai pour la marque la plus proche au-dessus de la ligne visée, ou pour une marque couverte
    /// par l'extrait visé.
    pub in_focus: bool,
}

/// Notes d'un code (RETROUVER.md, section 3 ; INV-CODE-07).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeNotes {
    /// Ancrages actifs du fichier : correspondances d'abord, puis par note et par ancrage.
    pub anchors: Vec<AnchoredNote>,
    /// Ancrages actifs d'un autre chemin au même nom de fichier, dans le même ordre.
    pub near: Vec<AnchoredNote>,
    /// Marques du texte, par ligne.
    pub markers: Vec<MarkerHit>,
}

/// Dernier composant d'un chemin séparé par `/`.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Lignes (à partir de 1) couvertes par la première occurrence de `excerpt` dans `text`.
fn excerpt_lines(text: &str, excerpt: &str) -> Option<(usize, usize)> {
    let start = text.find(excerpt)?;
    let first = text[..start].matches('\n').count() + 1;
    let last = first + excerpt.trim_end_matches('\n').matches('\n').count();
    Some((first, last))
}

impl Snapshot {
    /// Notes d'un fichier de code, de texte `text`, au chemin `path` relatif à `root`
    /// (RETROUVER.md, section 3). Lecture seule.
    ///
    /// # Errors
    /// [`Error::MissingScope`] si le contexte n'existe pas.
    pub fn code_notes(
        &self,
        scope: ScopeId,
        root: &str,
        path: &str,
        text: &str,
        focus: &Focus,
    ) -> Result<CodeNotes, Error> {
        let graph = self.graph(scope)?;
        let path = relative_path(root, path);
        let line_text = match focus {
            Focus::Line(n) => text.lines().nth(n.saturating_sub(1)).map(str::trim),
            _ => None,
        };
        let matched = |excerpt: &str| match focus {
            Focus::Line(_) if line_text.is_some_and(|l| !l.is_empty() && excerpt.contains(l)) => {
                AnchorMatch::ContainsLine
            }
            Focus::Excerpt(x)
                if !x.trim().is_empty()
                    && (excerpt.contains(x.trim()) || x.contains(excerpt.trim())) =>
            {
                AnchorMatch::ContainsExcerpt
            }
            _ => AnchorMatch::FileOnly,
        };
        let mut notes = CodeNotes::default();
        for (&anchor, data) in &self.anchors {
            if !data.active || graph.node(data.node).is_err() {
                continue;
            }
            let anchored = relative_path(root, &data.path);
            let note = AnchoredNote {
                anchor,
                node: data.node,
                matched: matched(&data.excerpt),
                path: anchored.clone(),
            };
            if anchored == path {
                notes.anchors.push(note);
            } else if file_name(&anchored) == file_name(&path) {
                notes.near.push(note);
            }
        }
        for list in [&mut notes.anchors, &mut notes.near] {
            list.sort_by_key(|a| (a.matched, a.node, a.anchor));
        }
        let found = find_markers(text);
        let nearest = match focus {
            Focus::Line(n) => found.iter().map(|m| m.line).filter(|l| l <= n).max(),
            _ => None,
        };
        let covered = match focus {
            Focus::Excerpt(x) => excerpt_lines(text, x),
            _ => None,
        };
        notes.markers = found
            .into_iter()
            .map(|m| MarkerHit {
                line: m.line,
                node: m.node,
                known: m.node.is_some_and(|n| graph.node(n).is_ok()),
                in_focus: Some(m.line) == nearest
                    || covered.is_some_and(|(a, b)| (a..=b).contains(&m.line)),
            })
            .collect();
        Ok(notes)
    }
}
