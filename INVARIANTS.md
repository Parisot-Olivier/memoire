# Invariants et architecture Rust — document technique exploratoire

Version 0.3 — 21 octobre 2026 — fin de la phase 2 (incréments 2.1 à 2.7) : crate memoire-domain complète pour le modèle v0.2, contrat porté et étendu, tests de propriétés, prototype memoire-core retiré.

Phase 5, incrément 5.3 : opérations groupées (LOT.md, D22) : commande `lot` du noyau, tout ou rien, un seul commit et un seul `annuler`, criticité sur l’effet net ; `note creer` avec liens (R1) ; autocomplétion à plusieurs notes. Phase 5, incrément 5.2d : migration des bases de schéma 1 (`memoire migrer`, question de l’interface) : sauvegarde, conversion dans un nouveau fichier, vérification complète, remplacement atomique ; essayée sur la base réelle de l’utilisateur, dont l’export est resté identique à celui de v0.5.0-alpha.1. Phase 5, incrément 5.2c : schéma 2 (ETAT-HISTORIQUE.md, D20, D21) : état courant de chaque branche et historique par différences, état de référence tous les cent commits ; lectures et écritures indépendantes de la longueur de l’historique ; vérification complète sur demande (`memoire verifier`). Phase 5, incrément 5.2b : historique derrière un trait (ETAT-HISTORIQUE.md, D20) : le dépôt ne garde en mémoire que la branche active, les branches et l’état de la tête active ; écritures préparées puis appliquées tout ou rien ; équivalence vérifiée avec l’oracle de v0.5.0-alpha.1. Phase 5, incrément 5.1 : format textuel version 2 (FORMAT.md, section 13 ; VERSION2.md) : instantané avec citations et rangs de révision, dépôt par différences avec un état de référence tous les cent commits, migration contrôlée de la version 1, export en version 2 et import des deux versions. Phase 4, incréments 4.1 à 4.4 : recherche par nom, autocomplétion et sauvegarde manuelle (R3, R9) ; interface graphique minimale (INTERFACE.md, D17) ; ligne de commande memoire (CLI.md) ; S-REF-01 rejoué en entier par la ligne de commande. Phase 3, incréments 3.1 à 3.4 : format textuel des instantanés et du dépôt complet (FORMAT.md, D15), stockage SQLite (STOCKAGE.md, crate memoire-store), couche service avec export et import (SERVICE.md, crate memoire-app).
Périmètre : cœur du logiciel de mémoire de conception.
Ce document est distinct du cahier des charges fonctionnel, versionné dans le même dépôt (CAHIER_DES_CHARGES.md, version 0.2 depuis le 21 octobre 2026).
Les invariants ci-dessous sont des propositions techniques à discuter. Le code livré est un prototype exécutable en mémoire, pas l’application finale.

## 1. Faisabilité et objectif

Le projet est réalisable en Rust. Sa difficulté principale est la cohérence des états, des relations et de leur historique : le langage ne peut pas décider si un raisonnement est pertinent.

Le noyau applique les annotations structurées de l’utilisateur. Il ne dépend ni d’un modèle d’IA, ni d’un éditeur particulier. Rust permet de distinguer les types d’identifiants, d’encadrer les mutations et de retourner explicitement les erreurs. Les règles de conception restent à exprimer et à tester.

« Invariant du logiciel » désigne ici une propriété de ce noyau. Un invariant de RTOS mentionné dans une note constitue une donnée utilisateur ; le noyau ne prouve pas sa validité technique.

## 2. Objets et frontières

| Objet | Identité | État pouvant évoluer |
| --- | --- | --- |
| Note : catégorie, raisonnement, idée d’implémentation, note personnelle | UUID stable à travers branches et contextes ; rôle immuable | Révisions ; statuts et visibilité par contexte |
| Échange | UUID ; question et réponse délimitées explicitement | Complet ou incomplet ; classements et relations |
| Source | Texte collé, conservé octet pour octet | Aucun changement après création |
| Passage | Source, plage d’octets, empreinte | Relations et annotations |
| Révision | Note et numéro ; type mineur ou substantiel | Aucun changement après création |
| Réserve | UUID ; passage et échange d’origine | Statut par contexte : ouverte, traitée à vérifier, résolue |
| Relation | UUID ; origine, destination et type immuables | Activation par contexte, historisée avec motif |
| Acquittement | Dépendant et sa révision, cause et son empreinte, contexte | Aucun changement ; devient caduc si une empreinte change |
| Instantané | État immuable du réseau | Aucun changement après publication |
| Commit | Identifiant et parent(s) | Aucun changement de contenu |
| Branche | Identifiant stable ; origine et commit de base | Référence vers un commit courant |
| Contexte | Variante réellement suivie en parallèle | Appartenance, statuts, activation des relations |
| Repère de code | UUID de la note | Ancrages : chemin, copie et empreinte du code, commit Git éventuel ; état de détection |

Les relations peuvent viser une note, un échange ou un passage. Les types prévus en v0.2 : classé dans, dépend de, développé à partir de, justifie, contredit, remplace, restreint le périmètre de, cite, s’applique à (réserve), traite (réserve), lié au code, même sujet.

Une branche est une trajectoire d’exploration. Un contexte représente une variante, par exemple monocœur ou multicœur. Ces deux dimensions sont distinctes : une branche peut contenir plusieurs contextes.

Les catégories organisent ; les raisonnements argumentent ; les idées d’implémentation proposent une réalisation. Une appartenance à une catégorie ne crée pas de dépendance logique. Les catégories peuvent s’imbriquer, par exemple « Exploration / évolution » puis « Exploration multicœur » (D7) ; un statut ne s’exprime jamais par une catégorie.

## 3. Registre initial des invariants

Statuts : **T** = propriété couverte, dans le périmètre limité du prototype, par des tests du contrat dont l’efficacité est contrôlée par mutation (aucun mutant survivant, voir section 9) ; **T\*** = couverte, mais le contrat lui-même sera redéfini par une décision arrêtée (section 3 bis) ; **P** = exigence identifiée, à implémenter et à tester. « T » ne signifie pas preuve générale.

| Identifiant | Propriété à préserver | Statut et mécanisme |
| --- | --- | --- |
| INV-ID-01 | Modifier, réattacher ou restaurer une note ne change pas son identité ni son repère de code ; deux notes distinctes ont deux repères distincts dans un même dépôt | T — identifiants séparés du contenu. Réattachement et unicité testés depuis la v0.1.1. Collision entre dépôts : défaut S7, décision 3 |
| INV-ID-02 | Un identifiant abandonné après annulation n’est jamais réattribué dans le même dépôt | T — allocation monotone non annulée |
| INV-ID-03 | Importer deux objets distincts ne les fusionne pas sur collision d’identifiants | P — espace de projet et contrôle d’import |
| INV-REF-01 | Toute relation, même inactive, référence des objets existants dans son contexte | T — validation des extrémités, testée isolément (tests internes). Les relations inactives sont aussi jugées selon les règles courantes : à réexaminer avec INV-MOD-02 |
| INV-SEM-01 | Catégorie et proximité ne propagent aucune réfutation ; une catégorie ne porte pas de verdict technique | T — types de relations et API contrôlée |
| INV-SEM-02 | Un dépendant à réexaminer ne devient pas automatiquement faux | T — verdict et causes de réexamen séparés |
| INV-CAUSE-01 | Chaque cause indépendante de réexamen est conservée ; retirer une cause ne masque pas les autres | T — ensemble de racines calculé |
| INV-SCOPE-01 | Modifier une branche ou un contexte n’altère pas les autres états | T — instantanés et contexte explicitement ciblé ; contextes superposés depuis l’incrément 2.3a (D2, INV-SCOPE-02) : le contenu est partagé, la qualification reste propre au contexte |
| INV-BR-01 | Une modification critique engagée sur main est publiée dans une nouvelle branche ; l’origine reste inchangée | T — redéfini par D1 et mis en œuvre à l’incrément 2.2 ; publication après validation du candidat |
| INV-TX-01 | Une commande refusée ne modifie ni l’état, ni les relations, ni les références de branche, ni les compteurs | T — préparer puis publier. Depuis la v0.1.1 : égalité complète du dépôt vérifiée pour les 19 refus du domaine et pour un candidat invalide soumis directement à la publication |
| INV-HIST-01 | Retirer des ressources actuelles ne supprime pas l’objet et ne signifie pas le réfuter | T — visibilité distincte du verdict |
| INV-HIST-02 | Réattacher conserve l’ancien lien et les relations internes non concernées | T — ancien lien inactivé, nouveau lien ajouté |
| INV-UNDO-01 | Annuler une opération retrouve l’état métier précédent, avec tous ses effets calculés | T — déplacement de tête vers le parent ; depuis l’incrément 2.2, l’annulation de l’opération fondatrice d’une branche ramène sur son origine (INV-BR-03) |
| INV-UNDO-02 | Une nouvelle action après annulation ne supprime pas les états abandonnés | T — commits conservés, même si la pile redo est effacée |
| INV-GRAPH-01 | Le calcul du réexamen termine sur un réseau fini, même cyclique, et suit uniquement les relations autorisées | T — sommets visités et oracle indépendant |
| INV-DET-01 | Même état et mêmes commandes produisent le même état métier dans ce modèle | T — transitions déterministes, ordre explicite. Test faible : aucun aléa dans le modèle actuel ; il deviendra significatif avec les UUID injectés (décision 3) |
| INV-SOURCE-01 | Un passage désigne exactement un texte source immuable et une plage UTF-8 valide | T — constructeur de passage contrôlé, exemple isolé |
| INV-ACK-01 | Un acquittement porte sur la révision du dépendant, l’empreinte de la cause (dernière révision substantielle et statuts) et le contexte ; toute variation de l’un d’eux rouvre le réexamen ; l’acquittement ne supprime aucune relation | P — précisé en v0.2 ; S-REF-01 étape 9 ; défaut S8 |
| INV-SOURCE-02 | Les surlignages restent liés à la source originale après changement du rendu Markdown | T (6.1, D25) : plages en octets de la source, correspondance tenue par gui/rendu.py ; gui/test_rendu.py (Plages), test_une_note_se_lit_rendue_ou_brute_et_se_cite_par_selection (surlignage identique en rendu et en brut) |
| INV-MERGE-01 | Aucune sauvegarde, fermeture ou navigation ne fusionne implicitement vers main | T (5.4c) : seule `Command::Merge` publie un commit à deux parents, et seule la commande « fusionner », avec le code du plan, la construit ; merging_a_branch_follows_its_plan_after_a_backup (memoire-cli) |
| INV-MERGE-02 | Une fusion compare base commune, destination et source ; aucun conflit explicite n’est écrasé silencieusement | T (5.4b, 5.4c ; D19, D23) : independent_changes_are_merged_in_one_undoable_commit, merging_into_an_unchanged_origin_gives_the_branch_state, conflicts_are_reported_and_resolved_by_side, malformed_resolutions_are_refused, acknowledgement_conflicts_are_resolved_by_side, several_merge_bases_are_refused, merges_are_refused_on_a_wrong_or_stale_source, merge_blocks_are_written_as_specified_and_round_trip, malformed_merge_blocks_are_refused, a_merge_clears_the_redo_stack_of_the_target ; tests internes de fusion.rs (relations, ancrages, acquittements, contextes d’un seul côté) ; propriétés avec fusions ; merges_are_stored_like_in_memory, propriétés avec fusions (memoire-store) ; merging_a_branch_follows_its_plan_after_a_backup, merge_conflicts_are_resolved_by_their_numbers, structural_conflicts_and_merge_refusals (memoire-cli) |
| INV-MERGE-03 | Le résultat fusionné est revalidé, même si chaque branche était valide séparément | T (5.4b, 5.4c) : structural_conflicts_are_resolved_by_deactivation, structural_candidates_are_the_relations_changed_since_the_base (tests internes) ; propriétés avec fusions (tout état publié est valide) ; structural_conflicts_and_merge_refusals (memoire-cli) |
| INV-ID-04 | T de bout en bout : identités attribuées par la couche service (UUID v4 en usage, suite déterministe en test), jamais nulles ni répétées ; D3 appliquée | identities_are_never_nil_nor_repeated, a_plan_reports_every_assigned_identity (memoire-app) | 3.3 |
| INV-DUR-01 | Après interruption, le dépôt correspond à l’état avant ou après une transaction, jamais à un mélange | P — stockage transactionnel et tests d’arrêt |
| INV-RETRY-01 | Réexécuter une même opération identifiée n’applique pas deux fois ses effets | P — identifiant d’opération stocké avec le résultat |
| INV-DUR-01 | T pour l’arrêt brutal d’un processus : six processus tués pendant leurs écritures, fichier rouvert égal à un préfixe exact de la suite de référence ; coupure d’alimentation non simulée (garanties de SQLite avec synchronous=FULL) | a_killed_process_leaves_a_whole_state (memoire-store) | 3.2 |
| INV-RETRY-01 | T, durable : une enveloppe rejouée après réouverture rend le même résultat sans aucune écriture | a_retried_envelope_has_no_second_effect_after_reopening (memoire-store) | 3.2 |
| INV-EXPORT-01 | T : instantané et dépôt complet relus à l’identique (domaine) ; export atomique vers un fichier texte, deux exports identiques à l’octet, import dans une nouvelle base égal au dépôt exporté, journal compris (service) ; version 2 (5.1) : mêmes garanties, migration de la v1 contrôlée par relecture et comparaison de chaque état, export en v2, import des deux versions | the_reference_example_is_frozen_in_both_directions, every_record_and_value_round_trips, the_repository_example_is_frozen_in_both_directions, a_repository_using_every_command_round_trips, repository_consistency_rules_are_enforced, the_v2_snapshot_example_is_frozen_in_both_directions, every_record_round_trips_in_v2, v2_snapshot_deviations_are_refused, the_v2_repository_example_is_frozen_and_is_the_migration_of_v1, a_repository_using_every_command_migrates_to_v2, references_every_hundred_commits, reference_placement_is_enforced, v2_repository_deviations_are_refused, errors_inside_v2_commit_blocks_are_located ; tests/proprietes.rs ; export_then_import_gives_back_the_repository, exports_are_deterministic_and_replace_atomically, import_refuses_existing_bases_and_invalid_texts, a_v1_export_can_still_be_imported (memoire-app) ; import_reads_both_format_versions (memoire-store) | 3.1, 3.1b, 3.4, 5.1 |
| INV-MIGR-01 | Une base de schéma 1 migre au schéma 2 en redonnant exactement le même dépôt, vérifié en entier avant de remplacer l’original, après une sauvegarde | T (5.2d) : base figée écrite par v0.5.0-alpha.1 migrée vers l’export exact de cette version ; dépôts quelconques comparés commit par commit ; remplacement en place, seconde migration sans effet — a_base_written_by_the_previous_version_migrates_to_its_own_export, schema_1_bases_migrate_to_the_same_repository, bases_are_written_in_wal_mode (memoire-store) ; the_service_migrates_a_base_in_place, the_backup_directory_of_a_migration_is_created (memoire-app) ; migration_of_a_schema_1_base (memoire-cli) ; essai sur la base réelle (VERIFICATION-PHASE5.txt) |
| INV-MIGR-02 | Une migration refusée laisse la base d’origine intacte octet pour octet, sans fichier temporaire ; une base ouverte par un autre processus n’est pas remplacée | T (5.2d) : a_refused_migration_leaves_the_source_intact, migrating_a_wal_base_leaves_no_side_file, the_schema_version_is_read_without_writing, opening_a_base_of_another_schema_writes_nothing, a_failed_write_leaves_no_file (memoire-store) ; a_refused_migration_leaves_the_base_intact (memoire-app) ; migration_refusals (memoire-cli) ; essais de l’interface (jamais automatique) |
| INV-LOT-01 | Un lot est tout ou rien : le refus de n’importe quel élément ne modifie rien ; un lot accepté publie un seul commit et une seule opération, qu’un seul `annuler` défait | T (5.3) : a_lot_creates_and_links_in_one_operation, a_refused_element_refuses_the_whole_lot, the_shape_of_a_lot_is_checked, lot_blocks_are_written_as_specified_and_round_trip, malformed_lot_blocks_are_refused ; propriétés avec lots (même état que les éléments un à un) ; a_lot_is_written_in_one_transaction (memoire-store) ; a_lot_is_planned_and_applied_through_the_service (memoire-app) ; creating_a_note_with_its_links_is_one_operation, malformed_note_lists_are_refused (memoire-cli) |
| INV-LOT-02 | La criticité d’un lot se juge sur son effet net ; un lot critique sur main crée une seule branche | T (5.3) : a_lot_is_judged_on_its_net_effect ; a_critical_note_with_links_needs_confirmation (memoire-cli) |
| INV-LOT-03 | Les identités créées par un lot sont distinctes entre elles et jamais réemployées, même après annulation | T (5.3) : a_refused_element_refuses_the_whole_lot, identities_of_a_lot_are_never_reused, a_lot_is_written_in_one_transaction (memoire-store) |
| INV-BR-05 | Abandonner une branche ne supprime rien : elle garde tête, base, origine, pile de rétablissement et historique, et sa réactivation la rend telle qu’elle était ; main ne s’abandonne pas ; une branche abandonnée n’est ni active ni source d’une fusion tant qu’elle n’est pas réactivée | T (5.4a, D19, D23) : abandoning_a_branch_keeps_it_and_reactivation_restores_it, abandoning_the_active_branch_returns_to_its_origin, an_abandoned_branch_cannot_become_active_until_reactivated, meaningless_abandons_and_reactivations_are_refused, notes_are_not_looked_for_in_abandoned_branches, abandon_blocks_are_written_as_specified_and_round_trip, abandon_rules_are_enforced_when_reading ; propriétés avec abandons ; abandon_and_reactivation_are_stored_like_in_memory, abandoned_branches_are_imported_and_checked, propriétés avec abandons (memoire-store) ; abandoning_and_reactivating_a_branch, abandon_refusals (memoire-cli). « Ni source d’une fusion » : merges_are_refused_on_a_wrong_or_stale_source (5.4b) |
| INV-CIT-01 | Une citation désigne exactement le texte d’une version de sa source (partie d’échange, ou version d’une note) ; une révision ultérieure de la note ne la déplace pas et la fait signaler « source révisée » | T (5.5, D18, D24) : a_note_is_cited_at_a_version, note_citation_blocks_round_trip ; propriétés avec citations ; note_citations_and_exchange_links_are_stored_like_in_memory (memoire-store) ; citing_a_note_or_an_exchange_links_the_citation_to_its_source (memoire-cli) |
| INV-CIT-02 | Une citation créée par `citer` naît avec son lien « développé à partir de » vers sa source, dans la même opération ; ce lien se retire comme toute relation | T (5.5, D24) : a_citation_and_its_link_follow_their_source ; citing_a_note_or_an_exchange_links_the_citation_to_its_source (memoire-cli) |
| INV-ORPH-01 | Une note est orpheline si et seulement si elle a au moins un appui déclaré, que tous sont écartés, qu’elle n’a aucune cause de réexamen en cours et qu’elle n’est pas écartée ; une orpheline n’est pas une ressource actuelle | T (5.5, R6, D24) : orphans_are_notes_whose_supports_all_fell, acknowledgment_lifts_review_without_destroying_provenance, reference_scenario_steps_1_to_9_on_real_sources ; loi des propriétés (check_state) ; exchanges_in_dependencies_withdrawal_and_orphans (memoire-cli) |
| INV-VIEW-01 | Tout caractère affiché par le visionneur provient d’une position de la source ; une sélection se cite comme la plus petite plage de la source qui la contient ; une citation se surligne exactement sur les caractères affichés issus de sa plage | T (6.1, D25), sans mutation pour le Python (limite connue) : source_prints_the_exact_text, citations_list_the_cited_ranges_of_a_source, citing_by_byte_range_and_version (memoire-cli) ; gui/test_rendu.py (Correspondance, SousEnsemble, Plages) ; gui/test_gui.py (EssaiDuVisionneur) |
| INV-VIEW-02 | La vue locale montre exactement les relations actives qui touchent la note dans le contexte, chacune une fois, classée par origine : catégorie (« classée dans »), citation (l’autre extrémité est une citation), note entière (sinon) ; elle ne modifie rien | T (6.6, VOISINAGE.md ; Python sans mutation) : the_neighborhood_classifies_each_link_by_origin ; the_neighborhood_is_printed_by_origin (aucune écriture), the_neighborhood_has_a_tab_separated_form (memoire-cli) ; gui/test_voisinage.py ; test_gui : EssaiDeLaVueLocale |
| INV-TERM-01 | Le shell du terminal n’est jamais coupé par un changement de place du panneau (ancré, flottant, masqué) ; à l’arrêt du logiciel, il ne reste ni processus du shell ni fichier de démarrage | T (6.7, TERMINAL.md, D27 ; Python sans mutation) : UneSessionReelle (test_exit_puis_arret_sans_reste, test_arret_d_un_shell_occupe, test_sessions_oubliees_arretees_a_la_sortie) ; EssaiDuPlanDeTravail (test_detacher_et_rattacher_sans_relancer_le_shell, test_la_disposition_est_conservee_et_le_shell_arrete_a_la_fermeture) ; plusieurs terminaux (6.9) : EssaiDePlusieursTerminaux (test_afficher_ranger_fermer, test_les_terminaux_sont_retrouves_a_la_reouverture) ; console (8.5, D32) : chaque place de la console et chaque geste de la mosaïque gardent les shells (EssaiDeLaConsole : test_pleine_page_reduire_et_fenetre, test_page_pleine_ranger_et_rouvrir, test_agrandir_et_tirer_le_point_milieu) |
| INV-TERM-02 | Chaque commande terminée dans le terminal (bash) donne exactement un événement de fin avec son code de retour ; la marque de fin n’apparaît jamais dans l’affichage | T (6.7, TERMINAL.md, D27 ; Python sans mutation) : LeTampon (test_marque_de_fin_reperee_et_retiree), LeSuivi (dont test_commande_envoyee_avant_l_invite_initiale, ajouté en 6.7c), UneSessionReelle (test_codes_de_retour_et_duree, test_interrompre_sans_tuer_le_shell, test_cd_persistant_et_bashrc_charge) |
| INV-TERM-03 | Le nom d’un terminal est la commande qu’il exécute, sinon son shell et son dossier, ou « terminé (code N) » ; deux terminaux n’ont jamais le même nom dans la liste | T (6.9, TERMINAUX.md, D29 ; Python sans mutation) : gui/test_terminaux.py (LeNommage) ; EssaiDePlusieursTerminaux (test_plusieurs_terminaux_nommes_par_ce_qu_ils_font) ; LesNomsDansLaConsole (8.5) |
| INV-TERM-04 | La mosaïque de la console n’a qu’une source de vérité, son modèle : chaque terminal ouvert est soit sur la page (quatre au plus), soit en arrière-plan, jamais les deux ni aucun ; le terminal actif est sur la page et toujours montré ; aucune part ne descend sous un quart ; placer, ranger, agrandir ou redimensionner n’arrête aucun shell ; elle se relit à l’identique, et un contenu invalide donne la mosaïque par défaut | T (8.5, CONSOLE.md, D32 ; Python sans mutation) : gui/test_mosaique.py (LesDispositions, LesOperations, LePointMilieu, LaPlace, LaConservation) ; EssaiDeLaConsole (test_la_mosaique_de_un_a_quatre_terminaux, test_page_pleine_ranger_et_rouvrir, test_agrandir_et_tirer_le_point_milieu, test_la_console_est_retrouvee_a_la_reouverture ; retouches 8.5b : test_tirer_le_point_ne_refait_que_la_mosaique, test_la_page_vide_propose_un_terminal) |
| INV-CODE-07 | Retrouver les notes d’un code ne modifie ni le code ni le projet ; une note est rendue si et seulement si un ancrage actif désigne ce fichier (ou un chemin proche, signalé à part) ou si sa marque figure dans le texte ; une marque de note inconnue est signalée | T (7.2, RETROUVER.md) : the_notes_of_a_code_are_found_from_a_file_a_line_or_an_excerpt, an_excerpt_covers_exactly_its_lines ; the_notes_of_a_code_file, the_notes_of_a_code_file_near_paths_and_refusals, the_notes_of_a_code_file_other_forms (memoire-cli, aucune écriture) ; test_quelles_notes_concernent_ce_code (interface) |
| INV-UI-01 | Chaque action du catalogue de l’interface produit des commandes que l’aide figée de la ligne de commande accepte ; chaque commande est atteignable, par une action ou par le mode expert | T (7.1, ACTIONS.md, D30 ; Python sans mutation pour l’interface, Rust muté pour les sorties --tsv) : gui/test_actions.py (LaCoherenceAvecLaLigneDeCommande, LesCommandesProduites) ; branches_have_a_readable_name, the_merge_plan_has_a_tab_separated_form (memoire-cli) ; EssaiDuPanneauDesActions (gui/test_gui.py) ; rail et liste des actions (8.1b) : EssaiDeLHabillage, LeRail (gui/test_apparence.py) |
| INV-LOOK-01 | Le dessin de l’interface est déterministe et toujours valide : mêmes paramètres, mêmes octets ; toute image est un PNG RGBA bien formé, aux dimensions annoncées | T (8.1, APPARENCE.md, D31 ; Python sans mutation) : gui/test_dessin.py (LeFormatPng, LeDeterminisme, LaForme, dont le calcul rapide identique au calcul pixel par pixel) |
| INV-LOOK-02 | Tout couple texte / surface déclaré par le thème atteint un contraste de 4,5:1 (3:1 pour un texte de 18 points et plus) ; l’encre reste lisible sur le verre givré posé sur n’importe quel point du dégradé | T (8.1, APPARENCE.md, D31 ; Python sans mutation) : gui/test_apparence.py (LesContrastes) |
| INV-PLAN-01 | La disposition des panneaux n’a qu’une source de vérité, son modèle ; elle se relit à l’identique ; un fichier absent, illisible ou incohérent donne la disposition par défaut, sans erreur bloquante | T (6.7, PLAN-DE-TRAVAIL.md, D28 ; Python sans mutation) : gui/test_plan_de_travail.py (LesOperations, LaConservation, LeFichier) ; EssaiDuPlanDeTravail (test_la_disposition_est_conservee_et_le_shell_arrete_a_la_fermeture) ; ordre des fenêtres flottantes (6.8) : test_l_utilisateur_decide_de_l_ordre_d_une_fenetre_flottante, test_sous_un_gestionnaire_de_fenetres_l_ordre_est_celui_demande (sous KWin virtuel) ; pleine page (8.5, D32) : LaPleinePage |
| INV-WORK-01 | Citer et relier en un geste ne crée qu’une opération : citation, relation vers la source, relations demandées, nouvelle note et réserve, ou rien | T (6.2, CENTRE.md ; Python sans mutation) : citing_and_linking_in_one_gesture, citing_and_linking_refusals (memoire-cli) ; test_schema : test_champs_a_plusieurs_notes ; test_gui : test_relier_la_selection_en_un_geste, test_la_reserve_n_est_offerte_que_pour_un_echange |
| INV-CODE-06 | L’indexation ne modifie ni le code ni le dépôt ; deux indexations des mêmes fichiers rendent le même rapport | T (6.3, INDEXEUR.md) : the_code_report_classifies_every_anchor_and_marker ; indexing_reports_the_state_of_every_marker_and_anchor (fichiers et dépôt inchangés, même rapport deux fois) |
| INV-RESTORE-02 | Une restauration ciblée ne change que la sélection, ses relations internes, ses ancrages et ses acquittements ; une seule opération, qu’un seul `annuler` défait | T (6.4, RESTAURATION.md) : a_targeted_restore_brings_back_an_idea_with_its_links_only, a_targeted_restore_replaces_later_versions_and_says_so, invalid_targeted_restores_are_refused, targeted_restores_are_critical_and_full_restores_have_a_plan, targeted_restore_blocks_round_trip, targeted_restores_are_refused_in_v1 ; propriétés avec restaurations ciblées ; a_targeted_restore_after_reopening_is_the_same_as_in_memory (memoire-store) ; restoring_an_idea_without_the_rest, restore_refusals (memoire-cli) |
| INV-CODE-01 | Un repère absent ou ambigu est signalé ; aucune occurrence n’est choisie arbitrairement | T (6.3, INDEXEUR.md) : markers_are_found_whatever_the_comment_syntax, the_code_report_classifies_every_anchor_and_marker, anchor_paths_are_compared_relative_to_the_indexed_folder ; indexing_reports_the_state_of_every_marker_and_anchor, indexing_skips_what_it_cannot_read (memoire-cli) |
| INV-RESTORE-01 | Une restauration ciblée conserve le travail indépendant et explicite les conflits ultérieurs | T (6.4, RESTAURATION.md) : a_targeted_restore_brings_back_an_idea_with_its_links_only (relation extérieure conservée et signalée), restoring_an_idea_without_the_rest (memoire-cli, plan « Conservé ») |
| INV-RESUME-01 | Un point de reprise ne change pas le réseau : son commit a l’état de son parent ; il conserve, avec ses identifiants, l’état du réseau et l’état du code au moment où il est noté, après fermeture, export et réimport | T (6.5, FICHE-REPRISE.md, D26) : a_resume_point_publishes_a_commit_without_changing_the_network, resume_points_are_listed_per_branch_newest_first, invalid_resume_points_are_refused, resume_blocks_round_trip, resume_points_are_refused_in_v1, a_patch_of_exactly_the_limit_is_kept_and_read_back ; propriétés avec points de reprise ; resume_points_are_stored_like_in_memory (memoire-store, réouverture) ; a_resume_point_is_noted_with_the_state_of_the_code (réseau inchangé, fiche relue à chaque réouverture), resume_points_are_listed_per_branch (memoire-cli) |
| INV-RESUME-02 | À la lecture d’un point de reprise, tout changement du code depuis (commit ou travail non enregistré) est signalé ; la lecture de l’état du code n’écrit ni dans le code ni dans le dépôt Git (D9) | T (6.5, FICHE-REPRISE.md, D26) : code_states_are_compared, code_state_accessors_give_back_what_was_noted ; a_resume_point_is_noted_with_the_state_of_the_code (code et .git identiques octet pour octet), the_resume_point_says_what_changed_since, resume_points_describe_every_state_of_the_code, git_absent_or_failing_is_reported, unborn_heads_and_the_base_inside_the_repository, resume_refusals (memoire-cli) |

Les statuts positifs de décision, d’implémentation et de vérification devront rester des dimensions distinctes. Le prototype ne fournit volontairement pas encore d’API de validation d’une idée, de clôture d’une réserve ou de validation de code.

## 3 bis. Décisions arrêtées

Les fiches complètes, avec contexte, options écartées et conséquences, sont dans DECISIONS.md. Les contrats marqués T\* seront redéfinis par ces décisions.

| N° | Décision |
| --- | --- |
| D1 | Bifurcation automatique depuis main uniquement ; origine et base enregistrées ; annulation bornée à la base |
| D2 | Contextes superposés au réseau ; un contexte seulement pour une variante suivie en parallèle |
| D3 | UUID de 128 bits injectés par la commande |
| D4 | SQLite fait autorité (WAL, synchronous=FULL) ; Git pour l’export seulement |
| D5 | Révision déclarée mineure ou substantielle |
| D6 | Aucune suppression : statut réversible « plus retenue » |
| D7 | Catégories imbriquées pour organiser, statuts pour décider |
| D8 | Un dépôt par projet, sans lien entre projets |
| D9 | Le retour arrière concerne les idées, jamais le code |
| D10 | Est critique une modification qui peut créer ou lever une cause de réexamen |
| D11 | Les ressources actuelles comprennent les notes proposées ; seules en sont exclues les notes écartées ou à réexaminer |
| D12 | Motif obligatoire pour tout changement de statut |
| D13 | Criticité évaluée sur l’effet réel de l’opération |
| D14 | Domaine sans dépendance ; rusqlite et uuid admis dans les couches stockage et service, versions figées ; audit des dépendances |
| D15 | Format textuel maison, déterministe, à forme canonique unique, spécifié dans FORMAT.md |
| D16 | Versions des règles introduites au premier changement de règle ; format version 1 = règles version 1 |
| D17 | Interface graphique minimale en Tkinter, sans règle, pilotant l’exécutable memoire |
| D18 | Échanges dans le réseau de dépendances ; citations d’échanges et de notes, liées à leur source |
| D19 | Fusion à trois voies par objet, sauvegarde avant fusion, abandon d’une branche sans suppression |

## 3 ter. Registre v0.2 : invariants ajoutés

Tous ont le statut P : spécifiés, non implémentés. Chacun renvoie à son scénario de référence (SCENARIOS.md) et, le cas échéant, au défaut connu qui le viole aujourd’hui. Les identifiants INV-DEL proposés pendant l’audit sont abandonnés : la décision D6 exclut toute suppression.

**Retrait sans suppression (D6)**

| Identifiant | Propriété à préserver | Scénario | Défaut |
| --- | --- | --- | --- |
| INV-RET-01 | Monotonie des identités : tout objet présent dans un commit l’est dans chacun de ses descendants ; aucune commande ne supprime un objet | S-RET-01, S-RET-02 | S10 |
| INV-RET-02 | « Plus retenue » est un statut de décision réversible ; il retire la note des ressources actuelles et crée une cause de réexamen pour ses dépendants, sans modifier leur verdict | S-REF-01 ét. 8 | — |
| INV-RET-03 | Une restauration rétablit l’état des objets existant à la cible ; les objets créés depuis restent présents et résolubles, retirés des ressources actuelles | S-RET-02 | S10 |
| INV-RET-04 | Une relation vers un élément plus retenu ou retiré reste affichée comme telle, jamais filtrée en silence | S-REF-01 ét. 8 | — |

**Modification**

| Identifiant | Propriété à préserver | Scénario | Défaut |
| --- | --- | --- | --- |
| INV-MOD-01 | Toute modification de contenu crée une révision immuable et adressable ; identité et repère sont inchangés | S-MOD-01 | — |
| INV-MOD-02 | Une révision est déclarée mineure ou substantielle ; seule la seconde crée une cause de réexamen ; le type est toujours enregistré | S-MOD-01, S-REF-01 ét. 9 | S6 |
| INV-MOD-03 | Le rôle d’une note est immuable ; changer de rôle passe par une nouvelle note qui remplace l’ancienne | S-MOD-03 | — |
| INV-MOD-04 | Une commande sans effet métier ne publie rien | S-MOD-02 | S2 |
| INV-MOD-05 | Chaque transition de statut (décision, vérification, implémentation, réserve) est une commande explicite avec motif ; revenir sur une transition est une nouvelle transition | S-REF-01 ét. 4 et 8 | — |
| INV-MOD-06 | Doublons : la note remplacée reste résoluble avec ses relations, et son repère indique la remplaçante | S-MOD-04 | — |

**Relations**

| Identifiant | Propriété à préserver | Scénario | Défaut |
| --- | --- | --- | --- |
| INV-REL-01 | Aucune auto-relation ; aucun doublon actif (origine, destination, type, contexte) | S-REL-01 | S4 |
| INV-REL-02 | Les relations « remplace » forment un graphe acyclique dans chaque contexte, vérifié à chaque publication | S-REL-02 | — |
| INV-REL-03 | « Classé dans » entre catégories forme une hiérarchie acyclique ; un élément peut être classé dans plusieurs catégories ; aucun classement ne modifie un statut ni ne crée de cause de réexamen | S-REF-01 ét. 2, S-REL-03 | S11 |
| INV-REL-04 | Les règles de type d’une relation sont vérifiées à son activation ; l’historique n’est jamais rejugé selon des règles postérieures | S-REL-04 | — |
| INV-REL-05 | Aucun remplacement n’est déduit de l’ordre chronologique : « remplace » est explicite et vise un passage, un échange ou une note précis | S-REF-01 ét. 1 | — |
| INV-REL-06 | « Restreint le périmètre » ajoute une condition d’applicabilité sans modifier le verdict de la cible ; il crée une cause de réexamen pour ses dépendants | S-REF-01 ét. 3 | — |
| INV-REL-07 | « Contredit » enregistre un désaccord assumé, avec justification ; il ne propage rien et ne modifie pas la cible | S-REF-01 ét. 4 | — |

**Réserves**

| Identifiant | Propriété à préserver | Scénario | Défaut |
| --- | --- | --- | --- |
| INV-RES-01 | Une réserve est un objet identifié créé depuis un passage, sans destination obligatoire ; elle conserve passage et échange d’origine | S-REF-01 ét. 5, S-RES-01 | — |
| INV-RES-02 | Cycle de vie par contexte : ouverte, traitée à vérifier, résolue ; seule une commande explicite le fait avancer, jamais un lien, une note ou une implémentation | S-RES-01 | — |
| INV-RES-03 | « S’applique à » et « traite » sont distincts ; seule l’action « traite » fait passer à « traitée à vérifier » | S-REF-01 ét. 5 | — |
| INV-RES-04 | Une cible future (fichier inexistant) est enregistrée en texte ; aucune relation ne vise un objet inexistant | S-REF-01 ét. 5 | — |
| INV-RES-05 | Résoudre une réserve dans un contexte ne la résout pas dans un autre | S-RES-01 | — |

**Sources et échanges**

| Identifiant | Propriété à préserver | Scénario | Défaut |
| --- | --- | --- | --- |
| INV-SOURCE-03 | Une source est conservée octet pour octet telle que collée, artefacts compris ; le rendu nettoyé est une vue ; les plages des passages se réfèrent à la source brute | S-REF-01 ét. 1 | — |
| INV-SOURCE-04 | Les frontières question/réponse sont explicites ; aucune n’est déduite d’un marqueur ambigu ; une réponse sans question reste « incomplète » et visible | S-REF-01 ét. 1, S-SRC-01 | — |

**Repères de code (D9)**

| Identifiant | Propriété à préserver | Scénario | Défaut |
| --- | --- | --- | --- |
| INV-CODE-02 | Le repère n’encode que l’identité de la note, jamais son titre ni son contenu | S-REF-01 ét. 6 | S7 |
| INV-CODE-03 | Un ancrage conserve chemin, copie et empreinte du code ciblé à sa création ; la référence Git s’y ajoute si elle existe | S-REF-01 ét. 6, S-CODE-01 | — |
| INV-CODE-04 | Le logiciel n’écrit jamais dans les fichiers de code et ne les restaure pas | S-CODE-03 | — |
| INV-CODE-05 | La résolution d’un repère ne dépend pas du chemin du fichier ; un chemin périmé est signalé, jamais deviné | S-CODE-02 | — |

**Branches, contextes, identités, exécution (D1 à D3)**

| Identifiant | Propriété à préserver | Scénario | Défaut |
| --- | --- | --- | --- |
| INV-BR-02 | Bifurcation automatique depuis main seulement ; sur une branche expérimentale, les opérations critiques s’y poursuivent | S-BR-01 | S1 |
| INV-BR-03 | Chaque branche enregistre origine et base ; l’annulation ne franchit pas la base ; annuler l’opération fondatrice ramène sur l’origine, la branche restant consultable | S-BR-01, S-REF-01 ét. 9 | S3 |
| INV-BR-04 | Le résultat de toute commande indique commit, branche active et bifurcation éventuelle | S-BR-01 | S9 |
| INV-SCOPE-02 | Identité et révisions sont partagées entre contextes ; un contexte porte appartenance, statuts et activation des relations | S-SCOPE-01 | S5 |
| INV-ID-04 | Les identifiants sont générés hors du domaine et transmis par la commande ; même état et mêmes commandes, identifiants compris, donnent le même état | S-ID-01 | S7 |
| INV-PREVIEW-01 | L’effet publié est identique à l’aperçu présenté ; si la tête attendue a changé, la commande est refusée sans effet | S-PREVIEW-01 | — |

## 3 quater. Avancement de la phase 2

Statut T dans memoire-domain : propriété couverte par tests/contrat.rs ou par les tests internes de la crate, efficacité contrôlée par mutation. Les invariants T du registre initial restent couverts : leurs 29 tests ont été portés sur le harnais de scénarios, assertions inchangées.

| Identifiant | Statut | Tests de contrat | Incrément |
| --- | --- | --- | --- |
| INV-ID-04 | T | identical_envelopes_produce_identical_repositories, markers_carry_only_the_identity_and_differ_across_repositories | 2.1 |
| INV-CODE-02 | T | markers_carry_only_the_identity_and_differ_across_repositories | 2.1 |
| INV-BR-04 | T | every_outcome_reports_commit_branch_and_fork | 2.1 |
| INV-MOD-04 | T | commands_without_business_effect_publish_nothing | 2.1 |
| INV-PREVIEW-01 | T | preview_and_execution_share_effects_and_a_stale_preview_is_refused | 2.1 |
| INV-RETRY-01 | T en mémoire ; durable en phase 3 | retrying_an_operation_applies_it_exactly_once | 2.1 |
| INV-ID-02 | T, reprouvé avec identités injectées | an_identity_is_never_reused_even_after_undo | 2.1 |
| INV-TX-01 | T, reprouvé avec identités injectées | identities_of_a_refused_command_remain_available, nil_identifiers_are_refused | 2.1 |
| INV-BR-02 | T | critical_changes_on_an_experimental_branch_stay_on_it, preview_announces_the_fork_decision | 2.2 |
| INV-BR-03 | T | undo_is_bounded_by_the_branch_base_and_returns_to_origin, reentering_a_branch_that_changed_since_is_refused | 2.2 |
| INV-RET-01 | T : suite représentative du contrat et tests de propriétés sur suites aléatoires | every_accepted_change_keeps_all_identities ; tests/proprietes.rs | 2.2, 2.7 |
| INV-RET-03 | T | restore_keeps_later_objects_retired_and_resolvable | 2.2 |
| INV-SCOPE-02 | T | contexts_share_identity_and_revisions ; test interne snapshot_validate_rejects_divergent_or_orphan_content | 2.3a |
| INV-MOD-01 | T | revisions_are_kept_with_their_kind | 2.3a |
| INV-MOD-02 | T pour la révision substantielle ; l’acquittement arrive en 2.6 | substantive_revision_requires_review_of_dependents | 2.3a |
| INV-MOD-03 | T par construction : aucune commande ne porte de rôle hors création | — | 2.3a |
| INV-MOD-04 | T, étendu à la révision identique et à l’inclusion redondante | commands_without_business_effect_publish_nothing | 2.3a |
| INV-RET-02 | T | no_longer_retained_is_reversible_and_requires_review ; test interne every_withdrawal_triggers_review_and_leaves_current_resources | 2.3b |
| INV-MOD-05 | T, étendu au remplacement | status_changes_require_a_reason, replacement_requires_a_reason | 2.3b, 2.4 |
| INV-REL-01 | T | relations_reject_self_and_active_duplicates ; test interne duplicates_are_refused_only_when_active | 2.4 |
| INV-REL-02 | T | replacement_sets_status_atomically_and_stays_resolvable ; test interne cycle_detection_ignores_other_types_and_inactive_edges | 2.4 |
| INV-REL-03 | T | category_hierarchy_is_acyclic | 2.4 |
| INV-REL-06 | T | restriction_adds_a_condition_and_requires_review_of_dependents ; test interne restrictions_change_the_fingerprint | 2.4 |
| INV-REL-07 | T | contradiction_records_a_justification_without_propagation | 2.4 |
| INV-MOD-06 | T | replacement_sets_status_atomically_and_stays_resolvable ; test interne replaced_status_matches_replace_relations | 2.4 |
| INV-SOURCE-01 | T, étendu aux passages du réseau | passages_designate_exact_ranges_and_can_be_corrected ; test interne validate_checks_bodies_against_kinds_and_sources | 2.5 |
| INV-SOURCE-03 | T | exchanges_keep_sources_byte_for_byte_with_explicit_boundaries, reference_scenario_steps_1_and_5_on_real_sources | 2.5 |
| INV-SOURCE-04 | T | exchanges_keep_sources_byte_for_byte_with_explicit_boundaries | 2.5 |
| INV-REL-05 | T sur les sources réelles : cinq échanges saisis dans l’ordre, aucune relation créée | reference_scenario_steps_1_and_5_on_real_sources | 2.5 |
| INV-RES-01 à INV-RES-05 | T | reserves_follow_an_explicit_lifecycle_per_scope, reserve_transitions_require_a_reason, reference_scenario_steps_1_and_5_on_real_sources | 2.5 |
| INV-ACK-01 | T | acknowledgment_lifts_review_without_destroying_provenance, acknowledgment_and_detachment_require_a_reason | 2.6a |
| INV-CODE-03 | T | anchors_keep_path_copy_and_digest, reference_scenario_steps_1_to_9_on_real_sources | 2.6b |
| INV-CODE-04 | T par construction et par garde d’architecture | tests/architecture.rs | 2.6b |
| INV-CODE-05 | T pour la résolution ; la détection des occurrences relève de l’indexeur (phase 5) | markers_resolve_independently_of_paths | 2.6b |
| INV-BR-01, INV-BR-02 | T, redéfinis par D13 : seule une modification critique par son effet bifurque, et seulement depuis main | criticality_follows_the_actual_effect | 2.6c |
| INV-EXPORT-01 | T au niveau du domaine : un instantané comme un dépôt complet (branches, piles de rétablissement, identités employées, commits, journal des 27 commandes) s’encode et se relit à l’identique, en forme canonique unique, avec cohérence vérifiée ; restent l’écriture sur disque et les migrations (phase 3, incrément 3.2 et suivants) | the_reference_example_is_frozen_in_both_directions, every_record_and_value_round_trips, the_repository_example_is_frozen_in_both_directions, a_repository_using_every_command_round_trips, repository_consistency_rules_are_enforced ; tests/proprietes.rs | 3.1, 3.1b |
| INV-REL-04 | Partiel : les rôles étant immuables, une règle de rôle ne peut changer de verdict sur l’historique ; la version des règles (S-REL-04) est reportée par D16 au premier changement de règle | — | 2.4 ; versionnement reporté par D16 au premier changement de règle |

Changements de contrat de l’incrément 2.6c (D13) : criticality_follows_the_actual_effect remplace le test qui classait les commandes par type ; sept tests adaptés, chacun signalé « Contrat modifié par D13 » ; le scénario de référence complété des étapes 8 et 9. Incrément 2.6b : politique de criticité complétée, aucune assertion modifiée.

Changements de contrat de l’incrément 2.6a : deux constructions directes d’échange reçoivent le champ « previous » ; la politique de criticité est complétée (acquitter et retirer une relation sont critiques). Aucune assertion modifiée. Précision d’INV-ACK-01 : « révision du dépendant » s’entend de ses révisions substantielles, une correction mineure ne rouvrant rien (D5).

Changements de contrat de l’incrément 2.5 : aucune assertion modifiée ; la politique de criticité est complétée des nouvelles commandes, toutes ordinaires. Deux attendus mal écrits avant le commit de contrat ont été corrigés, le code n’étant pas en cause : le second supposait que toutes les tabulations du document figuraient dans la première réponse.

Changements de contrat de l’incrément 2.4 : le test exhaustif exclut les graphes à boucle, puisque les auto-relations sont refusées ; la politique de criticité est complétée. Un troisième commit de contrat ajoute au test de remplacement une assertion oubliée (dépôt inchangé après le refus d’un cycle). Le texte attendu d’un nouveau test avait été mal écrit avant le premier commit ; il a été corrigé avant ce commit, l’implémentation n’étant pas en cause.

Changements de contrat de l’incrément 2.3b : la réfutation devient un changement de verdict dans trois tests, signalés en commentaire ; criticality_policy_follows_d10 est complété des changements de statut.

Changements de contrat imposés par D5 et D10 à l’incrément 2.3a : trois tests, signalés en commentaire (commands_without_business_effect_publish_nothing, preview_announces_the_fork_decision, et criticality_policy_follows_d10 qui remplace criticality_policy_is_the_prototype_one). Le commit de contrat de 2.3a ne compile pas seul, puisqu’il emploie les nouvelles commandes ; le commit suivant les implémente.

Changements de contrat imposés par D1 à l’incrément 2.2 : trois tests existants, signalés en commentaire (discarded_redo_remains_addressable_in_history, every_outcome_reports_commit_branch_and_fork, preview_and_execution_share_effects_and_a_stale_preview_is_refused). Ils sont isolés dans deux commits de contrat, le second corrigeant un oubli du premier.

Garantie nouvelle de structure : toute commande passe par une préparation qui ne reçoit qu’une référence partagée du dépôt, puis par une application infaillible. Un refus ne peut donc rien modifier, ce que le compilateur vérifie. Les tests d’égalité complète du dépôt restent nécessaires pour les contrôles faits dans `execute` avant la préparation.

## 4. Modèle du réexamen

Convention : une arête « B dépend de A » va de B vers A.

Dans le prototype, les racines de réexamen sont les notes explicitement réfutées ou retirées des ressources actuelles, dans le contexte consulté. Les catégories n’en sont pas.

Pour une note n, les causes sont les racines atteignables depuis n en suivant les arêtes actives « dépend de » ou « développé à partir de ». La racine n’est pas ajoutée comme son propre dépendant. Les liens thématiques et de classement sont exclus.

Le calcul retourne un ensemble de causes, pas un booléen à modifier dans chaque descendant. Annuler une réfutation ou modifier une relation permet ainsi de recalculer les effets sans conserver de drapeau périmé.

Attention à un point encore ouvert : « développé à partir de » décrit une provenance, qui peut justifier une première révision sans constituer une dépendance permanente. L’utilisateur doit pouvoir confirmer que la note reste pertinente tout en conservant sa provenance. Il faudra donc un acquittement versionné, pas supprimer le lien historique pour faire disparaître l’alerte. Ce mécanisme n’est pas encore livré.

### Modèle v0.2 proposé

Une note c est une **cause** pour un dépendant n lorsque c est atteignable depuis n par des relations actives « dépend de » ou « développé à partir de », et que l’une des conditions suivantes est vraie :

- c est dans un état racine : réfutée, plus retenue ou retirée des ressources actuelles ;
- l’empreinte de c diffère de sa valeur de référence. L’empreinte réunit la dernière révision substantielle, les statuts et les restrictions de périmètre de c ; la valeur de référence est celle enregistrée à l’activation de la relation ou au dernier acquittement.

Mise en œuvre : chaque relation mémorise, à son activation, l’empreinte de sa destination, soit ses révisions substantielles (2.3a) et ses restrictions de périmètre actives (2.4) ; une variation fait de la destination une cause pour la source et ses dépendants indirects. Les statuts agissent par l’écart (réfutée, retirée, plus retenue, remplacée), qui rend la note cause pour tous ses dépendants (2.3b). L’acquittement arrive en 2.6.

n est **à réexaminer** s’il possède au moins une cause non couverte par un acquittement (n, révision de n, c, empreinte de c, contexte). Le calcul reste un ensemble de causes, jamais un drapeau stocké : annuler une opération rétablit exactement les causes précédentes. Une révision mineure ne change pas l’empreinte ; elle ne crée donc aucune cause (D5).

## 5. Réversibilité : quelle égalité attend-on ?

Après annulation d’une commande, l’état métier doit être égal à son état précédent : textes, relations actives et inactives, visibilité, verdicts et causes de réexamen.

En revanche, l’historique et les compteurs d’identité ne doivent pas revenir matériellement en arrière : le commit abandonné reste accessible et ses identifiants ne sont pas réutilisés.

Une restauration est une nouvelle action qui référence un état conservé. Elle est elle-même annulable. Le prototype restaure l’ensemble du réseau ; restaurer seulement un groupe demande encore une résolution des interactions avec les modifications intervenues ensuite.

Une opération collective doit devenir une transaction unique. Le prototype fournit seulement des commandes élémentaires ; l’API de lot devra valider toutes les étapes avant publication et créer une seule branche pour l’ensemble.

## 6. Architecture cible

| Couche | Responsabilité | Interdiction importante |
| --- | --- | --- |
| Interface ou CLI | Sélectionner, annoter, présenter les effets et les conflits | Ne modifie pas directement les objets stockés |
| Service de commandes | Identifier l’opération, vérifier l’état attendu, orchestrer une transaction | Ne considère pas la réussite d’un affichage comme une sauvegarde |
| Domaine Rust | Préparer un état candidat, vérifier les relations et calculer les effets | N’accède pas directement au navigateur, au fournisseur d’IA ou aux fichiers de code |
| Stockage | Publier atomiquement commits, relations, branche et résultat d’opération | Ne publie pas une tête dont le contenu n’est pas durable |
| Index de recherche et de code | Construire des vues dérivées | Ne constitue pas l’unique copie des données métier |

Pipeline cible : réception de commande → préparation et classification → validation → écriture transactionnelle → publication du résultat → mise à jour des vues.

Organisation du workspace :

- crates/memoire-core : prototype v0.1.2, retiré à l’incrément 2.7 après la parité ; il reste dans l’historique Git (étiquette v0.1.2).
- crates/memoire-domain : domaine, sans aucune dépendance externe. Repository y combine encore service et stockage en mémoire ; la persistance viendra en phase 3, dans une crate distincte.

Dans memoire-domain, toute modification est une commande (Command : Change, Undo, Redo, Checkout) transportée dans une enveloppe (identifiant d’opération, état attendu, motif). `execute` contrôle l’enveloppe (journal, identifiant nul, état attendu), puis enchaîne `plan`, en lecture seule et faillible, et `apply`, infaillible. `preview` emploie le même `plan` : l’aperçu et l’exécution ne peuvent pas diverger. Le commit publié et la branche éventuellement créée portent l’identité de l’opération. Graph contient les règles de relations et de réexamen ; Snapshot représente les états par contexte ; SourceText et Excerpt conservent les sources.

L’interface publique ne fournit pas d’accès mutable direct aux instantanés. Les identifiants NodeId, EdgeId, ScopeId, CommitId, BranchId et OpId sont des types Rust différents, de 128 bits, fournis par l’appelant ; l’identifiant nul est réservé aux objets initiaux. Les invariants qui dépassent ces types sont vérifiés par Result et par la validation du candidat.

## 7. Branche automatique et classement des commandes

Le prototype traite comme critiques : réfuter, archiver, réattacher, restaurer, éditer le contenu et ajouter une dépendance. Il traite comme ordinaires la création d’une note et l’ajout d’un simple classement ou lien thématique. Depuis l’incrément 2.6c, memoire-domain applique D13 : est critique une modification qui, dans l’état courant, crée ou lève une cause de réexamen, écarte une note ou restaure un état antérieur ; l’aperçu annonce la décision. Une branche n’est créée que pour une modification critique engagée sur main (D1).

C’est une politique conservatrice proposée pour l’exploration technique. Le noyau ne devine pas si un changement de texte est une correction typographique ou un changement d’hypothèse. Il faudra définir des commandes plus précises pour alléger l’usage sans permettre à l’interface de contourner les protections.

Une branche critique est publiée seulement si l’état candidat est valide. En cas de refus, ne laisser ni branche vide ni relation partiellement désactivée.

Le prototype conserve les instantanés complets par copie : facile à comprendre et à tester, mais coûteux pour un grand historique. Des révisions partagées, des deltas ou des collections persistantes pourront remplacer cette représentation sans modifier les invariants métier.

## 8. Git, stockage durable et fusion

Les branches logiques ne nécessitent pas de choisir immédiatement Git comme moteur.

Deux options restent possibles :
- Git stocke les objets et instantanés du réseau ; le logiciel garde sa validation métier et sa présentation des conflits.
- Un stockage transactionnel local conserve le réseau et ses branches ; Git reste lié au code, avec un export possible du réseau.

Proposition à examiner ensuite : un stockage local transactionnel comme autorité du réseau, et des références explicites aux versions du code. Cela évite de demander à une fusion textuelle de décider de la validité des idées. Ce n’est pas encore un choix arrêté.

Si Git et une base sont utilisés ensemble, ne pas leur confier deux vérités concurrentes sans protocole de synchronisation. Une transaction sur la base ne rend pas atomique une écriture indépendante dans un dépôt Git.

La fusion doit traiter au moins :
- deux changements incompatibles du même texte ou statut ;
- un retrait face à un réattachement ;
- deux remplacements concurrents ;
- l’apparition d’un cycle de remplacement après réunion d’arêtes individuellement valides ;
- la modification de main après préparation du plan de fusion.

L’application devra vérifier les têtes de branches attendues au moment de publier. Une fusion obsolète est recalculée ou refusée. Le prototype ne contient aucune méthode de fusion ; il ne simule pas ce comportement avec un remplacement silencieux de main.

## 9. Stratégie de tests pour faire évoluer le code avec l’IA

### Trois niveaux de protection

1. Types et encapsulation : distinguer les identités, limiter les mutations, représenter les rôles.
2. Validation de production : retourner une erreur sur entrée invalide et publier seulement un état valide.
3. Tests contractuels : vérifier les propriétés observables et les suites d’actions.

Un mauvais identifiant ou un lien interdit constitue une erreur récupérable, pas une raison normale de faire paniquer l’application.

Les tests utilisent assert! et assert_eq!. Les validations métier ne doivent pas dépendre de debug_assert!, désactivé par défaut dans les compilations optimisées. Les assertions internes de débogage ne remplacent ni Result ni une transaction durable.

### Tests déjà écrits

Le contrat comprend 37 tests : 29 tests d’intégration (tests/invariants.rs) et 8 tests internes (src/lib.rs). Les 22 tests d’origine n’ont pas été modifiés. Ils couvrent les invariants marqués T, notamment :
- stabilité d’identité et du commentaire ;
- absence d’effets après refus d’une commande ;
- isolation de main et des contextes ;
- invalidation d’une sous-idée sans propagation au parent ni aux voisins ;
- conservation de l’ancien rattachement et des liens internes ;
- annulation et rétablissement de l’état métier complet ;
- accès à un état abandonné après une nouvelle modification ;
- conservation de plusieurs causes de réexamen ;
- sources contenant accents et emoji ;
- comparaison du parcours du moteur à une fermeture transitive indépendante.

Le dernier test énumérait à l’origine les 512 graphes dirigés de trois notes, boucles incluses, avec trois choix de racine (1 536 cas). Depuis l’incrément 2.4, les auto-relations sont refusées : il énumère les 64 graphes sans boucle, soit 192 cas. C’est exhaustif sur ce petit domaine seulement ; ce n’est pas une preuve pour tous les graphes ni tous les scénarios utilisateur.

Deux régressions ciblées ont aussi été injectées dans des copies du projet : publier dans la branche d’origine et propager par les liens thématiques. Dans les deux cas, le test attendu échoue. Ce contrôle montre que ces tests détectent ces deux erreurs précises.

### Mutation systématique (depuis la v0.1.1)

Sur la v0.1.0, cargo-mutants a produit 13 mutants survivants. Les plus graves : remplacer la validation du candidat par un succès inconditionnel, ou le repère par une constante, ne faisait échouer aucun test. La validation n’était pas atteignable comme dernier rempart depuis l’API publique, car chaque commande filtre ses erreurs en amont ; elle est désormais testée par des tests internes.

Critère de sortie de toute évolution du noyau : aucun mutant survivant, mesuré sur le contrat seul (tests internes et tests/invariants.rs). Un mutant jugé équivalent est supprimé par réécriture du code ou documenté ici avec sa justification ; aucun n’est toléré en silence.

Les tests internes de validation documentent aussi un défaut de diagnostic : l’erreur CategoryHasNoVerdict sert aux deux sens de la règle (catégorie avec verdict, note sans verdict). Renommage prévu avec la refonte des erreurs.

### Registre des défauts connus

Le registre actif est crates/memoire-domain/tests/defauts_connus.rs : il décrit les comportements actuels jugés incorrects. Ces tests passent tant que le défaut existe ; ils ne comptent pas comme protection et sont exclus du test de mutation. Lorsqu’un correctif en fait échouer un, le test est retiré du registre et le test du comportement correct est ajouté à tests/contrat.rs, dans un commit dédié au changement de contrat. Le registre de memoire-core reste figé avec le prototype.

### Défauts corrigés

| Défaut | Incrément | Test qui le remplace | Remarque |
| --- | --- | --- | --- |
| S2 | 2.1 | commands_without_business_effect_publish_nothing | — |
| S7 | 2.1 | markers_carry_only_the_identity_and_differ_across_repositories | Corrigé dans le domaine ; l’unicité entre dépôts dépendra de la génération d’UUID aléatoires par la couche service (phase 3) |
| S8 | 2.6a | acknowledgment_lifts_review_without_destroying_provenance | La relation de provenance reste active |
| S12 | 2.7 | a_reserve_treated_twice_reopens_only_when_no_treatment_remains | Découvert en cherchant à tuer trois mutants survivants : avec deux traitements, le dernier retrait ne rouvrait pas la réserve |
| S4 | 2.4 | relations_reject_self_and_active_duplicates | — |
| S11 | 2.4 | category_hierarchy_is_acyclic | — |
| S5 | 2.3a | contexts_share_identity_and_revisions | — |
| S6 | 2.3a | substantive_revision_requires_review_of_dependents | — |
| S1 | 2.2 | critical_changes_on_an_experimental_branch_stay_on_it | — |
| S3 | 2.2 | undo_is_bounded_by_the_branch_base_and_returns_to_origin | — |
| S10 | 2.2 | restore_keeps_later_objects_retired_and_resolvable | Monotonie générale vérifiée par every_accepted_change_keeps_all_identities |
| S9 | 2.1 | every_outcome_reports_commit_branch_and_fork | La sonde d’origine ne pouvait pas échouer : elle constatait la bascule de branche, pas l’absence de signal. Le nouveau test vérifie le signal |

Aucun défaut actif depuis l’incrément 2.6a ; le registre reste le lieu où consigner un défaut découvert.

### Données d’entrée des scénarios

tests/scenario_fixtures.rs vérifie que les fichiers figés de fixtures/rtos-2026-10 n’ont pas changé (taille, artefacts de collage) et que chaque passage cité par S-REF-01 y figure une seule fois. Ces tests ne protègent pas le noyau et sont exclus du test de mutation.

### Tests de propriétés (incrément 2.7)

tests/proprietes.rs engendre 48 suites de 80 commandes, à graines fixes, sans dépendance externe. Après chaque commande, il vérifie : refus sans effet (INV-TX-01), aperçu identique à l’exécution (INV-PREVIEW-01), conservation des identités (INV-RET-01), annulation et rétablissement exacts (INV-UNDO-01), bifurcation seulement depuis main (INV-BR-02), validité de l’état et absence de note cause d’elle-même ; en fin de suite, le rejeu des enveloppes acceptées redonne le même dépôt (INV-ID-04). Sa sensibilité a été vérifiée en réintroduisant la restauration destructive d’origine (défaut S10) : elle est détectée dès la première graine. Ces tests ne font pas partie du test de mutation, trop long ; la clé de reprise de la mutation ne couvre donc que le contrat qu’elle exécute.

### Contrôle unique

Depuis la phase 2, CHECK_MUTANTS_SHARDS=n découpe la mutation en n parts reprenables : une part réussie est consignée sous une clé calculée sur le code, les tests, les données d’entrée et Cargo.lock, et n’est sautée que si cette clé est inchangée.

tools/check.sh enchaîne formatage, clippy (avertissements bloquants), tests debug et release, mutation sur le contrat et régressions ciblées. Il s’arrête au premier écart ; sauter la mutation rend un code de sortie distinct (3, contrôle incomplet). Le code livré interdit unwrap, expect et panic (attribut de crate vérifié par clippy) : une erreur de domaine passe par Result.

### Traçabilité

TRACABILITE.md attribue des identifiants stables CA-01 à CA-40 aux critères d’acceptation du cahier des charges et les relie aux invariants et aux tests.

### Suite des vérifications

- Tests de séquences générées : créer, lier, invalider, annuler, refaire, changer de branche, réattacher. Comparer à un modèle de référence plus simple, notamment avec proptest.
- Réouverture après chaque point d’arrêt du protocole d’enregistrement.
- Import, export et migrations sur des données de versions précédentes.
- Fusion à base commune et conflits structurels.
- Idempotence en cas de répétition d’une opération.
- Reconstruction des index depuis les données autoritatives.
- Jeux de données réels pour les performances et les gestes de l’interface.

Ne pas écrire les tests en recopiant l’algorithme testé. Préférer des exemples dont le résultat est fixé avant l’implémentation, des lois d’état et des oracles indépendants.

### Ce que signifie « verrouiller les tests »

Des fichiers de tests ne sont pas inviolables si l’IA peut les modifier. Les identifier par invariants, les faire exécuter systématiquement et examiner séparément les changements de contrat constituent une protection pratique.

Une évolution interne doit conserver les tests de comportement. Si une règle métier change volontairement, modifier explicitement l’invariant et ses cas de référence, puis expliquer pourquoi. Ne pas corriger un test uniquement parce qu’il bloque un code nouvellement généré.

Une CI et les protections de la branche de référence pourront ajouter un contrôle extérieur au processus qui écrit le code. Elles ne sont pas configurées dans cette archive locale.

## 10. Limites explicites du code livré

- Exécution en mémoire uniquement : fermeture du processus = perte du dépôt si aucun stockage n’est ajouté.
- Pas de fusion, de restauration ciblée ni de commande collective atomique.
- Pas d’importeur de conversations, d’annotations branchées ou de réserves avec résolution.
- SourceText et Excerpt, repris du prototype, sont remplacés dans le réseau par les échanges et passages de l’incrément 2.5 ; ils seront retirés avec memoire-core.
- L’ordre des échanges d’une conversation est donné explicitement depuis l’incrément 2.6a (échange précédent) ; il ne crée aucune relation.
- Pas de méthode de confirmation d’un réexamen et pas de validation positive avec preuve.
- Le repère produit un commentaire C ; pas d’indexeur ni de détection des occurrences.
- memoire-domain : identifiants de 128 bits fournis par l’appelant ; aucun générateur d’UUID n’existe encore (couche service, phase 3).
- memoire-domain : le journal des opérations grandit sans limite en mémoire ; sa conservation durable relève de la phase 3.
- memoire-domain : depuis l’incrément 2.6a, Detach retire une relation en la conservant inactive ; un seul remplacement actif par note.
- Copie complète des instantanés et parcours simples : aucune cible de performance validée.
- Une commande refuse les erreurs de domaine avant publication ; cela ne garantit pas la durabilité face à une panne ou à un arrêt par manque de mémoire.
- memoire-domain conserve désormais l’identifiant d’opération et le motif de chaque commit ; l’auteur n’est pas enregistré (usage personnel, un seul auteur).

- Le registre v0.2 (échanges, révisions, réserves, acquittements, repères ancrés) est spécifié mais non implémenté.

Le noyau sert à éprouver des propriétés avant de choisir la structure finale.

## 11. Prochain ordre de travail proposé

La phase 2 est close : le modèle v0.2 est implémenté dans memoire-domain, S-REF-01 s’exécute en entier sur les sources réelles, aucun défaut n’est actif, et le prototype est retiré.

1. Phase 3 — persistance SQLite (D4) : schéma v1, journal des opérations idempotent et durable, couche service générant les UUID (D3), tests d’arrêt brutal, export et import versionnés, versions des règles (INV-REL-04, S-REL-04).
2. Phase 4 — CLI minimale et usage réel sur le mini-RTOS, en rejouant S-REF-01 sur les fichiers réels puis en travaillant avec.
3. Phase 5 — indexeur de repères : occurrences, chemins périmés, états retrouvé, modifié, ambigu, perdu (INV-CODE-01, CA-30, CA-31, CA-38).
4. Phase 6 — fusion et restauration ciblée (INV-MERGE-01 à -03, INV-RESTORE-01).
5. Ensuite seulement : interface.

## 12. Sources techniques primaires consultées

- Rust, erreurs récupérables avec Result : https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html
- Rust, écriture des tests : https://doc.rust-lang.org/book/ch11-01-writing-tests.html
- Rust, assert! : https://doc.rust-lang.org/std/macro.assert.html
- Rust, debug_assert! : https://doc.rust-lang.org/std/macro.debug_assert.html
- Proptest, tests de machines à états : https://proptest-rs.github.io/proptest/proptest/state-machine.html
- Git, bases communes : https://git-scm.com/docs/git-merge-base
- Git, fusion : https://git-scm.com/docs/git-merge
- SQLite, protocole de commit atomique et hypothèses matérielles : https://sqlite.org/atomiccommit.html
- im, partage structurel pour les collections persistantes Rust, piste d’optimisation non retenue à ce stade : https://docs.rs/im/latest/im/

## 13. Vérification réalisée sur cette version

Phase 5, incrément 5.2b (commit efd6a84), contrôle complet par tools/check.sh dans une copie figée (détail : VERIFICATION-PHASE5.txt) : toutes les étapes réussies, essai graphique compris ; mutation : domaine 765 mutants (667 détectés, 98 non compilables), stockage 18, service 23, ligne de commande 406, aucun survivant, après l’ajout d’un test (quitter une branche dont l’origine a avancé) ; les deux régressions ciblées restent détectées, après adaptation explicite d’un point d’injection ; équivalence avec l’oracle après chaque commande.

Phase 5, incréments 5.2c et 5.2d (commit 77febbe), contrôle complet par tools/check.sh dans une copie figée (détail : VERIFICATION-PHASE5.txt) : toutes les étapes réussies, essai graphique compris (4 tests, dont la question de migration) ; 254 tests Rust, en debug et en release ; mutation : domaine 824 mutants (704 détectés, 120 non compilables), stockage 94, service 33, ligne de commande 422, aucun survivant, après l’ajout de quatre tests et le remplacement d’une garde redondante par un refus explicite ; les deux régressions ciblées restent détectées. Deux passages précédents arrêtés par un mutant qui épuisait la mémoire : chaque binaire de test de la mutation est désormais plafonné (tools/lanceur-mutation.sh).

Livraison v0.5.0-alpha.2 (incréments 5.2a à 5.2e, commit 4910e01), contrôle complet (VERIFICATION-PHASE5.txt) : toutes les étapes réussies, essai graphique compris ; 220 tests Rust, en debug et en release (oracle memoire-reference retiré comme prévu) ; mutation : domaine 824 mutants (repris sous sa clé), stockage 94, service 33, ligne de commande 422, rejoués, aucun survivant ; les deux régressions ciblées restent détectées. Mesure finale : toutes les commandes sous 0,11 s à 5 000 opérations et 1 500 notes ; durées indépendantes de la longueur de l’historique à état constant.

Phase 5, incrément 5.3 (commit 47ef3d5), contrôle complet (VERIFICATION-PHASE5.txt) : toutes les étapes réussies au premier passage, essai graphique compris (5 tests) ; 234 tests Rust, en debug et en release, dont les propriétés avec lots aléatoires à travers l’historique en mémoire et SQLite ; mutation : domaine 836 mutants (713 détectés, 123 non compilables), stockage 94, service 33, ligne de commande 426, aucun survivant ; les deux régressions ciblées restent détectées.

Phase 5, incrément 5.4 (commit c928f3b), contrôle complet (VERIFICATION-PHASE5.txt) : toutes les étapes réussies au premier passage, essai graphique compris (5 tests, 35 commandes) ; 272 tests Rust, en debug et en release, dont les propriétés avec abandons et avec fusions résolues au hasard, à travers l’historique en mémoire et SQLite ; mutation : domaine 943 mutants (800 détectés, 143 non compilables), stockage 106, service 33, ligne de commande 485, aucun survivant ; les deux régressions ciblées restent détectées. INV-BR-05 et INV-MERGE-01 à 03 passent au statut T.

Phase 5, incrément 5.5 (commit 765377c), contrôle complet (VERIFICATION-PHASE5.txt) : réussi au second passage (le premier, au commit 5991224, avait échoué aux régressions ciblées sur un point d’injection devenu ambigu, corrigé en factorisant Relation::propagates) ; 281 tests Rust, en debug et en release, dont les propriétés avec citations de notes ; essai graphique (5 tests, 36 commandes) ; mutation : domaine 983 mutants (837 détectés, 146 non compilables), stockage 106, service 33, ligne de commande 495, aucun survivant ; les deux régressions ciblées restent détectées. INV-CIT-01, INV-CIT-02 et INV-ORPH-01 au statut T.

Phase 6, incrément 6.1 (commit c996d96), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; 284 tests Rust, en debug et en release ; interface : 22 tests sans affichage (lecture de l’aide, rendu du visionneur) et 10 essais graphiques (dont 5 du visionneur, 38 commandes) ; mutation : domaine 983 mutants (repris), stockage 106, service 33, ligne de commande 525, aucun survivant ; les deux régressions ciblées restent détectées. INV-VIEW-01 et INV-SOURCE-02 au statut T (le Python n’est pas soumis à la mutation).

Phase 6, incrément 6.2 (commit 6bf944b), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; 286 tests Rust, en debug et en release ; interface : 22 tests sans affichage et 12 essais graphiques ; mutation : domaine 983 mutants (repris), stockage 106, service 33, ligne de commande 531, aucun survivant ; les deux régressions ciblées restent détectées. INV-WORK-01 au statut T.

Phase 6, incrément 6.3 (commit c82cf22), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; 291 tests Rust, en debug et en release ; interface : 23 tests sans affichage et 12 essais graphiques (39 commandes) ; mutation : domaine 1 008 mutants (860 détectés, 148 non compilables), stockage 106, service 33, ligne de commande 558, aucun survivant ; les deux régressions ciblées restent détectées. INV-CODE-01 et INV-CODE-06 au statut T ; CA-05, CA-30, CA-31 couverts.

Phase 8, livraison v0.8.0-alpha.2 (incrément 8.3, commit 349151c), contrôle complet (VERIFICATION-PHASE8.txt) : réussi au premier passage ; code Rust inchangé, 334 tests Rust en debug et en release ; interface : 146 tests sans affichage et 60 essais graphiques sous KWin virtuel ; mutation reprise du cache, aucun survivant. R18 fait (reste le terminal en onglets).

Phase 8, livraison v0.8.0-alpha.1 (incréments 8.1 et 8.2, commit 21aeb75), contrôle complet (VERIFICATION-PHASE8.txt) : réussi au premier passage ; code Rust inchangé, 334 tests Rust en debug et en release ; interface : 144 tests sans affichage et 58 essais graphiques sous KWin virtuel ; mutation reprise du cache (domaine 1 131 mutants, stockage 106, service 33, ligne de commande 776), aucun survivant ; les deux régressions ciblées restent détectées. R15, R16 et R17 faits.

Phase 8, incrément 8.1 (commit 4830655), contrôle complet (VERIFICATION-PHASE8.txt) : réussi au premier passage ; code Rust inchangé, 334 tests Rust en debug et en release ; interface : 137 tests sans affichage et 40 essais graphiques sous KWin virtuel (41 après la correction du survol, d49a5df, rejoués sous KWin) ; mutation : domaine 1 131 mutants (972 détectés, 159 non compilables), stockage 106, service 33, ligne de commande 776, aucun survivant ; les deux régressions ciblées restent détectées. INV-LOOK-01 et INV-LOOK-02 au statut T (Python sans mutation) ; R15 fait.

Phase 7, incrément 7.2 (commit 1c36464), contrôle complet (VERIFICATION-PHASE7.txt) : réussi au premier passage ; 334 tests Rust, en debug et en release ; interface : 99 tests sans affichage et 36 essais graphiques sous KWin virtuel ; mutation : domaine 1 131 mutants (972 détectés, 159 non compilables), stockage 106, service 33, ligne de commande 776, aucun survivant ; les deux régressions ciblées restent détectées. INV-CODE-07 au statut T ; R14 fait.

Phase 7, incrément 7.1 (commit db3b56f), contrôle complet (VERIFICATION-PHASE7.txt) : réussi au premier passage ; 328 tests Rust, en debug et en release ; interface : 98 tests sans affichage et 33 essais graphiques sous KWin virtuel ; mutation : domaine 1 100 mutants, stockage 106, service 33, ligne de commande 756, aucun survivant ; les deux régressions ciblées restent détectées. INV-UI-01 au statut T ; R13 fait.

Phase 6, incrément 6.9 (commit c5a9799), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; code Rust inchangé, 326 tests Rust ; interface : 85 tests sans affichage et 27 essais graphiques sous KWin virtuel ; mutation : domaine 1 100 mutants, stockage 106, service 33, ligne de commande 732, aucun survivant ; les deux régressions ciblées restent détectées. INV-TERM-03 au statut T ; R11 et R12 faits.

Phase 6, incrément 6.8 (commit e8a9391), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; code Rust inchangé, 326 tests Rust ; interface : 73 tests sans affichage et 23 essais graphiques, désormais sous KWin virtuel (ordre des fenêtres flottantes vérifié sous un vrai gestionnaire de fenêtres) ; mutation : domaine 1 100 mutants, stockage 106, service 33, ligne de commande 732, aucun survivant ; les deux régressions ciblées restent détectées.

Phase 6, incrément 6.7 (commit 928db9c), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; code Rust inchangé, 326 tests Rust en debug et en release ; interface : 72 tests sans affichage (moteur du terminal sur de vrais shells, notifications, modèle du plan de travail) et 21 essais graphiques ; mutation : domaine 1 100 mutants, stockage 106, service 33, ligne de commande 732, aucun survivant ; les deux régressions ciblées restent détectées. INV-TERM-01, INV-TERM-02 et INV-PLAN-01 au statut T (Python hors mutation) ; R5 fait.

Phase 6, incrément 6.6 (commit a3e6e38), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; 326 tests Rust, en debug et en release ; interface : 31 tests sans affichage et 16 essais graphiques (47 commandes) ; mutation : domaine 1 100 mutants (942 détectés, 158 non compilables), stockage 106, service 33, ligne de commande 732, aucun survivant ; les deux régressions ciblées restent détectées. INV-VIEW-02 au statut T ; R7 fait.

Phase 6, incrément 6.5 (commit d749453), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au troisième passage — le premier a échoué à l’audit RustSec (RUSTSEC-2026-0459, SQLite embarqué par libsqlite3-sys 0.30.1, corrigé par rusqlite 0.41.0 et libsqlite3-sys 0.39.1, conformes à D14), le deuxième a été interrompu par une coupure d’électricité ; 323 tests Rust, en debug et en release, dont les propriétés avec points de reprise ; interface : 24 tests sans affichage et 13 essais graphiques (46 commandes) ; mutation : domaine 1 091 mutants (934 détectés, 157 non compilables), stockage 106, service 33, ligne de commande 703, aucun survivant ; les deux régressions ciblées restent détectées. INV-RESUME-01 et INV-RESUME-02 au statut T ; CA-08 couvert.

Phase 6, incrément 6.4 (commit 298cb78), contrôle complet (VERIFICATION-PHASE6.txt) : réussi au premier passage ; 304 tests Rust, en debug et en release, dont les propriétés avec restaurations ciblées ; interface : 23 tests sans affichage et 12 essais graphiques (42 commandes) ; mutation : domaine 1 038 mutants (886 détectés, 152 non compilables), stockage 106, service 33, ligne de commande 598, aucun survivant ; les deux régressions ciblées restent détectées. INV-RESTORE-01 et INV-RESTORE-02 au statut T ; CA-27 couvert.

Phase 5, incrément 5.1 (commit 833b512), contrôle complet par tools/check.sh dans une copie figée (détail : VERIFICATION-PHASE5.txt) : formatage, cohérence documentaire, dépendances (D14), audit RustSec, clippy, tests debug et release réussis ; mutation : domaine 745 mutants (659 détectés, 86 non compilables), stockage 18 (12, 6), service 23 (13, 10), ligne de commande 406 en quatre parts (388 détectés, 18 non compilables), aucun survivant ni dépassement de délai ; les deux régressions ciblées restent détectées. Limite : xvfb-run absent, l'essai graphique de l'interface a été sauté ; la lecture de l'aide sans Tkinter est vérifiée, et ni l'interface ni l'aide figée ne changent dans cet incrément.

Phase 4, incrément 4.4 (commit a290376), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE4.txt) : étapes précédentes et essais de l’interface réussis ; mutation : domaine repris sous sa clé (code inchangé), stockage 18 mutants, service 23, ligne de commande 406 en quatre parts (388 détectés, 18 non compilables), aucun survivant ; les deux régressions ciblées restent détectées.

Phase 4, incrément 4.3 (commit 31925aa), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE4.txt) : étapes précédentes réussies ; interface : lecture de l’aide (7 tests, sans Tkinter) et essai graphique sous affichage virtuel réussis ; aucune modification du code Rust, mutations reprises sous leurs clés ; les deux régressions ciblées restent détectées. Limite : le code Python de l’interface n’est pas soumis à la mutation (aucun outil sans dépendance nouvelle) ; sa logique est isolée dans gui/schema.py et couverte par son contrat.

Phase 4, incrément 4.2 (commit 46a64af), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE4.txt) : formatage, cohérence documentaire, dépendances conformes à D14, audit RustSec sans alerte, clippy, tests debug et release des quatre crates réussis, dont la recette S-REF-01 rejouée en entier par la ligne de commande sur les sources réelles ; mutation : domaine, stockage et service repris sous leurs clés (code inchangé), ligne de commande 312 mutants, 294 détectés, 18 non compilables, aucun survivant, après correction de cinq survivants dans repere ; les deux régressions ciblées restent détectées.

Phase 4, incrément 4.1 (commit 8e60b25), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE4.txt) : formatage, cohérence documentaire, dépendances conformes à D14, audit RustSec sans alerte, clippy, tests debug et release des quatre crates réussis ; mutation : domaine 593 mutants (parts reprises sous leur clé, code inchangé), stockage 17, service 22, ligne de commande 156 (144 détectés, 12 non compilables), aucun survivant ; les deux régressions ciblées restent détectées.

Incrément 3.4 et fin de la phase 3 (commit 198f832), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE3.txt) : formatage, cohérence documentaire, dépendances conformes à D14, audit RustSec sans alerte, clippy, tests debug et release des trois crates réussis ; mutation de memoire-domain, 593 mutants, 515 détectés, 78 non compilables, aucun survivant ; memoire-store, 17 mutants, 11 détectés, 6 non compilables, aucun survivant ; memoire-app, 20 mutants, 10 détectés, 10 non compilables, aucun survivant, une fonction exclue avec justification (sync_directory, synchronisation de répertoire observable seulement en cas de coupure d’alimentation) ; contrats : 86 tests d’intégration et 27 tests internes (domaine), 8 (stockage), 9 et 1 test interne (service) ; les deux régressions ciblées restent détectées.

Incrément 3.3 (commit 1105dcb), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE3.txt) : formatage, cohérence documentaire, dépendances conformes à D14 (40 paquets), audit RustSec sans alerte (43 paquets), clippy, tests debug et release des trois crates réussis ; mutation de memoire-domain en huit parts, 593 mutants, 515 détectés, 78 non compilables, aucun survivant ; memoire-store, 15 mutants, aucun survivant ; memoire-app, 16 mutants, 7 détectés, 9 non compilables, aucun survivant ; contrats : 86 tests d’intégration et 27 tests internes (domaine), 8 (stockage), 6 (service) ; les deux régressions ciblées restent détectées.

Incrément 3.2 (commit 56d01a8), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE3.txt) : formatage, cohérence documentaire, dépendances conformes à D14 (23 paquets), audit RustSec sans alerte (25 paquets, avertissements bloquants), clippy, tests debug et release des deux crates réussis ; mutation de memoire-domain en huit parts, 593 mutants, 515 détectés, 78 non compilables, aucun survivant ; mutation de memoire-store, 15 mutants, 11 détectés, 4 non compilables, aucun survivant, après correction de deux survivants (messages d’erreur, mutant équivalent à l’ouverture) ; contrat de 86 tests d’intégration et 27 tests internes pour le domaine, 8 tests pour le stockage ; les deux régressions ciblées restent détectées.

Incrément 3.1b (commit dbfbd74), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE3.txt) : formatage, cohérence documentaire, clippy, tests debug et release réussis, aller-retour du dépôt complet dans les tests de propriétés compris ; mutation de memoire-domain sur le contrat seul en huit parts, 568 mutants, 493 détectés, 75 non compilables, aucun survivant, après correction de trois survivants (ordre et localisation des lignes d’effet) ; contrat de 83 tests d’intégration et 27 tests internes ; les deux régressions ciblées restent détectées.

Incrément 3.1 (commit 7a00942), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE3.txt) : formatage, cohérence documentaire, clippy, tests debug et release réussis, aller-retour textuel de chaque état publié compris ; mutation de memoire-domain sur le contrat seul en six parts, 433 mutants, 368 détectés, 65 non compilables, aucun survivant, après correction d’un survivant au premier passage ; contrat de 78 tests d’intégration et 27 tests internes ; les deux régressions ciblées restent détectées.

Version 0.3, fin de la phase 2 (incréments 2.6a à 2.7, commit 4f5e704), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE2.txt) : formatage, cohérence documentaire, clippy, tests debug et release réussis, tests de propriétés compris ; mutation de memoire-domain sur le contrat seul, en quatre parts couvrant tous les mutants, 330 mutants, 276 détectés, 54 non compilables, aucun survivant ; contrat de 74 tests d’intégration et 24 tests internes ; aucun défaut actif ; les deux régressions ciblées restent détectées. Seul l’état final a été vérifié : il contient tout le code des incréments 2.6a à 2.7. Les passages successifs ont trouvé huit survivants et un défaut réel (S12), tous corrigés par des tests ou des réécritures consignés dans l’historique ; chaque correction a relancé la vérification complète.


Version 0.2.6 (incrément 2.5) : contrôle repris après un redémarrage, dans une copie de travail figée au commit vérifié ; formatage, cohérence documentaire, clippy, tests debug et release réussis ; mutation de memoire-domain sur le contrat seul en quatre parts, 272 mutants, 226 détectés, 46 non compilables, aucun survivant, après correction d’un survivant détecté au premier passage (test interne complété) ; contrat de 63 tests d’intégration et 22 tests internes ; 1 défaut actif ; les deux régressions ciblées restent détectées.

Version 0.2.5 (incrément 2.4) : contrôle réalisé par étapes après deux redémarrages de l’environnement d’exécution (un seul cœur disponible) ; formatage, cohérence documentaire, clippy, tests debug et release réussis ; mutation de memoire-domain sur le contrat seul en trois parts couvrant tous les mutants, 236 mutants, 199 détectés, 37 non compilables, aucun survivant ; contrat de 58 tests d’intégration et 20 tests internes ; 1 défaut actif ; les deux régressions ciblées restent détectées.

Version 0.2.4 (incrément 2.3b) : mutation de memoire-domain sur le contrat seul, 183 mutants, 147 détectés, 36 non compilables, aucun survivant ; contrat de 52 tests d’intégration et 16 tests internes ; 3 défauts actifs ; les deux régressions ciblées restent détectées.

Version 0.2.3 (incrément 2.3a) : mutation de memoire-domain sur le contrat seul, 175 mutants, 141 détectés, 34 non compilables, aucun survivant ; contrat de 48 tests d’intégration et 14 tests internes ; 3 défauts actifs ; les deux régressions ciblées restent détectées.

Version 0.2.2 (incrément 2.2) : mutation de memoire-domain sur le contrat seul, 149 mutants, 120 détectés, 29 non compilables, aucun survivant ; contrat de 45 tests d’intégration et 11 tests internes ; 5 défauts actifs ; les deux régressions ciblées restent détectées.

Version 0.2.1 (incrément 2.1) : mutation de memoire-domain sur le contrat seul, 138 mutants, 109 détectés, 29 non compilables, aucun survivant ; contrat de 39 tests d’intégration et 11 tests internes ; les deux régressions ciblées historiques, réinjectées dans memoire-domain, restent détectées.

Version 0.1.2 (registre v0.2), contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE1.txt) : cohérence documentaire (61 invariants, 20 scénarios, 9 décisions, 11 défauts connus), formatage, clippy, tests debug et release, mutation sans survivant, régressions ciblées. Le code du noyau est identique à celui de la v0.1.1.

Version 0.1.1, contrôle complet par tools/check.sh (détail : VERIFICATION-PHASE0.txt).

- Compilateur : rustc 1.91.1, cible x86_64 Linux ; rust-version déclarée : 1.91.
- Formatage : réussi. Clippy avec avertissements bloquants : aucun avertissement.
- Tests en debug et en release : 8 internes, 29 d’intégration, 9 défauts connus ; aucun échec.
- Mutation (cargo-mutants 27.1.0) sur le contrat seul : 89 mutants, 63 détectés, 26 non compilables, aucun survivant. Référence v0.1.0 : 13 survivants sur 91.
- Les deux régressions ciblées historiques restent détectées par les tests prévus.
- Le test exhaustif couvrait 1 536 couples graphe/racine sur trois notes ; 192 depuis l’incrément 2.4 (graphes sans boucle).
- Ces résultats ne couvrent ni la persistance, ni la fusion, ni une interface, absentes du prototype.

Version 0.1.0 : vérifiée par son auteur avec rustc 1.99.1 (22 tests) ; cette vérification n’a pas été reproduite avec ce compilateur.
