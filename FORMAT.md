# Format textuel des instantanés — `memoire-instantane`, version 1

Version du document : 1.2 — 5 novembre 2026 — incrément 3.1 (instantanés) ; section 12 (dépôt complet) rédigée avant l’implémentation de l’incrément 3.1b, puis complétée de son exemple de référence ; section 13 (version 2 du format) ajoutée à l’incrément 5.1.

Ce document est la référence du format. Le code de `crates/memoire-domain/src/format.rs` l’implémente ; le contrat (`crates/memoire-domain/tests/contrat.rs`) et les tests internes du module le vérifient, y compris contre l’exemple de la section 9, figé octet pour octet dans `fixtures/format/exemple-v1.txt`. Toute modification du format suit la procédure de la section 10.

## 1. Rôle et garanties

Le format représente un **instantané** : l’état complet du réseau à un commit (contenu canonique des notes, contextes, relations, acquittements, ancrages). Il sert au stockage (phase 3, une ligne de table SQLite par instantané) et à l’export textuel destiné à Git (D4).

Garanties, vérifiées par les tests :

- **Fidélité** : décoder un texte encodé redonne exactement l’instantané d’origine (égalité complète).
- **Déterminisme** : un même instantané donne toujours les mêmes octets.
- **Forme canonique unique** : le décodeur n’accepte que la forme produite par l’encodeur. Pour tout texte accepté, réencoder l’instantané décodé redonne ce texte, octet pour octet.
- **Refus explicite** : tout écart (en-tête, ordre, champ, valeur, état invalide) est refusé avec le numéro de ligne et un motif. Aucun instantané partiellement chargé n’est jamais rendu.
- **Validation** : un instantané décodé est validé par les mêmes règles que tout état publié (`Snapshot::validate`).

Le format n’est pas un format de transport entre versions du logiciel : la compatibilité est assurée par un numéro de version et des migrations explicites (section 10).

## 2. Structure générale

- Encodage : UTF-8, sans marque d’ordre d’octets.
- Fin de ligne : un seul caractère LF (U+000A) ; CR n’apparaît jamais hors chaîne échappée.
- Le texte est une suite de lignes, chacune terminée par LF, y compris la dernière.
- Première ligne : `memoire-instantane 1`.
- Dernière ligne : `fin`.
- Aucune ligne vide, aucun commentaire, aucune espace en début ou fin de ligne.

## 3. Jetons

Une ligne est une suite de jetons séparés par **une seule** espace (U+0020).

| Jeton | Forme | Exemple |
|---|---|---|
| mot | lettres minuscules ASCII, chiffres et `-`, `:` | `contenu`, `plus-retenue`, `decision:retenue` |
| identifiant | forme canonique UUID, 36 caractères minuscules | `7e000000-0000-0000-0000-000000000001` |
| entier | décimal non signé, sans zéro initial (sauf `0`) | `0`, `12` |
| empreinte | exactement 16 chiffres hexadécimaux minuscules | `af63dc4c8601ec8c` |
| chaîne | entre guillemets droits, échappée (section 4) | `"titre \"cité\""` |
| absence | le mot `-`, pour un champ facultatif absent | `-` |

Une chaîne absente s’écrit `-` ; la chaîne contenant le seul caractère tiret s’écrit `"-"`. Les deux ne se confondent jamais.

## 4. Chaînes

Une chaîne est délimitée par `"`. À l’intérieur, exactement cinq séquences d’échappement existent :

| Séquence | Caractère |
|---|---|
| `\\` | barre oblique inverse |
| `\"` | guillemet droit |
| `\n` | LF (U+000A) |
| `\r` | CR (U+000D) |
| `\t` | tabulation (U+0009) |

Tout autre caractère, y compris les espaces insécables, les caractères invisibles (U+2060, U+FFFC) et les emoji, est écrit tel quel. Une barre oblique inverse suivie d’un autre caractère est refusée. L’encodeur échappe toujours ces cinq caractères et n’échappe rien d’autre : la forme est donc unique.

## 5. Enregistrements

Chaque ligne commence par un mot-clé. Les champs suivent dans l’ordre indiqué ; les étiquettes (`titre`, `texte`, `verdict`…) sont des mots littéraux qui doivent figurer exactement. Notation : `<id>` identifiant, `<n>` entier, `<ch>` chaîne, `<ch|->` chaîne ou absence.

### 5.1 Contenu canonique des notes (partagé entre contextes)

```
contenu <note> <rôle> titre <ch> texte <ch>
revision <note> <nature> titre <ch> texte <ch>
echange <note> question <ch|-> reponse <ch> precedent <id|->
passage <note> echange <id> partie <partie> debut <n> fin <n>
reserve <note> passage <id> cible <ch|->
```

- `contenu` : version de création de la note. `<rôle>` vaut `categorie`, `raisonnement`, `implementation`, `note`, `echange`, `passage` ou `reserve`.
- `revision` : une révision ultérieure ; `<nature>` vaut `mineure` ou `substantielle`. Les révisions suivent immédiatement leur `contenu`, dans l’ordre chronologique.
- `echange`, `passage`, `reserve` : données propres du rôle, juste après les révisions de la note. Une note de rôle `echange`, `passage` ou `reserve` a exactement un tel enregistrement ; les autres n’en ont aucun. `<partie>` vaut `question` ou `reponse` ; `debut` et `fin` sont des positions en octets UTF-8, fin exclusive.
- Le texte d’un passage, dans son `contenu`, doit être exactement la plage désignée ; la validation le vérifie.

### 5.2 Ancrages de code

```
ancrage <id> note <id> etat <actif|retire> chemin <ch> extrait <ch> empreinte <hex16> git <ch|->
```

L’empreinte est celle de l’extrait (FNV-1a sur 64 bits, `text_digest`) ; la validation vérifie leur concordance.

### 5.3 Contextes

```
contexte <portée>
membre <portée> <note> verdict <v|-> decision <d|-> implementation <i|-> reserve <r|-> visibilite <vis>
relation <portée> <relation> <origine> <destination> <type> etat <active|inactive> empreinte <n> <n> annotation <ch|-> anterieur <état|->
acquittement <portée> <dépendant> <cause> sens <n> empreinte <n> <n> verdict <v|-> decision <d|-> visibilite <vis>
```

Valeurs :

| Champ | Valeurs |
|---|---|
| `verdict` | `non-verifiee`, `verifiee`, `refutee` |
| `decision` | `proposee`, `retenue`, `plus-retenue`, `remplacee` |
| `implementation` | `non-commencee`, `en-cours`, `implementee` |
| `reserve` | `ouverte`, `traitee-a-verifier`, `resolue` |
| `visibilite` | `courante`, `retiree` |
| `<type>` de relation | `classe-dans`, `depend-de`, `developpe-a-partir-de`, `lien`, `remplace`, `restreint`, `contredit`, `s-applique-a`, `traite` |
| `anterieur` | `decision:<d>`, `reserve:<r>` ou `-` |

- La portée est répétée sur chaque ligne de son bloc : une ligne isolée reste lisible et un écart est détecté.
- `relation … empreinte <a> <b>` : nombre de révisions substantielles et de restrictions actives de la destination à l’activation de la relation.
- `acquittement … sens <n>` : nombre de révisions substantielles du dépendant confirmé ; `empreinte`, `verdict`, `decision`, `visibilite` : état de la cause confirmée.

## 6. Ordre canonique

1. `memoire-instantane 1`.
2. Les notes, par identifiant croissant (ordre des 16 octets) : pour chacune, `contenu`, puis ses `revision`, puis son éventuel enregistrement de données propres.
3. Les `ancrage`, par identifiant croissant.
4. Les contextes, par identifiant croissant : pour chacun, `contexte`, puis ses `membre` par note croissante, ses `relation` par identifiant croissant, ses `acquittement` par couple (dépendant, cause) croissant.
5. `fin`.

Un ordre différent, un doublon ou une ligne hors de son bloc est refusé : l’ordre est vérifié strictement croissant.

## 7. Validation après lecture

Une fois toutes les lignes lues, l’instantané est validé par `Snapshot::validate`. Les règles comprennent : présence du contexte par défaut, concordance rôle et données propres, plage exacte des passages, statuts présents selon le rôle, extrémités et règles des relations, absence d’auto-relation, de doublon actif et de cycle, cohérence entre « remplacée » et « remplace », acquittements entre notes existantes, ancrages complets et empreinte cohérente, contenu de chaque contexte égal au contenu canonique. Un échec est rendu avec la ligne 0 (état global) et l’erreur du domaine. Exception : un membre dont la note n’a aucun contenu est signalé à sa propre ligne, avec l’erreur « note absente ».

## 8. Erreurs de lecture

Toute erreur indique la ligne, numérotée à partir de 1 (0 pour la validation globale), et un motif :

| Motif | Cas |
|---|---|
| en-tête | première ligne absente ou différente de `memoire-instantane 1` |
| fin manquante | texte terminé sans `fin`, ou dernière ligne sans LF |
| après la fin | contenu après `fin` |
| enregistrement inconnu | mot-clé non défini dans cette version |
| champ | étiquette attendue absente, champ manquant ou en trop |
| valeur | jeton mal formé : identifiant, entier, empreinte, chaîne, valeur énumérée |
| ordre | ordre canonique non respecté, doublon |
| hors bloc | `revision` ou données propres sans leur `contenu`, ligne de contexte sans son `contexte` ou d’une autre portée |
| état invalide | l’instantané lu viole une règle du domaine |

## 9. Exemple de référence

Instantané d’un échange incomplet, d’un passage, d’une réserve, d’une note révisée, d’une dépendance et d’un ancrage, dans le contexte par défaut (identifiant nul). Ce texte est figé dans `fixtures/format/exemple-v1.txt` ; toute différence fait échouer le contrat. La réponse de l’échange se termine par un emoji suivi des caractères invisibles U+2060 et U+FFFC, écrits tels quels.

```
memoire-instantane 1
contenu a5000000-0000-0000-0000-000000000001 echange titre "conversation" texte ""
echange a5000000-0000-0000-0000-000000000001 question - reponse "Élément\tRôle\nfin \"cité\" \\ 🚀⁠￼" precedent -
contenu a5000000-0000-0000-0000-000000000002 passage titre "" texte "Élément"
passage a5000000-0000-0000-0000-000000000002 echange a5000000-0000-0000-0000-000000000001 partie reponse debut 0 fin 9
contenu a5000000-0000-0000-0000-000000000003 reserve titre "réserve" texte ""
reserve a5000000-0000-0000-0000-000000000003 passage a5000000-0000-0000-0000-000000000002 cible "startup.s (à créer)"
contenu a5000000-0000-0000-0000-000000000004 implementation titre "N" texte "v0"
revision a5000000-0000-0000-0000-000000000004 mineure titre "N" texte "v1"
ancrage a5000000-0000-0000-0000-000000000006 note a5000000-0000-0000-0000-000000000004 etat actif chemin "src/driver.c" extrait "void refresh() {}" empreinte 3f38db1c879fb6fd git -
contexte 00000000-0000-0000-0000-000000000000
membre 00000000-0000-0000-0000-000000000000 a5000000-0000-0000-0000-000000000001 verdict - decision - implementation - reserve - visibilite courante
membre 00000000-0000-0000-0000-000000000000 a5000000-0000-0000-0000-000000000002 verdict non-verifiee decision proposee implementation - reserve - visibilite courante
membre 00000000-0000-0000-0000-000000000000 a5000000-0000-0000-0000-000000000003 verdict - decision - implementation - reserve ouverte visibilite courante
membre 00000000-0000-0000-0000-000000000000 a5000000-0000-0000-0000-000000000004 verdict non-verifiee decision retenue implementation non-commencee reserve - visibilite courante
relation 00000000-0000-0000-0000-000000000000 a5000000-0000-0000-0000-000000000005 a5000000-0000-0000-0000-000000000004 a5000000-0000-0000-0000-000000000002 depend-de etat active empreinte 0 0 annotation - anterieur -
fin
```

## 10. Faire évoluer le format

1. Toute modification du format, même l’ajout d’un champ facultatif, crée une nouvelle version : `memoire-instantane 2`.
2. La décision est consignée dans DECISIONS.md, et ce document est mis à jour dans le même commit que le code.
3. Le décodeur de la version précédente est conservé ; une fonction de migration transforme un instantané lu en version précédente en instantané courant. Elle est testée sur des textes figés de l’ancienne version, conservés dans `fixtures/format/`.
4. L’exemple de la section 9 est régénéré, relu, puis figé à nouveau ; le changement de l’exemple est un changement de contrat isolé dans son propre commit.
5. Ajouter une valeur à une énumération du domaine (rôle, statut, type de relation) impose d’ajouter sa forme textuelle ici et dans `format.rs` : le compilateur signale les correspondances manquantes, les correspondances étant écrites sans cas générique.

## 11. Diagnostic

Pour corriger un problème de lecture : le numéro de ligne et le motif suffisent en général ; la ligne fautive est lisible telle quelle, chaque enregistrement portant ses étiquettes. Pour comparer deux états, un `diff` ligne à ligne des textes est significatif : l’ordre est canonique et une modification ne touche que les lignes concernées.

## 12. Dépôt complet — `memoire-depot`, version 1

Spécification rédigée avant l’implémentation (incrément 3.1b). Elle reprend les jetons, chaînes et règles de forme canonique des sections 2 à 4. Le dépôt complet sert à l’export avec historique et au contrôle de la persistance : un dépôt exporté puis relu doit être égal, champ pour champ, au dépôt d’origine (INV-EXPORT-01).

### 12.1 Structure

```
memoire-depot 1
active <branche>
branche <id> tete <commit> origine <branche|-> base <commit>
retablir <branche> <rang> commit <commit>
retablir <branche> <rang> entrer <branche> tete <commit>
identite <categorie> <id>
commit <id> parent <commit|-> motif <ch>
  <instantané, section 2 à 6, chaque ligne préfixée de deux espaces>
operation <op> attendu <branche> <commit> motif <ch>
  commande <nom> <champs>
  resultat branche <branche> tete <commit> publie <oui|non> bifurque <oui|non>
  effet <portée> <note> avant <k> <id>… apres <m> <id>…
fin
```

- `active` : branche active.
- `branche` : tête, branche d’origine (absente pour main), commit de base (`00000000-…` pour main).
- `retablir` : pile de rétablissement de la branche, du bas vers le haut, `<rang>` commençant à 0 ; une entrée désigne soit un commit annulé, soit la rentrée dans une branche quittée avec la tête qu’elle avait (D1).
- `identite` : identités déjà employées, y compris par des états abandonnés (INV-ID-02). `<categorie>` vaut `note`, `relation`, `contexte` ou `ancrage`.
- `commit` : parent (absent pour le commit racine), motif, puis l’instantané complet de ce commit, encodé selon les sections 2 à 6 et indenté de deux espaces, en-tête `memoire-instantane 1` et `fin` compris.
- `operation` : une entrée du journal (INV-RETRY-01) : état attendu, motif, commande, résultat et effets annoncés. `effet` donne, pour une note d’un contexte, le nombre puis la liste de ses causes de réexamen avant et après l’opération.

### 12.2 Commandes

| Nom | Champs |
|---|---|
| `creer-note` | `portee <id> note <id> role <rôle> titre <ch> texte <ch>` |
| `reviser` | `note <id> texte <ch> nature <nature>` |
| `inclure` | `portee <id> note <id>` |
| `copier-contexte` | `source <id> contexte <id>` |
| `lier` | `portee <id> relation <id> origine <id> destination <id> type <type>` |
| `verdict` | `portee <id> note <id> valeur <verdict>` |
| `decision` | `portee <id> note <id> valeur <décision>` |
| `implementation` | `portee <id> note <id> valeur <implémentation>` |
| `retirer` | `portee <id> note <id>` |
| `reattacher` | `portee <id> relation <id> destination <id> nouvelle <id>` |
| `remplacer` | `portee <id> relation <id> remplacante <id> remplacee <id>` |
| `restreindre` | `portee <id> relation <id> origine <id> destination <id> condition <ch>` |
| `contredire` | `portee <id> relation <id> origine <id> destination <id> justification <ch>` |
| `creer-echange` | `portee <id> note <id> titre <ch> question <ch\|-> reponse <ch> precedent <id\|->` |
| `completer-question` | `echange <id> question <ch>` |
| `creer-passage` | `portee <id> note <id> echange <id> partie <partie> debut <n> fin <n>` |
| `creer-reserve` | `portee <id> note <id> passage <id> titre <ch> cible <ch\|->` |
| `traiter` | `portee <id> relation <id> note <id> reserve <id>` |
| `etat-reserve` | `portee <id> reserve <id> valeur <état de réserve>` |
| `acquitter` | `portee <id> dependant <id> cause <id>` |
| `detacher` | `portee <id> relation <id>` |
| `ancrer` | `ancrage <id> note <id> chemin <ch> extrait <ch> git <ch\|->` |
| `retirer-ancrage` | `ancrage <id>` |
| `restaurer` | `cible <commit>` |
| `annuler` | aucun champ |
| `retablir` | aucun champ |
| `basculer` | `branche <id>` |

### 12.3 Ordre canonique

`active`, puis les `branche` par identifiant croissant, chacune suivie de ses `retablir` par rang croissant ; les `identite` par catégorie (`note`, `relation`, `contexte`, `ancrage`) puis identifiant croissant ; les `commit` par identifiant croissant ; les `operation` par identifiant croissant, chacune suivie de sa commande, de son résultat et de ses effets par couple (portée, note) croissant ; `fin`.

### 12.4 Validation après lecture

Chaque instantané est validé (section 7). Le dépôt doit en outre être cohérent : commit racine nul et sans parent, parents existants, têtes et bases de branches existantes, branche active existante, entrées de rétablissement vers des commits et des branches existants, résultats d’opérations vers des commits et des branches existants, identités employées couvrant toutes les identités présentes dans les instantanés. Un écart est refusé avec le motif « état invalide ».

### 12.5 Exemple de référence

Deux notes, une dépendance, une réfutation qui crée une branche depuis main, puis l’annulation qui ramène sur main : la pile de rétablissement de main contient la rentrée dans la branche quittée. Les identifiants sont ceux du harnais de test ; le motif « motif du harnais » est celui qu’il fournit. Ce texte est figé dans `fixtures/format/depot-v1.txt` ; toute différence fait échouer le contrat.

```
memoire-depot 1
active 00000000-0000-0000-0000-000000000000
branche 00000000-0000-0000-0000-000000000000 tete 00000000-0000-0001-0000-000000000005 origine - base 00000000-0000-0000-0000-000000000000
retablir 00000000-0000-0000-0000-000000000000 0 entrer 00000000-0000-0001-0000-000000000007 tete 00000000-0000-0001-0000-000000000007
branche 00000000-0000-0001-0000-000000000007 tete 00000000-0000-0001-0000-000000000007 origine 00000000-0000-0000-0000-000000000000 base 00000000-0000-0001-0000-000000000005
identite note 00000000-0000-0001-0000-000000000002
identite note 00000000-0000-0001-0000-000000000004
identite relation 00000000-0000-0001-0000-000000000006
identite contexte 00000000-0000-0000-0000-000000000000
commit 00000000-0000-0000-0000-000000000000 parent - motif ""
  memoire-instantane 1
  contexte 00000000-0000-0000-0000-000000000000
  fin
commit 00000000-0000-0001-0000-000000000001 parent 00000000-0000-0000-0000-000000000000 motif "motif du harnais"
  memoire-instantane 1
  contenu 00000000-0000-0001-0000-000000000002 raisonnement titre "hypothèse" texte "texte initial"
  contexte 00000000-0000-0000-0000-000000000000
  membre 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000002 verdict non-verifiee decision proposee implementation non-commencee reserve - visibilite courante
  fin
commit 00000000-0000-0001-0000-000000000003 parent 00000000-0000-0001-0000-000000000001 motif "motif du harnais"
  memoire-instantane 1
  contenu 00000000-0000-0001-0000-000000000002 raisonnement titre "hypothèse" texte "texte initial"
  contenu 00000000-0000-0001-0000-000000000004 implementation titre "s'appuie sur l'hypothèse" texte "texte initial"
  contexte 00000000-0000-0000-0000-000000000000
  membre 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000002 verdict non-verifiee decision proposee implementation non-commencee reserve - visibilite courante
  membre 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000004 verdict non-verifiee decision proposee implementation non-commencee reserve - visibilite courante
  fin
commit 00000000-0000-0001-0000-000000000005 parent 00000000-0000-0001-0000-000000000003 motif "motif du harnais"
  memoire-instantane 1
  contenu 00000000-0000-0001-0000-000000000002 raisonnement titre "hypothèse" texte "texte initial"
  contenu 00000000-0000-0001-0000-000000000004 implementation titre "s'appuie sur l'hypothèse" texte "texte initial"
  contexte 00000000-0000-0000-0000-000000000000
  membre 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000002 verdict non-verifiee decision proposee implementation non-commencee reserve - visibilite courante
  membre 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000004 verdict non-verifiee decision proposee implementation non-commencee reserve - visibilite courante
  relation 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000006 00000000-0000-0001-0000-000000000004 00000000-0000-0001-0000-000000000002 depend-de etat active empreinte 0 0 annotation - anterieur -
  fin
commit 00000000-0000-0001-0000-000000000007 parent 00000000-0000-0001-0000-000000000005 motif "motif du harnais"
  memoire-instantane 1
  contenu 00000000-0000-0001-0000-000000000002 raisonnement titre "hypothèse" texte "texte initial"
  contenu 00000000-0000-0001-0000-000000000004 implementation titre "s'appuie sur l'hypothèse" texte "texte initial"
  contexte 00000000-0000-0000-0000-000000000000
  membre 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000002 verdict refutee decision proposee implementation non-commencee reserve - visibilite courante
  membre 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000004 verdict non-verifiee decision proposee implementation non-commencee reserve - visibilite courante
  relation 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000006 00000000-0000-0001-0000-000000000004 00000000-0000-0001-0000-000000000002 depend-de etat active empreinte 0 0 annotation - anterieur -
  fin
operation 00000000-0000-0001-0000-000000000001 attendu 00000000-0000-0000-0000-000000000000 00000000-0000-0000-0000-000000000000 motif "motif du harnais"
  commande creer-note portee 00000000-0000-0000-0000-000000000000 note 00000000-0000-0001-0000-000000000002 role raisonnement titre "hypothèse" texte "texte initial"
  resultat branche 00000000-0000-0000-0000-000000000000 tete 00000000-0000-0001-0000-000000000001 publie oui bifurque non
operation 00000000-0000-0001-0000-000000000003 attendu 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000001 motif "motif du harnais"
  commande creer-note portee 00000000-0000-0000-0000-000000000000 note 00000000-0000-0001-0000-000000000004 role implementation titre "s'appuie sur l'hypothèse" texte "texte initial"
  resultat branche 00000000-0000-0000-0000-000000000000 tete 00000000-0000-0001-0000-000000000003 publie oui bifurque non
operation 00000000-0000-0001-0000-000000000005 attendu 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000003 motif "motif du harnais"
  commande lier portee 00000000-0000-0000-0000-000000000000 relation 00000000-0000-0001-0000-000000000006 origine 00000000-0000-0001-0000-000000000004 destination 00000000-0000-0001-0000-000000000002 type depend-de
  resultat branche 00000000-0000-0000-0000-000000000000 tete 00000000-0000-0001-0000-000000000005 publie oui bifurque non
operation 00000000-0000-0001-0000-000000000007 attendu 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000005 motif "motif du harnais"
  commande verdict portee 00000000-0000-0000-0000-000000000000 note 00000000-0000-0001-0000-000000000002 valeur refutee
  resultat branche 00000000-0000-0001-0000-000000000007 tete 00000000-0000-0001-0000-000000000007 publie oui bifurque oui
  effet 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000004 avant 0 apres 1 00000000-0000-0001-0000-000000000002
operation 00000000-0000-0001-0000-000000000008 attendu 00000000-0000-0001-0000-000000000007 00000000-0000-0001-0000-000000000007 motif "motif du harnais"
  commande annuler
  resultat branche 00000000-0000-0000-0000-000000000000 tete 00000000-0000-0001-0000-000000000005 publie non bifurque non
  effet 00000000-0000-0000-0000-000000000000 00000000-0000-0001-0000-000000000004 avant 1 00000000-0000-0001-0000-000000000002 apres 0
fin
```

## 13. Version 2 — `memoire-instantane 2` et `memoire-depot 2`

Incrément 5.1. Spécification validée : VERSION2.md, sections 4 à 7 ; cette section décrit ce qui est implémenté. Les décodeurs de la version 1 (sections 2 à 12) sont conservés tels quels, avec leurs exemples figés.

### 13.1 Instantané

Mêmes jetons, chaînes, ordre canonique et validation que la version 1, avec trois changements :

```
revision <note> <rang> <nature> titre <ch> texte <ch>
citation <note> source <id> partie <question|reponse|note> version <n|-> debut <n> fin <n> empreinte <hex16>
```

- `revision` porte son rang, à partir de 1, vérifié à la lecture (motif « valeur : rang »).
- `citation` remplace `passage`, et le rôle `citation` remplace le rôle `passage`. Pour une source échange, `partie` vaut `question` ou `reponse` et `version` vaut `-`. `empreinte` est celle du texte cité (`text_digest`), vérifiée à la lecture.
- Citation d'une note (incrément 5.5, D18, D24) : `partie note`, `version` est le rang de la révision citée (0 pour le texte de création) ; la plage et l'empreinte portent sur le texte de cette version. Refusés : `version -` ou non numérique (`Value("version")`), une plage absente de la version citée (`InvalidState(InvalidExcerpt)`). La commande du journal est `citer portee <id> note <id> source <note> partie note version <n> debut <n> fin <n>`. La version 1 ne représente pas la citation d'une note.

### 13.2 Clés de ligne et différences

Chaque ligne d'un instantané a une clé (VERSION2.md, section 4) : `contenu <note>`, `revision <note> <rang>`, `corps <note>` (`echange`, `citation`, `reserve`), `ancrage <id>`, `contexte <portée>`, `membre <portée> <note>`, `relation <portée> <id>`, `acquittement <portée> <dépendant> <cause>`. L'ordre des clés est l'ordre canonique du texte. Une différence entre deux états liste les lignes retirées puis les lignes ajoutées, chacune par clé croissante ; une ligne modifiée est retirée puis ajoutée, une ligne inchangée n'apparaît pas.

### 13.3 Dépôt

Même structure et même ordre que la version 1 (section 12), avec ces changements :

```
memoire-depot 2
branche <id> tete <commit> origine <branche|-> base <commit> etat <active|abandonnee>
commit <id> parents <k> <commit>… motif <ch>
  reference
    <instantané version 2, chaque ligne indentée de quatre espaces>
commit <id> parents 1 <commit> motif <ch>
  difference
    - <ligne retirée>
    + <ligne ajoutée>
```

- `parents` : 0 pour la racine, 2 pour un commit de fusion (la tête de la cible, puis celle de la source ; incrément 5.4), 1 sinon. Deux parents identiques, ou tout autre nombre, sont refusés (`Value("parents")`) ; un second parent inconnu est une incohérence (`Inconsistent("parent de commit")`). La différence d'un commit de fusion est prise avec son premier parent.
- Profondeur d'un commit : 0 pour la racine, puis un de plus que son premier parent. Un état de référence complet (`reference`) est posé exactement aux profondeurs multiples de 100 ; tout autre commit porte sa différence avec son premier parent (`difference`). La profondeur ne dépend pas de l'ordre des identifiants.
- Commande `citer portee <id> note <id> source <id> partie <p> version - debut <n> fin <n>`, à la place de `creer-passage`.
- Commande `lot <n>` (incrément 5.3, LOT.md, D22), suivie de exactement *n* lignes, une par élément, indentées de quatre espaces, avant la ligne `resultat` :

```
  commande lot <n>
    element <mot de la modification> <champs>
```

  Chaque ligne `element` a les mots et les champs de la ligne `commande` de la modification seule. Refusés : `lot 0` (`Value("lot")`) ; une ligne manquante ou qui n'est pas un élément (`Field("element")`) ; un élément `annuler`, `retablir`, `basculer`, `restaurer` ou `lot` (`Value("element")`) ; une identité créée par deux éléments du même lot (`Inconsistent("identités du lot")`, à la ligne du second).
- Branche abandonnée (incrément 5.4a, D19, D23) : `etat abandonnee` ; commandes `abandonner branche <id>` et `reactiver branche <id>`, sans commit (`publie non`), avec les lignes `effet` du retour sur l'origine quand la branche abandonnée était active. Refusés à la lecture : main abandonnée (`Inconsistent("branche main")`), branche active abandonnée (`Inconsistent("branche active")`), `abandonner` ou `reactiver` comme élément d'un lot (`Value("element")`).
- Commande `fusionner source <branche> tete <commit> resolutions <k>` (incrément 5.4, FUSION.md, section 9.6, D23), suivie de exactement *k* lignes indentées de quatre espaces, dans l'ordre de la commande :

```
  commande fusionner source <branche> tete <commit> resolutions <k>
    resolution champ <portée> <note> <verdict|decision|implementation|reserve|visibilite> <cible|source>
    resolution texte <note> <cible|source>
    resolution acquittement <portée> <dépendant> <cause> <cible|source>
    resolution desactiver <portée> <relation>
```

  Refusés : une ligne manquante ou qui n'est pas une résolution (`Field("resolution")`), une forme inconnue (`Value("resolution")`), un champ inconnu (`Value("champ")`), un côté inconnu (`Value("côté")`), une fusion comme élément d'un lot (`Value("element")`).
- En version 1, `fusionner`, `abandonner`, `reactiver`, `lot` et `citer` sont inconnues ; un dépôt qui contient un lot, une branche abandonnée ou un commit de fusion n'est pas représentable en version 1, et aucune base de schéma 1 n'en contient.

### 13.4 Lecture d'un dépôt version 2

Les commits sont lus, puis leurs états reconstruits par profondeur croissante, chacun à partir des lignes de son parent :

- chaque ligne retirée doit être exactement présente, chaque ligne ajoutée absente ; sinon « dépôt incohérent : différence », à la ligne fautive ;
- dans une différence, retraits avant ajouts et clés strictement croissantes (motif « ordre ») ; une ligne retirée puis remise telle quelle est refusée (« différence minimale ») ; ces deux règles assurent la forme canonique unique sans réencoder le dépôt ;
- un état de référence hors des profondeurs multiples de 100, ou une différence à l'une d'elles, est refusé (« état de référence »), à la ligne `reference` ou `difference` ;
- un parent absent ou des parents en cycle sont refusés (« parent de commit ») ;
- chaque état reconstruit est validé comme un instantané (section 7). Une erreur d'un état de référence est située à sa ligne dans le dépôt, ou à la ligne `reference` pour une erreur de l'état global ; une erreur d'un état reconstruit par différence est située à la ligne ajoutée qui l'a introduite, sinon à la ligne `difference` ;
- la cohérence du dépôt est ensuite vérifiée comme en version 1 (section 12.4).

### 13.5 Migration, export et import

- `migrate_v1_to_v2` lit un dépôt v1, l'encode en v2, puis relit la v2 et la compare au dépôt d'origine, état de chaque commit compris : la moindre différence annule la migration (« migration »).
- L'export écrit la version 2 ; l'import lit la version qu'annonce l'en-tête (1 ou 2). Depuis l'incrément 5.2c, le stockage SQLite (schéma 2, STOCKAGE.md) ne conserve que des lignes de ce texte v2, découpées par le domaine ; les bases de schéma 1, en fragments v1, se convertissent par `memoire migrer` (5.2d).

### 13.6 Exemples de référence

`fixtures/format/exemple-v2.txt` est la migration de l'exemple de la section 9. Il en diffère exactement par l'en-tête, le rôle `citation` du passage, la ligne `citation … empreinte 0f661db2d23f65f3` et le rang `1` de la révision. `fixtures/format/depot-v2.txt` est la migration de l'exemple de la section 12.5 : seule la racine porte un état de référence ; une création ajoute ses lignes `contenu` et `membre`, un lien sa ligne `relation`, la réfutation retire puis ajoute la ligne `membre`. Les deux ont été dérivés à la main des exemples v1, puis comparés à l'encodeur ; ils sont figés comme ceux de la version 1.
