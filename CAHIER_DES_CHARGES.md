

# Cahier des charges — Espace personnel de conception assistée par IA

Version 0.2 — 21 octobre 2026 — version consolidée après les décisions D1 à D13
Statut : document de référence fonctionnel. Les décisions D1 à D13 (fichier DECISIONS.md du dépôt memoire) y sont intégrées ; le journal ci-dessous indique chaque modification. Le noyau est écrit en Rust ; son stockage sera SQLite (D4). L’interface reste à concevoir.

## Journal des révisions

**Version 0.2 (21 octobre 2026).** Intégration des décisions arrêtées pendant la conception du noyau. Chaque modification renvoie à sa fiche dans DECISIONS.md.

| Section | Modification | Décision |
|---|---|---|
| 1 | Un dépôt par projet, sans lien entre le RTOS et le processeur | D8 |
| 4 | Ordre des échanges enregistré explicitement ; actions de placement « Restreint le périmètre » et « Contredit » | INV-REL-05, INV-REL-06, INV-REL-07 |
| 5 | Catégories imbriquées sans cycle, sans statut | D7 |
| 5 | « Abandonnée » remplacé par « plus retenue », réversible ; aucune suppression | D6 |
| 5 | Motif obligatoire pour tout changement d’état ; états propres au contexte | D12, D2 |
| 5 | Modification mineure ou substantielle déclarée par l’utilisateur ; confirmation d’un réexamen | D5, INV-ACK-01 |
| 5 | Ressources actuelles : éléments proposés ou retenus, hors éléments écartés ou à réexaminer | D11 |
| 5 | Contenu partagé entre contextes, qualification propre à chacun | D2 |
| 6 | Copie et empreinte du code ciblé obligatoires sans Git ; repère = UUID seul, résolu indépendamment du chemin | D3, D9 |
| 10 | Bifurcation depuis main uniquement ; annulation bornée à la base de la branche | D1 |
| 10 | Criticité évaluée sur l’effet réel de l’opération | D10, D13 |
| 10 | Stockage SQLite faisant autorité, Git pour l’export | D4 |
| 10 | Restauration non destructive ; le code n’est jamais restauré par le logiciel | D6, D9 |
| 11, 12, 13 | Numérotation corrigée ; critères d’acceptation identifiés CA-01 à CA-40 ; CA-34 précisé | — |

**Version 0.1 (20 octobre 2026).** Branches automatiques et génération des repères.

## 1. Finalité

Créer un espace de travail personnel pour concevoir des projets techniques complexes, notamment un mini-RTOS et une architecture de processeur. Le logiciel relie les conversations avec l’IA, les idées, les décisions, le code et l’état d’avancement.

Un dépôt correspond à un projet.

Le résultat recherché est de pouvoir répondre rapidement à cinq questions :

- Pourquoi ce code a-t-il été conçu ainsi ?

- Quel échange contient le raisonnement original ?

- Quelles parties de ce raisonnement sont encore applicables ?

- Quel changement impose de réexaminer quelles parties du projet ?

- Où reprendre le travail maintenant ?

## 2. Besoins exprimés et propositions

### Besoins exprimés

- Conserver les questions originales et les réponses de l’IA, dans leur ordre et leur contexte.

- Retrouver une information sans devoir la redemander à l’IA.

- Surligner et annoter directement les passages importants, sans accumuler des commentaires à la suite.

- Invalider des éléments devenus obsolètes et accéder à leur correction.

- Associer un raisonnement à une partie précise du code.

- Respecter une organisation visuelle du code par responsabilités et groupes de fonctions.

- Sauvegarder l’état d’avancement, l’intention en cours et les prochaines actions.

- Pouvoir changer d’éditeur ; envisager un logiciel personnel développé avec l’IA.

- Ajouter les échanges par copier-coller ; une conversation intégrée avec l’IA n’est pas nécessaire.

- Définir le rôle du couple question–réponse par les annotations faites lors de son placement dans un réseau de conversations.

### Propositions à discuter

- Application centrée sur le projet, avec conversations, code et annotations dans des panneaux coordonnés.

- Données locales, exportables et sauvegardables indépendamment du fournisseur d’IA.

- Conservation des sources originales ; annotations et états stockés séparément.

- Capture manuelle simple en première version ; automatisation ultérieure.

- Première version utilisable sans connexion à un modèle d’IA.

- Appui sur un composant d’édition existant si l’écriture du code est intégrée.

Ces propositions ne doivent pas être présentées comme des préférences déjà validées.

## 3. Parcours principal

1. Ouvrir un projet et consulter la fiche de reprise.

2. Ouvrir la zone de code ou la responsabilité sur laquelle travailler.

3. Consulter les passages de conversations et décisions associés.

4. Ajouter un nouvel échange par collage ou import.

5. Sélectionner un passage pour le surligner, le relier, le qualifier ou indiquer son remplacement.

6. Examiner les éléments dont les hypothèses ou les ancrages ont changé.

7. En fin de session, enregistrer un point de reprise court.

La capture d’un échange et le surlignage doivent être possibles sans remplir une fiche complète. La structuration détaillée reste facultative.

## 4. Conversations et annotations

### Conservation

L’unité de placement dans le réseau est un échange identifié : question originale et réponse associée. Les passages de chacune restent annotables séparément. Une conversation conserve les messages, leur auteur, leur ordre et, si disponible, leur date et leur origine. Le texte original reste consultable. Les blocs de code sont préservés. L’ordre des échanges d’une conversation est enregistré explicitement à la saisie, en désignant l’échange précédent ; il ne crée aucune relation et ne remplace rien.

Le copier-coller est le mode d’entrée de référence. L’interface propose deux zones explicites « Question » et « Réponse », ou un collage groupé avec vérification de leur séparation. Si seule la réponse est collée, conserver l’entrée comme incomplète : l’utilisateur peut coller la question ou la relier à une question déjà enregistrée. Le logiciel ne reconstruit pas silencieusement une question supposée.

Un import ne doit pas inventer les auteurs ou les frontières des messages lorsqu’ils sont ambigus : une vérification manuelle est proposée.

### Annotations sur le texte

L’utilisateur peut sélectionner un extrait et lui associer :

- un surlignage ;

- un degré d’importance ;

- une note facultative ;

- un lien vers du code, une idée, une décision ou un autre extrait ;

- une qualification de validité et sa justification.

Les annotations apparaissent dans le texte rendu : fond coloré, soulignement, badge ou atténuation. Leur détail s’ouvre à la demande. Le sens ne repose pas uniquement sur la couleur.

Une annotation peut concerner une phrase ; le reste de la réponse conserve son état. Plusieurs annotations peuvent se recouvrir sans modifier le texte source.

Une vue de lecture montre le contexte original. Une vue de travail permet de privilégier les éléments encore applicables. Un passage exclu de cette dernière reste accessible depuis l’historique.

### Placement dans le réseau de conversations

Le placement est une opération de classement logique. La proximité visuelle et la date d’ajout ne créent aucune dépendance ni invalidation. Un échange peut rejoindre plusieurs échanges ou thèmes : le réseau n’est pas contraint à un arbre.

À l’ajout :

1. Coller le couple question–réponse ou compléter son association.

2. Choisir le projet et le point de rattachement : échange, passage précis ou responsabilité.

3. Définir une ou plusieurs relations par des actions courtes.

4. Surligner les passages utiles et préciser les cibles d’une correction.

5. Examiner les effets proposés avant de terminer le placement.

Le logiciel peut enregistrer une entrée en brouillon sans exiger toutes ces étapes.

| Action de placement proposée | Sens enregistré | Effet sur l’existant |
|---|---|---|
| Approfondit | Développe une question ou une explication | Aucun changement de validité |
| Explore une alternative | Ouvre une piste parallèle | Préserve les autres pistes |
| Corrige / remplace | Corrige un passage ciblé dans un périmètre donné | Marque le passage comme corrigé ou remplacé et relie sa correction |
| S’appuie sur | Déclare une dépendance du raisonnement | Permet de signaler un besoin de réexamen ultérieur |
| Relie au même sujet | Ajoute un lien de navigation | Aucune propagation de validité |
| Restreint le périmètre | Limite l’applicabilité d’une affirmation à une condition | Aucun verdict ; les dépendants de la cible sont à réexaminer |
| Contredit | Enregistre un désaccord assumé, avec sa justification | Aucune propagation ; la cible est inchangée |

Les libellés sont proposés et restent à tester avec l’utilisateur. « Approfondit » ne signifie pas automatiquement « dépend de » ; l’interface peut proposer de préciser une dépendance.

Une correction doit cibler l’extrait concerné, ou l’échange entier si l’utilisateur le choisit explicitement. Relier B à A par « corrige » ne doit pas rendre toute la conversation A obsolète.

Pour chaque action, le logiciel applique des règles explicites aux annotations structurées. Il n’a pas besoin de comprendre sémantiquement toute la réponse pour gérer les liens et états. Une note libre ambiguë ne déclenche pas de changement de validité : elle demeure une annotation tant que l’utilisateur n’a pas choisi une action précise.

L’interface affiche les conséquences d’une correction : passage visé, correction associée et dépendants à réexaminer. Ces effets sont enregistrés ensemble et annulables. Les relations de remplacement ne peuvent former une boucle ; le parcours des dépendances doit terminer même en présence de cycles.

Les états de validité sont distincts des alertes de réexamen. Modifier ou retirer une relation recalcule les effets qui en dépendent, sans supprimer les annotations ou verdicts manuels. Chaque état calculé affiche sa cause.

### Réserves et rattachements ajoutés a posteriori

Une réserve telle que « dans tel cas, il manque un invariant » peut être sélectionnée dans une réponse et conservée comme objet identifiable, même si aucune note de destination n’existe encore. Elle conserve son passage source, le couple question–réponse d’origine, son périmètre et ses éventuelles cibles de code.

L’utilisateur peut ensuite la rattacher à une note personnelle, un nouvel échange ou un passage précis, quelle que soit leur date de création. Les notes personnelles sont des objets autonomes ; elles ne nécessitent pas de question–réponse fictive.

Deux intentions restent distinctes :

- « Cette réserve s’applique à cette note » : la note affiche la réserve, qui reste ouverte.

- « Cette note traite cette réserve » : la note devient une proposition de réponse ; la clôture de la réserve est une action séparée et explicite.

L’annotation peut rester « ouverte », devenir « traitée à vérifier », puis être marquée « résolue » dans un périmètre explicite. Ces libellés sont proposés. Un simple lien, la rédaction d’une explication ou une implémentation ne suffisent pas à prouver la résolution.

Une réserve peut être reliée à plusieurs notes. La traiter pour une variante ne la clôt pas pour toutes les autres. Le lien apparaît dans les deux sens ; la réserve reste consultable à son emplacement original avec son état actuel et l’accès aux éléments qui la traitent.

Exemple de référence, fictif :

1. Un échange A décrit une transition vers l’état WAIT.

2. La réponse contient la réserve : « Il faut garantir qu’un réveil concurrent ne puisse pas être perdu. »

3. L’utilisateur sélectionne cette phrase et choisit « Réserve / invariant à préciser », sans créer immédiatement de note de solution.

4. Plus tard, il crée une note N sur le protocole d’attente et y rattache la réserve existante. N affiche l’avertissement ; A permet d’ouvrir N.

5. Il rédige dans N une proposition d’invariant et son mécanisme, puis choisit « traite cette réserve ». Le suivi passe à « traitée à vérifier ».

6. Il documente une justification ou une vérification adaptée, puis confirme la résolution pour la variante concernée. Le texte original reste intact.

Le logiciel propose une liste des réserves ouvertes pour les retrouver au moment de créer ou de relier une nouvelle note. Il ne crée ni copie indépendante de la réserve ni lien vers une destination inexistante.

### Exemple de référence pour le prototype

Exemple fictif : E1 contient une question générale sur la mise en attente d’une tâche ; E2 approfondit la question du réveil ; E3 corrige une phrase précise de E1. Une fonction liée à cette phrase et déclarée dépendante du raisonnement apparaît « à réexaminer ». E2 reste accessible et n’est pas automatiquement invalidé par sa seule position dans le réseau.

Le test vérifie que l’utilisateur peut retrouver la phrase originale de E1, la correction de E3, la question qui a suscité chaque réponse et le lien vers la fonction, sans recopier ces contenus.

## 5. Idées, décisions et validité

### Catégories, raisonnements et idées d’implémentation

Le réseau distingue explicitement trois rôles :

| Rôle | Fonction | Exemple fictif |
|---|---|---|
| Catégorie | Organise et facilite la navigation, sans affirmer une vérité technique | Ordonnancement, files de tâches, réveils |
| Raisonnement | Conserve hypothèses, invariants, arguments et conclusions, reliés à leurs sources | Sous telles hypothèses, le parcours de la file est borné |
| Idée d’implémentation | Décrit un choix réalisable, ses alternatives et ses points de code | Pour cette politique, représenter la file par telle structure |

Une décision d’architecture telle que « retenir un scheduler centralisé » est un choix suivi, et peut être justifiée par un raisonnement ; une catégorie du même nom ne suffit pas à exprimer ce choix.

La relation « classé dans » est indépendante de « développé à partir de », « justifie », « dépend de » et « implémenté par ». L’imbrication visuelle ne déclenche pas d’invalidation. Chaque objet affiche son rôle ; les libellés restent modifiables. Les catégories peuvent s’imbriquer, sans cycle, par exemple « Exploration / évolution » puis « Exploration multicœur ». Une catégorie ne porte jamais de statut : un statut ne s’exprime pas en rangeant une note dans une catégorie « abandonné » ou « à revoir », car seuls les statuts déclenchent le réexamen (D7).

Une sous-idée d’implémentation peut être invalidée ou remplacée sans abandonner l’architecture qui l’accueille. Si une autre note dépend de cette sous-idée, elle passe « à réexaminer » ; la catégorie et les idées voisines ne sont pas invalidées par proximité.

Exemple : conserver le choix d’un scheduler centralisé tout en abandonnant la structure de données proposée pour une politique d’éviction particulière. Le raisonnement qui justifiait cette structure conserve ses sources et son historique ; seule la partie contestée est signalée. Une autre proposition peut ensuite reprendre le rôle d’implémentation sans déplacer ou supprimer les autres notes de l’architecture.

Les réserves peuvent viser un raisonnement, une idée d’implémentation ou un passage précis. Une même conversation peut alimenter plusieurs objets de rôles différents.

Une idée ou décision peut être liée à plusieurs passages sources et à plusieurs zones de code. Il n’est pas obligatoire de transformer chaque passage en fiche.

Les dimensions suivantes sont distinctes :

- nature : question, hypothèse, explication, contrainte, décision ;

- état de décision : proposée, retenue, plus retenue, remplacée. « Plus retenue » remplace toute suppression : la note reste consultable, son retour est possible et ses dépendants passent à réexaminer (D6). « Remplacée » est posé par la relation « remplace » ;

- état de vérification : non vérifiée, vérifiée dans un périmètre indiqué, réfutée ;

- état d’implémentation : non commencée, en cours, implémentée ;

- importance : ordinaire, importante, critique.

L’interface ne présente que les champs utiles au contexte. Une décision retenue peut rester non implémentée et non vérifiée. Chaque changement d’état exige un motif, conservé dans l’historique (D12). Les états sont propres à chaque contexte (D2).

Les éléments disposent d’un périmètre : projet, variante d’architecture, cible matérielle ou version de code. Une proposition valable en monocœur peut devenir inapplicable en multicœur sans être fausse dans son contexte initial.

### Relations

Relations minimales :

- dépend de ;

- remplace ;

- contredit ;

- justifie ;

- concerne une zone de code.

La relation « dépend de » exprime une dépendance du raisonnement. Une association thématique ou un lien de navigation n’en crée pas implicitement une.

Lorsqu’une hypothèse est réfutée ou remplacée :

1. enregistrer le motif, le périmètre et, s’il existe, le passage correctif ;

2. marquer ses dépendants comme « à réexaminer » ;

3. laisser l’utilisateur confirmer les conséquences : confirmer qu’une note reste pertinente lève l’alerte sans supprimer le lien ; la confirmation devient caduque si la cause change d’état ou si la note confirmée change de sens, une correction mineure ne la rouvrant pas ;

4. préserver l’historique et permettre d’annuler l’opération.

Un nouveau message de l’IA ne remplace rien automatiquement. La propagation suit les dépendances déclarées ; elle ne promet pas de découvrir toutes les conséquences logiques du changement.

### Réexamen et réutilisation des sous-graphes

Lorsque l’utilisateur abandonne, remplace ou modifie substantiellement une idée structurante, les notes développées à partir de cette idée doivent pouvoir être réexaminées en groupe. L’objectif est de conserver les idées réutilisables et de retirer les autres des ressources actuelles du développement, sans perdre leur provenance ni leurs liens.

Le caractère mineur ou substantiel d’une modification est déclaré par l’utilisateur au moment où il la fait, par exemple par un bouton, sans valeur implicite. Seule une modification substantielle met les dépendants en réexamen (D5).

Trois vues s’appuient sur les mêmes objets :

- « Ressources actuelles » : éléments proposés ou retenus pour la variante sélectionnée, hors éléments écartés (réfutés, retirés, plus retenus, remplacés) ou à réexaminer ; les réserves ouvertes restent visibles (D11).

- « À traiter — encore pertinent ? » : sous-graphes dont l’applicabilité doit être réévaluée.

- « Historique / pistes écartées » : éléments et rattachements antérieurs, absents du panneau principal mais toujours consultables.

Il s’agit de vues, pas de déplacements destructifs entre dossiers. L’état d’inclusion dans les ressources actuelles est distinct de la validité d’une idée et de son état de vérification. Une idée écartée peut être correcte mais inadaptée à la direction retenue.

Le réexamen porte sur les dépendances déclarées et les relations explicites « développé à partir de ». Un simple lien thématique ne suffit pas. Une provenance commune justifie un réexamen, pas une invalidation automatique.

Le logiciel rassemble les éléments concernés avec leurs relations internes, réserves, liens au code et rattachements extérieurs. Le parcours termine même si le réseau contient des cycles. Il présente un aperçu du groupe avant l’application d’une action collective.

Actions disponibles sur une note ou un groupe :

- Conserver dans le développement actuel : confirmer son applicabilité au périmètre choisi.

- Réattacher : ajouter un rattachement à une autre idée, responsabilité ou variante, en conservant les liens internes pertinents et la provenance.

- Adapter puis réattacher : conserver la version d’origine et créer une révision liée dont les changements sont consultables.

- Laisser à traiter : conserver les questions ouvertes.

- Retirer des ressources actuelles : masquer l’élément dans la vue de travail concernée, tout en le gardant dans l’historique.

Le réattachement enregistre sa destination, son périmètre et les relations que l’utilisateur souhaite activer. Il ne suppose pas que toutes les dépendances de l’ancien contexte sont satisfaites dans le nouveau. Les contraintes ou réserves non réévaluées restent signalées.

Les notes possèdent des identifiants stables ; un réattachement sans modification ne crée pas de copie. Les relations précédentes restent consultables avec leur période ou état d’activation et le motif de leur retrait. Les liens internes d’un groupe ne sont pas supprimés lors de sa mise en attente ou de son retrait de la vue principale.

Un élément peut appartenir à plusieurs contextes. Retirer son rattachement à une piste ne doit pas retirer son autre rattachement actif ni le masquer globalement. La file de réexamen indique le contexte concerné et reste distincte de la présence de l’élément dans une autre variante. Son identité et son contenu sont partagés entre contextes ; chaque contexte porte l’appartenance, les statuts et l’activation des relations (D2). Un contexte n’est créé que pour une variante réellement suivie en parallèle ; une exploration se range dans une catégorie (D7).

Aucun élément n’est supprimé parce qu’il n’a plus de rattachement actif. Il reste retrouvable dans l’historique et dans la recherche incluant les pistes écartées. Les retraits et réattachements sont annulables.

Exemple fictif : l’idée A décrit un scheduler centralisé. Les notes N1 « représentation des tâches » et N2 « protocole de réveil » sont développées à partir de A ; elles sont liées entre elles et contiennent leurs propres réserves. L’utilisateur abandonne A pour une piste B. N1 et N2 apparaissent ensemble dans « À traiter », avec leurs liens conservés. Il réattache N1 à B après réexamen ; il retire N2 des ressources actuelles. L’historique conserve le lien A–N1, le lien A–N2 et le lien N1–N2. La vue actuelle affiche les relations retenues pour B ; les anciennes restent accessibles à la demande.

### Historique et apport de Git

L’application doit enregistrer les changements de contenu, de rattachement et de statut avec leurs motifs ; elle peut reconstituer un état antérieur du réseau. Les instantanés et exports doivent préserver les objets et leurs identifiants, les anciennes relations, les états et les révisions de notes.

Git peut versionner une représentation textuelle exportée et relier des points de conception aux versions du code. Les actions sémantiques « à réexaminer », « retiré des ressources actuelles » et « réattaché » restent définies et gérées par l’application. Le fonctionnement courant ne doit pas obliger l’utilisateur à parcourir des commits pour retrouver une ancienne relation.

## 6. Liaisons au code

Depuis une sélection de code, l’utilisateur peut retrouver les annotations et raisonnements associés. Depuis un extrait de conversation, il peut ouvrir les zones concernées. Un même raisonnement peut concerner plusieurs fonctions.

Prévoir deux niveaux :

- ancrage précis sur une fonction, un label ou une sélection ;

- responsabilité logique : ordonnancement, interruptions, helpers, gestion mémoire, etc.

Une responsabilité peut réunir des fonctions de plusieurs fichiers. Son organisation peut être visuelle sans déplacer physiquement le code.

Un ancrage conserve, autant que possible :

- le fichier et un symbole ou label ;

- la sélection et un extrait de contexte ;

- la version d’origine ;

- une copie du texte ciblé et son empreinte, obligatoires tant que le code n’est pas versionné ;

- son état : retrouvé, modifié, ambigu ou perdu.

Le numéro de ligne seul ne suffit pas. En cas d’ambiguïté, proposer un rattachement manuel plutôt qu’ouvrir silencieusement une mauvaise zone.

La détection d’un changement de texte ne prouve pas qu’un raisonnement est devenu faux. Elle produit un signal de révision distinct d’une invalidation conceptuelle.

L’analyse détaillée des symboles C, assembleur et HDL peut venir progressivement. La première version accepte des ancrages textuels avec leurs limites explicites.

### Repères générés automatiquement pour les commentaires du code

Lorsqu’une note est déclarée liée à une zone de code, le logiciel crée ou réutilise son identifiant stable et génère automatiquement le commentaire à copier à cet emplacement. Cela concerne notamment les idées d’implémentation, mais une autre note peut aussi être directement liée au code.

Parcours :

1. Créer la note ou choisir une note existante.

2. Choisir « Relier à du code » et préciser la zone ou le fichier si disponible.

3. Générer un commentaire dans la syntaxe appropriée, avec un bouton « Copier ».

4. L’utilisateur colle ce commentaire dans le code.

5. Le logiciel détecte l’occurrence dans le périmètre indexé et confirme le lien ; avant détection, afficher « Repère généré — présence à confirmer ».

Exemple fictif en C ou SystemVerilog :
// @design: 8c7e0a2d-6b94-4f51-9a23-d06b7185e4c2

Un libellé lisible est facultatif. Le format peut être un identifiant opaque ; la convention et la syntaxe de commentaire dépendent des langages et outils. Aucun commentaire n’est inséré ou déplacé automatiquement par le seul fait d’en générer le texte.

L’identifiant représente l’identité de la note et non son contenu courant. Il peut être généré aléatoirement ; si une forme de hash est utilisée, elle ne doit pas être recalculée à chaque modification du texte. Les empreintes des révisions et du code sont des informations distinctes. L’identifiant est un UUID de 128 bits attribué à la création ; le repère ne contient que lui, jamais le titre ni le contenu de la note (D3). Un repère se résout par cet identifiant, quel que soit le fichier où il se trouve ; un chemin d’ancrage devenu périmé est signalé, jamais deviné.

Modifier, renommer, invalider, restaurer ou réattacher la note ne change pas cet identifiant. Les branches partagent l’identité de l’objet et conservent leurs propres versions et statuts ; fusionner une branche ne nécessite pas de réécrire les commentaires existants.

Le logiciel indexe les repères et les relie aux objets enregistrés. Depuis la note, ouvrir les occurrences ; depuis le repère, retrouver la note, ses raisonnements, ses réserves et son historique dans le contexte sélectionné. Si l’objet existe seulement dans une autre branche, proposer d’ouvrir ce contexte au lieu de le déclarer inexistant.

Une note peut avoir plusieurs occurrences intentionnelles dans le code. Le logiciel les liste et signale les identifiants inconnus, les collisions réelles entre objets et les occurrences disparues. Il ne choisit pas arbitrairement un emplacement unique.

Le commentaire identifie la note. La fonction ou la plage couverte est précisée par l’ancrage associé ; une seule ligne de commentaire ne délimite pas automatiquement toute la zone concernée.

Une invalidation n’efface ni le commentaire ni le code. Une restauration vérifie les ancrages face au code courant et affiche les écarts. La génération du repère et les états du lien sont exportables avec le reste du projet.

## 7. Interface proposée

Trois espaces coordonnés :

- navigation du projet et des responsabilités ;

- document principal : code, conversation ou carte de conception ;

- informations liées : passages sources, décisions, corrections et travaux ouverts.

L’utilisateur peut épingler une conversation à côté du code et suivre les liens dans les deux sens.

Une carte spatiale peut représenter les responsabilités et leurs relations. Elle reste une vue sur les mêmes objets : déplacer une carte ne crée pas une copie de l’idée.

Le choix entre éditeur intégré, extension et application compagnon reste ouvert. Le critère est le nombre de manipulations nécessaires pour relier un passage à du code, puis le retrouver. Construire un nouvel éditeur complet n’est pas un prérequis au premier prototype.

## 8. Fiche de reprise

Chaque session peut se terminer par un point de reprise contenant :

- objectif et question précise en cours ;

- état réellement atteint ;

- dernier résultat de compilation, exécution ou test pertinent ;

- hypothèse actuelle et incertitudes ;

- fichiers, fonctions et passages ouverts ;

- prochaine action concrète ;

- état du code associé.

L’état du code inclut une référence de version si disponible et un moyen de retrouver le travail non enregistré dans un commit. Le logiciel ne doit pas imposer un commit à chaque arrêt.

Le point de reprise conserve son contexte historique. À la réouverture, signaler si le code a changé depuis.

Cible d’usage à tester : retrouver en moins de deux minutes l’objectif, le blocage et la première action à effectuer.

## 9. Recherche et préparation du contexte IA

Première version :

- recherche textuelle dans les conversations, annotations et décisions ;

- filtres par projet, responsabilité, validité et importance ;

- recherche des éléments liés à la sélection de code ;

- affichage visible des états dans les résultats.

Recherche sémantique : évolution possible, après validation du besoin. Un résultat proche sémantiquement ne doit pas masquer son état obsolète.

Le logiciel peut produire un contexte à copier dans une nouvelle session d’IA :

- objectif en cours ;

- décisions applicables au périmètre ;

- contraintes et questions ouvertes ;

- extraits de code choisis ;

- sources et repères utiles ;

- propositions abandonnées uniquement lorsqu’elles expliquent une contrainte ou évitent une répétition.

L’utilisateur voit et modifie ce contexte avant transmission. Aucun accès automatique d’une IA externe aux notes n’est supposé.

## 10. Données et fiabilité

Objets minimaux :

| Objet | Rôle |
|---|---|
| Projet | Regroupe le travail et ses variantes |
| Conversation / échange / message | Conserve les couples question–réponse et leur contexte |
| Sélection / annotation | Ajoute une lecture à un extrait |
| Note personnelle | Conserve une réflexion autonome sans échange IA obligatoire |
| Réserve / point ouvert | Suit un avertissement ou un invariant à préciser et ses rattachements différés |
| Catégorie | Organise les éléments sans porter de validité technique |
| Raisonnement | Conserve hypothèses, invariants et arguments |
| Idée d’implémentation / décision | Suit un choix réalisable et ses repères de code |
| Opération / état enregistré | Permet l’annulation, le rétablissement et la restauration |
| Relation | Relie les raisonnements et leurs conséquences |
| Ancrage de code | Associe un élément à une zone du projet |
| Responsabilité | Regroupe les éléments selon leur rôle |
| Session / point de reprise | Permet de reprendre le travail |

Chaque objet suivi possède un identifiant stable. Les changements d’état sont enregistrés et annulables. Les sources ne sont pas effacées par une invalidation.

Prévoir export et réimport des textes, annotations, relations et points de reprise, avec leurs identifiants. Une sauvegarde doit préserver l’ensemble, pas seulement les conversations.

Le dépôt de code peut rester géré par Git. La version du code et la validité des raisonnements constituent des informations complémentaires.

### Branches automatiques pour les modifications critiques

Le fonctionnement retenu s’inspire de Git : une branche principale « main » représente la conception de référence, et les modifications critiques sont effectuées dans une nouvelle branche créée automatiquement. L’utilisateur décide explicitement de la fusion vers main.

Lorsqu’une opération critique est engagée :

1. Enregistrer un état de départ complet du contexte courant, y compris les dernières modifications non encore regroupées dans un jalon.

2. Créer automatiquement une branche issue de cet état, avec un nom généré et renommable.

3. Effectuer l’opération et ses effets dans cette branche, puis poursuivre le travail dans celle-ci.

4. Laisser la branche d’origine inchangée par cette opération ; afficher clairement la branche actuellement consultée.

Seule main déclenche une bifurcation : sur une branche expérimentale, l’opération critique s’y poursuit sans créer de nouvelle branche (D1). Chaque branche enregistre son origine et son commit de base ; l’annulation ne franchit pas cette base : annuler l’opération fondatrice ramène sur l’origine, la branche restant consultable. Une opération collective constitue un seul changement critique et crée une branche unique pour l’ensemble de ses effets. Les étapes internes de propagation ne créent pas chacune leur propre branche.

Est critique une opération qui, dans l’état courant, crée ou lève effectivement une cause de réexamen, écarte une note (réfutation, « plus retenue », retrait des ressources actuelles, remplacement) ou restaure un état antérieur. La criticité est évaluée sur l’effet réel de l’opération, pas sur son seul type (D10, D13). Ainsi, déclarer qu’une note s’appuie sur une autre note saine reste ordinaire, et le travail courant reste sur main jusqu’à un véritable changement d’avis. Une modification portant sur une sous-idée peut être critique sans affecter toute l’architecture. Une branche peut aussi être créée volontairement avant une exploration.

Le logiciel gère l’enregistrement des changements sans exiger de commandes Git ou de commits manuels à chaque annotation. Les opérations ordinaires conservent Annuler / Rétablir ; une simple correction de mise en forme ne crée pas nécessairement de branche.

L’utilisateur peut :

- revenir immédiatement à main ou à la branche d’origine ;

- rouvrir une piste expérimentale et retrouver son état ;

- comparer les contenus, relations, réserves et ressources actives entre deux branches ;

- demander une fusion vers main ;

- archiver une branche sans effacer son historique.

Aucune fusion vers main n’est déclenchée par une sauvegarde, une fermeture ou une simple réactivation d’idée.

Les identifiants des notes restent stables entre branches. Les versions de contenu, les relations actives et les statuts sont propres à l’état consulté. Une invalidation dans une branche ne modifie pas silencieusement la validité du même objet dans main.

La fusion utilise une base commune pour distinguer les modifications indépendantes des modifications concurrentes. Des changements compatibles peuvent être réunis ; les conflits explicites sont présentés par objets et relations, sans imposer à l’utilisateur de résoudre des fichiers de données bruts. Exemples : contenu modifié de deux façons, même idée conservée dans une branche et retirée dans l’autre, relations de remplacement incompatibles.

La résolution tient compte des règles de validité et des périmètres enregistrés ; elle ne prétend pas détecter toutes les contradictions du langage naturel. Avant la fusion, afficher son effet sur les ressources actuelles et les points à réexaminer. Une fusion est enregistrée et réversible.

Le réseau est conservé dans un historique applicatif local et transactionnel (SQLite), qui fait autorité ; Git ne sert qu’à un export textuel déterministe (D4). Les objets et opérations du réseau doivent rester compréhensibles dans l’interface dans les deux cas.

Le branchement du réseau ne change pas automatiquement la branche Git du code. Le point de départ peut enregistrer une référence au code et, si nécessaire, à son état non commité. La coordination avec les branches du code constitue une fonction distincte à définir.

### Annulation et restauration simples

La réversibilité est une exigence de la première version, pas un outil de récupération exceptionnel. L’utilisateur doit pouvoir essayer une organisation ou écarter une idée sans considérer cette action comme irréversible.

Trois commandes sont proposées :

- Annuler / Rétablir : revenir sur la dernière opération de conception ou la réappliquer, avec raccourcis accessibles.

- Restaurer cet élément ou ce groupe : retrouver un ancien état sélectionné sans revenir sur les changements indépendants effectués ailleurs.

- Revenir à un point de reprise : restaurer une vue cohérente du réseau à une date ou un jalon choisi.

Chaque opération de conception est une transaction : contenu éventuellement modifié, relations activées ou retirées, visibilité et effets calculés sont enregistrés ensemble. Invalider une idée et créer les alertes dépendantes constitue une seule opération annulable. Un déplacement collectif ou un réattachement de sous-graphe ne nécessite pas d’annuler chaque lien séparément.

Le journal conserve les opérations, leur périmètre, leurs motifs et les états nécessaires à leur inversion. L’historique utile reste accessible après fermeture et réouverture de l’application. Créer une nouvelle action après une annulation ne supprime pas la piste historique abandonnée ; elle reste restaurable, même si la commande de rétablissement immédiat suit une pile classique.

Une restauration est elle-même enregistrée comme nouvelle opération. Elle ne supprime pas les états plus récents et peut être annulée. Réactiver une idée ancienne ne nécessite pas de recréer ses notes, identifiants, réserves et liens internes. Les objets créés après l’état restauré ne disparaissent pas : ils restent résolubles et sont retirés des ressources actuelles (D6).

Lors d’une restauration ciblée, conserver les changements indépendants. Si des modifications ultérieures portent sur les mêmes objets ou dépendent des relations à restaurer, présenter les éléments concernés et proposer la restauration du groupe cohérent ou une réactivation conservant les modifications compatibles. Ne pas écraser silencieusement une version plus récente.

L’interface prévoit un accès direct « Annuler » après les opérations courantes et un historique lisible. Les manipulations ordinaires réversibles ne déclenchent pas de confirmation répétitive. Un aperçu est réservé aux effets collectifs ou aux conflits de restauration.

Le périmètre du retour arrière est explicite :

- restaurer le réseau restaure notes, annotations, statuts et relations ;

- restaurer le code nécessite un état du code correspondant, géré séparément par Git ou un instantané ;

- une fiche de reprise conserve la référence du code ; la restauration du code reste une action de l’utilisateur dans Git. Le logiciel n’écrit jamais dans le code et ne le restaure pas (D9).

Les tests de reprise doivent couvrir une fermeture après une opération et une restauration après réouverture. La conservation du journal ne remplace pas les sauvegardes.

## 11. Première version et évolutions

### Première version utilisable

- Un projet local par dépôt (D8).

- Distinction explicite entre catégories, raisonnements et idées d’implémentation.

- Branches automatiques pour les modifications critiques, comparaison et fusion explicite vers main.

- Annuler / Rétablir, restauration ciblée et historique persistant des opérations de conception.

- Génération automatique des repères à copier dans les commentaires du code pour les notes liées.

- Échanges question–réponse ajoutés par collage contrôlé.

- Réseau navigable d’échanges avec relations explicites, choix du rattachement et cibles de correction.

- Lecture avec surlignage et annotations sur des extraits.

- Remplacement explicite d’un passage par un autre.

- Réserves ouvertes et rattachement ultérieur à des notes personnelles ou des échanges.

- Liens bidirectionnels avec des sélections de code.

- Dépendances manuelles et liste des éléments à réexaminer.

- Mise en réexamen de sous-graphes, réattachement et retrait des ressources actuelles avec historique des relations.

- Recherche textuelle.

- Fiche de reprise.

- Export et réimport.

### Évolutions

- Carte spatiale des responsabilités et décisions.

- Meilleur suivi des ancrages après modification ou déplacement du code.

- Capture automatisée des conversations.

- Suggestions de liens et de contradictions par IA.

- Recherche sémantique.

- Contexte IA généré selon la zone de code.

- Édition intégrée plus complète si les essais la justifient.

L’IA peut proposer une correction ou une relation ; son application reste explicite.

## 12. Critères d’acceptation

Les identifiants CA-01 à CA-40 sont stables et ne sont jamais réattribués ; ils remplacent la numérotation 13 à 52 de la version 0.1, fondue par erreur dans celle des sections. Leur couverture est suivie dans TRACABILITE.md.

- **CA-01** — Après import, retrouver une question et sa réponse dans l’ordre original.

- **CA-02** — Surligner une phrase sans ajouter de commentaire au texte source.

- **CA-03** — Remplacer une affirmation en conservant le reste du message et un accès à la correction.

- **CA-04** — Depuis une fonction, ouvrir l’extrait exact qui justifie son organisation, puis revenir au code.

- **CA-05** — Après déplacement du code, retrouver l’ancrage ou signaler qu’il doit être rétabli.

- **CA-06** — Après invalidation d’une hypothèse, lister les dépendants à réexaminer sans les déclarer automatiquement faux.

- **CA-07** — Conserver simultanément deux choix applicables à deux variantes différentes.

- **CA-08** — Après arrêt et réouverture, retrouver l’objectif, l’état réel du code et la prochaine action.

- **CA-09** — Après export et réimport, conserver les identifiants, annotations et relations.

- **CA-10** — Utiliser les fonctions essentielles sans appel à une IA externe.

- **CA-11** — Placer un échange en approfondissement sans modifier la validité du précédent.

- **CA-12** — Corriger un passage précis sans invalider le reste de sa réponse ni tous les échanges suivants.

- **CA-13** — En cas de réponse collée sans question, conserver un état incomplet visible sans inventer la question.

- **CA-14** — Annuler une relation de correction et retrouver les états manuels antérieurs ; maintenir les alertes justifiées par d’autres causes.

- **CA-15** — Rattacher un échange à deux échanges existants sans dupliquer son contenu.

- **CA-16** — Créer une réserve depuis un extrait sans disposer encore de note de destination, puis la retrouver dans les points ouverts.

- **CA-17** — Rattacher ultérieurement cette réserve à une note personnelle avec navigation dans les deux sens, sans duplication du passage.

- **CA-18** — Lier une réserve à une note sans la clôturer ; distinguer lien de pertinence, proposition de traitement et résolution confirmée.

- **CA-19** — Résoudre une réserve pour une variante tout en conservant son état ouvert dans une autre variante concernée.

- **CA-20** — Après abandon d’une idée, retrouver les notes développées à partir d’elle dans une vue de réexamen, avec leurs relations internes et externes conservées.

- **CA-21** — Réattacher une note ou un groupe sans dupliquer leur contenu ni perdre les rattachements antérieurs.

- **CA-22** — Retirer un élément des ressources actuelles et le retrouver dans l’historique sans consulter Git.

- **CA-23** — Retirer un rattachement dans une variante sans supprimer les rattachements actifs dans les autres variantes.

- **CA-24** — Adapter une note en conservant sa version originale et le lien vers sa révision.

- **CA-25** — Après annulation d’un retrait ou reconstitution d’un état passé, retrouver les relations alors actives.

- **CA-26** — Annuler en une seule action l’invalidation d’une sous-idée et tous ses effets calculés, puis rétablir cette opération.

- **CA-27** — Après réouverture, restaurer une idée écartée avec ses liens sans revenir sur les changements indépendants du reste du projet.

- **CA-28** — Retrouver un état abandonné après une annulation suivie d’un nouveau travail ; ne pas perdre cette piste historique.

- **CA-29** — Invalider une structure de données liée à une politique d’éviction sans invalider le scheduler ni sa catégorie.

- **CA-30** — Retrouver une idée d’implémentation par son identifiant en commentaire, même après déplacement du code.

- **CA-31** — Lister plusieurs occurrences d’un même identifiant et signaler les repères inconnus ou disparus.

- **CA-32** — Restaurer un état du réseau sans écraser le code courant ; afficher les ancrages devenus incompatibles.

- **CA-33** — Annuler une restauration et retrouver l’état précédant celle-ci.

- **CA-34** — Engager depuis main une modification critique et constater la création d’une branche avant son application, sans modification de main ; engagée sur une branche expérimentale, constater qu’elle s’y poursuit (D1, D13).

- **CA-35** — Revenir à main, puis rouvrir la branche expérimentale et retrouver ses notes, relations et états.

- **CA-36** — Vérifier qu’aucune sauvegarde ou fermeture ne fusionne automatiquement vers main.

- **CA-37** — Fusionner sur demande deux branches compatibles et présenter les conflits d’objets ou de relations s’ils existent.

- **CA-38** — Déclarer une note liée à du code, copier le commentaire généré et retrouver la note après détection du repère collé.

- **CA-39** — Modifier, réattacher et fusionner la note sans changer l’identifiant inscrit dans le commentaire.

- **CA-40** — Conserver une modification ordinaire non encore enregistrée dans un jalon lors de la création de la branche critique.

## 13. Questions à trancher avant l’architecture technique

- Le travail quotidien doit-il se faire d’abord autour du code, d’une carte spatiale ou de la conversation ?

- Quelle présentation du réseau rend le placement le plus rapide : graphe spatial, fil ramifié ou panneaux liés ? Le copier-coller est accepté ; la conversation intégrée n’est pas requise.

- Quelle plateforme doit être utilisable en premier ?

- Quel effort de classement manuel est acceptable après un échange ?

Prochaine étape proposée : décrire un parcours réel de dix minutes sur le mini-RTOS, puis dessiner les écrans nécessaires. Choisir ensuite la forme du logiciel et ses technologies.
