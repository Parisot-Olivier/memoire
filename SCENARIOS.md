# Scénarios de référence

Version 0.2 — 20 octobre 2026 — phase 1.

Les résultats attendus sont fixés avant toute implémentation. Ils décrivent des commandes et des observations métier, indépendantes de l’API Rust. La phase 2 les traduira dans un harnais de scénarios sans modifier les résultats attendus. Un résultat attendu ne change que par une fiche de décision (DECISIONS.md).

Avancement (fin de la phase 2, incrément 2.7) : S-REF-01 s’exécute en entier, étapes 1 à 9, sur les sources réelles figées. S-MOD-01, S-MOD-02, S-MOD-04, S-PREVIEW-01, S-BR-01, S-RET-02, S-SCOPE-01, S-REL-01, S-REL-02, S-REL-03, S-SRC-01, S-RES-01, S-CODE-01, S-CODE-02 et S-CODE-03 sont couverts par le contrat de memoire-domain ; S-MOD-03 l’est par construction ; S-ID-01 hors import ; S-RET-01 sur une suite représentative et par les tests de propriétés (incrément 2.7) ; S-REL-04 attend le schéma persistant (phase 3).

Depuis l’incrément 4.2, S-REF-01 est aussi rejoué en entier par la ligne de commande (crates/memoire-cli/tests/recette.rs), sur les mêmes sources figées.

Précision d’exécution de S-REF-01 : « citer » un passage est réalisé par la relation « dépend de » (s’appuie sur) ; une note doit être réexaminée si une source qu’elle cite est corrigée. Sous D13, les étapes 1 à 7 restent entièrement sur main.

Conventions : les objets reçoivent des noms symboliques (C- catégorie, E- échange, N- note, P- passage, R- réserve). Leurs identifiants réels sont des UUID fournis par la couche service (D3). Sauf mention contraire, chaque scénario part d’un dépôt vide, sur main, avec un seul contexte.

## S-REF-01 — Du document RTOS à un état explicite

Scénario validé par l’utilisateur le 20 octobre 2026.

**Entrées :** fixtures/rtos-2026-10/doc.md et driver.c, figés octet pour octet (empreintes : SHA256SUMS ; intégrité vérifiée par tests/scenario_fixtures.rs).

**Objet :** le document est une donnée d’entrée, pas un modèle. Il illustre la difficulté à fixer ses choix : cinq échanges copiés dans l’ordre, une conclusion implicite (« la dernière réponse fait foi »), des réserves dispersées, un renvoi du code vers un titre de conversation. Le scénario mesure la transformation de ce matériau en état explicite, puis la reprise de travail à partir du code.

### Étape 1 — Saisie des échanges

Quand l’utilisateur colle les cinq échanges E1 à E5, en séparant explicitement question et réponse,

alors :
1. cinq échanges complets existent, ordonnés E1 → E5 ; aucun n’est découpé sur les lignes « * … » des réponses (INV-SOURCE-04) ;
2. chaque question et chaque réponse est une source conservée octet pour octet ; la réponse E1 conserve en particulier ses tabulations et la séquence U+2060 U+FFFC qui suit « 01-LM3S9B92-DataSheet.pdf » (INV-SOURCE-03) ;
3. l’ordre E1 → E5 ne crée aucune relation et ne remplace aucun passage (INV-REL-05).

### Étape 2 — Organisation

Quand l’utilisateur crée les catégories C-DRV « Driver GPIO : réveil des tâches », C-EXP « Exploration / évolution » et C-MC « Exploration multicœur » classée dans C-EXP, puis classe E1 et E2 dans C-DRV, E3 dans C-DRV et dans C-MC, E4 et E5 dans C-MC,

alors :
1. E3 appartient aux deux catégories sans duplication de contenu ;
2. aucun classement ne modifie un statut ni ne crée de cause de réexamen (INV-SEM-01) ;
3. classer C-EXP dans C-MC est refusé, sans aucun effet sur le dépôt (INV-REL-03, INV-TX-01).

### Étape 3 — Restriction de périmètre

Passages :
- P-E3c : « En multicœur, cependant, une écriture atomique de l’état ne suffit pas à garantir ce transfert. »
- P-E4 : « Ma réponse précédente supposait plusieurs cœurs capables d’ordonnancer simultanément. »

Quand l’utilisateur déclare que P-E4 restreint le périmètre de P-E3c à la condition « plusieurs ordonnanceurs »,

alors P-E3c garde son verdict « non vérifié », affiche la condition « plusieurs ordonnanceurs » et donne accès à P-E4. Aucun verdict n’est posé et le texte source est inchangé (INV-REL-06).

### Étape 4 — Décisions

Passages :
- P-E2a : « Ton idée d’organiser les attentes par port, puis par broche, est pertinente. »
- P-E3a : « Une tâche bloquée peut être référencée directement dans les structures d’attente du périphérique, puis rendue à l’ordonnanceur lors du réveil. »
- P-E3b : « le gestionnaire d’attente identifie la tâche à réveiller, puis une primitive commune réalise le transfert vers PENDING. »

Quand l’utilisateur crée :
- N-DRV-1, idée d’implémentation « Le driver réveille les tâches par écriture directe de l’état », décision « retenue », classée dans C-DRV, citant P-E2a et P-E3a, et reliée à P-E3b par « contredit » avec la justification « monocœur, section critique courte » ;
- N-DRV-2, idée d’implémentation « Un événement arrivé avant l’attente est conservé ; il est consommé par la poignée de main "j’attends" + "événement disponible" », décision « retenue », implémentation « en cours » avec le motif « refresh() consomme l’événement sans vérifier qu’une tâche attend », classée dans C-DRV, et déclarée dépendante de N-DRV-1,

alors :
1. N-DRV-1 et N-DRV-2 figurent dans les ressources actuelles ;
2. « contredit » ne crée aucune cause de réexamen et ne modifie pas P-E3b (INV-REL-07) ;
3. chaque transition de statut porte son motif (INV-MOD-05).

### Étape 5 — Réserves

Passages :
- P-E1-5 : « C’est là que peut se produire un réveil perdu. »
- P-E1-ack : « sur ce composant, GPIOICR acquitte les bits auxquels on écrit 1 »
- P-E1-4 : « une tâche peut attendre presque une période de tick après l’interruption avant d’être rendue prête »

Quand l’utilisateur crée :
- R-1 « Réveil perdu entre vérification et blocage », depuis P-E1-5, déclarée applicable à N-DRV-2, puis traitée par N-DRV-2 (action « traite ») ;
- R-2 « Acquittement GPIOICR : bits écrits à 1 », depuis P-E1-ack, avec la cible prévue, en texte, « startup.s (à créer) » ;
- R-3 « Latence d’un tick : paramètre à fixer », depuis P-E1-4, déclarée applicable à N-DRV-1,

alors :
1. R-1 est « traitée à vérifier » ; R-2 et R-3 sont « ouvertes » (INV-RES-02, INV-RES-03) ;
2. R-2 conserve E1 comme échange d’origine et n’est reliée à aucun fichier ; aucune relation ne vise un objet inexistant (INV-RES-01, INV-RES-04) ;
3. les trois passages sources sont inchangés.

### Étape 6 — Repères de code

Quand l’utilisateur relie N-DRV-1 à `wake_up()` et N-DRV-2 à `refresh()` dans src/driver/cortex_arm_m3/evalbot/driver.c,

alors :
1. deux commentaires `// @design: <uuid>` sont générés, distincts, ne contenant que l’identité de la note : ni titre ni contenu (INV-CODE-02) ;
2. chaque ancrage conserve le chemin, une copie du texte de la fonction et son empreinte ; aucune référence Git, le code n’étant pas encore versionné (INV-CODE-03) ;
3. chaque lien est « repère généré, présence à confirmer » ; la détection relève de l’indexeur, livré plus tard ;
4. driver.c n’est pas modifié par le logiciel (INV-CODE-04).

### Étape 7 — Reprise depuis le code

Étant donné l’état après l’étape 6, quand l’utilisateur part du repère de `wake_up()`,

alors le logiciel fournit, sans afficher la conversation entière :
1. **pourquoi :** N-DRV-1, ses passages cités P-E2a et P-E3a, la relation « contredit » vers P-E3b et sa justification ;
2. **encore retenue :** décision « retenue », aucune cause de réexamen ;
3. **réserves :** directement, R-3 ouverte ; dans la catégorie C-DRV, R-1 traitée à vérifier par N-DRV-2 et R-2 ouverte avec sa cible prévue ; l’implémentation « en cours » de N-DRV-2 et son motif ;
4. **ce qui ne vaut qu’en monocœur :** la justification de N-DRV-1, la restriction de P-E3c à « plusieurs ordonnanceurs » par P-E4, et les échanges classés dans C-MC.

### Étape 8 — Changement d’avis

Quand l’utilisateur passe N-DRV-1 à « plus retenue », avec le motif « primitive commune du noyau »,

alors :
1. l’opération est critique : une branche B est créée depuis main ; main est inchangée (INV-BR-01, INV-BR-02) ;
2. dans B, N-DRV-1 ne figure plus dans les ressources actuelles mais reste consultable ; son repère se résout et affiche « plus retenue » (INV-RET-01, INV-RET-02) ;
3. N-DRV-2 est « à réexaminer » avec la cause N-DRV-1 ; sa décision reste « retenue », son verdict inchangé, la relation « dépend de » active (INV-SEM-02, INV-RET-04) ;
4. R-1, R-2 et R-3 sont inchangées.

### Étape 9 — Acquittement, puis retour arrière

Quand l’utilisateur confirme que N-DRV-2 reste pertinente malgré la cause N-DRV-1,

alors N-DRV-2 n’est plus à réexaminer ; son unique appui, N-DRV-1, étant écarté, elle est **orpheline** (R6, D24, incrément 5.5) : listée parmi les orphelines, hors des ressources actuelles (avant 5.5 : « revient dans les ressources actuelles ») ; la relation « dépend de » est conservée ; l’acquittement référence la révision de N-DRV-2, l’empreinte de N-DRV-1 et le contexte (INV-ACK-01). L’opération reste sur B (INV-BR-02).

Quand N-DRV-1 reçoit ensuite une révision substantielle,

alors N-DRV-2 redevient « à réexaminer » : l’empreinte de la cause a changé.

Quand l’utilisateur annule ces trois opérations,

alors l’état métier est identique à celui de l’étape 7, la branche active est main, et B reste consultable (INV-UNDO-01, INV-BR-03).

## Scénarios par famille

### S-RET-01 — Monotonie des identités

Pour toute suite de commandes acceptées, l’ensemble des objets d’un commit est inclus dans celui de chacun de ses descendants. Il n’existe aucune commande de suppression. Vérification prévue : test de propriétés sur suites générées (INV-RET-01).

### S-RET-02 — Restauration non destructive

Étant donné A créée, puis le commit T, puis B créée avec son repère, quand l’utilisateur restaure T, alors A retrouve son état en T ; B reste présente et résoluble, retirée des ressources actuelles ; son repère se résout. Annuler la restauration remet B dans les ressources actuelles (INV-RET-03, INV-UNDO-01 ; défaut S10).

### S-MOD-01 — Révisions

Étant donné B dépendant de A, quand A reçoit une révision mineure, alors aucune cause de réexamen n’apparaît ; quand A reçoit une révision substantielle, alors B est à réexaminer. Les trois révisions de A restent adressables, avec leur type ; identité et repère de A sont inchangés (INV-MOD-01, INV-MOD-02 ; défaut S6).

### S-MOD-02 — Commande sans effet

Éditer une note avec son texte actuel, ou réappliquer son statut actuel, ne crée ni commit ni branche (INV-MOD-04 ; défaut S2).

### S-MOD-03 — Rôle immuable

Aucune commande ne change le rôle d’une note. Pour transformer un raisonnement en idée d’implémentation, l’utilisateur crée une nouvelle note qui remplace l’ancienne (INV-MOD-03).

### S-MOD-04 — Doublons

Étant donné N1 et N2 décrivant la même idée, N1 portant un repère, quand l’utilisateur déclare que N2 remplace N1, alors N1 passe « remplacée », reste consultable avec ses relations, et son repère se résout en indiquant N2 (INV-MOD-06).

### S-REL-01 — Auto-relation et doublon

Une relation d’une note vers elle-même, ou une seconde relation active identique (origine, destination, type, contexte), est refusée sans effet (INV-REL-01 ; défaut S4).

### S-REL-02 — Cycle de remplacement

Étant donné A remplace B et B remplace C, la déclaration « C remplace A » est refusée sans effet (INV-REL-02).

### S-REL-03 — Hiérarchie des catégories

Une catégorie peut être classée dans une autre ; une note peut l’être dans deux catégories ; un cycle de classement est refusé sans effet (INV-REL-03 ; défaut S11).

### S-REL-04 — Règles jugées à l’activation

Étant donné une relation inactive créée sous les règles v1, quand les règles v2 l’interdisent, alors le dépôt se charge et l’historique reste valide ; réactiver cette relation est refusé (INV-REL-04).

### S-SRC-01 — Réponse sans question

Quand l’utilisateur colle une réponse seule, alors l’échange est « incomplet » et visible comme tel ; aucune question n’est reconstruite. Le relier ensuite à une question enregistrée le rend complet (INV-SOURCE-04).

### S-RES-01 — Cycle de vie d’une réserve

Étant donné R créée depuis un passage sans destination, alors R figure dans la liste des points ouverts. Relier R à une note personnelle N par « s’applique à » ne change pas son statut et rend la navigation possible dans les deux sens. L’action « N traite R » la fait passer à « traitée à vérifier ». La résolution est une commande explicite, propre à un contexte : résolue dans le contexte X, R reste ouverte dans le contexte Y (INV-RES-01 à INV-RES-05).

### S-BR-01 — Branches

Sur main, une opération critique crée la branche B ; sur B, une deuxième opération critique reste sur B. Chaque résultat indique commit, branche active et bifurcation. Annuler jusqu’à l’opération fondatrice ramène sur main ; une annulation de plus porte sur main elle-même, jamais sur l’historique de B au-delà de sa base (INV-BR-02 à INV-BR-04 ; défauts S1, S3, S9).

### S-SCOPE-01 — Contextes superposés

Étant donné les contextes X et Y contenant N, quand le texte de N est révisé, alors la révision est visible dans X et dans Y ; quand le statut de N change dans X, alors il est inchangé dans Y. Une note créée après Y peut y être ajoutée (INV-SCOPE-02 ; défaut S5).

### S-ID-01 — Identités injectées

Deux dépôts recevant les mêmes commandes avec les mêmes UUID aboutissent au même état. Deux notes d’UUID distincts ont des repères distincts, y compris dans deux dépôts. Importer un objet dont l’UUID existe avec un contenu différent est refusé, sans fusion (INV-ID-03, INV-ID-04 ; défaut S7).

### S-CODE-01 — Ancrage avec et sans Git

Sans dépôt Git, l’ancrage conserve chemin, copie et empreinte. Avec Git, il conserve en plus le commit courant. Dans les deux cas, le repère ne contient que l’UUID (INV-CODE-02, INV-CODE-03).

### S-CODE-02 — Fichier renommé

Étant donné un repère placé dans engine.c, quand le fichier devient kernel.c, alors le repère retrouvé dans kernel.c se résout vers la même note ; l’ancrage signale son chemin périmé au lieu de choisir une occurrence (INV-CODE-01, INV-CODE-05).

### S-CODE-03 — Code jamais modifié

Générer un repère, restaurer le réseau, annuler ou basculer de branche n’écrit dans aucun fichier de code (INV-CODE-04).

### S-PREVIEW-01 — Aperçu périmé

Étant donné un aperçu calculé sur la tête H, quand une autre commande a publié entre-temps, alors l’application de l’aperçu est refusée sans effet (INV-PREVIEW-01).
