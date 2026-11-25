//! Modèle métier. Incrément 2.3a : contenu partagé entre contextes (D2) et révisions mineures
//! ou substantielles (D5). Incrément 2.3b : statuts séparés de décision, de vérification et
//! d'implémentation, propres à chaque contexte (D6, D11). Les réserves arrivent en 2.5.
//!
//! Représentation : le contenu d'une note est conservé une seule fois, dans `Snapshot::notes`.
//! Chaque contexte référence ce même contenu (`Arc`) et porte seulement la qualification de la
//! note (verdict, visibilité) et l'activation de ses relations. La validation vérifie que chaque
//! contexte référence bien le contenu canonique (INV-SCOPE-02).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::error::Error;
use crate::id::{AnchorId, EdgeId, NodeId, ScopeId, DEFAULT_SCOPE};

/// Rôle d'une note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Organise ; ne porte aucun verdict.
    Category,
    /// Hypothèses, invariants et arguments.
    Reasoning,
    /// Choix réalisable et ses points de code.
    Implementation,
    /// Réflexion autonome, sans échange obligatoire.
    PersonalNote,
    /// Couple question–réponse collé par l'utilisateur ; sources immuables (incrément 2.5).
    Exchange,
    /// Plage exacte d'une source d'échange ; immuable, qualifiable comme une affirmation.
    Passage,
    /// Avertissement ou invariant à préciser, créé depuis un passage ; cycle de vie propre.
    Reserve,
}

/// Statuts portés par un rôle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Statuses {
    pub(crate) verdict: bool,
    pub(crate) decision: bool,
    pub(crate) implementation: bool,
    pub(crate) reserve: bool,
}

impl Kind {
    /// Rôles qui affirment quelque chose : ils portent verdict et décision, et peuvent dépendre,
    /// remplacer, restreindre ou contredire.
    pub(crate) fn is_claim(self) -> bool {
        matches!(
            self,
            Self::Reasoning | Self::Implementation | Self::PersonalNote | Self::Passage
        )
    }

    /// Rôles créés par une commande dédiée, parce qu'ils portent une source ou un passage.
    pub(crate) fn has_body(self) -> bool {
        matches!(self, Self::Exchange | Self::Passage | Self::Reserve)
    }

    pub(crate) fn statuses(self) -> Statuses {
        let note = matches!(
            self,
            Self::Reasoning | Self::Implementation | Self::PersonalNote
        );
        Statuses {
            verdict: self.is_claim(),
            decision: self.is_claim(),
            implementation: note,
            reserve: self == Self::Reserve,
        }
    }
}

/// État de vérification d'une note qui n'est pas une catégorie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Pas encore vérifiée.
    Unverified,
    /// Vérifiée dans le contexte considéré.
    Verified,
    /// Réfutée explicitement.
    Refuted,
}

/// État de décision d'une note qui n'est pas une catégorie (D6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Proposée : état à la création ; fait partie des ressources actuelles (D11).
    Proposed,
    /// Retenue pour la variante considérée.
    Retained,
    /// Plus retenue : réversible, conservée, retirée des ressources actuelles ; ses dépendants
    /// sont à réexaminer. Remplace toute suppression.
    NoLongerRetained,
    /// Remplacée par une autre note ; posée par la relation « remplace » (incrément 2.4).
    Replaced,
}

/// État d'implémentation d'une note qui n'est pas une catégorie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Implementation {
    /// Aucun code associé.
    NotStarted,
    /// Implémentation en cours.
    InProgress,
    /// Implémentée.
    Implemented,
}

/// Présence dans les ressources actuelles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// Dans les ressources actuelles.
    Current,
    /// Retirée des ressources actuelles, toujours consultable.
    Archived,
}

/// Type d'une relation. Le sens de « dépend de » va du dépendant vers ce dont il dépend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relation {
    /// Classement dans une catégorie ; aucune propagation. Entre catégories, sans cycle.
    ClassifiedIn,
    /// Dépendance du raisonnement ; propage le réexamen.
    DependsOn,
    /// Provenance ; propage le réexamen.
    DevelopedFrom,
    /// Lien de navigation ; aucune propagation.
    Related,
    /// L'origine remplace la destination, qui passe « remplacée » ; sans cycle (INV-REL-02).
    /// Créée seulement par la commande dédiée, avec motif.
    Replaces,
    /// L'origine restreint le périmètre de la destination à une condition (INV-REL-06).
    /// Créée seulement par la commande dédiée, avec sa condition.
    RestrictsScope,
    /// L'origine contredit la destination, désaccord assumé et justifié ; aucune propagation
    /// (INV-REL-07). Créée seulement par la commande dédiée, avec sa justification.
    Contradicts,
    /// La réserve d'origine s'applique à la destination ; aucun changement de statut (INV-RES-03).
    AppliesTo,
    /// L'origine propose de traiter la réserve de destination, qui passe « traitée à vérifier ».
    /// Créée seulement par la commande dédiée, avec motif.
    Treats,
}

impl Relation {
    /// Vrai pour les relations qui propagent le réexamen et forment les appuis déclarés d'une
    /// note (orphelines, R6) : « dépend de » et « développé à partir de ».
    pub(crate) fn propagates(self) -> bool {
        matches!(self, Relation::DependsOn | Relation::DevelopedFrom)
    }

    /// Vrai pour les relations créées par Link et déplaçables par Reattach. Les autres ont une
    /// commande dédiée, parce qu'elles portent un statut ou un texte.
    pub(crate) fn is_plain(self) -> bool {
        matches!(
            self,
            Self::ClassifiedIn
                | Self::DependsOn
                | Self::DevelopedFrom
                | Self::Related
                | Self::AppliesTo
        )
    }
}

/// Empreinte de sens d'une note dans un contexte : révisions substantielles et restrictions de
/// périmètre actives. Toute variation par rapport à la valeur enregistrée à l'activation d'une
/// relation fait de la destination une cause de réexamen pour la source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    pub(crate) substantive: usize,
    pub(crate) restrictions: usize,
}

/// Nature d'une révision postérieure à la création, déclarée par l'utilisateur (D5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionKind {
    /// Correction sans changement de sens : aucune cause de réexamen.
    Minor,
    /// Changement de sens : cause de réexamen pour les dépendants.
    Substantive,
}

/// Partie d'un échange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// Question posée.
    Question,
    /// Réponse reçue.
    Answer,
}

/// Cycle de vie d'une réserve, propre à chaque contexte (INV-RES-02, INV-RES-05).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReserveStatus {
    /// Point ouvert.
    Open,
    /// Une note propose de la traiter ; la résolution reste à confirmer.
    TreatedPendingVerification,
    /// Résolue dans ce contexte, par une commande explicite.
    Resolved,
}

/// Données propres à un rôle, partagées entre contextes comme le reste du contenu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Body {
    /// Note ordinaire ou catégorie.
    Note,
    /// Sources d'un échange, conservées octet pour octet ; question absente : échange incomplet.
    Exchange {
        question: Option<Arc<str>>,
        answer: Arc<str>,
        /// Échange qui précède dans la conversation, donné explicitement à la saisie.
        previous: Option<NodeId>,
    },
    /// Plage d'octets UTF-8 d'une partie d'un échange, fin exclusive.
    Passage {
        exchange: NodeId,
        part: Part,
        start: usize,
        end: usize,
    },
    /// Plage d'octets UTF-8 d'une version d'une note (0 : texte de création ; *k* : *k*-ième
    /// révision), fin exclusive (D18, CITATIONS.md, section 3).
    Citation {
        note: NodeId,
        version: usize,
        start: usize,
        end: usize,
    },
    /// Passage d'origine et cible prévue, en texte, quand elle n'existe pas encore.
    Reserve {
        passage: NodeId,
        target: Option<String>,
    },
}

/// Sources d'un échange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExchangeView<'a> {
    question: Option<&'a str>,
    answer: &'a str,
    previous: Option<NodeId>,
}

impl<'a> ExchangeView<'a> {
    /// Question, si elle a été fournie.
    #[must_use]
    pub fn question(&self) -> Option<&'a str> {
        self.question
    }

    /// Réponse, telle que collée.
    #[must_use]
    pub fn answer(&self) -> &'a str {
        self.answer
    }

    /// Faux tant que la question manque (INV-SOURCE-04).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.question.is_some()
    }

    /// Échange qui précède dans la conversation ; absent pour le premier.
    #[must_use]
    pub fn previous(&self) -> Option<NodeId> {
        self.previous
    }
}

/// Emplacement d'un passage dans son échange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassageView {
    /// Échange d'origine.
    pub exchange: NodeId,
    /// Partie de l'échange.
    pub part: Part,
    /// Début, en octets.
    pub start: usize,
    /// Fin exclusive, en octets.
    pub end: usize,
}

/// Source d'une citation (D18) : une partie d'un échange, ou une version d'une note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CitedSource {
    /// Partie d'un échange, immuable.
    Exchange {
        /// Échange cité.
        exchange: NodeId,
        /// Question ou réponse.
        part: Part,
    },
    /// Version d'une note : 0 pour le texte de création, *k* pour la *k*-ième révision.
    Note {
        /// Note citée.
        note: NodeId,
        /// Version citée.
        version: usize,
    },
}

/// Citation : sa source, sa plage, et si la note citée a été révisée depuis (INV-CIT-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CitationView {
    /// Source citée.
    pub source: CitedSource,
    /// Début, en octets.
    pub start: usize,
    /// Fin exclusive, en octets.
    pub end: usize,
    /// Vrai si la note citée a une révision plus récente que la version citée ; toujours faux
    /// pour un échange.
    pub source_revised: bool,
}

/// Origine et cible d'une réserve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReserveView<'a> {
    /// Passage d'où la réserve a été créée.
    pub passage: NodeId,
    /// Échange d'origine du passage.
    pub exchange: NodeId,
    /// Cible prévue, en texte, quand elle n'existe pas encore (INV-RES-04).
    pub target: Option<&'a str>,
}

/// Version du contenu d'une note, telle que conservée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    title: String,
    text: String,
    kind: Option<RevisionKind>,
}

impl Revision {
    /// Titre de cette version.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Contenu de cette version.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Nature de la révision ; absente pour la version de création.
    #[must_use]
    pub fn kind(&self) -> Option<RevisionKind> {
        self.kind
    }
}

/// Titre et contenu d'une version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Text {
    pub(crate) title: String,
    pub(crate) text: String,
}

/// Identité durable d'une note : rôle immuable et suite de versions (INV-MOD-01, INV-MOD-03).
/// La version de création est séparée des révisions : la suite n'est jamais vide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Content {
    pub(crate) kind: Kind,
    pub(crate) created: Text,
    pub(crate) revisions: Vec<(RevisionKind, Text)>,
    pub(crate) body: Body,
}

impl Content {
    pub(crate) fn new(kind: Kind, title: &str, text: &str) -> Self {
        Self::with_body(kind, title, text, Body::Note)
    }

    pub(crate) fn with_body(kind: Kind, title: &str, text: &str, body: Body) -> Self {
        Self {
            kind,
            created: Text {
                title: title.into(),
                text: text.into(),
            },
            revisions: Vec::new(),
            body,
        }
    }

    pub(crate) fn current(&self) -> &Text {
        self.revisions
            .last()
            .map_or(&self.created, |(_, text)| text)
    }

    /// Nombre de révisions substantielles : empreinte du sens de la note (D5).
    pub(crate) fn substantive(&self) -> usize {
        self.revisions
            .iter()
            .filter(|(kind, _)| *kind == RevisionKind::Substantive)
            .count()
    }

    pub(crate) fn history(&self) -> Vec<Revision> {
        let created = Revision {
            title: self.created.title.clone(),
            text: self.created.text.clone(),
            kind: None,
        };
        std::iter::once(created)
            .chain(self.revisions.iter().map(|(kind, t)| Revision {
                title: t.title.clone(),
                text: t.text.clone(),
                kind: Some(*kind),
            }))
            .collect()
    }
}

/// Note telle qu'elle apparaît dans un contexte : contenu partagé, qualification propre.
/// Une catégorie ne porte aucun statut ; toute autre note porte les trois.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub(crate) content: Arc<Content>,
    pub(crate) verdict: Option<Verdict>,
    pub(crate) decision: Option<Decision>,
    pub(crate) implementation: Option<Implementation>,
    pub(crate) reserve: Option<ReserveStatus>,
    pub(crate) visibility: Visibility,
}

impl Node {
    pub(crate) fn new(content: Arc<Content>) -> Self {
        let statuses = content.kind.statuses();
        Self {
            content,
            verdict: statuses.verdict.then_some(Verdict::Unverified),
            decision: statuses.decision.then_some(Decision::Proposed),
            implementation: statuses
                .implementation
                .then_some(Implementation::NotStarted),
            reserve: statuses.reserve.then_some(ReserveStatus::Open),
            visibility: Visibility::Current,
        }
    }

    /// Cycle de vie de la réserve dans ce contexte ; absent pour tout autre rôle.
    #[must_use]
    pub fn reserve_status(&self) -> Option<ReserveStatus> {
        self.reserve
    }

    /// Vrai pour une note écartée : réfutée, retirée des ressources actuelles, plus retenue ou
    /// remplacée. Une note écartée est cause de réexamen pour tous ses dépendants.
    #[must_use]
    pub fn withdrawn(&self) -> bool {
        self.verdict == Some(Verdict::Refuted)
            || self.visibility == Visibility::Archived
            || matches!(
                self.decision,
                Some(Decision::NoLongerRetained | Decision::Replaced)
            )
    }

    /// État de décision ; toujours absent pour une catégorie.
    #[must_use]
    pub fn decision(&self) -> Option<Decision> {
        self.decision
    }

    /// État d'implémentation ; toujours absent pour une catégorie.
    #[must_use]
    pub fn implementation(&self) -> Option<Implementation> {
        self.implementation
    }

    /// Rôle de la note, immuable.
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.content.kind
    }

    /// Titre courant.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.content.current().title
    }

    /// Contenu courant, identique dans tous les contextes.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.content.current().text
    }

    /// Nombre de versions conservées, création comprise.
    #[must_use]
    pub fn revision_count(&self) -> usize {
        self.content.revisions.len() + 1
    }

    /// Verdict ; toujours absent pour une catégorie.
    #[must_use]
    pub fn verdict(&self) -> Option<Verdict> {
        self.verdict
    }

    /// Présence dans les ressources actuelles.
    #[must_use]
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }
}

/// Relation entre deux notes d'un même contexte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub(crate) from: NodeId,
    pub(crate) to: NodeId,
    pub(crate) relation: Relation,
    pub(crate) active: bool,
    /// Empreinte de sens de la destination au moment de l'activation.
    pub(crate) baseline: Fingerprint,
    /// Condition d'une restriction de périmètre, ou justification d'une contradiction.
    pub(crate) annotation: Option<String>,
    /// État de la destination avant un remplacement ou un traitement, rétabli au retrait.
    pub(crate) prior: Option<Prior>,
}

/// État posé par une relation sur sa destination, mémorisé pour pouvoir le rétablir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Prior {
    Decision(Decision),
    Reserve(ReserveStatus),
}

/// État d'une cause de réexamen tel qu'acquitté : empreinte de sens et statuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CauseState {
    pub(crate) fingerprint: Fingerprint,
    pub(crate) verdict: Option<Verdict>,
    pub(crate) decision: Option<Decision>,
    pub(crate) visibility: Visibility,
}

/// Acquittement d'une cause par un dépendant (INV-ACK-01) : valable tant que le sens du
/// dépendant et l'état de la cause sont ceux qui ont été confirmés.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ack {
    pub(crate) dependent: usize,
    pub(crate) cause: CauseState,
}

impl Edge {
    /// Origine.
    #[must_use]
    pub fn from(&self) -> NodeId {
        self.from
    }

    /// Destination.
    #[must_use]
    pub fn to(&self) -> NodeId {
        self.to
    }

    /// Type.
    #[must_use]
    pub fn relation(&self) -> Relation {
        self.relation
    }

    /// Faux pour une relation conservée dans l'historique mais désactivée.
    #[must_use]
    pub fn active(&self) -> bool {
        self.active
    }

    /// Condition d'une restriction de périmètre ou justification d'une contradiction.
    #[must_use]
    pub fn annotation(&self) -> Option<&str> {
        self.annotation.as_deref()
    }
}

/// Réseau d'un contexte.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    pub(crate) nodes: BTreeMap<NodeId, Node>,
    pub(crate) edges: BTreeMap<EdgeId, Edge>,
    /// Acquittements de ce contexte : (dépendant, cause) -> état confirmé.
    pub(crate) acks: BTreeMap<(NodeId, NodeId), Ack>,
}

impl Graph {
    /// Note de ce contexte.
    ///
    /// # Errors
    /// [`Error::MissingNode`] si la note n'existe pas dans ce contexte.
    pub fn node(&self, id: NodeId) -> Result<&Node, Error> {
        self.nodes.get(&id).ok_or(Error::MissingNode(id))
    }

    pub(crate) fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, Error> {
        self.nodes.get_mut(&id).ok_or(Error::MissingNode(id))
    }

    /// Relation de ce contexte.
    ///
    /// # Errors
    /// [`Error::MissingEdge`] si la relation n'existe pas dans ce contexte.
    pub fn edge(&self, id: EdgeId) -> Result<&Edge, Error> {
        self.edges.get(&id).ok_or(Error::MissingEdge(id))
    }

    /// Notes, par identifiant croissant.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes.iter().map(|(id, n)| (*id, n))
    }

    /// Relations, actives ou non, par identifiant croissant.
    pub fn edges(&self) -> impl Iterator<Item = (EdgeId, &Edge)> {
        self.edges.iter().map(|(id, e)| (*id, e))
    }

    pub(crate) fn validate_relation(
        &self,
        from: NodeId,
        to: NodeId,
        kind: Relation,
    ) -> Result<(), Error> {
        let a = self.node(from)?.kind();
        let b = self.node(to)?.kind();
        if from == to {
            return Err(Error::SelfRelation);
        }
        let allowed = match kind {
            Relation::ClassifiedIn => b == Kind::Category,
            Relation::Related => true,
            // D18 : une affirmation peut dépendre d'un échange entier, ou s'en être développée ;
            // un échange peut s'être développé d'un autre.
            Relation::DependsOn => a.is_claim() && (b.is_claim() || b == Kind::Exchange),
            Relation::DevelopedFrom => {
                (a.is_claim() && (b.is_claim() || b == Kind::Exchange))
                    || (a == Kind::Exchange && b == Kind::Exchange)
            }
            Relation::Replaces | Relation::RestrictsScope | Relation::Contradicts => {
                a.is_claim() && b.is_claim()
            }
            Relation::AppliesTo => a == Kind::Reserve && b.is_claim(),
            Relation::Treats => a.is_claim() && b == Kind::Reserve,
        };
        if allowed {
            Ok(())
        } else {
            Err(Error::InvalidRelation)
        }
    }

    /// Crée une relation active après vérification des règles de rôle ; son empreinte de
    /// référence est celle de la destination à cet instant. Les règles globales (doublons,
    /// cycles, statut « remplacée ») sont vérifiées sur l'état candidat complet.
    /// Retire une relation active (`retirer`, et désactivation d'une fusion) : elle reste, inactive ;
    /// un remplacement rend à sa destination sa décision d'avant ; le dernier traitement actif
    /// d'une réserve encore « traitée à vérifier » lui rend son état d'avant.
    pub(crate) fn detach(&mut self, edge: EdgeId) -> Result<(), Error> {
        let old = self.edge(edge)?.clone();
        if !old.active {
            return Err(Error::InactiveEdge);
        }
        self.edges
            .get_mut(&edge)
            .ok_or(Error::MissingEdge(edge))?
            .active = false;
        let still = self
            .edges
            .values()
            .any(|e| e.active && e.relation == old.relation && e.to == old.to);
        let target = self.node_mut(old.to)?;
        match (old.relation, old.prior) {
            // Une note n'a qu'un remplacement actif à la fois : le retirer lui rend sa décision
            // d'avant.
            (Relation::Replaces, Some(Prior::Decision(prior))) => {
                target.decision = Some(prior);
            }
            // Plus aucun traitement actif, réserve encore « traitée à vérifier » : elle retrouve
            // son état d'avant. Une résolution confirmée depuis n'est pas défaite.
            (Relation::Treats, Some(Prior::Reserve(prior)))
                if !still && target.reserve == Some(ReserveStatus::TreatedPendingVerification) =>
            {
                target.reserve = Some(prior);
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn insert_edge(
        &mut self,
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        relation: Relation,
        annotation: Option<String>,
        prior: Option<Prior>,
    ) -> Result<(), Error> {
        self.validate_relation(from, to, relation)?;
        let baseline = self.fingerprint(to)?;
        // Construction explicite : aucun champ ajouté plus tard n'est recopié en silence.
        self.edges.insert(
            id,
            Edge {
                from,
                to,
                relation,
                active: true,
                baseline,
                annotation,
                prior,
            },
        );
        Ok(())
    }

    /// État courant d'une note en tant que cause possible de réexamen.
    pub(crate) fn cause_state(&self, id: NodeId) -> Result<CauseState, Error> {
        let node = self.node(id)?;
        Ok(CauseState {
            fingerprint: self.fingerprint(id)?,
            verdict: node.verdict,
            decision: node.decision,
            visibility: node.visibility,
        })
    }

    /// Vrai si le dépendant a confirmé cette cause dans son état courant et dans le sien.
    fn acknowledged(&self, dependent: NodeId, cause: NodeId, state: CauseState) -> bool {
        let meaning = self.nodes.get(&dependent).map(|n| n.content.substantive());
        self.acks
            .get(&(dependent, cause))
            .is_some_and(|ack| Some(ack.dependent) == meaning && ack.cause == state)
    }

    /// Empreinte de sens d'une note de ce contexte.
    pub(crate) fn fingerprint(&self, id: NodeId) -> Result<Fingerprint, Error> {
        let substantive = self.node(id)?.content.substantive();
        let restrictions = self
            .edges
            .values()
            .filter(|e| e.active && e.relation == Relation::RestrictsScope && e.to == id)
            .count();
        Ok(Fingerprint {
            substantive,
            restrictions,
        })
    }

    /// Réserves de ce contexte non résolues et non retirées : liste des points ouverts.
    #[must_use]
    pub fn open_reserves(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, n)| {
                n.visibility == Visibility::Current
                    && matches!(
                        n.reserve,
                        Some(ReserveStatus::Open | ReserveStatus::TreatedPendingVerification)
                    )
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Notes qui remplacent celle-ci par une relation active (INV-MOD-06).
    #[must_use]
    pub fn replaced_by(&self, id: NodeId) -> Vec<NodeId> {
        self.edges
            .values()
            .filter(|e| e.active && e.relation == Relation::Replaces && e.to == id)
            .map(|e| e.from)
            .collect()
    }

    /// Vrai si les relations actives du type donné forment un cycle (tri topologique).
    fn has_cycle(&self, relation: Relation) -> bool {
        let edges: Vec<(NodeId, NodeId)> = self
            .edges
            .values()
            .filter(|e| e.active && e.relation == relation)
            .map(|e| (e.from, e.to))
            .collect();
        let mut indegree: BTreeMap<NodeId, usize> = BTreeMap::new();
        for &(from, to) in &edges {
            indegree.entry(from).or_default();
            *indegree.entry(to).or_default() += 1;
        }
        let mut ready: Vec<NodeId> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut removed = 0;
        while let Some(node) = ready.pop() {
            removed += 1;
            for &(_, to) in edges.iter().filter(|(from, _)| *from == node) {
                if let Some(degree) = indegree.get_mut(&to) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push(to);
                    }
                }
            }
        }
        removed < indegree.len()
    }

    /// Vérifie toutes les règles du contexte, relations inactives comprises.
    ///
    /// # Errors
    /// La première règle violée.
    pub fn validate(&self) -> Result<(), Error> {
        for (&id, node) in &self.nodes {
            let expected = node.kind().statuses();
            let present = Statuses {
                verdict: node.verdict.is_some(),
                decision: node.decision.is_some(),
                implementation: node.implementation.is_some(),
                reserve: node.reserve.is_some(),
            };
            let pairs = [
                (expected.verdict, present.verdict),
                (expected.decision, present.decision),
                (expected.implementation, present.implementation),
                (expected.reserve, present.reserve),
            ];
            if pairs.contains(&(false, true)) {
                return Err(Error::CategoryHasNoVerdict);
            }
            if pairs.contains(&(true, false)) {
                return Err(Error::MissingVerdict(id));
            }
        }
        for e in self.edges.values() {
            self.validate_relation(e.from, e.to, e.relation)?;
        }
        // Un acquittement relie deux notes du contexte.
        for &(dependent, cause) in self.acks.keys() {
            self.node(dependent)?;
            self.node(cause)?;
        }
        // INV-REL-01 : aucun doublon actif.
        let mut seen = BTreeSet::new();
        for e in self.edges.values().filter(|e| e.active) {
            if !seen.insert((e.from, e.to, e.relation)) {
                return Err(Error::DuplicateRelation);
            }
        }
        // INV-REL-02 et INV-REL-03 : ni cycle de remplacement, ni cycle de catégories.
        if self.has_cycle(Relation::Replaces) || self.has_cycle(Relation::ClassifiedIn) {
            return Err(Error::CyclicRelation);
        }
        // Le statut « remplacée » et la relation « remplace » vont ensemble : un remplacement
        // actif impose le statut ; le statut suppose une relation, même désactivée par une
        // restauration.
        for (&id, node) in &self.nodes {
            let replaced = node.decision == Some(Decision::Replaced);
            let replacers = self
                .edges
                .values()
                .filter(|e| e.relation == Relation::Replaces && e.to == id);
            let (mut any, mut active) = (false, false);
            for e in replacers {
                any = true;
                active |= e.active;
            }
            if (active && !replaced) || (replaced && !any) {
                return Err(Error::ReservedStatus);
            }
        }
        Ok(())
    }

    /// Causes calculées : dépendant -> ensemble des racines à réexaminer.
    ///
    /// Seules les relations actives « dépend de » et « développé à partir de » propagent. Une
    /// note écartée est cause pour tous ses dépendants ; une note dont l'empreinte de sens a
    /// changé (révision substantielle, restriction de périmètre) l'est pour ceux dont la
    /// relation a été activée avant ce changement. La cause se transmet aux dépendants indirects.
    #[must_use]
    pub fn review_causes(&self) -> BTreeMap<NodeId, BTreeSet<NodeId>> {
        let propagating: Vec<&Edge> = self
            .edges
            .values()
            .filter(|e| e.active && e.relation.propagates())
            .collect();
        let mut result: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for (&root, node) in &self.nodes {
            if node.kind() == Kind::Category {
                continue;
            }
            let withdrawn = node.withdrawn();
            // La racine existe : son état est toujours calculable.
            let Ok(state) = self.cause_state(root) else {
                continue;
            };
            let meaning = state.fingerprint;
            let mut seen = BTreeSet::from([root]);
            let mut pending = Vec::new();
            for edge in propagating
                .iter()
                .filter(|e| e.to == root && (withdrawn || e.baseline != meaning))
            {
                // Un dépendant qui a acquitté cette cause n'est pas à réexaminer et ne la
                // transmet pas (INV-ACK-01). Aucune auto-relation n'existe (INV-REL-01).
                if self.acknowledged(edge.from, root, state) {
                    continue;
                }
                result.entry(edge.from).or_default().insert(root);
                if seen.insert(edge.from) {
                    pending.push(edge.from);
                }
            }
            while let Some(parent) = pending.pop() {
                for edge in propagating.iter().filter(|e| e.to == parent) {
                    let child = edge.from;
                    if child == root || self.acknowledged(child, root, state) {
                        continue;
                    }
                    result.entry(child).or_default().insert(root);
                    if seen.insert(child) {
                        pending.push(child);
                    }
                }
            }
        }
        result
    }

    /// Ressources actuelles (D11) : notes proposées ou retenues, non écartées et sans cause de
    /// réexamen. Une catégorie courante, qui ne porte aucun statut, en fait partie.
    #[must_use]
    pub fn current_resources(&self) -> Vec<NodeId> {
        let pending = self.review_causes();
        let orphans = self.orphans_given(&pending);
        self.nodes
            .iter()
            .filter(|(id, n)| !n.withdrawn() && !pending.contains_key(id) && !orphans.contains(id))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Notes orphelines (R6, D24 ; INV-ORPH-01), par identifiant croissant : au moins un appui
    /// déclaré (« dépend de », « développé à partir de », actif), tous écartés, aucune cause de
    /// réexamen en cours, et la note elle-même non écartée. Elles ne sont pas ressources actuelles.
    #[must_use]
    pub fn orphans(&self) -> Vec<NodeId> {
        self.orphans_given(&self.review_causes())
    }

    fn orphans_given(&self, pending: &BTreeMap<NodeId, BTreeSet<NodeId>>) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(id, node)| {
                let mut supports = self
                    .edges
                    .values()
                    .filter(|e| e.active && e.from == **id && e.relation.propagates());
                let first = supports.next();
                !node.withdrawn()
                    && !pending.contains_key(id)
                    && first.is_some()
                    && first
                        .into_iter()
                        .chain(supports)
                        .all(|e| self.nodes.get(&e.to).is_some_and(Node::withdrawn))
            })
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Empreinte FNV-1a sur 64 bits d'un texte. Elle détecte une modification accidentelle du code
/// ancré ; elle n'est pas cryptographique et ne prouve rien contre une modification délibérée.
#[must_use]
pub fn text_digest(text: &str) -> u64 {
    bytes_digest(text.as_bytes())
}

/// Empreinte d'une suite d'octets, la même que [`text_digest`] pour un texte.
#[must_use]
pub fn bytes_digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Ancrage d'une note dans le code (INV-CODE-03), partagé entre contextes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub(crate) node: NodeId,
    pub(crate) path: String,
    pub(crate) excerpt: String,
    pub(crate) digest: u64,
    pub(crate) git: Option<String>,
    pub(crate) active: bool,
}

impl Anchor {
    /// Note ancrée.
    #[must_use]
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// Chemin connu au moment du lien ; il peut être périmé (INV-CODE-05).
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Copie du texte ciblé au moment du lien.
    #[must_use]
    pub fn excerpt(&self) -> &str {
        &self.excerpt
    }

    /// Empreinte de la copie.
    #[must_use]
    pub fn digest(&self) -> u64 {
        self.digest
    }

    /// Commit du code au moment du lien, s'il était versionné.
    #[must_use]
    pub fn git(&self) -> Option<&str> {
        self.git.as_deref()
    }

    /// Faux pour un ancrage retiré, conservé dans l'historique.
    #[must_use]
    pub fn active(&self) -> bool {
        self.active
    }
}

/// État immuable du réseau : contenu canonique des notes et réseau de chaque contexte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub(crate) notes: BTreeMap<NodeId, Arc<Content>>,
    pub(crate) scopes: BTreeMap<ScopeId, Graph>,
    pub(crate) anchors: BTreeMap<AnchorId, Anchor>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            notes: BTreeMap::new(),
            scopes: BTreeMap::from([(DEFAULT_SCOPE, Graph::default())]),
            anchors: BTreeMap::new(),
        }
    }
}

impl Snapshot {
    /// Réseau d'un contexte.
    ///
    /// # Errors
    /// [`Error::MissingScope`] si le contexte n'existe pas dans cet état.
    pub fn graph(&self, id: ScopeId) -> Result<&Graph, Error> {
        self.scopes.get(&id).ok_or(Error::MissingScope(id))
    }

    pub(crate) fn graph_mut(&mut self, id: ScopeId) -> Result<&mut Graph, Error> {
        self.scopes.get_mut(&id).ok_or(Error::MissingScope(id))
    }

    /// Ajoute une nouvelle note au contexte et au contenu canonique.
    pub(crate) fn insert_note(
        &mut self,
        scope: ScopeId,
        id: NodeId,
        content: Content,
    ) -> Result<(), Error> {
        let content = Arc::new(content);
        self.graph_mut(scope)?
            .nodes
            .insert(id, Node::new(Arc::clone(&content)));
        self.notes.insert(id, content);
        Ok(())
    }

    pub(crate) fn content(&self, id: NodeId) -> Result<&Arc<Content>, Error> {
        self.notes.get(&id).ok_or(Error::MissingNode(id))
    }

    /// Versions conservées d'une note, de la création à la version courante (INV-MOD-01).
    ///
    /// # Errors
    /// [`Error::MissingNode`] si la note n'existe dans aucun contexte de cet état.
    pub fn revisions(&self, id: NodeId) -> Result<Vec<Revision>, Error> {
        self.content(id).map(|c| c.history())
    }

    /// Remplace le contenu d'une note partout où elle apparaît (D2).
    pub(crate) fn set_content(&mut self, id: NodeId, content: Arc<Content>) {
        for graph in self.scopes.values_mut() {
            if let Some(node) = graph.nodes.get_mut(&id) {
                node.content = Arc::clone(&content);
            }
        }
        self.notes.insert(id, content);
    }

    /// Sources d'un échange.
    ///
    /// # Errors
    /// [`Error::MissingNode`] si l'objet n'existe pas, [`Error::WrongKind`] s'il n'est pas un échange.
    pub fn exchange(&self, id: NodeId) -> Result<ExchangeView<'_>, Error> {
        match &self.content(id)?.body {
            Body::Exchange {
                question,
                answer,
                previous,
            } => Ok(ExchangeView {
                question: question.as_deref(),
                answer,
                previous: *previous,
            }),
            Body::Note | Body::Passage { .. } | Body::Citation { .. } | Body::Reserve { .. } => {
                Err(Error::WrongKind(id))
            }
        }
    }

    /// Citation d'un échange ou d'une note : source, plage, source révisée (INV-CIT-01).
    ///
    /// # Errors
    /// [`Error::MissingNode`] si l'objet n'existe pas, [`Error::WrongKind`] s'il n'est pas une
    /// citation.
    pub fn citation(&self, id: NodeId) -> Result<CitationView, Error> {
        match self.content(id)?.body {
            Body::Passage {
                exchange,
                part,
                start,
                end,
            } => Ok(CitationView {
                source: CitedSource::Exchange { exchange, part },
                start,
                end,
                source_revised: false,
            }),
            Body::Citation {
                note,
                version,
                start,
                end,
            } => Ok(CitationView {
                source: CitedSource::Note { note, version },
                start,
                end,
                source_revised: self.content(note)?.revisions.len() > version,
            }),
            Body::Note | Body::Exchange { .. } | Body::Reserve { .. } => Err(Error::WrongKind(id)),
        }
    }

    /// Texte exact d'une plage d'une version d'une note citable : raisonnement, implémentation
    /// ou note personnelle (D24).
    pub(crate) fn note_excerpt(
        &self,
        note: NodeId,
        version: usize,
        start: usize,
        end: usize,
    ) -> Result<&str, Error> {
        let content = self.content(note)?;
        if !matches!(
            content.kind,
            Kind::Reasoning | Kind::Implementation | Kind::PersonalNote
        ) {
            return Err(Error::WrongKind(note));
        }
        let text = match version {
            0 => Some(&content.created.text),
            k => content.revisions.get(k - 1).map(|(_, t)| &t.text),
        };
        text.filter(|_| start < end)
            .and_then(|text| text.get(start..end))
            .ok_or(Error::InvalidExcerpt)
    }

    /// Emplacement d'un passage.
    ///
    /// # Errors
    /// [`Error::MissingNode`] si l'objet n'existe pas, [`Error::WrongKind`] s'il n'est pas un passage.
    pub fn passage(&self, id: NodeId) -> Result<PassageView, Error> {
        match self.content(id)?.body {
            Body::Passage {
                exchange,
                part,
                start,
                end,
            } => Ok(PassageView {
                exchange,
                part,
                start,
                end,
            }),
            Body::Note | Body::Exchange { .. } | Body::Citation { .. } | Body::Reserve { .. } => {
                Err(Error::WrongKind(id))
            }
        }
    }

    /// Conversation menant à cet échange, du premier échange jusqu'à lui, dans l'ordre de
    /// saisie (CA-01). Une conversation peut se ramifier : chaque échange n'a qu'un prédécesseur.
    ///
    /// # Errors
    /// [`Error::MissingNode`] ou [`Error::WrongKind`] si un maillon n'est pas un échange.
    pub fn conversation(&self, id: NodeId) -> Result<Vec<NodeId>, Error> {
        let mut chain = vec![id];
        let mut seen = BTreeSet::from([id]);
        let mut current = self.exchange(id)?.previous;
        // Chaque prédécesseur existait avant son successeur : la chaîne est finie. Un maillon déjà
        // rencontré signale un état corrompu.
        while let Some(previous) = current {
            if !seen.insert(previous) {
                return Err(Error::CyclicRelation);
            }
            chain.push(previous);
            current = self.exchange(previous)?.previous;
        }
        chain.reverse();
        Ok(chain)
    }

    /// Origine et cible d'une réserve.
    ///
    /// # Errors
    /// [`Error::MissingNode`] si l'objet n'existe pas, [`Error::WrongKind`] s'il n'est pas une
    /// réserve.
    pub fn reserve(&self, id: NodeId) -> Result<ReserveView<'_>, Error> {
        match &self.content(id)?.body {
            Body::Reserve { passage, target } => Ok(ReserveView {
                passage: *passage,
                exchange: self.passage(*passage)?.exchange,
                target: target.as_deref(),
            }),
            Body::Note | Body::Exchange { .. } | Body::Passage { .. } | Body::Citation { .. } => {
                Err(Error::WrongKind(id))
            }
        }
    }

    /// Texte exact d'une plage d'une partie d'échange (INV-SOURCE-01).
    pub(crate) fn excerpt(
        &self,
        exchange: NodeId,
        part: Part,
        start: usize,
        end: usize,
    ) -> Result<&str, Error> {
        let sources = self.exchange(exchange)?;
        let source = match part {
            Part::Question => sources.question,
            Part::Answer => Some(sources.answer),
        };
        source
            .filter(|_| start < end)
            .and_then(|text| text.get(start..end))
            .ok_or(Error::InvalidExcerpt)
    }

    /// Cohérence entre rôle et données propres : passage exact, réserve issue d'un passage.
    fn validate_body(&self, id: NodeId, content: &Content) -> Result<(), Error> {
        match (&content.body, content.kind) {
            (Body::Note, kind) if !kind.has_body() => Ok(()),
            (Body::Exchange { previous, .. }, Kind::Exchange) => match previous {
                Some(previous) => self.exchange(*previous).map(drop),
                None => Ok(()),
            },
            (
                Body::Passage {
                    exchange,
                    part,
                    start,
                    end,
                },
                Kind::Passage,
            ) => {
                if self.excerpt(*exchange, *part, *start, *end)? == content.created.text {
                    Ok(())
                } else {
                    Err(Error::InvalidExcerpt)
                }
            }
            (
                Body::Citation {
                    note,
                    version,
                    start,
                    end,
                },
                Kind::Passage,
            ) => {
                if self.note_excerpt(*note, *version, *start, *end)? == content.created.text {
                    Ok(())
                } else {
                    Err(Error::InvalidExcerpt)
                }
            }
            (Body::Reserve { passage, .. }, Kind::Reserve) => self.passage(*passage).map(drop),
            _ => Err(Error::WrongKind(id)),
        }
    }

    /// Ancrages d'une note, actifs ou retirés, par identifiant croissant.
    #[must_use]
    pub fn anchors_of(&self, node: NodeId) -> Vec<(AnchorId, &Anchor)> {
        self.anchors
            .iter()
            .filter(|(_, a)| a.node == node)
            .map(|(id, a)| (*id, a))
            .collect()
    }

    /// Ancrage du dépôt.
    ///
    /// # Errors
    /// [`Error::MissingAnchor`] si l'ancrage est inconnu de cet état.
    pub fn anchor(&self, id: AnchorId) -> Result<&Anchor, Error> {
        self.anchors.get(&id).ok_or(Error::MissingAnchor(id))
    }

    /// Vérifie chaque contexte et l'unicité du contenu de chaque note (INV-SCOPE-02).
    ///
    /// # Errors
    /// La première règle violée.
    pub fn validate(&self) -> Result<(), Error> {
        // Le contexte par défaut existe dès la création du dépôt et n'est jamais retiré.
        self.graph(DEFAULT_SCOPE)?;
        for anchor in self.anchors.values() {
            self.content(anchor.node)?;
            let empty = anchor.path.trim().is_empty()
                || anchor.excerpt.trim().is_empty()
                || anchor.git.as_deref().is_some_and(|g| g.trim().is_empty());
            if empty || anchor.digest != text_digest(&anchor.excerpt) {
                return Err(Error::InvalidAnchor);
            }
        }
        for (&id, content) in &self.notes {
            self.validate_body(id, content)?;
        }
        for graph in self.scopes.values() {
            for (&id, node) in &graph.nodes {
                let canonical = self.content(id)?;
                if !(Arc::ptr_eq(canonical, &node.content) || **canonical == *node.content) {
                    return Err(Error::DivergentContent(id));
                }
            }
            graph.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> NodeId {
        NodeId::from_bytes([n; 16])
    }

    fn edge(from: u8, to: u8, relation: Relation, active: bool) -> Edge {
        Edge {
            from: id(from),
            to: id(to),
            relation,
            active,
            baseline: Fingerprint::default(),
            annotation: None,
            prior: None,
        }
    }

    fn node(kind: Kind) -> Node {
        Node::new(Arc::new(Content::new(kind, "titre", "texte")))
    }

    /// Graphe valide : 1 catégorie, 2 raisonnement, 3 implémentation.
    fn valid_graph() -> Graph {
        Graph {
            nodes: BTreeMap::from([
                (id(1), node(Kind::Category)),
                (id(2), node(Kind::Reasoning)),
                (id(3), node(Kind::Implementation)),
            ]),
            edges: BTreeMap::from([
                (
                    EdgeId::from_bytes([1; 16]),
                    edge(3, 1, Relation::ClassifiedIn, true),
                ),
                (
                    EdgeId::from_bytes([2; 16]),
                    edge(3, 2, Relation::DependsOn, true),
                ),
                (
                    EdgeId::from_bytes([3; 16]),
                    edge(3, 2, Relation::DevelopedFrom, false),
                ),
                (
                    EdgeId::from_bytes([4; 16]),
                    edge(2, 3, Relation::Related, true),
                ),
            ]),
            acks: BTreeMap::new(),
        }
    }

    // Portés de memoire-core (tests internes de la phase 0), sur les mêmes propriétés.
    #[test]
    fn validate_accepts_a_consistent_graph() {
        assert_eq!(valid_graph().validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_a_category_with_a_verdict() {
        let mut g = valid_graph();
        g.nodes.get_mut(&id(1)).unwrap().verdict = Some(Verdict::Unverified);
        assert_eq!(g.validate(), Err(Error::CategoryHasNoVerdict));
    }

    // Le diagnostic nomme désormais la note fautive (MissingVerdict), défaut documenté en phase 0.
    #[test]
    fn validate_rejects_a_non_category_without_verdict() {
        let mut g = valid_graph();
        g.nodes.get_mut(&id(2)).unwrap().verdict = None;
        assert_eq!(g.validate(), Err(Error::MissingVerdict(id(2))));
    }

    // Chaque dimension de statut est exigée hors catégorie et interdite sur une catégorie.
    #[test]
    fn validate_checks_each_status_dimension() {
        let mut g = valid_graph();
        g.nodes.get_mut(&id(2)).unwrap().decision = None;
        assert_eq!(g.validate(), Err(Error::MissingVerdict(id(2))));
        let mut g = valid_graph();
        g.nodes.get_mut(&id(2)).unwrap().implementation = None;
        assert_eq!(g.validate(), Err(Error::MissingVerdict(id(2))));
        let mut g = valid_graph();
        g.nodes.get_mut(&id(1)).unwrap().decision = Some(Decision::Proposed);
        assert_eq!(g.validate(), Err(Error::CategoryHasNoVerdict));
        let mut g = valid_graph();
        g.nodes.get_mut(&id(1)).unwrap().implementation = Some(Implementation::NotStarted);
        assert_eq!(g.validate(), Err(Error::CategoryHasNoVerdict));
    }

    // Toute forme d'écart rend la note cause de réexamen et l'exclut des ressources actuelles,
    // y compris « remplacée », que seule la relation « remplace » posera (incrément 2.4).
    #[test]
    fn every_withdrawal_triggers_review_and_leaves_current_resources() {
        type Withdraw = fn(&mut Node);
        let withdrawals: [Withdraw; 4] = [
            |n| n.verdict = Some(Verdict::Refuted),
            |n| n.visibility = Visibility::Archived,
            |n| n.decision = Some(Decision::NoLongerRetained),
            |n| n.decision = Some(Decision::Replaced),
        ];
        for (i, withdraw) in withdrawals.into_iter().enumerate() {
            let mut g = valid_graph();
            withdraw(g.nodes.get_mut(&id(2)).unwrap());
            assert_eq!(
                g.review_causes(),
                BTreeMap::from([(id(3), BTreeSet::from([id(2)]))]),
                "écart {i}"
            );
            assert_eq!(g.current_resources(), vec![id(1)], "écart {i}");
        }
        let mut g = valid_graph();
        for decision in [Decision::Proposed, Decision::Retained] {
            g.nodes.get_mut(&id(2)).unwrap().decision = Some(decision);
            g.nodes.get_mut(&id(2)).unwrap().verdict = Some(Verdict::Verified);
            assert!(g.review_causes().is_empty());
            assert_eq!(g.current_resources(), vec![id(1), id(2), id(3)]);
        }
    }

    #[test]
    fn validate_rejects_an_edge_with_a_missing_endpoint() {
        for (from, to, missing) in [(9, 2, 9), (3, 9, 9)] {
            let mut g = valid_graph();
            g.edges.insert(
                EdgeId::from_bytes([9; 16]),
                edge(from, to, Relation::Related, true),
            );
            assert_eq!(g.validate(), Err(Error::MissingNode(id(missing))));
        }
    }

    #[test]
    fn validate_rejects_an_inactive_edge_breaking_relation_rules() {
        let mut g = valid_graph();
        g.edges.insert(
            EdgeId::from_bytes([9; 16]),
            edge(3, 1, Relation::DependsOn, false),
        );
        assert_eq!(g.validate(), Err(Error::InvalidRelation));
    }

    #[test]
    fn validate_relation_rules_one_by_one() {
        let g = valid_graph();
        let (cat, reasoning, implementation) = (id(1), id(2), id(3));
        let mut refused = vec![
            (implementation, reasoning, Relation::ClassifiedIn),
            (implementation, cat, Relation::DependsOn),
            (cat, implementation, Relation::DependsOn),
            (implementation, cat, Relation::DevelopedFrom),
            (cat, implementation, Relation::DevelopedFrom),
        ];
        for relation in [
            Relation::Replaces,
            Relation::RestrictsScope,
            Relation::Contradicts,
        ] {
            refused.push((implementation, cat, relation));
            refused.push((cat, implementation, relation));
        }
        for (from, to, relation) in refused {
            assert_eq!(
                g.validate_relation(from, to, relation),
                Err(Error::InvalidRelation),
                "{from} -> {to} : {relation:?}"
            );
        }
        let accepted = [
            (implementation, cat, Relation::ClassifiedIn),
            (implementation, reasoning, Relation::DependsOn),
            (implementation, reasoning, Relation::DevelopedFrom),
            (cat, implementation, Relation::Related),
            (implementation, reasoning, Relation::Replaces),
            (implementation, reasoning, Relation::RestrictsScope),
            (implementation, reasoning, Relation::Contradicts),
        ];
        for (from, to, relation) in accepted {
            assert_eq!(g.validate_relation(from, to, relation), Ok(()));
        }
        assert_eq!(
            g.validate_relation(reasoning, reasoning, Relation::Related),
            Err(Error::SelfRelation)
        );
        assert!(Relation::DependsOn.is_plain() && !Relation::Replaces.is_plain());
        assert!(!Relation::RestrictsScope.is_plain() && !Relation::Contradicts.is_plain());
    }

    fn with_edges(edges: &[(u8, u8, Relation, bool)]) -> Graph {
        let mut g = Graph::default();
        for n in 1..=5 {
            g.nodes.insert(id(n), node(Kind::Reasoning));
        }
        for (i, &(from, to, relation, active)) in edges.iter().enumerate() {
            let key = EdgeId::from_bytes([u8::try_from(i).unwrap() + 1; 16]);
            g.edges.insert(key, edge(from, to, relation, active));
        }
        g
    }

    // INV-REL-02 : détection de cycle sur les relations actives d'un type donné.
    #[test]
    fn cycle_detection_ignores_other_types_and_inactive_edges() {
        let r = Relation::Replaces;
        let acyclic = with_edges(&[
            (1, 2, r, true),
            (1, 3, r, true),
            (2, 4, r, true),
            (3, 4, r, true),
        ]);
        assert!(!acyclic.has_cycle(r));
        for cyclic in [
            with_edges(&[(1, 2, r, true), (2, 1, r, true)]),
            with_edges(&[
                (1, 2, r, true),
                (2, 3, r, true),
                (3, 1, r, true),
                (4, 5, r, true),
            ]),
            with_edges(&[
                (4, 5, r, true),
                (1, 2, r, true),
                (2, 3, r, true),
                (3, 2, r, true),
            ]),
        ] {
            assert!(cyclic.has_cycle(r));
        }
        let broken = with_edges(&[(1, 2, r, true), (2, 1, r, false)]);
        assert!(!broken.has_cycle(r));
        let other = with_edges(&[(1, 2, r, true), (2, 1, Relation::Related, true)]);
        assert!(!other.has_cycle(r));
    }

    // INV-REL-01 : un doublon actif est refusé ; un doublon inactif est de l'historique.
    #[test]
    fn duplicates_are_refused_only_when_active() {
        let d = Relation::DependsOn;
        assert_eq!(
            with_edges(&[(1, 2, d, true), (1, 2, d, true)]).validate(),
            Err(Error::DuplicateRelation)
        );
        assert_eq!(
            with_edges(&[(1, 2, d, false), (1, 2, d, true)]).validate(),
            Ok(())
        );
        assert_eq!(
            with_edges(&[(1, 2, d, true), (1, 2, Relation::Related, true)]).validate(),
            Ok(())
        );
        assert_eq!(
            with_edges(&[
                (1, 2, Relation::Replaces, true),
                (2, 1, Relation::Replaces, true)
            ])
            .validate(),
            Err(Error::CyclicRelation)
        );
    }

    // « Remplacée » et « remplace » vont ensemble, dans les deux sens.
    #[test]
    fn replaced_status_matches_replace_relations() {
        let mut g = with_edges(&[(1, 2, Relation::Replaces, true)]);
        assert_eq!(g.validate(), Err(Error::ReservedStatus));
        g.nodes.get_mut(&id(2)).unwrap().decision = Some(Decision::Replaced);
        assert_eq!(g.validate(), Ok(()));
        assert_eq!(g.replaced_by(id(2)), vec![id(1)]);
        assert!(g.replaced_by(id(1)).is_empty());
        // Relation désactivée (restauration) : le statut reste admis.
        g.edges.values_mut().for_each(|e| e.active = false);
        assert_eq!(g.validate(), Ok(()));
        assert!(g.replaced_by(id(2)).is_empty());
        // Statut sans aucune relation : refusé.
        g.edges.clear();
        assert_eq!(g.validate(), Err(Error::ReservedStatus));
    }

    // INV-REL-06 : une restriction active modifie l'empreinte et crée une cause.
    #[test]
    fn restrictions_change_the_fingerprint() {
        let mut g = with_edges(&[(1, 2, Relation::DependsOn, true)]);
        assert_eq!(g.fingerprint(id(2)), Ok(Fingerprint::default()));
        assert!(g.review_causes().is_empty());
        g.edges.insert(
            EdgeId::from_bytes([9; 16]),
            edge(3, 2, Relation::RestrictsScope, true),
        );
        assert_eq!(
            g.fingerprint(id(2)),
            Ok(Fingerprint {
                substantive: 0,
                restrictions: 1
            })
        );
        assert_eq!(
            g.review_causes(),
            BTreeMap::from([(id(1), BTreeSet::from([id(2)]))])
        );
        g.edges.insert(
            EdgeId::from_bytes([8; 16]),
            edge(4, 1, Relation::RestrictsScope, false),
        );
        assert_eq!(g.fingerprint(id(1)), Ok(Fingerprint::default()));
        assert_eq!(g.fingerprint(id(9)), Err(Error::MissingNode(id(9))));
    }

    /// Instantané cohérent : le contenu canonique est celui des notes des contextes.
    fn snapshot(scopes: Vec<(ScopeId, Graph)>) -> Snapshot {
        let mut notes = BTreeMap::new();
        for (_, graph) in &scopes {
            for (&id, node) in &graph.nodes {
                notes.insert(id, Arc::clone(&node.content));
            }
        }
        Snapshot {
            notes,
            scopes: scopes.into_iter().collect(),
            anchors: BTreeMap::new(),
        }
    }

    #[test]
    fn snapshot_validate_checks_every_scope() {
        let mut bad = valid_graph();
        bad.nodes.get_mut(&id(2)).unwrap().verdict = None;
        let s = snapshot(vec![
            (DEFAULT_SCOPE, valid_graph()),
            (ScopeId::from_bytes([2; 16]), bad),
        ]);
        assert_eq!(s.validate(), Err(Error::MissingVerdict(id(2))));
    }

    // INV-SCOPE-02 : un contexte ne peut pas porter un contenu différent du contenu canonique.
    #[test]
    fn snapshot_validate_rejects_divergent_or_orphan_content() {
        let consistent = snapshot(vec![(DEFAULT_SCOPE, valid_graph())]);
        assert_eq!(consistent.validate(), Ok(()));

        let mut divergent = consistent.clone();
        let other = Arc::new(Content::new(Kind::Reasoning, "titre", "autre texte"));
        divergent.notes.insert(id(2), other);
        assert_eq!(divergent.validate(), Err(Error::DivergentContent(id(2))));

        // Contenu égal mais non partagé : accepté, l'égalité fait foi.
        let mut equal = consistent.clone();
        let copy = Arc::new((*equal.notes[&id(2)]).clone());
        equal.notes.insert(id(2), copy);
        assert_eq!(equal.validate(), Ok(()));

        let mut orphan = consistent;
        orphan.notes.remove(&id(3));
        assert_eq!(orphan.validate(), Err(Error::MissingNode(id(3))));
    }

    // Empreinte FNV-1a 64 bits : valeurs de référence publiées de l'algorithme, plus un caractère
    // non ASCII. Une simple cohérence avec elle-même ne suffit pas (mutation survivante).
    #[test]
    fn text_digest_matches_reference_values() {
        assert_eq!(text_digest(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(text_digest("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(text_digest("foobar"), 0x8594_4171_f739_67e8);
        assert_eq!(text_digest("é"), 0x0ac2_1707_b718_1e01);
    }

    // D5 : seules les révisions substantielles comptent dans l'empreinte de sens.
    #[test]
    fn substantive_count_and_history() {
        let mut content = Content::new(Kind::Reasoning, "t", "v0");
        assert_eq!(
            (content.substantive(), content.current().text.as_str()),
            (0, "v0")
        );
        for (kind, text) in [
            (RevisionKind::Minor, "v1"),
            (RevisionKind::Substantive, "v2"),
            (RevisionKind::Substantive, "v3"),
        ] {
            content.revisions.push((
                kind,
                Text {
                    title: "t".into(),
                    text: text.into(),
                },
            ));
        }
        assert_eq!(
            (content.substantive(), content.current().text.as_str()),
            (2, "v3")
        );
        let kinds: Vec<_> = content.history().iter().map(Revision::kind).collect();
        assert_eq!(
            kinds,
            [
                None,
                Some(RevisionKind::Minor),
                Some(RevisionKind::Substantive),
                Some(RevisionKind::Substantive)
            ]
        );
        let texts: Vec<_> = content
            .history()
            .iter()
            .map(|r| r.text().to_owned())
            .collect();
        assert_eq!(texts, ["v0", "v1", "v2", "v3"]);
        assert!(content.history().iter().all(|r| r.title() == "t"));
    }

    // Révision substantielle : cause pour les relations activées avant, pas après.
    #[test]
    fn review_causes_follow_the_activation_baseline() {
        let mut g = valid_graph();
        let mut revised = (*g.nodes[&id(2)].content).clone();
        revised.revisions.push((
            RevisionKind::Substantive,
            Text {
                title: "r".into(),
                text: "sens inversé".into(),
            },
        ));
        g.nodes.get_mut(&id(2)).unwrap().content = Arc::new(revised);
        assert_eq!(
            g.review_causes(),
            BTreeMap::from([(id(3), BTreeSet::from([id(2)]))])
        );
        for e in g.edges.values_mut() {
            e.baseline = Fingerprint {
                substantive: 1,
                restrictions: 0,
            };
        }
        assert!(g.review_causes().is_empty());
    }

    /// Instantané avec un échange (1), un passage (2) de sa réponse, une réserve (3) issue du
    /// passage et une note (4).
    fn sources() -> Snapshot {
        let exchange = Content::with_body(
            Kind::Exchange,
            "t",
            "",
            Body::Exchange {
                question: None,
                answer: Arc::from("réponse"),
                previous: None,
            },
        );
        let passage = Content::with_body(
            Kind::Passage,
            "",
            "rép",
            Body::Passage {
                exchange: id(1),
                part: Part::Answer,
                start: 0,
                end: 4,
            },
        );
        let reserve = Content::with_body(
            Kind::Reserve,
            "r",
            "",
            Body::Reserve {
                passage: id(2),
                target: None,
            },
        );
        let mut graph = Graph::default();
        for (n, content) in [
            (1, exchange),
            (2, passage),
            (3, reserve),
            (4, Content::new(Kind::Reasoning, "n", "")),
        ] {
            graph.nodes.insert(id(n), Node::new(Arc::new(content)));
        }
        snapshot(vec![(DEFAULT_SCOPE, graph)])
    }

    fn with_content(mut s: Snapshot, n: u8, content: Content) -> Snapshot {
        let content = Arc::new(content);
        s.notes.insert(id(n), Arc::clone(&content));
        for g in s.scopes.values_mut() {
            if let Some(node) = g.nodes.get_mut(&id(n)) {
                node.content = Arc::clone(&content);
            }
        }
        s
    }

    // Rôle et données propres doivent concorder ; un passage doit reproduire sa plage exacte ;
    // une réserve doit venir d'un passage. Ces états ne sont pas atteignables par les commandes.
    #[test]
    fn validate_checks_bodies_against_kinds_and_sources() {
        assert_eq!(sources().validate(), Ok(()));
        let s = sources();
        assert_eq!(s.exchange(id(1)).map(|e| e.answer()), Ok("réponse"));
        assert_eq!(s.reserve(id(3)).map(|r| r.exchange), Ok(id(1)));

        let mismatch = Content::with_body(
            Kind::Passage,
            "",
            "autre",
            Body::Passage {
                exchange: id(1),
                part: Part::Answer,
                start: 0,
                end: 4,
            },
        );
        assert_eq!(
            with_content(sources(), 2, mismatch).validate(),
            Err(Error::InvalidExcerpt)
        );
        let note_with_body = Content::with_body(
            Kind::Reasoning,
            "n",
            "",
            Body::Reserve {
                passage: id(2),
                target: None,
            },
        );
        assert_eq!(
            with_content(sources(), 4, note_with_body).validate(),
            Err(Error::WrongKind(id(4)))
        );
        let exchange_without_body = Content::new(Kind::Exchange, "t", "");
        assert_eq!(
            with_content(sources(), 1, exchange_without_body).validate(),
            Err(Error::WrongKind(id(1)))
        );
        // Rôle à données propres privé de ses données, sans autre objet qui le référence : seule
        // la concordance rôle et données peut le refuser.
        let reserve_without_body = Content::new(Kind::Reserve, "r", "");
        assert_eq!(
            with_content(sources(), 3, reserve_without_body).validate(),
            Err(Error::WrongKind(id(3)))
        );
        let reserve_from_note = Content::with_body(
            Kind::Reserve,
            "r",
            "",
            Body::Reserve {
                passage: id(4),
                target: None,
            },
        );
        assert_eq!(
            with_content(sources(), 3, reserve_from_note).validate(),
            Err(Error::WrongKind(id(4)))
        );
        assert_eq!(
            sources().excerpt(id(1), Part::Question, 0, 1),
            Err(Error::InvalidExcerpt)
        );
        assert_eq!(sources().excerpt(id(1), Part::Answer, 1, 3), Ok("é"));
    }

    // Statuts par rôle ; règles des relations vers et depuis une réserve ; réserves ouvertes.
    #[test]
    fn statuses_and_relations_of_new_kinds() {
        let s = sources();
        let g = s.graph(DEFAULT_SCOPE).unwrap();
        let exchange = g.node(id(1)).unwrap();
        assert_eq!((exchange.verdict(), exchange.decision()), (None, None));
        let passage = g.node(id(2)).unwrap();
        assert_eq!(
            (
                passage.verdict(),
                passage.decision(),
                passage.implementation()
            ),
            (Some(Verdict::Unverified), Some(Decision::Proposed), None)
        );
        assert_eq!(
            g.node(id(3)).unwrap().reserve_status(),
            Some(ReserveStatus::Open)
        );
        assert_eq!(g.node(id(4)).unwrap().reserve_status(), None);
        assert_eq!(g.validate(), Ok(()));
        for (from, to, relation, allowed) in [
            (3, 4, Relation::AppliesTo, true),
            (3, 2, Relation::AppliesTo, true),
            (4, 3, Relation::AppliesTo, false),
            (3, 1, Relation::AppliesTo, false),
            (4, 3, Relation::Treats, true),
            (2, 3, Relation::Treats, true),
            (3, 4, Relation::Treats, false),
            (4, 1, Relation::Treats, false),
            (4, 2, Relation::DependsOn, true),
            // Contrat modifié (5.5a, D18) : une affirmation peut dépendre d'un échange entier.
            (4, 1, Relation::DependsOn, true),
            (1, 4, Relation::DependsOn, false),
            (1, 4, Relation::Related, true),
        ] {
            let result = g.validate_relation(id(from), id(to), relation);
            assert_eq!(result.is_ok(), allowed, "{from} -> {to} : {relation:?}");
        }
        assert!(Relation::AppliesTo.is_plain() && !Relation::Treats.is_plain());
        assert!(
            Kind::Passage.is_claim() && !Kind::Reserve.is_claim() && !Kind::Exchange.is_claim()
        );
        assert!(Kind::Reserve.has_body() && !Kind::PersonalNote.has_body());

        assert_eq!(g.open_reserves(), vec![id(3)]);
        let mut s = sources();
        s.scopes
            .get_mut(&DEFAULT_SCOPE)
            .unwrap()
            .nodes
            .get_mut(&id(3))
            .unwrap()
            .visibility = Visibility::Archived;
        assert!(s.graph(DEFAULT_SCOPE).unwrap().open_reserves().is_empty());
        let mut s = sources();
        s.scopes
            .get_mut(&DEFAULT_SCOPE)
            .unwrap()
            .nodes
            .get_mut(&id(3))
            .unwrap()
            .reserve = Some(ReserveStatus::Resolved);
        assert!(s.graph(DEFAULT_SCOPE).unwrap().open_reserves().is_empty());
        let mut s = sources();
        s.scopes
            .get_mut(&DEFAULT_SCOPE)
            .unwrap()
            .nodes
            .get_mut(&id(3))
            .unwrap()
            .reserve = None;
        assert_eq!(s.validate(), Err(Error::MissingVerdict(id(3))));
        let mut s = sources();
        s.scopes
            .get_mut(&DEFAULT_SCOPE)
            .unwrap()
            .nodes
            .get_mut(&id(4))
            .unwrap()
            .reserve = Some(ReserveStatus::Open);
        assert_eq!(s.validate(), Err(Error::CategoryHasNoVerdict));
    }

    // Données de conversation et acquittements : états non atteignables par les commandes.
    #[test]
    fn validate_checks_conversations_and_acknowledgments() {
        let mut s = sources();
        let looped = Content::with_body(
            Kind::Exchange,
            "t",
            "",
            Body::Exchange {
                question: None,
                answer: Arc::from("réponse"),
                previous: Some(id(4)),
            },
        );
        s = with_content(s, 1, looped);
        assert_eq!(s.validate(), Err(Error::WrongKind(id(4))));

        let mut s = sources();
        for (n, previous) in [(1, 5), (5, 1)] {
            let content = Content::with_body(
                Kind::Exchange,
                "t",
                "",
                Body::Exchange {
                    question: None,
                    answer: Arc::from("réponse"),
                    previous: Some(id(previous)),
                },
            );
            s.notes.insert(id(n), Arc::new(content));
        }
        assert_eq!(s.conversation(id(1)), Err(Error::CyclicRelation));

        let mut s = sources();
        let ack = Ack {
            dependent: 0,
            cause: s.graph(DEFAULT_SCOPE).unwrap().cause_state(id(4)).unwrap(),
        };
        let graph = s.scopes.get_mut(&DEFAULT_SCOPE).unwrap();
        graph.acks.insert((id(4), id(2)), ack);
        assert_eq!(s.validate(), Ok(()));
        let graph = s.scopes.get_mut(&DEFAULT_SCOPE).unwrap();
        graph.acks.insert((id(9), id(2)), ack);
        assert_eq!(s.validate(), Err(Error::MissingNode(id(9))));
        let graph = s.scopes.get_mut(&DEFAULT_SCOPE).unwrap();
        graph.acks.clear();
        graph.acks.insert((id(4), id(9)), ack);
        assert_eq!(s.validate(), Err(Error::MissingNode(id(9))));
    }
}
