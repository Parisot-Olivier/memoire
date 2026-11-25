# Traçabilité : critères d'acceptation ↔ invariants ↔ tests

Version 0.2.8 — 21 octobre 2026 — incréments 2.1 à 2.6c. Les critères correspondent aux identifiants CA du cahier des charges v0.2. Les tests cités sont ceux du contrat de memoire-domain (tests/contrat.rs), où les tests de memoire-core ont été portés sous le même nom.

## Pourquoi des identifiants stables

Dans le cahier des charges v0.1, l'export Markdown a fondu les listes numérotées dans la numérotation des sections : les critères d'acceptation y apparaissent numérotés de 13 à 52, à la suite de la section 12. Toute référence par numéro est donc ambiguë. Les critères reçoivent ici des identifiants stables CA-01 à CA-40, dans l'ordre du document ; la colonne « CDC » rappelle le numéro affiché dans la v0.1. Les identifiants CA ne sont jamais réattribués : un critère abandonné garde son numéro avec le statut « retiré ».

## Statuts

Le statut mesure ce que les tests actuels vérifient. Les scénarios fixent le résultat attendu, mais ne changent le statut qu’une fois exécutables (phase 2).


- **Couvert (noyau)** : la propriété est vérifiée par des tests du noyau ; l'interface et la persistance éventuellement nécessaires n'existent pas encore.
- **Partiel** : une partie seulement de la propriété est vérifiée ; la colonne « Écart » précise le manque.
- **Non couvert** : aucun mécanisme ni test.

Un critère n'est déclaré « Accepté » qu'une fois vérifié de bout en bout (persistance et interface comprises). Aucun ne l'est à ce stade.

## Matrice

| CA | CDC | Critère (résumé) | Invariants | Scénarios | Tests actuels | Statut | Écart |
|---|---|---|---|---|---|---|---|
| CA-01 | 13 | Après import, retrouver question et réponse dans l'ordre | INV-SOURCE-03, INV-SOURCE-04 | S-REF-01 ét. 1 | exchanges_keep_sources_byte_for_byte_with_explicit_boundaries, reference_scenario_steps_1_and_5_on_real_sources, conversation_order_is_kept_explicitly | Couvert (noyau) | — |
| CA-02 | 14 | Surligner une phrase sans modifier la source | INV-SOURCE-01, INV-SOURCE-03 | S-REF-01 ét. 3 à 5 | passages_designate_exact_ranges_and_can_be_corrected | Couvert (noyau) | — |
| CA-03 | 15 | Remplacer une affirmation, garder le reste et l'accès à la correction | INV-REL-02, INV-REL-05 | S-REL-02, S-MOD-04 | replacement_sets_status_atomically_and_stays_resolvable, passages_designate_exact_ranges_and_can_be_corrected | Couvert (noyau) | — |
| CA-04 | 16 | Depuis une fonction, ouvrir l'extrait exact, puis revenir | INV-CODE-01, INV-CODE-03 | S-REF-01 ét. 6 et 7 | reference_scenario_steps_1_to_9_on_real_sources, anchors_keep_path_copy_and_digest | Couvert (noyau) | Ouverture dans l’éditeur : interface |
| CA-05 | 17 | Après déplacement du code, retrouver ou signaler l'ancrage | INV-CODE-01, INV-CODE-05 | S-CODE-02 | the_code_report_classifies_every_anchor_and_marker (6.3a), indexing_reports_the_state_of_every_marker_and_anchor, indexing_skips_what_it_cannot_read (6.3b) | Couvert | Indexeur livré (6.3, INDEXEUR.md) : commande `indexer DOSSIER` |
| CA-06 | 18 | Après invalidation, lister les dépendants sans les déclarer faux | INV-SEM-02, INV-GRAPH-01, INV-RET-02 | S-REF-01 ét. 8 | dependency_review_preserves…, exhaustive_three_node…, substantive_revision_requires_review_of_dependents | Couvert (noyau) | — |
| CA-07 | 19 | Deux choix applicables à deux variantes simultanément | INV-SCOPE-01, INV-SCOPE-02 | S-SCOPE-01 | scopes_are_isolated, contexts_share_identity_and_revisions | Couvert (noyau) | — |
| CA-08 | 20 | Après arrêt et réouverture, retrouver objectif, état du code, prochaine action | INV-DUR-01, INV-RESUME-01, INV-RESUME-02 | — | a_killed_process_leaves_a_whole_state, reopening_gives_back_exactly_the_repository, resume_points_are_stored_like_in_memory (memoire-store) ; a_resume_point_publishes_a_commit_without_changing_the_network, code_states_are_compared (6.5a) ; a_resume_point_is_noted_with_the_state_of_the_code, the_resume_point_says_what_changed_since (memoire-cli) ; test_la_fiche_de_reprise_s_affiche_a_l_ouverture (interface) | Couvert | Fiche de reprise livrée (6.5, FICHE-REPRISE.md, D26) : objectif, question et prochaine action en tête de `reprise voir`, changements du code depuis signalés ; la cible d’usage « moins de deux minutes » relève de l’essai par l’utilisateur |
| CA-09 | 21 | Export puis réimport sans perte d'identifiants, annotations, relations | INV-EXPORT-01, INV-ID-03, INV-MIGR-01, INV-MIGR-02 | S-ID-01 | every_record_and_value_round_trips, the_reference_example_is_frozen_in_both_directions, the_repository_example_is_frozen_in_both_directions, a_repository_using_every_command_round_trips, reopening_gives_back_exactly_the_repository (memoire-store), export_then_import_gives_back_the_repository (memoire-app) ; version 2 et migration (5.1) : the_v2_repository_example_is_frozen_and_is_the_migration_of_v1, a_repository_using_every_command_migrates_to_v2, a_v1_export_can_still_be_imported (memoire-app), import_reads_both_format_versions (memoire-store) ; migration des bases de schéma 1 (5.2d) : a_base_written_by_the_previous_version_migrates_to_its_own_export, schema_1_bases_migrate_to_the_same_repository, a_refused_migration_leaves_the_source_intact, opening_a_base_of_another_schema_writes_nothing, a_failed_write_leaves_no_file (memoire-store), the_service_migrates_a_base_in_place, a_refused_migration_leaves_the_base_intact (memoire-app), migration_of_a_schema_1_base, migration_refusals (memoire-cli) ; branches abandonnées (5.4a, INV-BR-05) : abandon_blocks_are_written_as_specified_and_round_trip, abandon_rules_are_enforced_when_reading, abandoned_branches_are_imported_and_checked (memoire-store) | Couvert (noyau) | Commande d’export et d’import : CLI, phase 4 |
| CA-10 | 22 | Fonctions essentielles sans IA externe | — | — | — | Couvert (noyau) | Par construction : aucune dépendance ni appel réseau |
| CA-11 | 23 | Placer en approfondissement sans modifier la validité du précédent | INV-SEM-01, INV-REL-05 | S-REF-01 ét. 1 et 2 | invalidating_a_subidea… | Partiel | Relation « approfondit » non modélisée |
| CA-12 | 24 | Corriger un passage sans invalider le reste ni les échanges suivants | INV-SOURCE-01, INV-REL-05, INV-REL-06 | S-REF-01 ét. 3 | passages_designate_exact_ranges_and_can_be_corrected, restriction_adds_a_condition_and_requires_review_of_dependents | Couvert (noyau) | — |
| CA-13 | 25 | Réponse collée sans question : état incomplet visible | INV-SOURCE-04 | S-SRC-01 | exchanges_keep_sources_byte_for_byte_with_explicit_boundaries | Couvert (noyau) | — |
| CA-14 | 26 | Annuler une correction ; conserver les alertes d'autres causes | INV-CAUSE-01, INV-UNDO-01 | — | review_keeps_each_independent_cause, removing_one_old_dependency…, detaching_a_relation_keeps_history_and_restores_prior_states | Couvert (noyau) | — |
| CA-15 | 27 | Rattacher un échange à deux échanges sans dupliquer | INV-HIST-02, INV-REL-03 | S-REF-01 ét. 2 | review_keeps_each_independent_cause, category_hierarchy_is_acyclic | Partiel | Échanges absents ; seul le rattachement multiple de notes est exercé |
| CA-16 | 28 | Créer une réserve sans note de destination | INV-RES-01, INV-RES-04 | S-REF-01 ét. 5, S-RES-01 | reserves_follow_an_explicit_lifecycle_per_scope, reference_scenario_steps_1_and_5_on_real_sources | Couvert (noyau) | — |
| CA-17 | 29 | Rattacher ultérieurement la réserve, navigation bidirectionnelle | INV-RES-01, INV-RES-03 | S-RES-01 | reserves_follow_an_explicit_lifecycle_per_scope | Couvert (noyau) | — |
| CA-18 | 30 | Lier sans clôturer ; distinguer pertinence, traitement, résolution | INV-RES-02, INV-RES-03 | S-REF-01 ét. 5, S-RES-01 | reserves_follow_an_explicit_lifecycle_per_scope, reserve_transitions_require_a_reason | Couvert (noyau) | — |
| CA-19 | 31 | Résoudre une réserve pour une variante seulement | INV-RES-05 | S-RES-01 | reserves_follow_an_explicit_lifecycle_per_scope | Couvert (noyau) | — |
| CA-20 | 32 | Après abandon, retrouver les notes développées avec leurs relations | INV-HIST-01, INV-GRAPH-01, INV-RET-02 | S-REF-01 ét. 8 | archival_preserves_verdict_and_relations, no_longer_retained_is_reversible_and_requires_review | Couvert (noyau) | Vue de réexamen non construite |
| CA-21 | 33 | Réattacher une note ou un groupe sans duplication ni perte | INV-HIST-02, INV-ID-01, INV-LOT-01 | — | reattachment_preserves_old_edge…, reattachment_preserves_identity… ; a_lot_creates_and_links_in_one_operation, a_refused_element_refuses_the_whole_lot (5.3) | Partiel | Groupe : le noyau accepte un lot atomique de réattachements (`Command::Lot`, incrément 5.3, D22), qui n’est pas encore exposé par la ligne de commande |
| CA-22 | 34 | Retirer des ressources actuelles et retrouver dans l'historique | INV-HIST-01, INV-RET-01 | S-REF-01 ét. 8 | archival_preserves_verdict_and_relations, every_accepted_change_keeps_all_identities, no_longer_retained_is_reversible_and_requires_review | Couvert (noyau) | — |
| CA-23 | 35 | Retirer un rattachement dans une variante sans toucher les autres | INV-SCOPE-01, INV-SCOPE-02 | S-SCOPE-01 | scopes_are_isolated, contexts_share_identity_and_revisions, detaching_a_relation_keeps_history_and_restores_prior_states | Couvert (noyau) | — |
| CA-24 | 36 | Adapter une note en gardant l'original et le lien vers la révision | INV-MOD-01 | S-MOD-01 | edit_changes_text_only, revisions_are_kept_with_their_kind | Couvert (noyau) | — |
| CA-25 | 37 | Après annulation ou reconstitution, retrouver les relations alors actives | INV-UNDO-01, INV-RET-03 | S-RET-02 | undo_redo_roundtrip…, restoration_itself_is_undoable, restore_keeps_later_objects_retired_and_resolvable | Couvert (noyau) | — |
| CA-26 | 38 | Annuler en une action une invalidation et tous ses effets, puis rétablir | INV-UNDO-01, INV-BR-03 | S-REF-01 ét. 9 | undo_redo_roundtrip_invalidation_and_all_derived_effects, undo_is_bounded_by_the_branch_base_and_returns_to_origin | Couvert (noyau) | — |
| CA-27 | 39 | Après réouverture, restaurer une idée sans revenir sur le reste | INV-RESTORE-01, INV-DUR-01 | S-RET-02 | a_targeted_restore_brings_back_an_idea_with_its_links_only, a_targeted_restore_replaces_later_versions_and_says_so (6.4a) ; a_targeted_restore_after_reopening_is_the_same_as_in_memory (memoire-store), restoring_an_idea_without_the_rest, the_restore_plan_says_how_internal_relations_change (memoire-cli) | Couvert | Restauration ciblée livrée (6.4, RESTAURATION.md) ; réouverture de la base vérifiée (memoire-store) |
| CA-28 | 40 | Retrouver un état abandonné après annulation puis nouveau travail | INV-UNDO-02 | — | discarded_redo_remains_addressable_in_history | Couvert (noyau) | — |
| CA-29 | 41 | Invalider une structure de données sans invalider scheduler ni catégorie | INV-SEM-01 | — | invalidating_a_subidea_keeps_architecture_category_and_sibling | Couvert (noyau) | — |
| CA-30 | 42 | Retrouver une idée par son identifiant en commentaire après déplacement | INV-ID-01, INV-CODE-05 | S-CODE-02 | marker_survives_edit_undo_restore, markers_are_distinct…, markers_carry_only_the_identity… ; the_code_report_classifies_every_anchor_and_marker, indexing_reports_the_state_of_every_marker_and_anchor (6.3) | Couvert | Détection des occurrences : indexeur (6.3) ; un fichier renommé est signalé « déplacé » |
| CA-31 | 43 | Lister plusieurs occurrences ; signaler repères inconnus ou disparus | INV-CODE-01 | S-CODE-02 | markers_are_found_whatever_the_comment_syntax, the_code_report_classifies_every_anchor_and_marker (6.3a), indexing_reports_the_state_of_every_marker_and_anchor, indexing_skips_what_it_cannot_read (6.3b) | Couvert | Indexeur livré (6.3, INDEXEUR.md) : commande `indexer DOSSIER` |
| CA-32 | 44 | Restaurer le réseau sans écraser le code ; afficher les ancrages incompatibles | INV-RESTORE-01, INV-CODE-04 | S-CODE-03 | restore_keeps_later_anchors_inactive, tests/architecture.rs | Partiel | Affichage des ancrages incompatibles : indexeur, phase 5 |
| CA-33 | 45 | Annuler une restauration | INV-UNDO-01 | S-RET-02 | restoration_itself_is_undoable | Couvert (noyau) | — |
| CA-34 | 46 | Modification critique : branche créée avant application, origine intacte | INV-BR-01, INV-BR-02, INV-TX-01 | S-BR-01, S-REF-01 ét. 8 | critical_change_forks_before_publish, publication_counts_commits_and_branches, every_outcome_reports_commit_branch_and_fork, critical_changes_on_an_experimental_branch_stay_on_it, criticality_follows_the_actual_effect | Couvert (noyau) | — |
| CA-35 | 47 | Revenir à main, rouvrir la branche expérimentale et retrouver son état | INV-SCOPE-01, INV-BR-03 | S-BR-01 | critical_change_forks_before_publish, undo_is_bounded_by_the_branch_base_and_returns_to_origin | Couvert (noyau) | — |
| CA-36 | 48 | Aucune sauvegarde ni fermeture ne fusionne vers main | INV-MERGE-01 | — | merging_a_branch_follows_its_plan_after_a_backup (memoire-cli : la fusion n'a lieu que par « fusionner » avec le code du plan, la sauvegarde ne fusionne rien) | Partiel | Seule « fusionner » publie un commit à deux parents (Command::Merge) ; aucune autre commande ne fusionne, par construction |
| CA-37 | 49 | Fusionner deux branches compatibles ; présenter les conflits | INV-MERGE-02, INV-MERGE-03 | — | contrat de 5.4b (noyau) : independent_changes_are_merged_in_one_undoable_commit, merging_into_an_unchanged_origin_gives_the_branch_state, conflicts_are_reported_and_resolved_by_side, malformed_resolutions_are_refused, acknowledgement_conflicts_are_resolved_by_side, structural_conflicts_are_resolved_by_deactivation, several_merge_bases_are_refused, merges_are_refused_on_a_wrong_or_stale_source, merge_blocks_are_written_as_specified_and_round_trip, malformed_merge_blocks_are_refused, a_merge_clears_the_redo_stack_of_the_target ; merges_are_stored_like_in_memory (memoire-store) ; merging_a_branch_follows_its_plan_after_a_backup, merge_conflicts_are_resolved_by_their_numbers, structural_conflicts_and_merge_refusals (memoire-cli) | Couvert | Fusion livrée en 5.4b et 5.4c (FUSION.md, D19, D23) ; l’interface la montre par les formulaires générés de l’aide |
| CA-38 | 50 | Déclarer une note liée au code, copier le repère, la retrouver après détection | INV-ID-01, INV-CODE-02, INV-CODE-03 | S-REF-01 ét. 6 | markers_are_distinct…, markers_carry_only_the_identity…, anchors_keep_path_copy_and_digest, reference_scenario_steps_1_to_9_on_real_sources | Partiel | Détection du repère collé : indexeur, phase 5 |
| CA-39 | 51 | Modifier, réattacher, fusionner sans changer l'identifiant du commentaire | INV-ID-01, INV-MOD-01 | S-MOD-01 | marker_survives_edit_undo_restore, reattachment_preserves_identity…, revisions_are_kept_with_their_kind | Partiel | Fusion absente |
| CA-40 | 52 | Conserver une modification ordinaire lors de la création de la branche critique | INV-BR-01 | — | ordinary_change_is_carried_into_the_critical_branch | Couvert (noyau) | — |

## Synthèse

| Statut | Nombre |
|---|---|
| Couvert (noyau) | 27 |
| Partiel | 8 |
| Non couvert | 5 |
| Total | 40 |

## Couverture par les scénarios

31 critères sur 40 sont rattachés à au moins un scénario de référence. Les 9 autres relèvent de la persistance (CA-08), de la fusion (CA-36, CA-37), d’une propriété acquise par construction (CA-10), ou s’appuient sur des tests du contrat existants (CA-14, CA-21, CA-28, CA-29, CA-40). CA-14 et CA-21 restent partiels : leurs écarts (relation de correction, commande collective) demanderont un scénario en phase 2.

Tests de contrat de 5.4a sans critère du cahier des charges : l’abandon d’une branche vient de D19 (FUSION.md, section 5) et non du cahier ; ils sont rattachés à INV-BR-05 (INVARIANTS.md) : abandoning_a_branch_keeps_it_and_reactivation_restores_it, abandoning_the_active_branch_returns_to_its_origin, an_abandoned_branch_cannot_become_active_until_reactivated, meaningless_abandons_and_reactivations_are_refused, notes_are_not_looked_for_in_abandoned_branches ; abandon_and_reactivation_are_stored_like_in_memory (memoire-store) ; abandoning_and_reactivating_a_branch, abandon_refusals (memoire-cli).

Tests de contrat de 5.5a (CITATIONS.md, D18, D24) : exchanges_take_part_in_dependencies (INV-REL-04, D18 ; R10), a_note_is_cited_at_a_version et note_citation_blocks_round_trip (INV-CIT-01 ; R8), a_citation_and_its_link_follow_their_source (INV-CIT-02), orphans_are_notes_whose_supports_all_fell (INV-ORPH-01 ; R6). 5.5b : note_citations_and_exchange_links_are_stored_like_in_memory (memoire-store), citing_a_note_or_an_exchange_links_the_citation_to_its_source, exchanges_in_dependencies_withdrawal_and_orphans (memoire-cli).

Tests de contrat de 6.1a (VISIONNEUR.md, D25 ; R2, R8 ; INV-VIEW-01) : source_prints_the_exact_text, citations_list_the_cited_ranges_of_a_source, citing_by_byte_range_and_version (memoire-cli).

Tests de contrat de 6.2a (CENTRE.md ; R2 ; INV-WORK-01, INV-LOT-01, INV-CIT-02) : citing_and_linking_in_one_gesture, citing_and_linking_refusals (memoire-cli).

Tests de contrat de 6.4a (RESTAURATION.md ; INV-RESTORE-01, INV-RESTORE-02) : the_branch_history_lists_commits_newest_first (historique, choix de l’état à restaurer), invalid_targeted_restores_are_refused, targeted_restores_are_critical_and_full_restores_have_a_plan, targeted_restore_blocks_round_trip.

Tests de contrat de 6.5a (FICHE-REPRISE.md, D26 ; INV-RESUME-01, INV-RESUME-02) : resume_points_are_listed_per_branch_newest_first (fiches d’une branche, opérations depuis), invalid_resume_points_are_refused, resume_blocks_round_trip, resume_points_are_refused_in_v1 ; propriétés : random_sequences_with_resume_points_respect_the_kernel_laws ; ajoutés après la mutation : code_state_accessors_give_back_what_was_noted, a_patch_of_exactly_the_limit_is_kept_and_read_back.

Tests de contrat de 6.5b (FICHE-REPRISE.md, D26 ; INV-RESUME-01, INV-RESUME-02 ; CA-08) : resume_points_are_stored_like_in_memory (memoire-store) ; a_resume_point_is_noted_with_the_state_of_the_code, the_resume_point_says_what_changed_since, resume_points_are_listed_per_branch, resume_points_describe_every_state_of_the_code, resume_refusals (memoire-cli) ; test_champs_de_la_fiche_de_reprise, test_la_fiche_de_reprise_s_affiche_a_l_ouverture (interface) ; ajoutés après la mutation : git_absent_or_failing_is_reported, unborn_heads_and_the_base_inside_the_repository (memoire-cli), status_lines_keep_the_old_path_of_a_rename, dates_at_the_minute (tests internes).

Tests de contrat de 6.6a (VOISINAGE.md ; R7 ; INV-VIEW-02) : the_neighborhood_classifies_each_link_by_origin ; the_neighborhood_is_printed_by_origin, the_neighborhood_has_a_tab_separated_form (memoire-cli).

Tests de contrat de 6.6b (VOISINAGE.md, section 4 ; INV-VIEW-02) : gui/test_voisinage.py (LectureDuVoisinage, Disposition ; test_un_trait_s_arrete_au_bord_d_une_etiquette ajouté avec le code) ; gui/test_gui.py (EssaiDeLaVueLocale).

Tests de contrat de 6.7a (TERMINAL.md, D27 ; R5 ; INV-TERM-01, INV-TERM-02) : gui/test_terminal_moteur.py (LeTampon, LHistorique, LeSuivi, LEnvironnementEtLaGeometrie, UneSessionReelle), gui/test_notification.py (LeTexte, LEnvoi).

Tests de contrat de 6.7b (PLAN-DE-TRAVAIL.md, D28 ; INV-PLAN-01) : gui/test_plan_de_travail.py (LesOperations, LaConservation, LeFichier).

Tests de contrat de 6.7c (TERMINAL.md, PLAN-DE-TRAVAIL.md ; D27, D28 ; INV-TERM-01, INV-PLAN-01) : gui/test_gui.py (EssaiDuPlanDeTravail).

Tests de contrat de 6.8 (PLAN-DE-TRAVAIL.md, section 10 ; D28) : gui/test_gui.py (EssaiDuPlanDeTravail.test_l_utilisateur_decide_de_l_ordre_d_une_fenetre_flottante).

Tests de contrat de 6.9a (TERMINAUX.md, D29 ; INV-TERM-03) : gui/test_terminaux.py (LeNommage, LeRegistre) ; gui/test_terminal_moteur.py (LeDossierDansLaMarque, UneSessionReelle.test_le_dossier_suit_cd) ; gui/test_plan_de_travail.py (LesPanneauxDynamiques).

Tests de contrat de 6.9b (TERMINAUX.md, D29 ; INV-TERM-03, INV-TERM-01) : gui/test_gui.py (EssaiDePlusieursTerminaux).

Tests de contrat de 7.1a (ACTIONS.md, D30 ; INV-UI-01) : gui/test_actions.py (LeCatalogue, LaCoherenceAvecLaLigneDeCommande, LesCommandesProduites, LeVocabulaire).

Tests de contrat de 7.1b (ACTIONS.md, section 7 ; D30 ; INV-UI-01) : branches_have_a_readable_name, the_merge_plan_has_a_tab_separated_form (memoire-cli).

Tests de contrat de 7.1c (ACTIONS.md, D30 ; INV-UI-01) : gui/test_gui.py (EssaiDuPanneauDesActions).

Tests de contrat de 7.2a (RETROUVER.md ; INV-CODE-07) : the_notes_of_a_code_are_found_from_a_file_a_line_or_an_excerpt ; the_notes_of_a_code_file, the_notes_of_a_code_file_near_paths_and_refusals, a_marker_is_found_whatever_the_comment_syntax (memoire-cli).

Tests de contrat de 8.1a (APPARENCE.md, D31 ; INV-LOOK-01, INV-LOOK-02) : gui/test_dessin.py (LeFormatPng, LeDeterminisme, LaForme, LeDegradeDeFond, LesIcones) ; gui/test_apparence.py (LesContrastes, LesPolices, LeNomDuProjet, LEtatDuProjet, LaRecherche, LeRail).

Tests de contrat de 8.1b (APPARENCE.md, section 3 ; D31 ; INV-UI-01) : gui/test_gui.py (EssaiDeLHabillage ; test_le_menu_par_sujet_et_le_mode_expert, contrat modifié : le rail et la liste remplacent l’arbre).

Tests de contrat de 8.1c (APPARENCE.md, sections 3 et 5) : gui/test_gui.py (test_le_survol_montre_son_fond_et_ne_s_accumule_pas, test_le_journal_se_redimensionne_et_s_agrandit, test_les_images_dessinees_sont_opaques, test_une_grande_forme_etiree_est_celle_du_calcul_complet).

Tests de contrat de 8.2 (APPARENCE.md, section 9 ; INV-UI-01, INV-LOOK-02, INV-VIEW-01, INV-WORK-01) : gui/test_gui.py (EssaiDesMenus, EssaiDeLApercu, EssaiDeLaLecture ; test_le_bouton_voir_ouvre_le_visionneur_de_la_note_du_champ, contrat modifié : lecture dans la carte) ; gui/test_actions.py (LApercuPrevu) ; gui/test_voisinage.py (LeTitreDuCentre).

Tests de contrat de 8.3 (APPARENCE.md, section 10) : gui/test_gui.py (test_les_deux_vues_et_leurs_panneaux, test_la_carte_du_voisinage_et_la_signature ; test_la_barre_de_selection_relie_et_cite_en_un_geste, contrat modifié) ; gui/test_voisinage.py (LaDispositionAuxCoins).

Tests de contrat de 8.4 (APPARENCE.md, section 11 ; INV-TERM-01, INV-TERM-03) : gui/test_terminal_moteur.py (LaCompletion) ; gui/test_gui.py (EssaiDesOngletsDuTerminal).

Tests de contrat de 8.5 (CONSOLE.md, D32 ; R21 ; INV-TERM-04, INV-TERM-01, INV-TERM-03, INV-PLAN-01) : gui/test_mosaique.py (LesDispositions, LesOperations, LePointMilieu, LaPlace, LaConservation) ; gui/test_plan_de_travail.py (LaPleinePage) ; gui/test_terminaux.py (LaConsoleDansLeRegistre, LesNomsDansLaConsole) ; gui/test_gui.py (EssaiDeLaConsole ; EssaiDuPlanDeTravail, EssaiDePlusieursTerminaux et test_le_bouton_du_bandeau_ouvre_son_menu, contrat modifié : la console remplace un panneau par terminal ; EssaiDesOngletsDuTerminal remplacé par EssaiDeLaCompletion, les onglets n’existent plus).

Tests de contrat de 8.5b (CONSOLE.md, section 11 ; INV-TERM-04, INV-LOOK-01) : gui/test_gui.py (EssaiDeLaConsole : test_aucun_fond_de_papier_sous_les_widgets_d_une_tuile, test_tirer_le_point_ne_refait_que_la_mosaique, test_la_page_vide_propose_un_terminal, test_le_bouton_de_l_en_tete_dit_si_la_console_est_montree ; test_la_mosaique_de_un_a_quatre_terminaux, contrat modifié : le liseré est la bordure d’un cadre classique ; EssaiDeLHabillage : test_les_grandes_surfaces_ne_sont_pas_pavees).

Tests de contrat de 7.2b (RETROUVER.md, section 1 et 2 ; INV-CODE-07) : test_schema (test_notes_affichees_par_leur_nom) ; test_gui (test_voir_une_note_ouvre_la_recherche_par_nom, test_suggestions_pendant_la_frappe_et_note_affichee_par_son_nom, test_quelles_notes_concernent_ce_code).

## Règle de maintenance

Toute modification d'un test de contrat, d'un invariant ou d'un critère met cette matrice à jour dans le même commit. Un test sans invariant ni critère de rattachement doit être justifié ou supprimé.
