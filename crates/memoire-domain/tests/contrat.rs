//! Contrat porté depuis memoire-core (tests/invariants.rs) : assertions inchangées.
//! Seule la mise en place change : les gestes passent par le harnais, qui fournit les
//! identifiants (D3). Comparer avec : diff crates/memoire-core/tests/invariants.rs ce fichier.
mod harness;

use harness::Harness;
use memoire_domain::*;
use std::collections::BTreeSet;

fn note(r: &mut Harness, kind: Kind, title: &str) -> NodeId {
    r.create_node(DEFAULT_SCOPE, kind, title, "texte initial")
        .unwrap()
}

// INV-ID-01 : l'identité et le commentaire ne dépendent pas du contenu courant.
#[test]
fn marker_survives_edit_undo_restore() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Implementation, "file");
    let marker = r.marker(DEFAULT_SCOPE, n).unwrap();
    let before = r.head();
    r.edit_text(DEFAULT_SCOPE, n, "autre algorithme").unwrap();
    assert_eq!(r.marker(DEFAULT_SCOPE, n).unwrap(), marker);
    r.undo().unwrap();
    assert_eq!(r.marker(DEFAULT_SCOPE, n).unwrap(), marker);
    r.restore(before).unwrap();
    assert_eq!(r.marker(DEFAULT_SCOPE, n).unwrap(), marker);
}

// INV-TX-01 : une commande refusée n'a aucun effet, même sur les compteurs.
#[test]
fn rejected_category_invalidation_is_atomic() {
    let mut r = Harness::new();
    let category = note(&mut r, Kind::Category, "scheduler");
    let mut control = r.clone();
    let snapshot = r.snapshot().clone();
    let head = r.head();
    let counts = (r.branch_count(), r.commit_count());
    assert_eq!(
        r.invalidate(DEFAULT_SCOPE, category),
        Err(Error::CategoryHasNoVerdict)
    );
    assert_eq!(r.snapshot(), &snapshot);
    assert_eq!(r.head(), head);
    assert_eq!((r.branch_count(), r.commit_count()), counts);
    assert_eq!(
        note(&mut r, Kind::PersonalNote, "après"),
        note(&mut control, Kind::PersonalNote, "après")
    );
}

// INV-BR-01 : une opération critique ne change ni main ni sa branche d'origine.
#[test]
fn critical_change_forks_before_publish() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Implementation, "liste");
    let original = r.head();
    let original_state = r.snapshot().clone();
    let experimental = r.invalidate(DEFAULT_SCOPE, n).unwrap();
    assert_ne!(experimental, MAIN);
    assert_eq!(r.branch_head(MAIN).unwrap(), original);
    // Contrat modifié (incrément 5.2b) : l'état d'un commit est lu dans l'historique, rendu
    // partagé (Arc) et non plus prêté.
    assert_eq!(*r.snapshot_at(original).unwrap(), original_state);
    assert_eq!(
        r.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(n)
            .unwrap()
            .verdict(),
        Some(Verdict::Refuted)
    );
    r.checkout(MAIN).unwrap();
    assert_eq!(r.snapshot(), &original_state);
    r.checkout(experimental).unwrap();
    assert_ne!(r.snapshot(), &original_state);
}

// INV-SCOPE-01 : une invalidation est locale à un contexte.
#[test]
fn scopes_are_isolated() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "hypothèse");
    let other = r.clone_scope(DEFAULT_SCOPE).unwrap();
    let other_before = r.snapshot().graph(other).unwrap().clone();
    r.invalidate(DEFAULT_SCOPE, n).unwrap();
    assert_eq!(r.snapshot().graph(other).unwrap(), &other_before);
}

// INV-SEM-01 : le classement et les voisins ne propagent pas la réfutation.
#[test]
fn invalidating_a_subidea_keeps_architecture_category_and_sibling() {
    let mut r = Harness::new();
    let cat = note(&mut r, Kind::Category, "ordonnancement");
    let scheduler = note(&mut r, Kind::Implementation, "centralisé");
    let eviction = note(&mut r, Kind::Implementation, "structure pour éviction");
    let sibling = note(&mut r, Kind::Implementation, "réveil");
    r.link(DEFAULT_SCOPE, scheduler, cat, Relation::ClassifiedIn)
        .unwrap();
    r.link(DEFAULT_SCOPE, eviction, scheduler, Relation::DevelopedFrom)
        .unwrap();
    r.link(DEFAULT_SCOPE, sibling, eviction, Relation::Related)
        .unwrap();
    r.invalidate(DEFAULT_SCOPE, eviction).unwrap();
    let graph = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert!(graph.review_causes().is_empty());
    assert_eq!(graph.node(cat).unwrap().verdict(), None);
    assert_eq!(
        graph.node(scheduler).unwrap().verdict(),
        Some(Verdict::Unverified)
    );
    assert!(graph.current_resources().contains(&scheduler));
    assert!(graph.current_resources().contains(&sibling));
}

// INV-SEM-02 : un dépendant est à réexaminer ; il n'est pas automatiquement faux.
#[test]
fn dependency_review_preserves_verdict_nodes_and_edges() {
    let mut r = Harness::new();
    let root = note(&mut r, Kind::Reasoning, "hypothèse");
    let child = note(&mut r, Kind::Implementation, "protocole");
    let edge = r
        .link(DEFAULT_SCOPE, child, root, Relation::DependsOn)
        .unwrap();
    r.invalidate(DEFAULT_SCOPE, root).unwrap();
    let graph = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(graph.review_causes()[&child], BTreeSet::from([root]));
    assert_eq!(
        graph.node(child).unwrap().verdict(),
        Some(Verdict::Unverified)
    );
    assert!(graph.edge(edge).unwrap().active());
    assert_eq!(graph.nodes().count(), 2);
    assert!(!graph.current_resources().contains(&child));
}

// INV-HIST-01 : archiver n'efface pas et ne signifie pas réfuter.
#[test]
fn archival_preserves_verdict_and_relations() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "piste A");
    let b = note(&mut r, Kind::Implementation, "B");
    let edge = r
        .link(DEFAULT_SCOPE, b, a, Relation::DevelopedFrom)
        .unwrap();
    r.archive(DEFAULT_SCOPE, a).unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(g.node(a).unwrap().verdict(), Some(Verdict::Unverified));
    assert_eq!(g.node(a).unwrap().visibility(), Visibility::Archived);
    assert!(g.edge(edge).unwrap().active());
    assert_eq!(g.review_causes()[&b], BTreeSet::from([a]));
}

// INV-HIST-02 : réattacher ajoute une relation et conserve l'ancienne.
#[test]
fn reattachment_preserves_old_edge_and_internal_subgraph() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Reasoning, "B");
    let child = note(&mut r, Kind::Implementation, "enfant");
    let grandchild = note(&mut r, Kind::Implementation, "sous-note");
    let old = r
        .link(DEFAULT_SCOPE, child, a, Relation::DevelopedFrom)
        .unwrap();
    let internal = r
        .link(DEFAULT_SCOPE, grandchild, child, Relation::DependsOn)
        .unwrap();
    let before = r.snapshot().clone();
    let new = r.reattach(DEFAULT_SCOPE, old, b).unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert!(!g.edge(old).unwrap().active());
    assert_eq!(g.edge(old).unwrap().to(), a);
    assert_eq!(g.edge(new).unwrap().to(), b);
    assert!(g.edge(new).unwrap().active());
    assert_eq!(
        g.edge(internal).unwrap(),
        before.graph(DEFAULT_SCOPE).unwrap().edge(internal).unwrap()
    );
    assert_eq!(g.nodes().count(), 4);
}

// INV-UNDO-01 : égalité de l'état métier complet, pas seulement d'un drapeau.
#[test]
fn undo_redo_roundtrip_invalidation_and_all_derived_effects() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Implementation, "B");
    r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    let before = r.snapshot().clone();
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    let after = r.snapshot().clone();
    r.undo().unwrap();
    assert_eq!(r.snapshot(), &before);
    assert!(r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .review_causes()
        .is_empty());
    r.redo().unwrap();
    assert_eq!(r.snapshot(), &after);
    assert!(!r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .review_causes()
        .is_empty());
}

// INV-UNDO-02 : nouvelle action après undo != effacement de l'ancien avenir.
#[test]
fn discarded_redo_remains_addressable_in_history() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::PersonalNote, "note");
    r.edit_text(DEFAULT_SCOPE, n, "piste A").unwrap();
    let abandoned = r.head();
    r.undo().unwrap();
    r.edit_text(DEFAULT_SCOPE, n, "piste B").unwrap();
    assert_eq!(r.redo(), Err(Error::NothingToRedo));
    assert_eq!(
        r.snapshot_at(abandoned)
            .unwrap()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(n)
            .unwrap()
            .text(),
        "piste A"
    );
    let branch_b = r.active_branch();
    let head_b = r.head();
    r.restore(abandoned).unwrap();
    assert_eq!(
        r.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(n)
            .unwrap()
            .text(),
        "piste A"
    );
    // Contrat modifié par D1 (incrément 2.2), puis par D13 (incrément 2.6c) : les éditions d'une
    // note sans dépendant sont ordinaires et restent sur main ; la restauration, critique, crée
    // une branche. L'état atteint reste adressable, main est intacte, et l'annulation y ramène.
    assert_ne!(r.active_branch(), branch_b);
    assert_eq!(r.branch_head(branch_b), Ok(head_b));
    assert_eq!(
        r.snapshot_at(head_b)
            .unwrap()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(n)
            .unwrap()
            .text(),
        "piste B"
    );
    r.undo().unwrap();
    assert_eq!(r.head(), head_b);
    assert_eq!(r.active_branch(), branch_b);
}

#[test]
fn restoration_itself_is_undoable() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::PersonalNote, "note");
    let old = r.head();
    r.edit_text(DEFAULT_SCOPE, n, "actuel").unwrap();
    let before = r.snapshot().clone();
    r.restore(old).unwrap();
    r.undo().unwrap();
    assert_eq!(r.snapshot(), &before);
}

// INV-ID-02 : les identifiants ne sont jamais recyclés après undo.
#[test]
fn abandoned_ids_are_not_reused() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::PersonalNote, "A");
    r.undo().unwrap();
    let b = note(&mut r, Kind::PersonalNote, "B");
    assert_ne!(a, b);
}

// INV-REF-01 : aucun lien vers un objet absent ; échec atomique.
#[test]
fn missing_endpoint_is_rejected_without_changes() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::PersonalNote, "A");
    let b = note(&mut r, Kind::PersonalNote, "B");
    r.undo().unwrap();
    let before = r.snapshot().clone();
    let count = r.commit_count();
    assert_eq!(
        r.link(DEFAULT_SCOPE, a, b, Relation::Related),
        Err(Error::MissingNode(b))
    );
    assert_eq!(r.snapshot(), &before);
    assert_eq!(r.commit_count(), count);
}

#[test]
fn category_cannot_be_a_technical_dependency() {
    let mut r = Harness::new();
    let category = note(&mut r, Kind::Category, "catégorie");
    let idea = note(&mut r, Kind::Implementation, "idée");
    let before = r.snapshot().clone();
    assert_eq!(
        r.link(DEFAULT_SCOPE, idea, category, Relation::DependsOn),
        Err(Error::InvalidRelation)
    );
    assert_eq!(r.snapshot(), &before);
    r.link(DEFAULT_SCOPE, idea, category, Relation::ClassifiedIn)
        .unwrap();
}

// INV-CAUSE-01 : plusieurs causes restent distinguées, undo d'une ne retire pas l'autre.
#[test]
fn review_keeps_each_independent_cause() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Reasoning, "B");
    let child = note(&mut r, Kind::Implementation, "enfant");
    r.link(DEFAULT_SCOPE, child, a, Relation::DependsOn)
        .unwrap();
    r.link(DEFAULT_SCOPE, child, b, Relation::DependsOn)
        .unwrap();
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    r.invalidate(DEFAULT_SCOPE, b).unwrap();
    assert_eq!(
        r.snapshot().graph(DEFAULT_SCOPE).unwrap().review_causes()[&child],
        BTreeSet::from([a, b])
    );
    r.undo().unwrap();
    assert_eq!(
        r.snapshot().graph(DEFAULT_SCOPE).unwrap().review_causes()[&child],
        BTreeSet::from([a])
    );
}

// INV-SOURCE-01 : préserver le texte exact et valider les bornes UTF-8.
#[test]
fn source_excerpt_is_exact_and_rejects_invalid_utf8_ranges() {
    let source = SourceText::new("é🙂 invariant");
    let excerpt = source.excerpt(0, 6).unwrap();
    assert_eq!(excerpt.text(), "é🙂");
    assert_eq!(excerpt.source().text(), "é🙂 invariant");
    for (a, b) in [(1, 2), (0, 1), (2, 5), (3, 6), (0, 100), (6, 2), (2, 2)] {
        assert!(matches!(source.excerpt(a, b), Err(Error::InvalidExcerpt)));
    }
    drop(source);
    assert_eq!(excerpt.text(), "é🙂");
}

// INV-DET-01 : mêmes commandes et même état initial donnent le même état métier.
#[test]
fn deterministic_replay_of_domain_commands() {
    fn run() -> Harness {
        let mut r = Harness::new();
        let a = note(&mut r, Kind::Reasoning, "A");
        let b = note(&mut r, Kind::Implementation, "B");
        r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
        r.invalidate(DEFAULT_SCOPE, a).unwrap();
        r.undo().unwrap();
        r.redo().unwrap();
        r
    }
    let a = run();
    let b = run();
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.head(), b.head());
    assert_eq!(a.active_branch(), b.active_branch());
}

// INV-GRAPH-01 : oracle indépendant par fermeture transitive, cycles compris.
// 2^9 graphes dirigés de 3 sommets, boucles autorisées, 3 racines = 1536 cas.
#[test]
fn exhaustive_three_node_graphs_match_transitive_closure() {
    // Contrat modifié par l'incrément 2.4 (INV-REL-01) : les auto-relations sont refusées ; les
    // graphes contenant une boucle sur une note sont exclus. Restent 64 graphes, soit 192 cas.
    const LOOPS: u16 = 1 | (1 << 4) | (1 << 8);
    for mask in (0u16..512).filter(|mask| mask & LOOPS == 0) {
        let mut r = Harness::new();
        let nodes = [
            note(&mut r, Kind::Reasoning, "0"),
            note(&mut r, Kind::Reasoning, "1"),
            note(&mut r, Kind::Reasoning, "2"),
        ];
        let mut reaches = [[false; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                if mask & (1 << (i * 3 + j)) != 0 {
                    reaches[i][j] = true;
                    r.link(DEFAULT_SCOPE, nodes[i], nodes[j], Relation::DependsOn)
                        .unwrap();
                }
            }
        }
        // Algorithme de référence différent du parcours du moteur.
        for k in 0..3 {
            for i in 0..3 {
                for j in 0..3 {
                    reaches[i][j] = reaches[i][j] || (reaches[i][k] && reaches[k][j]);
                }
            }
        }
        for root in 0..3 {
            let mut candidate = r.clone();
            candidate.invalidate(DEFAULT_SCOPE, nodes[root]).unwrap();
            let causes = candidate
                .snapshot()
                .graph(DEFAULT_SCOPE)
                .unwrap()
                .review_causes();
            for child in 0..3 {
                let expected = child != root && reaches[child][root];
                assert_eq!(
                    causes.contains_key(&nodes[child]),
                    expected,
                    "mask={mask}, root={root}, child={child}"
                );
            }
            candidate.snapshot().validate().unwrap();
        }
    }
}

#[test]
fn editing_content_is_conservatively_critical() {
    // Contrat modifié par D13 (incrément 2.6c) : réviser le sens d'une note n'est critique que si
    // d'autres notes en dépendent ; sans dépendant, l'édition reste ordinaire, sur main.
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "argument");
    r.edit_text(DEFAULT_SCOPE, n, "première formulation")
        .unwrap();
    assert_eq!(r.active_branch(), MAIN);
    let dependent = note(&mut r, Kind::Implementation, "s'appuie sur l'argument");
    r.link(DEFAULT_SCOPE, dependent, n, Relation::DependsOn)
        .unwrap();
    let main_head = r.head();
    r.edit_text(DEFAULT_SCOPE, n, "hypothèse différente")
        .unwrap();
    assert_ne!(r.active_branch(), MAIN);
    assert_eq!(r.branch_head(MAIN).unwrap(), main_head);
}

#[test]
fn refused_reattachment_preserves_old_edge_and_repository() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Implementation, "B");
    let category = note(&mut r, Kind::Category, "catégorie");
    let edge = r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    let before = r.snapshot().clone();
    let head = r.head();
    let branches = r.branch_count();
    assert_eq!(
        r.reattach(DEFAULT_SCOPE, edge, category),
        Err(Error::InvalidRelation)
    );
    assert_eq!(r.snapshot(), &before);
    assert_eq!(r.head(), head);
    assert_eq!(r.branch_count(), branches);
}

#[test]
fn review_is_updated_when_linking_to_an_already_refuted_idea() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Implementation, "B");
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    assert!(r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .review_causes()
        .is_empty());
    r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    assert_eq!(
        r.snapshot().graph(DEFAULT_SCOPE).unwrap().review_causes()[&b],
        BTreeSet::from([a])
    );
}

#[test]
fn removing_one_old_dependency_keeps_the_other_review_cause() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Reasoning, "B");
    let replacement = note(&mut r, Kind::Reasoning, "autre contexte");
    let child = note(&mut r, Kind::Implementation, "enfant");
    let edge_a = r
        .link(DEFAULT_SCOPE, child, a, Relation::DependsOn)
        .unwrap();
    r.link(DEFAULT_SCOPE, child, b, Relation::DependsOn)
        .unwrap();
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    r.invalidate(DEFAULT_SCOPE, b).unwrap();
    r.reattach(DEFAULT_SCOPE, edge_a, replacement).unwrap();
    assert_eq!(
        r.snapshot().graph(DEFAULT_SCOPE).unwrap().review_causes()[&child],
        BTreeSet::from([b])
    );
    assert_eq!(
        r.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(child)
            .unwrap()
            .verdict(),
        Some(Verdict::Unverified)
    );
}

// ---------------------------------------------------------------------------
// Phase 0 — tests ajoutés après le test de mutation (cargo-mutants).
// Aucun test ci-dessus n'a été modifié.
// ---------------------------------------------------------------------------

// INV-ID-01 : deux notes distinctes ont deux repères distincts ;
// un repère n'est produit que pour une note existante dans un contexte existant.
#[test]
fn markers_are_distinct_per_note_and_require_an_existing_note() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Implementation, "A");
    let b = note(&mut r, Kind::Implementation, "B");
    let ghost = note(&mut r, Kind::Implementation, "fantôme");
    let ghost_scope = r.clone_scope(DEFAULT_SCOPE).unwrap();
    r.undo().unwrap();
    r.undo().unwrap();
    let (ma, mb) = (
        r.marker(DEFAULT_SCOPE, a).unwrap(),
        r.marker(DEFAULT_SCOPE, b).unwrap(),
    );
    assert_ne!(ma, mb);
    assert!(ma.starts_with("// @design: ") && mb.starts_with("// @design: "));
    assert_eq!(
        r.marker(DEFAULT_SCOPE, ghost),
        Err(Error::MissingNode(ghost))
    );
    assert_eq!(
        r.marker(ghost_scope, a),
        Err(Error::MissingScope(ghost_scope))
    );
}

// INV-ID-01 : réattacher ne change ni l'identité, ni le contenu, ni le repère de la note.
// INV-HIST-02 : les deux relations restent listées, l'ancienne inactive.
#[test]
fn reattachment_preserves_identity_marker_and_lists_both_edges() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Reasoning, "B");
    let child = note(&mut r, Kind::Implementation, "enfant");
    let old = r
        .link(DEFAULT_SCOPE, child, a, Relation::DevelopedFrom)
        .unwrap();
    let marker = r.marker(DEFAULT_SCOPE, child).unwrap();
    let before = r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .node(child)
        .unwrap()
        .clone();
    let new = r.reattach(DEFAULT_SCOPE, old, b).unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(r.marker(DEFAULT_SCOPE, child).unwrap(), marker);
    assert_eq!(g.node(child).unwrap(), &before);
    let edges: Vec<(EdgeId, NodeId, NodeId, Relation, bool)> = g
        .edges()
        .map(|(id, e)| (id, e.from(), e.to(), e.relation(), e.active()))
        .collect();
    assert_eq!(
        edges,
        vec![
            (old, child, a, Relation::DevelopedFrom, false),
            (new, child, b, Relation::DevelopedFrom, true),
        ]
    );
}

// Éditer le texte ne modifie ni titre, ni rôle, ni verdict, ni visibilité.
#[test]
fn edit_changes_text_only() {
    let mut r = Harness::new();
    let n = r
        .create_node(
            DEFAULT_SCOPE,
            Kind::Reasoning,
            "titre d'origine",
            "texte d'origine",
        )
        .unwrap();
    r.edit_text(DEFAULT_SCOPE, n, "texte révisé").unwrap();
    let node = r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .node(n)
        .unwrap()
        .clone();
    assert_eq!(node.title(), "titre d'origine");
    assert_eq!(node.text(), "texte révisé");
    assert_eq!(node.kind(), Kind::Reasoning);
    assert_eq!(node.verdict(), Some(Verdict::Unverified));
    assert_eq!(node.visibility(), Visibility::Current);
}

// INV-SEM-01 : « classé dans » vise obligatoirement une catégorie ; le refus est sans effet.
#[test]
fn classification_must_target_a_category() {
    let mut r = Harness::new();
    let idea = note(&mut r, Kind::Implementation, "idée");
    let other = note(&mut r, Kind::Reasoning, "raisonnement");
    let category = note(&mut r, Kind::Category, "catégorie");
    let control = r.clone();
    assert_eq!(
        r.link(DEFAULT_SCOPE, idea, other, Relation::ClassifiedIn),
        Err(Error::InvalidRelation)
    );
    assert_eq!(r, control);
    r.link(DEFAULT_SCOPE, idea, category, Relation::ClassifiedIn)
        .unwrap();
}

// INV-BR-01 : classement observable des commandes depuis main.
// Ordinaire : un commit, aucune branche. Critique : un commit et une branche.
#[test]
fn publication_counts_commits_and_branches() {
    let mut r = Harness::new();
    assert_eq!((r.commit_count(), r.branch_count()), (1, 1));
    let a = note(&mut r, Kind::Reasoning, "A");
    let b = note(&mut r, Kind::Implementation, "B");
    assert_eq!((r.commit_count(), r.branch_count()), (3, 1));
    r.link(DEFAULT_SCOPE, b, a, Relation::Related).unwrap();
    assert_eq!((r.commit_count(), r.branch_count()), (4, 1));
    assert_eq!(r.active_branch(), MAIN);
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    assert_eq!((r.commit_count(), r.branch_count()), (5, 2));
}

// CA-40 : une modification ordinaire précédant une opération critique est présente
// dans la branche créée et reste dans la branche d'origine.
#[test]
fn ordinary_change_is_carried_into_the_critical_branch() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let ordinary = note(&mut r, Kind::PersonalNote, "modification ordinaire");
    let experimental = r.invalidate(DEFAULT_SCOPE, a).unwrap();
    for branch in [experimental, MAIN] {
        r.checkout(branch).unwrap();
        assert!(r
            .snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(ordinary)
            .is_ok());
    }
}

// INV-TX-01 : toute commande refusée laisse le dépôt strictement identique
// (états, historique, références de branche, branche active, pile redo, compteurs).
#[test]
fn every_rejected_command_leaves_the_repository_identical() {
    type Case = (
        &'static str,
        Error,
        Box<dyn Fn(&mut Harness) -> Result<(), Error>>,
    );

    // Identifiants valides dans un autre dépôt, absents de celui-ci.
    let (foreign_commit, foreign_branch) = {
        // Graine distincte : identifiants d'un autre dépôt.
        let mut x = Harness::with_seed(99);
        let n = note(&mut x, Kind::Reasoning, "x");
        for _ in 0..64 {
            x.invalidate(DEFAULT_SCOPE, n).unwrap();
        }
        (x.head(), x.active_branch())
    };

    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    // Objets créés puis annulés : identifiants absents et jamais réattribués (INV-ID-02).
    let ghost = note(&mut r, Kind::Reasoning, "fantôme");
    let ghost_edge = r.link(DEFAULT_SCOPE, a, ghost, Relation::Related).unwrap();
    let ghost_scope = r.clone_scope(DEFAULT_SCOPE).unwrap();
    for _ in 0..3 {
        r.undo().unwrap();
    }
    let b = note(&mut r, Kind::Implementation, "B");
    let c = note(&mut r, Kind::Reasoning, "C");
    let cat = note(&mut r, Kind::Category, "catégorie");
    let inactive = r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    let active = r.reattach(DEFAULT_SCOPE, inactive, c).unwrap();
    assert!(commit_absent(&r, foreign_commit));

    let s = DEFAULT_SCOPE;
    let cases: Vec<Case> = vec![
        (
            "réfuter une catégorie",
            Error::CategoryHasNoVerdict,
            Box::new(move |r| r.invalidate(s, cat).map(drop)),
        ),
        (
            "réfuter une note absente",
            Error::MissingNode(ghost),
            Box::new(move |r| r.invalidate(s, ghost).map(drop)),
        ),
        (
            "archiver une note absente",
            Error::MissingNode(ghost),
            Box::new(move |r| r.archive(s, ghost).map(drop)),
        ),
        (
            "éditer une note absente",
            Error::MissingNode(ghost),
            Box::new(move |r| r.edit_text(s, ghost, "x")),
        ),
        (
            "créer dans un contexte absent",
            Error::MissingScope(ghost_scope),
            Box::new(move |r| {
                r.create_node(ghost_scope, Kind::Reasoning, "x", "x")
                    .map(drop)
            }),
        ),
        (
            "copier un contexte absent",
            Error::MissingScope(ghost_scope),
            Box::new(move |r| r.clone_scope(ghost_scope).map(drop)),
        ),
        (
            "lier depuis une note absente",
            Error::MissingNode(ghost),
            Box::new(move |r| r.link(s, ghost, a, Relation::Related).map(drop)),
        ),
        (
            "lier vers une note absente",
            Error::MissingNode(ghost),
            Box::new(move |r| r.link(s, a, ghost, Relation::Related).map(drop)),
        ),
        (
            "dépendre d'une catégorie",
            Error::InvalidRelation,
            Box::new(move |r| r.link(s, b, cat, Relation::DependsOn).map(drop)),
        ),
        (
            "catégorie développée à partir d'une note",
            Error::InvalidRelation,
            Box::new(move |r| r.link(s, cat, a, Relation::DevelopedFrom).map(drop)),
        ),
        (
            "classer dans une non-catégorie",
            Error::InvalidRelation,
            Box::new(move |r| r.link(s, b, a, Relation::ClassifiedIn).map(drop)),
        ),
        (
            "réattacher une relation absente",
            Error::MissingEdge(ghost_edge),
            Box::new(move |r| r.reattach(s, ghost_edge, a).map(drop)),
        ),
        (
            "réattacher une relation inactive",
            Error::InactiveEdge,
            Box::new(move |r| r.reattach(s, inactive, a).map(drop)),
        ),
        (
            "réattacher vers une note absente",
            Error::MissingNode(ghost),
            Box::new(move |r| r.reattach(s, active, ghost).map(drop)),
        ),
        (
            "réattacher une dépendance vers une catégorie",
            Error::InvalidRelation,
            Box::new(move |r| r.reattach(s, active, cat).map(drop)),
        ),
        (
            "restaurer un commit absent",
            Error::MissingCommit(foreign_commit),
            Box::new(move |r| r.restore(foreign_commit).map(drop)),
        ),
        (
            "basculer vers une branche absente",
            Error::MissingBranch(foreign_branch),
            Box::new(move |r| r.checkout(foreign_branch)),
        ),
        (
            "rétablir sans pile redo",
            Error::NothingToRedo,
            Box::new(|r| r.redo()),
        ),
    ];
    for (name, expected, command) in cases {
        let mut candidate = r.clone();
        assert_eq!(command(&mut candidate), Err(expected), "{name}");
        assert_eq!(candidate, r, "{name} : le dépôt a changé");
    }

    let mut fresh = Harness::new();
    let control = fresh.clone();
    assert_eq!(fresh.undo(), Err(Error::NothingToUndo));
    assert_eq!(fresh, control, "annuler sans historique");
}

fn commit_absent(r: &Harness, c: CommitId) -> bool {
    r.snapshot_at(c).is_err()
}

// ---------------------------------------------------------------------------
// Incrément 2.1 — contrat ajouté : identités injectées, enveloppes, aperçu, résultats.
// Remplace les défauts S2, S7 et S9 du registre ; aucun test ci-dessus n'est modifié.
// ---------------------------------------------------------------------------

use std::collections::BTreeMap;

/// Identifiant d'essai non nul, distinct de ceux du harnais.
fn raw(n: u64) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = 0xa5;
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    bytes
}

fn envelope(repo: &Repository, op: u64, command: Command) -> Envelope {
    Envelope {
        op: OpId::from_bytes(raw(op)),
        expected: repo.expected(),
        reason: String::new(),
        command,
    }
}

fn create(scope: ScopeId, id: u64, kind: Kind) -> Command {
    Command::Change(Change::CreateNode {
        scope,
        id: NodeId::from_bytes(raw(id)),
        kind,
        title: "titre".into(),
        text: "texte".into(),
    })
}

fn link(id: u64, from: u64, to: u64, relation: Relation) -> Command {
    Command::Change(Change::Link {
        scope: DEFAULT_SCOPE,
        id: EdgeId::from_bytes(raw(id)),
        from: NodeId::from_bytes(raw(from)),
        to: NodeId::from_bytes(raw(to)),
        relation,
    })
}

// INV-MOD-04 (remplace S2) : une commande sans effet métier ne publie rien
// et laisse le dépôt strictement identique.
#[test]
fn commands_without_business_effect_publish_nothing() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "n");
    r.invalidate(DEFAULT_SCOPE, n).unwrap();
    r.archive(DEFAULT_SCOPE, n).unwrap();
    let before = r.repository().clone();
    // Contrat modifié par D5 (incrément 2.3a) : l'édition devient une révision déclarée ;
    // une révision au contenu identique, quelle que soit sa nature, ne publie rien, pas plus
    // qu'une inclusion dans un contexte qui contient déjà la note.
    let no_ops = [
        Command::Change(Change::Revise {
            node: n,
            text: "texte initial".into(),
            kind: RevisionKind::Minor,
        }),
        Command::Change(Change::Revise {
            node: n,
            text: "texte initial".into(),
            kind: RevisionKind::Substantive,
        }),
        Command::Change(Change::Include {
            scope: DEFAULT_SCOPE,
            node: n,
        }),
        // Contrat modifié par l'incrément 2.3b : la réfutation devient un changement de verdict ;
        // réappliquer un statut déjà en place ne publie rien.
        Command::Change(Change::SetVerdict {
            scope: DEFAULT_SCOPE,
            node: n,
            verdict: Verdict::Refuted,
        }),
        Command::Change(Change::SetDecision {
            scope: DEFAULT_SCOPE,
            node: n,
            decision: Decision::Proposed,
        }),
        Command::Change(Change::SetImplementation {
            scope: DEFAULT_SCOPE,
            node: n,
            implementation: Implementation::NotStarted,
        }),
        Command::Change(Change::Archive {
            scope: DEFAULT_SCOPE,
            node: n,
        }),
        Command::Change(Change::Restore { target: r.head() }),
        Command::Checkout {
            branch: r.active_branch(),
        },
    ];
    for command in no_ops {
        let outcome = r.execute(command.clone()).unwrap();
        assert!(!outcome.published && !outcome.forked, "{command:?}");
        assert_eq!(outcome.effects, Effects::default(), "{command:?}");
        assert_eq!(r.repository(), &before, "{command:?}");
    }
}

// INV-CODE-02 et INV-ID-04 (remplace S7) : le repère ne contient que l'identité de la note ;
// deux dépôts dont les générateurs d'identifiants diffèrent produisent des repères distincts.
#[test]
fn markers_carry_only_the_identity_and_differ_across_repositories() {
    let mut r1 = Harness::with_seed(1);
    let mut r2 = Harness::with_seed(2);
    let a = note(&mut r1, Kind::Implementation, "file RTOS");
    let b = note(&mut r2, Kind::Implementation, "pipeline CPU");
    let ma = r1.marker(DEFAULT_SCOPE, a).unwrap();
    let mb = r2.marker(DEFAULT_SCOPE, b).unwrap();
    assert_ne!(ma, mb);
    assert_eq!(ma, format!("{MARKER_PREFIX}{a}"));
    assert_eq!((parse_marker(&ma), parse_marker(&mb)), (Some(a), Some(b)));
    assert!(!ma.contains("file") && !ma.contains("RTOS"));
    assert_eq!(parse_marker("// @design: file RTOS"), None);
}

// INV-BR-04 (remplace S9) : chaque résultat indique commit, branche active et bifurcation.
#[test]
fn every_outcome_reports_commit_branch_and_fork() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    let b = note(&mut r, Kind::Reasoning, "b");

    r.link(DEFAULT_SCOPE, b, a, Relation::Related).unwrap();
    let ordinary = r.last_outcome().unwrap().clone();
    assert!(ordinary.published && !ordinary.forked);
    assert_eq!((ordinary.branch, ordinary.head), (MAIN, r.head()));

    // Contrat modifié par D13 (incrément 2.6c) : déclarer une dépendance vers une note saine est
    // ordinaire ; l'opération critique de ce test devient une réfutation.
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    let critical = r.last_outcome().unwrap().clone();
    assert!(critical.published && critical.forked);
    assert_ne!(critical.branch, MAIN);
    assert_eq!(
        (critical.branch, critical.head),
        (r.active_branch(), r.head())
    );

    // Contrat modifié par D1 (incrément 2.2) : annuler l'opération fondatrice ramène sur main ;
    // la branche expérimentale reste consultable et le changement de branche est signalé.
    r.undo().unwrap();
    let undone = r.last_outcome().unwrap().clone();
    assert!(!undone.published && !undone.forked);
    assert_eq!((undone.branch, undone.head), (MAIN, ordinary.head));

    r.checkout(critical.branch).unwrap();
    let switched = r.last_outcome().unwrap().clone();
    assert!(!switched.published && !switched.forked);
    assert_eq!(
        (switched.branch, switched.head),
        (critical.branch, critical.head)
    );
}

// INV-PREVIEW-01 : l'aperçu annonce exactement les effets publiés ; un aperçu périmé est refusé.
#[test]
fn preview_and_execution_share_effects_and_a_stale_preview_is_refused() {
    let mut repo = Repository::new();
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Reasoning)),
        (11, create(DEFAULT_SCOPE, 2, Kind::Implementation)),
        (12, link(3, 2, 1, Relation::DependsOn)),
    ] {
        let next = envelope(&repo, op, command);
        repo.execute(next).unwrap();
    }
    // Contrat modifié par D1 (incrément 2.2), puis par D13 (incrément 2.6c) : déclarer une
    // dépendance vers une note saine est ordinaire ; on est toujours sur main, et la réfutation
    // prévisualisée ci-dessous créera une branche.
    assert_eq!(repo.active_branch(), MAIN);
    let (a, b) = (NodeId::from_bytes(raw(1)), NodeId::from_bytes(raw(2)));
    // Contrat modifié par l'incrément 2.3b : la réfutation devient un changement de verdict.
    let invalidate = Command::Change(Change::SetVerdict {
        scope: DEFAULT_SCOPE,
        node: a,
        verdict: Verdict::Refuted,
    });
    let gained = ReviewChange {
        before: BTreeSet::new(),
        after: BTreeSet::from([a]),
    };

    let stale_preview = repo.preview(&invalidate).unwrap();
    assert_eq!(stale_preview.base, repo.expected());
    assert!(stale_preview.publishes && stale_preview.forks);
    assert_eq!(
        stale_preview.effects.review,
        BTreeMap::from([((DEFAULT_SCOPE, b), gained.clone())])
    );

    // Une autre commande publie entre l'aperçu et son application.
    let other = envelope(&repo, 13, create(DEFAULT_SCOPE, 4, Kind::PersonalNote));
    repo.execute(other).unwrap();
    let control = repo.clone();
    let late = Envelope {
        op: OpId::from_bytes(raw(14)),
        expected: stale_preview.base,
        reason: String::new(),
        command: invalidate.clone(),
    };
    let actual = repo.expected();
    assert_eq!(
        repo.execute(late),
        Err(Error::Stale {
            expected: stale_preview.base,
            actual
        })
    );
    assert_eq!(repo, control);

    // Aperçu à jour : l'exécution publie exactement ce qui était annoncé, avec son motif.
    let preview = repo.preview(&invalidate).unwrap();
    let outcome = repo
        .execute(Envelope {
            op: OpId::from_bytes(raw(15)),
            expected: preview.base,
            reason: "hypothèse réfutée".into(),
            command: invalidate,
        })
        .unwrap();
    assert_eq!(outcome.effects, preview.effects);
    assert_eq!(
        (outcome.published, outcome.forked),
        (preview.publishes, preview.forks)
    );
    // Contrat modifié (incrément 5.2b) : le motif est lu dans l'historique, rendu possédé.
    assert_eq!(
        repo.commit_reason(outcome.head),
        Ok("hypothèse réfutée".to_owned())
    );
    assert_eq!(
        repo.commit_reason(CommitId::from_bytes(raw(99))),
        Err(Error::MissingCommit(CommitId::from_bytes(raw(99))))
    );

    // Les déplacements de tête annoncent aussi leurs effets.
    let lost = ReviewChange {
        before: BTreeSet::from([a]),
        after: BTreeSet::new(),
    };
    for command in [Command::Undo, Command::Checkout { branch: MAIN }] {
        let preview = repo.preview(&command).unwrap();
        assert!(!preview.publishes && !preview.forks, "{command:?}");
        assert_eq!(
            preview.effects.review,
            BTreeMap::from([((DEFAULT_SCOPE, b), lost.clone())]),
            "{command:?}"
        );
    }
}

// INV-RETRY-01 : répéter une opération identifiée ne l'applique qu'une fois ;
// réemployer son identifiant pour une autre commande est refusé.
#[test]
fn retrying_an_operation_applies_it_exactly_once() {
    let mut repo = Repository::new();
    let first = envelope(&repo, 1, create(DEFAULT_SCOPE, 2, Kind::Reasoning));
    let outcome = repo.execute(first.clone()).unwrap();
    let after = repo.clone();
    assert_eq!(repo.execute(first.clone()), Ok(outcome.clone()));
    assert_eq!(repo, after);

    let mut other = first.clone();
    other.command = create(DEFAULT_SCOPE, 3, Kind::Reasoning);
    assert_eq!(repo.execute(other), Err(Error::OpIdInUse(first.op)));
    assert_eq!(repo, after);

    // Une annulation répétée n'annule qu'une fois ; rejouer l'opération annulée ne la refait pas.
    let undo = envelope(&repo, 4, Command::Undo);
    let undone = repo.execute(undo.clone()).unwrap();
    let after_undo = repo.clone();
    assert_eq!(repo.execute(undo), Ok(undone));
    assert_eq!(repo.execute(first), Ok(outcome));
    assert_eq!(repo, after_undo);
    assert_eq!(repo.head(), ROOT);
}

// INV-ID-02 sous D3 : une identité employée, même par un état annulé, n'est jamais réattribuée.
#[test]
fn an_identity_is_never_reused_even_after_undo() {
    let mut repo = Repository::new();
    let steps = [
        (1, create(DEFAULT_SCOPE, 7, Kind::Reasoning)),
        (2, create(DEFAULT_SCOPE, 8, Kind::Reasoning)),
        (3, link(9, 8, 7, Relation::Related)),
        (
            4,
            Command::Change(Change::CloneScope {
                source: DEFAULT_SCOPE,
                id: ScopeId::from_bytes(raw(10)),
            }),
        ),
        (5, Command::Undo),
        (6, Command::Undo),
    ];
    for (op, command) in steps {
        let next = envelope(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let control = repo.clone();
    let reuses = [
        (
            create(DEFAULT_SCOPE, 7, Kind::PersonalNote),
            Error::NodeIdInUse(NodeId::from_bytes(raw(7))),
        ),
        (
            link(9, 7, 8, Relation::Related),
            Error::EdgeIdInUse(EdgeId::from_bytes(raw(9))),
        ),
        (
            Command::Change(Change::CloneScope {
                source: DEFAULT_SCOPE,
                id: ScopeId::from_bytes(raw(10)),
            }),
            Error::ScopeIdInUse(ScopeId::from_bytes(raw(10))),
        ),
    ];
    for (op, (command, expected)) in (20..).zip(reuses) {
        let attempt = envelope(&repo, op, command);
        assert_eq!(repo.execute(attempt), Err(expected));
        assert_eq!(repo, control);
    }
}

// INV-TX-01 sous D3 : les identités portées par une commande refusée restent disponibles.
#[test]
fn identities_of_a_refused_command_remain_available() {
    let mut repo = Repository::new();
    let ghost = ScopeId::from_bytes(raw(50));
    let refused = envelope(&repo, 1, create(ghost, 2, Kind::Reasoning));
    assert_eq!(repo.execute(refused), Err(Error::MissingScope(ghost)));
    assert_eq!(repo, Repository::new());
    let accepted = envelope(&repo, 1, create(DEFAULT_SCOPE, 2, Kind::Reasoning));
    assert!(repo.execute(accepted).unwrap().published);
}

// L'identifiant nul est réservé aux objets initiaux : refusé pour une opération ou un objet créé.
#[test]
fn nil_identifiers_are_refused() {
    let mut repo = Repository::new();
    let nil = [0u8; 16];
    let attempts = [
        Envelope {
            op: OpId::from_bytes(nil),
            ..envelope(&repo, 1, create(DEFAULT_SCOPE, 2, Kind::Reasoning))
        },
        envelope(
            &repo,
            3,
            Command::Change(Change::CreateNode {
                scope: DEFAULT_SCOPE,
                id: NodeId::from_bytes(nil),
                kind: Kind::Reasoning,
                title: String::new(),
                text: String::new(),
            }),
        ),
        envelope(
            &repo,
            4,
            Command::Change(Change::CloneScope {
                source: DEFAULT_SCOPE,
                id: DEFAULT_SCOPE,
            }),
        ),
    ];
    for attempt in attempts {
        assert_eq!(repo.execute(attempt), Err(Error::NilIdentifier));
        assert_eq!(repo, Repository::new());
    }
}

// INV-ID-04 : mêmes enveloppes, identifiants compris, sur un même état initial = même dépôt.
#[test]
fn identical_envelopes_produce_identical_repositories() {
    fn replay() -> Repository {
        let mut repo = Repository::new();
        let steps = [
            (1, create(DEFAULT_SCOPE, 2, Kind::Reasoning)),
            (3, create(DEFAULT_SCOPE, 4, Kind::Implementation)),
            (5, link(6, 4, 2, Relation::DependsOn)),
            (7, Command::Undo),
            (8, Command::Redo),
            (9, Command::Checkout { branch: MAIN }),
        ];
        for (op, command) in steps {
            let next = envelope(&repo, op, command);
            repo.execute(next).unwrap();
        }
        repo
    }
    let (a, b) = (replay(), replay());
    assert_eq!(a, b);
    assert_eq!(a.active_branch(), MAIN);
}

// Contrat modifié par D13 (incrément 2.6c ; remplace criticality_policy_follows_d10, qui
// classait les commandes par type) : la criticité se juge sur l'effet réel dans l'état courant.
// Est critique ce qui crée ou lève une cause de réexamen, écarte une note, ou restaure un état.
// L'aperçu annonce la décision de bifurcation qui sera appliquée.
#[test]
fn criticality_follows_the_actual_effect() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let healthy = note(&mut r, Kind::Reasoning, "note saine dont une autre dépend");
    let lonely = note(&mut r, Kind::Implementation, "note sans dépendant");
    let dependent = note(&mut r, Kind::Implementation, "note dépendante");
    let category = note(&mut r, Kind::Category, "catégorie");
    // Déclarer une dépendance vers une note saine ne crée aucune cause : on reste sur main.
    r.link(S, dependent, healthy, Relation::DependsOn).unwrap();
    assert!(!r.last_outcome().unwrap().forked);
    assert_eq!(r.active_branch(), MAIN);

    let edge = |n| EdgeId::from_bytes(raw(n));
    let ordinary = [
        Change::Revise {
            node: lonely,
            text: "réécrite".into(),
            kind: RevisionKind::Substantive,
        },
        Change::Revise {
            node: healthy,
            text: "coquille".into(),
            kind: RevisionKind::Minor,
        },
        Change::SetVerdict {
            scope: S,
            node: lonely,
            verdict: Verdict::Verified,
        },
        Change::SetDecision {
            scope: S,
            node: lonely,
            decision: Decision::Retained,
        },
        Change::SetImplementation {
            scope: S,
            node: lonely,
            implementation: Implementation::InProgress,
        },
        Change::Link {
            scope: S,
            id: edge(1),
            from: lonely,
            to: category,
            relation: Relation::ClassifiedIn,
        },
        Change::Link {
            scope: S,
            id: edge(2),
            from: lonely,
            to: healthy,
            relation: Relation::DependsOn,
        },
        Change::Restrict {
            scope: S,
            id: edge(3),
            from: lonely,
            to: dependent,
            condition: "monocœur".into(),
        },
        Change::Contradict {
            scope: S,
            id: edge(4),
            from: lonely,
            to: healthy,
            justification: "désaccord".into(),
        },
        Change::CreateNode {
            scope: S,
            id: NodeId::from_bytes(raw(5)),
            kind: Kind::Reasoning,
            title: "t".into(),
            text: "x".into(),
        },
        Change::CloneScope {
            source: S,
            id: ScopeId::from_bytes(raw(6)),
        },
    ];
    let critical = [
        Change::Revise {
            node: healthy,
            text: "sens inversé".into(),
            kind: RevisionKind::Substantive,
        },
        Change::SetVerdict {
            scope: S,
            node: lonely,
            verdict: Verdict::Refuted,
        },
        Change::SetDecision {
            scope: S,
            node: lonely,
            decision: Decision::NoLongerRetained,
        },
        Change::Archive {
            scope: S,
            node: lonely,
        },
        Change::Replace {
            scope: S,
            id: edge(7),
            replacement: dependent,
            replaced: lonely,
        },
        Change::Restrict {
            scope: S,
            id: edge(8),
            from: lonely,
            to: healthy,
            condition: "monocœur".into(),
        },
        Change::Restore { target: ROOT },
    ];
    let repo = r.repository();
    for change in ordinary {
        let preview = repo.preview(&Command::Change(change.clone())).unwrap();
        assert!(preview.publishes && !preview.forks, "{change:?}");
    }
    for change in critical {
        let preview = repo.preview(&Command::Change(change.clone())).unwrap();
        assert!(preview.publishes && preview.forks, "{change:?}");
    }

    // Sur une branche expérimentale, rien ne bifurque (D1).
    r.archive(S, lonely).unwrap();
    assert_ne!(r.active_branch(), MAIN);
    let refute = Change::SetVerdict {
        scope: S,
        node: healthy,
        verdict: Verdict::Refuted,
    };
    let preview = r.repository().preview(&Command::Change(refute)).unwrap();
    assert!(preview.publishes && !preview.forks);
}

// ---------------------------------------------------------------------------
// Incrément 2.2 — contrat ajouté : branches (D1) et restauration non destructive (D6).
// Remplace les défauts S1, S3 et S10. Deux tests ci-dessus sont modifiés par D1 ; chaque
// modification est signalée en commentaire à l'endroit concerné.
// ---------------------------------------------------------------------------

// INV-BR-02 (remplace S1) : seule main déclenche une bifurcation ; sur une branche
// expérimentale, les opérations critiques s'y poursuivent.
#[test]
fn critical_changes_on_an_experimental_branch_stay_on_it() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "n");
    // Contrat modifié par D13 (incrément 2.6c) : n a un dépendant, sans quoi réviser son sens ne
    // serait pas critique.
    let dependent = note(&mut r, Kind::Implementation, "dépendante");
    r.link(DEFAULT_SCOPE, dependent, n, Relation::DependsOn)
        .unwrap();
    let main_head = r.head();
    r.edit_text(DEFAULT_SCOPE, n, "v0").unwrap();
    let experimental = r.active_branch();
    assert!(r.last_outcome().unwrap().forked);
    assert_ne!(experimental, MAIN);
    for i in 1..10 {
        r.edit_text(DEFAULT_SCOPE, n, &format!("v{i}")).unwrap();
        let outcome = r.last_outcome().unwrap();
        assert!(outcome.published && !outcome.forked, "v{i}");
        assert_eq!(outcome.branch, experimental, "v{i}");
    }
    assert_eq!(r.branch_count(), 2);
    assert_eq!(r.branch_head(MAIN), Ok(main_head));
}

// INV-BR-03 (remplace S3) : l'annulation ne franchit pas la base d'une branche ; annuler
// l'opération fondatrice ramène sur l'origine, la branche restant consultable.
#[test]
fn undo_is_bounded_by_the_branch_base_and_returns_to_origin() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    let b = note(&mut r, Kind::Reasoning, "b");
    let base = r.head();
    let main_state = r.snapshot().clone();
    let experimental = r.invalidate(DEFAULT_SCOPE, a).unwrap();
    let founding = r.head();
    r.archive(DEFAULT_SCOPE, b).unwrap();
    let explored = r.snapshot().clone();

    // Une annulation ordinaire reste sur la branche expérimentale.
    r.undo().unwrap();
    assert_eq!((r.active_branch(), r.head()), (experimental, founding));
    // Annuler l'opération fondatrice ramène sur main, à la base.
    r.undo().unwrap();
    assert_eq!((r.active_branch(), r.head()), (MAIN, base));
    assert_eq!(r.snapshot(), &main_state);
    assert_eq!(r.branch_head(experimental), Ok(founding));
    // Rétablir rentre dans la branche, puis réapplique l'opération suivante.
    r.redo().unwrap();
    assert_eq!((r.active_branch(), r.head()), (experimental, founding));
    r.redo().unwrap();
    assert_eq!(r.snapshot(), &explored);
    // De retour sur main, une annulation de plus porte sur main elle-même, jamais sur la branche.
    r.undo().unwrap();
    r.undo().unwrap();
    r.undo().unwrap();
    assert_eq!(r.active_branch(), MAIN);
    assert!(r.snapshot().graph(DEFAULT_SCOPE).unwrap().node(b).is_err());
    assert_eq!(r.branch_head(experimental), Ok(founding));
}

// INV-BR-03 : rétablir rentre dans la branche quittée seulement si elle n'a pas changé depuis ;
// sinon le rétablissement réappliquerait plus que l'opération annulée.
#[test]
fn reentering_a_branch_that_changed_since_is_refused() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    let experimental = r.invalidate(DEFAULT_SCOPE, a).unwrap();
    r.undo().unwrap();
    r.checkout(experimental).unwrap();
    note(
        &mut r,
        Kind::PersonalNote,
        "travail poursuivi sur la branche",
    );
    r.checkout(MAIN).unwrap();
    let control = r.clone();
    assert_eq!(r.redo(), Err(Error::StaleHead));
    assert_eq!(r, control);
}

// INV-RET-03 (remplace S10) : restaurer rétablit l'état des objets existant à la cible ;
// les objets créés depuis restent présents et résolubles, retirés des ressources actuelles,
// et les relations créées depuis sont inactives.
#[test]
fn restore_keeps_later_objects_retired_and_resolvable() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A");
    let target = r.head();
    let at_target = r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .node(a)
        .unwrap()
        .clone();
    r.edit_text(DEFAULT_SCOPE, a, "A révisée").unwrap();
    let later = note(&mut r, Kind::Implementation, "créée après la cible");
    let edge = r
        .link(DEFAULT_SCOPE, later, a, Relation::DependsOn)
        .unwrap();
    let scope = r.clone_scope(DEFAULT_SCOPE).unwrap();
    let marker = r.marker(DEFAULT_SCOPE, later).unwrap();
    let before_restore = r.snapshot().clone();

    r.restore(target).unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(g.node(a).unwrap(), &at_target);
    assert_eq!(g.node(later).unwrap().visibility(), Visibility::Archived);
    assert_eq!(g.node(later).unwrap().text(), "texte initial");
    assert!(!g.edge(edge).unwrap().active());
    assert!(g.review_causes().is_empty());
    assert_eq!(g.current_resources(), vec![a]);
    let copied = r.snapshot().graph(scope).unwrap();
    assert!(copied
        .nodes()
        .all(|(_, n)| n.visibility() == Visibility::Archived));
    assert!(copied.edges().all(|(_, e)| !e.active()));
    assert_eq!(r.marker(DEFAULT_SCOPE, later), Ok(marker));

    r.undo().unwrap();
    assert_eq!(r.snapshot(), &before_restore);
}

/// Identités présentes dans les contextes donnés d'un état.
fn identities(s: &Snapshot, scopes: &[ScopeId]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for &scope in scopes {
        if let Ok(g) = s.graph(scope) {
            set.insert(format!("contexte {scope}"));
            set.extend(g.nodes().map(|(id, _)| format!("{scope} note {id}")));
            set.extend(g.edges().map(|(id, _)| format!("{scope} relation {id}")));
        }
    }
    set
}

// INV-RET-01 : toute modification acceptée, restauration comprise, conserve toutes les
// identités de l'état précédent ; aucune commande ne supprime un objet.
#[test]
fn every_accepted_change_keeps_all_identities() {
    type Step = Box<dyn Fn(&mut Harness) -> Result<(), Error>>;
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    let target = r.head();
    let b = note(&mut r, Kind::Implementation, "b");
    let cat = note(&mut r, Kind::Category, "c");
    let e = r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    let copy = r.clone_scope(DEFAULT_SCOPE).unwrap();
    let scopes = [DEFAULT_SCOPE, copy];
    let steps: Vec<(&str, Step)> = vec![
        (
            "éditer",
            Box::new(move |r| r.edit_text(DEFAULT_SCOPE, a, "a2")),
        ),
        (
            "classer",
            Box::new(move |r| {
                r.link(DEFAULT_SCOPE, a, cat, Relation::ClassifiedIn)
                    .map(drop)
            }),
        ),
        (
            "réfuter",
            Box::new(move |r| r.invalidate(DEFAULT_SCOPE, a).map(drop)),
        ),
        (
            "retirer",
            Box::new(move |r| r.archive(DEFAULT_SCOPE, b).map(drop)),
        ),
        (
            "réattacher",
            Box::new(move |r| r.reattach(DEFAULT_SCOPE, e, a).map(drop)),
        ),
        (
            "copier un contexte",
            Box::new(move |r| r.clone_scope(copy).map(drop)),
        ),
        ("restaurer", Box::new(move |r| r.restore(target).map(drop))),
        (
            "créer",
            Box::new(|r| {
                r.create_node(DEFAULT_SCOPE, Kind::PersonalNote, "d", "")
                    .map(drop)
            }),
        ),
    ];
    for (name, step) in steps {
        let previous = identities(r.snapshot(), &scopes);
        step(&mut r).unwrap();
        assert!(r.last_outcome().unwrap().published, "{name}");
        assert!(
            previous.is_subset(&identities(r.snapshot(), &scopes)),
            "{name}"
        );
    }
}

// INV-PREVIEW-01 et D1 : l'aperçu annonce la décision de bifurcation qui sera appliquée.
#[test]
fn preview_announces_the_fork_decision() {
    let mut repo = Repository::new();
    let first = envelope(&repo, 1, create(DEFAULT_SCOPE, 2, Kind::Reasoning));
    repo.execute(first).unwrap();
    // Contrat modifié par D13 (incrément 2.6c) : la note révisée a un dépendant, sans quoi sa
    // révision ne serait pas critique. Déclarer cette dépendance est ordinaire.
    for (op, command) in [
        (4, create(DEFAULT_SCOPE, 5, Kind::Implementation)),
        (6, link(7, 5, 2, Relation::DependsOn)),
    ] {
        let next = envelope(&repo, op, command);
        assert!(!repo.execute(next).unwrap().forked);
    }
    // Contrat modifié par D5 (incrément 2.3a) : révision substantielle déclarée.
    let edit = |text: &str| {
        Command::Change(Change::Revise {
            node: NodeId::from_bytes(raw(2)),
            text: text.into(),
            kind: RevisionKind::Substantive,
        })
    };
    let from_main = repo.preview(&edit("v1")).unwrap();
    assert!(from_main.publishes && from_main.forks);
    let next = envelope(&repo, 3, edit("v1"));
    assert!(repo.execute(next).unwrap().forked);
    let on_branch = repo.preview(&edit("v2")).unwrap();
    assert!(on_branch.publishes && !on_branch.forks);
}

// ---------------------------------------------------------------------------
// Incrément 2.3a — contrat ajouté : contenu partagé entre contextes (D2) et révisions (D5).
// Remplace les défauts S5 et S6. Trois tests ci-dessus sont modifiés par D5 et D10, signalés
// en commentaire à l'endroit concerné.
// ---------------------------------------------------------------------------

fn text_in(r: &Harness, scope: ScopeId, n: NodeId) -> String {
    r.snapshot()
        .graph(scope)
        .unwrap()
        .node(n)
        .unwrap()
        .text()
        .to_owned()
}

// INV-SCOPE-02 (remplace S5) : identité et révisions sont partagées entre contextes ; un contexte
// porte l'appartenance et la qualification de la note, et l'activation de ses relations.
#[test]
fn contexts_share_identity_and_revisions() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "n");
    let multi = r.clone_scope(DEFAULT_SCOPE).unwrap();
    r.edit_text(DEFAULT_SCOPE, n, "corrigé en monocœur")
        .unwrap();
    for scope in [DEFAULT_SCOPE, multi] {
        assert_eq!(text_in(&r, scope, n), "corrigé en monocœur");
    }

    // La qualification reste propre au contexte.
    r.invalidate(multi, n).unwrap();
    let verdict = |r: &Harness, scope| {
        r.snapshot()
            .graph(scope)
            .unwrap()
            .node(n)
            .unwrap()
            .verdict()
    };
    assert_eq!(verdict(&r, DEFAULT_SCOPE), Some(Verdict::Unverified));
    assert_eq!(verdict(&r, multi), Some(Verdict::Refuted));

    // Une note créée après la copie peut rejoindre l'autre contexte, sans copie de contenu.
    let later = note(&mut r, Kind::Reasoning, "après copie");
    assert!(r.snapshot().graph(multi).unwrap().node(later).is_err());
    r.include(multi, later).unwrap();
    assert!(!r.last_outcome().unwrap().forked);
    r.revise(later, "révisée une fois", RevisionKind::Minor)
        .unwrap();
    assert_eq!(text_in(&r, multi, later), "révisée une fois");
    assert_eq!(text_in(&r, DEFAULT_SCOPE, later), "révisée une fois");

    // Inclure une note absente du dépôt, ou dans un contexte absent, est refusé sans effet.
    let ghost = note(&mut r, Kind::Reasoning, "fantôme");
    let ghost_scope = r.clone_scope(DEFAULT_SCOPE).unwrap();
    r.undo().unwrap();
    r.undo().unwrap();
    let control = r.clone();
    assert_eq!(r.include(multi, ghost), Err(Error::MissingNode(ghost)));
    assert_eq!(
        r.include(ghost_scope, later),
        Err(Error::MissingScope(ghost_scope))
    );
    assert_eq!(r, control);
}

// INV-MOD-02 (remplace S6), D5 et D10 : une révision substantielle met ses dépendants, directs
// et indirects, en réexamen ; une révision mineure ne crée aucune cause et reste ordinaire.
#[test]
fn substantive_revision_requires_review_of_dependents() {
    let mut r = Harness::new();
    let root = note(&mut r, Kind::Reasoning, "hypothèse");
    r.revise(root, "hypothèse reformulée", RevisionKind::Minor)
        .unwrap();
    assert!(!r.last_outcome().unwrap().forked);
    let child = note(&mut r, Kind::Implementation, "protocole");
    let grandchild = note(&mut r, Kind::Implementation, "détail");
    r.link(DEFAULT_SCOPE, child, root, Relation::DependsOn)
        .unwrap();
    r.link(DEFAULT_SCOPE, grandchild, child, Relation::DependsOn)
        .unwrap();
    let causes = |r: &Harness| r.snapshot().graph(DEFAULT_SCOPE).unwrap().review_causes();

    r.revise(
        root,
        "hypothèse reformulée, coquille corrigée",
        RevisionKind::Minor,
    )
    .unwrap();
    assert!(causes(&r).is_empty());

    r.revise(root, "hypothèse inverse", RevisionKind::Substantive)
        .unwrap();
    let expected = BTreeMap::from([
        (child, BTreeSet::from([root])),
        (grandchild, BTreeSet::from([root])),
    ]);
    assert_eq!(causes(&r), expected);
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(g.node(child).unwrap().verdict(), Some(Verdict::Unverified));
    assert!(!g.current_resources().contains(&child));
    assert!(g.current_resources().contains(&root));

    r.undo().unwrap();
    assert!(causes(&r).is_empty());
    r.redo().unwrap();
    assert_eq!(causes(&r), expected);

    // Une dépendance déclarée après la révision part de la version courante : aucune cause.
    let late = note(&mut r, Kind::Implementation, "tardif");
    r.link(DEFAULT_SCOPE, late, root, Relation::DependsOn)
        .unwrap();
    assert_eq!(causes(&r), expected);
}

// INV-MOD-01 : chaque version reste adressable avec sa nature ; identité et repère inchangés.
#[test]
fn revisions_are_kept_with_their_kind() {
    let mut r = Harness::new();
    let n = r
        .create_node(DEFAULT_SCOPE, Kind::Reasoning, "titre", "v0")
        .unwrap();
    let marker = r.marker(DEFAULT_SCOPE, n).unwrap();
    r.revise(n, "v1", RevisionKind::Minor).unwrap();
    r.revise(n, "v2", RevisionKind::Substantive).unwrap();
    let history: Vec<(String, String, Option<RevisionKind>)> = r
        .snapshot()
        .revisions(n)
        .unwrap()
        .iter()
        .map(|v| (v.title().to_owned(), v.text().to_owned(), v.kind()))
        .collect();
    assert_eq!(
        history,
        [
            ("titre".to_owned(), "v0".to_owned(), None),
            (
                "titre".to_owned(),
                "v1".to_owned(),
                Some(RevisionKind::Minor)
            ),
            (
                "titre".to_owned(),
                "v2".to_owned(),
                Some(RevisionKind::Substantive)
            ),
        ]
    );
    let current = r.snapshot().graph(DEFAULT_SCOPE).unwrap().node(n).unwrap();
    assert_eq!((current.text(), current.revision_count()), ("v2", 3));
    assert_eq!(r.marker(DEFAULT_SCOPE, n), Ok(marker));

    let ghost = note(&mut r, Kind::Reasoning, "fantôme");
    r.undo().unwrap();
    assert_eq!(
        r.snapshot().revisions(ghost),
        Err(Error::MissingNode(ghost))
    );
}

// ---------------------------------------------------------------------------
// Incrément 2.3b — contrat ajouté : statuts séparés (D6, D11) et motif obligatoire (D12).
// Trois tests ci-dessus sont modifiés : la réfutation devient un changement de verdict.
// ---------------------------------------------------------------------------

fn node_in(r: &Harness, scope: ScopeId, n: NodeId) -> Node {
    r.snapshot().graph(scope).unwrap().node(n).unwrap().clone()
}

// Les trois dimensions de statut sont indépendantes et propres au contexte ;
// une catégorie n'en porte aucune.
#[test]
fn statuses_are_independent_and_scoped() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Implementation, "idée");
    let other = r.clone_scope(DEFAULT_SCOPE).unwrap();
    let initial = node_in(&r, DEFAULT_SCOPE, n);
    assert_eq!(
        (
            initial.decision(),
            initial.verdict(),
            initial.implementation()
        ),
        (
            Some(Decision::Proposed),
            Some(Verdict::Unverified),
            Some(Implementation::NotStarted)
        )
    );

    r.set_decision(DEFAULT_SCOPE, n, Decision::Retained)
        .unwrap();
    r.set_verdict(DEFAULT_SCOPE, n, Verdict::Verified).unwrap();
    r.set_implementation(DEFAULT_SCOPE, n, Implementation::InProgress)
        .unwrap();
    let changed = node_in(&r, DEFAULT_SCOPE, n);
    assert_eq!(
        (
            changed.decision(),
            changed.verdict(),
            changed.implementation()
        ),
        (
            Some(Decision::Retained),
            Some(Verdict::Verified),
            Some(Implementation::InProgress)
        )
    );
    assert_eq!(node_in(&r, other, n), initial);
    assert_eq!(
        r.branch_count(),
        1,
        "aucun de ces changements n'est critique"
    );

    let category = note(&mut r, Kind::Category, "catégorie");
    let c = node_in(&r, DEFAULT_SCOPE, category);
    assert_eq!(
        (c.decision(), c.verdict(), c.implementation()),
        (None, None, None)
    );
    let control = r.clone();
    assert_eq!(
        r.set_decision(DEFAULT_SCOPE, category, Decision::Retained),
        Err(Error::CategoryHasNoVerdict)
    );
    assert_eq!(
        r.set_implementation(DEFAULT_SCOPE, category, Implementation::Implemented),
        Err(Error::CategoryHasNoVerdict)
    );
    assert_eq!(r, control);
}

// INV-RET-02 et D11 : « plus retenue » est réversible ; la note reste consultable et
// résoluble, sort des ressources actuelles, et ses dépendants sont à réexaminer sans changer
// de statut. Les notes proposées ou retenues font partie des ressources actuelles.
#[test]
fn no_longer_retained_is_reversible_and_requires_review() {
    let mut r = Harness::new();
    let idea = note(
        &mut r,
        Kind::Implementation,
        "le driver réveille les tâches",
    );
    let proposed = note(&mut r, Kind::Reasoning, "simplement proposée");
    let marker = r.marker(DEFAULT_SCOPE, idea).unwrap();
    r.set_decision(DEFAULT_SCOPE, idea, Decision::Retained)
        .unwrap();
    let resources = |r: &Harness| {
        r.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .current_resources()
    };
    let causes = |r: &Harness| r.snapshot().graph(DEFAULT_SCOPE).unwrap().review_causes();
    assert_eq!(resources(&r), vec![idea, proposed]);

    // Depuis main, ne plus retenir est critique : une branche est créée, main reste intacte.
    let main_head = r.head();
    let experimental = r
        .set_decision(DEFAULT_SCOPE, idea, Decision::NoLongerRetained)
        .unwrap();
    assert_ne!(experimental, MAIN);
    assert_eq!(r.branch_head(MAIN), Ok(main_head));
    r.undo().unwrap();
    assert_eq!(r.active_branch(), MAIN);

    // Avec un dépendant : la note sort des ressources mais reste résoluble ; le dépendant est
    // à réexaminer, sans changement de statut, et la relation reste active.
    let dependent = note(&mut r, Kind::Implementation, "poignée de main");
    let edge = r
        .link(DEFAULT_SCOPE, dependent, idea, Relation::DependsOn)
        .unwrap();
    r.set_decision(DEFAULT_SCOPE, idea, Decision::NoLongerRetained)
        .unwrap();
    assert_eq!(resources(&r), vec![proposed]);
    assert_eq!(
        causes(&r),
        BTreeMap::from([(dependent, BTreeSet::from([idea]))])
    );
    let d = node_in(&r, DEFAULT_SCOPE, dependent);
    assert_eq!(
        (d.decision(), d.verdict()),
        (Some(Decision::Proposed), Some(Verdict::Unverified))
    );
    assert!(r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .edge(edge)
        .unwrap()
        .active());
    assert_eq!(
        node_in(&r, DEFAULT_SCOPE, idea).decision(),
        Some(Decision::NoLongerRetained)
    );
    assert_eq!(r.marker(DEFAULT_SCOPE, idea), Ok(marker));

    // Réversible par un nouveau changement de statut, sans annulation.
    r.set_decision(DEFAULT_SCOPE, idea, Decision::Retained)
        .unwrap();
    assert!(causes(&r).is_empty());
    assert_eq!(resources(&r), vec![idea, proposed, dependent]);
}

// INV-MOD-05 et D12 : un changement de statut exige un motif non vide, conservé avec le commit.
// Les autres modifications n'en exigent pas.
#[test]
fn status_changes_require_a_reason() {
    let mut repo = Repository::new();
    let first = envelope(&repo, 1, create(DEFAULT_SCOPE, 2, Kind::Reasoning));
    repo.execute(first).unwrap();
    let n = NodeId::from_bytes(raw(2));
    let statuses = [
        Change::SetVerdict {
            scope: DEFAULT_SCOPE,
            node: n,
            verdict: Verdict::Verified,
        },
        Change::SetDecision {
            scope: DEFAULT_SCOPE,
            node: n,
            decision: Decision::Retained,
        },
        Change::SetImplementation {
            scope: DEFAULT_SCOPE,
            node: n,
            implementation: Implementation::InProgress,
        },
    ];
    let control = repo.clone();
    for (op, change) in (10..).zip(statuses.iter().cloned()) {
        for reason in ["", "  \t\n"] {
            let attempt = Envelope {
                reason: reason.into(),
                ..envelope(&repo, op, Command::Change(change.clone()))
            };
            assert_eq!(
                repo.execute(attempt),
                Err(Error::MissingReason),
                "{change:?}"
            );
            assert_eq!(repo, control);
        }
    }
    for (op, change) in (20..).zip(statuses) {
        let accepted = Envelope {
            reason: "vérifié sur la carte".into(),
            ..envelope(&repo, op, Command::Change(change))
        };
        let outcome = repo.execute(accepted).unwrap();
        assert_eq!(
            repo.commit_reason(outcome.head),
            Ok("vérifié sur la carte".to_owned())
        );
    }
    let revise = Command::Change(Change::Revise {
        node: n,
        text: "sans motif".into(),
        kind: RevisionKind::Minor,
    });
    let without_reason = envelope(&repo, 30, revise);
    assert!(repo.execute(without_reason).unwrap().published);
}

// « Remplacée » est réservé à la relation « remplace » (incrément 2.4) : refusé sans effet.
#[test]
fn replaced_is_reserved_to_the_replace_relation() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Implementation, "structure de file");
    let control = r.clone();
    assert_eq!(
        r.set_decision(DEFAULT_SCOPE, n, Decision::Replaced),
        Err(Error::ReservedStatus)
    );
    assert_eq!(r, control);
}

// ---------------------------------------------------------------------------
// Incrément 2.4 — contrat ajouté : règles des relations, remplacement, restriction de périmètre,
// contradiction. Remplace les défauts S4 et S11. Deux tests ci-dessus sont modifiés, signalés
// en commentaire.
// ---------------------------------------------------------------------------

// INV-REL-01 (remplace S4) : aucune auto-relation ; aucun doublon actif. Une relation
// identique redevient possible quand la précédente a été désactivée.
#[test]
fn relations_reject_self_and_active_duplicates() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    let b = note(&mut r, Kind::Reasoning, "b");
    let c = note(&mut r, Kind::Reasoning, "c");
    let control = r.clone();
    for relation in [
        Relation::DependsOn,
        Relation::DevelopedFrom,
        Relation::Related,
    ] {
        assert_eq!(
            r.link(DEFAULT_SCOPE, a, a, relation),
            Err(Error::SelfRelation),
            "{relation:?}"
        );
    }
    assert_eq!(r.replace(DEFAULT_SCOPE, a, a), Err(Error::SelfRelation));
    assert_eq!(
        r.restrict(DEFAULT_SCOPE, a, a, "si"),
        Err(Error::SelfRelation)
    );
    assert_eq!(
        r.contradict(DEFAULT_SCOPE, a, a, "car"),
        Err(Error::SelfRelation)
    );
    assert_eq!(r, control);

    let first = r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    let before = r.clone();
    assert_eq!(
        r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn),
        Err(Error::DuplicateRelation)
    );
    assert_eq!(r, before);
    // Même extrémités, autre type : admis.
    r.link(DEFAULT_SCOPE, b, a, Relation::Related).unwrap();
    // Réattacher vers une destination déjà liée de la même façon : doublon refusé.
    r.link(DEFAULT_SCOPE, b, c, Relation::DependsOn).unwrap();
    let before = r.clone();
    assert_eq!(
        r.reattach(DEFAULT_SCOPE, first, c),
        Err(Error::DuplicateRelation)
    );
    assert_eq!(r, before);
    // Réattacher vers soi-même : refusé.
    assert_eq!(
        r.reattach(DEFAULT_SCOPE, first, b),
        Err(Error::SelfRelation)
    );
    // Après désactivation par réattachement, la relation d'origine peut être recréée.
    let d = note(&mut r, Kind::Reasoning, "d");
    r.reattach(DEFAULT_SCOPE, first, d).unwrap();
    r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
}

// INV-REL-03 (remplace S11) : la hiérarchie des catégories est sans cycle ; une note peut être
// classée dans plusieurs catégories ; classer ne crée aucune cause.
#[test]
fn category_hierarchy_is_acyclic() {
    let mut r = Harness::new();
    let evolution = note(&mut r, Kind::Category, "Exploration / évolution");
    let multicore = note(&mut r, Kind::Category, "Exploration multicœur");
    let driver = note(&mut r, Kind::Category, "Driver GPIO");
    let exchange = note(&mut r, Kind::Reasoning, "E3");
    r.link(DEFAULT_SCOPE, multicore, evolution, Relation::ClassifiedIn)
        .unwrap();
    r.link(DEFAULT_SCOPE, exchange, multicore, Relation::ClassifiedIn)
        .unwrap();
    r.link(DEFAULT_SCOPE, exchange, driver, Relation::ClassifiedIn)
        .unwrap();
    assert!(!r.last_outcome().unwrap().forked);
    assert!(r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .review_causes()
        .is_empty());
    let control = r.clone();
    assert_eq!(
        r.link(DEFAULT_SCOPE, evolution, multicore, Relation::ClassifiedIn),
        Err(Error::CyclicRelation)
    );
    assert_eq!(r, control);
}

// INV-MOD-06, INV-REL-02 et D12 : remplacer pose le statut « remplacée » dans la même opération,
// avec motif ; la note remplacée reste consultable et résoluble, indique sa remplaçante, et ses
// dépendants sont à réexaminer. Aucun cycle de remplacement n'est admis.
#[test]
fn replacement_sets_status_atomically_and_stays_resolvable() {
    let mut r = Harness::new();
    let old = note(&mut r, Kind::Implementation, "structure de file A");
    let new = note(&mut r, Kind::Implementation, "structure de file B");
    let dependent = note(&mut r, Kind::Implementation, "politique d'éviction");
    r.link(DEFAULT_SCOPE, dependent, old, Relation::DependsOn)
        .unwrap();
    let marker = r.marker(DEFAULT_SCOPE, old).unwrap();
    let before = r.snapshot().clone();

    let edge = r.replace(DEFAULT_SCOPE, new, old).unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(g.node(old).unwrap().decision(), Some(Decision::Replaced));
    assert_eq!(g.edge(edge).unwrap().relation(), Relation::Replaces);
    assert_eq!(g.replaced_by(old), vec![new]);
    assert_eq!(
        g.review_causes(),
        BTreeMap::from([(dependent, BTreeSet::from([old]))])
    );
    assert!(!g.current_resources().contains(&old));
    assert!(g.current_resources().contains(&new));
    assert_eq!(r.marker(DEFAULT_SCOPE, old), Ok(marker));

    // Le statut ne peut plus être changé tant que le remplacement est actif.
    let control = r.clone();
    assert_eq!(
        r.set_decision(DEFAULT_SCOPE, old, Decision::Retained),
        Err(Error::ReservedStatus)
    );
    assert_eq!(r, control);
    // Une seule opération : l'annuler rétablit statut et relations.
    r.undo().unwrap();
    assert_eq!(r.snapshot(), &before);
    r.redo().unwrap();

    // Cycle de remplacement refusé ; relation « remplace » réservée à sa commande ; catégorie exclue.
    let third = note(&mut r, Kind::Implementation, "structure de file C");
    r.replace(DEFAULT_SCOPE, third, new).unwrap();
    let control = r.clone();
    assert_eq!(
        r.replace(DEFAULT_SCOPE, old, third),
        Err(Error::CyclicRelation)
    );
    assert_eq!(r, control);
    assert_eq!(
        r.link(DEFAULT_SCOPE, old, third, Relation::Replaces),
        Err(Error::InvalidRelation)
    );
    let category = note(&mut r, Kind::Category, "files");
    let after_category = r.clone();
    assert_eq!(
        r.replace(DEFAULT_SCOPE, category, third),
        Err(Error::InvalidRelation)
    );
    assert_eq!(r, after_category);
}

// D12 : remplacer change un statut, donc exige un motif non vide.
#[test]
fn replacement_requires_a_reason() {
    let mut repo = Repository::new();
    for (op, command) in [
        (1, create(DEFAULT_SCOPE, 2, Kind::Implementation)),
        (3, create(DEFAULT_SCOPE, 4, Kind::Implementation)),
    ] {
        let next = envelope(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let replace = Command::Change(Change::Replace {
        scope: DEFAULT_SCOPE,
        id: EdgeId::from_bytes(raw(5)),
        replacement: NodeId::from_bytes(raw(4)),
        replaced: NodeId::from_bytes(raw(2)),
    });
    let control = repo.clone();
    let without = envelope(&repo, 6, replace.clone());
    assert_eq!(repo.execute(without), Err(Error::MissingReason));
    assert_eq!(repo, control);
    let with = Envelope {
        reason: "structure B retenue".into(),
        ..envelope(&repo, 6, replace)
    };
    assert!(repo.execute(with).unwrap().forked);
}

// INV-REL-06 : restreindre le périmètre ajoute une condition visible, sans verdict ni
// modification de la cible ; les dépendants de la cible sont à réexaminer. Critique.
#[test]
fn restriction_adds_a_condition_and_requires_review_of_dependents() {
    let mut r = Harness::new();
    let claim = note(
        &mut r,
        Kind::Reasoning,
        "une écriture atomique ne suffit pas en multicœur",
    );
    let source = note(
        &mut r,
        Kind::Reasoning,
        "réponse supposant plusieurs ordonnanceurs",
    );
    let dependent = note(&mut r, Kind::Implementation, "protocole de transfert");
    let control = r.clone();
    for empty in ["", "   "] {
        assert_eq!(
            r.restrict(DEFAULT_SCOPE, source, claim, empty),
            Err(Error::MissingAnnotation)
        );
    }
    assert_eq!(r, control);

    let edge = r
        .restrict(DEFAULT_SCOPE, source, claim, "plusieurs ordonnanceurs")
        .unwrap();
    // Contrat modifié par D13 (incrément 2.6c) : sans dépendant, la restriction ne crée aucune
    // cause et reste ordinaire ; la seconde, ci-dessous, en crée une et devient critique.
    assert!(!r.last_outcome().unwrap().forked);
    r.link(DEFAULT_SCOPE, dependent, claim, Relation::DependsOn)
        .unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(
        g.edge(edge).unwrap().annotation(),
        Some("plusieurs ordonnanceurs")
    );
    assert_eq!(g.edge(edge).unwrap().relation(), Relation::RestrictsScope);
    assert!(
        g.review_causes().is_empty(),
        "dépendance déclarée après la restriction"
    );

    // Une seconde restriction modifie l'empreinte de la cible : le dépendant est à réexaminer.
    let second = note(&mut r, Kind::Reasoning, "cœur dédié à l'ordonnancement");
    r.restrict(
        DEFAULT_SCOPE,
        second,
        claim,
        "sans cœur d'ordonnancement dédié",
    )
    .unwrap();
    assert!(r.last_outcome().unwrap().forked);
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(
        g.review_causes(),
        BTreeMap::from([(dependent, BTreeSet::from([claim]))])
    );
    let target = g.node(claim).unwrap();
    assert_eq!(
        (target.verdict(), target.decision(), target.text()),
        (
            Some(Verdict::Unverified),
            Some(Decision::Proposed),
            "texte initial"
        )
    );
}

// INV-REL-07 : contredire enregistre un désaccord justifié, sans propagation ni modification
// de la cible. Ordinaire : depuis main, aucune branche n'est créée.
#[test]
fn contradiction_records_a_justification_without_propagation() {
    let mut r = Harness::new();
    let recommendation = note(&mut r, Kind::Reasoning, "primitive commune du noyau");
    let decision = note(&mut r, Kind::Implementation, "le driver écrit l'état");
    let control = r.clone();
    assert_eq!(
        r.contradict(DEFAULT_SCOPE, decision, recommendation, " "),
        Err(Error::MissingAnnotation)
    );
    assert_eq!(r, control);
    let before = r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .node(recommendation)
        .unwrap()
        .clone();
    let edge = r
        .contradict(
            DEFAULT_SCOPE,
            decision,
            recommendation,
            "monocœur, section critique courte",
        )
        .unwrap();
    assert!(!r.last_outcome().unwrap().forked);
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(
        g.edge(edge).unwrap().annotation(),
        Some("monocœur, section critique courte")
    );
    assert_eq!(g.node(recommendation).unwrap(), &before);
    assert!(g.review_causes().is_empty());
    assert_eq!(g.current_resources(), vec![recommendation, decision]);
    // Une contradiction ne se réattache pas : elle porte un texte propre à sa cible.
    let other = note(&mut r, Kind::Reasoning, "autre");
    assert_eq!(
        r.reattach(DEFAULT_SCOPE, edge, other),
        Err(Error::InvalidRelation)
    );
}

// ---------------------------------------------------------------------------
// Incrément 2.5 — contrat ajouté : échanges, sources, passages et réserves.
// Un test ci-dessus est complété (criticality_policy_follows_d10), sans assertion modifiée.
// ---------------------------------------------------------------------------

/// Réponse d'essai : tabulation, espace insécable, accents, emoji, et la séquence invisible
/// laissée par une citation perdue au collage (U+2060 U+FFFC).
const ANSWER: &str =
    "Élément\tRôle\nÉcrire 1 acquitte\u{a0}: voir 01-LM3S9B92-DataSheet.pdf\u{2060}\u{fffc}\nFin 🚀.";

// INV-SOURCE-03 et INV-SOURCE-04 : les sources sont conservées octet pour octet ; question et
// réponse sont délimitées explicitement ; une réponse seule reste « incomplète », la question
// s'ajoute une fois et ne se remplace pas ; une source ne se révise pas.
#[test]
fn exchanges_keep_sources_byte_for_byte_with_explicit_boundaries() {
    let mut r = Harness::new();
    let complete = r
        .create_exchange(
            DEFAULT_SCOPE,
            "rôle du driver",
            Some("Voici mon code"),
            ANSWER,
        )
        .unwrap();
    assert!(!r.last_outcome().unwrap().forked);
    let sources = r.snapshot().exchange(complete).unwrap();
    assert_eq!(sources.answer().as_bytes(), ANSWER.as_bytes());
    assert_eq!(sources.question(), Some("Voici mon code"));
    assert!(sources.is_complete());

    let partial = r
        .create_exchange(DEFAULT_SCOPE, "rôle du driver", None, ANSWER)
        .unwrap();
    let sources = r.snapshot().exchange(partial).unwrap();
    assert!(!sources.is_complete());
    assert_eq!(sources.question(), None);
    r.set_question(partial, "question retrouvée").unwrap();
    let sources = r.snapshot().exchange(partial).unwrap();
    assert!(sources.is_complete());
    assert_eq!(sources.question(), Some("question retrouvée"));

    let n = note(&mut r, Kind::Reasoning, "note");
    let control = r.clone();
    assert_eq!(
        r.set_question(partial, "autre question"),
        Err(Error::SourceImmutable(partial))
    );
    assert_eq!(
        r.revise(complete, "réponse réécrite", RevisionKind::Minor),
        Err(Error::SourceImmutable(complete))
    );
    for (question, answer) in [(None, ""), (None, " \n\t"), (Some(""), "réponse")] {
        assert_eq!(
            r.create_exchange(DEFAULT_SCOPE, "vide", question, answer),
            Err(Error::EmptySource)
        );
    }
    assert_eq!(r.set_question(n, "q"), Err(Error::WrongKind(n)));
    for kind in [Kind::Exchange, Kind::Passage, Kind::Reserve] {
        let attempt = r.create_node(DEFAULT_SCOPE, kind, "t", "x");
        assert!(matches!(attempt, Err(Error::WrongKind(_))), "{kind:?}");
    }
    assert_eq!(r.snapshot().exchange(n), Err(Error::WrongKind(n)));
    assert_eq!(r, control);

    let empty_question = r.create_exchange(DEFAULT_SCOPE, "t", None, "r").unwrap();
    let control = r.clone();
    assert_eq!(
        r.set_question(empty_question, "  "),
        Err(Error::EmptySource)
    );
    assert_eq!(r, control);
}

// INV-SOURCE-01 : un passage désigne exactement une plage valide d'une partie d'un échange ;
// il est immuable mais qualifiable comme une affirmation ; le corriger met ses dépendants en
// réexamen sans toucher au reste de l'échange (CA-12).
#[test]
fn passages_designate_exact_ranges_and_can_be_corrected() {
    let mut r = Harness::new();
    let exchange = r
        .create_exchange(DEFAULT_SCOPE, "rôle du driver", None, ANSWER)
        .unwrap();
    let word = "acquitte";
    let start = ANSWER.find(word).unwrap();
    let passage = r
        .create_passage(
            DEFAULT_SCOPE,
            exchange,
            Part::Answer,
            start,
            start + word.len(),
        )
        .unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(g.node(passage).unwrap().text(), word);
    assert_eq!(g.node(passage).unwrap().kind(), Kind::Passage);
    assert_eq!(
        r.snapshot().passage(passage).unwrap(),
        PassageView {
            exchange,
            part: Part::Answer,
            start,
            end: start + word.len()
        }
    );
    let invisible = ANSWER.find('\u{2060}').unwrap();
    let cite = r
        .create_passage(
            DEFAULT_SCOPE,
            exchange,
            Part::Answer,
            invisible,
            invisible + 6,
        )
        .unwrap();
    assert_eq!(
        r.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(cite)
            .unwrap()
            .text(),
        "\u{2060}\u{fffc}"
    );

    let control = r.clone();
    let rocket = ANSWER.find('🚀').unwrap();
    for (part, s, e) in [
        (Part::Answer, start, start),
        (Part::Answer, start + 3, start),
        (Part::Answer, 0, ANSWER.len() + 1),
        (Part::Answer, rocket, rocket + 1),
        (Part::Answer, 0, 1),
        (Part::Question, 0, 1),
    ] {
        let attempt = r.create_passage(DEFAULT_SCOPE, exchange, part, s, e);
        assert_eq!(attempt, Err(Error::InvalidExcerpt), "{part:?} {s}..{e}");
    }
    let other = r.clone_scope(DEFAULT_SCOPE).unwrap();
    r.undo().unwrap();
    assert_eq!(
        r.create_passage(other, exchange, Part::Answer, 0, 1),
        Err(Error::MissingScope(other))
    );
    assert_eq!(
        r.create_passage(DEFAULT_SCOPE, passage, Part::Answer, 0, 1),
        Err(Error::WrongKind(passage))
    );
    assert_eq!(
        r.revise(passage, "autre texte", RevisionKind::Minor),
        Err(Error::SourceImmutable(passage))
    );
    assert_eq!(
        r.snapshot().passage(exchange),
        Err(Error::WrongKind(exchange))
    );
    assert_eq!(r.repository().snapshot(), control.repository().snapshot());

    // Corriger le passage : une note qui s'appuie dessus est à réexaminer ; l'autre passage du
    // même échange et l'échange lui-même ne changent pas.
    let claim = note(&mut r, Kind::Implementation, "l'écriture de 1 acquitte");
    r.link(DEFAULT_SCOPE, claim, passage, Relation::DependsOn)
        .unwrap();
    let correction = note(&mut r, Kind::Reasoning, "vérifié dans la datasheet");
    r.replace(DEFAULT_SCOPE, correction, passage).unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(
        g.node(passage).unwrap().decision(),
        Some(Decision::Replaced)
    );
    assert_eq!(g.node(passage).unwrap().text(), word);
    assert_eq!(
        g.review_causes(),
        BTreeMap::from([(claim, BTreeSet::from([passage]))])
    );
    assert_eq!(g.node(cite).unwrap().decision(), Some(Decision::Proposed));
    assert_eq!(r.snapshot().exchange(exchange).unwrap().answer(), ANSWER);
}

// INV-RES-01 à INV-RES-05 : une réserve naît d'un passage sans destination obligatoire et figure
// dans les points ouverts ; « s'applique à » ne change pas son état ; seule l'action « traite »,
// avec motif, la fait passer à « traitée à vérifier » ; la résolution est explicite et propre
// au contexte ; une cible future est du texte, jamais une relation.
#[test]
fn reserves_follow_an_explicit_lifecycle_per_scope() {
    let mut r = Harness::new();
    let exchange = r
        .create_exchange(DEFAULT_SCOPE, "rôle du driver", Some("Voici"), ANSWER)
        .unwrap();
    let passage = r
        .create_passage(DEFAULT_SCOPE, exchange, Part::Answer, 0, "Élément".len())
        .unwrap();
    let reserve = r
        .create_reserve(
            DEFAULT_SCOPE,
            passage,
            "acquittement à vérifier",
            Some("startup.s (à créer)"),
        )
        .unwrap();
    let status = |r: &Harness, scope| {
        r.snapshot()
            .graph(scope)
            .unwrap()
            .node(reserve)
            .unwrap()
            .reserve_status()
    };
    assert_eq!(status(&r, DEFAULT_SCOPE), Some(ReserveStatus::Open));
    let view = r.snapshot().reserve(reserve).unwrap();
    assert_eq!(
        (view.passage, view.exchange, view.target),
        (passage, exchange, Some("startup.s (à créer)"))
    );
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(g.open_reserves(), vec![reserve]);
    assert!(
        g.edges().next().is_none(),
        "aucune relation vers une cible future"
    );
    let r_node = g.node(reserve).unwrap();
    assert_eq!((r_node.decision(), r_node.verdict()), (None, None));

    let note_id = note(&mut r, Kind::PersonalNote, "protocole d'attente");
    r.link(DEFAULT_SCOPE, reserve, note_id, Relation::AppliesTo)
        .unwrap();
    assert_eq!(status(&r, DEFAULT_SCOPE), Some(ReserveStatus::Open));
    let other = r.clone_scope(DEFAULT_SCOPE).unwrap();

    r.treat(DEFAULT_SCOPE, note_id, reserve).unwrap();
    assert_eq!(
        status(&r, DEFAULT_SCOPE),
        Some(ReserveStatus::TreatedPendingVerification)
    );
    assert_eq!(
        r.snapshot().graph(DEFAULT_SCOPE).unwrap().open_reserves(),
        vec![reserve]
    );
    r.set_reserve_status(DEFAULT_SCOPE, reserve, ReserveStatus::Resolved)
        .unwrap();
    assert_eq!(status(&r, DEFAULT_SCOPE), Some(ReserveStatus::Resolved));
    assert!(r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .open_reserves()
        .is_empty());
    assert_eq!(status(&r, other), Some(ReserveStatus::Open));
    assert!(!r.last_outcome().unwrap().forked);

    let control = r.clone();
    assert_eq!(
        r.set_reserve_status(DEFAULT_SCOPE, note_id, ReserveStatus::Resolved),
        Err(Error::WrongKind(note_id))
    );
    assert_eq!(
        r.set_decision(DEFAULT_SCOPE, reserve, Decision::Retained),
        Err(Error::CategoryHasNoVerdict)
    );
    assert_eq!(
        r.link(DEFAULT_SCOPE, note_id, reserve, Relation::AppliesTo),
        Err(Error::InvalidRelation)
    );
    assert_eq!(
        r.link(DEFAULT_SCOPE, note_id, reserve, Relation::Treats),
        Err(Error::InvalidRelation)
    );
    assert_eq!(
        r.create_reserve(DEFAULT_SCOPE, note_id, "pas un passage", None),
        Err(Error::WrongKind(note_id))
    );
    assert_eq!(
        r.create_reserve(DEFAULT_SCOPE, passage, "cible vide", Some(" ")),
        Err(Error::MissingAnnotation)
    );
    assert_eq!(
        r.snapshot().reserve(passage),
        Err(Error::WrongKind(passage))
    );
    assert_eq!(r, control);
}

// D12 : traiter une réserve et changer son état sont des changements de statut : motif exigé.
#[test]
fn reserve_transitions_require_a_reason() {
    let mut repo = Repository::new();
    let (exchange, passage, reserve, note_id) = (raw(1), raw(2), raw(3), raw(4));
    let steps = [
        // Contrat modifié par l'incrément 2.6 : un échange peut désigner son prédécesseur.
        Change::CreateExchange {
            scope: DEFAULT_SCOPE,
            id: NodeId::from_bytes(exchange),
            title: "t".into(),
            question: None,
            answer: "réponse".into(),
            previous: None,
        },
        Change::CreatePassage {
            scope: DEFAULT_SCOPE,
            id: NodeId::from_bytes(passage),
            exchange: NodeId::from_bytes(exchange),
            part: Part::Answer,
            start: 0,
            end: 3,
        },
        Change::CreateReserve {
            scope: DEFAULT_SCOPE,
            id: NodeId::from_bytes(reserve),
            passage: NodeId::from_bytes(passage),
            title: "r".into(),
            target: None,
        },
        Change::CreateNode {
            scope: DEFAULT_SCOPE,
            id: NodeId::from_bytes(note_id),
            kind: Kind::PersonalNote,
            title: "n".into(),
            text: "n".into(),
        },
    ];
    for (op, change) in (10..).zip(steps) {
        let next = envelope(&repo, op, Command::Change(change));
        repo.execute(next).unwrap();
    }
    let control = repo.clone();
    let transitions = [
        Change::Treat {
            scope: DEFAULT_SCOPE,
            id: EdgeId::from_bytes(raw(5)),
            note: NodeId::from_bytes(note_id),
            reserve: NodeId::from_bytes(reserve),
        },
        Change::SetReserveStatus {
            scope: DEFAULT_SCOPE,
            reserve: NodeId::from_bytes(reserve),
            status: ReserveStatus::Resolved,
        },
    ];
    for (op, change) in (20..).zip(transitions) {
        let without = envelope(&repo, op, Command::Change(change));
        assert_eq!(repo.execute(without), Err(Error::MissingReason));
        assert_eq!(repo, control);
    }
}

/// Découpe explicite du document en échanges, faite ici par l'utilisateur et non par le domaine :
/// les lignes « ** question » et « ** réponse » (ou « ** reponse ») délimitent les sources.
fn split_exchanges(doc: &str) -> Vec<(&str, &str)> {
    let mut marks = Vec::new();
    let mut offset = 0;
    for line in doc.split_inclusive('\n') {
        let question = match line.trim_end_matches('\n') {
            "** question" => Some(true),
            "** réponse" | "** reponse" => Some(false),
            _ => None,
        };
        if let Some(question) = question {
            marks.push((offset, offset + line.len(), question));
        }
        offset += line.len();
    }
    assert!(marks.chunks(2).all(|pair| pair[0].2 && !pair[1].2));
    marks
        .chunks(2)
        .enumerate()
        .map(|(k, pair)| {
            let next = marks.get(2 * k + 2).map_or(doc.len(), |m| m.0);
            (&doc[pair[0].1..pair[1].0], &doc[pair[1].1..next])
        })
        .collect()
}

// S-REF-01, étapes 1 et 5, sur les sources réelles figées : cinq échanges conservés octet pour
// octet, aucune relation déduite de l'ordre, trois réserves aux états attendus.
#[test]
fn reference_scenario_steps_1_and_5_on_real_sources() {
    const DOC: &str = include_str!("../../../fixtures/rtos-2026-10/doc.md");
    let parts = split_exchanges(DOC);
    assert_eq!(parts.len(), 5);
    let mut r = Harness::new();
    let title = "rôle du driver de géréer la gestion du wake up des task en fonction de la topologie de ses IO";
    let exchanges: Vec<NodeId> = parts
        .iter()
        .map(|(q, a)| r.create_exchange(DEFAULT_SCOPE, title, Some(q), a).unwrap())
        .collect();

    // Étape 1 : rien n'est perdu ni découpé ; les sources recomposées redonnent le document.
    let mut rebuilt = String::from(&DOC[..DOC.find("** question").unwrap()]);
    let mut answer_marker = "** réponse\n";
    for (i, &e) in exchanges.iter().enumerate() {
        let sources = r.snapshot().exchange(e).unwrap();
        rebuilt.push_str("** question\n");
        rebuilt.push_str(sources.question().unwrap());
        rebuilt.push_str(answer_marker);
        rebuilt.push_str(sources.answer());
        answer_marker = "** reponse\n";
        assert!(sources.is_complete(), "E{}", i + 1);
    }
    assert_eq!(rebuilt.as_bytes(), DOC.as_bytes());
    let e1 = r.snapshot().exchange(exchanges[0]).unwrap().answer();
    let tabs: usize = exchanges
        .iter()
        .map(|&e| {
            r.snapshot()
                .exchange(e)
                .unwrap()
                .answer()
                .matches('\t')
                .count()
        })
        .sum();
    assert_eq!(
        tabs, 30,
        "tableaux devenus du texte tabulé, conservés tels quels"
    );
    assert!(e1.contains("01-LM3S9B92-DataSheet.pdf\u{2060}\u{fffc}"));
    assert!(r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .edges()
        .next()
        .is_none());

    // Étape 5 : réserves depuis trois passages de la réponse E1.
    let passage = |r: &mut Harness, text: &str| {
        let start = e1_answer(r, exchanges[0]).find(text).unwrap();
        r.create_passage(
            DEFAULT_SCOPE,
            exchanges[0],
            Part::Answer,
            start,
            start + text.len(),
        )
        .unwrap()
    };
    let p_lost = passage(&mut r, "C’est là que peut se produire un réveil perdu.");
    let p_ack = passage(
        &mut r,
        "sur ce composant, GPIOICR acquitte les bits auxquels on écrit 1",
    );
    let p_tick = passage(
        &mut r,
        "une tâche peut attendre presque une période de tick après l’interruption avant d’être rendue prête",
    );
    let wake = note(
        &mut r,
        Kind::Implementation,
        "Le driver réveille les tâches",
    );
    let handshake = note(
        &mut r,
        Kind::Implementation,
        "Événement conservé, poignée de main",
    );
    let r1 = r
        .create_reserve(
            DEFAULT_SCOPE,
            p_lost,
            "Réveil perdu entre vérification et blocage",
            None,
        )
        .unwrap();
    r.link(DEFAULT_SCOPE, r1, handshake, Relation::AppliesTo)
        .unwrap();
    r.treat(DEFAULT_SCOPE, handshake, r1).unwrap();
    let r2 = r
        .create_reserve(
            DEFAULT_SCOPE,
            p_ack,
            "Acquittement GPIOICR : bits écrits à 1",
            Some("startup.s (à créer)"),
        )
        .unwrap();
    let r3 = r
        .create_reserve(
            DEFAULT_SCOPE,
            p_tick,
            "Latence d’un tick : paramètre à fixer",
            None,
        )
        .unwrap();
    r.link(DEFAULT_SCOPE, r3, wake, Relation::AppliesTo)
        .unwrap();

    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    let state = |id| g.node(id).unwrap().reserve_status();
    assert_eq!(state(r1), Some(ReserveStatus::TreatedPendingVerification));
    assert_eq!(state(r2), Some(ReserveStatus::Open));
    assert_eq!(state(r3), Some(ReserveStatus::Open));
    assert_eq!(r.snapshot().reserve(r2).unwrap().exchange, exchanges[0]);
    assert_eq!(
        r.snapshot().reserve(r2).unwrap().target,
        Some("startup.s (à créer)")
    );
    assert!(g.edges().all(|(_, e)| e.from() != r2 && e.to() != r2));
    assert_eq!(
        g.node(p_ack).unwrap().text(),
        "sur ce composant, GPIOICR acquitte les bits auxquels on écrit 1"
    );
    assert_eq!(
        r.snapshot().exchange(exchanges[0]).unwrap().answer(),
        parts[0].1
    );
}

fn e1_answer(r: &Harness, exchange: NodeId) -> String {
    r.snapshot().exchange(exchange).unwrap().answer().to_owned()
}

// ---------------------------------------------------------------------------
// Incrément 2.6a — contrat ajouté : ordre des conversations, acquittement, retrait de relation.
// Remplace le défaut S8. Deux constructions directes d'échange et la politique de criticité
// sont modifiées, signalées en commentaire.
// ---------------------------------------------------------------------------

// CA-01 : l'ordre des échanges d'une conversation est donné explicitement et se retrouve ;
// il ne crée aucune relation. Une conversation peut se ramifier.
#[test]
fn conversation_order_is_kept_explicitly() {
    let mut r = Harness::new();
    let e1 = r
        .create_exchange(DEFAULT_SCOPE, "rôle du driver", Some("q1"), "a1")
        .unwrap();
    let e2 = r.continue_exchange(DEFAULT_SCOPE, e1, "q2", "a2").unwrap();
    let e3 = r.continue_exchange(DEFAULT_SCOPE, e2, "q3", "a3").unwrap();
    assert_eq!(r.snapshot().conversation(e3), Ok(vec![e1, e2, e3]));
    assert_eq!(r.snapshot().conversation(e1), Ok(vec![e1]));
    assert_eq!(r.snapshot().exchange(e2).unwrap().previous(), Some(e1));
    assert_eq!(r.snapshot().exchange(e1).unwrap().previous(), None);
    let branch = r
        .continue_exchange(DEFAULT_SCOPE, e1, "q2 bis", "a2 bis")
        .unwrap();
    assert_eq!(r.snapshot().conversation(branch), Ok(vec![e1, branch]));
    assert!(r
        .snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .edges()
        .next()
        .is_none());

    let n = note(&mut r, Kind::Reasoning, "note");
    let ghost = r
        .create_exchange(DEFAULT_SCOPE, "fantôme", None, "a")
        .unwrap();
    r.undo().unwrap();
    let control = r.clone();
    assert_eq!(
        r.continue_exchange(DEFAULT_SCOPE, n, "q", "a"),
        Err(Error::WrongKind(n))
    );
    assert_eq!(
        r.continue_exchange(DEFAULT_SCOPE, ghost, "q", "a"),
        Err(Error::MissingNode(ghost))
    );
    assert_eq!(r.snapshot().conversation(n), Err(Error::WrongKind(n)));
    assert_eq!(r, control);
}

// INV-ACK-01 (remplace S8) : acquitter lève la cause sans supprimer la provenance ; la cause ne
// se transmet plus au-delà du dépendant confirmé ; l'acquittement devient caduc si le sens du
// dépendant ou l'état de la cause change ; il est propre au contexte et s'annule.
#[test]
fn acknowledgment_lifts_review_without_destroying_provenance() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "A : scheduler centralisé");
    let n1 = note(
        &mut r,
        Kind::Implementation,
        "N1 : représentation des tâches",
    );
    let n2 = note(&mut r, Kind::Implementation, "N2 : s'appuie sur N1");
    let provenance = r
        .link(DEFAULT_SCOPE, n1, a, Relation::DevelopedFrom)
        .unwrap();
    r.link(DEFAULT_SCOPE, n2, n1, Relation::DependsOn).unwrap();
    r.archive(DEFAULT_SCOPE, a).unwrap();
    let other = r.clone_scope(DEFAULT_SCOPE).unwrap();
    let causes = |r: &Harness, scope| r.snapshot().graph(scope).unwrap().review_causes();
    let initial = BTreeMap::from([(n1, BTreeSet::from([a])), (n2, BTreeSet::from([a]))]);
    assert_eq!(causes(&r, DEFAULT_SCOPE), initial);

    let control = r.clone();
    assert_eq!(r.acknowledge(DEFAULT_SCOPE, a, n1), Err(Error::NoSuchCause));
    assert_eq!(
        r.acknowledge(DEFAULT_SCOPE, n1, n2),
        Err(Error::NoSuchCause)
    );
    assert_eq!(r, control);

    r.acknowledge(DEFAULT_SCOPE, n1, a).unwrap();
    assert!(causes(&r, DEFAULT_SCOPE).is_empty());
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert!(g.edge(provenance).unwrap().active());
    // Contrat modifié (5.5a, R6, D24) : N1, dont l'unique appui A est retiré et la cause
    // acquittée, est orpheline : elle sort des ressources actuelles. N2 s'appuie sur N1, qui n'est
    // pas écartée : elle reste ressource actuelle.
    assert_eq!(g.orphans(), vec![n1]);
    assert!(!g.current_resources().contains(&n1) && g.current_resources().contains(&n2));
    assert_eq!(causes(&r, other), initial, "propre au contexte");

    // Révision mineure du dépendant : rien ne change. Substantielle : la cause revient, et N2
    // doit aussi réexaminer N1, qui a changé de sens.
    r.revise(n1, "N1, coquille corrigée", RevisionKind::Minor)
        .unwrap();
    assert!(causes(&r, DEFAULT_SCOPE).is_empty());
    r.revise(n1, "N1 réécrite", RevisionKind::Substantive)
        .unwrap();
    assert_eq!(
        causes(&r, DEFAULT_SCOPE),
        BTreeMap::from([(n1, BTreeSet::from([a])), (n2, BTreeSet::from([a, n1]))])
    );
    r.undo().unwrap();
    assert!(causes(&r, DEFAULT_SCOPE).is_empty());

    // La cause change d'état : l'acquittement ne vaut plus.
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    assert_eq!(causes(&r, DEFAULT_SCOPE), initial);
    r.undo().unwrap();
    assert!(causes(&r, DEFAULT_SCOPE).is_empty());

    // Annuler l'acquittement rétablit la cause.
    r.undo().unwrap();
    r.undo().unwrap();
    assert_eq!(causes(&r, DEFAULT_SCOPE), initial);
}

// D12 et D10 : acquitter et retirer une relation exigent un motif.
#[test]
fn acknowledgment_and_detachment_require_a_reason() {
    let mut repo = Repository::new();
    let (a, b, e) = (
        NodeId::from_bytes(raw(1)),
        NodeId::from_bytes(raw(2)),
        EdgeId::from_bytes(raw(3)),
    );
    let archive = Command::Change(Change::Archive {
        scope: DEFAULT_SCOPE,
        node: a,
    });
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Reasoning)),
        (11, create(DEFAULT_SCOPE, 2, Kind::Reasoning)),
        (12, link(3, 2, 1, Relation::DependsOn)),
        (13, archive),
    ] {
        let next = envelope(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let control = repo.clone();
    let commands = [
        Change::Acknowledge {
            scope: DEFAULT_SCOPE,
            dependent: b,
            cause: a,
        },
        Change::Detach {
            scope: DEFAULT_SCOPE,
            edge: e,
        },
    ];
    for (op, change) in (20..).zip(commands) {
        let without = envelope(&repo, op, Command::Change(change));
        assert_eq!(repo.execute(without), Err(Error::MissingReason));
        assert_eq!(repo, control);
    }
}

// Retrait d'une relation : elle reste dans l'historique, inactive ; ses effets sont recalculés ;
// le statut posé par un remplacement ou un traitement est rétabli.
#[test]
fn detaching_a_relation_keeps_history_and_restores_prior_states() {
    let mut r = Harness::new();
    let causes = |r: &Harness| r.snapshot().graph(DEFAULT_SCOPE).unwrap().review_causes();

    // Dépendance : la retirer lève la cause.
    let a = note(&mut r, Kind::Reasoning, "a");
    let b = note(&mut r, Kind::Implementation, "b");
    let e = r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    r.archive(DEFAULT_SCOPE, a).unwrap();
    assert_eq!(causes(&r), BTreeMap::from([(b, BTreeSet::from([a]))]));
    r.detach(DEFAULT_SCOPE, e).unwrap();
    assert!(causes(&r).is_empty());
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert!(!g.edge(e).unwrap().active());
    let control = r.clone();
    assert_eq!(r.detach(DEFAULT_SCOPE, e), Err(Error::InactiveEdge));
    assert_eq!(r, control);

    // Remplacement : « retenue » avant, « remplacée » pendant, « retenue » après le retrait.
    // Un seul remplacement actif à la fois.
    let old = note(&mut r, Kind::Implementation, "structure A");
    let new = note(&mut r, Kind::Implementation, "structure B");
    let other = note(&mut r, Kind::Implementation, "structure C");
    r.set_decision(DEFAULT_SCOPE, old, Decision::Retained)
        .unwrap();
    let replacement = r.replace(DEFAULT_SCOPE, new, old).unwrap();
    let control = r.clone();
    assert_eq!(
        r.replace(DEFAULT_SCOPE, other, old),
        Err(Error::ReservedStatus)
    );
    assert_eq!(r, control);
    r.detach(DEFAULT_SCOPE, replacement).unwrap();
    let g = r.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert_eq!(g.node(old).unwrap().decision(), Some(Decision::Retained));
    assert!(g.replaced_by(old).is_empty());
    assert_eq!(g.edge(replacement).unwrap().relation(), Relation::Replaces);

    // Traitement : « traitée à vérifier » redevient « ouverte » ; une résolution confirmée
    // entre-temps n'est pas défaite.
    let exchange = r
        .create_exchange(DEFAULT_SCOPE, "t", None, "réponse")
        .unwrap();
    let passage = r
        .create_passage(DEFAULT_SCOPE, exchange, Part::Answer, 0, 3)
        .unwrap();
    let reserve = r
        .create_reserve(DEFAULT_SCOPE, passage, "réserve", None)
        .unwrap();
    let status = |r: &Harness| {
        r.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(reserve)
            .unwrap()
            .reserve_status()
    };
    let first = r.treat(DEFAULT_SCOPE, b, reserve).unwrap();
    r.detach(DEFAULT_SCOPE, first).unwrap();
    assert_eq!(status(&r), Some(ReserveStatus::Open));
    let second = r.treat(DEFAULT_SCOPE, b, reserve).unwrap();
    r.set_reserve_status(DEFAULT_SCOPE, reserve, ReserveStatus::Resolved)
        .unwrap();
    r.detach(DEFAULT_SCOPE, second).unwrap();
    assert_eq!(status(&r), Some(ReserveStatus::Resolved));

    // Restriction : la retirer change l'empreinte de la cible ; un dépendant déclaré pendant la
    // restriction est à réexaminer.
    let claim = note(&mut r, Kind::Reasoning, "affirmation");
    let source = note(&mut r, Kind::Reasoning, "source");
    let dependent = note(&mut r, Kind::Implementation, "dépendant");
    let restriction = r
        .restrict(DEFAULT_SCOPE, source, claim, "monocœur")
        .unwrap();
    r.link(DEFAULT_SCOPE, dependent, claim, Relation::DependsOn)
        .unwrap();
    assert!(!causes(&r).contains_key(&dependent));
    r.detach(DEFAULT_SCOPE, restriction).unwrap();
    assert_eq!(causes(&r).get(&dependent), Some(&BTreeSet::from([claim])));
}

// ---------------------------------------------------------------------------
// Incrément 2.6b — contrat ajouté : ancrages de code, résolution des repères, scénario de
// référence jusqu'à la reprise depuis le code. La politique de criticité est complétée.
// ---------------------------------------------------------------------------

const DRIVER_PATH: &str = "src/driver/cortex_arm_m3/evalbot/driver.c";

// INV-CODE-03 : un ancrage conserve chemin, copie et empreinte du code ciblé, et le commit
// quand le code est versionné ; il se retire en restant dans l'historique.
#[test]
fn anchors_keep_path_copy_and_digest() {
    let mut r = Harness::new();
    let n = note(
        &mut r,
        Kind::Implementation,
        "le driver réveille les tâches",
    );
    let code = "void wake_up(int port, int pin) {\n}";
    let unversioned = r.anchor(n, DRIVER_PATH, code, None).unwrap();
    let versioned = r
        .anchor(n, "src/kernel.c", "void tick() {}", Some("a1b2c3d"))
        .unwrap();
    assert!(!r.last_outcome().unwrap().forked);
    let s = r.snapshot();
    let a = s.anchor(unversioned).unwrap();
    assert_eq!(
        (a.node(), a.path(), a.excerpt(), a.git(), a.active()),
        (n, DRIVER_PATH, code, None, true)
    );
    assert_eq!(a.digest(), text_digest(code));
    assert_ne!(
        text_digest(code),
        text_digest("void wake_up(int port, int pin) {\n }")
    );
    assert_eq!(s.anchor(versioned).unwrap().git(), Some("a1b2c3d"));
    let ids: Vec<AnchorId> = s.anchors_of(n).iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![unversioned, versioned]);

    let ghost = note(&mut r, Kind::Implementation, "fantôme");
    r.undo().unwrap();
    let control = r.clone();
    for (path, excerpt, git) in [
        ("", code, None),
        (DRIVER_PATH, " \n", None),
        (DRIVER_PATH, code, Some(" ")),
    ] {
        assert_eq!(r.anchor(n, path, excerpt, git), Err(Error::InvalidAnchor));
    }
    assert_eq!(
        r.anchor(ghost, DRIVER_PATH, code, None),
        Err(Error::MissingNode(ghost))
    );
    assert_eq!(r.repository(), control.repository());

    r.unanchor(unversioned).unwrap();
    let a = r.snapshot().anchor(unversioned).unwrap().clone();
    assert!(!a.active());
    assert_eq!(a.excerpt(), code);
    let before = r.repository().clone();
    r.unanchor(unversioned).unwrap();
    assert!(!r.last_outcome().unwrap().published);
    assert_eq!(r.repository(), &before);
}

// INV-CODE-05 : un repère se résout par l'identité qu'il porte, quel que soit le fichier où il
// est trouvé ; le chemin enregistré, devenu périmé, est rapporté tel quel, jamais deviné.
// CDC : si la note n'existe que dans une autre branche, on indique où la trouver.
#[test]
fn markers_resolve_independently_of_paths() {
    let mut r = Harness::new();
    let n = note(
        &mut r,
        Kind::Implementation,
        "contrat refresh() appelé par tick()",
    );
    r.anchor(n, "src/engine.c", "refresh();", None).unwrap();
    let marker = r.marker(DEFAULT_SCOPE, n).unwrap();
    // Le repère est retrouvé dans kernel.c, après renommage d'engine.c.
    let line_in_kernel = format!("    {marker}   ");
    assert_eq!(
        r.repository()
            .resolve_marker(DEFAULT_SCOPE, &line_in_kernel),
        Ok(n)
    );
    let paths: Vec<String> = r
        .snapshot()
        .anchors_of(n)
        .iter()
        .map(|(_, a)| a.path().to_owned())
        .collect();
    assert_eq!(paths, ["src/engine.c"]);
    for garbage in [
        "// voir dans doc.md pourquoi cette decision",
        "// @design: 1234",
        "",
    ] {
        assert_eq!(
            r.repository().resolve_marker(DEFAULT_SCOPE, garbage),
            Err(Error::NotAMarker)
        );
    }

    // Note présente seulement sur une branche expérimentale.
    let a = note(&mut r, Kind::Reasoning, "a");
    let experimental = r.invalidate(DEFAULT_SCOPE, a).unwrap();
    let only_there = note(&mut r, Kind::Reasoning, "créée sur la branche");
    let elsewhere = r.marker(DEFAULT_SCOPE, only_there).unwrap();
    r.checkout(MAIN).unwrap();
    assert_eq!(
        r.repository().resolve_marker(DEFAULT_SCOPE, &elsewhere),
        Err(Error::MissingNode(only_there))
    );
    // Contrat modifié (incrément 5.2b) : la recherche lit l'état courant des autres branches
    // dans l'historique, et peut donc échouer.
    assert_eq!(
        r.repository().branches_containing(only_there),
        Ok(vec![experimental])
    );
    assert_eq!(
        r.repository().branches_containing(n),
        Ok({
            let mut both = vec![MAIN, experimental];
            both.sort();
            both
        })
    );
}

// INV-RET-01 étendu aux ancrages : une restauration conserve les ancrages créés depuis la cible,
// retirés ; l'annuler les rend actifs.
#[test]
fn restore_keeps_later_anchors_inactive() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Implementation, "n");
    let target = r.head();
    let anchor = r.anchor(n, DRIVER_PATH, "void refresh() {}", None).unwrap();
    r.revise(n, "n révisée", RevisionKind::Substantive).unwrap();
    r.restore(target).unwrap();
    let a = r.snapshot().anchor(anchor).unwrap();
    assert!(!a.active());
    assert_eq!(a.excerpt(), "void refresh() {}");
    r.undo().unwrap();
    assert!(r.snapshot().anchor(anchor).unwrap().active());
}

/// Texte d'une fonction C du jeu d'essai, de sa signature à son accolade fermante.
fn c_function<'a>(code: &'a str, signature: &str) -> &'a str {
    let start = code.find(signature).unwrap();
    let end = start + code[start..].find("\n}").unwrap() + 2;
    &code[start..end]
}

/// Passage de la réponse d'un échange, retrouvé par son texte exact.
fn passage_of(r: &mut Harness, exchange: NodeId, text: &str) -> NodeId {
    let start = r
        .snapshot()
        .exchange(exchange)
        .unwrap()
        .answer()
        .find(text)
        .unwrap();
    r.create_passage(
        DEFAULT_SCOPE,
        exchange,
        Part::Answer,
        start,
        start + text.len(),
    )
    .unwrap()
}

/// Origines des relations actives d'un type donné vers une note.
fn sources_of(g: &Graph, to: NodeId, relation: Relation) -> BTreeSet<NodeId> {
    g.edges()
        .filter(|(_, e)| e.active() && e.relation() == relation && e.to() == to)
        .map(|(_, e)| e.from())
        .collect()
}

// S-REF-01, étapes 1 à 9, sur les sources réelles figées. « Citer » une source est réalisé par
// « dépend de » : une note s'appuie sur les passages qu'elle cite, et doit être réexaminée s'ils
// sont corrigés. Contrat complété par D13 (incrément 2.6c) : les étapes 8 et 9, qui attendaient
// une décision sur la criticité, sont ajoutées ; le test est renommé en conséquence.
#[test]
fn reference_scenario_steps_1_to_9_on_real_sources() {
    const DOC: &str = include_str!("../../../fixtures/rtos-2026-10/doc.md");
    const DRIVER: &str = include_str!("../../../fixtures/rtos-2026-10/driver.c");
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();

    // Étape 1 : cinq échanges, chaînés dans l'ordre de la conversation.
    let mut exchanges: Vec<NodeId> = Vec::new();
    for (question, answer) in split_exchanges(DOC) {
        let id = match exchanges.last() {
            None => r.create_exchange(S, "rôle du driver", Some(question), answer),
            Some(&previous) => r.continue_exchange(S, previous, question, answer),
        };
        exchanges.push(id.unwrap());
    }
    assert_eq!(
        r.snapshot().conversation(exchanges[4]),
        Ok(exchanges.clone())
    );
    let (e1, e2, e3, e4, e5) = (
        exchanges[0],
        exchanges[1],
        exchanges[2],
        exchanges[3],
        exchanges[4],
    );

    // Étape 2 : organisation, sans effet sur les statuts ; cycle de catégories refusé.
    let c_drv = note(&mut r, Kind::Category, "Driver GPIO : réveil des tâches");
    let c_exp = note(&mut r, Kind::Category, "Exploration / évolution");
    let c_mc = note(&mut r, Kind::Category, "Exploration multicœur");
    r.link(S, c_mc, c_exp, Relation::ClassifiedIn).unwrap();
    for (e, c) in [
        (e1, c_drv),
        (e2, c_drv),
        (e3, c_drv),
        (e3, c_mc),
        (e4, c_mc),
        (e5, c_mc),
    ] {
        r.link(S, e, c, Relation::ClassifiedIn).unwrap();
    }
    assert!(r.snapshot().graph(S).unwrap().review_causes().is_empty());
    let control = r.clone();
    assert_eq!(
        r.link(S, c_exp, c_mc, Relation::ClassifiedIn),
        Err(Error::CyclicRelation)
    );
    assert_eq!(r, control);

    // Étape 3 : P-E4 restreint P-E3c à « plusieurs ordonnanceurs », sans verdict.
    let p_e3c = passage_of(
        &mut r,
        e3,
        "En multicœur, cependant, une écriture atomique de l’état ne suffit pas à garantir ce transfert.",
    );
    let p_e4 = passage_of(
        &mut r,
        e4,
        "Ma réponse précédente supposait plusieurs cœurs capables d’ordonnancer simultanément.",
    );
    let restriction = r
        .restrict(S, p_e4, p_e3c, "plusieurs ordonnanceurs")
        .unwrap();

    // Étape 4 : décisions retenues, sources citées, désaccord assumé.
    let p_e2a = passage_of(
        &mut r,
        e2,
        "Ton idée d’organiser les attentes par port, puis par broche, est pertinente.",
    );
    let p_e3a = passage_of(
        &mut r,
        e3,
        "Une tâche bloquée peut être référencée directement dans les structures d’attente du périphérique, puis rendue à l’ordonnanceur lors du réveil.",
    );
    let p_e3b = passage_of(
        &mut r,
        e3,
        "le gestionnaire d’attente identifie la tâche à réveiller, puis une primitive commune réalise le transfert vers PENDING.",
    );
    let n_drv1 = r
        .create_node(
            S,
            Kind::Implementation,
            "N-DRV-1",
            "Le driver réveille les tâches par écriture directe de l’état",
        )
        .unwrap();
    r.set_decision(S, n_drv1, Decision::Retained).unwrap();
    r.link(S, n_drv1, c_drv, Relation::ClassifiedIn).unwrap();
    r.link(S, n_drv1, p_e2a, Relation::DependsOn).unwrap();
    r.link(S, n_drv1, p_e3a, Relation::DependsOn).unwrap();
    let disagreement = r
        .contradict(S, n_drv1, p_e3b, "monocœur, section critique courte")
        .unwrap();
    let n_drv2 = r
        .create_node(
            S,
            Kind::Implementation,
            "N-DRV-2",
            "Un événement arrivé avant l’attente est conservé ; il est consommé par la poignée de main",
        )
        .unwrap();
    r.set_decision(S, n_drv2, Decision::Retained).unwrap();
    r.set_implementation(S, n_drv2, Implementation::InProgress)
        .unwrap();
    r.link(S, n_drv2, c_drv, Relation::ClassifiedIn).unwrap();
    r.link(S, n_drv2, n_drv1, Relation::DependsOn).unwrap();
    {
        let g = r.snapshot().graph(S).unwrap();
        let resources = g.current_resources();
        assert!(resources.contains(&n_drv1) && resources.contains(&n_drv2));
        assert!(g.review_causes().is_empty());
        let target = g.node(p_e3b).unwrap();
        assert_eq!(
            (target.verdict(), target.decision()),
            (Some(Verdict::Unverified), Some(Decision::Proposed))
        );
    }

    // Étape 5 : réserves.
    let p_lost = passage_of(&mut r, e1, "C’est là que peut se produire un réveil perdu.");
    let p_ack = passage_of(
        &mut r,
        e1,
        "sur ce composant, GPIOICR acquitte les bits auxquels on écrit 1",
    );
    let p_tick = passage_of(
        &mut r,
        e1,
        "une tâche peut attendre presque une période de tick après l’interruption avant d’être rendue prête",
    );
    let r1 = r
        .create_reserve(
            S,
            p_lost,
            "Réveil perdu entre vérification et blocage",
            None,
        )
        .unwrap();
    r.link(S, r1, n_drv2, Relation::AppliesTo).unwrap();
    r.treat(S, n_drv2, r1).unwrap();
    let r2 = r
        .create_reserve(
            S,
            p_ack,
            "Acquittement GPIOICR : bits écrits à 1",
            Some("startup.s (à créer)"),
        )
        .unwrap();
    let r3 = r
        .create_reserve(S, p_tick, "Latence d’un tick : paramètre à fixer", None)
        .unwrap();
    r.link(S, r3, n_drv1, Relation::AppliesTo).unwrap();

    // Étape 6 : repères générés, distincts, porteurs de la seule identité ; ancrages avec copie
    // et empreinte, sans commit (code non versionné).
    let wake = c_function(DRIVER, "void wake_up(int port, int pin)");
    let refresh = c_function(DRIVER, "void refresh()");
    let a1 = r.anchor(n_drv1, DRIVER_PATH, wake, None).unwrap();
    let a2 = r.anchor(n_drv2, DRIVER_PATH, refresh, None).unwrap();
    let (m1, m2) = (r.marker(S, n_drv1).unwrap(), r.marker(S, n_drv2).unwrap());
    assert_ne!(m1, m2);
    assert_eq!(m1, format!("{MARKER_PREFIX}{n_drv1}"));
    for (anchor, text) in [(a1, wake), (a2, refresh)] {
        let a = r.snapshot().anchor(anchor).unwrap();
        assert_eq!((a.path(), a.excerpt(), a.git()), (DRIVER_PATH, text, None));
        assert_eq!(a.digest(), text_digest(text));
    }

    // Étape 7 : reprise depuis le repère de wake_up().
    let found = r.repository().resolve_marker(S, &m1).unwrap();
    assert_eq!(found, n_drv1);
    let g = r.snapshot().graph(S).unwrap();
    // 1. Pourquoi : sources citées et désaccord assumé, avec sa justification.
    let cited: BTreeSet<NodeId> = g
        .edges()
        .filter(|(_, e)| e.active() && e.from() == found && e.relation() == Relation::DependsOn)
        .map(|(_, e)| e.to())
        .collect();
    assert_eq!(cited, BTreeSet::from([p_e2a, p_e3a]));
    let contradiction = g.edge(disagreement).unwrap();
    assert_eq!(
        (contradiction.to(), contradiction.annotation()),
        (p_e3b, Some("monocœur, section critique courte"))
    );
    // 2. Encore retenue, sans cause de réexamen.
    assert_eq!(g.node(found).unwrap().decision(), Some(Decision::Retained));
    assert!(!g.review_causes().contains_key(&found));
    // 3. Réserves : directes, puis celles de la catégorie C-DRV.
    let direct = sources_of(g, found, Relation::AppliesTo);
    assert_eq!(direct, BTreeSet::from([r3]));
    assert_eq!(
        g.node(r3).unwrap().reserve_status(),
        Some(ReserveStatus::Open)
    );
    let members = sources_of(g, c_drv, Relation::ClassifiedIn);
    assert_eq!(members, BTreeSet::from([e1, e2, e3, n_drv1, n_drv2]));
    let in_category: BTreeSet<NodeId> = g
        .open_reserves()
        .into_iter()
        .filter(|&reserve| {
            let origin = r.snapshot().reserve(reserve).unwrap().exchange;
            let targets: BTreeSet<NodeId> = g
                .edges()
                .filter(|(_, e)| e.active() && e.from() == reserve)
                .map(|(_, e)| e.to())
                .collect();
            !direct.contains(&reserve)
                && (members.contains(&origin) || !targets.is_disjoint(&members))
        })
        .collect();
    assert_eq!(in_category, BTreeSet::from([r1, r2]));
    assert_eq!(
        g.node(r1).unwrap().reserve_status(),
        Some(ReserveStatus::TreatedPendingVerification)
    );
    assert_eq!(
        sources_of(g, r1, Relation::Treats),
        BTreeSet::from([n_drv2])
    );
    assert_eq!(
        r.snapshot().reserve(r2).unwrap().target,
        Some("startup.s (à créer)")
    );
    assert_eq!(
        g.node(n_drv2).unwrap().implementation(),
        Some(Implementation::InProgress)
    );
    // 4. Ce qui ne vaut qu'en monocœur : justification, restriction et exploration multicœur.
    assert_eq!(
        g.edge(restriction).unwrap().annotation(),
        Some("plusieurs ordonnanceurs")
    );
    assert_eq!(
        sources_of(g, p_e3c, Relation::RestrictsScope),
        BTreeSet::from([p_e4])
    );
    assert_eq!(
        sources_of(g, c_mc, Relation::ClassifiedIn),
        BTreeSet::from([e3, e4, e5])
    );

    // Sous D13, les étapes 1 à 7 n'ont créé aucune cause ni écarté aucune note : tout est sur main.
    assert_eq!(r.active_branch(), MAIN);
    let step7 = r.snapshot().clone();
    let main_head = r.head();
    let reserve_states = |r: &Harness| {
        let g = r.snapshot().graph(S).unwrap();
        [r1, r2, r3].map(|id| g.node(id).unwrap().reserve_status())
    };
    let reserves_before = reserve_states(&r);

    // Étape 8 : N-DRV-1 n'est plus retenue ; une branche est créée depuis main.
    let branch = r
        .set_decision(S, n_drv1, Decision::NoLongerRetained)
        .unwrap();
    assert!(r.last_outcome().unwrap().forked);
    assert_ne!(branch, MAIN);
    assert_eq!(r.branch_head(MAIN), Ok(main_head));
    let founding = r.head();
    {
        let g = r.snapshot().graph(S).unwrap();
        assert!(!g.current_resources().contains(&n_drv1));
        assert_eq!(
            g.node(n_drv1).unwrap().decision(),
            Some(Decision::NoLongerRetained)
        );
        assert_eq!(
            g.review_causes().get(&n_drv2),
            Some(&BTreeSet::from([n_drv1]))
        );
        let dependent = g.node(n_drv2).unwrap();
        assert_eq!(
            (dependent.decision(), dependent.verdict()),
            (Some(Decision::Retained), Some(Verdict::Unverified))
        );
        assert!(sources_of(g, n_drv1, Relation::DependsOn).contains(&n_drv2));
    }
    assert_eq!(r.repository().resolve_marker(S, &m1), Ok(n_drv1));
    assert_eq!(reserve_states(&r), reserves_before);

    // Étape 9 : acquittement, révision substantielle de la cause, puis retour arrière.
    r.acknowledge(S, n_drv2, n_drv1).unwrap();
    assert_eq!(r.active_branch(), branch);
    {
        let g = r.snapshot().graph(S).unwrap();
        // Contrat modifié (5.5a, R6, D24) : N-DRV-2 n'est plus à réexaminer ; son unique appui,
        // N-DRV-1, est « plus retenue » : elle est orpheline et sort des ressources actuelles.
        assert!(!g.current_resources().contains(&n_drv2));
        assert_eq!(g.orphans(), vec![n_drv2]);
        assert!(!g.review_causes().contains_key(&n_drv2));
    }
    r.revise(
        n_drv1,
        "Primitive commune du noyau",
        RevisionKind::Substantive,
    )
    .unwrap();
    assert_eq!(
        r.snapshot().graph(S).unwrap().review_causes().get(&n_drv2),
        Some(&BTreeSet::from([n_drv1]))
    );
    for _ in 0..3 {
        r.undo().unwrap();
    }
    assert_eq!(r.snapshot(), &step7);
    assert_eq!(r.active_branch(), MAIN);
    assert_eq!(r.branch_head(branch), Ok(founding));
}

// ---------------------------------------------------------------------------
// Incrément 2.7 — compléments après mutation (aucun test existant modifié).
// ---------------------------------------------------------------------------

// INV-ID-02 étendu aux ancrages : un identifiant d'ancrage n'est jamais réattribué, même après
// annulation ; un identifiant nul est refusé.
#[test]
fn an_anchor_identity_is_never_reused() {
    let mut repo = Repository::new();
    let n = NodeId::from_bytes(raw(1));
    let anchor = |id: [u8; 16]| {
        Command::Change(Change::AnchorToCode {
            id: AnchorId::from_bytes(id),
            node: n,
            path: "src/kernel.c".into(),
            excerpt: "void tick() {}".into(),
            git: None,
        })
    };
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Implementation)),
        (11, anchor(raw(2))),
        (12, Command::Undo),
    ] {
        let next = envelope(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let control = repo.clone();
    let reuse = envelope(&repo, 13, anchor(raw(2)));
    assert_eq!(
        repo.execute(reuse),
        Err(Error::AnchorIdInUse(AnchorId::from_bytes(raw(2))))
    );
    let nil = envelope(&repo, 14, anchor([0; 16]));
    assert_eq!(repo.execute(nil), Err(Error::NilIdentifier));
    assert_eq!(repo, control);
    let fresh = envelope(&repo, 15, anchor(raw(3)));
    assert!(repo.execute(fresh).unwrap().published);
}

// Retrait d'un traitement quand plusieurs notes traitent la même réserve : la réserve reste
// « traitée à vérifier » tant qu'un traitement demeure actif, et ne rouvre qu'au dernier retrait.
#[test]
fn a_reserve_treated_twice_reopens_only_when_no_treatment_remains() {
    let mut r = Harness::new();
    let exchange = r
        .create_exchange(DEFAULT_SCOPE, "t", None, "réponse")
        .unwrap();
    let passage = r
        .create_passage(DEFAULT_SCOPE, exchange, Part::Answer, 0, 3)
        .unwrap();
    let reserve = r
        .create_reserve(DEFAULT_SCOPE, passage, "réserve", None)
        .unwrap();
    let first_note = note(&mut r, Kind::PersonalNote, "première proposition");
    let second_note = note(&mut r, Kind::Reasoning, "seconde proposition");
    let status = |r: &Harness| {
        r.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .node(reserve)
            .unwrap()
            .reserve_status()
    };
    // Une autre relation active, sans état mémorisé, vise déjà la réserve : seul un traitement
    // actif doit transmettre l'état d'avant tout traitement.
    r.link(DEFAULT_SCOPE, first_note, reserve, Relation::Related)
        .unwrap();
    let first = r.treat(DEFAULT_SCOPE, first_note, reserve).unwrap();
    let second = r.treat(DEFAULT_SCOPE, second_note, reserve).unwrap();
    r.detach(DEFAULT_SCOPE, first).unwrap();
    assert_eq!(status(&r), Some(ReserveStatus::TreatedPendingVerification));
    r.detach(DEFAULT_SCOPE, second).unwrap();
    assert_eq!(status(&r), Some(ReserveStatus::Open));
}

// INV-PREVIEW-01 : l'aperçu d'un rétablissement annonce les causes que l'état rétabli contient ;
// il est identique au résultat de l'exécution.
#[test]
fn preview_of_redo_announces_the_causes_it_restores() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "hypothèse");
    let b = note(&mut r, Kind::Implementation, "s'appuie sur l'hypothèse");
    r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    r.undo().unwrap();
    assert_eq!(r.active_branch(), MAIN);
    let preview = r.repository().preview(&Command::Redo).unwrap();
    assert!(!preview.publishes && !preview.forks);
    assert_eq!(
        preview.effects.review,
        BTreeMap::from([(
            (DEFAULT_SCOPE, b),
            ReviewChange {
                before: BTreeSet::new(),
                after: BTreeSet::from([a])
            }
        )])
    );
    r.redo().unwrap();
    assert_eq!(r.last_outcome().unwrap().effects, preview.effects);
}

// ---------------------------------------------------------------------------
// Incrément 3.1 — contrat ajouté : format textuel des instantanés (FORMAT.md, D15).
// ---------------------------------------------------------------------------

const EXAMPLE_V1: &str = include_str!("../../../fixtures/format/exemple-v1.txt");

/// État de l'exemple de référence (FORMAT.md, section 9).
fn example_state() -> Snapshot {
    let mut repo = Repository::new();
    let n = |k| NodeId::from_bytes(raw(k));
    let answer = "Élément\tRôle\nfin \"cité\" \\ 🚀\u{2060}\u{fffc}";
    let changes = vec![
        Change::CreateExchange {
            scope: DEFAULT_SCOPE,
            id: n(1),
            title: "conversation".into(),
            question: None,
            answer: answer.into(),
            previous: None,
        },
        Change::CreatePassage {
            scope: DEFAULT_SCOPE,
            id: n(2),
            exchange: n(1),
            part: Part::Answer,
            start: 0,
            end: "Élément".len(),
        },
        Change::CreateReserve {
            scope: DEFAULT_SCOPE,
            id: n(3),
            passage: n(2),
            title: "réserve".into(),
            target: Some("startup.s (à créer)".into()),
        },
        Change::CreateNode {
            scope: DEFAULT_SCOPE,
            id: n(4),
            kind: Kind::Implementation,
            title: "N".into(),
            text: "v0".into(),
        },
        Change::Revise {
            node: n(4),
            text: "v1".into(),
            kind: RevisionKind::Minor,
        },
        Change::Link {
            scope: DEFAULT_SCOPE,
            id: EdgeId::from_bytes(raw(5)),
            from: n(4),
            to: n(2),
            relation: Relation::DependsOn,
        },
        Change::AnchorToCode {
            id: AnchorId::from_bytes(raw(6)),
            node: n(4),
            path: "src/driver.c".into(),
            excerpt: "void refresh() {}".into(),
            git: None,
        },
        Change::SetDecision {
            scope: DEFAULT_SCOPE,
            node: n(4),
            decision: Decision::Retained,
        },
    ];
    for (op, change) in (100..).zip(changes) {
        let next = Envelope {
            reason: "motif".into(),
            ..envelope(&repo, op, Command::Change(change))
        };
        repo.execute(next).unwrap();
    }
    repo.snapshot().clone()
}

// FORMAT.md, section 9 : l'exemple de référence est figé octet pour octet, dans les deux sens.
#[test]
fn the_reference_example_is_frozen_in_both_directions() {
    let state = example_state();
    assert_eq!(encode_snapshot(&state), EXAMPLE_V1);
    assert_eq!(decode_snapshot(EXAMPLE_V1), Ok(state));
    assert!(EXAMPLE_V1.starts_with(SNAPSHOT_HEADER));
}

/// État couvrant chaque enregistrement et chaque valeur énumérée du format.
fn rich_state() -> Snapshot {
    rich_harness().snapshot().clone()
}

/// Dépôt dont l'état courant est l'état riche ; complété par l'incrément 3.1b (factorisation,
/// aucune assertion modifiée).
fn rich_harness() -> Harness {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let category = note(&mut r, Kind::Category, "catégorie");
    let reasoning = note(&mut r, Kind::Reasoning, "raisonnement");
    let implementation = note(&mut r, Kind::Implementation, "implémentation");
    let personal = note(&mut r, Kind::PersonalNote, "note personnelle");
    let old = note(&mut r, Kind::Implementation, "remplacée");
    let withdrawn = note(&mut r, Kind::Reasoning, "plus retenue");
    let e1 = r
        .create_exchange(S, "conversation", Some("question ?"), "réponse une")
        .unwrap();
    let e2 = r
        .continue_exchange(S, e1, "suite ?", "réponse deux")
        .unwrap();
    r.create_exchange(S, "incomplet", None, "réponse seule")
        .unwrap();
    let on_question = r.create_passage(S, e1, Part::Question, 0, 8).unwrap();
    let on_answer = r.create_passage(S, e2, Part::Answer, 0, 3).unwrap();
    let treated = r
        .create_reserve(S, on_answer, "traitée", Some("startup.s"))
        .unwrap();
    let resolved = r.create_reserve(S, on_question, "résolue", None).unwrap();
    r.create_reserve(S, on_question, "ouverte", None).unwrap();
    r.revise(reasoning, "r1", RevisionKind::Minor).unwrap();
    r.revise(reasoning, "r2", RevisionKind::Substantive)
        .unwrap();
    r.link(S, implementation, category, Relation::ClassifiedIn)
        .unwrap();
    r.link(S, implementation, reasoning, Relation::DependsOn)
        .unwrap();
    r.link(S, personal, reasoning, Relation::DevelopedFrom)
        .unwrap();
    r.link(S, category, implementation, Relation::Related)
        .unwrap();
    r.link(S, treated, implementation, Relation::AppliesTo)
        .unwrap();
    r.replace(S, implementation, old).unwrap();
    r.restrict(S, on_question, personal, "monocœur").unwrap();
    r.contradict(S, personal, implementation, "désaccord \"assumé\"")
        .unwrap();
    r.treat(S, personal, treated).unwrap();
    r.set_reserve_status(S, resolved, ReserveStatus::Resolved)
        .unwrap();
    let detached = r
        .link(S, personal, category, Relation::ClassifiedIn)
        .unwrap();
    r.detach(S, detached).unwrap();
    r.set_verdict(S, implementation, Verdict::Verified).unwrap();
    r.set_decision(S, implementation, Decision::Retained)
        .unwrap();
    r.set_implementation(S, implementation, Implementation::Implemented)
        .unwrap();
    r.set_implementation(S, personal, Implementation::InProgress)
        .unwrap();
    r.set_decision(S, withdrawn, Decision::NoLongerRetained)
        .unwrap();
    r.archive(S, reasoning).unwrap();
    r.acknowledge(S, implementation, reasoning).unwrap();
    r.anchor(
        implementation,
        "src/driver.c",
        "void wake_up() {}",
        Some("a1b2c3d"),
    )
    .unwrap();
    let retired = r
        .anchor(personal, "src/kernel.c", "\tvoid tick() {}\r\n", None)
        .unwrap();
    r.unanchor(retired).unwrap();
    let other = r.clone_scope(S).unwrap();
    r.invalidate(other, withdrawn).unwrap();
    r
}

// Fidélité et forme canonique unique sur un état qui emploie tout le format.
#[test]
fn every_record_and_value_round_trips() {
    let state = rich_state();
    let text = encode_snapshot(&state);
    let decoded = decode_snapshot(&text).unwrap();
    assert_eq!(decoded, state);
    assert_eq!(encode_snapshot(&decoded), text);
    for word in [
        "contenu ",
        "revision ",
        "echange ",
        "passage ",
        "reserve ",
        "ancrage ",
        "contexte ",
        "membre ",
        "relation ",
        "acquittement ",
        " categorie ",
        " raisonnement ",
        " implementation ",
        " note ",
        " mineure ",
        " substantielle ",
        " non-verifiee ",
        " verifiee ",
        " refutee ",
        " proposee ",
        " retenue ",
        " plus-retenue ",
        " remplacee ",
        " non-commencee ",
        " en-cours ",
        " implementee ",
        " ouverte ",
        " traitee-a-verifier ",
        " resolue ",
        " courante",
        " retiree",
        " classe-dans ",
        " depend-de ",
        " developpe-a-partir-de ",
        " lien ",
        " remplace ",
        " restreint ",
        " contredit ",
        " s-applique-a ",
        " traite ",
        " partie question ",
        " partie reponse ",
        " etat active ",
        " etat inactive ",
        " etat actif ",
        " etat retire ",
        "anterieur decision:",
        "anterieur reserve:",
        " precedent 0",
        " question - ",
        "\\t",
        "\\r",
        "\\n",
        "\\\"",
    ] {
        assert!(text.contains(word), "« {word} » absent du texte encodé");
    }
}

fn refusal(text: &str) -> DecodeError {
    decode_snapshot(text).unwrap_err()
}

/// L'exemple de référence avec une ligne remplacée (numérotée à partir de 1).
fn with_line(n: usize, replacement: &str) -> String {
    let mut lines: Vec<&str> = EXAMPLE_V1.lines().collect();
    lines[n - 1] = replacement;
    lines.join("\n") + "\n"
}

fn line_of(n: usize) -> &'static str {
    EXAMPLE_V1.lines().nth(n - 1).unwrap()
}

// FORMAT.md, section 8 : chaque écart est refusé avec sa ligne et son motif.
#[test]
fn every_deviation_is_refused_with_its_line_and_reason() {
    let e = |line, reason| DecodeError { line, reason };
    let lines = EXAMPLE_V1.lines().count();
    assert_eq!(refusal(""), e(1, DecodeReason::Header));
    assert_eq!(
        refusal(EXAMPLE_V1.trim_end_matches('\n')),
        e(lines, DecodeReason::MissingEnd)
    );
    assert_eq!(
        refusal(&with_line(1, "memoire-instantane 2")),
        e(1, DecodeReason::Header)
    );
    assert_eq!(
        refusal(&EXAMPLE_V1.replace("fin\n", "")),
        e(lines, DecodeReason::MissingEnd)
    );
    assert_eq!(
        refusal(&format!("{EXAMPLE_V1}contexte\n")),
        e(lines + 1, DecodeReason::AfterEnd)
    );
    assert_eq!(
        refusal(&with_line(2, "inconnu a")),
        e(2, DecodeReason::UnknownRecord("inconnu".into()))
    );
    let cases: Vec<(usize, String, DecodeReason)> = vec![
        (
            2,
            line_of(2).replacen(' ', "  ", 1),
            DecodeReason::Value("jeton"),
        ),
        (2, format!("{} ", line_of(2)), DecodeReason::Value("jeton")),
        (
            2,
            line_of(2).replace("echange titre", "Echange titre"),
            DecodeReason::Value("jeton"),
        ),
        (
            2,
            line_of(2).replace("titre", "title"),
            DecodeReason::Field("titre"),
        ),
        (
            2,
            line_of(2).replace("echange titre", "inconnu titre"),
            DecodeReason::Value("rôle"),
        ),
        (
            2,
            format!("{} -", line_of(2)),
            DecodeReason::Field("fin de ligne"),
        ),
        (
            2,
            line_of(2).replace("\"conversation\"", "\"conver\\x\""),
            DecodeReason::Value("échappement"),
        ),
        (
            2,
            line_of(2).replace("texte \"\"", "texte \""),
            DecodeReason::Value("chaîne non fermée"),
        ),
        (
            2,
            line_of(2).replace("\"conversation\"", "\"a\"b"),
            DecodeReason::Value("séparateur"),
        ),
        (
            2,
            line_of(2).replace("titre \"conversation\"", "titre -"),
            DecodeReason::Value("titre"),
        ),
        (
            2,
            line_of(2).replace("a5000000-0000-0000-0000-000000000001", "a5"),
            DecodeReason::Value("identifiant"),
        ),
        (
            5,
            line_of(5).replace("fin 9", "fin 09"),
            DecodeReason::Value("entier"),
        ),
        (
            5,
            line_of(5).replace("partie reponse", "partie corps"),
            DecodeReason::Value("partie"),
        ),
        (
            10,
            line_of(10).replace("empreinte 3f38db1c879fb6fd", "empreinte 3f38db1c879fb6f"),
            DecodeReason::Value("empreinte"),
        ),
        (
            10,
            line_of(10).replace("etat actif", "etat en-cours"),
            DecodeReason::Value("etat"),
        ),
        (
            12,
            line_of(12).replace("visibilite courante", "visibilite cachee"),
            DecodeReason::Value("visibilite"),
        ),
        (
            15,
            line_of(15).replace("decision retenue", "decision retenu"),
            DecodeReason::Value("decision"),
        ),
        (
            16,
            line_of(16).replace("depend-de", "depend"),
            DecodeReason::Value("type"),
        ),
        (
            16,
            line_of(16).replace("anterieur -", "anterieur decision:x"),
            DecodeReason::Value("anterieur"),
        ),
        (
            16,
            line_of(16).replace("etat active", "etat actif"),
            DecodeReason::Value("etat"),
        ),
    ];
    for (n, line, reason) in cases {
        assert_eq!(
            refusal(&with_line(n, &line)),
            e(n, reason.clone()),
            "{line}"
        );
    }
}

// FORMAT.md, sections 6 et 8 : ordre canonique et blocs vérifiés strictement.
#[test]
fn order_and_blocks_are_enforced() {
    let e = |line, reason| DecodeError { line, reason };
    let lines: Vec<&str> = EXAMPLE_V1.lines().collect();
    let rebuild = |order: &[usize]| {
        order
            .iter()
            .map(|&i| lines[i - 1])
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    // Deux notes inversées : la deuxième lue précède la première.
    assert_eq!(
        refusal(&rebuild(&[
            1, 4, 5, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17
        ])),
        e(4, DecodeReason::Order)
    );
    // Révision détachée de sa note.
    assert_eq!(
        refusal(&rebuild(&[
            1, 2, 3, 9, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17
        ])),
        e(4, DecodeReason::OutOfBlock)
    );
    // Données propres en double pour une même note.
    assert_eq!(
        refusal(&rebuild(&[
            1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17
        ])),
        e(4, DecodeReason::OutOfBlock)
    );
    // Ancrage après les contextes.
    assert_eq!(
        refusal(&rebuild(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 16, 10, 17
        ])),
        e(16, DecodeReason::Order)
    );
    // Membre avant son contexte.
    assert_eq!(
        refusal(&rebuild(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 11, 13, 14, 15, 16, 17
        ])),
        e(11, DecodeReason::OutOfBlock)
    );
    // Relation avant un membre du même contexte.
    assert_eq!(
        refusal(&rebuild(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 15, 17
        ])),
        e(16, DecodeReason::Order)
    );
    // Membres en double.
    assert_eq!(
        refusal(&rebuild(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 12, 13, 14, 15, 16, 17
        ])),
        e(13, DecodeReason::Order)
    );
    // Révision rattachée à une autre note que la note ouverte (vérification par la mutation).
    let misplaced = lines[8].replace(
        "a5000000-0000-0000-0000-000000000004",
        "a5000000-0000-0000-0000-000000000001",
    );
    assert_eq!(
        refusal(&with_line(9, &misplaced)),
        e(9, DecodeReason::OutOfBlock)
    );
    // Membre d'une autre portée que le bloc ouvert.
    let foreign = lines[11].replacen(
        "00000000-0000-0000-0000-000000000000",
        "a5000000-0000-0000-0000-000000000009",
        1,
    );
    assert_eq!(
        refusal(&with_line(12, &foreign)),
        e(12, DecodeReason::OutOfBlock)
    );
    // Membre dont la note n'a pas de contenu : signalé à sa ligne.
    let ghost_id = "a5000000-0000-0000-0000-000000000000";
    let ghost = lines[11].replace("a5000000-0000-0000-0000-000000000001", ghost_id);
    assert_eq!(
        refusal(&with_line(12, &ghost)),
        e(
            12,
            DecodeReason::InvalidState(Error::MissingNode(NodeId::parse(ghost_id).unwrap()))
        )
    );
    // État global invalide : plage de passage qui ne reproduit pas son texte (ligne 0).
    let shifted = lines[4].replace("debut 0 fin 9", "debut 2 fin 9");
    assert_eq!(
        refusal(&with_line(5, &shifted)),
        e(0, DecodeReason::InvalidState(Error::InvalidExcerpt))
    );
}

// ---------------------------------------------------------------------------
// Incrément 3.1b — contrat ajouté : format du dépôt complet (FORMAT.md, section 12).
// ---------------------------------------------------------------------------

const REPOSITORY_EXAMPLE_V1: &str = include_str!("../../../fixtures/format/depot-v1.txt");

/// Dépôt de l'exemple de référence : création, dépendance, réfutation qui bifurque, annulation.
fn example_repository() -> Repository {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "hypothèse");
    let b = note(&mut r, Kind::Implementation, "s'appuie sur l'hypothèse");
    r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    r.undo().unwrap();
    r.repository().clone()
}

// FORMAT.md, section 12 : l'exemple de référence du dépôt est figé dans les deux sens.
#[test]
fn the_repository_example_is_frozen_in_both_directions() {
    let repo = example_repository();
    assert_eq!(encode_repository(&repo), REPOSITORY_EXAMPLE_V1);
    assert_eq!(decode_repository(REPOSITORY_EXAMPLE_V1), Ok(repo));
    assert!(REPOSITORY_EXAMPLE_V1.starts_with(REPOSITORY_HEADER));
}

/// Dépôt dont le journal emploie toutes les commandes ; factorisé par l'incrément 5.1 (aucune
/// assertion modifiée).
fn every_command_repository() -> Repository {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = rich_harness();
    let other = r.clone_scope(S).unwrap();
    let late = note(&mut r, Kind::PersonalNote, "créée après la copie");
    r.include(other, late).unwrap();
    let incomplete = r
        .create_exchange(S, "à compléter", None, "réponse")
        .unwrap();
    r.set_question(incomplete, "question \"ajoutée\"").unwrap();
    let target = note(&mut r, Kind::Category, "nouvelle catégorie");
    let edge = r.link(S, late, target, Relation::ClassifiedIn).unwrap();
    let source = note(&mut r, Kind::Category, "autre catégorie");
    r.reattach(S, edge, source).unwrap();
    // Piles de rétablissement non vides : un commit annulé sur la branche expérimentale, et la
    // rentrée dans une branche quittée depuis main.
    r.restore(ROOT).unwrap();
    r.undo().unwrap();
    r.redo().unwrap();
    r.undo().unwrap();
    r.checkout(MAIN).unwrap();
    let fresh = note(&mut r, Kind::Reasoning, "sur main");
    r.invalidate(S, fresh).unwrap();
    r.undo().unwrap();
    assert_eq!(r.active_branch(), MAIN);
    r.repository().clone()
}

// INV-EXPORT-01 : un dépôt dont le journal emploie toutes les commandes se relit à l'identique.
#[test]
fn a_repository_using_every_command_round_trips() {
    let repo = every_command_repository();
    let text = encode_repository(&repo);
    let decoded = decode_repository(&text).unwrap();
    assert_eq!(decoded, repo);
    assert_eq!(encode_repository(&decoded), text);
    for name in [
        "creer-note",
        "reviser",
        "inclure",
        "copier-contexte",
        "lier",
        "verdict",
        "decision",
        "implementation",
        "retirer",
        "reattacher",
        "remplacer",
        "restreindre",
        "contredire",
        "creer-echange",
        "completer-question",
        "creer-passage",
        "creer-reserve",
        "traiter",
        "etat-reserve",
        "acquitter",
        "detacher",
        "ancrer",
        "retirer-ancrage",
        "restaurer",
        "annuler",
        "retablir",
        "basculer",
    ] {
        assert!(
            text.contains(&format!("  commande {name}")),
            "commande « {name} » absente du journal encodé"
        );
    }
    for word in [
        "retablir ",
        " entrer ",
        " commit ",
        "publie oui",
        "publie non",
        "bifurque oui",
        "bifurque non",
        "  effet ",
        "identite note",
        "identite relation",
        "identite contexte",
        "identite ancrage",
        "origine -",
        " parent -",
    ] {
        assert!(text.contains(word), "« {word} » absent du dépôt encodé");
    }
}

fn repository_refusal(text: &str) -> DecodeError {
    decode_repository(text).unwrap_err()
}

/// Numéro (à partir de 1) de la première ligne de l'exemple qui commence par le préfixe donné.
fn repository_line(prefix: &str) -> usize {
    REPOSITORY_EXAMPLE_V1
        .lines()
        .position(|l| l.starts_with(prefix))
        .unwrap()
        + 1
}

// FORMAT.md, sections 8 et 12 : écarts de forme, d'ordre et de bloc refusés et situés.
#[test]
fn repository_deviations_are_refused_with_their_line() {
    let e = |line, reason| DecodeError { line, reason };
    let lines = REPOSITORY_EXAMPLE_V1.lines().count();
    assert_eq!(repository_refusal(""), e(1, DecodeReason::Header));
    assert_eq!(
        repository_refusal(&REPOSITORY_EXAMPLE_V1.replacen(
            "memoire-depot 1",
            "memoire-depot 2",
            1
        )),
        e(1, DecodeReason::Header)
    );
    assert_eq!(
        repository_refusal(REPOSITORY_EXAMPLE_V1.trim_end_matches('\n')),
        e(lines, DecodeReason::MissingEnd)
    );
    assert_eq!(
        repository_refusal(&REPOSITORY_EXAMPLE_V1.replace("\nfin\n", "\n")),
        e(lines, DecodeReason::MissingEnd)
    );
    assert_eq!(
        repository_refusal(&format!("{REPOSITORY_EXAMPLE_V1}active\n")),
        e(lines + 1, DecodeReason::AfterEnd)
    );
    let active = repository_line("active ");
    let first_branch = repository_line("branche ");
    let redo = repository_line("retablir ");
    let identity = repository_line("identite ");
    let commit = repository_line("commit ");
    let operation = repository_line("operation ");
    // En-tête de dépôt et instantané : erreur d'une ligne d'instantané située dans le dépôt.
    let snapshot_header = repository_line("  memoire-instantane 1");
    assert_eq!(snapshot_header, commit + 1);
    let broken =
        REPOSITORY_EXAMPLE_V1.replacen("  memoire-instantane 1", "  memoire-instantane 9", 1);
    assert_eq!(
        repository_refusal(&broken),
        e(commit + 1, DecodeReason::Header)
    );
    let lines_vec: Vec<&str> = REPOSITORY_EXAMPLE_V1.lines().collect();
    let replace_line = |n: usize, new: &str| {
        let mut v = lines_vec.clone();
        v[n - 1] = new;
        v.join("\n") + "\n"
    };
    let remove_line = |n: usize| {
        let mut v = lines_vec.clone();
        v.remove(n - 1);
        v.join("\n") + "\n"
    };
    let swap = |a: usize, b: usize| {
        let mut v = lines_vec.clone();
        v.swap(a - 1, b - 1);
        v.join("\n") + "\n"
    };
    assert_eq!(
        repository_refusal(&replace_line(active, "inconnu")),
        e(active, DecodeReason::UnknownRecord("inconnu".into()))
    );
    assert_eq!(
        repository_refusal(&remove_line(active)),
        e(lines - 1, DecodeReason::Field("active"))
    );
    assert_eq!(
        repository_refusal(&swap(active, first_branch)),
        e(first_branch, DecodeReason::Order)
    );
    let doubled = {
        let mut v = lines_vec.clone();
        v.insert(active, lines_vec[active - 1]);
        v.join("\n") + "\n"
    };
    assert_eq!(
        repository_refusal(&doubled),
        e(active + 1, DecodeReason::Order)
    );
    assert_eq!(
        repository_refusal(&swap(identity, commit)),
        // Le commit arrive sans son instantané indenté : refus à la ligne qui devait l'ouvrir.
        e(identity + 1, DecodeReason::Header)
    );
    assert_eq!(
        repository_refusal(&replace_line(
            redo,
            &lines_vec[redo - 1].replacen(" 0 ", " 1 ", 1)
        )),
        e(redo, DecodeReason::OutOfBlock)
    );
    assert_eq!(
        repository_refusal(&replace_line(
            redo,
            &lines_vec[redo - 1].replacen(" entrer ", " sortir ", 1)
        )),
        e(redo, DecodeReason::Value("entrée"))
    );
    assert_eq!(
        repository_refusal(&replace_line(
            identity,
            &lines_vec[identity - 1].replacen("identite note", "identite autre", 1)
        )),
        e(identity, DecodeReason::Value("categorie"))
    );
    assert_eq!(
        repository_refusal(&swap(identity, identity + 1)),
        e(identity + 1, DecodeReason::Order)
    );
    let command = operation + 1;
    assert_eq!(
        repository_refusal(&replace_line(command, "  commande inconnue")),
        e(command, DecodeReason::Value("commande"))
    );
    assert_eq!(
        repository_refusal(&replace_line(
            command,
            &format!("{} x", lines_vec[command - 1])
        )),
        e(command, DecodeReason::Field("fin de ligne"))
    );
    assert_eq!(
        repository_refusal(&remove_line(command + 1)).line,
        command + 1
    );
    assert_eq!(
        repository_refusal(&replace_line(
            command + 1,
            &lines_vec[command].replacen("publie ", "publié ", 1)
        )),
        e(command + 1, DecodeReason::Value("jeton"))
    );
    assert_eq!(
        repository_refusal(&replace_line(
            command + 1,
            &lines_vec[command]
                .replacen("publie oui", "publie peut-etre", 1)
                .replacen("publie non", "publie peut-etre", 1)
        )),
        e(command + 1, DecodeReason::Value("publie"))
    );
    assert_eq!(
        repository_refusal(&remove_line(command)),
        e(command, DecodeReason::Field("commande"))
    );
}

// FORMAT.md, section 12.1 : les listes d'identifiants d'un effet sont strictement croissantes ;
// une ligne d'effet fautive est située (compléments après mutation).
#[test]
fn effect_lines_are_strict_and_located() {
    let e = |line, reason| DecodeError { line, reason };
    let text = REPOSITORY_EXAMPLE_V1;
    let (index, effect) = text
        .lines()
        .enumerate()
        .find(|(_, l)| l.starts_with("  effet ") && l.contains("apres 1 "))
        .unwrap();
    let number = index + 1;
    let cause = effect.rsplit(' ').next().unwrap();
    let ghost = "ffffffff-ffff-ffff-ffff-ffffffffffff";
    let with = |line: String| text.replacen(effect, &line, 1);
    // Deux causes en ordre croissant : acceptées et réencodées à l'identique.
    let two = with(effect.replacen(
        &format!("apres 1 {cause}"),
        &format!("apres 2 {cause} {ghost}"),
        1,
    ));
    let decoded = decode_repository(&two).unwrap();
    assert_eq!(encode_repository(&decoded), two);
    // Ordre décroissant ou doublon : refusés.
    for list in [format!("{ghost} {cause}"), format!("{cause} {cause}")] {
        let bad = with(effect.replacen(&format!("apres 1 {cause}"), &format!("apres 2 {list}"), 1));
        assert_eq!(decode_repository(&bad), Err(e(number, DecodeReason::Order)));
    }
    // Étiquette fautive : refus à la ligne de l'effet.
    let mislabeled = with(effect.replacen("  effet ", "  efet ", 1));
    assert_eq!(
        decode_repository(&mislabeled),
        Err(e(number, DecodeReason::Field("effet")))
    );
}

// FORMAT.md, section 12.4 : chaque règle de cohérence est vérifiée et nommée.
#[test]
fn repository_consistency_rules_are_enforced() {
    let e = |rule| DecodeError {
        line: 0,
        reason: DecodeReason::Inconsistent(rule),
    };
    let repo = example_repository();
    let text = encode_repository(&repo);
    let branch_line = text
        .lines()
        .find(|l| l.starts_with("branche ") && !l.contains("origine -"))
        .unwrap();
    let experimental = branch_line.split(' ').nth(1).unwrap();
    let tete = branch_line.split(' ').nth(3).unwrap();
    let ghost = "a5000000-0000-0000-0000-0000000000ff";
    let cases: Vec<(String, &str)> = vec![
        (
            text.replace(&format!("active {}", MAIN), &format!("active {ghost}")),
            "branche active",
        ),
        (
            text.replacen(
                &format!("branche {experimental} tete {tete}"),
                &format!("branche {experimental} tete {ghost}"),
                1,
            ),
            "tête, base ou origine de branche",
        ),
        (
            text.replacen(
                &format!("origine - base {ROOT}"),
                &format!("origine {experimental} base {ROOT}"),
                1,
            ),
            "branche main",
        ),
    ];
    for (bad, rule) in cases {
        assert_eq!(decode_repository(&bad), Err(e(rule)), "{rule}");
    }
    // Chaque règle de FORMAT.md, section 12.4, et chaque terme des règles composées.
    let first = text
        .lines()
        .filter(|l| l.starts_with("commit "))
        .nth(1)
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap();
    let swap = |old: String, new: String| {
        assert_eq!(text.matches(&old).count(), 1, "« {old} » doit être unique");
        text.replacen(&old, &new, 1)
    };
    let without = |prefix: &str| {
        let kept: Vec<&str> = text.lines().filter(|l| !l.starts_with(prefix)).collect();
        assert_eq!(
            kept.len() + 1,
            text.lines().count(),
            "« {prefix} » doit être unique"
        );
        kept.join("\n") + "\n"
    };
    let identity = |category: &str| {
        text.lines()
            .find(|l| l.starts_with(&format!("identite {category} ")))
            .unwrap()
            .to_owned()
    };
    let more: Vec<(String, &str)> = vec![
        (
            swap(
                format!("commit {ROOT} parent -"),
                format!("commit {ROOT} parent {first}"),
            ),
            "commit racine",
        ),
        (
            swap(
                format!("commit {first} parent {ROOT}"),
                format!("commit {first} parent {ghost}"),
            ),
            "parent de commit",
        ),
        (
            swap(
                format!("commit {first} parent {ROOT}"),
                format!("commit {first} parent -"),
            ),
            "parent de commit",
        ),
        (
            swap(
                format!("origine - base {ROOT}"),
                format!("origine - base {first}"),
            ),
            "branche main",
        ),
        (
            swap(
                format!("origine {MAIN} base"),
                format!("origine {ghost} base"),
            ),
            "tête, base ou origine de branche",
        ),
        (
            swap(
                format!("entrer {experimental} tete {tete}"),
                format!("entrer {experimental} tete {ghost}"),
            ),
            "pile de rétablissement",
        ),
        (
            swap(
                format!("entrer {experimental} tete {tete}"),
                format!("entrer {ghost} tete {tete}"),
            ),
            "pile de rétablissement",
        ),
        (
            swap(
                format!("entrer {experimental} tete {tete}"),
                format!("commit {ghost}"),
            ),
            "pile de rétablissement",
        ),
        (
            swap(
                format!("resultat branche {experimental} tete {tete} publie oui"),
                format!("resultat branche {ghost} tete {tete} publie oui"),
            ),
            "résultat d'opération",
        ),
        (
            swap(
                format!("resultat branche {experimental} tete {tete} publie oui"),
                format!("resultat branche {experimental} tete {ghost} publie oui"),
            ),
            "résultat d'opération",
        ),
        (without(&identity("note")), "identités employées"),
        (without(&identity("relation")), "identités employées"),
        (without(&identity("contexte")), "identités employées"),
    ];
    for (bad, rule) in more {
        assert_eq!(decode_repository(&bad), Err(e(rule)), "{rule}");
    }
    // Base d'une branche expérimentale inexistante.
    let base_line = text
        .lines()
        .find(|l| l.starts_with(&format!("branche {experimental} ")))
        .unwrap();
    let rebased = base_line
        .rsplit_once(' ')
        .map(|(head, _)| format!("{head} {ghost}"))
        .unwrap();
    assert_eq!(
        decode_repository(&text.replacen(base_line, &rebased, 1)),
        Err(e("tête, base ou origine de branche"))
    );
    // Identité d'ancrage manquante, sur un dépôt qui emploie des ancrages.
    let rich = encode_repository(rich_harness().repository());
    let anchor = rich
        .lines()
        .find(|l| l.starts_with("identite ancrage "))
        .unwrap();
    assert_eq!(
        decode_repository(&rich.replacen(&format!("{anchor}\n"), "", 1)),
        Err(e("identités employées"))
    );
}

// ---------------------------------------------------------------------------
// Incrément 3.2 — contrat ajouté : fragments du dépôt, pour un stockage par morceaux.
// ---------------------------------------------------------------------------

fn categories(changes: &[FragmentChange]) -> Vec<(Category, bool)> {
    changes
        .iter()
        .map(|c| (c.category, c.text.is_some()))
        .collect()
}

// La mise à jour incrémentale égale le découpage complet, et ne rend que les fragments modifiés.
#[test]
fn fragment_updates_report_exactly_what_changed() {
    let mut r = Harness::new();
    let mut fragments = Fragments::of(r.repository());
    assert_eq!(fragments.assemble(), encode_repository(r.repository()));
    let mut step = |r: &Harness, expected: Vec<(Category, bool)>| {
        let changes = fragments.update(r.repository());
        assert_eq!(categories(&changes), expected);
        assert_eq!(fragments, Fragments::of(r.repository()));
        assert_eq!(fragments.assemble(), encode_repository(r.repository()));
    };
    use Category::{Active, Branch, Commit, Identity, Operation};
    // Création sur main : tête de main, identité, commit, journal.
    let a = note(&mut r, Kind::Reasoning, "a");
    step(
        &r,
        vec![
            (Branch, true),
            (Identity, true),
            (Commit, true),
            (Operation, true),
        ],
    );
    // Commande refusée : rien.
    assert!(r.link(DEFAULT_SCOPE, a, a, Relation::DependsOn).is_err());
    step(&r, vec![]);
    // Bifurcation : branche active, nouvelle branche, commit, journal (aucune identité nouvelle).
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    step(
        &r,
        vec![
            (Active, true),
            (Branch, true),
            (Commit, true),
            (Operation, true),
        ],
    );
    // Annulation de l'opération fondatrice : retour sur main, pile de main, journal.
    r.undo().unwrap();
    step(&r, vec![(Active, true), (Branch, true), (Operation, true)]);
    // Un fragment disparu du dépôt est retiré ; un commit déjà encodé n'est pas réencodé.
    let mut stale = Fragments::of(r.repository());
    let commits = stale.parts.get_mut(&Commit).unwrap();
    commits.insert("zzzz".into(), "commit périmé\n".into());
    let (key, text) = commits.iter_mut().next().unwrap();
    let key = key.clone();
    text.push_str("  mémorisé\n");
    let changes = stale.update(r.repository());
    assert_eq!(
        changes,
        vec![FragmentChange {
            category: Commit,
            key: "zzzz".into(),
            text: None
        }]
    );
    assert!(stale.parts[&Commit][&key].ends_with("  mémorisé\n"));
}

// Les catégories ont un nom stable, dans l'ordre canonique.
#[test]
fn fragment_categories_have_stable_names() {
    let names: Vec<&str> = Category::ALL.iter().map(|c| c.word()).collect();
    assert_eq!(
        names,
        ["active", "branche", "identite", "commit", "operation"]
    );
    for category in Category::ALL {
        assert_eq!(Category::from_word(category.word()), Some(category));
    }
    assert_eq!(Category::from_word("inconnue"), None);
    let mut sorted = Category::ALL;
    sorted.sort();
    assert_eq!(sorted, Category::ALL);
}

// Le journal est lisible, par identifiant d'opération croissant ; un refus n'y figure pas.
#[test]
fn the_journal_lists_accepted_operations_in_order() {
    let mut repo = Repository::new();
    assert_eq!(repo.journal().count(), 0);
    let first = envelope(&repo, 20, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    let outcome = repo.execute(first.clone()).unwrap();
    let refused = envelope(&repo, 21, link(2, 1, 1, Relation::DependsOn));
    assert!(repo.execute(refused).is_err());
    let second = envelope(&repo, 10, create(DEFAULT_SCOPE, 3, Kind::Category));
    let later = repo.execute(second.clone()).unwrap();
    let journal: Vec<(Envelope, Outcome)> = repo
        .journal()
        .map(|(e, o)| (e.clone(), o.clone()))
        .collect();
    assert_eq!(journal, vec![(second, later), (first, outcome)]);
}

// ---------------------------------------------------------------------------
// Incrément 5.1 — contrat ajouté : instantané au format version 2 (VERSION2.md, section 5).
// ---------------------------------------------------------------------------

const EXAMPLE_V2: &str = include_str!("../../../fixtures/format/exemple-v2.txt");

// VERSION2.md, section 7 : l'exemple v2 est figé dans les deux sens et décrit le même état que
// l'exemple v1, dont il est la migration.
#[test]
fn the_v2_snapshot_example_is_frozen_in_both_directions() {
    let state = example_state();
    assert_eq!(encode_snapshot_v2(&state), EXAMPLE_V2);
    assert_eq!(decode_snapshot_v2(EXAMPLE_V2), Ok(state.clone()));
    assert_eq!(decode_snapshot(EXAMPLE_V1), Ok(state));
    assert_eq!(
        encode_snapshot_v2(&decode_snapshot(EXAMPLE_V1).unwrap()),
        EXAMPLE_V2
    );
    assert!(EXAMPLE_V2.starts_with(SNAPSHOT_HEADER_V2));
}

// Fidélité et forme canonique unique en v2 : citations, rangs de révision, empreintes.
#[test]
fn every_record_round_trips_in_v2() {
    let state = rich_state();
    let text = encode_snapshot_v2(&state);
    let decoded = decode_snapshot_v2(&text).unwrap();
    assert_eq!(decoded, state);
    assert_eq!(encode_snapshot_v2(&decoded), text);
    for word in [
        "\ncitation ",
        " citation titre ",
        " source ",
        " partie question ",
        " partie reponse ",
        " version - ",
        " empreinte ",
    ] {
        assert!(text.contains(word), "« {word} » absent du texte v2");
    }
    let ranks: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("revision "))
        .map(|l| l.split(' ').nth(2).unwrap())
        .collect();
    assert_eq!(ranks, ["1", "2"]);
    assert!(!text.lines().any(|l| l.starts_with("passage ")));
    assert!(!text.contains(" passage titre "));
}

/// L'exemple v2 avec une ligne remplacée (numérotée à partir de 1).
fn with_line_v2(n: usize, replacement: &str) -> String {
    let mut lines: Vec<&str> = EXAMPLE_V2.lines().collect();
    lines[n - 1] = replacement;
    lines.join("\n") + "\n"
}

/// Numéro (à partir de 1) de la première ligne d'un texte qui commence par le préfixe donné.
fn line_starting(text: &str, prefix: &str) -> usize {
    text.lines().position(|l| l.starts_with(prefix)).unwrap() + 1
}

// VERSION2.md, section 5 : chaque écart propre à la v2 est refusé, avec sa ligne et son motif.
#[test]
fn v2_snapshot_deviations_are_refused() {
    let e = |line, reason| DecodeError { line, reason };
    // En-têtes croisés.
    assert_eq!(decode_snapshot(EXAMPLE_V2), Err(e(1, DecodeReason::Header)));
    assert_eq!(
        decode_snapshot_v2(EXAMPLE_V1),
        Err(e(1, DecodeReason::Header))
    );
    let content = line_starting(EXAMPLE_V2, "contenu a5000000-0000-0000-0000-000000000002 ");
    let citation = line_starting(EXAMPLE_V2, "citation ");
    let revision = line_starting(EXAMPLE_V2, "revision ");
    let v1_passage = EXAMPLE_V1
        .lines()
        .find(|l| l.starts_with("passage "))
        .unwrap();
    let cited = EXAMPLE_V2.lines().nth(citation - 1).unwrap();
    let revised = EXAMPLE_V2.lines().nth(revision - 1).unwrap();
    let cases: Vec<(usize, String, DecodeReason)> = vec![
        (
            citation,
            v1_passage.to_owned(),
            DecodeReason::UnknownRecord("passage".into()),
        ),
        (
            content,
            EXAMPLE_V2
                .lines()
                .nth(content - 1)
                .unwrap()
                .replace(" citation titre ", " passage titre "),
            DecodeReason::Value("rôle"),
        ),
        (
            revision,
            revised.replace(" 1 mineure ", " 2 mineure "),
            DecodeReason::Value("rang"),
        ),
        (
            revision,
            revised.replace(" 1 mineure ", " 0 mineure "),
            DecodeReason::Value("rang"),
        ),
        (
            revision,
            revised.replace(" 1 mineure ", " mineure "),
            DecodeReason::Value("entier"),
        ),
        // Contrat modifié (5.5a, D18, D24) : la citation d'une note est lue ; sa version est un
        // rang de révision, jamais « - ».
        (
            citation,
            cited.replace(" partie reponse ", " partie note "),
            DecodeReason::Value("version"),
        ),
        (
            citation,
            cited.replace(" partie reponse ", " partie titre "),
            DecodeReason::Value("partie"),
        ),
        (
            citation,
            cited.replace(" version - ", " version 0 "),
            DecodeReason::Value("version"),
        ),
        (
            citation,
            cited.replace(" empreinte 0f661db2d23f65f3", " empreinte 0f661db2d23f65f4"),
            DecodeReason::Value("empreinte"),
        ),
        (
            citation,
            cited.replace(" source ", " echange "),
            DecodeReason::Field("source"),
        ),
    ];
    for (line, replacement, reason) in cases {
        assert_eq!(
            decode_snapshot_v2(&with_line_v2(line, &replacement)),
            Err(e(line, reason)),
            "{replacement}"
        );
    }
    // En v1, le rôle citation reste inconnu.
    let v1_content = line_starting(EXAMPLE_V1, "contenu a5000000-0000-0000-0000-000000000002 ");
    assert_eq!(
        decode_snapshot(&with_line(
            v1_content,
            &line_of(v1_content).replace(" passage titre ", " citation titre ")
        )),
        Err(e(v1_content, DecodeReason::Value("rôle")))
    );
}

// L'enregistrement citation n'existe pas en v1.
#[test]
fn citation_records_are_refused_in_v1() {
    let passage = line_starting(EXAMPLE_V1, "passage ");
    let cited = EXAMPLE_V2
        .lines()
        .find(|l| l.starts_with("citation "))
        .unwrap();
    assert_eq!(
        decode_snapshot(&with_line(passage, cited)),
        Err(DecodeError {
            line: passage,
            reason: DecodeReason::UnknownRecord("citation".into())
        })
    );
}

// ---------------------------------------------------------------------------
// Incrément 5.1 — contrat ajouté : dépôt au format version 2 et migration (VERSION2.md, sections
// 3, 6 et 7).
// ---------------------------------------------------------------------------

const REPOSITORY_EXAMPLE_V2: &str = include_str!("../../../fixtures/format/depot-v2.txt");

/// Lignes du bloc qui suit l'en-tête du commit donné : `  reference` ou `  difference`, puis ses
/// lignes indentées de quatre espaces.
fn commit_block(text: &str, commit: &str) -> Vec<String> {
    let mut lines = text
        .lines()
        .skip_while(|l| !l.starts_with(&format!("commit {commit} ")))
        .skip(1);
    let mut block = vec![lines.next().unwrap().to_owned()];
    block.extend(
        lines
            .take_while(|l| l.starts_with("    "))
            .map(str::to_owned),
    );
    block
}

// VERSION2.md, section 7 : l'exemple v2 est figé, il est la migration exacte de l'exemple v1, et
// les deux versions se lisent. Seule la racine porte un état de référence.
#[test]
fn the_v2_repository_example_is_frozen_and_is_the_migration_of_v1() {
    let repo = example_repository();
    assert_eq!(encode_repository_v2(&repo), REPOSITORY_EXAMPLE_V2);
    assert_eq!(
        decode_repository_v2(REPOSITORY_EXAMPLE_V2),
        Ok(repo.clone())
    );
    assert_eq!(
        migrate_v1_to_v2(REPOSITORY_EXAMPLE_V1),
        Ok(REPOSITORY_EXAMPLE_V2.to_owned())
    );
    assert_eq!(
        decode_repository_any(REPOSITORY_EXAMPLE_V1),
        Ok(repo.clone())
    );
    assert_eq!(decode_repository_any(REPOSITORY_EXAMPLE_V2), Ok(repo));
    assert!(REPOSITORY_EXAMPLE_V2.starts_with(REPOSITORY_HEADER_V2));
    assert_eq!(REPOSITORY_EXAMPLE_V2.matches("\n  reference\n").count(), 1);
    assert_eq!(
        commit_block(REPOSITORY_EXAMPLE_V2, &ROOT.to_string())[0],
        "  reference"
    );
    // Une création ajoute ses lignes contenu et membre, un lien sa ligne relation, une
    // réfutation retire puis ajoute la ligne membre.
    let c = |n: u64| format!("00000000-0000-0001-0000-{n:012}");
    let shape = |commit: u64| -> Vec<String> {
        commit_block(REPOSITORY_EXAMPLE_V2, &c(commit))
            .iter()
            .map(|l| l.split(' ').take(6).collect::<Vec<_>>().join(" "))
            .collect()
    };
    assert_eq!(shape(1), ["  difference", "    + contenu", "    + membre"]);
    assert_eq!(shape(5), ["  difference", "    + relation"]);
    assert_eq!(shape(7), ["  difference", "    - membre", "    + membre"]);
}

// VERSION2.md, section 7 : un dépôt dont le journal emploie toutes les commandes migre vers la
// v2, se relit à l'identique, et creer-passage y devient citer.
#[test]
fn a_repository_using_every_command_migrates_to_v2() {
    let repo = every_command_repository();
    let v2 = encode_repository_v2(&repo);
    assert_eq!(migrate_v1_to_v2(&encode_repository(&repo)), Ok(v2.clone()));
    let decoded = decode_repository_v2(&v2).unwrap();
    assert_eq!(decoded, repo);
    assert_eq!(encode_repository_v2(&decoded), v2);
    for name in [
        "creer-note",
        "reviser",
        "inclure",
        "copier-contexte",
        "lier",
        "verdict",
        "decision",
        "implementation",
        "retirer",
        "reattacher",
        "remplacer",
        "restreindre",
        "contredire",
        "creer-echange",
        "completer-question",
        "citer",
        "creer-reserve",
        "traiter",
        "etat-reserve",
        "acquitter",
        "detacher",
        "ancrer",
        "retirer-ancrage",
        "restaurer",
        "annuler",
        "retablir",
        "basculer",
    ] {
        assert!(
            v2.contains(&format!("  commande {name} "))
                || v2.contains(&format!("  commande {name}\n")),
            "commande « {name} » absente du journal v2"
        );
    }
    assert!(!v2.contains("creer-passage"));
    for word in [
        " etat active\n",
        " parents 0 ",
        " parents 1 ",
        "\n  reference\n    memoire-instantane 2\n",
        "\n  difference\n",
        "\n    - ",
        "\n    + ",
        " source ",
        " version - ",
    ] {
        assert!(v2.contains(word), "« {word} » absent du dépôt v2");
    }
}

/// Dépôt linéaire sur main : une note d'implémentation, puis `changes` changements de son état
/// d'implémentation, alternés. L'état garde une taille constante.
fn linear_history(changes: usize) -> Repository {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Implementation, "suivie");
    for k in 0..changes {
        let value = if k % 2 == 0 {
            Implementation::InProgress
        } else {
            Implementation::Implemented
        };
        r.set_implementation(DEFAULT_SCOPE, n, value).unwrap();
    }
    assert_eq!(r.branch_count(), 1);
    r.repository().clone()
}

// VERSION2.md, section 3 : un état de référence à la racine, puis tous les cent commits le long
// des premiers parents ; une différence partout ailleurs.
#[test]
fn references_every_hundred_commits() {
    let repo = linear_history(249);
    assert_eq!(repo.commit_count(), 251);
    let text = encode_repository_v2(&repo);
    assert_eq!(text.matches("\n  reference\n").count(), 3);
    assert_eq!(text.matches("\n  difference\n").count(), 248);
    assert_eq!(decode_repository_v2(&text), Ok(repo.clone()));
    assert_eq!(migrate_v1_to_v2(&encode_repository(&repo)), Ok(text));
}

// VERSION2.md, section 6 : la taille de l'export suit les changements, non l'état complet.
#[test]
fn v2_size_follows_changes() {
    let mut r = Harness::new();
    let notes: Vec<NodeId> = (0..40)
        .map(|k| note(&mut r, Kind::Implementation, &format!("note {k}")))
        .collect();
    for n in notes {
        r.set_implementation(DEFAULT_SCOPE, n, Implementation::InProgress)
            .unwrap();
    }
    let v1 = encode_repository(r.repository()).len();
    let v2 = encode_repository_v2(r.repository()).len();
    assert!(v1 >= 5 * v2, "v1 : {v1} octets, v2 : {v2} octets");
}

// La profondeur suit les premiers parents, quel que soit l'ordre des identifiants : ici chaque
// commit a un identifiant plus petit que son parent.
#[test]
fn reference_placement_does_not_depend_on_identifier_order() {
    let mut repo = Repository::new();
    let node = NodeId::from_bytes(raw(1));
    repo.execute(envelope(
        &repo,
        2000,
        create(DEFAULT_SCOPE, 1, Kind::Implementation),
    ))
    .unwrap();
    for k in 1..102 {
        let implementation = if k % 2 == 0 {
            Implementation::Implemented
        } else {
            Implementation::InProgress
        };
        let command = Command::Change(Change::SetImplementation {
            scope: DEFAULT_SCOPE,
            node,
            implementation,
        });
        let next = Envelope {
            reason: "motif".into(),
            ..envelope(&repo, 2000 - k, command)
        };
        repo.execute(next).unwrap();
    }
    assert_eq!(repo.commit_count(), 103);
    let text = encode_repository_v2(&repo);
    assert_eq!(text.matches("\n  reference\n").count(), 2);
    // Profondeur 100 : la 100e opération, d'identifiant 2000 - 99.
    let at_100 = CommitId::from_bytes(raw(1901)).to_string();
    assert_eq!(commit_block(&text, &at_100)[0], "  reference");
    let at_99 = CommitId::from_bytes(raw(1902)).to_string();
    assert_eq!(commit_block(&text, &at_99)[0], "  difference");
    assert_eq!(decode_repository_v2(&text), Ok(repo));
}

/// Texte où le bloc du commit dont l'en-tête est à l'index `at` (à partir de 0) est remplacé.
fn with_block(text: &str, at: usize, block: &[&str]) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let end = (at + 2..lines.len())
        .find(|&k| !lines[k].starts_with("    "))
        .unwrap();
    let mut out: Vec<&str> = lines[..=at].to_vec();
    out.extend_from_slice(block);
    out.extend_from_slice(&lines[end..]);
    out.join("\n") + "\n"
}

// Une référence n'est admise qu'aux profondeurs multiples de cent, une différence partout ailleurs.
#[test]
fn reference_placement_is_enforced() {
    let e = |line| DecodeError {
        line,
        reason: DecodeReason::Inconsistent("état de référence"),
    };
    let text = encode_repository_v2(&linear_history(100));
    let lines: Vec<&str> = text.lines().collect();
    let references: Vec<usize> = (0..lines.len())
        .filter(|&k| lines[k] == "  reference")
        .collect();
    assert_eq!(references.len(), 2);
    // Une différence à la place de la référence de profondeur 100.
    let bad = with_block(&text, references[1] - 1, &["  difference"]);
    assert_eq!(decode_repository_v2(&bad), Err(e(references[1] + 1)));
    // Une référence à la place de la différence de profondeur 1.
    let first = lines.iter().position(|l| *l == "  difference").unwrap();
    let root_state = [
        "  reference",
        "    memoire-instantane 2",
        "    contexte 00000000-0000-0000-0000-000000000000",
        "    fin",
    ];
    let bad = with_block(&text, first - 1, &root_state);
    assert_eq!(decode_repository_v2(&bad), Err(e(first + 1)));
}

fn v2_refusal(text: &str) -> DecodeError {
    decode_repository_v2(text).unwrap_err()
}

/// L'exemple v2 avec la première ligne qui commence par `prefix` remplacée par `replacement`
/// (plusieurs lignes possibles) ; rend aussi son numéro (à partir de 1).
fn v2_with(prefix: &str, replacement: &str) -> (String, usize) {
    let number = line_starting(REPOSITORY_EXAMPLE_V2, prefix);
    let mut lines: Vec<&str> = REPOSITORY_EXAMPLE_V2.lines().collect();
    lines[number - 1] = replacement;
    (lines.join("\n") + "\n", number)
}

// VERSION2.md, section 6 : écarts propres au dépôt v2, refusés et situés.
#[test]
fn v2_repository_deviations_are_refused() {
    let e = |line, reason| DecodeError { line, reason };
    // En-têtes croisés ; un en-tête inconnu est refusé par la lecture selon l'en-tête.
    assert_eq!(
        v2_refusal(REPOSITORY_EXAMPLE_V1),
        e(1, DecodeReason::Header)
    );
    assert_eq!(
        decode_repository(REPOSITORY_EXAMPLE_V2),
        Err(e(1, DecodeReason::Header))
    );
    assert_eq!(
        decode_repository_any(&REPOSITORY_EXAMPLE_V2.replacen(
            "memoire-depot 2",
            "memoire-depot 9",
            1
        )),
        Err(e(1, DecodeReason::Header))
    );
    assert_eq!(decode_repository_any(""), Err(e(1, DecodeReason::Header)));
    let main = format!("branche {MAIN} ");
    let main_line = REPOSITORY_EXAMPLE_V2
        .lines()
        .find(|l| l.starts_with(&main))
        .unwrap();
    let c = |n: u64| format!("00000000-0000-0001-0000-{n:012}");
    let commit = |n: u64| format!("commit {} ", c(n));
    let header_of = |n: u64| {
        REPOSITORY_EXAMPLE_V2
            .lines()
            .find(|l| l.starts_with(&commit(n)))
            .unwrap()
            .to_owned()
    };
    let block_of = |n: u64| commit_block(REPOSITORY_EXAMPLE_V2, &c(n));
    let first_command = "  commande creer-note ";
    let cases: Vec<(&str, String, DecodeReason)> = vec![
        // Branche : état.
        // Contrat modifié (5.4a, D23) : « etat abandonnee » est lu ; main abandonnée est
        // désormais une incohérence du dépôt (abandon_rules_are_enforced_when_reading).
        (
            &main,
            main_line.replace(" etat active", " etat inconnu"),
            DecodeReason::Value("etat"),
        ),
        (
            &main,
            main_line.replace(" etat active", ""),
            DecodeReason::Field("etat"),
        ),
        // Commit : nombre de parents.
        // Contrat modifié (5.4b, D23) : deux parents sont lus (commit de fusion) ; deux parents
        // identiques sont une valeur refusée.
        (
            "commit 00000000-0000-0001-0000-000000000003 ",
            header_of(3).replace(
                " parents 1 00000000-0000-0001-0000-000000000001 ",
                &format!(" parents 2 {} {} ", c(1), c(1)),
            ),
            DecodeReason::Value("parents"),
        ),
        (
            "commit 00000000-0000-0001-0000-000000000003 ",
            header_of(3).replace(" parents 1 ", " parents 3 "),
            DecodeReason::Value("parents"),
        ),
        (
            "commit 00000000-0000-0001-0000-000000000003 ",
            header_of(3).replace(" parents 1 ", " parent "),
            DecodeReason::Field("parents"),
        ),
        // Commandes prévues par la v2 mais pas encore prises en charge, et commande de la v1.
        // Contrat modifié (5.3, LOT.md) : le lot est pris en charge ; un lot vide est une valeur
        // refusée.
        (
            first_command,
            "  commande lot 0".into(),
            DecodeReason::Value("lot"),
        ),
        // Contrat modifié (5.4b, D23) : `fusionner` est lu ; il porte la tête de la source
        // (`tete`), absente ici.
        (
            first_command,
            format!("  commande fusionner source {} resolutions 0", c(7)),
            DecodeReason::Field("tete"),
        ),
        // Contrat modifié (5.4a, D23) : `abandonner` et `reactiver` sont lus
        // (abandon_blocks_are_written_as_specified_and_round_trip).
        (
            first_command,
            format!(
                "  commande creer-passage portee {MAIN} note {} echange {} partie reponse debut 0 fin 1",
                c(2),
                c(4)
            ),
            DecodeReason::Value("commande"),
        ),
        // Contrat modifié (5.5a, D18, D24) : `citer … partie note version <n>` est lu
        // (note_citation_blocks_round_trip).
        (
            first_command,
            format!(
                "  commande citer portee {MAIN} note {} source {} partie reponse version 1 debut 0 fin 1",
                c(2),
                c(4)
            ),
            DecodeReason::Value("version"),
        ),
        // Différences : ligne retirée absente, ligne ajoutée déjà présente.
        (
            "    - membre ",
            block_of(7)[1].replace(" verdict non-verifiee ", " verdict verifiee "),
            DecodeReason::Inconsistent("différence"),
        ),
        (
            "    + relation ",
            format!("{}\n{}", block_of(1)[1], block_of(5)[1]),
            DecodeReason::Inconsistent("différence"),
        ),
        // Forme des lignes de différence.
        (
            "    + relation ",
            block_of(5)[1].replacen("    + ", "    * ", 1),
            DecodeReason::Value("différence"),
        ),
        (
            "    + relation ",
            "    + fin".into(),
            DecodeReason::UnknownRecord("fin".into()),
        ),
        // Commit sans bloc : la ligne qui suit l'en-tête n'est ni reference ni difference.
        (
            "  difference",
            block_of(1)[1].clone(),
            DecodeReason::Field("reference ou difference"),
        ),
    ];
    for (prefix, replacement, reason) in cases {
        let (text, number) = v2_with(prefix, &replacement);
        assert_eq!(v2_refusal(&text), e(number, reason), "{replacement}");
    }
    // Parent absent, ou parents en cycle : aucun état ne peut être reconstruit.
    let inconsistent = |rule| e(0, DecodeReason::Inconsistent(rule));
    let ghost = "a5000000-0000-0000-0000-0000000000ff";
    let orphan =
        REPOSITORY_EXAMPLE_V2.replacen(&header_of(3), &header_of(3).replace(&c(1), ghost), 1);
    assert_eq!(v2_refusal(&orphan), inconsistent("parent de commit"));
    let cycle = REPOSITORY_EXAMPLE_V2.replacen(
        &header_of(1),
        &header_of(1).replace(&format!("parents 1 {MAIN}"), &format!("parents 1 {}", c(3))),
        1,
    );
    assert_eq!(v2_refusal(&cycle), inconsistent("parent de commit"));
    // Un commit directement suivi d'un autre commit n'a pas de bloc.
    let only_header = format!(
        "{}\n{}\n",
        REPOSITORY_EXAMPLE_V2
            .lines()
            .take(line_starting(REPOSITORY_EXAMPLE_V2, "  reference") - 1)
            .collect::<Vec<_>>()
            .join("\n"),
        REPOSITORY_EXAMPLE_V2
            .lines()
            .skip(line_starting(REPOSITORY_EXAMPLE_V2, &commit(1)) - 1)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        v2_refusal(&only_header),
        e(
            line_starting(REPOSITORY_EXAMPLE_V2, "  reference"),
            DecodeReason::Field("reference ou difference")
        )
    );
    // Ordre : clés croissantes, retraits avant ajouts.
    let (a, b) = (&block_of(1)[1], &block_of(1)[2]);
    let swapped = REPOSITORY_EXAMPLE_V2.replacen(&format!("{a}\n{b}\n"), &format!("{b}\n{a}\n"), 1);
    assert_eq!(
        v2_refusal(&swapped),
        e(
            line_starting(REPOSITORY_EXAMPLE_V2, a) + 1,
            DecodeReason::Order
        )
    );
    let (minus, plus) = (&block_of(7)[1], &block_of(7)[2]);
    let swapped = REPOSITORY_EXAMPLE_V2.replacen(
        &format!("{minus}\n{plus}\n"),
        &format!("{plus}\n{minus}\n"),
        1,
    );
    assert_eq!(
        v2_refusal(&swapped),
        e(
            line_starting(REPOSITORY_EXAMPLE_V2, minus) + 1,
            DecodeReason::Order
        )
    );
    // Minimalité : une ligne retirée puis remise telle quelle est refusée.
    let kept = block_of(1)[1].replacen("    + ", "", 1);
    let relation = &block_of(5)[1];
    let not_minimal = REPOSITORY_EXAMPLE_V2.replacen(
        &format!("{relation}\n"),
        &format!("    - {kept}\n    + {kept}\n{relation}\n"),
        1,
    );
    assert_eq!(
        v2_refusal(&not_minimal),
        e(
            line_starting(REPOSITORY_EXAMPLE_V2, relation) + 1,
            DecodeReason::Inconsistent("différence minimale")
        )
    );
}

// Les commandes propres à la v2 sont inconnues de la v1.
#[test]
fn v2_commands_are_refused_in_v1() {
    let c = |n: u64| format!("00000000-0000-0001-0000-{n:012}");
    let number = repository_line("  commande creer-note ");
    for command in [
        format!(
            "  commande citer portee {MAIN} note {} source {} partie reponse version - debut 0 fin 1",
            c(2),
            c(4)
        ),
        "  commande lot 0".into(),
        format!("  commande fusionner source {} resolutions 0", c(7)),
        format!("  commande abandonner branche {}", c(7)),
        format!("  commande reactiver branche {}", c(7)),
    ] {
        let mut lines: Vec<&str> = REPOSITORY_EXAMPLE_V1.lines().collect();
        lines[number - 1] = &command;
        assert_eq!(
            repository_refusal(&(lines.join("\n") + "\n")),
            DecodeError {
                line: number,
                reason: DecodeReason::Value("commande")
            },
            "{command}"
        );
    }
}

// Une erreur dans le bloc d'un commit est située à sa ligne dans le dépôt : ligne de l'état de
// référence, ligne de différence, ou ligne du bloc pour une erreur de l'état global.
#[test]
fn errors_inside_v2_commit_blocks_are_located() {
    let at = |text: &str| decode_repository_v2(text).unwrap_err();
    let context = format!("    contexte {MAIN}");
    // Dans un état de référence : ligne de l'instantané, puis état global à la ligne reference.
    let (text, number) = v2_with(&context, "    contexte 0");
    assert_eq!(
        at(&text),
        DecodeError {
            line: number,
            reason: DecodeReason::Value("identifiant")
        }
    );
    let without_scope = REPOSITORY_EXAMPLE_V2.replacen(&format!("{context}\n"), "", 1);
    let error = at(&without_scope);
    assert_eq!(
        error.line,
        line_starting(REPOSITORY_EXAMPLE_V2, "  reference")
    );
    assert!(
        matches!(error.reason, DecodeReason::InvalidState(_)),
        "{error}"
    );
    // Dans une différence : ligne ajoutée mal formée.
    let c = |n: u64| format!("00000000-0000-0001-0000-{n:012}");
    let block_1 = commit_block(REPOSITORY_EXAMPLE_V2, &c(1));
    let (text, number) = v2_with(
        &block_1[2],
        &block_1[2].replace(" verdict non-verifiee ", " verdict inconnu "),
    );
    assert_eq!(
        at(&text),
        DecodeError {
            line: number,
            reason: DecodeReason::Value("verdict")
        }
    );
    // Membre ajouté sans le contenu de sa note : erreur à la ligne du membre.
    let without_content = REPOSITORY_EXAMPLE_V2.replacen(&format!("{}\n", block_1[1]), "", 1);
    assert_eq!(
        at(&without_content),
        DecodeError {
            line: line_starting(REPOSITORY_EXAMPLE_V2, &block_1[2]) - 1,
            reason: DecodeReason::InvalidState(Error::MissingNode(NodeId::from_bytes(
                NodeId::parse(&c(2)).unwrap().to_bytes()
            )))
        }
    );
    // État global invalide après une différence : ligne difference du commit.
    let block_5 = commit_block(REPOSITORY_EXAMPLE_V2, &c(5));
    let ghost = "a5000000-0000-0000-0000-0000000000ff";
    let dangling = REPOSITORY_EXAMPLE_V2.replacen(
        &block_5[1],
        &block_5[1].replace(
            &format!("{} depend-de", c(2)),
            &format!("{ghost} depend-de"),
        ),
        1,
    );
    let error = at(&dangling);
    assert_eq!(
        error.line,
        line_starting(REPOSITORY_EXAMPLE_V2, &format!("commit {} ", c(5))) + 1
    );
    assert!(
        matches!(error.reason, DecodeReason::InvalidState(_)),
        "{error}"
    );
}

// ---------------------------------------------------------------------------
// Incrément 5.2b — contrat ajouté : historique derrière un trait, écritures préparées
// (ETAT-HISTORIQUE.md, sections 4 et 9 ; D20).
// ---------------------------------------------------------------------------

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

fn reasoned(repo: &Repository, op: u64, command: Command) -> Envelope {
    Envelope {
        reason: "motif".into(),
        ..envelope(repo, op, command)
    }
}

fn written(prepared: Prepared) -> Writes {
    match prepared {
        Prepared::Write(writes) => writes,
        other => panic!("écritures attendues, obtenu {other:?}"),
    }
}

// Une modification acceptée prépare exactement ses écritures : commit, branche, branche active,
// identité nouvelle, entrée du journal, état de la tête après ; la préparation ne modifie rien,
// et l'exécution applique exactement ce qui a été préparé.
#[test]
fn an_accepted_change_prepares_exactly_its_writes() {
    let mut repo = Repository::new();
    let next = reasoned(&repo, 10, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    let control = repo.clone();
    let writes = written(repo.prepare(&next).unwrap());
    assert_eq!(repo, control);
    let commit = CommitId::from_bytes(raw(10));
    let new = writes.commit.clone().unwrap();
    assert_eq!(
        (new.id, new.parent, new.reason.as_str()),
        (commit, ROOT, "motif")
    );
    assert_eq!(new.state, writes.state);
    assert_eq!(
        writes.branches,
        BTreeMap::from([(
            MAIN,
            Branch {
                head: commit,
                redo: vec![],
                origin: None,
                base: ROOT,
                // Contrat modifié (5.4a, D23) : une branche porte son état d'abandon.
                abandoned: false,
            }
        )])
    );
    assert_eq!(writes.active, MAIN);
    // Contrat modifié (5.3, LOT.md) : un lot emploie plusieurs identités ; le champ devient une
    // liste, ici d'un seul élément.
    assert_eq!(
        writes.identities,
        vec![UsedId::Node(NodeId::from_bytes(raw(1)))]
    );
    assert_eq!(writes.envelope, next);
    assert!(writes.outcome.published && !writes.outcome.forked);
    assert_eq!((writes.outcome.branch, writes.outcome.head), (MAIN, commit));
    let outcome = repo.execute(next).unwrap();
    assert_eq!(outcome, writes.outcome);
    assert_eq!(repo.snapshot(), &*writes.state);
    assert!(writes
        .state
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .node(NodeId::from_bytes(raw(1)))
        .is_ok());
}

// Une bifurcation n'écrit que la branche créée ; main garde sa tête.
#[test]
fn a_fork_writes_only_the_new_branch() {
    let mut repo = Repository::new();
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Reasoning)),
        (11, create(DEFAULT_SCOPE, 2, Kind::Implementation)),
        (12, link(3, 2, 1, Relation::DependsOn)),
    ] {
        let next = reasoned(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let parent = repo.head();
    let refute = reasoned(
        &repo,
        13,
        Command::Change(Change::SetVerdict {
            scope: DEFAULT_SCOPE,
            node: NodeId::from_bytes(raw(1)),
            verdict: Verdict::Refuted,
        }),
    );
    let writes = written(repo.prepare(&refute).unwrap());
    let branch = BranchId::from_bytes(raw(13));
    let commit = CommitId::from_bytes(raw(13));
    assert_eq!(
        writes.commit.as_ref().map(|c| (c.id, c.parent)),
        Some((commit, parent))
    );
    assert_eq!(
        writes.branches,
        BTreeMap::from([(
            branch,
            Branch {
                head: commit,
                redo: vec![],
                origin: Some(MAIN),
                base: parent,
                // Contrat modifié (5.4a, D23) : une branche porte son état d'abandon.
                abandoned: false,
            }
        )])
    );
    assert_eq!(writes.active, branch);
    // Contrat modifié (5.3, LOT.md) : liste d'identités, vide ici.
    assert_eq!(writes.identities, vec![]);
    assert!(writes.outcome.forked);
    repo.execute(refute).unwrap();
    assert_eq!(repo.branch_head(MAIN), Ok(parent));
    // Liste des branches, en mémoire, par identifiant croissant.
    assert_eq!(repo.branches().collect::<Vec<_>>(), vec![MAIN, branch]);
    assert_eq!(repo.branches().count(), repo.branch_count());
}

// Annuler, quitter une branche, rétablir, basculer : des écritures sans commit nouveau.
#[test]
fn moving_heads_writes_branches_without_commits() {
    let mut repo = Repository::new();
    let first = reasoned(&repo, 10, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    repo.execute(first).unwrap();
    let commit = CommitId::from_bytes(raw(10));
    let undo = written(repo.prepare(&envelope(&repo, 11, Command::Undo)).unwrap());
    assert_eq!(undo.commit, None);
    assert_eq!(
        undo.branches,
        BTreeMap::from([(
            MAIN,
            Branch {
                head: ROOT,
                redo: vec![Redo::Commit(commit)],
                origin: None,
                base: ROOT,
                // Contrat modifié (5.4a, D23) : une branche porte son état d'abandon.
                abandoned: false,
            }
        )])
    );
    // Contrat modifié (5.3, LOT.md) : liste d'identités, vide ici.
    assert_eq!((undo.active, undo.identities), (MAIN, vec![]));
    assert_eq!(*undo.state, Snapshot::default());
    repo.execute(envelope(&repo, 11, Command::Undo)).unwrap();
    assert_eq!(repo.snapshot(), &Snapshot::default());
    let redo = written(repo.prepare(&envelope(&repo, 12, Command::Redo)).unwrap());
    assert_eq!(redo.commit, None);
    assert_eq!(redo.branches[&MAIN].head, commit);
    assert!(redo.branches[&MAIN].redo.is_empty());
    assert_eq!(redo.state, repo.snapshot_at(commit).unwrap());
    repo.execute(envelope(&repo, 12, Command::Redo)).unwrap();
    // Bascule vers une autre branche : seule la branche active change.
    let link_and_refute = [
        (13, create(DEFAULT_SCOPE, 2, Kind::Implementation)),
        (14, link(3, 2, 1, Relation::DependsOn)),
        (
            15,
            Command::Change(Change::SetVerdict {
                scope: DEFAULT_SCOPE,
                node: NodeId::from_bytes(raw(1)),
                verdict: Verdict::Refuted,
            }),
        ),
    ];
    for (op, command) in link_and_refute {
        let next = reasoned(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let branch = repo.active_branch();
    let checkout = written(
        repo.prepare(&envelope(&repo, 16, Command::Checkout { branch: MAIN }))
            .unwrap(),
    );
    assert_eq!(
        (checkout.commit.is_none(), checkout.branches.is_empty()),
        (true, true)
    );
    assert_eq!(checkout.active, MAIN);
    assert_eq!(
        checkout.state,
        repo.snapshot_at(repo.branch_head(MAIN).unwrap()).unwrap()
    );
    assert_ne!(branch, MAIN);
}

// Refus, répétition et commande sans effet ne préparent aucune écriture.
#[test]
fn refusals_repetitions_and_no_ops_write_nothing() {
    let mut repo = Repository::new();
    let first = reasoned(&repo, 10, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    let outcome = repo.execute(first.clone()).unwrap();
    let control = repo.clone();
    // Répétition de la même enveloppe : le résultat enregistré.
    assert_eq!(repo.prepare(&first), Ok(Prepared::Repeated(outcome)));
    // Refus : le même que celui de l'exécution.
    let refused = envelope(&repo, 11, link(2, 1, 1, Relation::DependsOn));
    assert_eq!(repo.prepare(&refused), Err(Error::SelfRelation));
    let reused = Envelope {
        reason: "autre".into(),
        ..first.clone()
    };
    assert_eq!(repo.prepare(&reused), Err(Error::OpIdInUse(first.op)));
    // Sans effet : rien n'est publié ni journalisé.
    let stay = envelope(&repo, 12, Command::Checkout { branch: MAIN });
    match repo.prepare(&stay) {
        Ok(Prepared::NoEffect(o)) => assert!(!o.published && !o.forked && o.head == repo.head()),
        other => panic!("sans effet attendu, obtenu {other:?}"),
    }
    assert_eq!(repo, control);
    repo.execute(stay).unwrap();
    assert_eq!(repo, control);
}

/// Historique d'essai : l'historique en mémoire, dont les lectures ou l'écriture échouent à la
/// demande (disque plein, fichier illisible).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Failing {
    inner: MemoryHistory,
    reads: Rc<Cell<bool>>,
    writes: Rc<Cell<bool>>,
}

impl Failing {
    fn read<T>(&self, value: Result<T, Error>) -> Result<T, Error> {
        if self.reads.get() {
            return Err(Error::Historique("lecture impossible".into()));
        }
        value
    }
}

impl History for Failing {
    fn active(&self) -> Result<BranchId, Error> {
        self.read(self.inner.active())
    }
    fn branches(&self) -> Result<BTreeMap<BranchId, Branch>, Error> {
        self.read(self.inner.branches())
    }
    fn commit_count(&self) -> Result<usize, Error> {
        self.read(self.inner.commit_count())
    }
    fn state(&self, commit: CommitId) -> Result<Arc<Snapshot>, Error> {
        self.read(self.inner.state(commit))
    }
    fn commit(&self, commit: CommitId) -> Result<Option<CommitInfo>, Error> {
        self.read(self.inner.commit(commit))
    }
    fn operation(&self, op: OpId) -> Result<Option<(Envelope, Outcome)>, Error> {
        self.read(self.inner.operation(op))
    }
    fn is_used(&self, id: UsedId) -> Result<bool, Error> {
        self.read(self.inner.is_used(id))
    }
    fn branches_containing(&self, node: NodeId) -> Result<Vec<BranchId>, Error> {
        self.read(self.inner.branches_containing(node))
    }
    fn apply(&mut self, writes: &Writes) -> Result<(), Error> {
        if self.writes.get() {
            return Err(Error::Historique("disque plein".into()));
        }
        self.inner.apply(writes)
    }
}

// INV-TX-01 avec un historique faillible : une écriture ou une lecture en échec rend
// Error::Historique et laisse le dépôt identique, historique compris.
#[test]
fn a_failing_history_leaves_the_repository_unchanged() {
    let reads = Rc::new(Cell::new(false));
    let writes = Rc::new(Cell::new(false));
    let history = Failing {
        inner: MemoryHistory::default(),
        reads: Rc::clone(&reads),
        writes: Rc::clone(&writes),
    };
    let mut repo = Repository::open(history).unwrap();
    assert_eq!(repo.snapshot(), Repository::new().snapshot());
    let first = Envelope {
        op: OpId::from_bytes(raw(10)),
        expected: repo.expected(),
        reason: "motif".into(),
        command: create(DEFAULT_SCOPE, 1, Kind::Reasoning),
    };
    // Écriture impossible.
    writes.set(true);
    let control = repo.clone();
    assert_eq!(
        repo.execute(first.clone()),
        Err(Error::Historique("disque plein".into()))
    );
    assert_eq!(repo, control);
    // L'écriture rétablie, la même enveloppe passe.
    writes.set(false);
    let outcome = repo.execute(first.clone()).unwrap();
    assert_eq!(repo.commit_count(), 2);
    // Lecture impossible : répétition, annulation, état ancien, recherche dans les branches.
    reads.set(true);
    let control = repo.clone();
    let failure: Result<(), Error> = Err(Error::Historique("lecture impossible".into()));
    assert_eq!(repo.execute(first).map(|_| ()), failure.clone());
    let undo = Envelope {
        op: OpId::from_bytes(raw(11)),
        expected: repo.expected(),
        reason: String::new(),
        command: Command::Undo,
    };
    assert_eq!(repo.execute(undo).map(|_| ()), failure.clone());
    assert_eq!(repo.snapshot_at(outcome.head).map(|_| ()), failure.clone());
    assert_eq!(
        repo.branches_containing(NodeId::from_bytes(raw(1)))
            .map(|_| ()),
        failure.clone()
    );
    assert_eq!(
        repo.commit_reason(outcome.head).map(|_| ()),
        failure.clone()
    );
    assert_eq!(repo, control);
    // Ouverture impossible.
    let unreadable = Failing {
        inner: MemoryHistory::default(),
        reads: Rc::new(Cell::new(true)),
        writes: Rc::new(Cell::new(false)),
    };
    assert_eq!(Repository::open(unreadable).map(|_| ()), failure);
}

// Ouvrir un historique en mémoire redonne le dépôt qu'il contient ; le message d'une erreur
// d'historique nomme sa cause.
#[test]
fn opening_a_memory_history_gives_back_its_repository() {
    assert_eq!(
        Repository::open(MemoryHistory::default()),
        Ok(Repository::new())
    );
    let repo = every_command_repository();
    let reopened = Repository::open(repo.history().clone()).unwrap();
    assert_eq!(reopened, repo);
    assert_eq!(encode_repository(&reopened), encode_repository(&repo));
    assert_eq!(
        Error::Historique("disque plein".into()).to_string(),
        "historique indisponible : disque plein"
    );
}

// Annuler l'opération fondatrice d'une branche ramène sur la tête actuelle de son origine, même
// si l'origine a avancé depuis la bifurcation (état repris, aperçu et écritures compris).
#[test]
fn leaving_a_branch_returns_to_the_current_head_of_its_origin() {
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    let b = note(&mut r, Kind::Implementation, "b");
    r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    let experimental = r.invalidate(DEFAULT_SCOPE, a).unwrap();
    let base = r
        .repository()
        .history()
        .commit(r.head())
        .unwrap()
        .unwrap()
        .parents[0];
    // main avance après la bifurcation.
    r.checkout(MAIN).unwrap();
    note(
        &mut r,
        Kind::Category,
        "créée sur main après la bifurcation",
    );
    let main_head = r.head();
    assert_ne!(main_head, base);
    r.checkout(experimental).unwrap();
    let undo = Envelope {
        op: OpId::from_bytes(raw(900)),
        expected: r.repository().expected(),
        reason: String::new(),
        command: Command::Undo,
    };
    let expected_state = r.snapshot_at(main_head).unwrap();
    let writes = written(r.repository().prepare(&undo).unwrap());
    assert_eq!(writes.active, MAIN);
    assert_eq!(writes.state, expected_state);
    assert_eq!(writes.outcome.head, main_head);
    r.undo().unwrap();
    assert_eq!(r.active_branch(), MAIN);
    assert_eq!(r.snapshot(), &*expected_state);
}

// ---------------------------------------------------------------------------
// Incrément 5.2c — contrat ajouté : forme publique des lignes, morceaux du dépôt v2,
// vérification complète à mémoire bornée (ETAT-HISTORIQUE.md, sections 3 et 9 ; D21).
// ---------------------------------------------------------------------------

/// États couvrant toutes les sortes de lignes, révisions multiples et acquittements compris.
fn line_test_states() -> Vec<Snapshot> {
    vec![
        example_state(),
        rich_state(),
        every_command_repository().snapshot().clone(),
        Snapshot::default(),
    ]
}

// L'ordre lexicographique des clés textuelles est l'ordre canonique du texte : parcourues par
// clé, les lignes redonnent exactement les lignes de l'instantané v2 ; chaque ligne a sa clé.
#[test]
fn textual_line_keys_follow_the_canonical_order() {
    for state in line_test_states() {
        let lines = state_lines(&state);
        let text = encode_snapshot_v2(&state);
        let canonical: Vec<&str> = text.lines().skip(1).filter(|l| *l != "fin").collect();
        let by_key: Vec<&str> = lines.values().map(String::as_str).collect();
        assert_eq!(by_key, canonical);
        for (key, line) in &lines {
            assert_eq!(line_key(line).as_deref(), Some(key.as_str()), "{line}");
        }
    }
    // Forme des clés : notes, puis ancrages, puis contextes ; rang de révision sur 20 chiffres.
    let n = "a5000000-0000-0000-0000-000000000004";
    let s = "00000000-0000-0000-0000-000000000000";
    let keys: Vec<String> = state_lines(&example_state()).into_keys().collect();
    for expected in [
        format!("1/{n}/0"),
        format!("1/{n}/1/00000000000000000001"),
        "2/a5000000-0000-0000-0000-000000000006".to_owned(),
        format!("3/{s}/0"),
        format!("3/{s}/1/{n}"),
        format!("3/{s}/2/a5000000-0000-0000-0000-000000000005"),
    ] {
        assert!(
            keys.contains(&expected),
            "clé {expected} absente : {keys:?}"
        );
    }
    let rich = state_lines(&rich_state());
    assert!(rich
        .keys()
        .any(|k| k.starts_with("3/") && k.split('/').nth(2) == Some("3")));
    assert!(rich.keys().any(|k| k.ends_with("/1/00000000000000000002")));
    for no_key in ["fin", SNAPSHOT_HEADER_V2, "passage x", ""] {
        assert_eq!(line_key(no_key), None, "{no_key}");
    }
}

// Un état se relit depuis ses lignes ; des lignes hors de l'ordre canonique sont refusées.
#[test]
fn states_round_trip_through_their_lines() {
    for state in line_test_states() {
        let lines = state_lines(&state);
        assert_eq!(
            state_from_lines(lines.values().map(String::as_str)),
            Ok(state)
        );
    }
    let lines = state_lines(&example_state());
    let mut reversed: Vec<&str> = lines.values().map(String::as_str).collect();
    reversed.reverse();
    assert!(state_from_lines(reversed).is_err());
    // Différence par clés textuelles : couples changés seulement, dans l'ordre des clés.
    let before = state_lines(&example_state());
    let after = state_lines(&rich_state());
    let changes = state_difference(&before, &before);
    assert!(changes.removed.is_empty() && changes.added.is_empty());
    let changes = state_difference(&before, &after);
    for (key, line) in &changes.removed {
        assert_eq!(before.get(*key).map(String::as_str), Some(*line));
        assert_ne!(after.get(*key).map(String::as_str), Some(*line));
    }
    for (key, line) in &changes.added {
        assert_eq!(after.get(*key).map(String::as_str), Some(*line));
        assert_ne!(before.get(*key).map(String::as_str), Some(*line));
    }
    let mut patched = before.clone();
    for (key, _) in &changes.removed {
        patched.remove(*key);
    }
    for (key, line) in &changes.added {
        patched.insert((*key).to_owned(), (*line).to_owned());
    }
    assert_eq!(patched, after);
    let keys = |v: &[(&str, &str)]| v.iter().map(|(k, _)| k.to_string()).collect::<Vec<_>>();
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    assert_eq!(keys(&changes.removed), sorted(keys(&changes.removed)));
    assert_eq!(keys(&changes.added), sorted(keys(&changes.added)));
}

/// Texte v2 d'un dépôt assemblé de ses morceaux, comme le fera un stockage ligne par ligne.
fn assembled_from_pieces(repo: &Repository) -> String {
    let mut out = format!("{REPOSITORY_HEADER_V2}\n");
    out.push_str(&encode_active(repo.active_branch()));
    for branch in repo.branches() {
        let meta = repo.history().branches().unwrap()[&branch].clone();
        out.push_str(&encode_branch_v2(branch, &meta));
    }
    let text = encode_repository_v2(repo);
    let mut identities: Vec<(String, String)> = text
        .lines()
        .filter_map(|l| l.strip_prefix("identite "))
        .map(|rest| {
            let (category, id) = rest.split_once(' ').unwrap();
            let id = NodeId::parse(id).unwrap().to_bytes();
            let used = match category {
                "note" => UsedId::Node(NodeId::from_bytes(id)),
                "relation" => UsedId::Edge(EdgeId::from_bytes(id)),
                "contexte" => UsedId::Scope(ScopeId::from_bytes(id)),
                _ => UsedId::Anchor(AnchorId::from_bytes(id)),
            };
            (identity_key(used), encode_identity(used))
        })
        .collect();
    identities.sort();
    identities.dedup();
    for (_, line) in identities {
        out.push_str(&line);
    }
    // Commits par identifiant croissant : référence aux profondeurs multiples de 100.
    let mut depth = std::collections::BTreeMap::new();
    let mut ids: Vec<CommitId> = Vec::new();
    for (envelope, outcome) in repo.journal() {
        let _ = envelope;
        if outcome.published {
            ids.push(outcome.head);
        }
    }
    ids.push(ROOT);
    ids.sort();
    for id in &ids {
        let info = repo.history().commit(*id).unwrap().unwrap();
        let parent = info.parents.first().copied();
        let mut chain = vec![*id];
        while let Some(p) = repo
            .history()
            .commit(*chain.last().unwrap())
            .unwrap()
            .unwrap()
            .parents
            .first()
        {
            chain.push(*p);
        }
        depth.insert(*id, chain.len() - 1);
        let state = state_lines(&repo.snapshot_at(*id).unwrap());
        let block = match parent {
            Some(p) if !holds_reference(depth[id]) => {
                let before = state_lines(&repo.snapshot_at(p).unwrap());
                let changes = state_difference(&before, &state);
                let removed: Vec<String> =
                    changes.removed.iter().map(|(_, l)| l.to_string()).collect();
                let added: Vec<String> = changes.added.iter().map(|(_, l)| l.to_string()).collect();
                (removed, added, false)
            }
            _ => (state.values().cloned().collect(), Vec::new(), true),
        };
        let (a, b, reference) = block;
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        let b: Vec<&str> = b.iter().map(String::as_str).collect();
        let block = if reference {
            CommitBlock::Reference(&a)
        } else {
            CommitBlock::Difference {
                removed: &a,
                added: &b,
            }
        };
        out.push_str(&encode_commit_v2(*id, parent, &info.reason, block));
    }
    for (envelope, outcome) in repo.journal() {
        out.push_str(&encode_operation_v2(envelope, outcome));
    }
    out.push_str("fin\n");
    out
}

// Le texte v2 complet est exactement l'assemblage de ses morceaux, dans l'ordre canonique ; un
// stockage qui écrit les morceaux un à un n'a donc aucune seconde définition du format.
#[test]
fn a_v2_repository_is_assembled_from_its_pieces() {
    for repo in [
        example_repository(),
        every_command_repository(),
        linear_history(120),
    ] {
        assert_eq!(assembled_from_pieces(&repo), encode_repository_v2(&repo));
    }
    assert!(holds_reference(0) && holds_reference(100) && holds_reference(200));
    assert!(!holds_reference(1) && !holds_reference(99) && !holds_reference(101));
}

// Un bloc d'opération se relit à l'identique ; un bloc incomplet ou prolongé est refusé.
#[test]
fn operation_blocks_round_trip() {
    let repo = every_command_repository();
    let mut count = 0;
    for (envelope, outcome) in repo.journal() {
        let block = encode_operation_v2(envelope, outcome);
        assert_eq!(
            decode_operation_v2(&block),
            Ok((envelope.clone(), outcome.clone()))
        );
        count += 1;
    }
    assert!(count > 30);
    let (envelope, outcome) = repo.journal().next().unwrap();
    let block = encode_operation_v2(envelope, outcome);
    let lines = block.lines().count();
    assert_eq!(
        decode_operation_v2(block.trim_end_matches('\n')),
        Err(DecodeError {
            line: lines,
            reason: DecodeReason::MissingEnd
        })
    );
    assert_eq!(
        decode_operation_v2(&format!("{block}fin\n")),
        Err(DecodeError {
            line: lines + 1,
            reason: DecodeReason::AfterEnd
        })
    );
    assert_eq!(
        decode_operation_v2(&block.replacen("operation ", "operations ", 1)),
        Err(DecodeError {
            line: 1,
            reason: DecodeReason::Field("operation")
        })
    );
    // Bloc plus long (opération avec effets) : la ligne en trop est située juste après lui.
    let (envelope, outcome) = repo
        .journal()
        .find(|(_, o)| o.effects.review.len() >= 2)
        .unwrap();
    let long = encode_operation_v2(envelope, outcome);
    let long_lines = long.lines().count();
    assert!(long_lines > 4);
    assert_eq!(
        decode_operation_v2(&format!("{long}fin\n")),
        Err(DecodeError {
            line: long_lines + 1,
            reason: DecodeReason::AfterEnd
        })
    );
    let first_line = block.lines().next().unwrap();
    assert_eq!(
        decode_operation_v2(&format!("{first_line}\n")),
        Err(DecodeError {
            line: 2,
            reason: DecodeReason::Field("commande")
        })
    );
}

// La vérification complète accepte exactement ce que le décodage accepte, refuse avec la même
// ligne et le même motif, et rend l'état de la tête de chaque branche.
#[test]
fn full_verification_matches_decoding() {
    for repo in [
        example_repository(),
        every_command_repository(),
        linear_history(250),
    ] {
        let text = encode_repository_v2(&repo);
        let verified = verify_repository_v2(&text).unwrap();
        assert_eq!(verified.commits, repo.commit_count());
        assert_eq!(verified.heads.len(), repo.branch_count());
        for branch in repo.branches() {
            let head = repo.branch_head(branch).unwrap();
            assert_eq!(
                verified.heads[&branch],
                encode_snapshot_v2(&repo.snapshot_at(head).unwrap())
            );
        }
    }
    // Mêmes refus que le décodage : ligne de différence absente, référence mal placée, parent
    // absent, état invalide, identité non couverte, cohérence de branche, forme.
    let c = |n: u64| format!("00000000-0000-0001-0000-{n:012}");
    let block_7 = commit_block(REPOSITORY_EXAMPLE_V2, &c(7));
    let block_5 = commit_block(REPOSITORY_EXAMPLE_V2, &c(5));
    let ghost = "a5000000-0000-0000-0000-0000000000ff";
    let identity = REPOSITORY_EXAMPLE_V2
        .lines()
        .find(|l| l.starts_with("identite relation "))
        .unwrap();
    let main_line = REPOSITORY_EXAMPLE_V2
        .lines()
        .find(|l| l.starts_with(&format!("branche {MAIN} ")))
        .unwrap();
    let deviations = [
        REPOSITORY_EXAMPLE_V2.replacen(
            &block_7[1],
            &block_7[1].replace(" verdict non-verifiee ", " verdict verifiee "),
            1,
        ),
        REPOSITORY_EXAMPLE_V2.replacen("  difference\n", "  reference\n", 1),
        REPOSITORY_EXAMPLE_V2.replacen(
            &format!("parents 1 {} motif", c(1)),
            &format!("parents 1 {ghost} motif"),
            1,
        ),
        REPOSITORY_EXAMPLE_V2.replacen(
            &block_5[1],
            &block_5[1].replace(
                &format!("{} depend-de", c(2)),
                &format!("{ghost} depend-de"),
            ),
            1,
        ),
        REPOSITORY_EXAMPLE_V2.replacen(&format!("{identity}\n"), "", 1),
        REPOSITORY_EXAMPLE_V2.replacen(
            main_line,
            &main_line.replace(&format!("tete {}", c(5)), &format!("tete {ghost}")),
            1,
        ),
        REPOSITORY_EXAMPLE_V2.replacen("etat active", "etat abandonnee", 1),
        String::new(),
    ];
    for text in deviations {
        let decoded = decode_repository_v2(&text).map(|_| ());
        assert!(decoded.is_err(), "écart accepté : {text}");
        assert_eq!(verify_repository_v2(&text).map(|_| ()), decoded);
    }
}

// La vérification ne garde un état que tant qu'un de ses enfants reste à reconstruire : une
// seule table de lignes pour un historique linéaire, quelle que soit sa longueur ; deux quand
// deux branches avancent chacune depuis un même commit.
#[test]
fn full_verification_holds_few_states() {
    for length in [1, 99, 250] {
        let text = encode_repository_v2(&linear_history(length));
        assert_eq!(
            verify_repository_v2(&text).unwrap().peak_states,
            1,
            "{length}"
        );
    }
    // Un commit qui a deux enfants sans descendance : une seule table à la fois.
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    note(&mut r, Kind::Reasoning, "abandonnée");
    r.undo().unwrap();
    note(&mut r, Kind::Reasoning, "gardée");
    let _ = a;
    let text = encode_repository_v2(r.repository());
    assert_eq!(verify_repository_v2(&text).unwrap().peak_states, 1);
    // Deux branches qui avancent chacune depuis un même commit : deux tables à la fois.
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "a");
    let b = note(&mut r, Kind::Implementation, "b");
    r.link(DEFAULT_SCOPE, b, a, Relation::DependsOn).unwrap();
    r.invalidate(DEFAULT_SCOPE, a).unwrap();
    note(&mut r, Kind::Reasoning, "sur la branche");
    r.checkout(MAIN).unwrap();
    r.set_decision(DEFAULT_SCOPE, b, Decision::Retained)
        .unwrap();
    note(&mut r, Kind::Reasoning, "sur main");
    assert_eq!(r.branch_count(), 2);
    let text = encode_repository_v2(r.repository());
    assert_eq!(verify_repository_v2(&text).unwrap().peak_states, 2);
}

// Contenu stockable de chaque commit (ETAT-HISTORIQUE.md, section 3) : profondeur le long des
// premiers parents, état de référence aux profondeurs multiples de 100, différence avec le
// parent ailleurs ; appliquée aux lignes du parent, elle redonne exactement les lignes du commit.
#[test]
fn commit_records_rebuild_every_state() {
    for repo in [every_command_repository(), linear_history(230)] {
        let records: Vec<CommitRecord> = commit_records(&repo).collect();
        assert_eq!(records.len(), repo.commit_count());
        let ids: Vec<CommitId> = records.iter().map(|r| r.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "commits par identifiant croissant");
        let depth: std::collections::BTreeMap<CommitId, usize> =
            records.iter().map(|r| (r.id, r.depth)).collect();
        for record in &records {
            let info = repo.history().commit(record.id).unwrap().unwrap();
            assert_eq!(record.parent, info.parents.first().copied());
            assert_eq!(record.reason, info.reason);
            let expected_depth = record.parent.map_or(0, |p| depth[&p] + 1);
            assert_eq!(record.depth, expected_depth);
            let lines = state_lines(&repo.snapshot_at(record.id).unwrap());
            match (&record.content, record.parent) {
                (CommitContent::Reference(reference), _) => {
                    assert!(holds_reference(record.depth));
                    assert_eq!(reference, &lines);
                }
                (CommitContent::Difference { removed, added }, Some(parent)) => {
                    assert!(!holds_reference(record.depth));
                    let mut rebuilt = state_lines(&repo.snapshot_at(parent).unwrap());
                    for (key, line) in removed {
                        assert_eq!(rebuilt.remove(key).as_ref(), Some(line));
                    }
                    for (key, line) in added {
                        assert_eq!(rebuilt.insert(key.clone(), line.clone()), None);
                    }
                    assert_eq!(rebuilt, lines);
                }
                (CommitContent::Difference { .. }, None) => panic!("différence sans parent"),
            }
        }
        assert!(records
            .iter()
            .any(|r| matches!(r.content, CommitContent::Difference { .. })));
    }
    let records: Vec<CommitRecord> = commit_records(&linear_history(230)).collect();
    let references: Vec<usize> = records
        .iter()
        .filter(|r| matches!(r.content, CommitContent::Reference(_)))
        .map(|r| r.depth)
        .collect();
    assert_eq!(references, [0, 100, 200]);
}

// Les identités employées d'un historique en mémoire s'énumèrent, dans l'ordre canonique.
#[test]
fn used_identities_are_listed_in_canonical_order() {
    let repo = every_command_repository();
    let listed: Vec<String> = repo.history().used_ids().map(encode_identity).collect();
    let text = encode_repository_v2(&repo);
    let in_text: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with("identite "))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(listed, in_text);
    let keys: Vec<String> = repo.history().used_ids().map(identity_key).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

// ---------------------------------------------------------------------------
// Incrément 5.3 — opérations groupées (LOT.md, D22). Contrat écrit avant le code.
// ---------------------------------------------------------------------------

fn node_id(n: u64) -> NodeId {
    NodeId::from_bytes(raw(n))
}

fn edge_id(n: u64) -> EdgeId {
    EdgeId::from_bytes(raw(n))
}

fn change_of(command: Command) -> Change {
    match command {
        Command::Change(change) => change,
        other => panic!("modification attendue : {other:?}"),
    }
}

fn verdict_change(node: u64, verdict: Verdict) -> Change {
    Change::SetVerdict {
        scope: DEFAULT_SCOPE,
        node: node_id(node),
        verdict,
    }
}

/// Dépôt sur main : une catégorie (1) et un raisonnement (2), opérations 10 et 11.
fn lot_base() -> Repository {
    let mut repo = Repository::new();
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Category)),
        (11, create(DEFAULT_SCOPE, 2, Kind::Reasoning)),
    ] {
        let next = envelope(&repo, op, command);
        repo.execute(next).unwrap();
    }
    repo
}

/// Lot de R1 : crée la note 3, la classe dans la catégorie 1, la fait dépendre de la note 2.
fn create_and_link() -> Command {
    Command::Lot(vec![
        change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
        change_of(link(4, 3, 1, Relation::ClassifiedIn)),
        change_of(link(5, 3, 2, Relation::DependsOn)),
    ])
}

// INV-LOT-01 : un lot accepté publie un seul commit et une seule opération, emploie toutes ses
// identités, et donne l'état de ses éléments exécutés un à un ; un seul « annuler » le défait,
// un seul « rétablir » le refait.
#[test]
fn a_lot_creates_and_links_in_one_operation() {
    let mut repo = lot_base();
    let before = repo.clone();
    let lot = envelope(&repo, 20, create_and_link());
    let writes = written(repo.prepare(&lot).unwrap());
    assert_eq!(repo, before);
    assert_eq!(
        writes.identities,
        vec![
            UsedId::Node(node_id(3)),
            UsedId::Edge(edge_id(4)),
            UsedId::Edge(edge_id(5)),
        ]
    );
    let commit = CommitId::from_bytes(raw(20));
    assert_eq!(
        writes.commit.as_ref().map(|c| (c.id, c.parent)),
        Some((commit, before.head()))
    );
    let outcome = repo.execute(lot.clone()).unwrap();
    assert_eq!(outcome, writes.outcome);
    assert!(outcome.published && !outcome.forked);
    assert_eq!((outcome.branch, outcome.head), (MAIN, commit));
    assert_eq!(repo.commit_count(), before.commit_count() + 1);
    assert_eq!(repo.journal().count(), before.journal().count() + 1);
    // Même état que les trois commandes exécutées une à une.
    let mut one_by_one = before.clone();
    for (op, command) in [
        (30, create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
        (31, link(4, 3, 1, Relation::ClassifiedIn)),
        (32, link(5, 3, 2, Relation::DependsOn)),
    ] {
        let next = envelope(&one_by_one, op, command);
        one_by_one.execute(next).unwrap();
    }
    assert_eq!(repo.snapshot(), one_by_one.snapshot());
    let after = repo.snapshot().clone();
    let undo = envelope(&repo, 21, Command::Undo);
    repo.execute(undo).unwrap();
    assert_eq!(repo.snapshot(), before.snapshot());
    assert_eq!(repo.head(), before.head());
    let redo = envelope(&repo, 22, Command::Redo);
    repo.execute(redo).unwrap();
    assert_eq!(repo.snapshot(), &after);
    assert_eq!(repo.head(), commit);
    // Répétition : même résultat, rien de réappliqué (INV-RETRY-01).
    let state = repo.clone();
    assert_eq!(repo.execute(lot), Ok(outcome));
    assert_eq!(repo, state);
}

// INV-LOT-01, INV-TX-01 : le refus d'un élément refuse tout le lot, nomme l'élément (à partir
// de 1) et laisse le dépôt strictement identique ; une identité déjà prise par un élément
// précédent du même lot est refusée comme une identité déjà employée.
#[test]
fn a_refused_element_refuses_the_whole_lot() {
    let mut repo = lot_base();
    let control = repo.clone();
    let missing = node_id(99);
    let cases = [
        (
            vec![
                change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
                change_of(link(4, 3, 99, Relation::DependsOn)),
            ],
            Error::InLot {
                element: 2,
                cause: Box::new(Error::MissingNode(missing)),
            },
        ),
        (
            vec![
                change_of(link(4, 99, 1, Relation::ClassifiedIn)),
                change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
            ],
            Error::InLot {
                element: 1,
                cause: Box::new(Error::MissingNode(missing)),
            },
        ),
        (
            vec![
                change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
                change_of(link(4, 3, 1, Relation::ClassifiedIn)),
                change_of(create(DEFAULT_SCOPE, 3, Kind::Implementation)),
            ],
            Error::InLot {
                element: 3,
                cause: Box::new(Error::NodeIdInUse(node_id(3))),
            },
        ),
        (
            vec![
                change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
                change_of(link(4, 3, 1, Relation::ClassifiedIn)),
                change_of(link(4, 3, 2, Relation::DependsOn)),
            ],
            Error::InLot {
                element: 3,
                cause: Box::new(Error::EdgeIdInUse(edge_id(4))),
            },
        ),
        (
            vec![change_of(create(DEFAULT_SCOPE, 2, Kind::Reasoning))],
            Error::InLot {
                element: 1,
                cause: Box::new(Error::NodeIdInUse(node_id(2))),
            },
        ),
        (
            vec![
                change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
                Change::CreateNode {
                    scope: DEFAULT_SCOPE,
                    id: NodeId::from_bytes([0; 16]),
                    kind: Kind::Reasoning,
                    title: "nul".into(),
                    text: "nul".into(),
                },
            ],
            Error::InLot {
                element: 2,
                cause: Box::new(Error::NilIdentifier),
            },
        ),
        (
            vec![
                change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
                change_of(link(4, 3, 3, Relation::DependsOn)),
            ],
            Error::InLot {
                element: 2,
                cause: Box::new(Error::SelfRelation),
            },
        ),
    ];
    for (k, (elements, refusal)) in cases.into_iter().enumerate() {
        let lot = reasoned(&repo, 40 + k as u64, Command::Lot(elements));
        assert_eq!(repo.preview(&lot.command), Err(refusal.clone()), "cas {k}");
        assert_eq!(repo.prepare(&lot), Err(refusal.clone()), "cas {k}");
        assert_eq!(repo.execute(lot), Err(refusal), "cas {k}");
        assert_eq!(repo, control, "cas {k}");
    }
    assert_eq!(
        Error::InLot {
            element: 2,
            cause: Box::new(Error::MissingNode(missing)),
        }
        .to_string(),
        format!("élément 2 du lot : note absente : {missing}")
    );
}

// LOT.md, section 2 : un lot vide et une restauration dans un lot sont refusés ; un lot qui
// contient un changement de statut exige un motif, comme ce changement seul (D12).
#[test]
fn the_shape_of_a_lot_is_checked() {
    let mut repo = lot_base();
    let control = repo.clone();
    let empty = reasoned(&repo, 50, Command::Lot(vec![]));
    assert_eq!(repo.execute(empty), Err(Error::EmptyLot));
    assert_eq!(repo.preview(&Command::Lot(vec![])), Err(Error::EmptyLot));
    let restore = reasoned(
        &repo,
        51,
        Command::Lot(vec![
            change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
            Change::Restore { target: ROOT },
        ]),
    );
    assert_eq!(repo.execute(restore), Err(Error::NotInLot));
    let status = Command::Lot(vec![
        change_of(create(DEFAULT_SCOPE, 3, Kind::Reasoning)),
        verdict_change(3, Verdict::Verified),
    ]);
    assert_eq!(
        repo.execute(envelope(&repo, 52, status.clone())),
        Err(Error::MissingReason)
    );
    assert_eq!(repo, control);
    assert_eq!(Error::EmptyLot.to_string(), "lot vide");
    assert_eq!(
        Error::NotInLot.to_string(),
        "une restauration ne peut pas faire partie d'un lot"
    );
    let outcome = repo.execute(reasoned(&repo, 53, status)).unwrap();
    assert!(outcome.published);
    assert_eq!(repo.commit_count(), control.commit_count() + 1);
}

// INV-LOT-02, D13 : la criticité d'un lot se juge sur son effet net. Un lot qui écarte une note
// crée une seule branche, celle de l'opération, avec un seul commit ; un lot dont un élément
// serait critique seul, mais dont l'effet net ne l'est pas, ne bifurque pas ; un lot sans effet
// net ne publie rien. L'aperçu annonce exactement ce que l'exécution fait (INV-PREVIEW-01).
#[test]
fn a_lot_is_judged_on_its_net_effect() {
    // Critique : la note créée est aussitôt réfutée, et une autre en dépend.
    let mut repo = lot_base();
    let before = repo.clone();
    let critical = Command::Lot(vec![
        change_of(create(DEFAULT_SCOPE, 3, Kind::Implementation)),
        change_of(link(4, 3, 2, Relation::DependsOn)),
        verdict_change(2, Verdict::Refuted),
    ]);
    let preview = repo.preview(&critical).unwrap();
    assert!(preview.publishes && preview.forks);
    assert!(!preview.effects.review.is_empty());
    let outcome = repo.execute(reasoned(&repo, 60, critical)).unwrap();
    assert!(outcome.published && outcome.forked);
    assert_eq!(outcome.effects, preview.effects);
    let branch = BranchId::from_bytes(raw(60));
    assert_eq!(outcome.branch, branch);
    assert_eq!(repo.branch_count(), before.branch_count() + 1);
    assert_eq!(repo.commit_count(), before.commit_count() + 1);
    assert_eq!(repo.branch_head(MAIN), Ok(before.head()));
    // Un seul « annuler » ramène sur main, dans l'état d'avant.
    repo.execute(envelope(&repo, 61, Command::Undo)).unwrap();
    assert_eq!(repo.active_branch(), MAIN);
    assert_eq!(repo.snapshot(), before.snapshot());

    // Effet net non critique : la réfutation est levée dans le même lot.
    let mut repo = lot_base();
    let net = Command::Lot(vec![
        change_of(create(DEFAULT_SCOPE, 3, Kind::Implementation)),
        change_of(link(4, 3, 2, Relation::DependsOn)),
        verdict_change(2, Verdict::Refuted),
        verdict_change(2, Verdict::Unverified),
    ]);
    let preview = repo.preview(&net).unwrap();
    assert!(preview.publishes && !preview.forks);
    assert!(preview.effects.review.is_empty());
    let outcome = repo.execute(reasoned(&repo, 62, net)).unwrap();
    assert!(outcome.published && !outcome.forked);
    assert_eq!(outcome.branch, MAIN);
    // Seul, le troisième élément serait critique.
    let alone = Command::Change(verdict_change(2, Verdict::Refuted));
    assert!(repo.preview(&alone).unwrap().forks);

    // Aucun effet net : rien n'est publié ni journalisé (INV-MOD-04).
    let mut repo = lot_base();
    let control = repo.clone();
    let nothing = Command::Lot(vec![
        verdict_change(2, Verdict::Verified),
        verdict_change(2, Verdict::Unverified),
    ]);
    let preview = repo.preview(&nothing).unwrap();
    assert!(!preview.publishes && !preview.forks);
    let outcome = repo.execute(reasoned(&repo, 63, nothing)).unwrap();
    assert!(!outcome.published);
    assert_eq!(repo, control);
}

// INV-LOT-03, INV-ID-02 : les identités d'un lot annulé restent employées : ni un autre lot ni
// une commande seule ne peuvent les reprendre.
#[test]
fn identities_of_a_lot_are_never_reused() {
    let mut repo = lot_base();
    repo.execute(envelope(&repo, 70, create_and_link()))
        .unwrap();
    repo.execute(envelope(&repo, 71, Command::Undo)).unwrap();
    let control = repo.clone();
    let again = envelope(&repo, 72, create_and_link());
    assert_eq!(
        repo.execute(again),
        Err(Error::InLot {
            element: 1,
            cause: Box::new(Error::NodeIdInUse(node_id(3))),
        })
    );
    let edge = envelope(&repo, 73, link(5, 2, 1, Relation::ClassifiedIn));
    assert_eq!(repo.execute(edge), Err(Error::EdgeIdInUse(edge_id(5))));
    assert_eq!(repo, control);
    for id in [
        UsedId::Node(node_id(3)),
        UsedId::Edge(edge_id(4)),
        UsedId::Edge(edge_id(5)),
    ] {
        assert_eq!(repo.history().is_used(id), Ok(true), "{id:?}");
    }
}

/// Bloc `operation` attendu pour le lot de R1 (LOT.md, section 3), écrit d'après la spécification.
fn expected_lot_block() -> String {
    let scope = "00000000-0000-0000-0000-000000000000";
    let id = |n: u64| format!("a5000000-0000-0000-0000-{n:012x}");
    format!(
        "operation {op} attendu {scope} {head} motif \"\"\n\
         \x20 commande lot 3\n\
         \x20   element creer-note portee {scope} note {n3} role raisonnement titre \"titre\" texte \"texte\"\n\
         \x20   element lier portee {scope} relation {e4} origine {n3} destination {n1} type classe-dans\n\
         \x20   element lier portee {scope} relation {e5} origine {n3} destination {n2} type depend-de\n\
         \x20 resultat branche {scope} tete {op} publie oui bifurque non\n",
        op = id(20),
        head = id(11),
        n1 = id(1),
        n2 = id(2),
        n3 = id(3),
        e4 = id(4),
        e5 = id(5),
    )
}

// LOT.md, section 3 : le bloc d'un lot a la forme réservée par VERSION2.md ; il se relit à
// l'identique, et le dépôt qui le contient aussi.
#[test]
fn lot_blocks_are_written_as_specified_and_round_trip() {
    let mut repo = lot_base();
    let lot = envelope(&repo, 20, create_and_link());
    let outcome = repo.execute(lot.clone()).unwrap();
    let block = encode_operation_v2(&lot, &outcome);
    assert_eq!(block, expected_lot_block());
    assert_eq!(decode_operation_v2(&block), Ok((lot, outcome)));
    let text = encode_repository_v2(&repo);
    assert_eq!(decode_repository_v2(&text), Ok(repo.clone()));
    assert_eq!(
        verify_repository_v2(&text).map(|v| v.commits),
        Ok(repo.commit_count())
    );
}

// LOT.md, section 3 : refus à la lecture d'un bloc de lot mal formé, situés à leur ligne.
#[test]
fn malformed_lot_blocks_are_refused() {
    let block = expected_lot_block();
    let lines: Vec<&str> = block.lines().collect();
    let with = |number: usize, line: &str| {
        let mut changed = lines.clone();
        changed[number - 1] = line;
        changed.join("\n") + "\n"
    };
    let scope = "00000000-0000-0000-0000-000000000000";
    let n3 = "a5000000-0000-0000-0000-000000000003";
    let duplicate = format!(
        "    element creer-note portee {scope} note {n3} role raisonnement titre \"titre\" texte \"texte\""
    );
    let cases: Vec<(String, DecodeError)> = vec![
        (
            with(2, "  commande lot 0"),
            DecodeError {
                line: 2,
                reason: DecodeReason::Value("lot"),
            },
        ),
        // Annonce de quatre éléments pour trois lignes : la ligne « resultat » n'est pas un élément.
        (
            with(2, "  commande lot 4"),
            DecodeError {
                line: 6,
                reason: DecodeReason::Field("element"),
            },
        ),
        (
            with(3, "    element annuler"),
            DecodeError {
                line: 3,
                reason: DecodeReason::Value("element"),
            },
        ),
        (
            with(3, &format!("    element restaurer cible {scope}")),
            DecodeError {
                line: 3,
                reason: DecodeReason::Value("element"),
            },
        ),
        (
            with(3, "    element lot 1"),
            DecodeError {
                line: 3,
                reason: DecodeReason::Value("element"),
            },
        ),
        (
            with(3, &lines[2].replace("    element ", "    elements ")),
            DecodeError {
                line: 3,
                reason: DecodeReason::Field("element"),
            },
        ),
        (
            with(4, &duplicate),
            DecodeError {
                line: 4,
                reason: DecodeReason::Inconsistent("identités du lot"),
            },
        ),
    ];
    for (text, refusal) in cases {
        assert_eq!(decode_operation_v2(&text), Err(refusal), "{text}");
    }
}

// ---------------------------------------------------------------------------
// Incrément 5.4a — contrat ajouté : abandon et réactivation d'une branche (FUSION.md,
// section 9.7 ; D19, D23 ; INV-BR-05).
// ---------------------------------------------------------------------------

fn branch_id(n: u64) -> BranchId {
    BranchId::from_bytes(raw(n))
}

fn abandon(n: u64) -> Command {
    Command::Abandon {
        branch: branch_id(n),
    }
}

fn reactivate(n: u64) -> Command {
    Command::Reactivate {
        branch: branch_id(n),
    }
}

/// main porte deux notes liées (opérations 10 à 12) ; la réfutation de la première (opération
/// 13) bifurque : la branche expérimentale `branch_id(13)` est active.
fn forked() -> Repository {
    let mut repo = Repository::new();
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Reasoning)),
        (11, create(DEFAULT_SCOPE, 2, Kind::Implementation)),
        (12, link(3, 2, 1, Relation::DependsOn)),
        (13, Command::Change(verdict_change(1, Verdict::Refuted))),
    ] {
        let next = reasoned(&repo, op, command);
        repo.execute(next).unwrap();
    }
    assert_eq!(repo.active_branch(), branch_id(13));
    repo
}

// INV-BR-05 : abandonner une branche ne supprime rien ; la réactiver la rend telle qu'elle était,
// sans la rendre active. Ni l'un ni l'autre ne publie de commit ni n'exige de motif.
#[test]
fn abandoning_a_branch_keeps_it_and_reactivation_restores_it() {
    let mut repo = forked();
    let x = branch_id(13);
    let checkout = envelope(&repo, 14, Command::Checkout { branch: MAIN });
    repo.execute(checkout).unwrap();
    let branches = repo.history().branches().unwrap();
    let commits = repo.commit_count();
    let main_state = repo.snapshot().clone();
    let x_head = branches[&x].head;
    let control = repo.clone();
    let preview = repo.preview(&abandon(13)).unwrap();
    assert_eq!(
        (preview.publishes, preview.forks, preview.effects),
        (false, false, Effects::default())
    );
    let next = envelope(&repo, 15, abandon(13));
    let writes = written(repo.prepare(&next).unwrap());
    assert_eq!(repo, control);
    assert_eq!(writes.commit, None);
    assert_eq!(
        writes.branches,
        BTreeMap::from([(
            x,
            Branch {
                abandoned: true,
                ..branches[&x].clone()
            }
        )])
    );
    assert_eq!((writes.active, writes.identities.clone()), (MAIN, vec![]));
    assert_eq!(*writes.state, main_state);
    let outcome = repo.execute(next.clone()).unwrap();
    assert_eq!(outcome, writes.outcome);
    assert_eq!(
        outcome,
        Outcome {
            op: OpId::from_bytes(raw(15)),
            branch: MAIN,
            head: control.head(),
            published: false,
            forked: false,
            effects: Effects::default(),
        }
    );
    // Rien n'est supprimé (D6) : tête, base, origine et état de la branche restent lisibles.
    assert_eq!(repo.is_abandoned(x), Ok(true));
    assert_eq!(repo.is_abandoned(MAIN), Ok(false));
    assert_eq!(repo.branches().collect::<Vec<_>>(), vec![MAIN, x]);
    assert_eq!(repo.branch_count(), 2);
    assert_eq!(repo.branch_head(x), Ok(x_head));
    assert_eq!(repo.commit_count(), commits);
    assert_eq!(repo.snapshot_at(x_head), control.snapshot_at(x_head));
    assert_eq!(repo.snapshot(), &main_state);
    // Répétition : le résultat enregistré, rien de réappliqué (INV-RETRY-01).
    let after = repo.clone();
    assert_eq!(repo.execute(next), Ok(outcome));
    assert_eq!(repo, after);
    // Réactivation : la branche redevient ordinaire, et la branche active ne change pas.
    let back = envelope(&repo, 16, reactivate(13));
    let preview = repo.preview(&back.command).unwrap();
    assert_eq!(
        (preview.publishes, preview.forks, preview.effects),
        (false, false, Effects::default())
    );
    let writes = written(repo.prepare(&back).unwrap());
    assert_eq!(writes.branches, BTreeMap::from([(x, branches[&x].clone())]));
    assert_eq!((writes.active, writes.commit.clone()), (MAIN, None));
    let outcome = repo.execute(back).unwrap();
    assert_eq!(outcome, writes.outcome);
    assert_eq!((outcome.branch, outcome.head), (MAIN, control.head()));
    assert!(!outcome.published && !outcome.forked);
    assert_eq!(repo.is_abandoned(x), Ok(false));
    assert_eq!(repo.history().branches(), Ok(branches));
    assert_eq!(repo.active_branch(), MAIN);
    assert_eq!(repo.commit_count(), commits);
    // Deux opérations journalisées, sans commit.
    let commands: Vec<Command> = repo.journal().map(|(e, _)| e.command.clone()).collect();
    assert_eq!(
        &commands[commands.len() - 2..],
        &[abandon(13), reactivate(13)]
    );
}

// FUSION.md, section 9.7 : abandonner la branche active ramène sur son origine, avec les effets
// de ce retour sur les réexamens ; `annuler` ne défait pas un abandon.
#[test]
fn abandoning_the_active_branch_returns_to_its_origin() {
    let mut repo = forked();
    let x = branch_id(13);
    let main_head = repo.branch_head(MAIN).unwrap();
    let main_state = repo.snapshot_at(main_head).unwrap();
    let main_branch = repo.history().branches().unwrap()[&MAIN].clone();
    // Sur main, la note 1 n'est pas réfutée : la cause de réexamen de la note 2 disparaît.
    let preview = repo.preview(&abandon(13)).unwrap();
    assert!(!preview.publishes && !preview.forks);
    assert_eq!(
        preview.effects.review.keys().copied().collect::<Vec<_>>(),
        vec![(DEFAULT_SCOPE, node_id(2))]
    );
    let next = envelope(&repo, 14, abandon(13));
    let writes = written(repo.prepare(&next).unwrap());
    // Seule la branche abandonnée est écrite : main ne reçoit aucune entrée de rétablissement.
    assert_eq!(writes.branches.keys().copied().collect::<Vec<_>>(), vec![x]);
    assert!(writes.branches[&x].abandoned);
    assert_eq!(writes.active, MAIN);
    assert_eq!(writes.state, main_state);
    let outcome = repo.execute(next).unwrap();
    assert_eq!(
        (
            outcome.branch,
            outcome.head,
            outcome.published,
            outcome.forked
        ),
        (MAIN, main_head, false, false)
    );
    assert_eq!(outcome.effects, preview.effects);
    assert_eq!(repo.active_branch(), MAIN);
    assert_eq!(repo.snapshot(), &*main_state);
    assert_eq!(repo.history().branches().unwrap()[&MAIN], main_branch);
    // `annuler` annule la dernière opération de main (le lien), pas l'abandon.
    repo.execute(envelope(&repo, 15, Command::Undo)).unwrap();
    assert_eq!(
        (repo.active_branch(), repo.head()),
        (MAIN, CommitId::from_bytes(raw(11)))
    );
    assert_eq!(repo.is_abandoned(x), Ok(true));
}

// INV-BR-05 : une branche abandonnée ne redevient active ni par `basculer` ni par `rétablir` ;
// réactivée, elle se rejoint comme avant, la pile de rétablissement de main étant conservée.
#[test]
fn an_abandoned_branch_cannot_become_active_until_reactivated() {
    let mut repo = forked();
    let x = branch_id(13);
    // Annuler l'opération fondatrice ramène sur main, d'où `rétablir` rentrerait dans x.
    repo.execute(envelope(&repo, 14, Command::Undo)).unwrap();
    assert_eq!(repo.active_branch(), MAIN);
    repo.execute(envelope(&repo, 15, abandon(13))).unwrap();
    let control = repo.clone();
    for command in [Command::Checkout { branch: x }, Command::Redo] {
        assert_eq!(
            repo.preview(&command).map(|_| ()),
            Err(Error::BranchAbandoned(x)),
            "{command:?}"
        );
        let refused = envelope(&repo, 16, command.clone());
        assert_eq!(
            repo.execute(refused),
            Err(Error::BranchAbandoned(x)),
            "{command:?}"
        );
        assert_eq!(repo, control);
    }
    repo.execute(envelope(&repo, 17, reactivate(13))).unwrap();
    let outcome = repo.execute(envelope(&repo, 18, Command::Redo)).unwrap();
    assert_eq!(
        (outcome.branch, outcome.head),
        (x, CommitId::from_bytes(raw(13)))
    );
    repo.execute(envelope(&repo, 19, Command::Checkout { branch: MAIN }))
        .unwrap();
    repo.execute(envelope(&repo, 20, abandon(13))).unwrap();
    let refused = envelope(&repo, 21, Command::Checkout { branch: x });
    assert_eq!(repo.execute(refused), Err(Error::BranchAbandoned(x)));
    repo.execute(envelope(&repo, 22, reactivate(13))).unwrap();
    let outcome = repo
        .execute(envelope(&repo, 23, Command::Checkout { branch: x }))
        .unwrap();
    assert_eq!(outcome.branch, x);
}

// FUSION.md, section 9.7 : main ne s'abandonne pas ; on n'abandonne pas deux fois, on ne
// réactive qu'une branche abandonnée. Chaque refus laisse le dépôt identique (INV-TX-01).
#[test]
fn meaningless_abandons_and_reactivations_are_refused() {
    let mut repo = forked();
    let x = branch_id(13);
    let ghost = branch_id(99);
    repo.execute(envelope(&repo, 14, abandon(13))).unwrap();
    let control = repo.clone();
    let cases = [
        (Command::Abandon { branch: MAIN }, Error::MainNotAbandonable),
        (abandon(13), Error::BranchAbandoned(x)),
        (abandon(99), Error::MissingBranch(ghost)),
        (
            Command::Reactivate { branch: MAIN },
            Error::BranchNotAbandoned(MAIN),
        ),
        (reactivate(99), Error::MissingBranch(ghost)),
    ];
    for (command, refusal) in cases {
        assert_eq!(
            repo.preview(&command).map(|_| ()),
            Err(refusal.clone()),
            "{command:?}"
        );
        let refused = envelope(&repo, 15, command.clone());
        assert_eq!(repo.execute(refused), Err(refusal), "{command:?}");
        assert_eq!(repo, control);
    }
    assert_eq!(repo.is_abandoned(ghost), Err(Error::MissingBranch(ghost)));
    // Un motif peut accompagner l'opération ; il est gardé au journal.
    let back = reasoned(&repo, 16, reactivate(13));
    repo.execute(back.clone()).unwrap();
    let refused = envelope(&repo, 17, reactivate(13));
    assert_eq!(repo.execute(refused), Err(Error::BranchNotAbandoned(x)));
    assert_eq!(repo.journal().last().map(|(e, _)| e.clone()), Some(back));
    assert_eq!(
        Error::MainNotAbandonable.to_string(),
        "main ne peut pas être abandonnée"
    );
    assert_eq!(
        Error::BranchAbandoned(x).to_string(),
        format!("branche abandonnée : {x}")
    );
    assert_eq!(
        Error::BranchNotAbandoned(x).to_string(),
        format!("branche non abandonnée : {x}")
    );
}

// Une note n'est pas cherchée dans une branche abandonnée, dont l'état courant n'est plus tenu
// (VERSION2.md, section 3) ; elle l'est de nouveau après réactivation.
#[test]
fn notes_are_not_looked_for_in_abandoned_branches() {
    let mut repo = forked();
    let x = branch_id(13);
    let only_on_x = reasoned(&repo, 14, create(DEFAULT_SCOPE, 4, Kind::Reasoning));
    repo.execute(only_on_x).unwrap();
    assert_eq!(repo.branches_containing(node_id(4)), Ok(vec![x]));
    assert_eq!(repo.branches_containing(node_id(1)), Ok(vec![MAIN, x]));
    repo.execute(envelope(&repo, 15, abandon(13))).unwrap();
    assert_eq!(repo.branches_containing(node_id(4)), Ok(vec![]));
    assert_eq!(repo.branches_containing(node_id(1)), Ok(vec![MAIN]));
    repo.execute(envelope(&repo, 16, reactivate(13))).unwrap();
    assert_eq!(repo.branches_containing(node_id(4)), Ok(vec![x]));
}

/// Identifiant textuel de `raw(n)`.
fn raw_text(n: u64) -> String {
    format!("a5000000-0000-0000-0000-{n:012x}")
}

// VERSION2.md, section 6 : blocs `abandonner` et `reactiver`, état « abandonnee » d'une branche ;
// tout se relit à l'identique, dépôt complet compris.
#[test]
fn abandon_blocks_are_written_as_specified_and_round_trip() {
    let mut repo = forked();
    repo.execute(envelope(&repo, 14, Command::Checkout { branch: MAIN }))
        .unwrap();
    let main = MAIN.to_string();
    let line = |etat: &str| {
        format!(
            "branche {x} tete {x} origine {main} base {base} etat {etat}",
            x = raw_text(13),
            base = raw_text(12),
        )
    };
    for (op, command, word, etat) in [
        (15, abandon(13), "abandonner", "abandonnee"),
        (16, reactivate(13), "reactiver", "active"),
    ] {
        let next = Envelope {
            reason: "plus utile".into(),
            ..envelope(&repo, op, command)
        };
        let outcome = repo.execute(next.clone()).unwrap();
        let block = encode_operation_v2(&next, &outcome);
        assert_eq!(
            block,
            format!(
                "operation {op} attendu {main} {head} motif \"plus utile\"\n\
                 \x20 commande {word} branche {x}\n\
                 \x20 resultat branche {main} tete {head} publie non bifurque non\n",
                op = raw_text(op),
                head = raw_text(12),
                x = raw_text(13),
            )
        );
        assert_eq!(decode_operation_v2(&block), Ok((next, outcome)));
        let text = encode_repository_v2(&repo);
        assert!(text.contains(&format!("\n{}\n", line(etat))), "{text}");
        assert_eq!(decode_repository_v2(&text), Ok(repo.clone()));
        assert_eq!(
            verify_repository_v2(&text).map(|v| v.commits),
            Ok(repo.commit_count())
        );
    }
    // Abandon de la branche active : le bloc porte les effets du retour sur main.
    let mut repo = forked();
    let next = envelope(&repo, 14, abandon(13));
    let outcome = repo.execute(next.clone()).unwrap();
    assert!(!outcome.effects.review.is_empty());
    let block = encode_operation_v2(&next, &outcome);
    assert!(block.contains("\n  effet "), "{block}");
    assert_eq!(decode_operation_v2(&block), Ok((next, outcome)));
    let text = encode_repository_v2(&repo);
    assert_eq!(decode_repository_v2(&text), Ok(repo));
}

// Règles de lecture : main n'est jamais abandonnée, la branche active non plus ; un abandon ne
// fait pas partie d'un lot ; la commande nomme sa branche.
#[test]
fn abandon_rules_are_enforced_when_reading() {
    let mut repo = forked();
    repo.execute(envelope(&repo, 14, Command::Checkout { branch: MAIN }))
        .unwrap();
    repo.execute(envelope(&repo, 15, abandon(13))).unwrap();
    let text = encode_repository_v2(&repo);
    let main = MAIN.to_string();
    let inconsistent = |rule| {
        Err(DecodeError {
            line: 0,
            reason: DecodeReason::Inconsistent(rule),
        })
    };
    let main_line = text
        .lines()
        .find(|l| l.starts_with(&format!("branche {main} ")))
        .unwrap();
    let main_abandoned = text.replacen(
        main_line,
        &main_line.replace(" etat active", " etat abandonnee"),
        1,
    );
    let active_abandoned = text.replacen(
        &format!("\nactive {main}\n"),
        &format!("\nactive {}\n", raw_text(13)),
        1,
    );
    for (changed, rule) in [
        (main_abandoned, "branche main"),
        (active_abandoned, "branche active"),
    ] {
        assert_ne!(changed, text);
        assert_eq!(
            decode_repository_v2(&changed).map(|_| ()),
            inconsistent(rule)
        );
        assert_eq!(
            verify_repository_v2(&changed).map(|_| ()),
            inconsistent(rule)
        );
    }
    let block = |command: &str| {
        format!(
            "operation {op} attendu {main} {head} motif \"\"\n{command}\n\
             \x20 resultat branche {main} tete {head} publie non bifurque non\n",
            op = raw_text(30),
            head = raw_text(12),
        )
    };
    let x = raw_text(13);
    let cases = [
        (
            block(&format!(
                "  commande lot 1\n    element abandonner branche {x}"
            )),
            3,
            DecodeReason::Value("element"),
        ),
        (
            block(&format!(
                "  commande lot 1\n    element reactiver branche {x}"
            )),
            3,
            DecodeReason::Value("element"),
        ),
        (
            block("  commande abandonner"),
            2,
            DecodeReason::Field("branche"),
        ),
        (
            block(&format!("  commande reactiver branche {x} motif")),
            2,
            DecodeReason::Field("fin de ligne"),
        ),
    ];
    for (text, line, reason) in cases {
        assert_eq!(
            decode_operation_v2(&text),
            Err(DecodeError { line, reason }),
            "{text}"
        );
    }
}

// ---------------------------------------------------------------------------
// Incrément 5.4b — contrat ajouté : fusion à trois voies (FUSION.md, sections 2 à 8 et 9.1 à
// 9.6 ; D19, D23 ; INV-MERGE-01 à 03).
// ---------------------------------------------------------------------------

fn commit_id(n: u64) -> CommitId {
    CommitId::from_bytes(raw(n))
}

fn merge(source: u64, head: u64, resolutions: Vec<Resolution>) -> Command {
    Command::Merge {
        source: branch_id(source),
        source_head: commit_id(head),
        resolutions,
    }
}

fn checkout(repo: &mut Repository, op: u64, branch: BranchId) {
    let next = envelope(repo, op, Command::Checkout { branch });
    repo.execute(next).unwrap();
}

fn run(repo: &mut Repository, op: u64, command: Command) -> Outcome {
    let next = reasoned(repo, op, command);
    repo.execute(next).unwrap()
}

fn revise(node: u64, text: &str) -> Command {
    Command::Change(Change::Revise {
        node: node_id(node),
        text: text.into(),
        kind: RevisionKind::Minor,
    })
}

fn set_decision(node: u64, decision: Decision) -> Command {
    Command::Change(Change::SetDecision {
        scope: DEFAULT_SCOPE,
        node: node_id(node),
        decision,
    })
}

fn acknowledge(dependent: u64, cause: u64) -> Command {
    Command::Change(Change::Acknowledge {
        scope: DEFAULT_SCOPE,
        dependent: node_id(dependent),
        cause: node_id(cause),
    })
}

fn member(repo: &Repository, n: u64) -> Node {
    repo.snapshot()
        .graph(DEFAULT_SCOPE)
        .unwrap()
        .node(node_id(n))
        .unwrap()
        .clone()
}

// INV-MERGE-02 : des changements indépendants des deux côtés sont réunis, champ par champ, sans
// question ; la fusion publie un commit à deux parents (la cible d'abord), une seule opération
// qu'un seul `annuler` défait ; la source n'est jamais modifiée ; une fusion ne bifurque jamais.
#[test]
fn independent_changes_are_merged_in_one_undoable_commit() {
    let mut repo = forked();
    let x = branch_id(13);
    run(&mut repo, 14, create(DEFAULT_SCOPE, 4, Kind::Category));
    run(
        &mut repo,
        15,
        Command::Change(Change::SetImplementation {
            scope: DEFAULT_SCOPE,
            node: node_id(2),
            implementation: Implementation::InProgress,
        }),
    );
    checkout(&mut repo, 16, MAIN);
    run(&mut repo, 17, create(DEFAULT_SCOPE, 5, Kind::PersonalNote));
    run(&mut repo, 18, set_decision(2, Decision::Retained));
    let target_state = repo.snapshot().clone();
    let control = repo.clone();
    let plan = repo.merge_plan(x, &[]).unwrap();
    assert_eq!(repo, control);
    assert_eq!(
        (plan.base, plan.target_head, plan.source_head),
        (commit_id(12), commit_id(18), commit_id(15))
    );
    assert!(!plan.up_to_date);
    assert_eq!(plan.conflicts, vec![]);
    assert_eq!(plan.structural, None);
    // Note 4 et son membre, verdict de la note 1, implémentation de la note 2.
    assert_eq!(plan.from_source, 4);
    // La réfutation venue de la source fait de la note 2 une note à réexaminer.
    assert_eq!(
        plan.effects.review.keys().copied().collect::<Vec<_>>(),
        vec![(DEFAULT_SCOPE, node_id(2))]
    );
    let command = merge(13, 15, vec![]);
    let preview = repo.preview(&command).unwrap();
    assert_eq!(
        (preview.publishes, preview.forks, &preview.effects),
        (true, false, &plan.effects)
    );
    let outcome = run(&mut repo, 19, command.clone());
    assert_eq!(
        (
            outcome.branch,
            outcome.head,
            outcome.published,
            outcome.forked
        ),
        (MAIN, commit_id(19), true, false)
    );
    assert_eq!(outcome.effects, plan.effects);
    assert_eq!(
        repo.history()
            .commit(commit_id(19))
            .unwrap()
            .unwrap()
            .parents,
        vec![commit_id(18), commit_id(15)]
    );
    assert_eq!(member(&repo, 1).verdict(), Some(Verdict::Refuted));
    assert_eq!(member(&repo, 2).decision(), Some(Decision::Retained));
    assert_eq!(
        member(&repo, 2).implementation(),
        Some(Implementation::InProgress)
    );
    assert_eq!(member(&repo, 4).kind(), Kind::Category);
    assert_eq!(member(&repo, 5).kind(), Kind::PersonalNote);
    // La source reste telle qu'elle était, et ordinaire.
    assert_eq!(repo.branch_head(x), Ok(commit_id(15)));
    assert_eq!(repo.is_abandoned(x), Ok(false));
    assert_eq!(repo.branch_count(), 2);
    // Un seul « annuler » défait la fusion ; « rétablir » la refait.
    let merged = repo.snapshot().clone();
    run(&mut repo, 20, Command::Undo);
    assert_eq!(
        (repo.head(), repo.snapshot()),
        (commit_id(18), &target_state)
    );
    run(&mut repo, 21, Command::Redo);
    assert_eq!((repo.head(), repo.snapshot()), (commit_id(19), &merged));
    // Fusion répétée : la source est déjà réunie, rien n'est publié ni journalisé (INV-MOD-04).
    let plan = repo.merge_plan(x, &[]).unwrap();
    assert!(plan.up_to_date);
    assert_eq!(
        (plan.conflicts.len(), plan.from_source, plan.effects),
        (0, 0, Effects::default())
    );
    let after = repo.clone();
    let again = reasoned(&repo, 22, command);
    let outcome = repo.execute(again).unwrap();
    assert!(!outcome.published);
    assert_eq!(repo, after);
    // Après une première fusion, la base suivante est la tête de la source déjà réunie.
    checkout(&mut repo, 23, x);
    run(&mut repo, 24, create(DEFAULT_SCOPE, 6, Kind::Category));
    checkout(&mut repo, 25, MAIN);
    let plan = repo.merge_plan(x, &[]).unwrap();
    assert_eq!(plan.base, commit_id(15));
    assert_eq!((plan.conflicts.len(), plan.from_source), (0, 2));
}

// FUSION.md, section 8 : fusionner une branche dans son origine restée inchangée redonne
// exactement l'état de la branche, avec un commit à deux parents.
#[test]
fn merging_into_an_unchanged_origin_gives_the_branch_state() {
    let mut repo = forked();
    run(&mut repo, 14, create(DEFAULT_SCOPE, 4, Kind::Category));
    let source_state = repo.snapshot().clone();
    checkout(&mut repo, 15, MAIN);
    let plan = repo.merge_plan(branch_id(13), &[]).unwrap();
    assert_eq!(
        (plan.base, plan.target_head),
        (commit_id(12), commit_id(12))
    );
    run(&mut repo, 16, merge(13, 14, vec![]));
    assert_eq!(repo.snapshot(), &source_state);
    assert_eq!(
        repo.history()
            .commit(commit_id(16))
            .unwrap()
            .unwrap()
            .parents,
        vec![commit_id(12), commit_id(14)]
    );
}

/// Deux branches en conflit sur la note 1 (verdict) et la note 2 (texte) : la source `x`
/// réfute la note 1 et révise la note 2 ; main vérifie la note 1 et révise autrement la note 2.
fn conflicting() -> Repository {
    let mut repo = forked();
    run(&mut repo, 14, revise(2, "côté source"));
    checkout(&mut repo, 15, MAIN);
    run(
        &mut repo,
        16,
        Command::Change(verdict_change(1, Verdict::Verified)),
    );
    run(&mut repo, 17, revise(2, "côté cible"));
    repo
}

// INV-MERGE-02 : aucun conflit n'est écrasé silencieusement ; chaque conflit est repéré par sa
// clé et se résout en choisissant la valeur d'un côté ; la fusion est refusée tant qu'il en reste.
#[test]
fn conflicts_are_reported_and_resolved_by_side() {
    let mut repo = conflicting();
    let x = branch_id(13);
    let verdict = Conflict::Field {
        scope: DEFAULT_SCOPE,
        node: node_id(1),
        field: MemberField::Verdict,
    };
    let text = Conflict::Text { node: node_id(2) };
    let plan = repo.merge_plan(x, &[]).unwrap();
    assert_eq!(plan.conflicts, vec![text, verdict]);
    assert_eq!(plan.from_source, 0);
    let control = repo.clone();
    let refused = reasoned(&repo, 18, merge(13, 14, vec![]));
    assert_eq!(repo.execute(refused), Err(Error::UnresolvedConflicts(2)));
    assert_eq!(repo, control);
    let partial = vec![Resolution::Choose(text, Side::Target)];
    assert_eq!(
        repo.merge_plan(x, &partial).unwrap().conflicts,
        vec![verdict]
    );
    let refused = reasoned(&repo, 18, merge(13, 14, partial));
    assert_eq!(repo.execute(refused), Err(Error::UnresolvedConflicts(1)));
    assert_eq!(repo, control);
    let resolutions = vec![
        Resolution::Choose(verdict, Side::Source),
        Resolution::Choose(text, Side::Target),
    ];
    let plan = repo.merge_plan(x, &resolutions).unwrap();
    assert_eq!((plan.conflicts.len(), plan.structural), (0, None));
    let outcome = run(&mut repo, 18, merge(13, 14, resolutions));
    assert!(outcome.published && !outcome.forked);
    assert_eq!(outcome.effects, plan.effects);
    assert_eq!(member(&repo, 1).verdict(), Some(Verdict::Refuted));
    assert_eq!(member(&repo, 2).text(), "côté cible");
    // L'autre choix donne l'autre état.
    let mut other = control.clone();
    let resolutions = vec![
        Resolution::Choose(verdict, Side::Target),
        Resolution::Choose(text, Side::Source),
    ];
    run(&mut other, 18, merge(13, 14, resolutions));
    assert_eq!(member(&other, 1).verdict(), Some(Verdict::Verified));
    assert_eq!(member(&other, 2).text(), "côté source");
}

// FUSION.md, section 9.5 : une résolution qui ne correspond à aucun conflit, ou qui répète une
// clé, est refusée ; de même une désactivation hors conflit structurel. Le refus nomme le rang de
// la résolution (à partir de 1) et ne modifie rien.
#[test]
fn malformed_resolutions_are_refused() {
    let mut repo = conflicting();
    let x = branch_id(13);
    let text = Conflict::Text { node: node_id(2) };
    let other_field = Conflict::Field {
        scope: DEFAULT_SCOPE,
        node: node_id(2),
        field: MemberField::Decision,
    };
    let control = repo.clone();
    for (resolutions, refusal) in [
        (
            vec![
                Resolution::Choose(text, Side::Target),
                Resolution::Choose(text, Side::Source),
            ],
            Error::DuplicateResolution(2),
        ),
        (
            vec![
                Resolution::Choose(text, Side::Target),
                Resolution::Choose(other_field, Side::Source),
            ],
            Error::UnmatchedResolution(2),
        ),
        (
            vec![Resolution::Deactivate {
                scope: DEFAULT_SCOPE,
                edge: edge_id(3),
            }],
            Error::UnmatchedResolution(1),
        ),
    ] {
        assert_eq!(
            repo.merge_plan(x, &resolutions).map(|_| ()),
            Err(refusal.clone())
        );
        let refused = reasoned(&repo, 18, merge(13, 14, resolutions));
        assert_eq!(repo.execute(refused), Err(refusal));
        assert_eq!(repo, control);
    }
}

// Acquittement (FUSION.md, section 9.3) : deux branches acquittent la même cause à des états
// différents de cette cause. Base : main après une première fusion qui y amène la cause ; cible
// et source sont deux branches créées par ces acquittements.
#[test]
fn acknowledgement_conflicts_are_resolved_by_side() {
    let mut repo = forked();
    checkout(&mut repo, 14, MAIN);
    run(&mut repo, 15, merge(13, 13, vec![]));
    // Lever la cause est critique : chaque acquittement crée sa branche depuis main.
    let first = run(&mut repo, 16, acknowledge(2, 1));
    assert_eq!(first.branch, branch_id(16));
    checkout(&mut repo, 17, MAIN);
    run(&mut repo, 18, set_decision(1, Decision::Retained));
    assert_eq!(repo.active_branch(), MAIN);
    let second = run(&mut repo, 19, acknowledge(2, 1));
    assert_eq!(second.branch, branch_id(19));
    checkout(&mut repo, 20, branch_id(16));
    let ack = Conflict::Ack {
        scope: DEFAULT_SCOPE,
        dependent: node_id(2),
        cause: node_id(1),
    };
    let plan = repo.merge_plan(branch_id(19), &[]).unwrap();
    assert_eq!(plan.base, commit_id(15));
    assert_eq!(plan.conflicts, vec![ack]);
    // Décision de la note 1, prise à la source.
    assert_eq!(plan.from_source, 1);
    // L'acquittement de la cible porte sur l'ancienne décision : il ne vaut plus, la cause
    // revient. Celui de la source porte sur l'état réuni : rien ne change.
    let keep_target = [Resolution::Choose(ack, Side::Target)];
    assert_eq!(
        repo.merge_plan(branch_id(19), &keep_target)
            .unwrap()
            .effects
            .review
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![(DEFAULT_SCOPE, node_id(2))]
    );
    let take_source = vec![Resolution::Choose(ack, Side::Source)];
    let plan = repo.merge_plan(branch_id(19), &take_source).unwrap();
    assert_eq!(plan.effects, Effects::default());
    run(&mut repo, 21, merge(19, 19, take_source));
    let graph = repo.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert!(graph.review_causes().is_empty());
    assert_eq!(member(&repo, 1).decision(), Some(Decision::Retained));
}

/// Deux catégories ; main classe la première dans la seconde, la branche `branch_id(15)` la
/// seconde dans la première : chaque côté est valide, leur réunion forme un cycle.
fn crossing_categories() -> Repository {
    let mut repo = Repository::new();
    run(&mut repo, 10, create(DEFAULT_SCOPE, 5, Kind::Category));
    run(&mut repo, 11, create(DEFAULT_SCOPE, 6, Kind::Category));
    run(&mut repo, 12, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    run(
        &mut repo,
        13,
        create(DEFAULT_SCOPE, 2, Kind::Implementation),
    );
    run(&mut repo, 14, link(3, 2, 1, Relation::DependsOn));
    run(
        &mut repo,
        15,
        Command::Change(verdict_change(1, Verdict::Refuted)),
    );
    assert_eq!(repo.active_branch(), branch_id(15));
    run(&mut repo, 16, link(7, 6, 5, Relation::ClassifiedIn));
    checkout(&mut repo, 17, MAIN);
    run(&mut repo, 18, link(8, 5, 6, Relation::ClassifiedIn));
    repo
}

// INV-MERGE-03 : l'état réuni est revalidé ; un cycle né de la seule réunion est un conflit
// structurel, avec pour candidates les relations actives créées ou modifiées depuis la base ; il
// se résout en désactivant l'une d'elles, et la fusion n'est appliquée que sur un état valide.
#[test]
fn structural_conflicts_are_resolved_by_deactivation() {
    let mut repo = crossing_categories();
    let x = branch_id(15);
    let plan = repo.merge_plan(x, &[]).unwrap();
    assert_eq!(plan.conflicts, vec![]);
    assert_eq!(
        plan.structural,
        Some(Structural {
            reason: Error::CyclicRelation,
            candidates: vec![(DEFAULT_SCOPE, edge_id(7)), (DEFAULT_SCOPE, edge_id(8))],
        })
    );
    let control = repo.clone();
    let refused = reasoned(&repo, 19, merge(15, 16, vec![]));
    assert_eq!(
        repo.execute(refused),
        Err(Error::StructuralConflict(Box::new(Error::CyclicRelation)))
    );
    assert_eq!(repo, control);
    // La relation 3 existait déjà dans la base : elle n'est pas candidate.
    let not_candidate = vec![Resolution::Deactivate {
        scope: DEFAULT_SCOPE,
        edge: edge_id(3),
    }];
    assert_eq!(
        repo.merge_plan(x, &not_candidate).map(|_| ()),
        Err(Error::UnmatchedResolution(1))
    );
    let deactivate = vec![Resolution::Deactivate {
        scope: DEFAULT_SCOPE,
        edge: edge_id(7),
    }];
    let plan = repo.merge_plan(x, &deactivate).unwrap();
    assert_eq!(plan.structural, None);
    run(&mut repo, 19, merge(15, 16, deactivate));
    let graph = repo.snapshot().graph(DEFAULT_SCOPE).unwrap();
    assert!(!graph.edge(edge_id(7)).unwrap().active());
    assert!(graph.edge(edge_id(8)).unwrap().active());
    assert_eq!(repo.snapshot().validate(), Ok(()));
    // La source garde sa relation active.
    let source = repo.snapshot_at(commit_id(16)).unwrap();
    let source_graph = source.graph(DEFAULT_SCOPE).unwrap();
    assert!(source_graph.edge(edge_id(7)).unwrap().active());
}

// FUSION.md, section 9.2 (D23) : après des fusions croisées dans les deux sens, deux bases
// communes sont aussi proches l'une que l'autre ; la fusion est refusée plutôt que d'en choisir
// une.
#[test]
fn several_merge_bases_are_refused() {
    let mut repo = forked();
    let x = branch_id(13);
    checkout(&mut repo, 14, MAIN);
    run(&mut repo, 15, create(DEFAULT_SCOPE, 4, Kind::Category));
    run(&mut repo, 16, merge(13, 13, vec![]));
    run(&mut repo, 17, Command::Undo);
    checkout(&mut repo, 18, x);
    let main_into_x = Command::Merge {
        source: MAIN,
        source_head: commit_id(15),
        resolutions: vec![],
    };
    run(&mut repo, 19, main_into_x);
    checkout(&mut repo, 20, MAIN);
    run(&mut repo, 21, Command::Redo);
    // main : fusion 16 (parents 15 et 13) ; x : fusion 19 (parents 13 et 15). Les commits 15 et
    // 13 sont deux ancêtres communs dont aucun n'est l'ancêtre de l'autre.
    assert_eq!(repo.head(), commit_id(16));
    let control = repo.clone();
    assert_eq!(
        repo.merge_plan(x, &[]).map(|_| ()),
        Err(Error::MultipleMergeBases)
    );
    let refused = reasoned(&repo, 22, merge(13, 19, vec![]));
    assert_eq!(repo.execute(refused), Err(Error::MultipleMergeBases));
    assert_eq!(repo, control);
}

// FUSION.md, sections 9.1 et 9.7 : une branche ne se fusionne pas dans elle-même ; la source
// existe et n'est pas abandonnée (INV-BR-05) ; si la source a bougé depuis le plan, la fusion est
// refusée comme périmée, comme si la cible a bougé (état attendu de l'enveloppe).
#[test]
fn merges_are_refused_on_a_wrong_or_stale_source() {
    let mut repo = forked();
    let x = branch_id(13);
    let control = repo.clone();
    assert_eq!(
        repo.merge_plan(x, &[]).map(|_| ()),
        Err(Error::MergeIntoItself)
    );
    let refused = reasoned(&repo, 14, merge(13, 13, vec![]));
    assert_eq!(repo.execute(refused), Err(Error::MergeIntoItself));
    assert_eq!(repo, control);
    checkout(&mut repo, 14, MAIN);
    let ghost = branch_id(99);
    assert_eq!(
        repo.merge_plan(ghost, &[]).map(|_| ()),
        Err(Error::MissingBranch(ghost))
    );
    // Plan calculé sur la tête 13 ; la source avance ensuite.
    let stale = merge(13, 13, vec![]);
    checkout(&mut repo, 15, x);
    run(&mut repo, 16, create(DEFAULT_SCOPE, 4, Kind::Category));
    checkout(&mut repo, 17, MAIN);
    let control = repo.clone();
    let refused = reasoned(&repo, 18, stale);
    assert_eq!(
        repo.execute(refused),
        Err(Error::StaleSource {
            expected: commit_id(13),
            actual: commit_id(16),
        })
    );
    assert_eq!(repo, control);
    // Une source abandonnée ne se fusionne pas tant qu'elle n'est pas réactivée.
    run(&mut repo, 18, abandon(13));
    let control = repo.clone();
    assert_eq!(
        repo.merge_plan(x, &[]).map(|_| ()),
        Err(Error::BranchAbandoned(x))
    );
    let refused = reasoned(&repo, 19, merge(13, 16, vec![]));
    assert_eq!(repo.execute(refused), Err(Error::BranchAbandoned(x)));
    assert_eq!(repo, control);
    run(&mut repo, 19, reactivate(13));
    assert!(run(&mut repo, 20, merge(13, 16, vec![])).published);
    // Messages.
    assert_eq!(
        Error::MergeIntoItself.to_string(),
        "une branche ne se fusionne pas dans elle-même"
    );
    assert_eq!(
        Error::MultipleMergeBases.to_string(),
        "plusieurs bases communes : fusion refusée"
    );
    assert_eq!(
        Error::StaleSource {
            expected: commit_id(13),
            actual: commit_id(16),
        }
        .to_string(),
        format!(
            "plan de fusion périmé : source attendue sur {}, tête actuelle {}",
            commit_id(13),
            commit_id(16)
        )
    );
    assert_eq!(
        Error::UnresolvedConflicts(2).to_string(),
        "2 conflit(s) non résolu(s)"
    );
    assert_eq!(
        Error::UnmatchedResolution(3).to_string(),
        "la résolution 3 ne correspond à aucun conflit"
    );
    assert_eq!(
        Error::DuplicateResolution(2).to_string(),
        "la résolution 2 répète une clé déjà résolue"
    );
    assert_eq!(
        Error::StructuralConflict(Box::new(Error::CyclicRelation)).to_string(),
        "conflit structurel : cycle de remplacement ou de catégories"
    );
}

/// Bloc `operation` attendu pour une fusion résolue de `conflicting()` (VERSION2.md, section 6 ;
/// FUSION.md, section 9.6), écrit d'après la spécification.
fn expected_merge_block() -> String {
    let scope = MAIN.to_string();
    format!(
        "operation {op} attendu {scope} {head} motif \"motif\"\n\
         \x20 commande fusionner source {x} tete {xhead} resolutions 2\n\
         \x20   resolution champ {scope} {n1} verdict source\n\
         \x20   resolution texte {n2} cible\n\
         \x20 resultat branche {scope} tete {op} publie oui bifurque non\n\
         \x20 effet {scope} {n2} avant 0 apres 1 {n1}\n",
        op = raw_text(18),
        head = raw_text(17),
        x = raw_text(13),
        xhead = raw_text(14),
        n1 = raw_text(1),
        n2 = raw_text(2),
    )
}

// VERSION2.md, section 6 : bloc `fusionner` et ses résolutions, commit à deux parents ; tout se
// relit à l'identique, dépôt complet compris, et se vérifie en entier.
#[test]
fn merge_blocks_are_written_as_specified_and_round_trip() {
    let mut repo = conflicting();
    let resolutions = vec![
        Resolution::Choose(
            Conflict::Field {
                scope: DEFAULT_SCOPE,
                node: node_id(1),
                field: MemberField::Verdict,
            },
            Side::Source,
        ),
        Resolution::Choose(Conflict::Text { node: node_id(2) }, Side::Target),
    ];
    let next = reasoned(&repo, 18, merge(13, 14, resolutions));
    let outcome = repo.execute(next.clone()).unwrap();
    let block = encode_operation_v2(&next, &outcome);
    assert_eq!(block, expected_merge_block());
    assert_eq!(decode_operation_v2(&block), Ok((next, outcome)));
    let text = encode_repository_v2(&repo);
    assert!(
        text.contains(&format!(
            "commit {} parents 2 {} {} motif ",
            raw_text(18),
            raw_text(17),
            raw_text(14)
        )),
        "{text}"
    );
    assert_eq!(decode_repository_v2(&text), Ok(repo.clone()));
    assert_eq!(
        verify_repository_v2(&text).map(|v| v.commits),
        Ok(repo.commit_count())
    );
    // Les autres formes de résolution s'écrivent et se relisent aussi.
    let others = vec![
        Resolution::Choose(
            Conflict::Ack {
                scope: DEFAULT_SCOPE,
                dependent: node_id(2),
                cause: node_id(1),
            },
            Side::Target,
        ),
        Resolution::Deactivate {
            scope: DEFAULT_SCOPE,
            edge: edge_id(3),
        },
    ];
    for field in [
        MemberField::Decision,
        MemberField::Implementation,
        MemberField::Reserve,
        MemberField::Visibility,
    ] {
        let mut resolutions = others.clone();
        resolutions.push(Resolution::Choose(
            Conflict::Field {
                scope: DEFAULT_SCOPE,
                node: node_id(1),
                field,
            },
            Side::Source,
        ));
        let envelope = reasoned(&repo, 30, merge(13, 14, resolutions));
        let block = encode_operation_v2(&envelope, &outcome_placeholder(&repo));
        assert_eq!(
            decode_operation_v2(&block),
            Ok((envelope, outcome_placeholder(&repo)))
        );
    }
}

/// Résultat quelconque, pour relire des blocs dont seule la commande importe.
fn outcome_placeholder(repo: &Repository) -> Outcome {
    Outcome {
        op: OpId::from_bytes(raw(30)),
        branch: MAIN,
        head: repo.head(),
        published: false,
        forked: false,
        effects: Effects::default(),
    }
}

// Lecture : un commit dont un parent est inconnu, des lignes de résolution en nombre différent
// de celui annoncé ou mal formées, une fusion dans un lot sont refusés et situés.
#[test]
fn malformed_merge_blocks_are_refused() {
    let block = expected_merge_block();
    let lines: Vec<&str> = block.lines().collect();
    let with = |number: usize, line: &str| {
        let mut changed = lines.clone();
        changed[number - 1] = line;
        changed.join("\n") + "\n"
    };
    let scope = MAIN.to_string();
    let (n1, n2) = (raw_text(1), raw_text(2));
    let cases: Vec<(String, usize, DecodeReason)> = vec![
        (
            with(2, &lines[1].replace("resolutions 2", "resolutions 3")),
            5,
            DecodeReason::Field("resolution"),
        ),
        (
            with(3, &format!("    resolution autre {scope} {n1}")),
            3,
            DecodeReason::Value("resolution"),
        ),
        (
            with(
                3,
                &format!("    resolution champ {scope} {n1} couleur source"),
            ),
            3,
            DecodeReason::Value("champ"),
        ),
        (
            with(4, &format!("    resolution texte {n2} gauche")),
            4,
            DecodeReason::Value("côté"),
        ),
        (
            with(4, &format!("    resolution texte {n2} cible en-trop")),
            4,
            DecodeReason::Field("fin de ligne"),
        ),
        (
            with(
                2,
                &format!(
                    "  commande lot 1\n    element fusionner source {} tete {} resolutions 0",
                    raw_text(13),
                    raw_text(14)
                ),
            ),
            3,
            DecodeReason::Value("element"),
        ),
    ];
    for (text, line, reason) in cases {
        assert_eq!(
            decode_operation_v2(&text),
            Err(DecodeError { line, reason }),
            "{text}"
        );
    }
    // Commit de fusion dont le second parent est inconnu.
    let mut repo = conflicting();
    let resolutions = vec![
        Resolution::Choose(
            Conflict::Field {
                scope: DEFAULT_SCOPE,
                node: node_id(1),
                field: MemberField::Verdict,
            },
            Side::Source,
        ),
        Resolution::Choose(Conflict::Text { node: node_id(2) }, Side::Target),
    ];
    run(&mut repo, 18, merge(13, 14, resolutions));
    let text = encode_repository_v2(&repo);
    let ghost = "a5000000-0000-0000-0000-0000000000ff";
    let orphan = text.replacen(
        &format!("parents 2 {} {}", raw_text(17), raw_text(14)),
        &format!("parents 2 {} {ghost}", raw_text(17)),
        1,
    );
    assert_ne!(orphan, text);
    assert_eq!(
        decode_repository_v2(&orphan).map(|_| ()),
        Err(DecodeError {
            line: 0,
            reason: DecodeReason::Inconsistent("parent de commit"),
        })
    );
}

// INV-UNDO-02 : une fusion est une action nouvelle ; comme toute publication, elle efface la pile
// de rétablissement de la cible (les commits annulés restent conservés).
#[test]
fn a_merge_clears_the_redo_stack_of_the_target() {
    let mut repo = forked();
    checkout(&mut repo, 14, MAIN);
    run(&mut repo, 15, create(DEFAULT_SCOPE, 4, Kind::Category));
    run(&mut repo, 16, Command::Undo);
    assert_eq!(
        repo.history().branches().unwrap()[&MAIN].redo,
        vec![Redo::Commit(commit_id(15))]
    );
    run(&mut repo, 17, merge(13, 13, vec![]));
    assert!(repo.history().branches().unwrap()[&MAIN].redo.is_empty());
    let refused = envelope(&repo, 18, Command::Redo);
    assert_eq!(repo.execute(refused), Err(Error::NothingToRedo));
    assert!(repo.snapshot_at(commit_id(15)).is_ok());
}

// ---------------------------------------------------------------------------
// Incrément 5.5a — contrat ajouté : échanges dans les dépendances, citation d'une note,
// orphelines (CITATIONS.md ; D18, D24 ; INV-CIT-01, INV-CIT-02, INV-ORPH-01, INV-REL-04).
// ---------------------------------------------------------------------------

// D18, points 1, 2 et 4 : une affirmation peut dépendre d'un échange entier ou s'en être
// développée ; un échange peut s'être développé d'un autre. Le reste est toujours refusé. Un
// échange ne porte pas de verdict ; retiré des ressources actuelles, il met ses dépendants à
// réexaminer.
#[test]
fn exchanges_take_part_in_dependencies() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let first = r
        .create_exchange(S, "première conversation", Some("q1"), "a1")
        .unwrap();
    let second = r
        .create_exchange(S, "conversation née de la première", Some("q2"), "a2")
        .unwrap();
    let claim = note(&mut r, Kind::Reasoning, "appui sur un échange entier");
    let derived = note(&mut r, Kind::Implementation, "développée d'un échange");
    let category = note(&mut r, Kind::Category, "catégorie");
    r.link(S, claim, first, Relation::DependsOn).unwrap();
    r.link(S, derived, first, Relation::DevelopedFrom).unwrap();
    r.link(S, second, first, Relation::DevelopedFrom).unwrap();
    let control = r.clone();
    for (from, to, relation) in [
        (second, first, Relation::DependsOn),
        (first, claim, Relation::DevelopedFrom),
        (first, claim, Relation::DependsOn),
        (category, first, Relation::DependsOn),
        (category, first, Relation::DevelopedFrom),
        (first, category, Relation::DevelopedFrom),
    ] {
        assert_eq!(
            r.link(S, from, to, relation),
            Err(Error::InvalidRelation),
            "{from} {relation:?} {to}"
        );
        assert_eq!(r, control);
    }
    assert_eq!(
        r.contradict(S, claim, first, "un échange ne se contredit pas"),
        Err(Error::InvalidRelation)
    );
    assert_eq!(
        r.set_verdict(S, first, Verdict::Refuted),
        Err(Error::CategoryHasNoVerdict)
    );
    r.archive(S, first).unwrap();
    let causes = r.snapshot().graph(S).unwrap().review_causes();
    assert_eq!(
        causes,
        BTreeMap::from([
            (claim, BTreeSet::from([first])),
            (derived, BTreeSet::from([first])),
            (second, BTreeSet::from([first])),
        ])
    );
}

// INV-CIT-01 : une citation de note désigne exactement le texte d'une version de la note ; une
// révision ultérieure ne la déplace pas et la fait signaler « source révisée ». Sources citables :
// raisonnement, implémentation, note personnelle (D24).
#[test]
fn a_note_is_cited_at_a_version() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "invariant de priorité");
    let c = r.cite_note(S, n, 0, 0, 5).unwrap();
    assert_eq!(
        r.snapshot().citation(c),
        Ok(CitationView {
            source: CitedSource::Note {
                note: n,
                version: 0
            },
            start: 0,
            end: 5,
            source_revised: false,
        })
    );
    let cited = |r: &Harness, id| {
        let node = r.snapshot().graph(S).unwrap().node(id).unwrap().clone();
        (node.kind(), node.text().to_owned())
    };
    assert_eq!(cited(&r, c), (Kind::Passage, "texte".to_owned()));
    // Une révision ne déplace pas la citation ; elle est signalée.
    r.revise(n, "nouveau texte", RevisionKind::Minor).unwrap();
    let view = r.snapshot().citation(c).unwrap();
    assert!(view.source_revised);
    assert_eq!(
        view.source,
        CitedSource::Note {
            note: n,
            version: 0
        }
    );
    assert_eq!(cited(&r, c), (Kind::Passage, "texte".to_owned()));
    let current = r.cite_note(S, n, 1, 0, 7).unwrap();
    assert_eq!(cited(&r, current), (Kind::Passage, "nouveau".to_owned()));
    assert!(!r.snapshot().citation(current).unwrap().source_revised);
    // Refus, sans aucun effet.
    let category = note(&mut r, Kind::Category, "catégorie");
    let exchange = r
        .create_exchange(S, "échange", Some("q"), "réponse")
        .unwrap();
    let accented = r
        .create_node(S, Kind::PersonalNote, "accents", "été")
        .unwrap();
    let ghost = NodeId::from_bytes(raw(999));
    let control = r.clone();
    for (source, version, start, end, refusal) in [
        (n, 2, 0, 1, Error::InvalidExcerpt),
        (n, 0, 3, 3, Error::InvalidExcerpt),
        (n, 0, 0, 99, Error::InvalidExcerpt),
        (accented, 0, 0, 1, Error::InvalidExcerpt),
        (category, 0, 0, 1, Error::WrongKind(category)),
        (exchange, 0, 0, 1, Error::WrongKind(exchange)),
        (c, 0, 0, 1, Error::WrongKind(c)),
        (ghost, 0, 0, 1, Error::MissingNode(ghost)),
    ] {
        assert_eq!(r.cite_note(S, source, version, start, end), Err(refusal));
        assert_eq!(r, control);
    }
    assert!(r.cite_note(S, accented, 0, 0, 2).is_ok());
    // Une citation de note n'est pas un passage d'échange : aucune réserve n'en naît (D24).
    assert_eq!(r.snapshot().passage(c), Err(Error::WrongKind(c)));
    assert_eq!(
        r.create_reserve(S, c, "réserve", None),
        Err(Error::WrongKind(c))
    );
    // La citation d'un échange se lit de la même façon ; une note ordinaire n'est pas une citation.
    let passage = r.create_passage(S, exchange, Part::Answer, 0, 3).unwrap();
    assert_eq!(
        r.snapshot().citation(passage),
        Ok(CitationView {
            source: CitedSource::Exchange {
                exchange,
                part: Part::Answer
            },
            start: 0,
            end: 3,
            source_revised: false,
        })
    );
    assert_eq!(r.snapshot().citation(n), Err(Error::WrongKind(n)));
}

// INV-CIT-02, D22 : la citation et son lien « développé à partir de » se créent en un seul lot ;
// une révision substantielle de la source met la citation à réexaminer par ce lien, une révision
// mineure seulement la signale ; le lien se retire comme toute relation.
#[test]
fn a_citation_and_its_link_follow_their_source() {
    let mut repo = Repository::new();
    let first = reasoned(&repo, 10, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    repo.execute(first).unwrap();
    let commits = repo.commit_count();
    let lot = Command::Lot(vec![
        Change::CiteNote {
            scope: DEFAULT_SCOPE,
            id: node_id(2),
            note: node_id(1),
            version: 0,
            start: 0,
            end: 5,
        },
        Change::Link {
            scope: DEFAULT_SCOPE,
            id: edge_id(3),
            from: node_id(2),
            to: node_id(1),
            relation: Relation::DevelopedFrom,
        },
    ]);
    let outcome = repo.execute(reasoned(&repo, 11, lot)).unwrap();
    assert!(outcome.published && !outcome.forked);
    assert_eq!(repo.commit_count(), commits + 1);
    let causes = |repo: &Repository| {
        repo.snapshot()
            .graph(DEFAULT_SCOPE)
            .unwrap()
            .review_causes()
    };
    let revise = |kind| {
        Command::Change(Change::Revise {
            node: node_id(1),
            text: "texte révisé".into(),
            kind,
        })
    };
    repo.execute(reasoned(&repo, 12, revise(RevisionKind::Minor)))
        .unwrap();
    assert!(causes(&repo).is_empty());
    assert!(repo.snapshot().citation(node_id(2)).unwrap().source_revised);
    let next = Command::Change(Change::Revise {
        node: node_id(1),
        text: "sens changé".into(),
        kind: RevisionKind::Substantive,
    });
    repo.execute(reasoned(&repo, 13, next)).unwrap();
    assert_eq!(
        causes(&repo),
        BTreeMap::from([(node_id(2), BTreeSet::from([node_id(1)]))])
    );
    let detach = Command::Change(Change::Detach {
        scope: DEFAULT_SCOPE,
        edge: edge_id(3),
    });
    repo.execute(reasoned(&repo, 14, detach)).unwrap();
    assert!(causes(&repo).is_empty());
    assert!(repo.snapshot().citation(node_id(2)).unwrap().source_revised);
}

// INV-ORPH-01 (R6, D24) : une note est orpheline si et seulement si elle a au moins un appui
// déclaré, que tous sont écartés, qu'aucune cause de réexamen n'est en cours et qu'elle n'est pas
// écartée ; une orpheline n'est pas une ressource actuelle.
#[test]
fn orphans_are_notes_whose_supports_all_fell() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Reasoning, "appui a");
    let b = note(&mut r, Kind::Reasoning, "appui b");
    let n = note(&mut r, Kind::Implementation, "s'appuie sur a et b");
    let alone = note(&mut r, Kind::PersonalNote, "sans appui");
    r.link(S, n, a, Relation::DependsOn).unwrap();
    r.link(S, n, b, Relation::DevelopedFrom).unwrap();
    let orphans = |r: &Harness| r.snapshot().graph(S).unwrap().orphans();
    let current = |r: &Harness| r.snapshot().graph(S).unwrap().current_resources();
    assert!(orphans(&r).is_empty());
    r.invalidate(S, a).unwrap();
    // À réexaminer, donc pas orpheline ; puis un appui tient encore.
    assert!(orphans(&r).is_empty());
    r.acknowledge(S, n, a).unwrap();
    assert!(orphans(&r).is_empty());
    assert!(current(&r).contains(&n));
    r.archive(S, b).unwrap();
    assert!(orphans(&r).is_empty());
    r.acknowledge(S, n, b).unwrap();
    assert_eq!(orphans(&r), vec![n]);
    assert!(!current(&r).contains(&n));
    // Une note sans appui n'est jamais orpheline.
    assert!(current(&r).contains(&alone));
    // Écartée elle-même, elle n'est pas orpheline.
    r.archive(S, n).unwrap();
    assert!(orphans(&r).is_empty());
    r.undo().unwrap();
    assert_eq!(orphans(&r), vec![n]);
    // Un appui qui redevient valide la rend ressource actuelle.
    r.set_verdict(S, a, Verdict::Verified).unwrap();
    assert!(orphans(&r).is_empty());
    assert!(current(&r).contains(&n));
}

// VERSION2.md, section 5 : la citation d'une note s'écrit `partie note version <rang>`, avec
// l'empreinte du texte cité ; instantané, bloc d'opération et dépôt se relisent à l'identique.
#[test]
fn note_citation_blocks_round_trip() {
    let mut repo = Repository::new();
    repo.execute(reasoned(
        &repo,
        10,
        create(DEFAULT_SCOPE, 1, Kind::Reasoning),
    ))
    .unwrap();
    let revise = Command::Change(Change::Revise {
        node: node_id(1),
        text: "autre texte".into(),
        kind: RevisionKind::Minor,
    });
    repo.execute(reasoned(&repo, 11, revise)).unwrap();
    let cite = reasoned(
        &repo,
        12,
        Command::Change(Change::CiteNote {
            scope: DEFAULT_SCOPE,
            id: node_id(2),
            note: node_id(1),
            version: 1,
            start: 0,
            end: 5,
        }),
    );
    let outcome = repo.execute(cite.clone()).unwrap();
    let line = format!(
        "citation {} source {} partie note version 1 debut 0 fin 5 empreinte {:016x}",
        raw_text(2),
        raw_text(1),
        text_digest("autre")
    );
    let snapshot = encode_snapshot_v2(repo.snapshot());
    assert!(snapshot.contains(&format!("\n{line}\n")), "{snapshot}");
    assert_eq!(decode_snapshot_v2(&snapshot).as_ref(), Ok(repo.snapshot()));
    let block = encode_operation_v2(&cite, &outcome);
    assert!(
        block.contains(&format!(
            "  commande citer portee {} note {} source {} partie note version 1 debut 0 fin 5\n",
            MAIN,
            raw_text(2),
            raw_text(1)
        )),
        "{block}"
    );
    assert_eq!(decode_operation_v2(&block), Ok((cite, outcome)));
    let text = encode_repository_v2(&repo);
    assert_eq!(decode_repository_v2(&text), Ok(repo.clone()));
    assert_eq!(
        verify_repository_v2(&text).map(|v| v.commits),
        Ok(repo.commit_count())
    );
    // Lecture refusée : empreinte fausse, version sans ce texte.
    let wrong_digest = snapshot.replace(
        &format!("empreinte {:016x}", text_digest("autre")),
        "empreinte 0000000000000000",
    );
    let at = line_starting(&snapshot, "citation ");
    assert_eq!(
        decode_snapshot_v2(&wrong_digest),
        Err(DecodeError {
            line: at,
            reason: DecodeReason::Value("empreinte"),
        })
    );
    let wrong_version = snapshot.replace(" partie note version 1 ", " partie note version 0 ");
    assert_eq!(
        decode_snapshot_v2(&wrong_version).map(|_| ()),
        Err(DecodeError {
            line: 0,
            reason: DecodeReason::InvalidState(Error::InvalidExcerpt),
        })
    );
}

// ---------------------------------------------------------------------------
// Incrément 6.3a — contrat ajouté : indexeur de repères (INDEXEUR.md, sections 2 et 3 ;
// INV-CODE-01, INV-CODE-05, INV-CODE-06 ; CA-05, CA-30, CA-31).
// ---------------------------------------------------------------------------

// INDEXEUR.md, section 2 : un repère est « @design: » suivi d'un identifiant canonique, n'importe
// où dans une ligne, quelle que soit la syntaxe de commentaire ; plusieurs par ligne ; un
// identifiant mal formé est rendu sans note, pour être signalé.
#[test]
fn markers_are_found_whatever_the_comment_syntax() {
    let (a, b, c) = (node_id(1), node_id(2), node_id(3));
    let texte = format!(
        "// @design: {a}\n# @design: {b} et ; @design: {c}\n/* @design: {a} */\n\
         @design: pas-un-identifiant\nrien ici\n@design: {}\n@design: {a}0\n\tx = 1; // @design: {b}",
        a.to_string().to_uppercase()
    );
    let trouves: Vec<(usize, Option<NodeId>)> = find_markers(&texte)
        .into_iter()
        .map(|m| (m.line, m.node))
        .collect();
    assert_eq!(
        trouves,
        vec![
            (1, Some(a)),
            (2, Some(b)),
            (2, Some(c)),
            (3, Some(a)),
            (4, None),
            (6, None),
            (7, None),
            (8, Some(b)),
        ]
    );
    assert_eq!(find_markers(""), vec![]);
    assert_eq!(find_markers("@design:"), vec![]);
}

fn occurrence(path: &str, line: usize, node: NodeId) -> Occurrence {
    Occurrence {
        path: path.into(),
        line,
        node,
    }
}

// INDEXEUR.md, section 3 : chaque ancrage actif est classé d'après les occurrences du repère de
// sa note : ambigu (plusieurs, aucune choisie : INV-CODE-01), déplacé (une, dans un autre
// fichier : INV-CODE-05, S-CODE-02), retrouvé ou modifié (une, dans son fichier, extrait présent
// ou non), perdu (aucune) ; puis les repères inconnus et non ancrés. Chemins comparés
// relativement au dossier indexé.
#[test]
fn the_code_report_classifies_every_anchor_and_marker() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let n = |r: &mut Harness, titre| note(r, Kind::Implementation, titre);
    let (trouvee, modifiee, deplacee, ambigue, absolue, perdue, retiree, libre) = (
        n(&mut r, "retrouvée"),
        n(&mut r, "modifiée"),
        n(&mut r, "déplacée"),
        n(&mut r, "ambiguë"),
        n(&mut r, "chemin absolu"),
        n(&mut r, "perdue"),
        n(&mut r, "ancrage retiré"),
        n(&mut r, "sans ancrage"),
    );
    let a_trouvee = r
        .anchor(trouvee, "src/driver.c", "void a() {}", None)
        .unwrap();
    let a_modifiee = r
        .anchor(modifiee, "src/driver.c", "void b() {}", None)
        .unwrap();
    let a_deplacee = r
        .anchor(deplacee, "src/engine.c", "void c() {}", None)
        .unwrap();
    let a_ambigue = r.anchor(ambigue, "./src/d.c", "void d() {}", None).unwrap();
    let a_absolue = r
        .anchor(absolue, "/projet/src/e.c", "void e() {}", None)
        .unwrap();
    let a_perdue = r.anchor(perdue, "src/old.c", "void f() {}", None).unwrap();
    let a_retiree = r.anchor(retiree, "src/r.c", "void r() {}", None).unwrap();
    r.unanchor(a_retiree).unwrap();
    // Seconde ancre de la note retrouvée, dans un autre fichier : son repère est ailleurs.
    let a_seconde = r
        .anchor(trouvee, "src/other.c", "void a() {}", None)
        .unwrap();
    let fantome = NodeId::from_bytes(raw(999));
    let fichiers = BTreeMap::from([
        (
            "src/driver.c".to_owned(),
            "void a() {}\n// @design: x\nvoid b2() {}\n".to_owned(),
        ),
        ("src/kernel.c".to_owned(), "void c() {}\n".to_owned()),
        ("src/a.c".to_owned(), String::new()),
        ("src/b.c".to_owned(), String::new()),
        ("src/e.c".to_owned(), "void e() {}\n".to_owned()),
        ("src/g.c".to_owned(), String::new()),
    ]);
    let occurrences = vec![
        occurrence("src/driver.c", 2, trouvee),
        occurrence("src/driver.c", 5, modifiee),
        occurrence("src/kernel.c", 1, deplacee),
        occurrence("src/b.c", 7, ambigue),
        occurrence("src/a.c", 3, ambigue),
        occurrence("src/e.c", 1, absolue),
        occurrence("src/g.c", 4, libre),
        occurrence("src/g.c", 9, retiree),
        occurrence("src/x.c", 5, fantome),
    ];
    let rapport = r
        .snapshot()
        .code_report(S, "/projet", &occurrences, &fichiers)
        .unwrap();
    let attendu = |anchor, node, path: &str, state| AnchorReport {
        anchor,
        node,
        path: path.into(),
        state,
    };
    assert_eq!(
        rapport.anchors,
        vec![
            attendu(
                a_trouvee,
                trouvee,
                "src/driver.c",
                AnchorState::Found(occurrence("src/driver.c", 2, trouvee))
            ),
            attendu(
                a_modifiee,
                modifiee,
                "src/driver.c",
                AnchorState::Modified(occurrence("src/driver.c", 5, modifiee))
            ),
            attendu(
                a_deplacee,
                deplacee,
                "src/engine.c",
                AnchorState::Moved {
                    at: occurrence("src/kernel.c", 1, deplacee),
                    excerpt_found: true,
                }
            ),
            attendu(
                a_ambigue,
                ambigue,
                "src/d.c",
                AnchorState::Ambiguous(vec![
                    occurrence("src/a.c", 3, ambigue),
                    occurrence("src/b.c", 7, ambigue),
                ])
            ),
            attendu(
                a_absolue,
                absolue,
                "src/e.c",
                AnchorState::Found(occurrence("src/e.c", 1, absolue))
            ),
            attendu(a_perdue, perdue, "src/old.c", AnchorState::Lost),
            attendu(
                a_seconde,
                trouvee,
                "src/other.c",
                AnchorState::Moved {
                    at: occurrence("src/driver.c", 2, trouvee),
                    excerpt_found: true,
                }
            ),
        ]
    );
    // Un ancrage retiré ne compte plus : son repère est celui d'une note sans ancrage actif.
    assert_eq!(
        rapport.unanchored,
        vec![
            occurrence("src/g.c", 4, libre),
            occurrence("src/g.c", 9, retiree)
        ]
    );
    assert_eq!(rapport.unknown, vec![occurrence("src/x.c", 5, fantome)]);
    // INV-CODE-06 : même entrée, même rapport ; l'ordre des occurrences n'y change rien.
    let mut melangees = occurrences.clone();
    melangees.reverse();
    assert_eq!(
        r.snapshot()
            .code_report(S, "/projet", &melangees, &fichiers)
            .unwrap(),
        rapport
    );
    let ailleurs = ScopeId::from_bytes(raw(998));
    assert_eq!(
        r.snapshot()
            .code_report(ailleurs, "/projet", &occurrences, &fichiers),
        Err(Error::MissingScope(ailleurs))
    );
}

// Chemins (INDEXEUR.md, section 3) : relatifs au dossier indexé, sans « ./ » en tête, séparés par
// « / » ; un chemin absolu hors du dossier reste tel quel.
#[test]
fn anchor_paths_are_compared_relative_to_the_indexed_folder() {
    for (racine, chemin, attendu) in [
        ("/projet", "src/a.c", "src/a.c"),
        ("/projet", "./src/a.c", "src/a.c"),
        ("/projet", "/projet/src/a.c", "src/a.c"),
        ("/projet/", "/projet/src/a.c", "src/a.c"),
        ("/projet", "/autre/a.c", "/autre/a.c"),
        ("/projet", "/projetx/a.c", "/projetx/a.c"),
        ("/projet", "src\\a.c", "src/a.c"),
        // Sans dossier, un chemin absolu reste absolu (ajouté après la mutation).
        ("", "/a.c", "/a.c"),
        ("", "./a.c", "a.c"),
    ] {
        assert_eq!(relative_path(racine, chemin), attendu, "{racine} {chemin}");
    }
}

// ---------------------------------------------------------------------------
// Incrément 6.4a — contrat ajouté : restauration ciblée et historique (RESTAURATION.md ;
// INV-RESTORE-01, INV-RESTORE-02, INV-RET-01, INV-RET-03 ; CA-27, CA-33).
// ---------------------------------------------------------------------------

// INV-RESTORE-01, INV-RESTORE-02 (S-RET-02 ciblée) : une idée écartée est rétablie avec ses liens
// internes ; une note indépendante, modifiée depuis, n'est pas touchée ; une relation avec
// l'extérieur, créée depuis, est conservée et signalée dans le plan ; aperçu et plan donnent les
// mêmes effets ; un seul « annuler » défait la restauration (CA-33).
#[test]
fn a_targeted_restore_brings_back_an_idea_with_its_links_only() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let idee = note(&mut r, Kind::Reasoning, "idée");
    let appui = note(&mut r, Kind::Implementation, "appui de l'idée");
    let independante = note(&mut r, Kind::PersonalNote, "indépendante");
    let interne = r.link(S, appui, idee, Relation::DependsOn).unwrap();
    let cible = r.head();
    // Depuis : l'idée est écartée, son lien interne retiré ; travail indépendant ailleurs ; une
    // note nouvelle s'appuie sur l'idée.
    r.set_decision(S, idee, Decision::NoLongerRetained).unwrap();
    r.detach(S, interne).unwrap();
    r.revise(independante, "révisée depuis", RevisionKind::Minor)
        .unwrap();
    let nouvelle = note(&mut r, Kind::Implementation, "nouvelle");
    let externe = r.link(S, nouvelle, idee, Relation::DependsOn).unwrap();
    let selection = BTreeSet::from([idee, appui]);
    let plan = r
        .repository()
        .restore_plan(cible, Some(&selection))
        .unwrap();
    assert_eq!(plan.target, cible);
    assert_eq!(
        plan.items,
        vec![
            RestoreItem::Field {
                scope: S,
                node: idee,
                field: MemberField::Decision,
            },
            RestoreItem::Relation {
                scope: S,
                edge: interne,
            },
            RestoreItem::Kept {
                scope: S,
                edge: externe,
            },
        ]
    );
    let command = Command::Change(Change::RestoreNodes {
        target: cible,
        nodes: selection.clone(),
    });
    let preview = r.repository().preview(&command).unwrap();
    assert_eq!(preview.effects, plan.effects);
    let avant = r.snapshot().clone();
    let indep_avant = r
        .snapshot()
        .graph(S)
        .unwrap()
        .node(independante)
        .unwrap()
        .clone();
    r.restore_nodes(cible, &[idee, appui]).unwrap();
    let g = r.snapshot().graph(S).unwrap();
    assert_eq!(g.node(idee).unwrap().decision(), Some(Decision::Proposed));
    assert!(g.edge(interne).unwrap().active());
    assert!(g.edge(externe).unwrap().active());
    assert_eq!(g.node(independante).unwrap(), &indep_avant);
    assert!(g.node(nouvelle).is_ok());
    // Un seul « annuler » ramène l'état d'avant la restauration.
    r.undo().unwrap();
    assert_eq!(r.snapshot(), &avant);
}

// RESTAURATION.md, section 2 : une version plus récente du texte est remplacée par celle de la
// cible et signalée dans le plan ; elle reste dans l'historique (D6). Statuts, ancrages et
// acquittements de la sélection suivent la cible.
#[test]
fn a_targeted_restore_replaces_later_versions_and_says_so() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let cause = note(&mut r, Kind::Reasoning, "cause");
    let idee = note(&mut r, Kind::Implementation, "idée");
    r.link(S, idee, cause, Relation::DependsOn).unwrap();
    let ancien = r.anchor(idee, "src/a.c", "void a() {}", None).unwrap();
    let cible = r.head();
    r.revise(idee, "texte révisé", RevisionKind::Minor).unwrap();
    let revisee = r.head();
    r.unanchor(ancien).unwrap();
    let nouveau = r.anchor(idee, "src/b.c", "void b() {}", None).unwrap();
    r.invalidate(S, cause).unwrap();
    r.acknowledge(S, idee, cause).unwrap();
    let selection = BTreeSet::from([idee]);
    let plan = r
        .repository()
        .restore_plan(cible, Some(&selection))
        .unwrap();
    assert_eq!(
        plan.items,
        vec![
            RestoreItem::Text {
                node: idee,
                target_version: 0,
                current_version: 1,
            },
            RestoreItem::Anchor { anchor: ancien },
            RestoreItem::Anchor { anchor: nouveau },
            RestoreItem::Ack {
                scope: S,
                dependent: idee,
                cause,
            },
        ]
    );
    r.restore_nodes(cible, &[idee]).unwrap();
    let s = r.snapshot();
    assert_eq!(
        s.graph(S).unwrap().node(idee).unwrap().text(),
        "texte initial"
    );
    assert!(s.anchor(ancien).unwrap().active());
    assert!(!s.anchor(nouveau).unwrap().active());
    // L'acquittement, venu après la cible, n'y était pas : la cause revient.
    assert_eq!(
        s.graph(S).unwrap().review_causes().get(&idee),
        Some(&BTreeSet::from([cause]))
    );
    // La cause, hors sélection, garde son état courant.
    assert_eq!(
        s.graph(S).unwrap().node(cause).unwrap().verdict(),
        Some(Verdict::Refuted)
    );
    // La version remplacée reste dans l'historique.
    let historique = r.snapshot_at(revisee).unwrap();
    assert_eq!(
        historique.graph(S).unwrap().node(idee).unwrap().text(),
        "texte révisé"
    );
}

// RESTAURATION.md, section 2 : sélection vide, note créée après la cible, commit inconnu, état
// résultant invalide (cycle avec des relations extérieures conservées), restauration dans un lot :
// refus sans effet (INV-TX-01).
#[test]
fn invalid_targeted_restores_are_refused() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let c1 = note(&mut r, Kind::Category, "c1");
    let c2 = note(&mut r, Kind::Category, "c2");
    let c3 = note(&mut r, Kind::Category, "c3");
    let interne = r.link(S, c1, c2, Relation::ClassifiedIn).unwrap();
    let cible = r.head();
    r.detach(S, interne).unwrap();
    r.link(S, c2, c3, Relation::ClassifiedIn).unwrap();
    r.link(S, c3, c1, Relation::ClassifiedIn).unwrap();
    let tard = note(&mut r, Kind::Reasoning, "créée après la cible");
    let control = r.clone();
    let inconnu = CommitId::from_bytes(raw(4242));
    for (target, nodes, refusal) in [
        (cible, vec![], Error::EmptySelection),
        (cible, vec![c1, tard], Error::AbsentFromTarget(tard)),
        (inconnu, vec![c1], Error::MissingCommit(inconnu)),
        (cible, vec![c1, c2], Error::CyclicRelation),
    ] {
        let selection: BTreeSet<NodeId> = nodes.iter().copied().collect();
        assert_eq!(
            r.repository()
                .restore_plan(target, Some(&selection))
                .map(|_| ()),
            Err(refusal.clone())
        );
        assert_eq!(r.restore_nodes(target, &nodes), Err(refusal));
        assert_eq!(r, control);
    }
    let lot = Command::Lot(vec![Change::RestoreNodes {
        target: cible,
        nodes: BTreeSet::from([c1]),
    }]);
    assert_eq!(r.execute(lot), Err(Error::NotInLot));
    assert_eq!(r, control);
    assert_eq!(Error::EmptySelection.to_string(), "sélection vide");
    assert_eq!(
        Error::AbsentFromTarget(tard).to_string(),
        format!("note absente de l'état à restaurer : {tard}")
    );
}

// D13, D1 : une restauration ciblée est critique ; engagée sur main, elle crée une branche. Le
// plan sans sélection est celui de la restauration complète (INV-RET-03).
#[test]
fn targeted_restores_are_critical_and_full_restores_have_a_plan() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let idee = note(&mut r, Kind::Reasoning, "idée");
    let cible = r.head();
    r.revise(idee, "révisée", RevisionKind::Minor).unwrap();
    let tard = note(&mut r, Kind::PersonalNote, "créée après");
    assert_eq!(r.active_branch(), MAIN);
    let selection = BTreeSet::from([idee]);
    let command = Command::Change(Change::RestoreNodes {
        target: cible,
        nodes: selection.clone(),
    });
    assert!(r.repository().preview(&command).unwrap().forks);
    let complet = r.repository().restore_plan(cible, None).unwrap();
    assert_eq!(
        complet.items,
        vec![
            RestoreItem::Text {
                node: idee,
                target_version: 0,
                current_version: 1,
            },
            RestoreItem::Field {
                scope: S,
                node: tard,
                field: MemberField::Visibility,
            },
        ]
    );
    let full = Command::Change(Change::Restore { target: cible });
    assert_eq!(
        complet.effects,
        r.repository().preview(&full).unwrap().effects
    );
    r.restore_nodes(cible, &[idee]).unwrap();
    assert_ne!(r.active_branch(), MAIN);
}

// RESTAURATION.md, section 3 : l'historique remonte les premiers parents de la tête de la branche
// active, du plus récent au plus ancien, avec le motif et la commande de chaque commit ; la racine
// n'a pas de commande.
#[test]
fn the_branch_history_lists_commits_newest_first() {
    let mut repo = Repository::new();
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Reasoning)),
        (11, create(DEFAULT_SCOPE, 2, Kind::Implementation)),
        (12, link(3, 2, 1, Relation::DependsOn)),
    ] {
        let next = reasoned(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let historique = repo.history_of_branch(10).unwrap();
    let vus: Vec<(CommitId, Option<&str>, &str)> = historique
        .iter()
        .map(|h| {
            (
                h.commit,
                h.command.as_ref().map(command_name),
                h.reason.as_str(),
            )
        })
        .collect();
    assert_eq!(
        vus,
        vec![
            (CommitId::from_bytes(raw(12)), Some("lier"), "motif"),
            (CommitId::from_bytes(raw(11)), Some("creer-note"), "motif"),
            (CommitId::from_bytes(raw(10)), Some("creer-note"), "motif"),
            (ROOT, None, ""),
        ]
    );
    assert_eq!(repo.history_of_branch(2).unwrap().len(), 2);
    assert_eq!(command_name(&Command::Undo), "annuler");
}

// VERSION2.md, section 6 : bloc `restaurer-notes`, sélection triée et sans doublon ; il se relit
// à l'identique, dépôt complet compris.
#[test]
fn targeted_restore_blocks_round_trip() {
    let mut repo = Repository::new();
    for (op, command) in [
        (10, create(DEFAULT_SCOPE, 1, Kind::Reasoning)),
        (11, create(DEFAULT_SCOPE, 2, Kind::Implementation)),
    ] {
        let next = reasoned(&repo, op, command);
        repo.execute(next).unwrap();
    }
    let restore = reasoned(
        &repo,
        12,
        Command::Change(Change::RestoreNodes {
            target: CommitId::from_bytes(raw(10)),
            nodes: BTreeSet::from([node_id(1)]),
        }),
    );
    let outcome = repo.execute(restore.clone()).unwrap();
    let block = encode_operation_v2(&restore, &outcome);
    let line = format!(
        "  commande restaurer-notes cible {} notes 1 {}\n",
        raw_text(10),
        raw_text(1)
    );
    assert!(block.contains(&line), "{block}");
    assert_eq!(decode_operation_v2(&block), Ok((restore, outcome)));
    let text = encode_repository_v2(&repo);
    assert_eq!(decode_repository_v2(&text), Ok(repo.clone()));
    let unsorted = block.replace(
        &line,
        &format!(
            "  commande restaurer-notes cible {} notes 2 {} {}\n",
            raw_text(10),
            raw_text(2),
            raw_text(1)
        ),
    );
    assert_eq!(
        decode_operation_v2(&unsorted),
        Err(DecodeError {
            line: 2,
            reason: DecodeReason::Order,
        })
    );
}

// La restauration ciblée est propre à la version 2 : inconnue de la version 1 (ajouté après la
// mutation de 6.4a).
#[test]
fn targeted_restores_are_refused_in_v1() {
    let number = repository_line("  commande creer-note ");
    let command = format!(
        "  commande restaurer-notes cible {MAIN} notes 1 00000000-0000-0001-0000-000000000002"
    );
    let mut lines: Vec<&str> = REPOSITORY_EXAMPLE_V1.lines().collect();
    lines[number - 1] = &command;
    assert_eq!(
        repository_refusal(&(lines.join("\n") + "\n")),
        DecodeError {
            line: number,
            reason: DecodeReason::Value("commande")
        }
    );
}

// ---------------------------------------------------------------------------
// Incrément 6.5a — contrat ajouté : fiche de reprise (FICHE-REPRISE.md, D26 ; INV-RESUME-01,
// INV-RESUME-02 ; CA-08).
// ---------------------------------------------------------------------------

/// Fiche minimale : les deux champs obligatoires.
fn point(objective: &str) -> ResumePoint {
    ResumePoint {
        objective: objective.into(),
        next_action: "action suivante".into(),
        ..ResumePoint::default()
    }
}

/// État du code d'essai, patch court.
fn code(commit: Option<&str>, changes: &[&str], diff: &str) -> CodeState {
    CodeState::new(
        commit.map(str::to_owned),
        Some("rtos-dev".to_owned()),
        changes.iter().map(|c| (*c).to_owned()).collect(),
        diff.as_bytes(),
    )
}

// INV-RESUME-01 (D26) : une fiche publie un commit dont l'état est celui de son parent ; elle ne
// bifurque jamais, même sur main ; elle se relit dans le journal ; elle figure dans l'historique ;
// « annuler » la défait et « rétablir » la rend. Aucun motif n'est exigé.
#[test]
fn a_resume_point_publishes_a_commit_without_changing_the_network() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "réveil perdu");
    let parent = r.head();
    let avant = r.snapshot().clone();
    let fiche = ResumePoint {
        date: "2026-11-08 17:42 UTC".into(),
        question: "le tick peut-il réveiller une tâche prête ?".into(),
        reached: "reproduit".into(),
        result: "test_wakeup échoue 1 fois sur 20".into(),
        hypothesis: "course avec l'ISR".into(),
        open: "src/sched.c (wake_task)".into(),
        notes: BTreeSet::from([n]),
        code: Some(code(
            Some("a1b2c3d4"),
            &[" M src/sched.c"],
            "diff --git a b\n",
        )),
        ..point("isoler le réveil perdu")
    };
    let command = Command::Resume(Box::new(fiche.clone()));
    let preview = r.repository().preview(&command).unwrap();
    assert!(preview.publishes);
    assert!(!preview.forks);
    assert_eq!(preview.effects, Effects::default());
    let outcome = r.resume(fiche.clone()).unwrap();
    assert!(outcome.published);
    assert!(!outcome.forked);
    assert_eq!(outcome.branch, MAIN);
    assert_ne!(r.head(), parent);
    assert_eq!(r.snapshot(), &avant);
    assert_eq!(
        *r.snapshot_at(r.head()).unwrap(),
        *r.snapshot_at(parent).unwrap()
    );
    assert_eq!(r.repository().resume_point(r.head()), Ok(fiche.clone()));
    let historique = r.repository().history_of_branch(1).unwrap();
    assert_eq!(historique[0].commit, r.head());
    assert_eq!(
        historique[0].command.as_ref().map(command_name),
        Some("reprise")
    );
    let fiche_commit = r.head();
    r.undo().unwrap();
    assert_eq!(r.head(), parent);
    assert_eq!(r.repository().resume_points(MAIN, 10), Ok(vec![]));
    // Annulée, la fiche reste lisible dans le journal (INV-UNDO-02).
    assert_eq!(r.repository().resume_point(fiche_commit), Ok(fiche.clone()));
    r.redo().unwrap();
    assert_eq!(r.head(), fiche_commit);
    // Sans motif.
    let mut repo = Repository::new();
    let sans_motif = envelope(&repo, 10, Command::Resume(Box::new(point("objectif"))));
    assert!(repo.execute(sans_motif).unwrap().published);
}

// FICHE-REPRISE.md, section 3 : fiches d'une branche le long des premiers parents, de la plus
// récente à la plus ancienne, au plus `limit` ; celles de main avant la bifurcation font partie
// de l'historique de la branche. Nombre d'opérations depuis une fiche, ou aucun si elle n'est pas
// dans l'historique de la branche active.
#[test]
fn resume_points_are_listed_per_branch_newest_first() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "hypothèse");
    r.resume(point("premier")).unwrap();
    let premier = r.head();
    note(&mut r, Kind::PersonalNote, "entre deux");
    r.resume(point("second")).unwrap();
    let second = r.head();
    let lister = |r: &Harness, branch, limit| -> Vec<(CommitId, String)> {
        r.repository()
            .resume_points(branch, limit)
            .unwrap()
            .into_iter()
            .map(|e| (e.commit, e.point.objective))
            .collect()
    };
    assert_eq!(
        lister(&r, MAIN, 10),
        vec![(second, "second".into()), (premier, "premier".into())]
    );
    assert_eq!(lister(&r, MAIN, 1), vec![(second, "second".into())]);
    assert_eq!(lister(&r, MAIN, 0), vec![]);
    assert_eq!(r.repository().operations_since(premier), Ok(Some(2)));
    assert_eq!(r.repository().operations_since(second), Ok(Some(0)));
    let branche = r.invalidate(DEFAULT_SCOPE, n).unwrap();
    r.resume(point("sur la branche")).unwrap();
    let sur_branche = r.head();
    assert_eq!(r.active_branch(), branche);
    assert_eq!(
        lister(&r, branche, 10),
        vec![
            (sur_branche, "sur la branche".into()),
            (second, "second".into()),
            (premier, "premier".into()),
        ]
    );
    assert_eq!(lister(&r, MAIN, 10).len(), 2);
    assert_eq!(r.repository().operations_since(second), Ok(Some(2)));
    r.checkout(MAIN).unwrap();
    assert_eq!(r.repository().operations_since(sur_branche), Ok(None));
    // Refus : commit inconnu, commit qui n'est pas une fiche, racine, branche inconnue.
    let inconnu = CommitId::from_bytes(raw(4242));
    assert_eq!(
        r.repository().operations_since(inconnu),
        Err(Error::MissingCommit(inconnu))
    );
    assert_eq!(
        r.repository().resume_point(inconnu),
        Err(Error::MissingCommit(inconnu))
    );
    let ordinaire = r.repository().history_of_branch(3).unwrap()[1].commit;
    assert_eq!(
        r.repository().resume_point(ordinaire),
        Err(Error::NotAResumePoint(ordinaire))
    );
    assert_eq!(
        r.repository().resume_point(ROOT),
        Err(Error::NotAResumePoint(ROOT))
    );
    let autre = BranchId::from_bytes(raw(4343));
    assert_eq!(
        r.repository().resume_points(autre, 10).map(|_| ()),
        Err(Error::MissingBranch(autre))
    );
    assert_eq!(
        Error::NotAResumePoint(ROOT).to_string(),
        format!("pas un point de reprise : {ROOT}")
    );
}

// FICHE-REPRISE.md, section 2 : objectif et prochaine action obligatoires (des espaces ne
// suffisent pas) ; chaque note citée existe dans l'état courant. Refus sans effet (INV-TX-01).
#[test]
fn invalid_resume_points_are_refused() {
    let mut r = Harness::new();
    let n = note(&mut r, Kind::Reasoning, "existante");
    let control = r.clone();
    let absente = NodeId::from_bytes(raw(4242));
    for (fiche, refusal) in [
        (point("  "), Error::EmptyResumeField("objectif")),
        (
            ResumePoint {
                next_action: "\n".into(),
                ..point("objectif")
            },
            Error::EmptyResumeField("prochaine action"),
        ),
        (
            ResumePoint {
                notes: BTreeSet::from([n, absente]),
                ..point("objectif")
            },
            Error::MissingNode(absente),
        ),
    ] {
        let command = Command::Resume(Box::new(fiche.clone()));
        assert_eq!(
            r.repository().preview(&command).map(|_| ()),
            Err(refusal.clone())
        );
        assert_eq!(r.resume(fiche), Err(refusal));
        assert_eq!(r, control);
    }
    assert_eq!(
        Error::EmptyResumeField("objectif").to_string(),
        "champ obligatoire vide dans le point de reprise : objectif"
    );
}

// INV-RESUME-02 (FICHE-REPRISE.md, section 3) : l'état du code noté est trié et sans doublon ; le
// travail non enregistré est conservé jusqu'à PATCH_LIMIT octets s'il est en UTF-8, sinon son
// empreinte et sa taille seulement ; la comparaison signale un commit différent et un travail non enregistré différent,
// pas un simple nom de branche.
#[test]
fn code_states_are_compared() {
    let etat = CodeState::new(
        Some("c1".into()),
        None,
        vec!["?? b.c".into(), " M a.c".into(), "?? b.c".into()],
        b"patch",
    );
    assert_eq!(etat.commit(), Some("c1"));
    assert_eq!(etat.branch(), None);
    assert_eq!(etat.changes(), [" M a.c".to_owned(), "?? b.c".to_owned()]);
    assert_eq!(etat.patch(), &Patch::Kept("patch".into()));
    let juste = "x".repeat(PATCH_LIMIT);
    assert_eq!(PATCH_LIMIT, 4 * 1024 * 1024);
    assert_eq!(Patch::new(juste.as_bytes()), Patch::Kept(juste.clone()));
    let trop = "y".repeat(PATCH_LIMIT + 1);
    assert_eq!(
        Patch::new(trop.as_bytes()),
        Patch::NotKept {
            bytes: PATCH_LIMIT + 1,
            digest: text_digest(&trop),
        }
    );
    // Contrat modifié (6.5a) : un patch hors UTF-8, même court, n'est pas conservé, plutôt que
    // corrompu ; l'empreinte porte sur ses octets.
    let latin1 = [b'c', 0xe9, b'\n'];
    assert_eq!(
        Patch::new(&latin1),
        Patch::NotKept {
            bytes: 3,
            digest: bytes_digest(&latin1),
        }
    );
    assert_eq!(bytes_digest(trop.as_bytes()), text_digest(&trop));
    let note = code(Some("c1"), &[" M a.c"], "d1");
    let fiche = ResumePoint {
        code: Some(note.clone()),
        ..point("objectif")
    };
    let compare = |now: Option<&CodeState>| fiche.code_change(now);
    assert_eq!(
        point("objectif").code_change(Some(&note)),
        CodeChange::NotRecorded
    );
    assert_eq!(point("objectif").code_change(None), CodeChange::NotRecorded);
    assert_eq!(compare(None), CodeChange::Unavailable);
    let same = |commit: bool, work: bool| CodeChange::Compared { commit, work };
    assert_eq!(compare(Some(&note)), same(false, false));
    assert_eq!(
        compare(Some(&code(Some("c2"), &[" M a.c"], "d1"))),
        same(true, false)
    );
    assert_eq!(
        compare(Some(&code(None, &[" M a.c"], "d1"))),
        same(true, false)
    );
    assert_eq!(
        compare(Some(&code(Some("c1"), &[], "d1"))),
        same(false, true)
    );
    assert_eq!(
        compare(Some(&code(Some("c1"), &[" M a.c"], "d2"))),
        same(false, true)
    );
    assert_eq!(compare(Some(&code(Some("c2"), &[], ""))), same(true, true));
    let autre_branche = CodeState::new(
        Some("c1".into()),
        Some("autre".into()),
        vec![" M a.c".into()],
        b"d1",
    );
    assert_eq!(compare(Some(&autre_branche)), same(false, false));
}

/// Opération d'essai : une fiche complète sur le dépôt donné.
fn resume_envelope(repo: &Repository, op: u64, code: Option<CodeState>) -> Envelope {
    envelope(
        repo,
        op,
        Command::Resume(Box::new(ResumePoint {
            date: "2026-11-08 17:42 UTC".into(),
            objective: "objectif".into(),
            next_action: "action".into(),
            question: "question".into(),
            reached: "etat".into(),
            result: "resultat".into(),
            hypothesis: "hypothese".into(),
            open: "ouverts".into(),
            notes: BTreeSet::from([node_id(1)]),
            code,
        })),
    )
}

// VERSION2.md, section 6 : bloc `reprise`, champs dans l'ordre, état du code sur la même ligne ;
// il se relit à l'identique, dépôt complet compris ; changements triés ; patch conservé au plus
// PATCH_LIMIT octets.
#[test]
fn resume_blocks_round_trip() {
    let mut repo = Repository::new();
    let create_first = reasoned(&repo, 9, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    repo.execute(create_first).unwrap();
    let debut = format!(
        "  commande reprise date \"2026-11-08 17:42 UTC\" objectif \"objectif\" action \"action\" \
         question \"question\" etat \"etat\" resultat \"resultat\" hypothese \"hypothese\" \
         ouverts \"ouverts\" notes 1 {} code",
        raw_text(1)
    );
    let complet = code(Some("a1b2"), &["?? b.c", " M a.c"], "l1\nl2\n");
    let trop = CodeState::new(None, None, vec![], "z".repeat(PATCH_LIMIT + 1).as_bytes());
    for (op, state, suite) in [
        (10, None, " -".to_owned()),
        (
            11,
            Some(complet),
            " commit \"a1b2\" branche \"rtos-dev\" changements 2 \" M a.c\" \"?? b.c\" \
             patch texte \"l1\\nl2\\n\""
                .to_owned(),
        ),
        (
            12,
            Some(trop),
            format!(
                " commit - branche - changements 0 patch non-conserve octets {} empreinte {:016x}",
                PATCH_LIMIT + 1,
                text_digest(&"z".repeat(PATCH_LIMIT + 1))
            ),
        ),
    ] {
        let fiche = resume_envelope(&repo, op, state);
        let outcome = repo.execute(fiche.clone()).unwrap();
        let block = encode_operation_v2(&fiche, &outcome);
        let line = format!("{debut}{suite}\n");
        assert!(block.contains(&line), "{block}");
        assert_eq!(decode_operation_v2(&block), Ok((fiche, outcome)));
    }
    let text = encode_repository_v2(&repo);
    assert_eq!(decode_repository_v2(&text), Ok(repo.clone()));
    // Écarts refusés et situés.
    let fiche = resume_envelope(&repo, 13, Some(code(None, &["a", "b"], "p")));
    let outcome = repo.execute(fiche.clone()).unwrap();
    let block = encode_operation_v2(&fiche, &outcome);
    let ordre = "changements 2 \"a\" \"b\"";
    assert!(block.contains(ordre), "{block}");
    for (from, to, reason) in [
        (ordre, "changements 2 \"b\" \"a\"", DecodeReason::Order),
        (ordre, "changements 2 \"a\" \"a\"", DecodeReason::Order),
        ("patch texte", "patch autre", DecodeReason::Value("patch")),
    ] {
        assert_eq!(
            decode_operation_v2(&block.replace(from, to)),
            Err(DecodeError { line: 2, reason }),
            "{to}"
        );
    }
    let long = block.replace(
        "patch texte \"p\"",
        &format!("patch texte \"{}\"", "w".repeat(PATCH_LIMIT + 1)),
    );
    assert_eq!(
        decode_operation_v2(&long),
        Err(DecodeError {
            line: 2,
            reason: DecodeReason::Value("patch"),
        })
    );
    assert_eq!(command_name(&fiche.command), "reprise");
}

// La fiche de reprise est propre à la version 2 : inconnue de la version 1.
#[test]
fn resume_points_are_refused_in_v1() {
    let number = repository_line("  commande creer-note ");
    let command =
        "  commande reprise date \"\" objectif \"o\" action \"a\" question \"\" etat \"\" \
                   resultat \"\" hypothese \"\" ouverts \"\" notes 0 code -";
    let mut lines: Vec<&str> = REPOSITORY_EXAMPLE_V1.lines().collect();
    lines[number - 1] = command;
    assert_eq!(
        repository_refusal(&(lines.join("\n") + "\n")),
        DecodeError {
            line: number,
            reason: DecodeReason::Value("commande")
        }
    );
}

// L'état du code noté se relit tel quel, branche comprise (ajouté après la mutation de 6.5a).
#[test]
fn code_state_accessors_give_back_what_was_noted() {
    let etat = code(Some("c1"), &[" M a.c"], "d");
    assert_eq!(etat.branch(), Some("rtos-dev"));
    assert_eq!(etat.commit(), Some("c1"));
}

// Un patch d'exactement PATCH_LIMIT octets est conservé et se relit (ajouté après la mutation de
// 6.5a).
#[test]
fn a_patch_of_exactly_the_limit_is_kept_and_read_back() {
    let mut repo = Repository::new();
    let create_first = reasoned(&repo, 9, create(DEFAULT_SCOPE, 1, Kind::Reasoning));
    repo.execute(create_first).unwrap();
    let juste = "v".repeat(PATCH_LIMIT);
    let fiche = resume_envelope(
        &repo,
        10,
        Some(CodeState::new(None, None, vec![], juste.as_bytes())),
    );
    let outcome = repo.execute(fiche.clone()).unwrap();
    let block = encode_operation_v2(&fiche, &outcome);
    assert!(block.contains(&format!("patch texte \"{juste}\"")));
    assert_eq!(decode_operation_v2(&block), Ok((fiche, outcome)));
}

// ---------------------------------------------------------------------------
// Incrément 6.6a — contrat ajouté : vue locale colorée (VOISINAGE.md ; INV-VIEW-02 ; R7).
// ---------------------------------------------------------------------------

// INV-VIEW-02 (VOISINAGE.md, section 2) : toutes les relations actives qui touchent la note,
// chacune une fois ; origine catégorie pour « classée dans », citation quand l'autre extrémité est
// une citation (d'un échange ou d'une note), note entière sinon ; une relation détachée, ou qui ne
// touche pas la note, est exclue ; ordre : origine, sens, type, autre note.
#[test]
fn the_neighborhood_classifies_each_link_by_origin() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let centre = note(&mut r, Kind::Reasoning, "centre");
    let appui = note(&mut r, Kind::Implementation, "appui");
    let voisine = note(&mut r, Kind::PersonalNote, "voisine");
    let categorie = note(&mut r, Kind::Category, "Scheduler");
    let echange = r
        .create_exchange(
            S,
            "échange",
            Some("question ?"),
            "le tick réveille la tâche",
        )
        .unwrap();
    let citation = r.create_passage(S, echange, Part::Answer, 0, 7).unwrap();
    let citation_de_note = r.cite_note(S, appui, 0, 0, 5).unwrap();
    let detachee = note(&mut r, Kind::PersonalNote, "détachée");
    let depend = r.link(S, centre, appui, Relation::DependsOn).unwrap();
    let depend_echange = r.link(S, centre, echange, Relation::DependsOn).unwrap();
    let lien = r.link(S, voisine, centre, Relation::Related).unwrap();
    let classe = r
        .link(S, centre, categorie, Relation::ClassifiedIn)
        .unwrap();
    let soutien = r.link(S, centre, citation, Relation::DependsOn).unwrap();
    let inspire = r
        .link(S, centre, citation_de_note, Relation::DevelopedFrom)
        .unwrap();
    let ailleurs = r.link(S, voisine, appui, Relation::Related).unwrap();
    let retiree = r.link(S, centre, detachee, Relation::Related).unwrap();
    r.detach(S, retiree).unwrap();
    let graphe = r.snapshot().graph(S).unwrap();
    let lien_de = |edge, direction, relation, other, origin| NeighborLink {
        edge,
        direction,
        relation,
        other,
        origin,
    };
    use Direction::{Incoming, Outgoing};
    assert_eq!(
        graphe.neighborhood(centre),
        Ok(vec![
            lien_de(
                depend,
                Outgoing,
                Relation::DependsOn,
                appui,
                LinkOrigin::WholeNote
            ),
            lien_de(
                depend_echange,
                Outgoing,
                Relation::DependsOn,
                echange,
                LinkOrigin::WholeNote
            ),
            lien_de(
                lien,
                Incoming,
                Relation::Related,
                voisine,
                LinkOrigin::WholeNote
            ),
            lien_de(
                soutien,
                Outgoing,
                Relation::DependsOn,
                citation,
                LinkOrigin::Citation
            ),
            lien_de(
                inspire,
                Outgoing,
                Relation::DevelopedFrom,
                citation_de_note,
                LinkOrigin::Citation
            ),
            lien_de(
                classe,
                Outgoing,
                Relation::ClassifiedIn,
                categorie,
                LinkOrigin::Category
            ),
        ])
    );
    // Catégorie vue de l'autre côté : la note classée dans la catégorie est entrante.
    assert_eq!(
        graphe.neighborhood(categorie),
        Ok(vec![lien_de(
            classe,
            Incoming,
            Relation::ClassifiedIn,
            centre,
            LinkOrigin::Category
        )])
    );
    // Vue d'une citation : l'autre extrémité n'est pas une citation, le lien est « note entière ».
    assert_eq!(
        graphe.neighborhood(citation),
        Ok(vec![lien_de(
            soutien,
            Incoming,
            Relation::DependsOn,
            centre,
            LinkOrigin::WholeNote
        )])
    );
    assert_eq!(
        graphe.neighborhood(appui).map(|v| v.len()),
        Ok(2),
        "{ailleurs}"
    );
    assert_eq!(graphe.neighborhood(detachee), Ok(vec![]));
    let absente = NodeId::from_bytes(raw(4242));
    assert_eq!(
        graphe.neighborhood(absente),
        Err(Error::MissingNode(absente))
    );
    // L'état d'une note écartée se lit publiquement (pour l'affichage de la vue).
    assert!(!graphe.node(appui).unwrap().withdrawn());
    r.invalidate(S, appui).unwrap();
    assert!(r
        .snapshot()
        .graph(S)
        .unwrap()
        .node(appui)
        .unwrap()
        .withdrawn());
}

// ---------------------------------------------------------------------------
// Incrément 7.2a — contrat ajouté : retrouver les notes d'un code (RETROUVER.md ; INV-CODE-07).
// ---------------------------------------------------------------------------

// INV-CODE-07 (RETROUVER.md, sections 2 et 3) : pour un fichier, les notes de ses ancrages actifs
// (chemins comparés relativement au dossier, chemin proche à part, ancrage retiré ignoré) et les
// marques de son texte (connue, inconnue, mal formée) ; avec une ligne, l'ancrage dont l'extrait la
// contient d'abord et la marque la plus proche au-dessus ; avec un extrait, l'ancrage qui le
// contient d'abord et les marques qu'il couvre.
#[test]
fn the_notes_of_a_code_are_found_from_a_file_a_line_or_an_excerpt() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let n = |r: &mut Harness, titre| note(r, Kind::Implementation, titre);
    let (reveil, file, seule, retiree, proche, ailleurs) = (
        n(&mut r, "réveil"),
        n(&mut r, "file"),
        n(&mut r, "seulement marquée"),
        n(&mut r, "ancrage retiré"),
        n(&mut r, "chemin proche"),
        n(&mut r, "ailleurs"),
    );
    let corps = "void wake_task(struct task *t)\n{\n    t->etat = PRETE;\n}";
    let a_reveil = r.anchor(reveil, "./src/sched.c", corps, None).unwrap();
    let a_file = r
        .anchor(file, "/projet/src/sched.c", "int file[8];", None)
        .unwrap();
    let a_retiree = r.anchor(retiree, "src/sched.c", "rien", None).unwrap();
    r.unanchor(a_retiree).unwrap();
    let a_proche = r.anchor(proche, "lib/autre/sched.c", "x", None).unwrap();
    r.anchor(ailleurs, "src/other.c", "y", None).unwrap();
    let inconnue = NodeId::from_bytes(raw(4242));
    let texte = format!(
        "#include <x.h>\n// @design: {seule}\nint file[8];\n\n// @design: {reveil}\n{corps}\n\
         // @design: {inconnue}\n/* @design: 1234 */\n"
    );
    let s = r.snapshot();
    let ancre = |anchor, node, matched, path: &str| AnchoredNote {
        anchor,
        node,
        matched,
        path: path.into(),
    };
    let marque = |line, node, known, in_focus| MarkerHit {
        line,
        node,
        known,
        in_focus,
    };
    use AnchorMatch::{ContainsExcerpt, ContainsLine, FileOnly};
    // Fichier entier.
    let tout = s
        .code_notes(S, "/projet", "src/sched.c", &texte, &Focus::File)
        .unwrap();
    assert_eq!(
        tout.anchors,
        vec![
            ancre(a_reveil, reveil, FileOnly, "src/sched.c"),
            ancre(a_file, file, FileOnly, "src/sched.c"),
        ]
    );
    assert_eq!(
        tout.near,
        vec![ancre(a_proche, proche, FileOnly, "lib/autre/sched.c")]
    );
    assert_eq!(
        tout.markers,
        vec![
            marque(2, Some(seule), true, false),
            marque(5, Some(reveil), true, false),
            marque(10, Some(inconnue), false, false),
            marque(11, None, false, false),
        ]
    );
    // Une ligne dans le corps de la fonction : son ancrage d'abord, la marque juste au-dessus.
    let ligne = s
        .code_notes(S, "/projet", "./src/sched.c", &texte, &Focus::Line(8))
        .unwrap();
    assert_eq!(
        ligne.anchors,
        vec![
            ancre(a_reveil, reveil, ContainsLine, "src/sched.c"),
            ancre(a_file, file, FileOnly, "src/sched.c"),
        ]
    );
    assert_eq!(
        ligne
            .markers
            .iter()
            .filter(|m| m.in_focus)
            .map(|m| m.line)
            .collect::<Vec<_>>(),
        vec![5]
    );
    // Une ligne vide ne désigne aucun extrait ; la marque la plus proche reste celle du dessus.
    let vide = s
        .code_notes(S, "/projet", "src/sched.c", &texte, &Focus::Line(4))
        .unwrap();
    assert!(vide.anchors.iter().all(|a| a.matched == FileOnly));
    assert_eq!(
        vide.markers
            .iter()
            .filter(|m| m.in_focus)
            .map(|m| m.line)
            .collect::<Vec<_>>(),
        vec![2]
    );
    // Avant toute marque : aucune la plus proche.
    let tete = s
        .code_notes(S, "/projet", "src/sched.c", &texte, &Focus::Line(1))
        .unwrap();
    assert!(tete.markers.iter().all(|m| !m.in_focus));
    // Un extrait contenu dans l'extrait ancré, puis un extrait qui contient l'extrait ancré.
    let extrait = s
        .code_notes(
            S,
            "/projet",
            "src/sched.c",
            &texte,
            &Focus::Excerpt("void wake_task(struct task *t)\n{".into()),
        )
        .unwrap();
    assert_eq!(
        extrait.anchors[0],
        ancre(a_reveil, reveil, ContainsExcerpt, "src/sched.c")
    );
    assert_eq!(extrait.anchors[1].matched, FileOnly);
    let large = s
        .code_notes(
            S,
            "/projet",
            "src/sched.c",
            &texte,
            &Focus::Excerpt(format!("// @design: {seule}\nint file[8];\n")),
        )
        .unwrap();
    assert_eq!(
        large.anchors[0],
        ancre(a_file, file, ContainsExcerpt, "src/sched.c")
    );
    assert_eq!(
        large
            .markers
            .iter()
            .filter(|m| m.in_focus)
            .map(|m| m.line)
            .collect::<Vec<_>>(),
        vec![2]
    );
    // Un extrait absent du fichier ne couvre aucune marque.
    let absent = s
        .code_notes(
            S,
            "/projet",
            "src/sched.c",
            &texte,
            &Focus::Excerpt("zzz".into()),
        )
        .unwrap();
    assert!(absent.markers.iter().all(|m| !m.in_focus));
    assert!(absent.anchors.iter().all(|a| a.matched == FileOnly));
    // Un fichier sans ancrage ni marque ; un contexte absent.
    let rien = s
        .code_notes(S, "/projet", "src/vide.c", "int x;\n", &Focus::File)
        .unwrap();
    assert_eq!(rien, CodeNotes::default());
    let absent = ScopeId::from_bytes(raw(4343));
    assert_eq!(
        s.code_notes(absent, "/projet", "src/sched.c", &texte, &Focus::File),
        Err(Error::MissingScope(absent))
    );
}

// Lignes couvertes par un extrait collé, exactement (ajouté après la mutation de 7.2a) : l'extrait
// commence à sa ligne et finit à sa dernière ligne.
#[test]
fn an_excerpt_covers_exactly_its_lines() {
    const S: ScopeId = DEFAULT_SCOPE;
    let mut r = Harness::new();
    let a = note(&mut r, Kind::Implementation, "a");
    let b = note(&mut r, Kind::Implementation, "b");
    let texte = format!("x\n// @design: {a}\nint file[8];\n\n// @design: {b}\nfin\n");
    let couvertes = |extrait: &str| -> Vec<usize> {
        r.snapshot()
            .code_notes(S, "", "f.c", &texte, &Focus::Excerpt(extrait.into()))
            .unwrap()
            .markers
            .iter()
            .filter(|m| m.in_focus)
            .map(|m| m.line)
            .collect()
    };
    assert_eq!(couvertes("int file[8];"), Vec::<usize>::new());
    assert_eq!(couvertes(&format!("\n// @design: {b}")), vec![5]);
    assert_eq!(couvertes("int file[8];\n\n// @design"), vec![5]);
}
