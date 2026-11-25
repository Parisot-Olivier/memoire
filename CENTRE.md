# Centre de travail : citer et relier en un geste — conception de l’incrément 6.2

Version 0.1 — 8 novembre 2026 — **validée par l’utilisateur le 8 novembre 2026** ; aucune décision nouvelle : application de D22, D24 et D25 ; **implémentée** (6.2a ligne de commande, 6.2b interface ; `v0.6.0-alpha.2`). Termine R2 (« centre de travail : surligner, relier les passages »). Prolonge le visionneur (VISIONNEUR.md, D25) et `citer` (CITATIONS.md, D24). Regroupe en section 7 les points à valider.

---

## 1. Ce que l’utilisateur obtient

En lisant une réponse dans le visionneur, il sélectionne un passage, puis, **dans le même geste** :
- dit que ce passage **soutient** une ou plusieurs de ses notes : chacune « dépend de » la citation ;
- dit qu’une ou plusieurs notes **en sont nées** : chacune « développée à partir de » la citation ;
- **classe** la citation dans des catégories, ou la relie par un **lien** de navigation ;
- crée une **nouvelle note** développée à partir de la citation (titre, rôle, texte facultatif) ;
- ouvre une **réserve** depuis la citation, si elle est tirée d’un échange (D24).

Tout cela forme **une seule opération** : la citation, sa relation vers la source (D24), les relations demandées, la nouvelle note et la réserve. Un seul `annuler` défait le tout. La confirmation, si l’ensemble est critique, est demandée une fois (D22, criticité sur l’effet net).

---

## 2. Noyau

**Aucun changement.** Tout est un lot (`Command::Lot`, D22) de modifications existantes, dans cet ordre :
1. la citation (`CreatePassage` ou `CiteNote`) ;
2. sa relation « développé à partir de » vers la source ;
3. les relations demandées, dans l’ordre des options puis des notes ;
4. la nouvelle note (`CreateNode`), puis sa relation « développé à partir de » vers la citation ;
5. la réserve (`CreateReserve`), qui naît de la citation.

Les règles du noyau s’appliquent sans exception :
- une catégorie ne « dépend » de rien ;
- une réserve exige une citation d’échange (WrongKind sinon, D24) ;
- les doublons sont refusés.

Le refus d’un élément refuse tout le lot et le nomme (« élément *k* du lot : … »).

---

## 3. Ligne de commande : `citer` étendu

```
citer NOTE (--texte X | --plage A-B) [--partie question|reponse] [--version N] [--occurrence N]
      [--soutient NOTES] [--inspire NOTES] [--classe-dans NOTES] [--lien NOTES]
      [--note TITRE] [--role categorie|raisonnement|implementation|note] [--texte-note X]
      [--reserve TITRE] [--cible C] [--motif M] [--oui]
```

| Option | Relation créée |
|---|---|
| `--soutient NOTES` | chaque note « dépend de » la citation (la citation la soutient) |
| `--inspire NOTES` | chaque note « développée à partir de » la citation |
| `--classe-dans NOTES` | la citation « classée dans » chaque catégorie |
| `--lien NOTES` | la citation « lien » chaque note |
| `--note TITRE` | nouvelle note (rôle `--role`, raisonnement par défaut ; texte `--texte-note`), « développée à partir de » la citation |
| `--reserve TITRE` | réserve née de la citation, cible facultative `--cible` |

- `NOTES` : identifiants ou préfixes séparés par des virgules, comme pour `note creer` (R1). Une note citée deux fois pour la même option est refusée (code 2).
- `--role` et `--texte-note` sans `--note`, ou `--cible` sans `--reserve`, sont refusés (code 2).

**Sortie**, une ligne par objet créé, dans l’ordre du lot :

```
Citation créée : <id> (octets A à B)
Relation créée : <id> (developpe-a-partir-de <source>)
Relation créée : <id> (<note> depend-de <citation>)          # --soutient
Relation créée : <id> (<note> developpe-a-partir-de <citation>)  # --inspire
Relation créée : <id> (classe-dans <catégorie>)              # --classe-dans
Relation créée : <id> (lien <note>)                           # --lien
Note créée : <id>
Relation créée : <id> (developpe-a-partir-de <citation>)
Réserve créée : <id>
```

La forme à deux termes, `(type destination)`, est celle d’une relation qui part de l’objet créé à la ligne précédente : la citation, ou la nouvelle note (comme pour `note creer`). La forme à trois termes, `(origine type destination)`, est celle d’une relation qui part d’une autre note (`--soutient`, `--inspire`). Précision apportée à la mise en œuvre : la version validée énonçait « trois termes pour une relation qui arrive sur la citation », ce que contredisait son propre exemple pour la relation de la nouvelle note ; l’exemple est retenu.

**Contrat modifié** : la ligne de `citer` dans l’aide figée (le nombre de commandes ne change pas : 38).

---

## 4. Interface

- Le visionneur gagne un panneau **« Relier la sélection »**, sous les zones de lecture :
  - les champs « Soutient », « Inspire », « Classe dans », « Lien », à plusieurs notes, avec l’autocomplétion par nom (R3, 5.3) ;
  - « Nouvelle note » (titre et rôle) ;
  - « Réserve » (titre), active seulement pour un échange.
- **« Citer la sélection »** emploie ce qui est rempli : sans rien de rempli, c’est la citation seule d’aujourd’hui. Après réussite, le panneau se vide.
- L’autocomplétion, aujourd’hui écrite dans la fenêtre principale, devient une fonction partagée par les deux fenêtres, sans changement de comportement.
- `gui/schema.py` reconnaît `--soutient` et `--inspire` comme champs à plusieurs notes, comme `--classe-dans` et `--lien`.

---

## 5. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-LOT-01 | (existant) Un lot est tout ou rien ; une seule opération, qu’un seul `annuler` défait |
| INV-CIT-02 | (existant) Une citation créée par `citer` naît avec son lien vers sa source, dans la même opération |
| INV-WORK-01 | Citer et relier en un geste ne crée qu’une opération : citation, relation vers la source, relations demandées, nouvelle note et réserve, ou rien |

---

## 6. Découpage et tests

| Étape | Contenu |
|---|---|
| 6.2a | ligne de commande : options de `citer`, contrat et code ; mutation sans survivant |
| 6.2b | interface : panneau du visionneur, autocomplétion partagée, essais sous `xvfb-run` ; documents ; contrôle complet ; livraison `v0.6.0-alpha.2` |

**Tests prévus :**
- **Ligne de commande** :
  - chaque option seule et toutes ensemble ;
  - ordre et forme des lignes de sortie ;
  - un seul `annuler` ;
  - lot critique (une note soutenue réfutée) avec confirmation ;
  - refus localisés : catégorie qui « soutient », réserve sur une citation de note, doublon, note introuvable, options orphelines.
- **Interface** :
  - citer une sélection en reliant à une note choisie par son nom ;
  - nouvelle note depuis la sélection ;
  - réserve depuis une citation d’échange ;
  - panneau vidé après réussite.

---

## 7. Points validés le 8 novembre 2026

1. **Tout en un lot construit par `citer`**, sans changement du noyau.
2. **Noms et sens des options** : `--soutient` (la note dépend de la citation), `--inspire` (la note est développée à partir de la citation), `--classe-dans` et `--lien` (la citation vers la note) ; `--note` et `--reserve` créent un objet nouveau.
3. **Forme des lignes de sortie** : deux termes pour une relation qui part de la citation, trois termes pour une relation qui y arrive.
4. **Panneau « Relier la sélection »** dans le visionneur, avec l’autocomplétion partagée.
5. **Hors de 6.2** : relier en un geste une citation **déjà existante**, cliquée dans le visionneur. On le fait aujourd’hui par `lier`, une relation à la fois. À proposer ensuite si le besoin se confirme.
