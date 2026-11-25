# Version 2 — spécification du stockage, du format et de la migration

Version 0.2 — 31 octobre 2026 — validée par l’utilisateur le 31 octobre 2026, avant le code. Une fois validée et implémentée, ses parties rejoindront FORMAT.md et STOCKAGE.md.

## 1. Objet

Une seule nouvelle version, une seule migration, pour cinq besoins :

| Besoin | Origine |
|---|---|
| Stockage en deux parties : état courant et historique par différences | performance mesurée le 31 octobre 2026 (section 2) |
| Opérations groupées : plusieurs modifications, une seule opération annulable | R1, plan de la phase 2 |
| Fusion à trois voies : commits à deux parents, résolutions de conflits | D19, FUSION.md |
| Abandon et réactivation d’une branche | D19 |
| Citations d’un échange ou d’une note, avec version citée et empreinte | D18, R8 |

## 2. Constat de départ

Mesure du 31 octobre 2026, exécutable optimisé, notes reliées en chaîne :

| Opérations | `statut` | Base |
|---|---|---|
| 101 | 0,05 s | 1,7 Mo |
| 303 | 0,35 s | 13 Mo |
| 505 | 1,05 s | 36 Mo |
| 606 | 1,44 s | 51 Mo |

Croissance quadratique : chaque commit stocke l’état complet du réseau, et chaque commande relit tout l’historique.

**Objectifs de la version 2**, mesurés sur un dépôt synthétique de 5 000 opérations et 1 500 notes :
- `statut` et une écriture ordinaire en moins de 0,2 s ;
- le temps d’une commande ordinaire ne dépend plus de la longueur de l’historique (mesure à 1 000, 2 500 et 5 000 opérations) ;
- la taille de la base croît linéairement avec le nombre d’opérations.

## 3. Les deux parties de la base

**Partie 1 — état courant.** Pour chaque branche non abandonnée, l’état complet de sa tête, rangé ligne par ligne : chaque ligne est une ligne canonique de l’instantané (section 5), identifiée par sa clé (section 4). Il contient toutes les notes, écartées comprises, et toutes les relations, retirées comprises : un repère de code doit retrouver sa note même réfutée, une cause de réexamen doit pouvoir être montrée.

**Partie 2 — historique.**
- Le journal : chaque opération, avec son motif, sa commande et son résultat (inchangé dans son principe).
- Chaque commit : ses parents, son motif et sa **différence** avec son premier parent : les lignes retirées et les lignes ajoutées.
- Des **états de référence** complets : le commit racine, puis un commit sur cent le long de chaque chaîne de premiers parents.

**Ce que fait chaque usage :**

| Usage | Lecture |
|---|---|
| Commande ordinaire, aperçu | état courant de la branche active |
| Annuler | différence du dernier commit, appliquée à l’envers |
| Rétablir | différence appliquée à l’endroit |
| Changer de branche | état courant de l’autre branche |
| Revoir ou restaurer un état ancien, base commune d’une fusion | état de référence le plus proche, puis au plus 99 différences |
| Répétition d’une opération | recherche de son identifiant dans le journal |
| Identité déjà employée | recherche dans la table des identités |

Rien n’est jamais effacé (D6). Une branche abandonnée perd son état courant, reconstructible à la demande ; son historique reste.

**Le noyau** ne garde plus en mémoire un instantané par commit. Il garde l’état de la tête de la branche active et lit tout autre état par une interface de lecture de l’historique, que fournit le stockage SQLite, ou, pour les tests, une implémentation en mémoire. Il reste sans entrée-sortie et sans dépendance (D14).

## 4. Clés de ligne

Chaque ligne canonique d’un instantané a une clé, tirée de ses premiers jetons ; une différence retire et ajoute des lignes entières, repérées par leur clé.

| Ligne | Clé |
|---|---|
| `contenu` | `contenu <note>` |
| `revision` | `revision <note> <rang>` (rang à partir de 1, nouveau champ, section 5) |
| `echange`, `citation`, `reserve` | `corps <note>` |
| `ancrage` | `ancrage <id>` |
| `contexte` | `contexte <portée>` |
| `membre` | `membre <portée> <note>` |
| `relation` | `relation <portée> <id>` |
| `acquittement` | `acquittement <portée> <dépendant> <cause>` |

Appliquer une différence exige que chaque ligne retirée soit exactement présente : toute divergence est refusée, jamais ignorée. C’est un contrôle d’intégrité à chaque lecture.

## 5. Instantané, version 2 (`memoire-instantane 2`)

Changements par rapport à la version 1 :

1. **`revision`** porte son rang : `revision <note> <rang> <nature> titre <ch> texte <ch>`.
2. **`citation`** remplace `passage`, et le rôle `citation` remplace le rôle `passage` :

```
citation <note> source <id> partie <question|reponse|note> version <n|-> debut <n> fin <n> empreinte <hex16>
```

- Source échange : `partie` vaut `question` ou `reponse`, `version` vaut `-`.
- Source note : `partie` vaut `note`, `version` est le rang de la révision citée (0 pour le texte de création).
- `empreinte` : empreinte du texte cité (`text_digest`). La validation vérifie que la plage désigne exactement ce texte dans la version citée.
- Une note révisée après avoir été citée ne change pas la citation, qui reste attachée à sa version ; la citation est signalée « source révisée » dans les vues, jamais déplacée.

Tout le reste de la version 1 est inchangé : jetons, chaînes, ordre canonique, validation.

## 6. Dépôt, version 2 (`memoire-depot 2`)

1. **Branche :** `branche <id> tete <c> origine <b|-> base <c> etat <active|abandonnee>`.
2. **Commit :** `commit <id> parents <k> <id>… motif <ch>`, suivi soit de `  reference` et de l’instantané complet indenté de quatre espaces, soit de `  difference` et de lignes `    - <ligne>` puis `    + <ligne>`, dans l’ordre canonique des clés.
3. **Nouvelles commandes du journal :**

| Commande | Champs |
|---|---|
| `lot` | `<n>`, suivi de n lignes `    element <commande> <champs>` |
| `fusionner` | `source <branche> resolutions <k>`, suivi de k lignes `    resolution …` |
| `abandonner` | `branche <id>` |
| `reactiver` | `branche <id>` |
| `citer` | `portee <id> note <id> source <id> partie <p> version <n\|-> debut <n> fin <n>` (remplace `creer-passage`) |
| `restaurer-notes` | `cible <commit> notes <k> <id>…`, sélection strictement croissante (RESTAURATION.md, section 2 ; ligne ajoutée en 6.5, oubliée en 6.4) |
| `reprise` | `date <ch> objectif <ch> action <ch> question <ch> etat <ch> resultat <ch> hypothese <ch> ouverts <ch> notes <k> <id>… code <état>`, sur une seule ligne (FICHE-REPRISE.md, D26) ; `<état>` vaut `-`, ou `commit <ch\|-> branche <ch\|-> changements <k> <ch>… patch <patch>`, changements strictement croissants ; `<patch>` vaut `texte <ch>` (au plus 4 Mo) ou `non-conserve octets <n> empreinte <16 hex>` (au-delà, ou hors UTF-8) |

4. **Résolutions de fusion**, repérées par clé et non par numéro, pour rester valables si le plan est recalculé :
   - `resolution champ <portée> <note> <champ> <cible|source>` (verdict, décision, implémentation, réserve, visibilité) ;
   - `resolution texte <note> <cible|source>` ;
   - `resolution desactiver <portée> <relation>`.

L’export textuel suit cette même forme : sa taille croît avec les changements, non plus avec l’état complet à chaque commit.

## 7. Migration de la version 1

- **Déclenchement :** à l’ouverture d’une base de schéma 1, après confirmation de l’utilisateur. L’ouverture d’une base plus récente que l’exécutable est refusée avec un motif.
- **Sauvegarde :** une copie de la base d’origine est d’abord écrite dans `sauvegardes/avant-migration-….db`.
- **Lecture :** le décodeur de la version 1 est conservé tel quel. Ses tests et ses exemples figés restent en vigueur.
- **Conversion :**
  - passages en citations de source échange, avec leur empreinte calculée ;
  - révisions numérotées ;
  - commits convertis en différences, et un état de référence posé tous les cent commits ;
  - `creer-passage` devient `citer` ;
  - toutes les branches sont actives.
- **Contrôle :** après conversion, chaque état de chaque commit est reconstruit et comparé à l’état de la version 1. La moindre différence annule la migration.
- **Tests :** les exemples figés de la version 1 (`fixtures/format/*-v1.txt`) sont migrés vers des exemples figés de la version 2. Les tests de propriétés migrent leurs dépôts aléatoires et vérifient l’égalité de tous les états.

## 8. Ordre d’implémentation

1. Format version 2 : codec complet (instantané, dépôt, différences, nouvelles commandes) et migration, sur le noyau actuel ; exemples figés.
2. Noyau : interface de lecture de l’historique, état de tête seul en mémoire ; stockage SQLite en deux parties ; mesure des objectifs de la section 2.
3. Opérations groupées.
4. Fusion, avec sauvegarde automatique, abandon et réactivation de branche.
5. Citations (D18) et vue « Orphelines » (R6).

Chaque étape suit la méthode habituelle : contrat avant le code, mutation sans survivant, livraison étiquetée.

## 9. Risques

- **Une étape suivante qui exige un enregistrement non prévu ici** imposerait une version 3. C’est pourquoi les enregistrements de toutes les étapes sont définis dès maintenant.
- **Le changement d’architecture du noyau est le plus gros chantier depuis la phase 2.** Le contrat existant (contrat, propriétés, recette) est conservé et doit rester vert à chaque étape.
- **La migration touche vos données réelles.** D’où la sauvegarde préalable, le contrôle état par état et l’annulation au moindre écart.
