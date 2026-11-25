# Panneau des actions — refonte des menus et du vocabulaire (incrément 7.1)

Version 0.2 — 10 novembre 2026 — **validée par l’utilisateur le 10 novembre 2026** ; une décision nouvelle (D30, section 9) ; **implémentée** (7.1a catalogue, 7.1b sorties `--tsv`, 7.1c interface ; `v0.7.0-alpha.1`) ; précisions de mise en œuvre : section 11. Version 0.2 : « intégrer une piste » devient **« adopter une piste »** (pour une autre piste), à la demande de l’utilisateur : « la définir comme sérieuse pour telle branche, piste vers idée principale de telle branche ».

**Demande de l’utilisateur** du 10 novembre 2026 :
- « réviser le panneau de contrôle (pas d’UX pour l’instant, juste une révision des menus) ; les termes ne sont pas très clairs, difficiles à suivre ; dans plusieurs mois, je ne suis pas sûr de comprendre comment l’utiliser » ;
- « un système un peu en graphe » ;
- « créer un échange simplement, pas dix paramètres » ;
- « des libellés courts et simples (créer une note, créer un échange avec une IA, réfuter, quel échange ?, pour quel motif ?) » ;
- « une création plus complexe en cochant progressivement ce que je veux » ;
- « surtout tout ce qui est du côté réfuter / fusionner » ;
- « je t’autorise à changer de vocabulaire pour l’interface graphique ».

**Périmètre : les menus, les questions posées, le vocabulaire et l’enchaînement des actions.** L’apparence (couleurs, mise en page fine, Liquid Glass) reste pour le designer d’interface. Le noyau ne change pas. La ligne de commande gagne seulement deux formes de sortie lisibles par l’interface (section 7).

La section 10 regroupe les points validés.

---

## 1. Ce qui ne va pas aujourd’hui

- **47 commandes à plat**, dans l’ordre de la ligne de commande, nommées comme elle : `reserve concerne`, `acquitter`, `fusion plan`, `restauration plan`…
- **Des formulaires calqués sur les options** : `--developpe-a-partir-de`, `--reponse-fichier`, `--motif`, `NOTE`, `VALEUR`… On doit savoir ce que fait chaque option.
- **Créer un échange** demande d’enregistrer la réponse de l’IA dans un fichier, puis de le choisir.
- **Réfuter** passe par une commande `verdict` à trois valeurs, puis une confirmation qui parle de « branche créée depuis main ».
- **Fusionner** demande de lancer `fusion plan`, de recopier un « code du plan », puis de recopier des numéros de conflits dans `--cible`, `--source` ou `--desactiver`.
- **Les branches** n’ont pour nom qu’un identifiant de 36 caractères.

---

## 2. Principes

1. **Des intentions, pas des commandes.** On choisit ce qu’on veut faire, dit simplement (« Réfuter une note »), dans un menu rangé par sujet. Une intention peut employer une ou plusieurs commandes.
2. **Une question par champ**, en mots simples : « Quelle note ? », « Pourquoi ? », « Le texte de la réponse ». Les notes se choisissent par leur nom, avec l’autocomplétion actuelle.
3. **Le minimum d’abord.** Ne s’affiche que ce qui est obligatoire. Le reste se déplie en cochant : « ☐ La ranger dans un thème », « ☐ Elle s’appuie sur une autre note ». Chaque case cochée ne fait apparaître que ses propres questions.
4. **Le graphe des actions.** Après chaque action réussie, l’interface propose **« Et ensuite ? »** : deux à quatre actions qui suivent naturellement, déjà remplies avec l’objet qu’on vient de créer ou de modifier. Par exemple, après « Nouvel échange » : « Citer un passage de cet échange », « Créer une note qui en découle ». On avance ainsi d’action en action le long des liens du réseau, sans revenir au menu.
5. **Dire ce qui va se passer avant les actions importantes**, en clair. Par exemple : « Ces 3 notes devront être revues. Pour ne pas toucher à la version principale, ce changement sera fait sur une piste à part, que vous pourrez adopter plus tard. »
6. **Une phrase d’aide par action**, sous son titre, pour comprendre dans six mois : à quoi elle sert, et ce qu’elle ne fait pas.
7. **Rien n’est perdu.** Toutes les commandes restent accessibles telles qu’aujourd’hui sous **« Toutes les commandes (mode expert) »**.

---

## 3. Vocabulaire de l’interface

La ligne de commande, la documentation technique et le noyau gardent leurs termes. Seule l’interface graphique emploie ceux de droite, et une table unique (section 7) fait la correspondance.

| Aujourd’hui | Dans l’interface | Explication affichée |
|---|---|---|
| base, dépôt | **projet** | « Le fichier qui contient toutes vos notes et leur histoire » |
| `init` | **Créer le projet** | |
| catégorie | **thème** | « Pour ranger des notes ; un thème n’est ni vrai ni faux » |
| raisonnement | **idée** | « Une hypothèse, un raisonnement, un invariant » |
| implémentation | **solution technique** | « Une façon de réaliser une idée dans le code » |
| note personnelle | **note libre** | |
| échange | **échange avec une IA** | « Une question et la réponse de l’IA, conservées telles quelles » |
| citation (passage) | **passage cité** | « Un extrait exact d’un échange ou d’une note » |
| réserve | **doute à suivre** | « Un avertissement à garder en tête jusqu’à ce qu’il soit levé » |
| relation `depend-de` | **s’appuie sur** | « Si l’appui tombe, la note devra être revue » |
| `developpe-a-partir-de` | **vient de** | « Son origine ; si l’origine tombe, elle devra être revue » |
| `classe-dans` | **rangée dans le thème** | |
| `lien` | **en rapport avec** | « Un simple renvoi, sans conséquence » |
| `restreint` | **ne vaut que si** | |
| `contredit` | **contredit** | « Un désaccord assumé, sans conséquence » |
| `remplace` | **remplace** | |
| verdict `verifiee` / `refutee` / `non-verifiee` | **vérifiée (c’est vrai)** / **réfutée (c’est faux)** / **pas encore vérifiée** | |
| décision `retenue` / `proposee` / `plus-retenue` / `remplacee` | **retenue** / **proposée** / **mise de côté** / **remplacée** | « Mise de côté : écartée, mais gardée en mémoire » |
| implémentation | **avancement** : pas commencé / en cours / fait | |
| à réexaminer | **à revoir** | |
| `acquitter` | **garder quand même** | « La note reste valable malgré ce qui a changé » |
| ressources actuelles | **notes en vigueur** | |
| orpheline | **sans appui** | |
| motif | **pourquoi ?** | |
| branche | **piste** | « Une version parallèle du projet, pour essayer sans toucher à la principale » |
| main | **piste principale** | |
| `basculer` | **aller sur une piste** | |
| `abandonner` / `reactiver` | **mettre une piste de côté** / **reprendre une piste** | |
| fusionner | **adopter une piste** (pour une autre piste) | « Les changements de la piste deviennent la version retenue d’une autre piste, souvent la principale » |
| commit | **étape** | |
| restaurer | **revenir à une étape** | |
| `ancrer` | **lier au code** | |
| repère `@design:` | **marque dans le code** | « Le commentaire à coller dans votre code » |
| `indexer` | **vérifier les liens avec le code** | |
| point de reprise | **où j’en suis** | |

**Le nom des pistes.** Une piste est désignée par ce qui l’a créée, et non par son identifiant :
- « Réfutation de « Hypothèse sur l’ISR » » ;
- « Mise de côté de « File d’attente » » ;
- « Retour à l’étape du 10 nov. » ;
- un numéro « (2) » si deux pistes portent le même nom.

---

## 4. Le menu

La liste des 47 commandes laisse place à un menu en arbre, rangé par sujet. Chaque action porte un verbe et un objet simples.

```
Écrire
  Nouvelle note
  Nouvel échange avec une IA
  Citer un passage                      (ouvre la note ou l’échange, on sélectionne le passage)
  Compléter la question d’un échange
  Nouveau doute à suivre
Relier
  Relier deux notes
  Ranger une note dans un thème
  Ce doute concerne une note
  Cette note répond à un doute
Juger
  Vérifier une note (c’est vrai)
  Réfuter une note (c’est faux)
  Retenir / mettre de côté / remplacer une idée
  Avancement d’une solution technique
  Garder une note quand même
  Retirer une note des notes en vigueur
  Lever ou rouvrir un doute
Lire
  Où en est le projet ?                 (notes en vigueur, à revoir, sans appui)
  Chercher une note
  Lire une note ou un échange           (visionneur)
  Voisinage d’une note
Code
  Lier une note au code
  Que désigne cette marque ?
  Vérifier les liens avec le code
Histoire et pistes
  Annuler / Rétablir
  Les dernières étapes
  Revenir à une étape
  Les pistes                            (liste, aller sur, mettre de côté, reprendre)
  Adopter une piste
Où j’en suis
  Noter où j’en suis
  Où en étais-je ?
Projet
  Créer le projet · Sauvegarder · Exporter · Importer · Vérifier · Mettre à jour une ancienne base
Toutes les commandes (mode expert)
```

---

## 5. Les actions principales, pas à pas

**Nouvel échange avec une IA**
- Titre de l’échange ?
- La réponse de l’IA (une grande zone où l’on colle le texte).
- ☐ Ajouter la question posée → une zone pour la coller.
- ☐ C’est la suite d’un autre échange → « Quel échange ? »
- *Et ensuite ?* Citer un passage de cet échange · Créer une note qui en découle · Le lire.

L’interface écrit elle-même les textes collés dans des fichiers temporaires, puis appelle `echange coller` : plus de fichier à préparer.

**Nouvelle note**
- Titre ?
- Quel genre de note ? (idée · solution technique · note libre · thème), chacune avec sa phrase d’explication.
- ☐ Ajouter un texte.
- ☐ Elle s’appuie sur d’autres notes → « Lesquelles ? »
- ☐ Elle vient d’une note ou d’un échange → « Lequel ? »
- ☐ La ranger dans un thème → « Quel thème ? »
- ☐ En rapport avec d’autres notes → « Lesquelles ? »
- *Et ensuite ?* La lier au code · Relier à une autre note · La lire.

Une seule opération, comme aujourd’hui (`note creer`, D22).

**Relier deux notes** : une phrase à compléter, le choix au milieu est en mots simples.

```
[ Quelle note ? ]  ( s’appuie sur | vient de | est rangée dans le thème | est en rapport avec | ne vaut que si | contredit )  [ Quelle autre note ? ]
```

« ne vaut que si » demande « À quelle condition ? » ; « contredit » demande « Pourquoi ? ».

**Réfuter une note (c’est faux)**
1. Quelle note ?
2. Pourquoi est-elle fausse ?
3. **Ce que ça va changer**, calculé par un aperçu de la commande, sans rien écrire :
   - « Ces notes s’appuient dessus et devront être revues : … » ;
   - « Pour ne pas toucher à la piste principale, ce sera fait sur une nouvelle piste, “Réfutation de « … »”. Vous pourrez l’adopter plus tard pour la piste principale, ou la mettre de côté. »
4. [Réfuter] [Ne rien faire].
5. *Et ensuite ?* Revoir les notes concernées · Garder une note quand même · Revenir à la piste principale.

« Vérifier une note », « Retenir / mettre de côté », « Retirer une note » suivent le même schéma : quelle note ?, pourquoi ?, ce que ça va changer, confirmer.

**Adopter une piste** (aujourd’hui `fusion plan` puis `fusionner`) : les changements d’une piste deviennent la version retenue d’une autre, souvent la principale.
1. **Quelle piste adopter ?** La liste des pistes par leur nom, avec ce qu’elles contiennent (« 4 changements »).
2. **Pour quelle piste ?** Par défaut, celle où l’on est, le plus souvent la piste principale. Si l’on en choisit une autre, l’interface y va d’abord (`basculer`) et le dit.
3. **La phrase qui résume**, avant toute autre question : « Les changements de “Réfutation de « Hypothèse sur l’ISR »” deviendront la version retenue de la piste principale. »
4. **Ce qui sera repris sans difficulté** : « 12 changements ».
5. **Les désaccords**, s’il y en a : une question par désaccord, en clair. Par exemple :
   ```
   « File d’attente » — décision : retenue dans la piste principale, mise de côté dans la piste adoptée.
   Garder :  ( ) ce qui est dans la piste principale   ( ) ce que propose la piste adoptée
   ```
   Pour un cycle impossible, la question est : « Ces relations forment une boucle ; laquelle désactiver ? »
6. **Ce que ça va changer** : les notes à revoir. Une sauvegarde est faite avant, comme aujourd’hui.
7. [Adopter] [Ne rien faire]. L’interface recalcule le plan et transmet elle-même code et numéros à `fusionner`. Si quelque chose a bougé entre-temps, elle le dit et repose les questions.
8. *Et ensuite ?* Mettre la piste adoptée de côté · Où en est le projet ?

**Revenir à une étape** : les dernières étapes, nommées par ce qu’elles ont fait (« Réfutation de « … » », « Nouvelle note « … » ») plutôt que par leur identifiant. On choisit l’étape, puis « tout le projet » ou « seulement ces notes », on lit ce qui sera rétabli, puis on confirme.

**Lier une note au code** : quelle note ?, quel fichier ? (« Parcourir… »), quelle fonction, ou quelles lignes. Le commentaire à coller s’affiche avec un bouton **« Copier »** qui le met dans le presse-papiers.

---

## 6. Le graphe des actions (« Et ensuite ? »)

Chaque action déclare ses suites possibles, et l’objet qu’elle leur passe.

| Après… | Et ensuite ? (pré-rempli) |
|---|---|
| Nouvelle note | Lier au code · Relier à une autre note · Lire |
| Nouvel échange | Citer un passage · Nouvelle note qui en vient · Lire |
| Passage cité | Nouvelle note qui en vient · Nouveau doute à suivre (pour un échange) |
| Relier deux notes | Voisinage de la note · Relier encore |
| Réfuter, mettre de côté, retirer | Revoir les notes concernées · Garder une note quand même · Aller sur la piste principale |
| Garder une note quand même | Où en est le projet ? |
| Adopter une piste | Mettre la piste adoptée de côté · Où en est le projet ? |
| Lier au code | Vérifier les liens avec le code |
| Revenir à une étape | Où en est le projet ? · Annuler |

Les suites ne s’exécutent jamais d’elles-mêmes : ce sont des boutons. Le graphe est une donnée du catalogue (section 7), testée, pas du code dispersé dans les fenêtres.

---

## 7. Architecture

- **`gui/actions.py`, le catalogue, sans Tkinter.** Pour chaque action :
  - son groupe, son titre et sa phrase d’aide ;
  - ses questions : type (texte, texte long, note, notes, choix, dossier, fichier), libellé, obligatoire ou non ;
  - ses sections à cocher ;
  - la ou les commandes qu’elle produit, à partir des réponses ;
  - ses suites.

  La table de vocabulaire de la section 3 y est aussi : mots de l’interface et valeurs de la ligne de commande. Un catalogue pur se teste sans affichage.
- **Le formulaire progressif** (`gui/formulaire.py`) affiche les questions d’une action, déplie les sections cochées, lance les commandes, montre « Ce que ça va changer » pour une action importante, puis « Et ensuite ? ». Il remplace le formulaire généré depuis l’aide, qui reste celui du mode expert.
- **Le lien avec la ligne de commande reste vérifié.** Un test construit, pour chaque action, les commandes qu’elle produit avec des réponses d’essai, et les fait relire par le lecteur de l’aide figée (`gui/schema.py`). Si la ligne de commande change, le catalogue échoue au test au lieu de casser en silence. Un autre test vérifie que chaque commande de l’aide est soit employée par une action, soit volontairement réservée au mode expert.
- **Deux formes de sortie ajoutées à la ligne de commande**, lisibles par l’interface (même principe que `voisinage --tsv`) :
  - `branches --tsv` : par piste, identifiant, active ou non, mise de côté ou non, et **nom lisible**, calculé d’après l’opération qui l’a créée (commande, note principale, motif) ;
  - `fusion plan SOURCE --tsv` : base, nombre de changements sans conflit, un désaccord par ligne (numéro, genre, note, champ, valeur ici, valeur dans la piste), relations candidates d’une boucle, notes à revoir, code du plan.

  Ces deux options changent l’aide figée (« Contrat modifié »). Elles sont testées et soumises à la mutation comme le reste de la ligne de commande. Le noyau ne change pas.
- **Le nom des étapes** (« Revenir à une étape ») vient de `historique`, qui donne déjà la commande et le motif de chaque étape. L’interface les traduit avec la table de vocabulaire.

---

## 8. Découpage et tests

| Étape | Contenu |
|---|---|
| 7.1a | catalogue `gui/actions.py` (groupes, questions, sections, vocabulaire, suites) ; tests sans affichage, dont la relecture par l’aide figée de chaque commande produite |
| 7.1b | ligne de commande : `branches --tsv` (nom lisible des pistes) et `fusion plan --tsv` ; contrat, mutation |
| 7.1c | formulaire progressif, menu en arbre, « Ce que ça va changer », « Et ensuite ? », bouton « Copier », mode expert ; parcours guidés « Réfuter » et « Adopter une piste » ; essais sous KWin virtuel ; documents ; contrôle complet ; livraison `v0.7.0-alpha.1` |

**Essais prévus sous KWin virtuel**, chacun de bout en bout et sans taper une option :
- créer un échange en collant question et réponse ;
- créer une note rangée dans un thème et appuyée sur une autre, en cochant ;
- réfuter une note et voir les notes à revoir avant de confirmer ;
- adopter une piste pour la piste principale, avec un désaccord tranché par un choix ;
- suivre une suite « Et ensuite ? » avec l’objet pré-rempli.

---

## 9. Décision D30

**D30 — Interface : un catalogue d’intentions en vocabulaire simple, distinct des commandes**

- **Contexte** : les formulaires étaient générés depuis l’aide de la ligne de commande (D17) ; ils en reprenaient le jargon et l’organisation à plat, difficiles à suivre et à réapprendre.
- **Décision** :
  - l’interface graphique présente un catalogue d’actions rangées par sujet, en vocabulaire simple, avec des questions courtes, des sections à cocher et des suites proposées ;
  - le catalogue traduit vers les commandes ;
  - la ligne de commande et le noyau gardent leurs termes ;
  - la cohérence entre catalogue et commandes est vérifiée par test contre l’aide figée ;
  - le formulaire généré reste disponible en mode expert ;
  - deux sorties `--tsv` (pistes et plan d’adoption) donnent à l’interface ce qu’elle doit expliquer.
- **Options écartées** :
  - **renommer les commandes de la ligne de commande** : casserait les habitudes, les scripts et toute la documentation, pour un besoin propre à l’interface ;
  - **garder les formulaires générés et seulement retoucher les libellés** : ne règle ni l’organisation à plat, ni la fusion par numéros, ni la création d’un échange par fichiers ;
  - **lire la sortie lisible de `fusion plan`** dans l’interface : fragile au moindre changement de texte.
- **Conséquences** :
  - deux vocabulaires, reliés par une table unique et testée ;
  - D17 précisée : l’interface reste sans règle métier, mais n’est plus seulement le reflet de l’aide.

---

## 10. Points validés le 10 novembre 2026

1. **Le vocabulaire de la section 3**, en particulier :
   - **piste** pour branche, **adopter une piste** (pour une autre piste) pour fusionner, **étape** pour commit ;
   - **idée / solution technique / note libre / thème** pour les genres de notes ;
   - **s’appuie sur / vient de** pour les deux relations qui propagent la révision ;
   - **garder quand même** pour acquitter ;
   - **projet** pour la base.
2. **Le menu en arbre de la section 4**, et le mode expert qui garde toutes les commandes.
3. **Le principe « minimum d’abord, puis cases à cocher »**, et **« Et ensuite ? »** comme graphe des actions.
4. **Les parcours guidés « Réfuter » et « Adopter une piste »** (section 5), avec le nom lisible des pistes.
5. **Deux sorties `--tsv` ajoutées à la ligne de commande** (pistes, plan d’adoption), et la **décision D30**.

---

## 11. Précisions apportées à la mise en œuvre

1. **Actions ajoutées au menu** : « Modifier le texte d’une note » (`note reviser`), « Remettre une note « à vérifier » » (`verdict non-verifiee`), « Récupérer le code non enregistré noté » (`reprise code`). Ainsi, toutes les commandes sauf `aide` ont une action ; `aide` reste au seul mode expert.
2. **« Retenir ou mettre de côté une idée »** ne propose pas « remplacée » : ce statut n’est posé que par la relation « remplace », que la ligne de commande n’expose pas.
3. **Nom des pistes** (7.1b) : il vient de l’opération qui a créé la piste (« Réfutation de « … » », « Mise de côté de « … » », « Retrait de « … » », « Retour à l’étape … », « Verdict sur… », « Décision sur… », sinon le mot de la commande), complété de « (2) »… pour les doublons, dans l’ordre des identifiants. La piste où l’on est porte « ★ » dans les listes de l’interface.
4. **« Ce que ça va changer »** : l’interface lance l’action ; si l’exécutable demande une confirmation (opération importante), elle montre son aperçu traduit dans le vocabulaire de l’interface et propose « Confirmer » ou « Ne rien faire », sans rien écrire avant.
5. **Adopter une piste** : si la piste qui adopte n’est pas celle où l’on est, « Voir le plan » y va d’abord (`basculer`) ; les désaccords se tranchent par un choix pour chacun, une boucle par le choix de la relation à désactiver ; « Adopter » transmet code et numéros à `fusionner`.
6. **Mode expert** : « Toutes les commandes » garde les formulaires générés depuis l’aide, avec leurs boutons « Exécuter » et « Confirmer (--oui) », masqués quand une action est ouverte.
7. **Sortie** : la zone de sortie montre toujours les commandes lancées et leur réponse brute, suivies de « ✓ Fait : … ». La mise en forme de cette zone reste pour le designer.
8. **Notes choisies** : elles s’affichent par leur identifiant une fois choisies dans la liste, comme avant. Afficher leur nom est une amélioration à prévoir.

