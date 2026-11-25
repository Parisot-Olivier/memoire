//! Format textuel des instantanés, version 1. La référence est FORMAT.md ; ce module l'implémente
//! et ses tests le vérifient. Aucun écart n'est toléré : le décodeur n'accepte que la forme
//! canonique produite par l'encodeur.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::error::Error;
use crate::id::{AnchorId, EdgeId, NodeId, ScopeId};
use crate::model::{
    text_digest, Ack, Anchor, Body, CauseState, Content, Decision, Edge, Fingerprint, Graph,
    Implementation, Kind, Node, Part, Prior, Relation, ReserveStatus, RevisionKind, Snapshot, Text,
    Verdict, Visibility,
};

/// Première ligne d'un instantané en version 1.
pub const SNAPSHOT_HEADER: &str = "memoire-instantane 1";

/// Première ligne d'un instantané en version 2 (VERSION2.md, section 5).
pub const SNAPSHOT_HEADER_V2: &str = "memoire-instantane 2";

/// Version du format textuel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Version {
    V1,
    V2,
}

impl Version {
    pub(crate) fn snapshot_header(self) -> &'static str {
        match self {
            Version::V1 => SNAPSHOT_HEADER,
            Version::V2 => SNAPSHOT_HEADER_V2,
        }
    }
}

/// Mot du rôle d'une note : en version 2, le rôle « passage » s'écrit « citation » (D18).
fn role_word(kind: Kind, version: Version) -> &'static str {
    match (kind, version) {
        (Kind::Passage, Version::V2) => "citation",
        _ => kind_word(kind),
    }
}

fn role_from(word: &str, version: Version) -> Option<Kind> {
    match (word, version) {
        ("citation", Version::V2) => Some(Kind::Passage),
        ("passage", Version::V2) | ("citation", Version::V1) => None,
        _ => kind_from(word),
    }
}

/// Motif d'un refus de lecture (FORMAT.md, section 8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeReason {
    /// Première ligne absente ou différente de l'en-tête attendu.
    Header,
    /// Texte terminé sans `fin`, ou dernière ligne sans saut de ligne.
    MissingEnd,
    /// Contenu après `fin`.
    AfterEnd,
    /// Mot-clé non défini dans cette version.
    UnknownRecord(String),
    /// Étiquette attendue absente, champ manquant ou en trop.
    Field(&'static str),
    /// Jeton mal formé ou valeur inconnue.
    Value(&'static str),
    /// Ordre canonique non respecté ou doublon.
    Order,
    /// Ligne hors de son bloc.
    OutOfBlock,
    /// L'instantané lu viole une règle du domaine.
    InvalidState(Error),
    /// Le dépôt lu viole une règle de cohérence (FORMAT.md, section 12.4), nommée.
    Inconsistent(&'static str),
    /// Enregistrement prévu par la version 2 mais pas encore pris en charge par ce logiciel
    /// (VERSION2.md, section 8) ; nommé.
    NotYetSupported(&'static str),
}

/// Refus de lecture : ligne (à partir de 1 ; 0 pour la validation globale) et motif.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    /// Ligne fautive.
    pub line: usize,
    /// Motif.
    pub reason: DecodeReason,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ligne {} : ", self.line)?;
        match &self.reason {
            DecodeReason::Header => f.write_str("en-tête attendu absent ou d'une autre version"),
            DecodeReason::MissingEnd => f.write_str("fin manquante"),
            DecodeReason::AfterEnd => f.write_str("contenu après la fin"),
            DecodeReason::UnknownRecord(word) => write!(f, "enregistrement inconnu « {word} »"),
            DecodeReason::Field(label) => write!(f, "champ attendu : {label}"),
            DecodeReason::Value(kind) => write!(f, "valeur invalide : {kind}"),
            DecodeReason::Order => f.write_str("ordre canonique non respecté"),
            DecodeReason::OutOfBlock => f.write_str("ligne hors de son bloc"),
            DecodeReason::InvalidState(error) => write!(f, "état invalide : {error}"),
            DecodeReason::Inconsistent(rule) => write!(f, "dépôt incohérent : {rule}"),
            DecodeReason::NotYetSupported(what) => {
                write!(f, "pas encore pris en charge par ce logiciel : {what}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

// ----- Correspondances entre valeurs et mots (sans cas générique côté encodage) -----

macro_rules! words {
    ($to:ident, $from:ident, $t:ty, { $($value:path => $word:literal),+ $(,)? }) => {
        pub(crate) fn $to(value: $t) -> &'static str {
            match value {
                $($value => $word,)+
            }
        }

        pub(crate) fn $from(word: &str) -> Option<$t> {
            match word {
                $($word => Some($value),)+
                _ => None,
            }
        }
    };
}

words!(kind_word, kind_from, Kind, {
    Kind::Category => "categorie",
    Kind::Reasoning => "raisonnement",
    Kind::Implementation => "implementation",
    Kind::PersonalNote => "note",
    Kind::Exchange => "echange",
    Kind::Passage => "passage",
    Kind::Reserve => "reserve",
});
words!(revision_word, revision_from, RevisionKind, {
    RevisionKind::Minor => "mineure",
    RevisionKind::Substantive => "substantielle",
});
words!(verdict_word, verdict_from, Verdict, {
    Verdict::Unverified => "non-verifiee",
    Verdict::Verified => "verifiee",
    Verdict::Refuted => "refutee",
});
words!(decision_word, decision_from, Decision, {
    Decision::Proposed => "proposee",
    Decision::Retained => "retenue",
    Decision::NoLongerRetained => "plus-retenue",
    Decision::Replaced => "remplacee",
});
words!(implementation_word, implementation_from, Implementation, {
    Implementation::NotStarted => "non-commencee",
    Implementation::InProgress => "en-cours",
    Implementation::Implemented => "implementee",
});
words!(reserve_word, reserve_from, ReserveStatus, {
    ReserveStatus::Open => "ouverte",
    ReserveStatus::TreatedPendingVerification => "traitee-a-verifier",
    ReserveStatus::Resolved => "resolue",
});
words!(visibility_word, visibility_from, Visibility, {
    Visibility::Current => "courante",
    Visibility::Archived => "retiree",
});
words!(relation_word, relation_from, Relation, {
    Relation::ClassifiedIn => "classe-dans",
    Relation::DependsOn => "depend-de",
    Relation::DevelopedFrom => "developpe-a-partir-de",
    Relation::Related => "lien",
    Relation::Replaces => "remplace",
    Relation::RestrictsScope => "restreint",
    Relation::Contradicts => "contredit",
    Relation::AppliesTo => "s-applique-a",
    Relation::Treats => "traite",
});
words!(part_word, part_from, Part, {
    Part::Question => "question",
    Part::Answer => "reponse",
});

// ----- Encodage -----

/// Ligne en construction : jetons séparés par une seule espace.
pub(crate) struct Line(pub(crate) String);

impl Line {
    pub(crate) fn new(keyword: &str) -> Self {
        Self(keyword.to_owned())
    }

    pub(crate) fn word(mut self, word: &str) -> Self {
        self.0.push(' ');
        self.0.push_str(word);
        self
    }

    pub(crate) fn value(self, value: impl fmt::Display) -> Self {
        self.word(&value.to_string())
    }

    pub(crate) fn string(mut self, text: &str) -> Self {
        self.0.push_str(" \"");
        for c in text.chars() {
            match c {
                '\\' => self.0.push_str("\\\\"),
                '"' => self.0.push_str("\\\""),
                '\n' => self.0.push_str("\\n"),
                '\r' => self.0.push_str("\\r"),
                '\t' => self.0.push_str("\\t"),
                other => self.0.push(other),
            }
        }
        self.0.push('"');
        self
    }

    pub(crate) fn optional_string(self, text: Option<&str>) -> Self {
        match text {
            Some(text) => self.string(text),
            None => self.word("-"),
        }
    }

    pub(crate) fn optional(self, word: Option<&str>) -> Self {
        self.word(word.unwrap_or("-"))
    }

    pub(crate) fn end(mut self, out: &mut String) {
        self.0.push('\n');
        out.push_str(&self.0);
    }
}

pub(crate) fn prior_word(prior: Option<Prior>) -> String {
    match prior {
        Some(Prior::Decision(d)) => format!("decision:{}", decision_word(d)),
        Some(Prior::Reserve(r)) => format!("reserve:{}", reserve_word(r)),
        None => "-".to_owned(),
    }
}

/// Texte canonique d'un instantané, version 1 (FORMAT.md, sections 2 à 6).
#[must_use]
pub fn encode_snapshot(snapshot: &Snapshot) -> String {
    encode_snapshot_version(snapshot, Version::V1)
}

/// Texte canonique d'un instantané, version 2 (VERSION2.md, section 5).
#[must_use]
pub fn encode_snapshot_v2(snapshot: &Snapshot) -> String {
    encode_snapshot_version(snapshot, Version::V2)
}

pub(crate) fn encode_snapshot_version(snapshot: &Snapshot, version: Version) -> String {
    let mut out = String::new();
    out.push_str(version.snapshot_header());
    out.push('\n');
    for (id, content) in &snapshot.notes {
        Line::new("contenu")
            .value(id)
            .word(role_word(content.kind, version))
            .word("titre")
            .string(&content.created.title)
            .word("texte")
            .string(&content.created.text)
            .end(&mut out);
        for (rang, (kind, text)) in (1..).zip(&content.revisions) {
            let line = Line::new("revision").value(id);
            let line = if version == Version::V2 {
                line.value(rang)
            } else {
                line
            };
            line.word(revision_word(*kind))
                .word("titre")
                .string(&text.title)
                .word("texte")
                .string(&text.text)
                .end(&mut out);
        }
        match &content.body {
            Body::Note => {}
            Body::Exchange {
                question,
                answer,
                previous,
            } => Line::new("echange")
                .value(id)
                .word("question")
                .optional_string(question.as_deref())
                .word("reponse")
                .string(answer)
                .word("precedent")
                .optional(previous.map(|p| p.to_string()).as_deref())
                .end(&mut out),
            // La version 1 ne représente pas la citation d'une note : la ligne est écrite comme en
            // version 2, et le décodeur de la version 1 la refuse.
            Body::Citation {
                note,
                version: cited,
                start,
                end,
            } => Line::new("citation")
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
                .value(end)
                .word("empreinte")
                .word(&format!("{:016x}", text_digest(&content.created.text)))
                .end(&mut out),
            Body::Passage {
                exchange,
                part,
                start,
                end,
            } if version == Version::V2 => Line::new("citation")
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
                .value(end)
                .word("empreinte")
                .word(&format!("{:016x}", text_digest(&content.created.text)))
                .end(&mut out),
            Body::Passage {
                exchange,
                part,
                start,
                end,
            } => Line::new("passage")
                .value(id)
                .word("echange")
                .value(exchange)
                .word("partie")
                .word(part_word(*part))
                .word("debut")
                .value(start)
                .word("fin")
                .value(end)
                .end(&mut out),
            Body::Reserve { passage, target } => Line::new("reserve")
                .value(id)
                .word("passage")
                .value(passage)
                .word("cible")
                .optional_string(target.as_deref())
                .end(&mut out),
        }
    }
    for (id, anchor) in &snapshot.anchors {
        Line::new("ancrage")
            .value(id)
            .word("note")
            .value(anchor.node)
            .word("etat")
            .word(if anchor.active { "actif" } else { "retire" })
            .word("chemin")
            .string(&anchor.path)
            .word("extrait")
            .string(&anchor.excerpt)
            .word("empreinte")
            .word(&format!("{:016x}", anchor.digest))
            .word("git")
            .optional_string(anchor.git.as_deref())
            .end(&mut out);
    }
    for (scope, graph) in &snapshot.scopes {
        Line::new("contexte").value(scope).end(&mut out);
        for (id, node) in &graph.nodes {
            Line::new("membre")
                .value(scope)
                .value(id)
                .word("verdict")
                .optional(node.verdict.map(verdict_word))
                .word("decision")
                .optional(node.decision.map(decision_word))
                .word("implementation")
                .optional(node.implementation.map(implementation_word))
                .word("reserve")
                .optional(node.reserve.map(reserve_word))
                .word("visibilite")
                .word(visibility_word(node.visibility))
                .end(&mut out);
        }
        for (id, edge) in &graph.edges {
            Line::new("relation")
                .value(scope)
                .value(id)
                .value(edge.from)
                .value(edge.to)
                .word(relation_word(edge.relation))
                .word("etat")
                .word(if edge.active { "active" } else { "inactive" })
                .word("empreinte")
                .value(edge.baseline.substantive)
                .value(edge.baseline.restrictions)
                .word("annotation")
                .optional_string(edge.annotation.as_deref())
                .word("anterieur")
                .word(&prior_word(edge.prior))
                .end(&mut out);
        }
        for ((dependent, cause), ack) in &graph.acks {
            Line::new("acquittement")
                .value(scope)
                .value(dependent)
                .value(cause)
                .word("sens")
                .value(ack.dependent)
                .word("empreinte")
                .value(ack.cause.fingerprint.substantive)
                .value(ack.cause.fingerprint.restrictions)
                .word("verdict")
                .optional(ack.cause.verdict.map(verdict_word))
                .word("decision")
                .optional(ack.cause.decision.map(decision_word))
                .word("visibilite")
                .word(visibility_word(ack.cause.visibility))
                .end(&mut out);
        }
    }
    out.push_str("fin\n");
    out
}

// ----- Décodage -----

pub(crate) type Result<T> = std::result::Result<T, DecodeReason>;

pub(crate) enum Token<'a> {
    Bare(&'a str),
    Text(String),
}

/// Découpe une ligne en jetons : une seule espace entre jetons, aucune en bordure.
pub(crate) fn tokenize(line: &str) -> Result<Vec<Token<'_>>> {
    let mut tokens = Vec::new();
    let mut rest = line;
    loop {
        if let Some(after) = rest.strip_prefix('"') {
            let (text, consumed) = unquote(after)?;
            tokens.push(Token::Text(text));
            rest = &after[consumed..];
        } else {
            let end = rest.find(' ').unwrap_or(rest.len());
            let bare = &rest[..end];
            let allowed =
                |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == ':';
            if bare.is_empty() || !bare.chars().all(allowed) {
                return Err(DecodeReason::Value("jeton"));
            }
            tokens.push(Token::Bare(bare));
            rest = &rest[end..];
        }
        if rest.is_empty() {
            return Ok(tokens);
        }
        rest = rest
            .strip_prefix(' ')
            .ok_or(DecodeReason::Value("séparateur"))?;
    }
}

/// Lit une chaîne échappée jusqu'au guillemet fermant ; rend le texte et les octets consommés.
fn unquote(text: &str) -> Result<(String, usize)> {
    let mut out = String::new();
    let mut chars = text.char_indices();
    while let Some((index, c)) = chars.next() {
        match c {
            '"' => return Ok((out, index + 1)),
            '\\' => match chars.next().map(|(_, e)| e) {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                _ => return Err(DecodeReason::Value("échappement")),
            },
            other => out.push(other),
        }
    }
    Err(DecodeReason::Value("chaîne non fermée"))
}

/// Champs d'une ligne, lus dans l'ordre.
pub(crate) struct Fields<'a> {
    pub(crate) tokens: std::vec::IntoIter<Token<'a>>,
}

impl<'a> Fields<'a> {
    pub(crate) fn bare(&mut self, what: &'static str) -> Result<&'a str> {
        match self.tokens.next() {
            Some(Token::Bare(word)) => Ok(word),
            Some(Token::Text(_)) => Err(DecodeReason::Value(what)),
            None => Err(DecodeReason::Field(what)),
        }
    }

    pub(crate) fn label(&mut self, label: &'static str) -> Result<()> {
        match self.tokens.next() {
            Some(Token::Bare(word)) if word == label => Ok(()),
            _ => Err(DecodeReason::Field(label)),
        }
    }

    pub(crate) fn uuid(&mut self) -> Result<[u8; 16]> {
        let word = self.bare("identifiant")?;
        NodeId::parse(word)
            .map(NodeId::to_bytes)
            .ok_or(DecodeReason::Value("identifiant"))
    }

    pub(crate) fn node(&mut self) -> Result<NodeId> {
        self.uuid().map(NodeId::from_bytes)
    }

    pub(crate) fn integer(&mut self) -> Result<usize> {
        let word = self.bare("entier")?;
        let canonical = word == "0" || !word.starts_with('0');
        match word.parse::<usize>() {
            Ok(n) if canonical && word.chars().all(|c| c.is_ascii_digit()) => Ok(n),
            _ => Err(DecodeReason::Value("entier")),
        }
    }

    pub(crate) fn string(&mut self, what: &'static str) -> Result<String> {
        match self.tokens.next() {
            Some(Token::Text(text)) => Ok(text),
            Some(Token::Bare(_)) => Err(DecodeReason::Value(what)),
            None => Err(DecodeReason::Field(what)),
        }
    }

    pub(crate) fn optional_string(&mut self, what: &'static str) -> Result<Option<String>> {
        match self.tokens.next() {
            Some(Token::Text(text)) => Ok(Some(text)),
            Some(Token::Bare("-")) => Ok(None),
            Some(Token::Bare(_)) => Err(DecodeReason::Value(what)),
            None => Err(DecodeReason::Field(what)),
        }
    }

    /// Empreinte : exactement seize chiffres hexadécimaux en minuscules.
    pub(crate) fn digest(&mut self) -> Result<u64> {
        let digest = self.bare("empreinte")?;
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if digest.len() != 16 || !digest.chars().all(hex) {
            return Err(DecodeReason::Value("empreinte"));
        }
        u64::from_str_radix(digest, 16).map_err(|_| DecodeReason::Value("empreinte"))
    }

    pub(crate) fn word<T>(&mut self, what: &'static str, from: fn(&str) -> Option<T>) -> Result<T> {
        from(self.bare(what)?).ok_or(DecodeReason::Value(what))
    }

    pub(crate) fn optional<T>(
        &mut self,
        what: &'static str,
        from: fn(&str) -> Option<T>,
    ) -> Result<Option<T>> {
        match self.bare(what)? {
            "-" => Ok(None),
            word => from(word).map(Some).ok_or(DecodeReason::Value(what)),
        }
    }

    pub(crate) fn done(mut self) -> Result<()> {
        match self.tokens.next() {
            None => Ok(()),
            Some(_) => Err(DecodeReason::Field("fin de ligne")),
        }
    }
}

pub(crate) fn prior_from(word: &str) -> Option<Prior> {
    if let Some(d) = word.strip_prefix("decision:") {
        return decision_from(d).map(Prior::Decision);
    }
    word.strip_prefix("reserve:")
        .and_then(reserve_from)
        .map(Prior::Reserve)
}

/// Section du texte en cours de lecture ; elle ne peut qu'avancer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Notes,
    Anchors,
    Scopes,
}

/// Bloc de contexte en cours de lecture ; ses sous-sections ne peuvent qu'avancer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Block {
    Members,
    Relations,
    Acks,
}

/// Données d'un membre, en attendant le contenu canonique de sa note.
struct Member {
    line: usize,
    verdict: Option<Verdict>,
    decision: Option<Decision>,
    implementation: Option<Implementation>,
    reserve: Option<ReserveStatus>,
    visibility: Visibility,
}

#[derive(Default)]
struct ScopeData {
    members: BTreeMap<NodeId, Member>,
    edges: BTreeMap<EdgeId, Edge>,
    acks: BTreeMap<(NodeId, NodeId), Ack>,
}

/// État du lecteur : ce qui a été lu et ce qui peut venir ensuite.
struct Reader {
    version: Version,
    /// Ligne en cours de lecture, pour situer une note manquante d'un membre.
    line: usize,
    section: Section,
    notes: BTreeMap<NodeId, Content>,
    /// Note dont on peut encore lire révisions et données propres ; faux une fois les données lues.
    open: Option<(NodeId, bool)>,
    anchors: BTreeMap<AnchorId, Anchor>,
    scopes: BTreeMap<ScopeId, ScopeData>,
    scope: Option<(ScopeId, Block)>,
}

/// Vérifie qu'une clé suit strictement la dernière clé de sa table.
fn after<K: Ord, V>(map: &BTreeMap<K, V>, key: &K) -> Result<()> {
    match map.keys().next_back() {
        Some(last) if last >= key => Err(DecodeReason::Order),
        _ => Ok(()),
    }
}

impl Reader {
    fn enter(&mut self, section: Section) -> Result<()> {
        if section < self.section {
            return Err(DecodeReason::Order);
        }
        self.section = section;
        Ok(())
    }

    /// Note ouverte pour une ligne de révision ou de données propres.
    fn open_note(&mut self, id: NodeId, body: bool) -> Result<&mut Content> {
        match self.open {
            Some((open, false)) if open == id => {
                if body {
                    self.open = Some((open, true));
                }
                self.notes.get_mut(&id).ok_or(DecodeReason::OutOfBlock)
            }
            _ => Err(DecodeReason::OutOfBlock),
        }
    }

    /// Bloc de contexte ouvert pour une ligne de la sous-section donnée.
    fn open_scope(&mut self, scope: ScopeId, block: Block) -> Result<&mut ScopeData> {
        match self.scope {
            Some((open, current)) if open == scope && current <= block => {
                self.scope = Some((open, block));
                self.scopes.get_mut(&scope).ok_or(DecodeReason::OutOfBlock)
            }
            Some((open, _)) if open == scope => Err(DecodeReason::Order),
            _ => Err(DecodeReason::OutOfBlock),
        }
    }

    fn record(&mut self, keyword: &str, mut f: Fields<'_>) -> Result<()> {
        match keyword {
            "contenu" => {
                self.enter(Section::Notes)?;
                let id = f.node()?;
                let kind =
                    role_from(f.bare("rôle")?, self.version).ok_or(DecodeReason::Value("rôle"))?;
                f.label("titre")?;
                let title = f.string("titre")?;
                f.label("texte")?;
                let text = f.string("texte")?;
                f.done()?;
                after(&self.notes, &id)?;
                self.notes.insert(id, Content::new(kind, &title, &text));
                self.open = Some((id, false));
            }
            "revision" => {
                let id = f.node()?;
                let rang = if self.version == Version::V2 {
                    Some(f.integer()?)
                } else {
                    None
                };
                let kind = f.word("nature", revision_from)?;
                f.label("titre")?;
                let title = f.string("titre")?;
                f.label("texte")?;
                let text = f.string("texte")?;
                f.done()?;
                let content = self.open_note(id, false)?;
                if rang.is_some_and(|r| r != content.revisions.len() + 1) {
                    return Err(DecodeReason::Value("rang"));
                }
                content.revisions.push((kind, Text { title, text }));
            }
            "echange" => {
                let id = f.node()?;
                f.label("question")?;
                let question = f.optional_string("question")?;
                f.label("reponse")?;
                let answer = f.string("reponse")?;
                f.label("precedent")?;
                let previous = match f.bare("precedent")? {
                    "-" => None,
                    word => Some(NodeId::parse(word).ok_or(DecodeReason::Value("identifiant"))?),
                };
                f.done()?;
                self.open_note(id, true)?.body = Body::Exchange {
                    question: question.map(Arc::from),
                    answer: Arc::from(answer),
                    previous,
                };
            }
            "citation" if self.version == Version::V2 => {
                let id = f.node()?;
                f.label("source")?;
                let exchange = f.node()?;
                f.label("partie")?;
                // Citation d'une note : `partie note`, version au rang de révision (D18).
                let part = f.bare("partie")?;
                f.label("version")?;
                let version = f.bare("version")?;
                let source = match part {
                    "note" => Err(version
                        .parse::<usize>()
                        .map_err(|_| DecodeReason::Value("version"))?),
                    word => {
                        if version != "-" {
                            return Err(DecodeReason::Value("version"));
                        }
                        Ok(part_from(word).ok_or(DecodeReason::Value("partie"))?)
                    }
                };
                f.label("debut")?;
                let start = f.integer()?;
                f.label("fin")?;
                let end = f.integer()?;
                f.label("empreinte")?;
                let digest = f.bare("empreinte")?;
                f.done()?;
                let content = self.open_note(id, true)?;
                if digest != format!("{:016x}", text_digest(&content.created.text)) {
                    return Err(DecodeReason::Value("empreinte"));
                }
                // Ok : partie d'un échange ; Err : version d'une note.
                content.body = match source {
                    Ok(part) => Body::Passage {
                        exchange,
                        part,
                        start,
                        end,
                    },
                    Err(version) => Body::Citation {
                        note: exchange,
                        version,
                        start,
                        end,
                    },
                };
            }
            "passage" if self.version == Version::V1 => {
                let id = f.node()?;
                f.label("echange")?;
                let exchange = f.node()?;
                f.label("partie")?;
                let part = f.word("partie", part_from)?;
                f.label("debut")?;
                let start = f.integer()?;
                f.label("fin")?;
                let end = f.integer()?;
                f.done()?;
                self.open_note(id, true)?.body = Body::Passage {
                    exchange,
                    part,
                    start,
                    end,
                };
            }
            "reserve" => {
                let id = f.node()?;
                f.label("passage")?;
                let passage = f.node()?;
                f.label("cible")?;
                let target = f.optional_string("cible")?;
                f.done()?;
                self.open_note(id, true)?.body = Body::Reserve { passage, target };
            }
            "ancrage" => {
                self.enter(Section::Anchors)?;
                let id = AnchorId::from_bytes(f.uuid()?);
                f.label("note")?;
                let node = f.node()?;
                f.label("etat")?;
                let active = match f.bare("etat")? {
                    "actif" => true,
                    "retire" => false,
                    _ => return Err(DecodeReason::Value("etat")),
                };
                f.label("chemin")?;
                let path = f.string("chemin")?;
                f.label("extrait")?;
                let excerpt = f.string("extrait")?;
                f.label("empreinte")?;
                let digest = f.digest()?;
                f.label("git")?;
                let git = f.optional_string("git")?;
                f.done()?;
                after(&self.anchors, &id)?;
                self.anchors.insert(
                    id,
                    Anchor {
                        node,
                        path,
                        excerpt,
                        digest,
                        git,
                        active,
                    },
                );
            }
            "contexte" => {
                self.enter(Section::Scopes)?;
                let scope = ScopeId::from_bytes(f.uuid()?);
                f.done()?;
                after(&self.scopes, &scope)?;
                self.scopes.insert(scope, ScopeData::default());
                self.scope = Some((scope, Block::Members));
            }
            "membre" => {
                let scope = ScopeId::from_bytes(f.uuid()?);
                let node = f.node()?;
                f.label("verdict")?;
                let verdict = f.optional("verdict", verdict_from)?;
                f.label("decision")?;
                let decision = f.optional("decision", decision_from)?;
                f.label("implementation")?;
                let implementation = f.optional("implementation", implementation_from)?;
                f.label("reserve")?;
                let reserve = f.optional("reserve", reserve_from)?;
                f.label("visibilite")?;
                let visibility = f.word("visibilite", visibility_from)?;
                f.done()?;
                let line = self.line;
                let data = self.open_scope(scope, Block::Members)?;
                after(&data.members, &node)?;
                data.members.insert(
                    node,
                    Member {
                        line,
                        verdict,
                        decision,
                        implementation,
                        reserve,
                        visibility,
                    },
                );
            }
            "relation" => {
                let scope = ScopeId::from_bytes(f.uuid()?);
                let id = EdgeId::from_bytes(f.uuid()?);
                let from = f.node()?;
                let to = f.node()?;
                let relation = f.word("type", relation_from)?;
                f.label("etat")?;
                let active = match f.bare("etat")? {
                    "active" => true,
                    "inactive" => false,
                    _ => return Err(DecodeReason::Value("etat")),
                };
                f.label("empreinte")?;
                let baseline = Fingerprint {
                    substantive: f.integer()?,
                    restrictions: f.integer()?,
                };
                f.label("annotation")?;
                let annotation = f.optional_string("annotation")?;
                f.label("anterieur")?;
                let prior = f.optional("anterieur", prior_from)?;
                f.done()?;
                let data = self.open_scope(scope, Block::Relations)?;
                after(&data.edges, &id)?;
                data.edges.insert(
                    id,
                    Edge {
                        from,
                        to,
                        relation,
                        active,
                        baseline,
                        annotation,
                        prior,
                    },
                );
            }
            "acquittement" => {
                let scope = ScopeId::from_bytes(f.uuid()?);
                let dependent = f.node()?;
                let cause = f.node()?;
                f.label("sens")?;
                let meaning = f.integer()?;
                f.label("empreinte")?;
                let fingerprint = Fingerprint {
                    substantive: f.integer()?,
                    restrictions: f.integer()?,
                };
                f.label("verdict")?;
                let verdict = f.optional("verdict", verdict_from)?;
                f.label("decision")?;
                let decision = f.optional("decision", decision_from)?;
                f.label("visibilite")?;
                let visibility = f.word("visibilite", visibility_from)?;
                f.done()?;
                let data = self.open_scope(scope, Block::Acks)?;
                let key = (dependent, cause);
                after(&data.acks, &key)?;
                data.acks.insert(
                    key,
                    Ack {
                        dependent: meaning,
                        cause: CauseState {
                            fingerprint,
                            verdict,
                            decision,
                            visibility,
                        },
                    },
                );
            }
            other => return Err(DecodeReason::UnknownRecord(other.to_owned())),
        }
        Ok(())
    }

    /// Assemble l'instantané : chaque membre reçoit le contenu canonique de sa note.
    fn finish(self) -> std::result::Result<Snapshot, DecodeError> {
        let notes: BTreeMap<NodeId, Arc<Content>> = self
            .notes
            .into_iter()
            .map(|(id, c)| (id, Arc::new(c)))
            .collect();
        let mut scopes = BTreeMap::new();
        for (scope, data) in self.scopes {
            let mut graph = Graph {
                nodes: BTreeMap::new(),
                edges: data.edges,
                acks: data.acks,
            };
            for (id, member) in data.members {
                let content = notes.get(&id).ok_or(DecodeError {
                    line: member.line,
                    reason: DecodeReason::InvalidState(Error::MissingNode(id)),
                })?;
                graph.nodes.insert(
                    id,
                    Node {
                        content: Arc::clone(content),
                        verdict: member.verdict,
                        decision: member.decision,
                        implementation: member.implementation,
                        reserve: member.reserve,
                        visibility: member.visibility,
                    },
                );
            }
            scopes.insert(scope, graph);
        }
        let snapshot = Snapshot {
            notes,
            scopes,
            anchors: self.anchors,
        };
        snapshot.validate().map_err(|error| DecodeError {
            line: 0,
            reason: DecodeReason::InvalidState(error),
        })?;
        Ok(snapshot)
    }
}

/// Lit un instantané en forme canonique, version 1 (FORMAT.md) et le valide.
///
/// # Errors
/// [`DecodeError`] au premier écart, avec la ligne et le motif.
pub fn decode_snapshot(text: &str) -> std::result::Result<Snapshot, DecodeError> {
    decode_snapshot_version(text, Version::V1)
}

/// Lit un instantané en forme canonique, version 2 (VERSION2.md, section 5) et le valide.
///
/// # Errors
/// [`DecodeError`] au premier écart, avec la ligne et le motif.
pub fn decode_snapshot_v2(text: &str) -> std::result::Result<Snapshot, DecodeError> {
    decode_snapshot_version(text, Version::V2)
}

pub(crate) fn decode_snapshot_version(
    text: &str,
    version: Version,
) -> std::result::Result<Snapshot, DecodeError> {
    let error = |line: usize, reason| DecodeError { line, reason };
    let mut lines: Vec<&str> = text.split('\n').collect();
    // Le texte se termine par un saut de ligne : le dernier morceau est vide.
    match lines.pop() {
        Some("") => {}
        _ => return Err(error(lines.len() + 1, DecodeReason::MissingEnd)),
    }
    if lines.first() != Some(&version.snapshot_header()) {
        return Err(error(1, DecodeReason::Header));
    }
    let mut reader = Reader {
        version,
        line: 0,
        section: Section::Notes,
        notes: BTreeMap::new(),
        open: None,
        anchors: BTreeMap::new(),
        scopes: BTreeMap::new(),
        scope: None,
    };
    for (index, line) in lines.iter().enumerate().skip(1) {
        let number = index + 1;
        if *line == "fin" {
            return if number == lines.len() {
                reader.finish()
            } else {
                Err(error(number + 1, DecodeReason::AfterEnd))
            };
        }
        reader.line = number;
        let tokens = tokenize(line).map_err(|reason| error(number, reason))?;
        let mut fields = Fields {
            tokens: tokens.into_iter(),
        };
        let keyword = fields
            .bare("mot-clé")
            .map_err(|reason| error(number, reason))?;
        reader
            .record(keyword, fields)
            .map_err(|reason| error(number, reason))?;
    }
    Err(error(lines.len() + 1, DecodeReason::MissingEnd))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Chaque valeur a un mot, et chaque mot redonne sa valeur ; un mot inconnu est refusé.
    #[test]
    fn every_word_maps_back_to_its_value() {
        for v in [
            Kind::Category,
            Kind::Reasoning,
            Kind::Implementation,
            Kind::PersonalNote,
            Kind::Exchange,
            Kind::Passage,
            Kind::Reserve,
        ] {
            assert_eq!(kind_from(kind_word(v)), Some(v));
        }
        for v in [RevisionKind::Minor, RevisionKind::Substantive] {
            assert_eq!(revision_from(revision_word(v)), Some(v));
        }
        for v in [Verdict::Unverified, Verdict::Verified, Verdict::Refuted] {
            assert_eq!(verdict_from(verdict_word(v)), Some(v));
        }
        for v in [
            Decision::Proposed,
            Decision::Retained,
            Decision::NoLongerRetained,
            Decision::Replaced,
        ] {
            assert_eq!(decision_from(decision_word(v)), Some(v));
            assert_eq!(
                prior_from(&prior_word(Some(Prior::Decision(v)))),
                Some(Prior::Decision(v))
            );
        }
        for v in [
            Implementation::NotStarted,
            Implementation::InProgress,
            Implementation::Implemented,
        ] {
            assert_eq!(implementation_from(implementation_word(v)), Some(v));
        }
        for v in [
            ReserveStatus::Open,
            ReserveStatus::TreatedPendingVerification,
            ReserveStatus::Resolved,
        ] {
            assert_eq!(reserve_from(reserve_word(v)), Some(v));
            assert_eq!(
                prior_from(&prior_word(Some(Prior::Reserve(v)))),
                Some(Prior::Reserve(v))
            );
        }
        for v in [Visibility::Current, Visibility::Archived] {
            assert_eq!(visibility_from(visibility_word(v)), Some(v));
        }
        for v in [
            Relation::ClassifiedIn,
            Relation::DependsOn,
            Relation::DevelopedFrom,
            Relation::Related,
            Relation::Replaces,
            Relation::RestrictsScope,
            Relation::Contradicts,
            Relation::AppliesTo,
            Relation::Treats,
        ] {
            assert_eq!(relation_from(relation_word(v)), Some(v));
        }
        for v in [Part::Question, Part::Answer] {
            assert_eq!(part_from(part_word(v)), Some(v));
        }
        assert_eq!(prior_word(None), "-");
        for unknown in [
            "",
            "inconnu",
            "decision:",
            "reserve:inconnue",
            "decision:ouverte",
        ] {
            assert_eq!(prior_from(unknown), None, "{unknown}");
            assert_eq!(kind_from(unknown), None);
        }
    }

    // Chaînes : cinq échappements exactement, tout le reste tel quel ; aller-retour exact.
    #[test]
    fn strings_escape_exactly_five_characters() {
        let text = "a\\b\"c\nd\re\tf \u{a0}\u{2060}\u{fffc}🚀";
        let encoded = Line::new("x").string(text).0;
        assert_eq!(
            encoded,
            "x \"a\\\\b\\\"c\\nd\\re\\tf \u{a0}\u{2060}\u{fffc}🚀\""
        );
        let tokens = tokenize(&encoded).unwrap();
        match &tokens[1] {
            Token::Text(decoded) => assert_eq!(decoded, text),
            Token::Bare(_) => panic!("chaîne attendue"),
        }
        assert_eq!(Line::new("x").optional_string(None).0, "x -");
        assert_eq!(Line::new("x").optional(None).0, "x -");
        assert_eq!(Line::new("x").optional(Some("y")).0, "x y");
        assert!(matches!(
            unquote("ab\\"),
            Err(DecodeReason::Value("échappement"))
        ));
        assert_eq!(unquote("é\" reste"), Ok(("é".to_owned(), 3)));
    }

    // Entiers canoniques seulement ; empreinte et identifiants stricts.
    #[test]
    fn integers_and_identifiers_are_canonical() {
        let read = |line: &'static str| {
            let mut f = Fields {
                tokens: tokenize(line).unwrap().into_iter(),
            };
            f.integer()
        };
        assert_eq!(read("0"), Ok(0));
        assert_eq!(read("907"), Ok(907));
        for bad in ["00", "07", "007", "1a"] {
            assert_eq!(read(bad), Err(DecodeReason::Value("entier")), "{bad}");
        }
        let error = DecodeError {
            line: 3,
            reason: DecodeReason::Order,
        };
        assert_eq!(error.to_string(), "ligne 3 : ordre canonique non respecté");
        let header = DecodeError {
            line: 1,
            reason: DecodeReason::Header,
        };
        // Contrat modifié (incrément 5.1) : le message ne nomme plus l'en-tête, commun aux deux
        // versions du format.
        assert_eq!(
            header.to_string(),
            "ligne 1 : en-tête attendu absent ou d'une autre version"
        );
    }
}
