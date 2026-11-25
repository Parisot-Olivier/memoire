//! Format textuel du dépôt complet : branches, identités employées, commits, journal des
//! opérations. Mêmes jetons, mêmes chaînes et même exigence de forme canonique unique que pour les
//! instantanés.
//!
//! - Version 1 (FORMAT.md, section 12) : chaque commit porte son instantané complet.
//! - Version 2 (VERSION2.md, section 6) : chaque commit porte sa différence avec son premier
//!   parent, sauf un état de référence complet à la racine puis tous les cent commits le long des
//!   premiers parents. La migration de la v1 vers la v2 est contrôlée par relecture.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::command::{Change, Command, Envelope, Expected, UsedId};
use crate::difference::{assembler, cle, state_difference, state_lines, Cle, Lignes};
use crate::format::{
    decision_from, decision_word, decode_snapshot_version, encode_snapshot_version,
    implementation_from, implementation_word, kind_from, kind_word, part_from, part_word,
    relation_from, relation_word, reserve_from, reserve_word, revision_from, revision_word,
    tokenize, verdict_from, verdict_word, DecodeError, DecodeReason, Fields, Line, Result, Version,
};
use crate::fusion::{Conflict, MemberField, Resolution, Side};
use crate::history::MemoryHistory;
use crate::id::{AnchorId, BranchId, CommitId, EdgeId, NodeId, OpId, ScopeId, MAIN, ROOT};
use crate::model::Snapshot;
use crate::repository::{Branch, Commit, Effects, Outcome, Redo, Repository, ReviewChange, Used};
use crate::resume::{CodeState, Patch, ResumePoint, PATCH_LIMIT};

/// Première ligne d'un dépôt complet en version 1.
pub const REPOSITORY_HEADER: &str = "memoire-depot 1";

/// Première ligne d'un dépôt complet en version 2 (VERSION2.md, section 6).
pub const REPOSITORY_HEADER_V2: &str = "memoire-depot 2";

/// Indentation des lignes d'un bloc de commit ou d'opération.
const INDENT: &str = "  ";

/// Indentation des lignes d'un état de référence ou d'une différence (version 2).
const BLOCK_INDENT: &str = "    ";

/// Un état de référence tous les cent commits le long des premiers parents (VERSION2.md, section 3).
const REFERENCE_INTERVAL: usize = 100;

impl Version {
    fn repository_header(self) -> &'static str {
        match self {
            Version::V1 => REPOSITORY_HEADER,
            Version::V2 => REPOSITORY_HEADER_V2,
        }
    }
}

fn optional_id(id: Option<impl ToString>) -> Option<String> {
    id.map(|id| id.to_string())
}

fn yes(value: bool) -> &'static str {
    if value {
        "oui"
    } else {
        "non"
    }
}

/// Mot d'une commande dans le format v2 (FORMAT.md, section 12.2 ; VERSION2.md, section 6), pour
/// la nommer dans un historique.
#[must_use]
pub fn command_name(command: &Command) -> &'static str {
    match command {
        Command::Undo => "annuler",
        Command::Redo => "retablir",
        Command::Checkout { .. } => "basculer",
        Command::Abandon { .. } => "abandonner",
        Command::Reactivate { .. } => "reactiver",
        Command::Merge { .. } => "fusionner",
        Command::Lot(_) => "lot",
        Command::Resume(_) => "reprise",
        Command::Change(change) => match change {
            Change::CreateNode { .. } => "creer-note",
            Change::Revise { .. } => "reviser",
            Change::Include { .. } => "inclure",
            Change::CloneScope { .. } => "copier-contexte",
            Change::Link { .. } => "lier",
            Change::SetVerdict { .. } => "verdict",
            Change::SetDecision { .. } => "decision",
            Change::SetImplementation { .. } => "implementation",
            Change::Archive { .. } => "retirer",
            Change::Reattach { .. } => "reattacher",
            Change::Replace { .. } => "remplacer",
            Change::Restrict { .. } => "restreindre",
            Change::Contradict { .. } => "contredire",
            Change::CreateExchange { .. } => "creer-echange",
            Change::SetQuestion { .. } => "completer-question",
            Change::CreatePassage { .. } | Change::CiteNote { .. } => "citer",
            Change::CreateReserve { .. } => "creer-reserve",
            Change::Treat { .. } => "traiter",
            Change::SetReserveStatus { .. } => "etat-reserve",
            Change::Acknowledge { .. } => "acquitter",
            Change::Detach { .. } => "detacher",
            Change::AnchorToCode { .. } => "ancrer",
            Change::Unanchor { .. } => "retirer-ancrage",
            Change::Restore { .. } => "restaurer",
            Change::RestoreNodes { .. } => "restaurer-notes",
        },
    }
}

/// Ligne `  commande …` d'une opération (FORMAT.md, section 12.2 ; VERSION2.md, section 6).
fn command_line(command: &Command, version: Version) -> Line {
    let line = Line::new("  commande");
    match command {
        Command::Undo => line.word("annuler"),
        Command::Redo => line.word("retablir"),
        Command::Checkout { branch } => line.word("basculer").word("branche").value(branch),
        Command::Abandon { branch } => line.word("abandonner").word("branche").value(branch),
        Command::Reactivate { branch } => line.word("reactiver").word("branche").value(branch),
        // Les résolutions suivent, une ligne `    resolution …` chacune (FUSION.md, section 9.6).
        Command::Merge {
            source,
            source_head,
            resolutions,
        } => line
            .word("fusionner")
            .word("source")
            .value(source)
            .word("tete")
            .value(source_head)
            .word("resolutions")
            .value(resolutions.len()),
        // Les éléments suivent, une ligne `    element …` chacun (LOT.md, section 3).
        Command::Lot(elements) => line.word("lot").value(elements.len()),
        // La version 1 ne représente pas les fiches : la ligne est écrite comme en version 2, et
        // le décodeur de la version 1 la refuse.
        Command::Resume(point) => resume_line(line, point),
        Command::Change(change) => change_line(line, change, version),
    }
}

/// Mots et champs d'une fiche de reprise (FICHE-REPRISE.md, section 3), état du code compris, sur
/// une seule ligne.
fn resume_line(line: Line, point: &ResumePoint) -> Line {
    let line = line
        .word("reprise")
        .word("date")
        .string(&point.date)
        .word("objectif")
        .string(&point.objective)
        .word("action")
        .string(&point.next_action)
        .word("question")
        .string(&point.question)
        .word("etat")
        .string(&point.reached)
        .word("resultat")
        .string(&point.result)
        .word("hypothese")
        .string(&point.hypothesis)
        .word("ouverts")
        .string(&point.open);
    let line = ids_line(line, "notes", &point.notes).word("code");
    let Some(code) = &point.code else {
        return line.word("-");
    };
    let mut line = line
        .word("commit")
        .optional_string(code.commit.as_deref())
        .word("branche")
        .optional_string(code.branch.as_deref())
        .word("changements")
        .value(code.changes.len());
    for change in &code.changes {
        line = line.string(change);
    }
    let line = line.word("patch");
    match &code.patch {
        Patch::Kept(text) => line.word("texte").string(text),
        Patch::NotKept { bytes, digest } => line
            .word("non-conserve")
            .word("octets")
            .value(bytes)
            .word("empreinte")
            .word(&format!("{digest:016x}")),
    }
}

/// Fiche de reprise d'une ligne `  commande reprise …`, après le mot `reprise`. Forme canonique :
/// changements strictement croissants, patch conservé d'au plus [`PATCH_LIMIT`] octets.
fn read_resume(f: &mut Fields<'_>) -> Result<ResumePoint> {
    let date = labeled_string(f, "date")?;
    let objective = labeled_string(f, "objectif")?;
    let next_action = labeled_string(f, "action")?;
    let question = labeled_string(f, "question")?;
    let reached = labeled_string(f, "etat")?;
    let result = labeled_string(f, "resultat")?;
    let hypothesis = labeled_string(f, "hypothese")?;
    let open = labeled_string(f, "ouverts")?;
    let notes = read_ids(f, "notes")?;
    f.label("code")?;
    let code = match f.bare("code")? {
        "-" => None,
        "commit" => {
            let commit = f.optional_string("commit")?;
            f.label("branche")?;
            let branch = f.optional_string("branche")?;
            f.label("changements")?;
            let count = f.integer()?;
            let mut changes: Vec<String> = Vec::new();
            for _ in 0..count {
                let change = f.string("changement")?;
                if changes.last().is_some_and(|last| *last >= change) {
                    return Err(DecodeReason::Order);
                }
                changes.push(change);
            }
            f.label("patch")?;
            let patch = match f.bare("patch")? {
                "texte" => match f.string("patch")? {
                    text if text.len() > PATCH_LIMIT => return Err(DecodeReason::Value("patch")),
                    text => Patch::Kept(text),
                },
                "non-conserve" => {
                    f.label("octets")?;
                    let bytes = f.integer()?;
                    f.label("empreinte")?;
                    let digest = f.digest()?;
                    Patch::NotKept { bytes, digest }
                }
                _ => return Err(DecodeReason::Value("patch")),
            };
            Some(CodeState {
                commit,
                branch,
                changes,
                patch,
            })
        }
        _ => return Err(DecodeReason::Value("code")),
    };
    Ok(ResumePoint {
        date,
        objective,
        next_action,
        question,
        reached,
        result,
        hypothesis,
        open,
        notes,
        code,
    })
}

/// Mots et champs d'une modification, après le début de ligne donné (`  commande` ou
/// `    element`).
fn change_line(line: Line, change: &Change, version: Version) -> Line {
    match change {
        Change::CreateNode {
            scope,
            id,
            kind,
            title,
            text,
        } => line
            .word("creer-note")
            .word("portee")
            .value(scope)
            .word("note")
            .value(id)
            .word("role")
            .word(kind_word(*kind))
            .word("titre")
            .string(title)
            .word("texte")
            .string(text),
        Change::Revise { node, text, kind } => line
            .word("reviser")
            .word("note")
            .value(node)
            .word("texte")
            .string(text)
            .word("nature")
            .word(revision_word(*kind)),
        Change::Include { scope, node } => line
            .word("inclure")
            .word("portee")
            .value(scope)
            .word("note")
            .value(node),
        Change::CloneScope { source, id } => line
            .word("copier-contexte")
            .word("source")
            .value(source)
            .word("contexte")
            .value(id),
        Change::Link {
            scope,
            id,
            from,
            to,
            relation,
        } => line
            .word("lier")
            .word("portee")
            .value(scope)
            .word("relation")
            .value(id)
            .word("origine")
            .value(from)
            .word("destination")
            .value(to)
            .word("type")
            .word(relation_word(*relation)),
        Change::SetVerdict {
            scope,
            node,
            verdict,
        } => line
            .word("verdict")
            .word("portee")
            .value(scope)
            .word("note")
            .value(node)
            .word("valeur")
            .word(verdict_word(*verdict)),
        Change::SetDecision {
            scope,
            node,
            decision,
        } => line
            .word("decision")
            .word("portee")
            .value(scope)
            .word("note")
            .value(node)
            .word("valeur")
            .word(decision_word(*decision)),
        Change::SetImplementation {
            scope,
            node,
            implementation,
        } => line
            .word("implementation")
            .word("portee")
            .value(scope)
            .word("note")
            .value(node)
            .word("valeur")
            .word(implementation_word(*implementation)),
        Change::Archive { scope, node } => line
            .word("retirer")
            .word("portee")
            .value(scope)
            .word("note")
            .value(node),
        Change::Reattach {
            scope,
            edge,
            to,
            id,
        } => line
            .word("reattacher")
            .word("portee")
            .value(scope)
            .word("relation")
            .value(edge)
            .word("destination")
            .value(to)
            .word("nouvelle")
            .value(id),
        Change::Replace {
            scope,
            id,
            replacement,
            replaced,
        } => line
            .word("remplacer")
            .word("portee")
            .value(scope)
            .word("relation")
            .value(id)
            .word("remplacante")
            .value(replacement)
            .word("remplacee")
            .value(replaced),
        Change::Restrict {
            scope,
            id,
            from,
            to,
            condition,
        } => line
            .word("restreindre")
            .word("portee")
            .value(scope)
            .word("relation")
            .value(id)
            .word("origine")
            .value(from)
            .word("destination")
            .value(to)
            .word("condition")
            .string(condition),
        Change::Contradict {
            scope,
            id,
            from,
            to,
            justification,
        } => line
            .word("contredire")
            .word("portee")
            .value(scope)
            .word("relation")
            .value(id)
            .word("origine")
            .value(from)
            .word("destination")
            .value(to)
            .word("justification")
            .string(justification),
        Change::CreateExchange {
            scope,
            id,
            title,
            question,
            answer,
            previous,
        } => line
            .word("creer-echange")
            .word("portee")
            .value(scope)
            .word("note")
            .value(id)
            .word("titre")
            .string(title)
            .word("question")
            .optional_string(question.as_deref())
            .word("reponse")
            .string(answer)
            .word("precedent")
            .optional(optional_id(*previous).as_deref()),
        Change::SetQuestion { exchange, question } => line
            .word("completer-question")
            .word("echange")
            .value(exchange)
            .word("question")
            .string(question),
        // La version 1 ne représente pas la citation d'une note : la ligne est écrite comme en
        // version 2, et le décodeur de la version 1 la refuse.
        Change::CiteNote {
            scope,
            id,
            note,
            version: cited,
            start,
            end,
        } => line
            .word("citer")
            .word("portee")
            .value(scope)
            .word("note")
            .value(id)
            .word("source")
            .value(note)
            .word("partie")
            .word("note")
            .word("version")
            .value(cited)
            .word("debut")
            .value(start)
            .word("fin")
            .value(end),
        Change::CreatePassage {
            scope,
            id,
            exchange,
            part,
            start,
            end,
        } if version == Version::V2 => line
            .word("citer")
            .word("portee")
            .value(scope)
            .word("note")
            .value(id)
            .word("source")
            .value(exchange)
            .word("partie")
            .word(part_word(*part))
            .word("version")
            .word("-")
            .word("debut")
            .value(start)
            .word("fin")
            .value(end),
        Change::CreatePassage {
            scope,
            id,
            exchange,
            part,
            start,
            end,
        } => line
            .word("creer-passage")
            .word("portee")
            .value(scope)
            .word("note")
            .value(id)
            .word("echange")
            .value(exchange)
            .word("partie")
            .word(part_word(*part))
            .word("debut")
            .value(start)
            .word("fin")
            .value(end),
        Change::CreateReserve {
            scope,
            id,
            passage,
            title,
            target,
        } => line
            .word("creer-reserve")
            .word("portee")
            .value(scope)
            .word("note")
            .value(id)
            .word("passage")
            .value(passage)
            .word("titre")
            .string(title)
            .word("cible")
            .optional_string(target.as_deref()),
        Change::Treat {
            scope,
            id,
            note,
            reserve,
        } => line
            .word("traiter")
            .word("portee")
            .value(scope)
            .word("relation")
            .value(id)
            .word("note")
            .value(note)
            .word("reserve")
            .value(reserve),
        Change::SetReserveStatus {
            scope,
            reserve,
            status,
        } => line
            .word("etat-reserve")
            .word("portee")
            .value(scope)
            .word("reserve")
            .value(reserve)
            .word("valeur")
            .word(reserve_word(*status)),
        Change::Acknowledge {
            scope,
            dependent,
            cause,
        } => line
            .word("acquitter")
            .word("portee")
            .value(scope)
            .word("dependant")
            .value(dependent)
            .word("cause")
            .value(cause),
        Change::Detach { scope, edge } => line
            .word("detacher")
            .word("portee")
            .value(scope)
            .word("relation")
            .value(edge),
        Change::AnchorToCode {
            id,
            node,
            path,
            excerpt,
            git,
        } => line
            .word("ancrer")
            .word("ancrage")
            .value(id)
            .word("note")
            .value(node)
            .word("chemin")
            .string(path)
            .word("extrait")
            .string(excerpt)
            .word("git")
            .optional_string(git.as_deref()),
        Change::Unanchor { anchor } => line.word("retirer-ancrage").word("ancrage").value(anchor),
        Change::Restore { target } => line.word("restaurer").word("cible").value(target),
        Change::RestoreNodes { target, nodes } => {
            let mut line = line
                .word("restaurer-notes")
                .word("cible")
                .value(target)
                .word("notes")
                .value(nodes.len());
            for node in nodes {
                line = line.value(node);
            }
            line
        }
    }
}

fn ids_line(line: Line, label: &str, ids: &BTreeSet<NodeId>) -> Line {
    let mut line = line.word(label).value(ids.len());
    for id in ids {
        line = line.value(id);
    }
    line
}

fn branch_fragment(id: BranchId, branch: &Branch, version: Version) -> String {
    let mut out = String::new();
    let line = Line::new("branche")
        .value(id)
        .word("tete")
        .value(branch.head)
        .word("origine")
        .optional(optional_id(branch.origin).as_deref())
        .word("base")
        .value(branch.base);
    match version {
        // La version 1 ne représente pas l'abandon, qu'aucune base de schéma 1 ne contient.
        Version::V1 => line,
        Version::V2 => line.word("etat").word(if branch.abandoned {
            "abandonnee"
        } else {
            "active"
        }),
    }
    .end(&mut out);
    for (rank, entry) in branch.redo.iter().enumerate() {
        let line = Line::new("retablir").value(id).value(rank);
        match entry {
            Redo::Commit(commit) => line.word("commit").value(commit),
            Redo::Enter { branch, head } => {
                line.word("entrer").value(branch).word("tete").value(head)
            }
        }
        .end(&mut out);
    }
    out
}

/// Rang de la catégorie, mot et forme textuelle d'une identité employée.
fn identity_parts(id: UsedId) -> (usize, &'static str, String) {
    match id {
        UsedId::Node(id) => (0, "note", id.to_string()),
        UsedId::Edge(id) => (1, "relation", id.to_string()),
        UsedId::Scope(id) => (2, "contexte", id.to_string()),
        UsedId::Anchor(id) => (3, "ancrage", id.to_string()),
    }
}

/// Clé d'ordre d'une identité employée : son ordre lexicographique est l'ordre canonique du
/// texte (catégorie, puis identifiant).
#[must_use]
pub fn identity_key(id: UsedId) -> String {
    let (rank, _, id) = identity_parts(id);
    format!("{rank}:{id}")
}

/// Ligne `identite …` d'une identité employée (FORMAT.md, section 12.1), fin de ligne comprise ;
/// identique dans les deux versions du format.
#[must_use]
pub fn encode_identity(id: UsedId) -> String {
    let (_, category, id) = identity_parts(id);
    let mut line = String::new();
    Line::new("identite")
        .word(category)
        .word(&id)
        .end(&mut line);
    line
}

fn identity_fragments(used: &Used) -> BTreeMap<String, String> {
    let all = used
        .nodes
        .iter()
        .map(|id| UsedId::Node(*id))
        .chain(used.edges.iter().map(|id| UsedId::Edge(*id)))
        .chain(used.scopes.iter().map(|id| UsedId::Scope(*id)))
        .chain(used.anchors.iter().map(|id| UsedId::Anchor(*id)));
    all.map(|id| (identity_key(id), encode_identity(id)))
        .collect()
}

fn commit_fragment(id: CommitId, commit: &Commit) -> String {
    let mut out = String::new();
    Line::new("commit")
        .value(id)
        .word("parent")
        .optional(optional_id(commit.parent).as_deref())
        .word("motif")
        .string(&commit.reason)
        .end(&mut out);
    for line in encode_snapshot_version(&commit.state, Version::V1).lines() {
        out.push_str(INDENT);
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn operation_fragment(
    op: OpId,
    envelope: &Envelope,
    outcome: &Outcome,
    version: Version,
) -> String {
    let mut out = String::new();
    Line::new("operation")
        .value(op)
        .word("attendu")
        .value(envelope.expected.branch)
        .value(envelope.expected.head)
        .word("motif")
        .string(&envelope.reason)
        .end(&mut out);
    command_line(&envelope.command, version).end(&mut out);
    if let Command::Lot(elements) = &envelope.command {
        for change in elements {
            change_line(Line::new("    element"), change, version).end(&mut out);
        }
    }
    if let Command::Merge { resolutions, .. } = &envelope.command {
        for resolution in resolutions {
            resolution_line(resolution).end(&mut out);
        }
    }
    Line::new("  resultat")
        .word("branche")
        .value(outcome.branch)
        .word("tete")
        .value(outcome.head)
        .word("publie")
        .word(yes(outcome.published))
        .word("bifurque")
        .word(yes(outcome.forked))
        .end(&mut out);
    for ((scope, node), change) in &outcome.effects.review {
        let line = Line::new("  effet").value(scope).value(node);
        let line = ids_line(line, "avant", &change.before);
        ids_line(line, "apres", &change.after).end(&mut out);
    }
    out
}

/// Mots d'un champ de membre dans une résolution.
const MEMBER_FIELDS: [(&str, MemberField); 5] = [
    ("verdict", MemberField::Verdict),
    ("decision", MemberField::Decision),
    ("implementation", MemberField::Implementation),
    ("reserve", MemberField::Reserve),
    ("visibilite", MemberField::Visibility),
];

fn side_word(side: Side) -> &'static str {
    match side {
        Side::Target => "cible",
        Side::Source => "source",
    }
}

/// Ligne `    resolution …` d'une fusion (VERSION2.md, section 6 ; FUSION.md, section 9.5).
fn resolution_line(resolution: &Resolution) -> Line {
    let line = Line::new("    resolution");
    match resolution {
        Resolution::Choose(conflict, side) => match conflict {
            Conflict::Field { scope, node, field } => {
                let word = MEMBER_FIELDS
                    .iter()
                    .find(|(_, f)| f == field)
                    .map_or("?", |(w, _)| w);
                line.word("champ").value(scope).value(node).word(word)
            }
            Conflict::Text { node } => line.word("texte").value(node),
            Conflict::Ack {
                scope,
                dependent,
                cause,
            } => line
                .word("acquittement")
                .value(scope)
                .value(dependent)
                .value(cause),
        }
        .word(side_word(*side)),
        Resolution::Deactivate { scope, edge } => line.word("desactiver").value(scope).value(edge),
    }
}

/// Résolution d'une ligne `    resolution …`, indentation retirée.
fn read_resolution(mut fields: Fields<'_>) -> Result<Resolution> {
    let f = &mut fields;
    f.label("resolution")?;
    let side = |f: &mut Fields<'_>| match f.bare("côté")? {
        "cible" => Ok(Side::Target),
        "source" => Ok(Side::Source),
        _ => Err(DecodeReason::Value("côté")),
    };
    let resolution = match f.bare("resolution")? {
        "champ" => {
            let scope = id(f, ScopeId::from_bytes)?;
            let node = f.node()?;
            let word = f.bare("champ")?;
            let field = MEMBER_FIELDS
                .iter()
                .find(|(w, _)| *w == word)
                .map(|(_, field)| *field)
                .ok_or(DecodeReason::Value("champ"))?;
            Resolution::Choose(Conflict::Field { scope, node, field }, side(f)?)
        }
        "texte" => Resolution::Choose(Conflict::Text { node: f.node()? }, side(f)?),
        "acquittement" => {
            let scope = id(f, ScopeId::from_bytes)?;
            let dependent = f.node()?;
            let cause = f.node()?;
            Resolution::Choose(
                Conflict::Ack {
                    scope,
                    dependent,
                    cause,
                },
                side(f)?,
            )
        }
        "desactiver" => Resolution::Deactivate {
            scope: id(f, ScopeId::from_bytes)?,
            edge: id(f, EdgeId::from_bytes)?,
        },
        _ => return Err(DecodeReason::Value("resolution")),
    };
    fields.done()?;
    Ok(resolution)
}

/// Catégorie d'un fragment, dans l'ordre canonique du texte (FORMAT.md, section 12.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    /// Ligne `active`.
    Active,
    /// Ligne `branche` suivie de ses lignes `retablir`.
    Branch,
    /// Ligne `identite`.
    Identity,
    /// Ligne `commit` suivie de son instantané indenté.
    Commit,
    /// Bloc `operation`.
    Operation,
}

impl Category {
    /// Toutes les catégories, dans l'ordre canonique.
    pub const ALL: [Category; 5] = [
        Category::Active,
        Category::Branch,
        Category::Identity,
        Category::Commit,
        Category::Operation,
    ];

    /// Nom stable de la catégorie, employé par le stockage.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Category::Active => "active",
            Category::Branch => "branche",
            Category::Identity => "identite",
            Category::Commit => "commit",
            Category::Operation => "operation",
        }
    }

    /// Catégorie d'un nom stable.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.word() == word)
    }
}

/// Modification d'un fragment : nouveau texte, ou retrait si `text` est absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentChange {
    /// Catégorie du fragment.
    pub category: Category,
    /// Clé du fragment dans sa catégorie ; son ordre est l'ordre canonique.
    pub key: String,
    /// Nouveau texte, ou `None` si le fragment disparaît.
    pub text: Option<String>,
}

/// Texte canonique du dépôt découpé en fragments indépendants (FORMAT.md, section 12) : un
/// stockage peut n'écrire que les fragments modifiés, puis rassembler le texte complet et le lire
/// avec [`decode_repository`]. Les clés d'une catégorie se trient dans l'ordre canonique.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fragments {
    /// Fragments par catégorie, puis par clé.
    pub parts: BTreeMap<Category, BTreeMap<String, String>>,
}

impl Fragments {
    /// Fragments complets d'un dépôt.
    #[must_use]
    pub fn of(repo: &Repository) -> Self {
        let mut fragments = Fragments::default();
        fragments.update(repo);
        fragments
    }

    /// Met les fragments à jour pour le dépôt donné et rend les modifications. Les commits et les
    /// entrées du journal, immuables une fois créés, ne sont encodés qu'à leur apparition.
    pub fn update(&mut self, repo: &Repository) -> Vec<FragmentChange> {
        let mut fresh: BTreeMap<Category, BTreeMap<String, String>> = BTreeMap::new();
        let mut active = String::new();
        Line::new("active")
            .value(repo.history.active)
            .end(&mut active);
        fresh.insert(Category::Active, BTreeMap::from([(String::new(), active)]));
        fresh.insert(
            Category::Branch,
            repo.history
                .branches
                .iter()
                .map(|(id, b)| (id.to_string(), branch_fragment(*id, b, Version::V1)))
                .collect(),
        );
        fresh.insert(Category::Identity, identity_fragments(&repo.history.used));
        let known = |category| self.parts.get(&category);
        let commits = repo
            .history
            .commits
            .iter()
            .map(|(id, c)| {
                let key = id.to_string();
                let text = known(Category::Commit)
                    .and_then(|m| m.get(&key).cloned())
                    .unwrap_or_else(|| commit_fragment(*id, c));
                (key, text)
            })
            .collect();
        fresh.insert(Category::Commit, commits);
        let operations = repo
            .history
            .journal
            .iter()
            .map(|(op, (envelope, outcome))| {
                let key = op.to_string();
                let text = known(Category::Operation)
                    .and_then(|m| m.get(&key).cloned())
                    .unwrap_or_else(|| operation_fragment(*op, envelope, outcome, Version::V1));
                (key, text)
            })
            .collect();
        fresh.insert(Category::Operation, operations);
        let mut changes = Vec::new();
        for category in Category::ALL {
            let empty = BTreeMap::new();
            let old = self.parts.get(&category).unwrap_or(&empty);
            let new = &fresh[&category];
            for (key, text) in new {
                if old.get(key) != Some(text) {
                    changes.push(FragmentChange {
                        category,
                        key: key.clone(),
                        text: Some(text.clone()),
                    });
                }
            }
            for key in old.keys().filter(|k| !new.contains_key(*k)) {
                changes.push(FragmentChange {
                    category,
                    key: key.clone(),
                    text: None,
                });
            }
        }
        self.parts = fresh;
        changes
    }

    /// Texte complet, dans l'ordre canonique : en-tête, fragments par catégorie et par clé, `fin`.
    #[must_use]
    pub fn assemble(&self) -> String {
        let mut out = String::new();
        out.push_str(REPOSITORY_HEADER);
        out.push('\n');
        for category in Category::ALL {
            for text in self
                .parts
                .get(&category)
                .into_iter()
                .flat_map(BTreeMap::values)
            {
                out.push_str(text);
            }
        }
        out.push_str("fin\n");
        out
    }
}

/// Texte canonique du dépôt complet (FORMAT.md, section 12).
#[must_use]
pub fn encode_repository(repo: &Repository) -> String {
    Fragments::of(repo).assemble()
}

/// Profondeur de chaque commit le long des premiers parents : 0 pour un commit sans parent.
/// Aucune profondeur si un parent est absent ou si les parents forment un cycle. L'ordre des
/// identifiants n'intervient pas : un parent peut avoir un identifiant plus grand que son enfant.
pub(crate) fn profondeurs(
    parents: &BTreeMap<CommitId, Option<CommitId>>,
) -> Option<BTreeMap<CommitId, usize>> {
    let mut depths = BTreeMap::new();
    for &start in parents.keys() {
        // Remonte jusqu'à un commit de profondeur connue, ou jusqu'à un commit sans parent.
        let mut chain = Vec::new();
        let mut next = Some(start);
        let mut above = None;
        while let Some(commit) = next {
            if let Some(&depth) = depths.get(&commit) {
                above = Some(depth);
                break;
            }
            if chain.len() == parents.len() {
                return None;
            }
            next = *parents.get(&commit)?;
            chain.push(commit);
        }
        let first = above.map_or(0, |d| d + 1);
        for (depth, commit) in (first..).zip(chain.into_iter().rev()) {
            depths.insert(commit, depth);
        }
    }
    Some(depths)
}

/// Lignes d'un instantané en version 2 lues dans son texte canonique, sans le réencoder.
fn lines_of_text(text: &str) -> Lignes {
    text.lines()
        .filter_map(|line| cle(line).map(|key| (key, line.to_owned())))
        .collect()
}

/// Contenu d'un commit en version 2 : état de référence complet, ou différence avec le premier
/// parent (lignes canoniques de l'instantané, sans indentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitBlock<'a> {
    /// Toutes les lignes de l'état, dans l'ordre canonique, en-tête et fin exclus.
    Reference(&'a [&'a str]),
    /// Lignes retirées puis ajoutées, chacune dans l'ordre des clés.
    Difference {
        /// Lignes retirées.
        removed: &'a [&'a str],
        /// Lignes ajoutées.
        added: &'a [&'a str],
    },
}

/// Vrai si un commit de cette profondeur, le long des premiers parents, porte un état de
/// référence (VERSION2.md, section 3).
#[must_use]
pub fn holds_reference(depth: usize) -> bool {
    depth.is_multiple_of(REFERENCE_INTERVAL)
}

/// Bloc `commit …` d'un dépôt en version 2 (VERSION2.md, section 6), fin de ligne comprise :
/// en-tête, puis `  reference` et l'instantané complet indenté, ou `  difference` et ses lignes
/// `- ` puis `+ `.
#[must_use]
pub fn encode_commit_v2(
    id: CommitId,
    parent: Option<CommitId>,
    reason: &str,
    block: CommitBlock<'_>,
) -> String {
    let parents: Vec<CommitId> = parent.into_iter().collect();
    encode_commit_parents_v2(id, &parents, reason, block)
}

/// Bloc `commit …` en version 2 d'un commit à zéro, un ou deux parents (fusion : la cible, puis la
/// source ; FUSION.md, section 9.6). La différence est toujours prise avec le premier parent.
#[must_use]
pub fn encode_commit_parents_v2(
    id: CommitId,
    parents: &[CommitId],
    reason: &str,
    block: CommitBlock<'_>,
) -> String {
    let mut out = String::new();
    let mut line = Line::new("commit")
        .value(id)
        .word("parents")
        .value(parents.len());
    for parent in parents {
        line = line.value(parent);
    }
    line.word("motif").string(reason).end(&mut out);
    let kind = match block {
        CommitBlock::Reference(_) => "reference",
        CommitBlock::Difference { .. } => "difference",
    };
    out.push_str(INDENT);
    out.push_str(kind);
    out.push('\n');
    let mut push = |prefix: &str, line: &str| {
        out.push_str(BLOCK_INDENT);
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    };
    match block {
        CommitBlock::Reference(lines) => {
            push("", Version::V2.snapshot_header());
            for line in lines {
                push("", line);
            }
            push("", "fin");
        }
        CommitBlock::Difference { removed, added } => {
            for line in removed {
                push("- ", line);
            }
            for line in added {
                push("+ ", line);
            }
        }
    }
    out
}

/// Contenu stockable d'un commit en version 2, à clés textuelles ([`crate::state_lines`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitContent {
    /// État de référence complet : toutes ses lignes, par clé.
    Reference(BTreeMap<String, String>),
    /// Différence avec le premier parent : couples (clé, ligne) retirés puis ajoutés, chacun
    /// dans l'ordre des clés.
    Difference {
        /// Lignes retirées.
        removed: Vec<(String, String)>,
        /// Lignes ajoutées.
        added: Vec<(String, String)>,
    },
}

/// Commit d'un dépôt en version 2, tel qu'un stockage l'enregistre (ETAT-HISTORIQUE.md,
/// section 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRecord {
    /// Identité du commit.
    pub id: CommitId,
    /// Premier parent ; aucun pour la racine.
    pub parent: Option<CommitId>,
    /// Second parent d'un commit de fusion.
    pub merged: Option<CommitId>,
    /// Motif enregistré.
    pub reason: String,
    /// Profondeur le long des premiers parents ; 0 pour la racine.
    pub depth: usize,
    /// État de référence ou différence avec le parent.
    pub content: CommitContent,
}

/// Contenu stockable de chaque commit d'un dépôt en mémoire, par identifiant croissant : état de
/// référence aux profondeurs multiples de 100 (et pour tout commit sans parent), différence avec
/// le premier parent ailleurs. Les lignes de chaque commit ne sont calculées qu'une fois.
pub fn commit_records(repo: &Repository) -> impl Iterator<Item = CommitRecord> + '_ {
    let commits = &repo.history.commits;
    let parents = commits.iter().map(|(id, c)| (*id, c.parent)).collect();
    // Un dépôt valide n'a ni parent absent ni cycle. À défaut, chaque commit porterait un état de
    // référence, que la relecture refuse : l'écart ne passerait pas inaperçu.
    let depths = profondeurs(&parents).unwrap_or_default();
    let mut cache: BTreeMap<CommitId, BTreeMap<String, String>> = BTreeMap::new();
    commits.iter().map(move |(id, commit)| {
        let depth = depths.get(id).copied().unwrap_or(0);
        let lines = cache
            .entry(*id)
            .or_insert_with(|| state_lines(&commit.state))
            .clone();
        let parent = commit
            .parent
            .filter(|_| !holds_reference(depth))
            .and_then(|p| commits.get(&p).map(|c| (p, c)));
        let content = match parent {
            None => CommitContent::Reference(lines),
            Some((parent, parent_commit)) => {
                let before = cache
                    .entry(parent)
                    .or_insert_with(|| state_lines(&parent_commit.state));
                let changes = state_difference(before, &lines);
                let owned = |v: Vec<(&str, &str)>| {
                    v.into_iter()
                        .map(|(k, l)| (k.to_owned(), l.to_owned()))
                        .collect()
                };
                CommitContent::Difference {
                    removed: owned(changes.removed),
                    added: owned(changes.added),
                }
            }
        };
        CommitRecord {
            id: *id,
            parent: commit.parent,
            merged: commit.merged,
            reason: commit.reason.clone(),
            depth,
            content,
        }
    })
}

impl CommitRecord {
    /// Parents, le premier d'abord.
    #[must_use]
    pub fn parents(&self) -> Vec<CommitId> {
        self.parent.into_iter().chain(self.merged).collect()
    }
}

/// Lignes seules d'une liste de couples (clé, ligne).
fn values(pairs: &[(String, String)]) -> Vec<&str> {
    pairs.iter().map(|(_, line)| line.as_str()).collect()
}

/// Bloc `commit …` d'un enregistrement ([`commit_records`]).
fn encode_record(record: &CommitRecord) -> String {
    match &record.content {
        CommitContent::Reference(lines) => {
            let lines: Vec<&str> = lines.values().map(String::as_str).collect();
            encode_commit_parents_v2(
                record.id,
                &record.parents(),
                &record.reason,
                CommitBlock::Reference(&lines),
            )
        }
        CommitContent::Difference { removed, added } => encode_commit_parents_v2(
            record.id,
            &record.parents(),
            &record.reason,
            CommitBlock::Difference {
                removed: &values(removed),
                added: &values(added),
            },
        ),
    }
}

/// Ligne `active …` d'un dépôt, fin de ligne comprise ; identique dans les deux versions.
#[must_use]
pub fn encode_active(branch: BranchId) -> String {
    let mut out = String::new();
    Line::new("active").value(branch).end(&mut out);
    out
}

/// Ligne `branche …` d'un dépôt en version 2, suivie de ses lignes `retablir`, fin de ligne
/// comprise.
#[must_use]
pub fn encode_branch_v2(id: BranchId, branch: &Branch) -> String {
    branch_fragment(id, branch, Version::V2)
}

/// Bloc `operation …` d'un dépôt en version 2 : commande, résultat et effets.
#[must_use]
pub fn encode_operation_v2(envelope: &Envelope, outcome: &Outcome) -> String {
    operation_fragment(envelope.op, envelope, outcome, Version::V2)
}

/// Texte canonique du dépôt complet en version 2 (VERSION2.md, section 6) : en-tête, `active`,
/// branches, identités, commits, opérations, chacun dans l'ordre canonique, puis `fin`. Il est
/// assemblé des mêmes morceaux qu'un stockage peut écrire un à un ([`encode_active`],
/// [`encode_branch_v2`], [`encode_identity`], [`encode_commit_v2`], [`encode_operation_v2`]).
#[must_use]
pub fn encode_repository_v2(repo: &Repository) -> String {
    let mut out = String::new();
    out.push_str(REPOSITORY_HEADER_V2);
    out.push('\n');
    out.push_str(&encode_active(repo.history.active));
    for (id, branch) in &repo.history.branches {
        out.push_str(&encode_branch_v2(*id, branch));
    }
    for text in identity_fragments(&repo.history.used).values() {
        out.push_str(text);
    }
    for record in commit_records(repo) {
        out.push_str(&encode_record(&record));
    }
    for (envelope, outcome) in repo.history.journal.values() {
        out.push_str(&encode_operation_v2(envelope, outcome));
    }
    out.push_str("fin\n");
    out
}

/// Migre un dépôt de la version 1 vers la version 2 (VERSION2.md, section 7) : lecture de la v1,
/// encodage en v2, puis contrôle par relecture de la v2 et comparaison avec le dépôt d'origine.
///
/// # Errors
/// [`DecodeError`] si le texte v1 est refusé, ou si le contrôle de la migration échoue.
pub fn migrate_v1_to_v2(text: &str) -> std::result::Result<String, DecodeError> {
    let repo = decode_repository(text)?;
    let migrated = encode_repository_v2(&repo);
    verifier_migration(&repo, &migrated)?;
    Ok(migrated)
}

/// Contrôle d'une migration : la v2 relue doit être égale, champ pour champ, au dépôt d'origine,
/// états de chaque commit compris.
fn verifier_migration(repo: &Repository, migrated: &str) -> std::result::Result<(), DecodeError> {
    if decode_repository_v2(migrated)? == *repo {
        Ok(())
    } else {
        Err(DecodeError {
            line: 0,
            reason: DecodeReason::Inconsistent("migration"),
        })
    }
}

// ----- Décodage -----

fn id<T>(f: &mut Fields<'_>, make: fn([u8; 16]) -> T) -> Result<T> {
    f.uuid().map(make)
}

fn labeled<T>(f: &mut Fields<'_>, label: &'static str, make: fn([u8; 16]) -> T) -> Result<T> {
    f.label(label)?;
    id(f, make)
}

fn optional_uuid<T>(
    f: &mut Fields<'_>,
    what: &'static str,
    make: fn([u8; 16]) -> T,
) -> Result<Option<T>> {
    match f.bare(what)? {
        "-" => Ok(None),
        word => NodeId::parse(word)
            .map(|n| Some(make(n.to_bytes())))
            .ok_or(DecodeReason::Value("identifiant")),
    }
}

fn boolean(f: &mut Fields<'_>, label: &'static str) -> Result<bool> {
    f.label(label)?;
    match f.bare(label)? {
        "oui" => Ok(true),
        "non" => Ok(false),
        _ => Err(DecodeReason::Value(label)),
    }
}

fn labeled_string(f: &mut Fields<'_>, label: &'static str) -> Result<String> {
    f.label(label)?;
    f.string(label)
}

fn labeled_word<T>(
    f: &mut Fields<'_>,
    label: &'static str,
    from: fn(&str) -> Option<T>,
) -> Result<T> {
    f.label(label)?;
    f.word(label, from)
}

fn read_ids(f: &mut Fields<'_>, label: &'static str) -> Result<BTreeSet<NodeId>> {
    f.label(label)?;
    let count = f.integer()?;
    let mut ids = BTreeSet::new();
    for _ in 0..count {
        let next = f.node()?;
        if ids.last().is_some_and(|last| *last >= next) {
            return Err(DecodeReason::Order);
        }
        ids.insert(next);
    }
    Ok(ids)
}

/// Commande d'une ligne `  commande …` (FORMAT.md, section 12.2 ; VERSION2.md, section 6).
fn read_command(f: &mut Fields<'_>, version: Version) -> Result<Command> {
    let scope = |f: &mut Fields<'_>| labeled(f, "portee", ScopeId::from_bytes);
    let node = |f: &mut Fields<'_>, label| labeled(f, label, NodeId::from_bytes);
    let edge = |f: &mut Fields<'_>, label| labeled(f, label, EdgeId::from_bytes);
    let change = match f.bare("commande")? {
        "annuler" => return Ok(Command::Undo),
        "retablir" => return Ok(Command::Redo),
        "basculer" => {
            return Ok(Command::Checkout {
                branch: labeled(f, "branche", BranchId::from_bytes)?,
            })
        }
        "creer-note" => Change::CreateNode {
            scope: scope(f)?,
            id: node(f, "note")?,
            kind: labeled_word(f, "role", kind_from)?,
            title: labeled_string(f, "titre")?,
            text: labeled_string(f, "texte")?,
        },
        "reviser" => Change::Revise {
            node: node(f, "note")?,
            text: labeled_string(f, "texte")?,
            kind: labeled_word(f, "nature", revision_from)?,
        },
        "inclure" => Change::Include {
            scope: scope(f)?,
            node: node(f, "note")?,
        },
        "copier-contexte" => Change::CloneScope {
            source: labeled(f, "source", ScopeId::from_bytes)?,
            id: labeled(f, "contexte", ScopeId::from_bytes)?,
        },
        "lier" => Change::Link {
            scope: scope(f)?,
            id: edge(f, "relation")?,
            from: node(f, "origine")?,
            to: node(f, "destination")?,
            relation: labeled_word(f, "type", relation_from)?,
        },
        "verdict" => Change::SetVerdict {
            scope: scope(f)?,
            node: node(f, "note")?,
            verdict: labeled_word(f, "valeur", verdict_from)?,
        },
        "decision" => Change::SetDecision {
            scope: scope(f)?,
            node: node(f, "note")?,
            decision: labeled_word(f, "valeur", decision_from)?,
        },
        "implementation" => Change::SetImplementation {
            scope: scope(f)?,
            node: node(f, "note")?,
            implementation: labeled_word(f, "valeur", implementation_from)?,
        },
        "retirer" => Change::Archive {
            scope: scope(f)?,
            node: node(f, "note")?,
        },
        "reattacher" => Change::Reattach {
            scope: scope(f)?,
            edge: edge(f, "relation")?,
            to: node(f, "destination")?,
            id: edge(f, "nouvelle")?,
        },
        "remplacer" => Change::Replace {
            scope: scope(f)?,
            id: edge(f, "relation")?,
            replacement: node(f, "remplacante")?,
            replaced: node(f, "remplacee")?,
        },
        "restreindre" => Change::Restrict {
            scope: scope(f)?,
            id: edge(f, "relation")?,
            from: node(f, "origine")?,
            to: node(f, "destination")?,
            condition: labeled_string(f, "condition")?,
        },
        "contredire" => Change::Contradict {
            scope: scope(f)?,
            id: edge(f, "relation")?,
            from: node(f, "origine")?,
            to: node(f, "destination")?,
            justification: labeled_string(f, "justification")?,
        },
        "creer-echange" => Change::CreateExchange {
            scope: scope(f)?,
            id: node(f, "note")?,
            title: labeled_string(f, "titre")?,
            question: {
                f.label("question")?;
                f.optional_string("question")?
            },
            answer: labeled_string(f, "reponse")?,
            previous: {
                f.label("precedent")?;
                optional_uuid(f, "precedent", NodeId::from_bytes)?
            },
        },
        "completer-question" => Change::SetQuestion {
            exchange: node(f, "echange")?,
            question: labeled_string(f, "question")?,
        },
        "creer-passage" if version == Version::V1 => Change::CreatePassage {
            scope: scope(f)?,
            id: node(f, "note")?,
            exchange: node(f, "echange")?,
            part: labeled_word(f, "partie", part_from)?,
            start: {
                f.label("debut")?;
                f.integer()?
            },
            end: {
                f.label("fin")?;
                f.integer()?
            },
        },
        "citer" if version == Version::V2 => {
            let scope = scope(f)?;
            let id = node(f, "note")?;
            let source = node(f, "source")?;
            f.label("partie")?;
            let part = f.bare("partie")?;
            f.label("version")?;
            let cited = f.bare("version")?;
            f.label("debut")?;
            let start = f.integer()?;
            f.label("fin")?;
            let end = f.integer()?;
            match part {
                // Citation d'une note (D18) : la version est un rang de révision.
                "note" => Change::CiteNote {
                    scope,
                    id,
                    note: source,
                    version: cited.parse().map_err(|_| DecodeReason::Value("version"))?,
                    start,
                    end,
                },
                word => {
                    let part = part_from(word).ok_or(DecodeReason::Value("partie"))?;
                    if cited != "-" {
                        return Err(DecodeReason::Value("version"));
                    }
                    Change::CreatePassage {
                        scope,
                        id,
                        exchange: source,
                        part,
                        start,
                        end,
                    }
                }
            }
        }
        // Un lot en tête de ligne `commande` est lu par `read_operation` ; ici, c'est un lot
        // dans un lot.
        "lot" if version == Version::V2 => return Err(DecodeReason::Value("element")),
        // Une fusion en tête de ligne `commande` est lue par `read_operation` ; ici, c'est un
        // élément de lot.
        "fusionner" if version == Version::V2 => return Err(DecodeReason::Value("element")),
        "reprise" if version == Version::V2 => {
            return Ok(Command::Resume(Box::new(read_resume(f)?)))
        }
        "abandonner" if version == Version::V2 => {
            return Ok(Command::Abandon {
                branch: labeled(f, "branche", BranchId::from_bytes)?,
            })
        }
        "reactiver" if version == Version::V2 => {
            return Ok(Command::Reactivate {
                branch: labeled(f, "branche", BranchId::from_bytes)?,
            })
        }
        "creer-reserve" => Change::CreateReserve {
            scope: scope(f)?,
            id: node(f, "note")?,
            passage: node(f, "passage")?,
            title: labeled_string(f, "titre")?,
            target: {
                f.label("cible")?;
                f.optional_string("cible")?
            },
        },
        "traiter" => Change::Treat {
            scope: scope(f)?,
            id: edge(f, "relation")?,
            note: node(f, "note")?,
            reserve: node(f, "reserve")?,
        },
        "etat-reserve" => Change::SetReserveStatus {
            scope: scope(f)?,
            reserve: node(f, "reserve")?,
            status: labeled_word(f, "valeur", reserve_from)?,
        },
        "acquitter" => Change::Acknowledge {
            scope: scope(f)?,
            dependent: node(f, "dependant")?,
            cause: node(f, "cause")?,
        },
        "detacher" => Change::Detach {
            scope: scope(f)?,
            edge: edge(f, "relation")?,
        },
        "ancrer" => Change::AnchorToCode {
            id: labeled(f, "ancrage", AnchorId::from_bytes)?,
            node: node(f, "note")?,
            path: labeled_string(f, "chemin")?,
            excerpt: labeled_string(f, "extrait")?,
            git: {
                f.label("git")?;
                f.optional_string("git")?
            },
        },
        "retirer-ancrage" => Change::Unanchor {
            anchor: labeled(f, "ancrage", AnchorId::from_bytes)?,
        },
        "restaurer" => Change::Restore {
            target: labeled(f, "cible", CommitId::from_bytes)?,
        },
        // Restauration ciblée (RESTAURATION.md) : sélection strictement croissante.
        "restaurer-notes" if version == Version::V2 => Change::RestoreNodes {
            target: labeled(f, "cible", CommitId::from_bytes)?,
            nodes: read_ids(f, "notes")?,
        },
        _ => return Err(DecodeReason::Value("commande")),
    };
    Ok(Command::Change(change))
}

/// Sections du dépôt ; elles ne peuvent qu'avancer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Active,
    Branches,
    Identities,
    Commits,
    Operations,
}

/// Fait avancer la section courante ; un retour en arrière viole l'ordre canonique.
fn advance(section: &mut Section, next: Section) -> Result<()> {
    if next < *section {
        return Err(DecodeReason::Order);
    }
    *section = next;
    Ok(())
}

fn fields(line: &str) -> Result<Fields<'_>> {
    Ok(Fields {
        tokens: tokenize(line)?.into_iter(),
    })
}

/// Lit un dépôt complet en forme canonique (FORMAT.md, section 12) et vérifie sa cohérence.
///
/// # Errors
/// [`DecodeError`] au premier écart, avec la ligne et le motif.
pub fn decode_repository(text: &str) -> std::result::Result<Repository, DecodeError> {
    decode_repository_version(text, Version::V1)
}

/// Lit un dépôt complet en version 2 (VERSION2.md, section 6) et vérifie sa cohérence : chaque
/// état est reconstruit, parents d'abord, et validé.
///
/// # Errors
/// [`DecodeError`] au premier écart, avec la ligne et le motif.
pub fn decode_repository_v2(text: &str) -> std::result::Result<Repository, DecodeError> {
    decode_repository_version(text, Version::V2)
}

/// Lit un dépôt complet dans la version qu'annonce sa première ligne.
///
/// # Errors
/// [`DecodeError`] au premier écart, avec la ligne et le motif ; un en-tête inconnu est refusé
/// à la ligne 1.
pub fn decode_repository_any(text: &str) -> std::result::Result<Repository, DecodeError> {
    if text.split('\n').next() == Some(REPOSITORY_HEADER_V2) {
        decode_repository_v2(text)
    } else {
        decode_repository(text)
    }
}

/// Ligne de différence : numéro dans le dépôt, clé et texte de la ligne de l'instantané.
type DiffLine = (usize, Cle, String);

/// Bloc d'un commit en version 2, lu mais pas encore reconstruit.
enum Block {
    /// `  reference` à cette ligne, suivi du texte de l'instantané.
    Reference { line: usize, text: String },
    /// `  difference` à cette ligne, suivi des lignes retirées puis ajoutées.
    Difference {
        line: usize,
        removed: Vec<DiffLine>,
        added: Vec<DiffLine>,
    },
}

/// Commit en version 2 en attente de reconstruction.
struct Pending {
    parent: Option<CommitId>,
    merged: Option<CommitId>,
    reason: String,
    block: Block,
}

/// Parents d'un commit en version 2 : aucun, un, ou deux distincts pour une fusion (premier
/// parent, puis second).
fn read_parents(f: &mut Fields<'_>) -> Result<(Option<CommitId>, Option<CommitId>)> {
    f.label("parents")?;
    match f.integer()? {
        0 => Ok((None, None)),
        1 => Ok((Some(id(f, CommitId::from_bytes)?), None)),
        2 => {
            let first = id(f, CommitId::from_bytes)?;
            let second = id(f, CommitId::from_bytes)?;
            if first == second {
                return Err(DecodeReason::Value("parents"));
            }
            Ok((Some(first), Some(second)))
        }
        _ => Err(DecodeReason::Value("parents")),
    }
}

/// Lit le bloc qui suit l'en-tête d'un commit en version 2, dont `index` désigne la ligne ;
/// `index` désigne ensuite la dernière ligne du bloc. Les lignes du bloc sont délimitées d'un seul
/// tenant, sans boucle d'avancement.
fn read_block(lines: &[&str], index: &mut usize) -> std::result::Result<Block, DecodeError> {
    let header = *index + 1;
    let number = header + 1;
    let error = |line, reason| DecodeError { line, reason };
    let kind = lines.get(header).and_then(|l| l.strip_prefix(INDENT));
    let inner: Vec<&str> = lines
        .get(header + 1..)
        .unwrap_or_default()
        .iter()
        .map_while(|l| l.strip_prefix(BLOCK_INDENT))
        .collect();
    let block = match kind {
        Some("reference") => Block::Reference {
            line: number,
            text: inner.iter().flat_map(|line| [*line, "\n"]).collect(),
        },
        Some("difference") => {
            let mut removed: Vec<DiffLine> = Vec::new();
            let mut added: Vec<DiffLine> = Vec::new();
            for (at, line) in (number + 1..).zip(inner.iter().copied()) {
                let (adds, text) = match (line.strip_prefix("- "), line.strip_prefix("+ ")) {
                    (Some(text), _) => (false, text),
                    (_, Some(text)) => (true, text),
                    _ => return Err(error(at, DecodeReason::Value("différence"))),
                };
                let key = cle(text).ok_or_else(|| {
                    let word = text.split(' ').next().unwrap_or_default();
                    error(at, DecodeReason::UnknownRecord(word.to_owned()))
                })?;
                // Retraits puis ajouts, chacun par clé strictement croissante.
                if !adds && !added.is_empty() {
                    return Err(error(at, DecodeReason::Order));
                }
                let group = if adds { &added } else { &removed };
                if group.last().is_some_and(|(_, last, _)| *last >= key) {
                    return Err(error(at, DecodeReason::Order));
                }
                // Minimalité : une ligne retirée n'est pas remise telle quelle.
                let restored = removed
                    .binary_search_by(|(_, k, _)| k.cmp(&key))
                    .is_ok_and(|k| removed[k].2 == text);
                if adds && restored {
                    return Err(error(at, DecodeReason::Inconsistent("différence minimale")));
                }
                let entry = (at, key, text.to_owned());
                if adds {
                    added.push(entry);
                } else {
                    removed.push(entry);
                }
            }
            Block::Difference {
                line: number,
                removed,
                added,
            }
        }
        _ => {
            return Err(error(
                number,
                DecodeReason::Field("reference ou difference"),
            ))
        }
    };
    *index = header + inner.len();
    Ok(block)
}

/// Applique une différence aux lignes du parent : chaque ligne retirée doit être exactement
/// présente, chaque ligne ajoutée absente ; tout écart est refusé, jamais ignoré.
fn apply(
    mut lines: Lignes,
    removed: &[DiffLine],
    added: &[DiffLine],
) -> std::result::Result<Lignes, DecodeError> {
    let mismatch = |line| DecodeError {
        line,
        reason: DecodeReason::Inconsistent("différence"),
    };
    for (at, key, text) in removed {
        match lines.remove(key) {
            Some(old) if old == *text => {}
            _ => return Err(mismatch(*at)),
        }
    }
    for (at, key, text) in added {
        if lines.insert(*key, text.clone()).is_some() {
            return Err(mismatch(*at));
        }
    }
    Ok(lines)
}

/// Situe dans le dépôt une erreur de l'instantané reconstruit par une différence : à la ligne
/// ajoutée qui l'a introduite, sinon à la ligne `  difference`.
fn locate(error: DecodeError, block: usize, lines: &Lignes, added: &[DiffLine]) -> DecodeError {
    // Ligne n du texte rassemblé : 1 pour l'en-tête, puis une ligne par clé.
    let line = error
        .line
        .checked_sub(2)
        .and_then(|k| lines.keys().nth(k))
        .and_then(|key| added.iter().find(|(_, k, _)| k == key))
        .map_or(block, |(at, _, _)| *at);
    DecodeError {
        line,
        reason: error.reason,
    }
}

/// États reconstruits d'un dépôt en version 2.
struct Rebuilt {
    /// Tous les commits, avec leur état, si la reconstruction les garde.
    commits: BTreeMap<CommitId, Commit>,
    /// Texte de l'état des commits demandés (têtes de branches, pour la vérification).
    heads: BTreeMap<CommitId, String>,
    /// Nombre maximal de tables de lignes gardées à la fois.
    peak: usize,
}

/// Vrai si toutes les identités présentes dans l'état sont employées (FORMAT.md, 12.4).
fn covered(used: &Used, s: &Snapshot) -> bool {
    s.notes.keys().all(|n| used.nodes.contains(n))
        && s.scopes.keys().all(|n| used.scopes.contains(n))
        && s.anchors.keys().all(|n| used.anchors.contains(n))
        && s.scopes
            .values()
            .all(|g| g.edges.keys().all(|e| used.edges.contains(e)))
}

/// Reconstruit l'état de chaque commit, parents d'abord (par profondeur croissante), en
/// réutilisant les lignes du parent ; vérifie la place des états de référence et la couverture
/// des identités. Les lignes d'un commit ne sont gardées que tant qu'un de ses enfants reste à
/// reconstruire ; `keep` garde en plus l'état de chaque commit (décodage complet).
fn rebuild(
    pending: BTreeMap<CommitId, Pending>,
    used: &Used,
    keep: bool,
    wanted: &BTreeSet<CommitId>,
) -> std::result::Result<Rebuilt, DecodeError> {
    let error = |line, reason| DecodeError { line, reason };
    let parents = pending.iter().map(|(id, p)| (*id, p.parent)).collect();
    let depths =
        profondeurs(&parents).ok_or(error(0, DecodeReason::Inconsistent("parent de commit")))?;
    let mut order: Vec<(usize, CommitId)> = depths.iter().map(|(c, d)| (*d, *c)).collect();
    order.sort_unstable();
    // Enfants restant à reconstruire, par commit.
    let mut remaining: BTreeMap<CommitId, usize> = BTreeMap::new();
    for parent in parents.values().flatten() {
        *remaining.entry(*parent).or_default() += 1;
    }
    let mut cache: BTreeMap<CommitId, Lignes> = BTreeMap::new();
    let mut rebuilt = Rebuilt {
        commits: BTreeMap::new(),
        heads: BTreeMap::new(),
        peak: 0,
    };
    for (depth, id) in order {
        let Some(commit) = pending.get(&id) else {
            continue;
        };
        let reference = holds_reference(depth);
        // Tout enfant, référence comprise, décompte son parent : le dernier reprend ses lignes
        // (et les libère), les autres en reçoivent une copie s'ils en ont besoin.
        let base = match commit.parent {
            Some(parent) => {
                let left = remaining.entry(parent).or_default();
                *left = left.saturating_sub(1);
                if *left == 0 {
                    cache.remove(&parent)
                } else if matches!(commit.block, Block::Difference { .. }) {
                    cache.get(&parent).cloned()
                } else {
                    None
                }
            }
            None => None,
        };
        let (state, lines) = match &commit.block {
            Block::Reference { line, text } if reference => {
                let state = decode_snapshot_version(text, Version::V2).map_err(|e| {
                    error(if e.line == 0 { *line } else { line + e.line }, e.reason)
                })?;
                (state, lines_of_text(text))
            }
            Block::Difference {
                line,
                removed,
                added,
            } if !reference => {
                // Profondeur non nulle : le parent existe et a déjà été reconstruit.
                let base = base.ok_or(error(0, DecodeReason::Inconsistent("parent de commit")))?;
                let lines = apply(base, removed, added)?;
                let state = decode_snapshot_version(&assembler(&lines), Version::V2)
                    .map_err(|e| locate(e, *line, &lines, added))?;
                (state, lines)
            }
            Block::Reference { line, .. } | Block::Difference { line, .. } => {
                return Err(error(
                    *line,
                    DecodeReason::Inconsistent("état de référence"),
                ));
            }
        };
        if !covered(used, &state) {
            return Err(error(0, DecodeReason::Inconsistent("identités employées")));
        }
        if wanted.contains(&id) {
            rebuilt.heads.insert(id, assembler(&lines));
        }
        // Un commit traité n'a encore aucun enfant reconstruit : il a une entrée si et seulement
        // s'il a des enfants.
        if remaining.contains_key(&id) {
            cache.insert(id, lines);
            rebuilt.peak = rebuilt.peak.max(cache.len());
        }
        if keep {
            rebuilt.commits.insert(
                id,
                Commit {
                    state: Arc::new(state),
                    parent: commit.parent,
                    merged: commit.merged,
                    reason: commit.reason.clone(),
                },
            );
        }
    }
    Ok(rebuilt)
}

/// Dépôt lu ligne à ligne, avant reconstruction des états (v2) et contrôle de cohérence.
struct Parsed {
    active: BranchId,
    branches: BTreeMap<BranchId, Branch>,
    used: Used,
    journal: BTreeMap<OpId, (Envelope, Outcome)>,
    /// Commits de la version 1, avec leur instantané complet.
    commits: BTreeMap<CommitId, Commit>,
    /// Commits de la version 2, à reconstruire.
    pending: BTreeMap<CommitId, Pending>,
}

fn decode_repository_version(
    text: &str,
    version: Version,
) -> std::result::Result<Repository, DecodeError> {
    let inconsistent = |rule| DecodeError {
        line: 0,
        reason: DecodeReason::Inconsistent(rule),
    };
    let parsed = parse_repository(text, version)?;
    let commits = match version {
        Version::V1 => parsed.commits,
        Version::V2 => rebuild(parsed.pending, &parsed.used, true, &BTreeSet::new())?.commits,
    };
    let parents = commits.iter().map(|(id, c)| (*id, c.parent)).collect();
    let seconds: Vec<CommitId> = commits.values().filter_map(|c| c.merged).collect();
    check(
        &parents,
        &seconds,
        &parsed.branches,
        parsed.active,
        &parsed.journal,
    )
    .map_err(inconsistent)?;
    // En version 1, la couverture des identités se vérifie sur les états lus ; en version 2,
    // pendant leur reconstruction (et de nouveau ici, sans effet).
    if !commits.values().all(|c| covered(&parsed.used, &c.state)) {
        return Err(inconsistent("identités employées"));
    }
    let history = MemoryHistory {
        commits,
        branches: parsed.branches,
        active: parsed.active,
        used: parsed.used,
        journal: parsed.journal,
    };
    // Un historique cohérent s'ouvre toujours ; un refus serait un état invalide.
    Repository::open(history).map_err(|e| DecodeError {
        line: 0,
        reason: DecodeReason::InvalidState(e),
    })
}

/// Résultat de la vérification complète d'un dépôt en version 2 ([`verify_repository_v2`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    /// Texte canonique v2 de l'état de la tête de chaque branche.
    pub heads: BTreeMap<BranchId, String>,
    /// Nombre de commits vérifiés.
    pub commits: usize,
    /// Nombre maximal d'états gardés à la fois pendant la vérification : un état n'est conservé
    /// que tant qu'un de ses enfants reste à reconstruire.
    pub peak_states: usize,
}

/// Vérification complète d'un dépôt en version 2, mêmes règles que [`decode_repository_v2`],
/// sans garder en mémoire l'état de chaque commit (D21) : chaque état est reconstruit, validé,
/// puis oublié dès que ses enfants l'ont été.
///
/// # Errors
/// Les mêmes refus que [`decode_repository_v2`], avec la même ligne et le même motif.
pub fn verify_repository_v2(text: &str) -> std::result::Result<Verified, DecodeError> {
    let parsed = parse_repository(text, Version::V2)?;
    let parents: BTreeMap<CommitId, Option<CommitId>> = parsed
        .pending
        .iter()
        .map(|(id, p)| (*id, p.parent))
        .collect();
    let wanted = parsed.branches.values().map(|b| b.head).collect();
    let seconds: Vec<CommitId> = parsed.pending.values().filter_map(|p| p.merged).collect();
    let commits = parsed.pending.len();
    let rebuilt = rebuild(parsed.pending, &parsed.used, false, &wanted)?;
    check(
        &parents,
        &seconds,
        &parsed.branches,
        parsed.active,
        &parsed.journal,
    )
    .map_err(|rule| DecodeError {
        line: 0,
        reason: DecodeReason::Inconsistent(rule),
    })?;
    let heads = parsed
        .branches
        .iter()
        .filter_map(|(id, b)| rebuilt.heads.get(&b.head).map(|text| (*id, text.clone())))
        .collect();
    Ok(Verified {
        heads,
        commits,
        peak_states: rebuilt.peak,
    })
}

/// Lit tout le texte d'un dépôt jusqu'à `fin`, sans reconstruire les états de la version 2.
fn parse_repository(text: &str, version: Version) -> std::result::Result<Parsed, DecodeError> {
    let error = |line: usize, reason| DecodeError { line, reason };
    let mut lines: Vec<&str> = text.split('\n').collect();
    match lines.pop() {
        Some("") => {}
        _ => return Err(error(lines.len() + 1, DecodeReason::MissingEnd)),
    }
    if lines.first() != Some(&version.repository_header()) {
        return Err(error(1, DecodeReason::Header));
    }
    let mut section = Section::Active;
    let mut active = None;
    let mut branches: BTreeMap<BranchId, Branch> = BTreeMap::new();
    let mut used = Used::default();
    let mut identity_key: Option<(u8, [u8; 16])> = None;
    let mut commits: BTreeMap<CommitId, Commit> = BTreeMap::new();
    let mut pending: BTreeMap<CommitId, Pending> = BTreeMap::new();
    let mut journal: BTreeMap<OpId, (Envelope, Outcome)> = BTreeMap::new();
    let mut index = 1;
    while index < lines.len() {
        let number = index + 1;
        let line = lines[index];
        if line == "fin" {
            if number != lines.len() {
                return Err(error(number + 1, DecodeReason::AfterEnd));
            }
            let active = active.ok_or(error(number, DecodeReason::Field("active")))?;
            return Ok(Parsed {
                active,
                branches,
                used,
                journal,
                commits,
                pending,
            });
        }
        let mut f = fields(line).map_err(|r| error(number, r))?;
        let keyword = f.bare("mot-clé").map_err(|r| error(number, r))?;
        let at = |r| error(number, r);
        match keyword {
            "active" => {
                if active.is_some() || section != Section::Active {
                    return Err(at(DecodeReason::Order));
                }
                active = Some(id(&mut f, BranchId::from_bytes).map_err(at)?);
                f.done().map_err(at)?;
                advance(&mut section, Section::Branches).map_err(at)?;
            }
            "branche" => {
                advance(&mut section, Section::Branches).map_err(at)?;
                let read = |f: &mut Fields<'_>| -> Result<(BranchId, Branch)> {
                    let branch = id(f, BranchId::from_bytes)?;
                    let head = labeled(f, "tete", CommitId::from_bytes)?;
                    f.label("origine")?;
                    let origin = optional_uuid(f, "origine", BranchId::from_bytes)?;
                    let base = labeled(f, "base", CommitId::from_bytes)?;
                    let abandoned = match version {
                        Version::V1 => false,
                        Version::V2 => {
                            f.label("etat")?;
                            match f.bare("etat")? {
                                "active" => false,
                                "abandonnee" => true,
                                _ => return Err(DecodeReason::Value("etat")),
                            }
                        }
                    };
                    Ok((
                        branch,
                        Branch {
                            head,
                            redo: Vec::new(),
                            origin,
                            base,
                            abandoned,
                        },
                    ))
                };
                let (key, branch) = read(&mut f).map_err(at)?;
                f.done().map_err(at)?;
                if branches.keys().next_back().is_some_and(|last| *last >= key) {
                    return Err(at(DecodeReason::Order));
                }
                branches.insert(key, branch);
            }
            "retablir" => {
                let branch = id(&mut f, BranchId::from_bytes).map_err(at)?;
                let rank = f.integer().map_err(at)?;
                let entry = match f.bare("entrée").map_err(at)? {
                    "commit" => Redo::Commit(id(&mut f, CommitId::from_bytes).map_err(at)?),
                    "entrer" => Redo::Enter {
                        branch: id(&mut f, BranchId::from_bytes).map_err(at)?,
                        head: labeled(&mut f, "tete", CommitId::from_bytes).map_err(at)?,
                    },
                    _ => return Err(at(DecodeReason::Value("entrée"))),
                };
                f.done().map_err(at)?;
                match branches.iter_mut().next_back() {
                    Some((last, open)) if *last == branch && open.redo.len() == rank => {
                        open.redo.push(entry);
                    }
                    _ => return Err(at(DecodeReason::OutOfBlock)),
                }
            }
            "identite" => {
                advance(&mut section, Section::Identities).map_err(at)?;
                let category = f.bare("categorie").map_err(at)?;
                let rank = ["note", "relation", "contexte", "ancrage"]
                    .iter()
                    .position(|c| *c == category)
                    .ok_or(at(DecodeReason::Value("categorie")))?;
                let bytes = f.uuid().map_err(at)?;
                f.done().map_err(at)?;
                let key = (u8::try_from(rank).unwrap_or(u8::MAX), bytes);
                if identity_key.is_some_and(|last| last >= key) {
                    return Err(at(DecodeReason::Order));
                }
                identity_key = Some(key);
                match rank {
                    0 => used.nodes.insert(NodeId::from_bytes(bytes)),
                    1 => used.edges.insert(EdgeId::from_bytes(bytes)),
                    2 => used.scopes.insert(ScopeId::from_bytes(bytes)),
                    _ => used.anchors.insert(AnchorId::from_bytes(bytes)),
                };
            }
            "commit" => {
                advance(&mut section, Section::Commits).map_err(at)?;
                let commit = id(&mut f, CommitId::from_bytes).map_err(at)?;
                let (parent, merged) = match version {
                    Version::V1 => {
                        f.label("parent").map_err(at)?;
                        let parent =
                            optional_uuid(&mut f, "parent", CommitId::from_bytes).map_err(at)?;
                        (parent, None)
                    }
                    Version::V2 => read_parents(&mut f).map_err(at)?,
                };
                let reason = labeled_string(&mut f, "motif").map_err(at)?;
                f.done().map_err(at)?;
                let last = commits.keys().chain(pending.keys()).next_back();
                if last.is_some_and(|last| *last >= commit) {
                    return Err(at(DecodeReason::Order));
                }
                if version == Version::V2 {
                    let block = read_block(&lines, &mut index)?;
                    pending.insert(
                        commit,
                        Pending {
                            parent,
                            merged,
                            reason,
                            block,
                        },
                    );
                    index += 1;
                    continue;
                }
                let mut block = String::new();
                while let Some(inner) = lines.get(index + 1).and_then(|l| l.strip_prefix(INDENT)) {
                    block.push_str(inner);
                    block.push('\n');
                    index += 1;
                }
                let state =
                    decode_snapshot_version(&block, Version::V1).map_err(|e| DecodeError {
                        line: if e.line == 0 { number } else { number + e.line },
                        reason: e.reason,
                    })?;
                commits.insert(
                    commit,
                    Commit {
                        state: Arc::new(state),
                        parent,
                        merged,
                        reason,
                    },
                );
            }
            "operation" => {
                advance(&mut section, Section::Operations).map_err(at)?;
                let (op, expected, reason) = read_operation_header(f).map_err(at)?;
                if journal.keys().next_back().is_some_and(|last| *last >= op) {
                    return Err(at(DecodeReason::Order));
                }
                let (envelope, outcome) =
                    read_operation(&lines, &mut index, op, expected, reason, version)?;
                journal.insert(op, (envelope, outcome));
            }
            other => return Err(at(DecodeReason::UnknownRecord(other.to_owned()))),
        }
        index += 1;
    }
    Err(error(lines.len() + 1, DecodeReason::MissingEnd))
}

/// Champs de la ligne `operation …`, après le mot-clé : identité, état attendu, motif.
fn read_operation_header(mut f: Fields<'_>) -> Result<(OpId, Expected, String)> {
    let op = id(&mut f, OpId::from_bytes)?;
    f.label("attendu")?;
    let expected = Expected {
        branch: id(&mut f, BranchId::from_bytes)?,
        head: id(&mut f, CommitId::from_bytes)?,
    };
    let reason = labeled_string(&mut f, "motif")?;
    f.done()?;
    Ok((op, expected, reason))
}

/// Lit un bloc `operation …` en version 2, tel que l'écrit [`encode_operation_v2`].
///
/// # Errors
/// [`DecodeError`] au premier écart ; les lignes sont numérotées à partir de 1 dans le bloc.
pub fn decode_operation_v2(text: &str) -> std::result::Result<(Envelope, Outcome), DecodeError> {
    let error = |line: usize, reason| DecodeError { line, reason };
    let mut lines: Vec<&str> = text.split('\n').collect();
    match lines.pop() {
        Some("") => {}
        _ => return Err(error(lines.len() + 1, DecodeReason::MissingEnd)),
    }
    let at = |reason| error(1, reason);
    let first = lines.first().ok_or(at(DecodeReason::MissingEnd))?;
    let mut f = fields(first).map_err(at)?;
    f.label("operation").map_err(at)?;
    let (op, expected, reason) = read_operation_header(f).map_err(at)?;
    let mut index = 0;
    let operation = read_operation(&lines, &mut index, op, expected, reason, Version::V2)?;
    if index + 1 != lines.len() {
        return Err(error(index + 2, DecodeReason::AfterEnd));
    }
    Ok(operation)
}

/// Lit le bloc indenté d'une opération : commande, résultat, puis effets.
fn read_operation(
    lines: &[&str],
    index: &mut usize,
    op: OpId,
    expected: Expected,
    reason: String,
    version: Version,
) -> std::result::Result<(Envelope, Outcome), DecodeError> {
    let mut next = |what: &'static str| -> std::result::Result<(usize, &str), DecodeError> {
        *index += 1;
        let number = *index + 1;
        lines
            .get(*index)
            .and_then(|l| l.strip_prefix(INDENT))
            .map(|l| (number, l))
            .ok_or(DecodeError {
                line: number,
                reason: DecodeReason::Field(what),
            })
    };
    let (number, line) = next("commande")?;
    let at = |r| DecodeError {
        line: number,
        reason: r,
    };
    let mut f = fields(line).map_err(at)?;
    f.label("commande").map_err(at)?;
    let mut peek = fields(line).map_err(at)?;
    peek.label("commande").map_err(at)?;
    let word = peek.bare("commande").ok();
    let command = if version == Version::V2 && word == Some("fusionner") {
        let source = labeled(&mut peek, "source", BranchId::from_bytes).map_err(at)?;
        let source_head = labeled(&mut peek, "tete", CommitId::from_bytes).map_err(at)?;
        peek.label("resolutions").map_err(at)?;
        let count = peek.integer().map_err(at)?;
        peek.done().map_err(at)?;
        let mut resolutions = Vec::with_capacity(count.min(lines.len()));
        for _ in 0..count {
            let (number, line) = next("resolution")?;
            let at = |r| DecodeError {
                line: number,
                reason: r,
            };
            let line = line
                .strip_prefix(INDENT)
                .ok_or(at(DecodeReason::Field("resolution")))?;
            let f = fields(line).map_err(at)?;
            resolutions.push(read_resolution(f).map_err(at)?);
        }
        Command::Merge {
            source,
            source_head,
            resolutions,
        }
    } else if version == Version::V2 && word == Some("lot") {
        let count = peek.integer().map_err(at)?;
        if count == 0 {
            return Err(at(DecodeReason::Value("lot")));
        }
        peek.done().map_err(at)?;
        let mut elements = Vec::with_capacity(count);
        let mut created = BTreeSet::new();
        for _ in 0..count {
            let (number, line) = next("element")?;
            let at = |r| DecodeError {
                line: number,
                reason: r,
            };
            let line = line
                .strip_prefix(INDENT)
                .ok_or(at(DecodeReason::Field("element")))?;
            let mut f = fields(line).map_err(at)?;
            f.label("element").map_err(at)?;
            let change = match read_command(&mut f, version).map_err(at)? {
                Command::Change(Change::Restore { .. })
                | Command::Undo
                | Command::Redo
                | Command::Checkout { .. }
                | Command::Abandon { .. }
                | Command::Reactivate { .. }
                | Command::Merge { .. }
                | Command::Resume(_)
                | Command::Lot(_) => return Err(at(DecodeReason::Value("element"))),
                Command::Change(change) => change,
            };
            f.done().map_err(at)?;
            if let Some(id) = change.new_id() {
                if !created.insert(id) {
                    return Err(at(DecodeReason::Inconsistent("identités du lot")));
                }
            }
            elements.push(change);
        }
        Command::Lot(elements)
    } else {
        let command = read_command(&mut f, version).map_err(at)?;
        f.done().map_err(at)?;
        command
    };
    let (number, line) = next("resultat")?;
    let at = |r| DecodeError {
        line: number,
        reason: r,
    };
    let mut f = fields(line).map_err(at)?;
    f.label("resultat").map_err(at)?;
    let branch = labeled(&mut f, "branche", BranchId::from_bytes).map_err(at)?;
    let head = labeled(&mut f, "tete", CommitId::from_bytes).map_err(at)?;
    let published = boolean(&mut f, "publie").map_err(at)?;
    let forked = boolean(&mut f, "bifurque").map_err(at)?;
    f.done().map_err(at)?;
    let mut effects = Effects::default();
    while let Some(line) = lines.get(*index + 1).and_then(|l| l.strip_prefix(INDENT)) {
        *index += 1;
        let number = *index + 1;
        let at = |r| DecodeError {
            line: number,
            reason: r,
        };
        let mut f = fields(line).map_err(at)?;
        f.label("effet").map_err(at)?;
        let scope = id(&mut f, ScopeId::from_bytes).map_err(at)?;
        let node = f.node().map_err(at)?;
        let before = read_ids(&mut f, "avant").map_err(at)?;
        let after = read_ids(&mut f, "apres").map_err(at)?;
        f.done().map_err(at)?;
        let key = (scope, node);
        if effects
            .review
            .keys()
            .next_back()
            .is_some_and(|last| *last >= key)
        {
            return Err(at(DecodeReason::Order));
        }
        effects.review.insert(key, ReviewChange { before, after });
    }
    let envelope = Envelope {
        op,
        expected,
        reason,
        command,
    };
    let outcome = Outcome {
        op,
        branch,
        head,
        published,
        forked,
        effects,
    };
    Ok((envelope, outcome))
}

/// Cohérence structurelle du dépôt lu (FORMAT.md, section 12.4) ; rend la règle violée. La
/// couverture des identités par les états est vérifiée à part ([`covered`]).
fn check(
    parents: &BTreeMap<CommitId, Option<CommitId>>,
    seconds: &[CommitId],
    branches: &BTreeMap<BranchId, Branch>,
    active: BranchId,
    journal: &BTreeMap<OpId, (Envelope, Outcome)>,
) -> std::result::Result<(), &'static str> {
    let commit = |id: &CommitId| parents.contains_key(id);
    let branch = |id: &BranchId| branches.contains_key(id);
    match parents.get(&ROOT) {
        Some(None) => {}
        _ => return Err("commit racine"),
    }
    for (id, parent) in parents {
        if *id != ROOT && !parent.as_ref().is_some_and(commit) {
            return Err("parent de commit");
        }
    }
    // Second parent d'un commit de fusion : un commit connu.
    if !seconds.iter().all(commit) {
        return Err("parent de commit");
    }
    match branches.get(&MAIN) {
        Some(main) if main.origin.is_none() && main.base == ROOT && !main.abandoned => {}
        _ => return Err("branche main"),
    }
    if !branches.get(&active).is_some_and(|b| !b.abandoned) {
        return Err("branche active");
    }
    for b in branches.values() {
        if !commit(&b.head) || !commit(&b.base) || !b.origin.as_ref().is_none_or(branch) {
            return Err("tête, base ou origine de branche");
        }
        for entry in &b.redo {
            let ok = match entry {
                Redo::Commit(c) => commit(c),
                Redo::Enter { branch: to, head } => branch(to) && commit(head),
            };
            if !ok {
                return Err("pile de rétablissement");
            }
        }
    }
    for (_, outcome) in journal.values() {
        if !branch(&outcome.branch) || !commit(&outcome.head) {
            return Err("résultat d'opération");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::DEFAULT_SCOPE;
    use crate::model::Kind;

    fn commit(n: u8) -> CommitId {
        let mut bytes = [0; 16];
        bytes[15] = n;
        CommitId::from_bytes(bytes)
    }

    // La migration n'est admise que si la v2 relue redonne exactement le dépôt d'origine.
    #[test]
    fn the_migration_check_refuses_a_different_repository() {
        let empty = Repository::new();
        let mut other = Repository::new();
        let one = commit(1).to_bytes();
        let envelope = Envelope {
            op: OpId::from_bytes(one),
            expected: other.expected(),
            reason: String::new(),
            command: Command::Change(Change::CreateNode {
                scope: DEFAULT_SCOPE,
                id: NodeId::from_bytes(one),
                kind: Kind::Reasoning,
                title: "t".into(),
                text: "t".into(),
            }),
        };
        other.execute(envelope).unwrap();
        assert_eq!(
            verifier_migration(&empty, &encode_repository_v2(&empty)),
            Ok(())
        );
        assert_eq!(
            verifier_migration(&other, &encode_repository_v2(&other)),
            Ok(())
        );
        assert_eq!(
            verifier_migration(&empty, &encode_repository_v2(&other)),
            Err(DecodeError {
                line: 0,
                reason: DecodeReason::Inconsistent("migration"),
            })
        );
        assert_eq!(
            verifier_migration(&empty, ""),
            Err(DecodeError {
                line: 1,
                reason: DecodeReason::Header,
            })
        );
    }

    // Profondeurs le long des premiers parents, quel que soit l'ordre des identifiants ; aucune
    // si un parent manque ou si les parents forment un cycle.
    #[test]
    fn depths_follow_first_parents_and_refuse_cycles() {
        let parents = |pairs: &[(u8, Option<u8>)]| -> BTreeMap<CommitId, Option<CommitId>> {
            pairs
                .iter()
                .map(|(c, p)| (commit(*c), p.map(commit)))
                .collect()
        };
        // Chaîne 0 <- 9 <- 5 <- 7 <- 2, et une seconde branche 5 <- 3.
        let depths = profondeurs(&parents(&[
            (0, None),
            (9, Some(0)),
            (5, Some(9)),
            (7, Some(5)),
            (2, Some(7)),
            (3, Some(5)),
        ]))
        .unwrap();
        let expected: BTreeMap<CommitId, usize> = [(0, 0), (9, 1), (5, 2), (7, 3), (2, 4), (3, 3)]
            .into_iter()
            .map(|(c, d)| (commit(c), d))
            .collect();
        assert_eq!(depths, expected);
        assert_eq!(profondeurs(&BTreeMap::new()), Some(BTreeMap::new()));
        assert_eq!(profondeurs(&parents(&[(0, None), (1, Some(4))])), None);
        assert_eq!(profondeurs(&parents(&[(1, Some(1))])), None);
        assert_eq!(
            profondeurs(&parents(&[(0, None), (1, Some(2)), (2, Some(1))])),
            None
        );
    }
}
