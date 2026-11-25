# Retrouver une note par son nom ou par le code — conception de l’incrément 7.2

Version 0.1 — 11 novembre 2026 — **validée par l’utilisateur le 11 novembre 2026** ; **implémentée** (7.2a noyau et ligne de commande, 7.2b interface ; `v0.7.0-alpha.2`) ; précisions de mise en œuvre : section 8 ; aucune décision nouvelle : application de D9 (rien n’est écrit dans le code), de l’indexeur (INDEXEUR.md) et de D30 (panneau des actions).

**Demande de l’utilisateur** du 11 novembre 2026 : « rechercher une note plus facilement au lieu d’une recherche par identifiant » (boutons « Voir une note… » et « Voisinage d’une note… ») ; « je donne le code et je trouve la note associée ».

**Constats** (vérifiés le 11 novembre 2026) :
- **les deux boutons du haut** demandent un identifiant ;
- **dans les formulaires**, une note se cherche bien par son nom, mais il faut ouvrir la liste, et le champ montre ensuite l’identifiant ;
- **« Que désigne cette marque ? »** (`repere`) ne reconnaît que la ligne exacte `// @design: <id>`. Il refuse `wake(t); // @design: …`, `# @design: …` et `/* @design: … */`, que l’indexeur reconnaît pourtant ;
- **aucune commande** ne donne les notes liées à un fichier ou à un endroit du code sans marque.

La section 7 regroupe les points validés.

---

## 1. Trouver une note par son nom

- **« Voir une note… » et « Voisinage d’une note… »** n’ouvrent plus une fenêtre qui demande un identifiant. Ils ouvrent les actions « Lire une note ou un échange » et « Voisinage d’une note » du panneau, dont le champ se remplit par le nom.
- **Suggestions pendant la frappe** : dans tout champ de note (panneau des actions, visionneur, mode expert), dès deux lettres tapées, une liste des notes dont le nom contient ce texte s’ouvre sous le champ, sans tenir compte des majuscules ni des accents. ↑ ↓ et Entrée, ou un clic, choisissent. Il n’est plus nécessaire d’ouvrir la liste déroulante.
- **Une note choisie s’affiche par son nom** : `Réveil perdu · 9b1e9e5d`, et plus seulement par son identifiant. Au moment de lancer la commande, l’interface en extrait l’identifiant. Pour un champ à plusieurs notes : `Réveil perdu · 9b1e9e5d, File · 3f2a91c0`.
  - Un identifiant tapé à la main reste accepté.
  - Le préfixe de 8 caractères suffit, comme aujourd’hui (R1).

---

## 2. Du code vers la note

**« Que désigne cette marque ? »** (`repere LIGNE`) accepte toute ligne qui contient une marque, quelle que soit la syntaxe de commentaire :
- elle emploie la détection de l’indexeur (`find_markers`, INDEXEUR.md, section 2) ;
- une ligne qui porte plusieurs marques donne chacune de leurs notes ;
- une marque mal formée est signalée.

C’est un **contrat modifié** : des lignes refusées aujourd’hui deviennent acceptées.

**Nouvelle action « Quelles notes concernent ce code ? »** (groupe Code), commande `code notes FICHIER [--ligne N | --extrait TEXTE] [--tsv]`.
- **Le fichier seul** :
  - les notes **liées à ce fichier** (ancrages actifs), avec ce qui est lié (fonction ou lignes) ;
  - les **marques présentes dans le fichier**, avec leur ligne.
- **Avec une ligne** (`--ligne 42`) :
  - en tête, les notes dont **l’extrait lié contient cette ligne** ;
  - la **marque la plus proche au-dessus** de la ligne, qui désigne en général la fonction où l’on se trouve ;
  - puis les autres notes du fichier.
- **Avec un extrait collé** (`--extrait`) : en tête, les notes dont l’extrait lié contient l’extrait collé, ou y est contenu, et les marques qu’il porte ; puis les autres notes du fichier.

Exemple de sortie :

```
Notes liées à src/sched.c :
  - Réveil perdu (9b1e9e5d-…) — fonction « void wake_task(struct task *t) » — contient la ligne 42
  - File d’attente (3f2a91c0-…) — lignes 80-95
Marques dans le fichier :
  - ligne 40 : Réveil perdu (9b1e9e5d-…) — la plus proche au-dessus de la ligne 42
```

**Règles :**
- **Chemins** : le fichier est comparé aux chemins des ancrages comme le fait l’indexeur, relativement au dossier courant. Si aucun ancrage ne porte exactement ce chemin, ceux dont le chemin **se termine** par le même nom de fichier sont proposés à part, sous « Chemin proche ». C’est utile quand le code a été lié depuis un autre dossier.
- **Lecture seule** : rien n’est écrit, ni dans le code (D9), ni dans le projet.
- Une marque dont la note n’existe pas sur la piste actuelle est signalée, comme dans l’indexeur.
- La forme `--tsv` donne une ligne par résultat, pour l’interface.

**Dans l’interface**, l’action pose trois questions :
- « Quel fichier ? », avec « Parcourir… » ;
- « Une ligne précise », à cocher : « Laquelle ? » ;
- « Un extrait collé », à cocher : « Collez l’extrait ».

Chaque note trouvée apparaît avec des boutons **« Lire »** et **« Voisinage »**, dans l’esprit de « Et ensuite ? ».

---

## 3. Noyau

Le classement est une règle : il va dans le noyau, testé et muté, à côté de l’indexeur (`markers.rs`).

- `Snapshot::code_notes(scope, root, path, text, focus) -> CodeNotes`, où `focus` vaut **fichier entier**, **ligne N** ou **extrait**. La fonction rend :
  - les ancrages actifs du fichier, chacun avec sa correspondance : contient la ligne, contient l’extrait, ou fichier seulement ;
  - les ancrages au chemin proche ;
  - les marques du texte, chacune avec sa note, connue ou non, et l’indication « la plus proche au-dessus ».

  L’ordre est fixe : correspondances d’abord, puis par ligne, puis par identifiant.
- **Aucune lecture de fichier dans le noyau.** La ligne de commande lit le fichier et lui passe son texte, comme pour `indexer`.
- `repere` passe par `find_markers`, qui existe déjà.

---

## 4. Invariants

| Identifiant | Énoncé |
|---|---|
| INV-CODE-07 | Retrouver les notes d’un code ne modifie ni le code ni le projet ; une note est rendue si et seulement si un ancrage actif désigne ce fichier (ou un chemin proche, signalé à part) ou si sa marque figure dans le texte ; une marque de note inconnue est signalée |

---

## 5. Découpage et tests

| Étape | Contenu |
|---|---|
| 7.2a | noyau : `code_notes` (chaque correspondance, marque la plus proche, chemin proche, marque inconnue, ordre) ; ligne de commande : `code notes` (sortie lisible et `--tsv`), `repere` sur toute ligne contenant une marque (contrat modifié) ; mutation sans survivant |
| 7.2b | interface : boutons du haut vers les actions, suggestions pendant la frappe, notes affichées par leur nom, action « Quelles notes concernent ce code ? » ; essais ; documents ; contrôle complet ; livraison `v0.7.0-alpha.2` |

**Tests prévus :**
- **Noyau** :
  - une note liée par sa fonction, une autre par des lignes, une troisième seulement marquée ;
  - recherche par ligne : dans l’extrait et hors de l’extrait ;
  - recherche par extrait ;
  - marque la plus proche ;
  - chemin proche ;
  - marque inconnue ;
  - ancrage retiré ignoré.
- **Ligne de commande** :
  - sortie exacte et `--tsv` ;
  - fichier inexistant (code 2) ;
  - aucune écriture ;
  - `repere` sur les quatre formes de commentaire et sur une ligne à deux marques.
- **Interface** :
  - « Voir une note… » ouvre l’action et trouve une note par son nom ;
  - la liste de suggestions apparaît en tapant ;
  - le champ montre `nom · id8` et la commande reçoit l’identifiant ;
  - l’action du code trouve la note depuis une ligne, puis « Lire ».

**Contrat modifié :**
- l’aide figée gagne une ligne (`code notes`), soit 48 commandes, et la phrase de `repere` change ;
- les tests qui refusaient une ligne de code contenant une marque sont adaptés.

---

## 6. Hors de 7.2

- Retrouver une note à partir d’un code **sans** ancrage ni marque, par ressemblance du texte.
- Ouvrir le fichier de code à la bonne ligne depuis une note (dépend de l’éditeur de l’utilisateur).
- Recherche dans le texte des notes, et pas seulement dans leur nom.

---

## 7. Points validés le 11 novembre 2026

1. **Les boutons « Voir une note… » et « Voisinage d’une note… »** ouvrent les actions du panneau (recherche par nom) ; **suggestions pendant la frappe** ; **notes affichées `nom · id8`**.
2. **`repere`** accepte toute ligne contenant une marque, quelle que soit la syntaxe (contrat modifié).
3. **L’action « Quelles notes concernent ce code ? »** : fichier seul, ou avec une ligne, ou avec un extrait collé ; notes liées, marques, « chemin proche » à part ; boutons « Lire » et « Voisinage » ; lecture seule.
4. **La commande `code notes FICHIER [--ligne N | --extrait TEXTE] [--tsv]`** (48 commandes).

---

## 8. Précisions apportées à la mise en œuvre

1. **Propositions** : elles se lisent « nom — genre », le genre dans le vocabulaire de l’interface (« idée », « solution technique »…), sans l’identifiant. La liste déroulante du champ propose les mêmes.
2. **Identifiant transmis** : l’interface retient l’identifiant complet de chaque note choisie, et c’est lui que reçoit la commande. Un identifiant tapé à la main passe tel quel. Une note pré-remplie par « Et ensuite ? » reste affichée par son identifiant.
3. **Champ à plusieurs notes** : on peut mêler notes choisies par leur nom et identifiants tapés. Un nom peut contenir une virgule.
4. **Extrait collé** : il couvre les lignes de sa première occurrence dans le fichier.
5. **« Que désigne cette marque ? »** : une ligne à plusieurs marques donne une fiche par note, séparées par une ligne vide. Une marque de note absente de la piste donne le même refus qu’avant.

