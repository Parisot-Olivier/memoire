# Fiches de décision

Version 0.3 — 21 octobre 2026 — D1 à D9 (phase 1), D10 et D11 (avant l’incrément 2.3), D12 (avant l’incrément 2.3b).

Chaque fiche est arrêtée par l’utilisateur. Une décision ne change que par une nouvelle fiche qui la remplace explicitement ; l’ancienne reste ici avec le statut « remplacée par Dx ». Les invariants cités sont dans INVARIANTS.md, les scénarios dans SCENARIOS.md.

## D1 — Bifurcation automatique depuis main uniquement

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** le prototype crée une branche à chaque opération critique, y compris depuis une branche expérimentale : dix corrections de texte produisent dix branches (défaut S1). L’annulation franchit le point de bifurcation et laisse l’utilisateur sur une branche vide (S3).
- **Décision :** main est protégée. Une opération critique engagée sur main crée une branche ; engagée sur une branche expérimentale, elle s’y poursuit. Chaque branche enregistre son origine et son commit de base. L’annulation ne franchit pas la base ; annuler l’opération fondatrice ramène sur la branche d’origine, la branche restant consultable.
- **Option écartée :** nouvelle branche à chaque opération critique (cahier des charges v0.1, section 10).
- **Conséquences :** le cahier des charges devra être amendé sur ce point. Le résultat de chaque commande indique commit, branche active et bifurcation éventuelle.
- **Invariants :** INV-BR-02, INV-BR-03, INV-BR-04. **Scénarios :** S-BR-01, S-REF-01 (étapes 8 et 9).
- **Mise en œuvre :** memoire-domain, incrément 2.2. Rétablir après l’annulation d’une opération fondatrice rentre dans la branche quittée, et le refuse si elle a changé depuis.

## D2 — Les contextes se superposent au réseau

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** le prototype copie le graphe pour créer un contexte ; un même identifiant porte alors deux textes divergents, et une note créée après la copie n’appartient qu’à un contexte (S5).
- **Décision :** identité et révisions sont partagées. Un contexte porte l’appartenance, les statuts et l’activation des relations.
- **Précision du 20 octobre 2026 :** l’utilisateur suit aujourd’hui une seule cible (Evalbot, monocœur). Une exploration, comme le multicœur, se classe dans une catégorie (D7) ; un contexte n’est créé que pour une variante réellement suivie en parallèle.
- **Invariants :** INV-SCOPE-01, INV-SCOPE-02. **Scénario :** S-SCOPE-01.
- **Mise en œuvre :** incrément 2.3a. Le contenu de chaque note est conservé une fois ; chaque contexte le référence et la validation refuse toute divergence.

## D3 — Identifiants de 128 bits injectés par la commande

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** compteurs u64 locaux ; deux dépôts produisent le même repère pour deux notes différentes (S7). Le cahier des charges donne déjà un exemple d’UUID.
- **Décision :** UUID générés par la couche service et transmis dans la commande. Le domaine ne génère aucun identifiant.
- **Conséquences :** domaine déterministe, répétition d’une commande idempotente, repères sans collision.
- **Invariants :** INV-ID-03, INV-ID-04, INV-RETRY-01. **Scénario :** S-ID-01.

## D4 — SQLite fait autorité

- **Statut :** arrêtée le 20 octobre 2026.
- **Décision :** SQLite est l’autorité du réseau, en mode WAL avec synchronous=FULL. Les états résultants sont stockés, pas seulement les commandes : rejouer d’anciennes commandes sous des règles modifiées produirait un autre historique. Git ne sert qu’à un export textuel déterministe.
- **Option écartée :** Git comme moteur de stockage.
- **Invariants :** INV-DUR-01, INV-RETRY-01, INV-EXPORT-01, INV-REL-04.

- **Précision du 31 octobre 2026 :** l’utilisateur écarte Git pour l’instant, ni comme moteur ni comme miroir automatique. L’historique des idées reste dans la base SQLite, consultable par le logiciel ; l’export textuel reste disponible à la demande.

## D5 — Édition mineure ou substantielle

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** le prototype traite toute édition comme critique, mais une réécriture d’hypothèse ne met aucun dépendant en réexamen (S6).
- **Décision :** l’utilisateur déclare une révision mineure ou substantielle. Seule la seconde crée une cause de réexamen. Le type est enregistré dans tous les cas.
- **Invariants :** INV-MOD-01, INV-MOD-02. **Scénarios :** S-MOD-01, S-REF-01 (étape 9).
- **Confirmation du 21 octobre 2026 :** une correction mineure ne rouvre pas un réexamen déjà acquitté. La nature de la révision est déclarée par l’utilisateur au moment où il la fait, par exemple par un bouton de l’interface ; le domaine n’impose aucune valeur par défaut.
- **Mise en œuvre :** incrément 2.3a (commande Revise). Chaque relation mémorise l’empreinte de sens de sa destination à son activation.

## D6 — Aucune suppression : statut « plus retenue »

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** l’utilisateur exige que ses actions ne soient jamais destructives. Une commande « supprimer » a été envisagée, puis jugée inutile.
- **Décision :** il n’existe aucune commande de suppression ni de purge. Ce qui n’est plus voulu passe au statut de décision « plus retenue », réversible ; il déclenche le réexamen des dépendants. Une identité, une fois créée, reste résoluble dans toute la descendance de son commit, y compris après une restauration.
- **Conséquences :** la restauration globale du prototype doit être corrigée (S10). Les doublons se traitent par « remplace » (INV-MOD-06), pas par fusion d’objets.
- **Invariants :** INV-RET-01 à INV-RET-04. **Scénarios :** S-RET-01, S-RET-02, S-REF-01 (étape 8).
- **Mise en œuvre :** restauration non destructive dans memoire-domain, incrément 2.2 ; statut « plus retenue » réversible, incrément 2.3b.

## D7 — Catégories imbriquées pour organiser, statuts pour décider

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** l’utilisateur range ses explorations dans une catégorie « Exploration / évolution », avec une sous-catégorie « Exploration multicœur ».
- **Décision :** les catégories peuvent s’imbriquer. Un élément peut appartenir à plusieurs catégories. Une catégorie organise ; elle ne porte ni verdict ni statut et ne déclenche aucun réexamen.
- **Garde-fou :** un statut ne s’exprime jamais par une catégorie (« abandonné », « à revoir »). Le ranger ainsi contournerait le réexamen des dépendants, que seuls les statuts déclenchent.
- **Invariants :** INV-SEM-01, INV-REL-03. **Scénarios :** S-REF-01 (étape 2), S-REL-03.
- **Mise en œuvre :** incrément 2.4 ; un cycle de classement entre catégories est refusé.

## D8 — Un dépôt par projet, sans lien entre projets

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** le mini-RTOS et le projet SystemVerilog sont indépendants.
- **Décision :** un dépôt correspond à un projet. Aucune relation ne traverse deux dépôts. Les UUID (D3) laissent possible un import ultérieur sans collision.
- **Invariants :** INV-REF-01, INV-ID-03.

## D9 — Le retour arrière concerne les idées, jamais le code

- **Statut :** arrêtée le 20 octobre 2026.
- **Contexte :** l’utilisateur définit le logiciel comme « un Git pour ses idées ». Le code du RTOS sera versionné séparément par Git.
- **Décision :** annulation, restauration et branches portent sur le réseau d’idées. Le logiciel n’écrit jamais dans les fichiers de code et ne les restaure pas. Un ancrage conserve chemin, copie et empreinte du code ciblé ; la référence Git s’y ajoute lorsqu’elle existe.
- **Invariants :** INV-CODE-03, INV-CODE-04. **Scénarios :** S-CODE-01, S-CODE-03.

## D10 — Critère de criticité

- **Statut :** arrêtée le 20 octobre 2026, sur la réponse « Continuez » à la proposition de l’incrément 2.3. À infirmer avant l’écriture du contrat de 2.3 si elle ne convient pas.
- **Contexte :** le prototype fixe une liste de commandes critiques, dont toute édition de texte. D5 distingue désormais révision mineure et substantielle.
- **Décision :** une modification est critique si elle peut créer ou lever une cause de réexamen. Sont critiques : réfutation, passage à « plus retenue », retrait des ressources actuelles, révision substantielle, dépendance ou provenance, réattachement, restriction de périmètre, acquittement, restauration. Sont ordinaires : création, révision mineure, classement, lien thématique, « contredit », copie de contexte.
- **Conséquences :** la liste de is_critical devient la traduction de ce critère ; chaque nouvelle commande est classée en appliquant le critère, pas par analogie. Une modification critique engagée sur main crée une branche (D1).
- **Invariants :** INV-BR-01, INV-BR-02, INV-MOD-02.
- **Mise en œuvre :** incrément 2.3a pour les commandes existantes ; chaque commande ajoutée ensuite est classée selon ce critère.
- **Limite levée par D13 :** la criticité s’évalue désormais sur l’effet réel. Texte d’origine de la limite : la criticité est évaluée sur la commande, pas sur l’état. Revenir sur « plus retenue » ou sur une réfutation peut lever une cause, mais est classé ordinaire. Sans conséquence aujourd’hui : tout écart est critique et quitte main, si bien qu’un tel retour n’a lieu que sur une branche expérimentale, où la criticité ne crée plus de branche. La règle sera reprise avec la fusion (phase 6), quand des notes écartées pourront revenir sur main.

## D11 — Ressources actuelles

- **Statut :** arrêtée le 20 octobre 2026, dans les mêmes conditions que D10.
- **Contexte :** le cahier des charges définit les ressources actuelles comme les « éléments retenus ». Appliquée à la lettre, cette définition ferait disparaître de la vue de travail toute note fraîchement saisie, tant qu’elle n’est pas explicitement retenue.
- **Décision :** les ressources actuelles comprennent les notes proposées et retenues. En sont exclues les notes plus retenues, remplacées, réfutées, retirées des ressources actuelles, ou ayant une cause de réexamen non acquittée.
- **Conséquences :** amendement du cahier des charges (table ci-dessous).
- **Invariants :** INV-RET-02, INV-ACK-01.
- **Mise en œuvre :** incrément 2.3b.

## D12 — Motif obligatoire pour tout changement de statut

- **Statut :** arrêtée le 20 octobre 2026, sur la réponse « Tu peux passer à la suite » à la proposition de l’incrément 2.3b. À infirmer si l’exigence se révèle trop lourde à l’usage.
- **Contexte :** l’invariant INV-MOD-05 demande que chaque transition de statut porte un motif. Sans motif, le logiciel ne pourrait pas répondre plus tard à « pourquoi cette idée n’est-elle plus retenue ? ».
- **Décision :** un changement de décision, de vérification ou d’implémentation est refusé si son motif est vide ou ne contient que des espaces. Le motif est conservé avec le commit. Les autres modifications n’en exigent pas.
- **Conséquence :** chaque changement de statut demande une phrase à l’utilisateur.
- **Invariants :** INV-MOD-05. **Mise en œuvre :** incrément 2.3b.

## D13 — Criticité selon l’effet réel

- **Statut :** arrêtée le 21 octobre 2026 (option 1 proposée après l’incrément 2.6b).
- **Contexte :** évaluée sur le type de commande (D10), la criticité faisait quitter main dès la première dépendance déclarée, puisqu’une dépendance « peut » créer une cause. Toute session de travail se serait déroulée sur une branche expérimentale jusqu’à la fusion, et les étapes 8 et 9 de S-REF-01 ne pouvaient pas passer.
- **Décision :** une modification est critique si, dans l’état courant, elle crée ou lève effectivement une cause de réexamen, écarte une note (réfutation, « plus retenue », retrait des ressources actuelles, remplacement) ou restaure un état antérieur. Engagée sur main, elle crée une branche (D1).
- **Options écartées :** classement par type de commande (D10 d’origine) ; amendement des résultats attendus de S-REF-01.
- **Conséquences :** déclarer une dépendance vers une note saine, réviser une note dont rien ne dépend ou restreindre une affirmation sans dépendant restent ordinaires. L’aperçu annonce la décision (Preview::forks) ; la fonction publique is_critical disparaît. La limite connue de D10 sur le retour d’une note écartée est levée.
- **Invariants :** INV-BR-01, INV-BR-02. **Scénarios :** S-BR-01, S-REF-01 (étapes 8 et 9). **Mise en œuvre :** incrément 2.6c.

## D14 — Politique de dépendances

- **Statut :** arrêtée le 21 octobre 2026, avant la phase 3.
- **Contexte :** la persistance (D4) et la génération des identifiants (D3) demandent des bibliothèques éprouvées : rusqlite, qui embarque SQLite, et uuid.
- **Décision :** le domaine reste sans aucune dépendance, ce que vérifie la garde d’architecture. Les couches stockage et service peuvent employer rusqlite et uuid, à des versions figées par Cargo.lock et publiées depuis au moins deux semaines. Toute autre dépendance passe par une nouvelle fiche. Un contrôle d’audit des dépendances rejoint le contrôle complet.
- **Mise en œuvre (incrément 3.2) :** `tools/deps.py` contrôle l’âge et la présence de chaque paquet verrouillé dans `DEPENDANCES.md` ; `cargo audit --deny warnings` (cargo-audit 0.22.2) contrôle les alertes RustSec ; les deux étapes font partie du contrôle complet.
- **Invariants :** INV-DUR-01, INV-ID-04, INV-CODE-04. **Mise en œuvre :** phase 3.

## D15 — Format textuel maison, documenté

- **Statut :** arrêtée le 21 octobre 2026, avant l’incrément 3.1.
- **Contexte :** stockage (D4) et export pour Git demandent une représentation textuelle des états. Options : format maison sans dépendance, ou serde et serde_json, qui exigeraient une fiche au titre de D14 et un travail pour garantir le déterminisme.
- **Décision :** format maison, déterministe, à forme canonique unique, spécifié dans FORMAT.md de façon assez précise pour corriger un problème ou faire évoluer le logiciel sans lire le code. Le domaine reste sans dépendance.
- **Conséquences :** davantage de code à écrire et à tester ; en contrepartie, le même format sert au stockage et à l’export, son évolution est versionnée avec migrations testées (FORMAT.md, section 10).
- **Invariants :** INV-EXPORT-01, INV-DUR-01. **Mise en œuvre :** incrément 3.1.

## D16 — Versions des règles reportées au premier changement de règle

- **Statut :** arrêtée le 24 octobre 2026, à la fin de l’incrément 3.3.
- **Contexte :** INV-REL-04 et S-REL-04 demandent qu’une relation reste jugée selon les règles en vigueur à sa création, même si ces règles changent ensuite. Une seule version des règles existe aujourd’hui.
- **Décision :** tout dépôt au format `memoire-depot 1` est jugé selon les règles version 1. Le versionnement des règles, avec sa migration et son test sur un dépôt figé de l’ancienne version, est introduit dans la même modification que le premier changement de règle, une fois le moteur de traitement prêt.
- **Option écartée :** construire dès maintenant une infrastructure de versions sans règle à y faire varier, donc sans cas réel pour la tester.
- **Invariants :** INV-REL-04. **Scénarios :** S-REL-04.

## D17 — Interface graphique minimale en Tkinter, pilotant l’exécutable

- **Statut :** arrêtée le 25 octobre 2026, à la demande de l’utilisateur (boutons transcrivant les commandes, invites pour les arguments, lancement en une commande).
- **Options :** bibliothèque graphique Rust (egui, iced : plusieurs dizaines de dépendances, fiche D14 et audit à chaque mise à jour) ; page web servie localement (serveur HTTP écrit à la main, surface d’attaque) ; Tkinter, inclus dans la bibliothèque standard de Python.
- **Décision :** Tkinter, dans `gui/`, sans aucune dépendance supplémentaire. L’interface ne contient aucune règle : elle lance l’exécutable `memoire` et construit ses formulaires depuis `memoire aide` (INTERFACE.md).
- **Conséquences :** Python 3 et Tkinter deviennent des prérequis de l’interface, pas du logiciel ; la ligne de commande reste utilisable seule. Une interface plus riche pourra remplacer celle-ci sans toucher au noyau.

## D18 — Échanges et citations dans le réseau de dépendances

- **Statut :** arrêtée le 31 octobre 2026, sur proposition faite après les retours d’usage R8 et R10.
- **Contexte :** les règles réservent « dépend de » et « développé à partir de » aux notes qui affirment quelque chose ; un échange, simple source, ne peut ni dépendre d’un autre ni être l’appui d’une note entière. L’utilisateur veut relier des conversations entre elles (une conversation née d’une question soulevée dans une autre) et faire dépendre ses notes d’un échange.
- **Décision :**
  1. « développé à partir de » est permis d’un échange vers un échange, et d’une note vers un échange ;
  2. « dépend de » est permis d’une note vers un échange entier, par choix explicite ;
  3. toute citation crée, dans la même opération, un lien « développé à partir de » vers sa source (échange ou note), visible et retirable ;
  4. un échange peut être retiré des ressources actuelles ; ses dépendants passent alors à réexaminer ; un échange ne porte pas de verdict ;
  5. « citation » désigne toute sélection, dans un échange comme dans une note (R8) ; la commande `passage` devient `citer`.
- **Conséquences :** les règles sont assouplies, jamais durcies : tout dépôt existant reste valide, le versionnement des règles (D16) n’est pas déclenché. Citer une note change le format : version 2 de FORMAT.md, avec la première migration réelle (décodeur de la version 1 conservé, textes figés de la version 1 relus). Un ancien exécutable refuse proprement un dépôt de version 2.
- **Invariants :** INV-REL-04, INV-SOURCE-01, INV-EXPORT-01. **Retours :** R8, R10.

## D19 — Fusion à trois voies par objet ; sauvegarde avant fusion ; abandon d’une branche

- **Statut :** arrêtée le 31 octobre 2026 (FUSION.md, version 0.2).
- **Décision :** fusion champ par champ depuis la base commune ; texte révisé des deux côtés : choix d’une version entière ; conflits structurels résolus par la désactivation d’une relation ; préparation, résolutions, application ; commit à deux parents, une seule opération annulable. Exception à D1 : une fusion vers main ne bifurque jamais. Une sauvegarde automatique précède toute fusion. Une branche peut être abandonnée, sans suppression, et réactivée.
- **Option écartée :** fusion des textes ligne à ligne.
- **Précision :** la sauvegarde avant fusion est un filet de sécurité, pas un moyen d’alléger la base : l’historique complet reste dans la base courante (D6) ; la performance est traitée par le stockage (VERSION2.md).
- **Invariants :** INV-MERGE-01 à 03, INV-BR-01, INV-BR-02. **Critères :** CA-36, CA-37.

## D20 — Égalité des dépôts : contrat sur l’historique en mémoire, SQLite vérifié par équivalence

- **Statut :** arrêtée le 6 novembre 2026, validée par l’utilisateur avec ETAT-HISTORIQUE.md version 0.2.
- **Contexte :** le contrat compare souvent des dépôts entiers (`let control = r.clone(); … assert_eq!(r, control)`), historique compris : c’est ce qui prouve qu’un refus ne laisse aucune trace (INV-TX-01), qu’un rejeu redonne le même dépôt (INV-ID-04), qu’un export se relit à l’identique (INV-EXPORT-01). Une fois l’historique sorti de la mémoire (incrément 5.2), copier un dépôt ne copierait plus son historique, et une écriture faite à tort dans l’historique échapperait à la comparaison.
- **Décision :** `Repository<H: History = MemoryHistory>`. Avec l’historique en mémoire, `Clone` et `PartialEq` restent ceux d’aujourd’hui et comparent tout ; le contrat du domaine garde ses assertions. L’historique SQLite n’a ni `Clone` ni `PartialEq` ; il est vérifié par équivalence avec l’historique en mémoire après chaque commande (mêmes résultats, même texte canonique v2, y compris après réouverture). Les tests du stockage et du service qui comparaient deux dépôts comparent leurs textes canoniques complets (contrat modifié, de même force).
- **Options écartées :** égalité définie par le texte canonique partout, contrat du domaine compris (même force, mais contrat et mutation plus lents, diagnostics moins lisibles) ; égalité limitée à la partie en mémoire (affaiblit le contrat sans que cela se voie).
- **Conséquences :** un paramètre de type sur `Repository`, caché par un alias dans le service et la ligne de commande ; un risque de divergence entre les deux historiques, couvert par les tests d’équivalence et par la mutation de `memoire-store`.
- **Invariants :** INV-TX-01, INV-ID-04, INV-EXPORT-01, INV-DUR-01.

## D21 — Vérifications à l’ouverture d’une base ; vérification complète sur demande

- **Statut :** arrêtée le 6 novembre 2026, validée par l’utilisateur avec ETAT-HISTORIQUE.md version 0.2.
- **Contexte :** chaque ouverture relit et revalide tout l’historique (STOCKAGE.md, section 2 : « une seule voie de lecture et de validation »). C’est la cause principale de la lenteur : 2,81 s d’ouverture à 1 205 opérations, pour 0,05 s d’exécution.
- **Décision :** à l’ouverture, schéma, format, métadonnées des branches et état courant de la branche active, lu et validé en entier ; tout état ancien est validé à sa lecture (annuler, basculer, restaurer), chaque ligne retirée par une différence devant être exactement présente ; la vérification complète de tout l’historique a lieu à l’import, après une migration, à l’export et par la commande `memoire verifier`.
- **Conséquence assumée :** une corruption d’un commit ancien, faite hors du logiciel, n’est plus détectée à l’ouverture, mais à la première lecture de cet état ou par `memoire verifier` ; elle reste toujours refusée, jamais ignorée.
- **Option écartée :** vérification complète à chaque ouverture (durée proportionnelle à l’historique : objectifs de VERSION2.md inatteignables).
- **Invariants :** INV-DUR-01, INV-EXPORT-01.

## D22 — Opérations groupées : une opération, un commit ; criticité sur l’effet net

- **Statut :** arrêtée le 6 novembre 2026, validée par l’utilisateur avec LOT.md version 0.1.
- **Contexte :** R1 (« créer une note et la lier dans le même geste ») ; VERSION2.md, étape 3 ; INVARIANTS.md, section 5 : « l’API de lot devra valider toutes les étapes avant publication et créer une seule branche pour l’ensemble ».
- **Décision :** le noyau reçoit une commande générique `lot`, suite de modifications élémentaires appliquées dans l’ordre sur un état de travail, toutes validées avant toute écriture ; un lot accepté publie un seul commit et une seule opération, qu’un seul `annuler` défait ; sa criticité (D13) se juge sur l’effet net de l’état courant à l’état final, et un lot critique sur main crée une seule branche. La ligne de commande l’expose d’abord par `note creer` avec des options de relation, la note créée étant l’origine de chaque relation.
- **Options écartées :** criticité « dès qu’un élément le serait seul » (confirmations et branches sans effet réel, contraire à D13) ; commande `memoire lot FICHIER` dès 5.3 (syntaxe de désignation des notes créées à concevoir, inutile pour R1 ; reportée).
- **Invariants :** INV-LOT-01, INV-LOT-02, INV-LOT-03, INV-TX-01, INV-PREVIEW-01.

## D23 — Fusion : tête de la source dans la commande, base commune unique, abandon hors annulation, code de plan

- **Statut :** arrêtée le 6 novembre 2026, validée par l’utilisateur avec FUSION.md, section 9 (version 0.3).
- **Contexte :** mise en œuvre de D19 (incrément 5.4) ; points que FUSION.md (version 0.2) et VERSION2.md laissaient ouverts.
- **Décision :** la commande de fusion porte la tête de la source sur laquelle le plan a été calculé (`tete <commit>` dans le format v2) et une fusion dont la source ou la cible a bougé est refusée comme périmée ; une forme de résolution s’ajoute (`acquittement`) — la forme `relation`, d’abord proposée, est retirée le même jour avec l’accord de l’utilisateur : une relation n’a que son activité de modifiable, à deux valeurs, et toute relation présente des deux côtés est dans la base, si bien qu’aucun conflit n’y est possible ; une fusion dont les deux têtes ont plusieurs ancêtres communs les plus proches est refusée ; `annuler` ne défait ni un abandon ni une réactivation, qui se défont l’une l’autre, et une branche abandonnée n’est ni active ni source tant qu’elle n’est pas réactivée ; `fusionner` exige le code de plan affiché par `fusion plan`, qui garantit que les numéros de conflit désignent ceux que l’utilisateur a lus.
- **Options écartées :** fusion sur la tête courante de la source (fusion d’un état non vu) ; choix arbitraire ou base virtuelle récursive parmi plusieurs bases (complexité et résultat difficile à expliquer pour un cas rare) ; abandon comme commit annulable (un abandon n’est pas un changement d’état d’une branche) ; résolutions désignées par des numéros sans code de plan (numéros pouvant désigner d’autres conflits que ceux lus).
- **Invariants :** INV-MERGE-01 à 03, INV-BR-01, INV-BR-02, INV-PREVIEW-01, INV-MOD-04.

## D24 — Citations et orphelines : lien créé par la commande, notes citables, orphelines hors des ressources actuelles

- **Statut :** arrêtée le 6 novembre 2026, validée par l’utilisateur avec CITATIONS.md version 0.1.
- **Contexte :** mise en œuvre de D18 (citations, échanges dans les dépendances) et de R6 (orphelines) à l’incrément 5.5.
- **Décision :** le lien « développé à partir de » d’une citation vers sa source est créé par la commande `citer`, dans un lot (D22), et non imposé par le noyau, puisqu’il peut être retiré ensuite ; sont citables les raisonnements, implémentations et notes personnelles, à leur version courante pour la ligne de commande, et les parties d’un échange ; les réserves ne naissent que d’une citation d’échange ; `passage` devient `citer` sans alias, et le rôle s’affiche « citation » ; une note dont tous les appuis déclarés sont écartés et dont les causes sont acquittées est orpheline et **sort des ressources actuelles**, ce qui complète D11 ; une commande `retirer` archive une note ou un échange.
- **Options écartées :** lien imposé par le noyau (sans garantie, puisqu’on peut le retirer ; journal existant à réinterpréter) ; alias `passage` (deux noms pour un geste) ; orphelines restant ressources actuelles (contraire à R6).
- **Invariants :** INV-CIT-01, INV-CIT-02, INV-ORPH-01, INV-REL-04. **Retours :** R6, R8, R10.

## D25 — Visionneur en Tkinter, avec un rendu Markdown écrit pour le projet

- **Statut :** arrêtée le 6 novembre 2026, validée par l’utilisateur avec VISIONNEUR.md version 0.1. Confirme D17.
- **Contexte :** R2 et R8 demandent un visionneur qui rende le Markdown collé depuis une IA et permette de citer une sélection ; l’utilisateur avait autorisé à remplacer Tkinter s’il était insuffisant.
- **Décision :** l’interface reste en Tkinter, sans dépendance ; un module `gui/rendu.py`, en Python pur, rend un sous-ensemble de Markdown et tient la correspondance entre chaque caractère affiché et sa position en octets dans la source ; une sélection se cite comme la plus petite plage de la source qui la contient ; trois ajouts à la ligne de commande (`source`, `citations`, `citer --plage`) donnent à l’interface ce qu’elle affiche, sans règle propre.
- **Options écartées :** PySide6 (rendu Markdown natif, mais sans correspondance avec les positions de la source, plus de 100 Mo de dépendances à auditer, D14) ; page web locale (serveur à écrire, correspondance à écrire en JavaScript). PySide6 reste l’alternative si l’usage montre Tkinter insuffisant.
- **Conséquences :** tableaux, images et formules s’affichent en texte brut ; la logique du rendu, non soumise à la mutation, est concentrée dans un module testé en détail.
- **Invariants :** INV-SOURCE-02, INV-VIEW-01.

## D26 — Point de reprise : opération sans changement du réseau ; état du code lu par l’exécutable Git

- **Statut :** arrêtée le 9 novembre 2026, validée par l’utilisateur avec FICHE-REPRISE.md version 0.1.
- **Contexte :** le cahier des charges (sections 3, 8 et 10) demande une fiche de reprise qui conserve son contexte historique, la référence du code et un moyen de retrouver le travail non enregistré dans un commit, et qui signale à la réouverture si le code a changé ; le logiciel n’écrit jamais dans le code (D9).
- **Décision :** une fiche est une commande du journal (`Command::Resume`) qui publie un commit d’état identique à son parent ; ce commit est son identité et son contexte historique ; elle figure dans l’historique, sert de point de retour à `restaurer`, et `annuler` la défait comme toute opération ; la ligne de commande lit l’état du code par l’exécutable `git`, en lecture seule (`--no-optional-locks`, `core.fsmonitor=false`, ni stash ni référence), et le transmet au noyau comme une donnée, avec le travail non enregistré sous forme de patch jusqu’à 4 Mo s’il est en UTF-8, sinon sa taille et son empreinte seulement ; la date de la fiche est une donnée de la commande, en UTC à la minute (D3) ; la comparaison de l’état noté et de l’état actuel est une règle du noyau.
- **Options écartées :** fiche dans l’état du réseau (soumise à la restauration, aux branches et à la fusion, alors qu’elle décrit une session) ; journal à part, hors de l’annulation (changement de schéma, migration, nouveau chemin d’export) ; bibliothèque Git (git2, gix : dépendance lourde au titre de D14) ; `git stash` (crée des références, donc écrit dans le dépôt Git).
- **Conséquences :** aucun changement de schéma ; l’export emporte les fiches et le patch noté ; un `annuler` juste après une fiche défait la fiche, ce que sa sortie dit ; les essais de la ligne de commande demandent l’exécutable `git`.
- **Invariants :** INV-RESUME-01, INV-RESUME-02, INV-DUR-01.

## D27 — Terminal intégré : le shell de l’utilisateur sous pseudo-terminal, premier panneau du plan de travail, fin de commande signalée par une marque d’invite

- **Statut :** arrêtée le 10 novembre 2026, validée par l’utilisateur avec TERMINAL.md version 0.3.
- **Contexte :** R5, précisé par l’utilisateur : le shell du système dans le logiciel, flottant ou incrusté en haut à droite, redimensionnable, avec une notification à la fin d’une compilation ; apparence (direction Liquid Glass) renvoyée à une phase ultérieure. Environnement : Linux, Wayland, sans `xterm` ; `notify-send` et le service D-Bus des notifications présents.
- **Décision :** le shell de l’utilisateur tourne dans un pseudo-terminal (bibliothèque standard de Python), moteur sans affichage séparé de la vue ; le terminal est un panneau du plan de travail (D28) ; la fin de chaque commande est repérée par une marque invisible ajoutée à `PROMPT_COMMAND` après le chargement de `~/.bashrc` (fichier de démarrage temporaire, supprimé à l’arrêt) ; une commande d’au moins 10 secondes terminée hors de la vue est notifiée par le service de notifications du bureau ; `memoire` y vise la base de la fenêtre ; pas d’émulateur complet (`TERM=dumb`, pagers désactivés, couleurs retirées).
- **Options écartées :** `xterm -into` (absent, installation par `rpm-ostree`, X11) ; émulateur VT100 complet (hors de proportion) ; mini-shell propre (moins utile) ; fin de commande devinée d’après la sortie, ou commande enveloppée (visible, peu fiable) ; bibliothèque Python de notifications (dépendance inutile).
- **Conséquences :** aucune dépendance nouvelle ; Linux et macOS seulement ; fin de commande suivie pour bash seulement ; code Python hors mutation, logique concentrée dans le moteur testé.
- **Invariants :** INV-TERM-01, INV-TERM-02.

## D28 — Plan de travail modulaire : panneaux ancrables ou flottants, modèle séparé de la réalisation, disposition propre à l’utilisateur

- **Statut :** arrêtée le 10 novembre 2026, validée par l’utilisateur avec PLAN-DE-TRAVAIL.md version 0.1.
- **Contexte :** l’utilisateur veut un plan de travail aussi modulaire que possible, à la manière de Vivado ; l’apparence sera conçue avec un designer d’interface.
- **Décision :** autour d’une zone centrale fixe, quatre zones d’ancrage (haut, bas, gauche, droite) empilent des panneaux redimensionnables, qui peuvent aussi flotter ou être masqués ; un modèle sans affichage (`Disposition`) est la seule source de vérité ; la réalisation Tkinter passe un panneau d’ancré à flottant sans le recréer (`wm manage`, `wm forget`, essayé sous Tk 9.0.2) ; la disposition est conservée dans `$XDG_CONFIG_HOME/memoire/plan-de-travail.json`, hors du dépôt, avec retour à la disposition par défaut si le fichier est absent ou invalide.
- **Options écartées :** fenêtres séparées seulement ; bibliothèque d’ancrage existante (PySide6 et Qt Advanced Docking System : changement de bibliothèque et dépendance lourde, D14, avant le besoin visuel précis) ; disposition dans la base (pas une donnée du projet).
- **Conséquences :** le terminal est le premier panneau, les autres vues suivront ; un fichier de configuration dans `~/.config/memoire/`.
- **Invariants :** INV-PLAN-01.

## D29 — Plusieurs terminaux : un panneau par terminal, créé à la demande, nommé par ce qu’il fait, géré depuis un panneau « Terminaux »

- **Statut :** arrêtée le 10 novembre 2026, validée par l’utilisateur avec TERMINAUX.md version 0.1.
- **Contexte :** l’utilisateur veut plusieurs terminaux, un menu et un panneau plaçable qui les liste, nommés par la commande qu’ils exécutent et non par un identifiant, et qu’on met devant ou derrière facilement ; le plan de travail (D28) ne connaissait que des panneaux fixes.
- **Décision :** chaque terminal est un panneau dynamique du plan de travail, avec son shell ; son nom est calculé par une fonction pure d’après la commande en cours, sinon le shell et son dossier (transmis par la marque de fin, qui porte désormais le dossier), doublons numérotés ; un panneau « Terminaux » et un menu « Terminaux ▾ » listent les terminaux et permettent d’afficher, mettre devant ou derrière, masquer, fermer et en ouvrir un nouveau ; la liste des terminaux et leurs dossiers sont conservés (`terminaux.json`), pas leurs commandes ni leurs sorties.
- **Options écartées :** onglets dans un seul panneau (ni côte à côte, ni un seul flottant) ; nom choisi à la main (la demande est un nom qui dit ce que fait le terminal) ; commande lue dans `/proc` (propre à Linux, moins fidèle que la commande saisie).
- **Conséquences :** panneaux dynamiques dans le modèle du plan de travail ; second fichier de configuration ; marque de fin avec le dossier.
- **Invariants :** INV-TERM-03, INV-TERM-01.

## D30 — Interface : un catalogue d’intentions en vocabulaire simple, distinct des commandes

- **Statut :** arrêtée le 10 novembre 2026, validée par l’utilisateur avec ACTIONS.md version 0.2 (« adopter une piste » au lieu d’« intégrer »).
- **Contexte :** les formulaires de l’interface étaient générés depuis l’aide de la ligne de commande (D17) et en reprenaient le jargon et l’organisation à plat : difficiles à suivre, et à réapprendre après plusieurs mois. L’utilisateur demande des intentions simples, une création progressive, un enchaînement en graphe, et autorise un autre vocabulaire pour l’interface.
- **Décision :** l’interface présente un catalogue d’actions rangées par sujet, en vocabulaire simple (projet, piste, étape, idée, solution technique, note libre, thème, s’appuie sur, vient de, garder quand même, adopter une piste…), avec des questions courtes, des sections à cocher et des suites « Et ensuite ? » ; le catalogue traduit vers les commandes, qui gardent leurs termes ; sa cohérence avec les commandes est vérifiée par test contre l’aide figée ; le formulaire généré reste en mode expert ; deux sorties `--tsv` (pistes avec un nom lisible, plan d’adoption) donnent à l’interface ce qu’elle doit expliquer.
- **Options écartées :** renommer les commandes (habitudes, scripts, documentation) ; retoucher seulement les libellés des formulaires générés (ni organisation, ni fusion par numéros, ni échange par fichiers réglés) ; lire la sortie lisible de `fusion plan` (fragile).
- **Conséquences :** deux vocabulaires reliés par une table unique et testée ; D17 précisée : l’interface reste sans règle métier, mais n’est plus seulement le reflet de l’aide.
- **Invariants :** INV-UI-01.

## D31 — Apparence : un thème dessiné en Python pur, sur Tkinter, d’après le modèle de l’utilisateur

- **Statut :** arrêtée le 14 novembre 2026, avec la maquette validée par l’utilisateur et APPARENCE.md version 0.1.
- **Contexte :** le moteur terminé, l’utilisateur passe au design (REPRISE.md, section 7 : « apparence du plan de travail et du terminal avec un designer d’interface »). Son modèle Figma (dégradé, carte crème, pastilles de verre à ombre courte) donne une « technicité douce » qu’il veut retrouver. Tkinter n’a ni transparence par widget, ni flou d’arrière-plan, ni lissage des formes du canevas sous X11.
- **Décision :** rester en Tkinter ; dessiner les formes en images PNG calculées en Python pur (`gui/dessin.py`, aucune dépendance), posées sur un canevas de fond (le canevas compose la transparence des images) et utilisées comme éléments d’un thème ttk « memoire » ; ne pas imiter le flou (inutile sur un dégradé lisse) ; garder le texte sur le papier ou sur un verre givré ; séparer couleurs de sens (Okabe-Ito) et couleurs de décor ; vérifier les contrastes par test.
- **Options écartées :** passer à une interface web (Tauri, en Rust) : vrai flou et animations, mais réécriture de l’interface et de ses essais, et des centaines de crates à auditer (D14) ; ajouter Pillow pour dessiner : dépendance Python nouvelle pour un calcul simple ; dessiner les formes avec les primitives du canevas : crénelées sous X11, sans transparence ; installer des polices : le logiciel n’écrit pas dans le système, il choisit parmi celles présentes.
- **Conséquences :** l’apparence est une couche sans règle, testée sans affichage (dessin, contrastes, lecture de l’état) ; la fenêtre principale est recomposée (en-tête, rail, carte, barre d’état, recherche), le plan de travail passe dans la carte ; un essai graphique change (le menu n’est plus un arbre). Un passage à une interface web reste possible plus tard, avec sa propre fiche.
- **Invariants :** INV-LOOK-01, INV-LOOK-02, INV-UI-01.

## D32 — Console : un seul bloc en mosaïque, sur le côté, en pleine page ou en fenêtre

- **Statut :** arrêtée le 16 novembre 2026, validée par l’utilisateur sur maquette interactive avec CONSOLE.md version 0.1. **Remplace D29** pour la place des terminaux ; le nommage, la confirmation de fermeture et la conservation des dossiers de D29 restent.
- **Contexte :** avec un panneau par terminal (D29) et les onglets de 8.4, deux terminaux ancrés à droite ne montraient plus qu’une ligne de sortie chacun, sous trois rangées d’onglets. L’utilisateur veut plusieurs terminaux sur une même page, en créer ou en rouvrir un d’un geste, les agrandir facilement avec un point milieu, et garder les deux usages : sur le côté pour taper vite, en pleine page pour travailler. Règle donnée : une action est simple et efficace, jamais une combinaison de menus.
- **Décision :** la console est un seul panneau du plan de travail, qui contient tous les terminaux. Elle est sur le côté (un terminal à la fois, les autres en pastilles), en pleine page (nouvel état « page » d’un panneau, posé sur toute la carte), détachée ou masquée. En pleine page et en fenêtre, une mosaïque montre de un à quatre terminaux ; un modèle sans affichage (`gui/mosaique.py`) tient la page, l’arrière-plan, le terminal actif et le point milieu, dont le déplacement redimensionne tous les terminaux ; aucune part sous un quart. Un clic crée un terminal, un clic sur une pastille en rouvre un ; page pleine, l’arrivant prend la place du terminal actif, qui continue en arrière-plan. Le seuil de notification devient un réglage de la console.
- **Options écartées :** garder un panneau par terminal et ajouter une grille par-dessus (deux systèmes de placement pour le même objet, et les onglets restent) ; un menu « + » à deux niveaux (créer ou choisir un terminal existant) : deux gestes là où des pastilles toujours visibles en demandent un et montrent l’état ; des séparations indépendantes par rangée, comme un gestionnaire de fenêtres en pavage : plus souple, mais le point milieu unique demandé n’existe plus ; maximiser une tuile en cachant les autres : la demande est que les autres se redimensionnent ; plus de quatre terminaux sur la page : tuiles illisibles à la taille par défaut de la fenêtre.
- **Conséquences :** les panneaux `terminal-N` et `terminaux`, les onglets et le menu « Terminaux ▾ » disparaissent ; une disposition enregistrée avant 8.5 ne retrouve pas la place des anciens panneaux, une fois ; on ne fait plus flotter un terminal seul, mais la console ; `terminaux.json` reçoit une clé `console` ; les essais graphiques des terminaux visent la console (contrat modifié).
- **Invariants :** INV-TERM-04, INV-TERM-01, INV-TERM-03, INV-PLAN-01.

## Amendements à reporter dans le cahier des charges

Ces amendements, complétés de D13, sont reportés dans CAHIER_DES_CHARGES.md version 0.2 (21 octobre 2026), à la demande de l’utilisateur. La table reste ici comme trace ; les titres de sections sont ceux de la v0.1 :

| Section du cahier des charges | Texte actuel | Amendement proposé | Décision |
|---|---|---|---|
| Branches automatiques pour les modifications critiques | Depuis une branche expérimentale, la nouvelle branche critique part de celle-ci | Les opérations critiques se poursuivent sur la branche expérimentale ; seule main déclenche une bifurcation | D1 |
| Idées, décisions et validité | État de décision : proposée, retenue, abandonnée, remplacée | Remplacer « abandonnée » par « plus retenue », statut réversible ; aucune suppression | D6 |
| Annulation et restauration simples | Une fiche de reprise peut associer les références du réseau et du code pour une restauration coordonnée | La fiche conserve la référence du code ; la restauration du code reste une action de l’utilisateur dans Git | D9 |
| Placement dans le réseau de conversations | Cinq actions de placement | Ajouter « Restreint le périmètre » et « Contredit », sans effet sur la validité de la cible | INV-REL-06, INV-REL-07 |
| Catégories, raisonnements et idées d’implémentation | Rôle des catégories | Préciser que les catégories peuvent s’imbriquer et ne portent jamais de statut | D7 |
| Critères d’acceptation | Numérotation fondue dans celle des sections | Adopter les identifiants CA-01 à CA-40 de TRACABILITE.md | — |
| Finalité | Projets techniques complexes, dont un mini-RTOS et un processeur | Un dépôt par projet, sans lien entre dépôts | D8 |
| Réexamen et réutilisation des sous-graphes | « Ressources actuelles » : éléments retenus pour la variante | Éléments proposés ou retenus, hors éléments écartés ou à réexaminer | D11 |
| Idées, décisions et validité | États de décision, de vérification et d’implémentation | Préciser que chaque changement d’état exige un motif, conservé dans l’historique | D12 |
