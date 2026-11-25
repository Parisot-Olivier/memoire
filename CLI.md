# Ligne de commande — conception de la phase 4

Version 0.3 — 25 octobre 2026 — validée par l’utilisateur le 25 octobre 2026, avant le code ; sections 5 à 7 pour l’incrément 4.1, section 8 pour l’incrément 4.2, rédigées avant leur code.

## 1. Rôle

La crate `memoire-cli` fournit l’exécutable `memoire`, première interface utilisable au quotidien. Elle ne contient aucune règle métier : chaque commande passe par la couche service (préparer, afficher l’aperçu si besoin, appliquer).

Aucune dépendance nouvelle (D14) : la lecture des arguments est écrite à la main, sur une grammaire courte et stricte ; toute option inconnue est refusée.

## 2. Principes d’usage

- Base : option `--base <fichier>`, sinon variable `MEMOIRE_BASE`, sinon `.memoire/depot.db` dans le répertoire courant.
- Identifiants : affichés en entier ; en entrée, un préfixe d’au moins 8 caractères est accepté s’il désigne un seul objet, sinon refus avec la liste des candidats.
- Motif : obligatoire (`--motif`) pour tout changement de statut (D12) ; refusé s’il manque, sans écriture.
- Opération critique (D13) : l’aperçu (branche créée, causes de réexamen) est affiché ; l’opération n’est appliquée qu’avec `--oui`, sinon rien n’est écrit.
- Textes longs (question, réponse) : lus depuis un fichier (`--question-fichier`, `--reponse-fichier`) ou l’entrée standard, octet pour octet.
- Sorties en français, stables, lisibles ; code de retour non nul en cas de refus.

## 3. Commandes de la première version

| Commande | Effet |
|---|---|
| `init` | crée la base |
| `statut` | branche active, ressources actuelles, notes à réexaminer avec leurs causes, orphelines (appuis tous écartés, causes acquittées : R6, D24, incrément 5.5) |
| `note creer` | crée une note (rôle, titre, texte) et affiche son identifiant ; avec `--classe-dans`, `--depend-de`, `--developpe-a-partir-de` ou `--lien NOTES` (notes séparées par des virgules, identifiant ou préfixe d’au moins 8 caractères), la crée et la relie en une seule opération, qu’un seul `annuler` défait (R1, incrément 5.3, LOT.md, D22) ; la note créée est l’origine de chaque relation ; une liste mal formée, une note inconnue ou citée deux fois pour le même type : code 2 ; opération critique : `--oui` |
| `echange coller` | enregistre question et réponse, éventuellement à la suite d’un échange précédent |
| `citer NOTE --texte X [--partie question\|reponse] [--occurrence N] [--motif M] [--oui]` | remplace `passage` (incrément 5.5, D18, D24) : cite le texte exact d’une partie d’un échange (la réponse par défaut) ou de la version courante d’une note (raisonnement, implémentation, note personnelle), et la relie à sa source par « developpe-a-partir-de », en un seul lot ; sortie « Citation créée : ID (octets A à B) » puis « Relation créée : ID (developpe-a-partir-de SOURCE) » ; une citation dont la note source a été révisée s’affiche suivie de « (source révisée) » ; le rôle s’affiche « citation » |
| `citer` (incrément 6.1) | accepte aussi `--plage A-B` (octets UTF-8 de la source, fin exclue) au lieu de `--texte`, et `--version N` pour citer une version ancienne d’une note ; une plage qui coupe un caractère ou sort du texte est un refus du domaine (code 1) |
| `citer` (incrément 6.2, CENTRE.md) | ajoute à la même opération : `--soutient NOTES` (chaque note « dépend de » la citation), `--inspire NOTES` (chaque note « développée à partir de » la citation), `--classe-dans NOTES` et `--lien NOTES` (de la citation vers les notes), `--note TITRE [--role R] [--texte-note X]` (note nouvelle développée à partir de la citation), `--reserve TITRE [--cible C]` (réserve née d’une citation d’échange) ; une ligne de compte rendu par objet créé ; refus d’un élément : « élément k du lot : … » |
| `source NOTE [--partie question\|reponse] [--version N]` | texte exact d’une partie d’un échange (la réponse par défaut) ou d’une version d’une note (la courante par défaut), octet pour octet, sans rien ajouter (incrément 6.1, VISIONNEUR.md, D25) |
| `voisinage NOTE [--tsv]` | vue locale (incrément 6.6, VOISINAGE.md, INV-VIEW-02) : relations actives de la note dans le contexte par défaut, groupées par origine (note entière, citation, catégorie), avec la flèche du sens, le type, la note voisine et son état (écartée, à réexaminer, orpheline), la source d’une citation ; `--tsv` : en-tête `relation sens type origine voisine role nom source etat` puis une ligne par lien, séparées par des tabulations, forme stable lue par l’interface ; lecture seule |
| `citations NOTE [--partie question\|reponse] [--version N]` | une ligne `ID<tab>DEBUT<tab>FIN<tab>STATUT` par citation de cette source, par début croissant, en octets ; STATUT est le verdict, suivi de `,source-revisee` le cas échéant |
| `retirer NOTE [--motif M] [--oui]` | retire une note ou un échange des ressources actuelles (archivage, sans suppression ; incrément 5.5, D18 point 4) ; opération critique si des dépendants passent à réexaminer ; « Retrait enregistré. » |
| `reserve creer`, `reserve traiter`, `reserve etat` | réserves et leur cycle de vie |
| `lier`, `detacher`, `remplacer`, `restreindre`, `contredire` | relations |
| `verdict`, `decision`, `implementation`, `acquitter` | statuts |
| `ancrer` | ancre une note dans un fichier (fonction désignée par sa signature ou plage de lignes) et affiche le repère à copier |
| `repere` | depuis une ligne de code contenant un ou plusieurs repères, quelle que soit la syntaxe de commentaire (incrément 7.2, RETROUVER.md) : la fiche de chaque note (rôle, statuts, appuis, désaccords, réserves, ancrages) |
| `code notes FICHIER [--ligne N \| --extrait TEXTE] [--tsv]` | notes d’un fichier de code (incrément 7.2, RETROUVER.md, INV-CODE-07) : ancrages actifs du fichier (avec la ligne ou l’extrait qu’ils contiennent en tête), « Chemin proche » à part (même nom de fichier, autre chemin), marques du fichier (connue, note absente de cette piste, mal formée ; la plus proche au-dessus de la ligne, ou celles couvertes par l’extrait) ; chemins relatifs au dossier courant ; lecture seule ; code 2 pour un fichier absent, une ligne hors du fichier, ou --ligne avec --extrait |
| `indexer DOSSIER [--extensions LISTE]` | parcourt le code (incrément 6.3, INDEXEUR.md) et rend l’état des repères `@design:` et des ancrages : retrouvés, modifiés (extrait ancré absent), déplacés (chemin de l’ancrage périmé), ambigus (plusieurs occurrences, aucune choisie), perdus, inconnus (avec les branches où la note existe), non ancrés, mal formés ; lecture seule, jamais d’écriture dans le code ni dans le dépôt ; dossiers cachés, `target` et `node_modules` sautés ; fichiers non UTF-8, de plus de 4 Mo, liens et dossiers illisibles comptés comme ignorés |
| `annuler`, `retablir`, `branches`, `basculer` | historique |
| `historique [--nombre N]`, `restauration plan --depuis COMMIT [--notes NOTES]`, `restaurer --depuis COMMIT [--notes NOTES] [--motif M] [--oui]` | historique de la branche active (« ID — commande — motif », du plus récent au plus ancien) ; plan et restauration ciblée des notes choisies avec leurs liens internes, sans toucher le reste, ou complète sans `--notes` (incrément 6.4, RESTAURATION.md) ; COMMIT : identifiant ou préfixe d’un commit de l’historique ; opération critique, annulable d’un seul `annuler` |
| `reprise noter --objectif TEXTE --action TEXTE [--question TEXTE] [--etat TEXTE] [--resultat TEXTE] [--hypothese TEXTE] [--ouverts TEXTE] [--notes NOTES] [--code DOSSIER] [--sans-code] [--motif M]`, `reprise voir [--depuis COMMIT] [--code DOSSIER]`, `reprise liste [--toutes] [--nombre N]`, `reprise code COMMIT` | fiche de reprise (incrément 6.5, FICHE-REPRISE.md, D26) : `noter` publie un commit sans changer le réseau, avec la date UTC à la minute et l’état du code lu par `git` en lecture seule dans `--code` (par défaut le répertoire courant) : commit, branche, fichiers changés (la base et ses fichiers associés exclus), travail non enregistré conservé en patch (jusqu’à 4 Mo, en UTF-8) ; sans dépôt Git, la fiche est notée sans état du code ; `voir` affiche en tête objectif, question et prochaine action, puis « Depuis : » les opérations du réseau et le changement du code (commits depuis, historique réécrit, travail différent) ; `liste` : « ID — date — objectif », `--toutes` par branche active, chaque branche ne montrant que ses fiches notées depuis main ; `code` écrit le patch noté, à réappliquer par `git apply` (code 2 si la fiche n’en a pas) ; `annuler` juste après une fiche la défait et le dit ; revenir à une fiche côté réseau : `restaurer --depuis` |
| `abandonner BRANCHE [--motif M]`, `reactiver BRANCHE [--motif M]`, `branches [--toutes]` | abandon d’une branche sans rien supprimer, retour sur l’origine si elle était active, réactivation sans la rendre active ; `branches` omet les branches abandonnées, `--toutes` les montre marquées « (abandonnée) » ; pas de confirmation : l’abandon se défait par `reactiver` (incrément 5.4a, FUSION.md, section 9.9, D23) |
| `branches --tsv`, `fusion plan SOURCE --tsv` | sorties pour le panneau des actions (incrément 7.1b, ACTIONS.md, section 7, D30) : `branches --tsv` donne l’en-tête `piste active mise-de-cote nom motif` puis une ligne par branche, avec un nom lisible tiré de l’opération qui l’a créée (« piste principale », « Réfutation de « … » », « Mise de côté de « … » », « Retrait de « … » », « Retour à l’étape … », sinon le mot de la commande), doublons numérotés « (2) » dans l’ordre des identifiants ; `fusion plan --tsv` donne `code`, `base`, `sans-conflit`, une ligne `conflit` par désaccord (rang, genre texte, champ ou acquittement, note, titre, champ, valeur ici, valeur dans la branche), `boucle` et ses lignes `candidate`, puis `a-revoir` et `plus-a-revoir` ; une branche déjà réunie donne `a-jour oui` ; champs séparés par des tabulations, nettoyés des tabulations et retours à la ligne |
| `fusion plan SOURCE`, `fusionner SOURCE --plan CODE [--cible NUMEROS] [--source NUMEROS] [--desactiver NUMEROS] [--motif M] [--oui]` | fusion de SOURCE dans la branche active (incrément 5.4, FUSION.md, section 9.9, D19, D23) : `fusion plan` montre la base, le nombre d’objets réunis sans conflit, les conflits numérotés avec la valeur de chaque côté, le conflit structurel et ses relations candidates numérotées, les effets sur les réexamens et le code du plan ; `fusionner` recalcule le plan, refuse un code périmé (code 2), traduit les numéros en résolutions, demande `--oui` si des réexamens changent (code 4), écrit `sauvegardes/avant-fusion-AAAAMMJJ-HHMMSSZ.db` puis publie un commit à deux parents, qu’un seul `annuler` défait ; conflit restant : refus du domaine (code 1), sans sauvegarde |
| `exporter`, `importer` | texte canonique pour Git |
| `verifier` | vérification complète de la base (incrément 5.2c, D21) : tout l’historique relu et validé, l’état courant de chaque branche confirmé |
| `migrer` | conversion d’une base de schéma 1 au schéma 2 (incrément 5.2d, ETAT-HISTORIQUE.md, sections 7 et 16) : sauvegarde `sauvegardes/avant-migration-AAAAMMJJ-HHMMSSZ.db`, conversion dans un nouveau fichier, vérification complète, puis remplacement atomique ; jamais automatique. Une base déjà au schéma 2 n’est pas touchée (code 0) ; tout échec sort en code 3, la base d’origine intacte |

## 4. Tests

Les tests lancent l’exécutable lui-même dans un répertoire temporaire et vérifient sa sortie, son code de retour et le dépôt obtenu. Le test de recette rejoue S-REF-01 en entier par la ligne de commande sur `fixtures/rtos-2026-10/doc.md` et `driver.c`, et compare le dépôt final au test du domaine.

## 5. Ensemble délimité de l’incrément 4.1

La première version de l’exécutable couvre chaque mécanisme une fois, sur le contexte par défaut :

| Commande | Mécanisme exercé |
|---|---|
| `init` | création de la base |
| `statut` | lecture : branche, ressources actuelles, notes à réexaminer et leurs causes |
| `note creer --role R --titre T [--texte X] [--classe-dans NOTES] [--depend-de NOTES] [--developpe-a-partir-de NOTES] [--lien NOTES] [--motif M] [--oui]` | identités attribuées par le service ; avec des liens, `Command::Lot` : création, puis une relation par note, dans l’ordre des options puis des notes |
| `lier ORIGINE TYPE DESTINATION [--oui]` | relations sans annotation : `classe-dans`, `depend-de`, `developpe-a-partir-de`, `lien` |
| `verdict NOTE VALEUR --motif M [--oui]`, `decision NOTE VALEUR --motif M [--oui]` | statuts, motif obligatoire, opération critique |
| `annuler`, `retablir` | historique |
| `branches`, `basculer BRANCHE` | branches |
| `exporter FICHIER`, `importer FICHIER` | texte canonique pour Git |
| `aide [COMMANDE]` | aide générée depuis la table des commandes |

Les échanges, passages, réserves, ancrages, repères, restrictions, contradictions, remplacements, acquittements, contextes et restaurations viennent aux incréments suivants.

**Opération critique.** L’aperçu est affiché avant le compte rendu, y compris avec `--oui`. Une commande est présentée comme critique si son aperçu crée une branche ou modifie une cause de réexamen. L’aperçu est affiché ; sans `--oui`, rien n’est écrit et le code de retour est 4. Les déplacements dans l’historique (`annuler`, `retablir`, `basculer`) ne demandent pas de confirmation : ils sont eux-mêmes réversibles.

**Lecture des arguments.** Les options se placent n’importe où après le nom de la commande ; une valeur d’option ne peut pas commencer par `--`. `--base` se place avant le nom de la commande.

**Codes de retour.** 0 : réussite ; 1 : refus du domaine (règle métier) ; 2 : usage incorrect (commande, option ou argument) ; 3 : erreur de stockage ou d’entrée-sortie, y compris un historique illisible (incrément 5.2c) ; 4 : confirmation requise, rien d’écrit.

## 6. Architecture

- `src/main.rs` ne fait que réunir les arguments, l’environnement, la source d’identités (UUID v4) et les flux, puis appelle `memoire_cli::executer` et rend son code de retour.
- `src/arguments.rs` : lecture stricte ; toute option inconnue, répétée ou sans valeur, et tout argument en trop, sont refusés (code 2).
- `src/commandes/` : une commande par type implémentant le trait `Commande` (nom, résumé, usage, exécution) ; une seule table, `COMMANDES`, les recense dans l’ordre de l’aide.
- `executer` choisit la commande dont le nom (un ou deux mots) ouvre la ligne, ouvre la base, exécute, et traduit l’échec en code de retour.

## 7. Ajouter une commande

1. Écrire d’abord son test dans `tests/cli.rs` : sortie attendue, code de retour, dépôt obtenu, cas de refus.
2. Créer son type dans `src/commandes/`, implémenter `Commande`, en passant par la couche service pour toute écriture (`preparer_et_appliquer` gère aperçu, confirmation et motif).
3. L’ajouter à la table `COMMANDES` ; l’aide la liste automatiquement.
4. Compléter la section 5 de ce document. Aucune autre partie du programme n’est à modifier.

## 8. Incrément 4.2 : de quoi rejouer le scénario de référence

Rédigé avant le code. Toutes ces commandes s’ajoutent par la procédure de la section 7, sans toucher aux autres.

| Commande | Effet |
|---|---|
| `echange coller --titre T --reponse-fichier R [--question-fichier Q] [--precedent ECHANGE]` | enregistre un échange, octet pour octet ; sans question, il reste incomplet |
| `echange question ECHANGE --fichier Q` | complète la question d’un échange incomplet |
| `passage ECHANGE --texte X [--partie question\|reponse] [--occurrence N]` | sélectionne le passage dont le texte est exactement X ; plusieurs occurrences sans `--occurrence` : refus avec leur nombre |
| `reserve creer PASSAGE --titre T [--cible C]` | crée une réserve depuis un passage |
| `reserve concerne RESERVE NOTE` | la réserve s’applique à la note |
| `reserve traiter NOTE RESERVE --motif M` | la note traite la réserve (« traitée à vérifier ») ; motif obligatoire, le statut de la réserve changeant (D12) |
| `reserve etat RESERVE ouverte\|traitee-a-verifier\|resolue --motif M` | cycle de vie de la réserve |
| `restreindre ORIGINE DESTINATION --condition C [--oui]` | restriction de périmètre |
| `contredire ORIGINE DESTINATION --justification J` | désaccord assumé |
| `acquitter DEPENDANT CAUSE --motif M [--oui]` | confirme qu’une note reste pertinente malgré une cause |
| `implementation NOTE non-commencee\|en-cours\|implementee --motif M` | état d’implémentation |
| `note reviser NOTE --texte X --nature mineure\|substantielle [--oui]` | révision ; la nature est déclarée par l’utilisateur, sans valeur implicite (D5) |
| `ancrer NOTE --fichier F (--fonction SIGNATURE \| --lignes A-B) [--git COMMIT]` | ancre la note dans le code et affiche le repère à copier ; le fichier n’est jamais modifié (D9) |
| `repere LIGNE` | depuis une ligne de code contenant un repère : note, décision, sources, désaccords, restrictions, réserves, ancrages |

- **Fonction** : l’extrait va de la première occurrence exacte de SIGNATURE jusqu’à la première ligne qui commence par `}` ; une signature absente ou présente plusieurs fois est refusée.
- **Lignes** : plage inclusive, numérotée à partir de 1, dans les bornes du fichier.
- **Test de recette** : S-REF-01 rejoué en entier par la ligne de commande, sur `fixtures/rtos-2026-10/doc.md` et `driver.c`, avec les quatre réponses de l’étape 7 lues dans la sortie de `repere`.

**Sorties.** `echange coller` : « Échange créé : ID » ; `passage` : « Passage créé : ID (octets A à B) » ; `reserve creer` : « Réserve créée : ID » ; relations : « Relation créée : ID » ; statuts : « Statut enregistré. » ; `acquitter` : « Acquittement enregistré. » ; `note reviser` : « Révision enregistrée. » ; `ancrer` : « Ancrage créé : ID » puis « Repère à copier : // @design: ID ». Une opération qui bifurque ajoute « Branche créée : ID ».

**`repere`** affiche, dans cet ordre : `Note : TITRE (ID)`, `Rôle`, `Décision`, `Vérification` et `Implémentation` s’ils existent, puis les sections `S'appuie sur`, `Désaccords` (avec leur justification), `Réserves de la note`, `Réserves de ses catégories` (réserves qui visent une note de la même catégorie ou qui proviennent d’un échange classé dans cette catégorie, avec leur état et leur cible) et `Ancrages`. Un passage est désigné par son texte entre guillemets.
