# État courant et historique — spécification technique

Version 0.2 — 6 novembre 2026 — **validée par l’utilisateur le 6 novembre 2026**, avant le code, avec D20 et D21.
Étape 2 de VERSION2.md (sections 3 et 8), incrément 5.2. Prérequis rempli : incrément 5.1 livré et vérifié (`v0.5.0-alpha.1` : format version 2, clés de ligne, différences, migration du texte).

Historique des versions :
- 0.1 (3 novembre 2026) : rédaction d’origine, restée hors du dépôt lors de la réinitialisation de l’environnement.
- 0.2 (6 novembre 2026) : fusion avec la proposition HISTORIQUE.md du 5 novembre 2026, retirée depuis. Ajouts : mesure du 5 novembre (section 1) ; lectures faillibles et `Error::Historique` (section 4) ; écriture par le trait, sans changer la signature d’`execute` (section 4) ; liste des branches sans le journal (sections 5 et 8) ; test d’indépendance par comptage des lectures (section 9) ; numérotation 5.2a à 5.2e (section 11) ; décisions D20 et D21 (section 13).

Ici, « historique » désigne l’historique **du logiciel** : les états successifs du réseau de notes (commits, branches, journal des opérations), conservés dans la base SQLite et servant à annuler, rétablir, changer de branche et restaurer. L’historique Git du code source n’est pas concerné.

---

## 1. Problème et objectif

**Constat mesuré le 31 octobre 2026** (exécutable optimisé, notes reliées en chaîne) :

| Opérations dans l’historique | Durée de `statut` | Taille de la base |
|---|---|---|
| 101 | 0,05 s | 1,7 Mo |
| 303 | 0,35 s | 13 Mo |
| 505 | 1,05 s | 36 Mo |
| 606 | 1,44 s | 51 Mo |

**Mesure complémentaire du 5 novembre 2026** (exécutable optimisé, `memoire-store` appelé directement, notes créées puis liées à la précédente) : elle sépare l’ouverture de la base de l’exécution de la commande.

| Opérations | Notes | Ouverture de la base | Exécution d’une commande | Base SQLite | Texte v1 | Texte v2 |
|---|---|---|---|---|---|---|
| 200 | 100 | 0,08 s | 0,002 s | 7,8 Mo | 7,4 Mo | 0,33 Mo |
| 602 | 300 | 0,69 s | 0,015 s | 67 Mo | 66 Mo | 1,4 Mo |
| 1 205 | 600 | **2,81 s** | 0,051 s | **263 Mo** | 261 Mo | 4,1 Mo |

L’ouverture représente presque toute la durée : la cause 2 ci-dessous domine. Le texte v2 est 63 fois plus petit que la base actuelle à 1 205 opérations.

**Les deux causes :**
1. Chaque commit stocke l’état complet du réseau, sur disque comme en mémoire.
2. Chaque commande (donc chaque clic de l’interface, qui lance l’exécutable) ouvre la base et reconstruit tout l’historique avant de faire quoi que ce soit.

**Objectif.** Le coût d’une commande ordinaire dépend de la taille de l’état courant de la branche active, jamais de la longueur de l’historique. L’historique reste entier dans la même base, consultable par le logiciel (D6) ; il n’est lu que lorsqu’un usage en a besoin.

**Critères d’acceptation mesurés** (section 10) :
- `statut` et une écriture ordinaire en moins de 0,2 s sur un dépôt de 5 000 opérations et 1 500 notes ;
- rapport des durées entre 5 000 et 1 000 opérations inférieur à 1,5 ;
- taille de la base linéaire en nombre d’opérations ;
- critère déterministe, sans chronomètre : une commande ordinaire et `statut` lisent le même nombre de lignes de l’historique à 100, 1 000 et 5 000 opérations (section 9, point 8).

**Hors objectif :** changer le comportement observable. Tous les contrats existants (domaine, stockage, service, ligne de commande, recette S-REF-01) restent valables ; les seules modifications admises sont des changements de signature listés en section 8, chacun consigné comme « contrat modifié ».

---

## 2. Principe : deux parties dans la même base

| Partie | Contenu | Lue par |
|---|---|---|
| **1. État courant** | pour chaque branche non abandonnée, l’état complet de sa tête, rangé ligne par ligne (une ligne canonique de l’instantané version 2 par enregistrement, repérée par sa clé) | toute commande ordinaire, l’aperçu, `statut`, `repere`, `chercher` |
| **2. Historique** | commits (parents, motif, profondeur, différence ou état de référence), journal des opérations, identités employées, pile de rétablissement | annuler, rétablir, restaurer, revoir un état ancien, base commune d’une fusion, répétition d’une opération, contrôle d’identité |

**Règle de cohérence, absolue :** l’état courant d’une branche est toujours égal à l’état reconstruit depuis l’historique pour la tête de cette branche. Les deux parties sont écrites dans la **même transaction SQLite** ; il n’existe aucun chemin d’écriture qui modifie l’une sans l’autre. C’est la seule redondance du système, et elle est contrôlée (section 9).

---

## 3. Schéma SQLite, version 2

```sql
CREATE TABLE meta (
  cle    TEXT PRIMARY KEY,           -- 'schema' = '2', 'format' = 'memoire-depot 2',
  valeur TEXT NOT NULL               -- 'intervalle' = '100'
);

CREATE TABLE branches (
  id      TEXT PRIMARY KEY,          -- UUID canonique
  tete    TEXT NOT NULL REFERENCES commits(id),
  origine TEXT,                      -- NULL pour main
  base    TEXT NOT NULL REFERENCES commits(id),
  etat    TEXT NOT NULL CHECK (etat IN ('active', 'abandonnee'))
);

CREATE TABLE active (
  unique_ligne INTEGER PRIMARY KEY CHECK (unique_ligne = 1),
  branche      TEXT NOT NULL REFERENCES branches(id)
);

CREATE TABLE retablir (               -- pile de rétablissement par branche
  branche TEXT NOT NULL REFERENCES branches(id),
  rang    INTEGER NOT NULL,
  ligne   TEXT NOT NULL,             -- ligne canonique « retablir … » du format v2
  PRIMARY KEY (branche, rang)
);

CREATE TABLE identites (
  categorie TEXT NOT NULL,           -- note, relation, contexte, ancrage, commit, branche, operation
  id        TEXT NOT NULL,
  PRIMARY KEY (categorie, id)
);

CREATE TABLE commits (
  id         TEXT PRIMARY KEY,
  parent1    TEXT REFERENCES commits(id),   -- NULL pour la racine
  parent2    TEXT REFERENCES commits(id),   -- fusion (étape 4) ; NULL sinon
  motif      TEXT NOT NULL,
  profondeur INTEGER NOT NULL,              -- le long des premiers parents ; racine = 0
  nature     TEXT NOT NULL CHECK (nature IN ('reference', 'difference'))
);

CREATE TABLE lignes_reference (       -- état complet des commits de nature 'reference'
  commit_id TEXT NOT NULL REFERENCES commits(id),
  cle       TEXT NOT NULL,
  ligne     TEXT NOT NULL,
  PRIMARY KEY (commit_id, cle)
) WITHOUT ROWID;

CREATE TABLE lignes_difference (      -- différence avec le premier parent
  commit_id TEXT NOT NULL REFERENCES commits(id),
  rang      INTEGER NOT NULL,         -- ordre : retraits puis ajouts, chacun par clé croissante
  sens      TEXT NOT NULL CHECK (sens IN ('-', '+')),
  cle       TEXT NOT NULL,
  ligne     TEXT NOT NULL,
  PRIMARY KEY (commit_id, rang)
) WITHOUT ROWID;

CREATE TABLE etat_courant (           -- partie 1
  branche TEXT NOT NULL REFERENCES branches(id),
  cle     TEXT NOT NULL,
  ligne   TEXT NOT NULL,
  PRIMARY KEY (branche, cle)
) WITHOUT ROWID;

CREATE TABLE operations (             -- journal : bloc canonique v2 de l’opération
  op   TEXT PRIMARY KEY,
  bloc TEXT NOT NULL                  -- commande, résultat et effets, comme dans l’export
);
```

**Forme textuelle des clés** (colonne `cle`), choisie pour que l’ordre lexicographique de SQLite soit l’ordre canonique (VERSION2.md, section 4). Les UUID sont en minuscules, forme canonique : leur ordre lexicographique est celui de leurs 16 octets.

| Clé (VERSION2.md) | Texte |
|---|---|
| `Note(n, Contenu)` | `n/<n>/0` |
| `Note(n, Revision(r))` | `n/<n>/1/<r sur 10 chiffres>` |
| `Note(n, Corps)` | `n/<n>/2` |
| `Ancrage(a)` | `a/<a>` |
| `Contexte(s, Entete)` | `s/<s>/0` |
| `Contexte(s, Membre(n))` | `s/<s>/1/<n>` |
| `Contexte(s, Relation(e))` | `s/<s>/2/<e>` |
| `Contexte(s, Acquittement(d, c))` | `s/<s>/3/<d>/<c>` |

Un test de propriété vérifie que, pour tout couple de clés, l’ordre des textes égale l’ordre de `Cle`.

**Réglages :** `journal_mode = WAL`, `synchronous = FULL` (inchangés) ; une transaction par commande.

---

## 4. Le noyau : interface de lecture de l’historique

Le domaine reste sans entrée-sortie et sans dépendance (D14). Il ne garde plus un instantané par commit ; il garde l’état de la tête de la branche active et lit le reste par un trait :

```rust
/// Historique (partie 2) et état courant des branches (partie 1), fournis par le stockage ou,
/// pour les tests, en mémoire. Toute lecture peut échouer (disque, base corrompue).
pub trait History {
    /// État complet d'un commit : tête d'une branche → état courant ; sinon état de référence
    /// le plus proche le long des premiers parents, puis au plus 99 différences.
    fn state(&self, commit: CommitId) -> Result<Arc<Snapshot>, Error>;
    /// Parents, motif et profondeur d'un commit ; Ok(None) s'il est inconnu.
    fn commit(&self, commit: CommitId) -> Result<Option<CommitInfo>, Error>;
    /// Différence d'un commit avec son premier parent (None pour un état de référence).
    fn difference(&self, commit: CommitId) -> Result<Option<Difference>, Error>;
    /// Opération déjà appliquée, pour la répétition sûre (INV-RETRY-01).
    fn operation(&self, op: OpId) -> Result<Option<(Envelope, Outcome)>, Error>;
    /// Identité déjà employée (INV-ID-02, INV-ID-04).
    fn is_used(&self, id: UsedId) -> Result<bool, Error>;
    /// Branches dont l'état courant contient la note (pour `branches_containing`).
    fn branches_containing(&self, node: NodeId) -> Result<Vec<BranchId>, Error>;
    /// Applique les écritures d'une opération, tout ou rien (une transaction SQLite).
    fn apply(&mut self, writes: &Writes) -> Result<(), Error>;
}
```

**Erreurs.** Une nouvelle variante `Error::Historique(String)` porte l’échec d’une lecture ou d’une écriture de l’historique, avec le message du stockage. `MemoryHistory` ne la produit jamais : le contrat du domaine n’en change pas. La ligne de commande la traduit en code 3 (stockage), et non en code 1 (refus métier), comme une erreur SQLite aujourd’hui.

**Ce que le noyau garde en mémoire :**
- les métadonnées de toutes les branches (tête, origine, base, état, pile de rétablissement) : petites ;
- la branche active et l’état de sa tête (`Arc<Snapshot>`) ;
- l’historique lui-même, par un paramètre de type avec l’historique en mémoire par défaut : `Repository<H: History = MemoryHistory>` (décision D20, section 13). `Clone` et `PartialEq` sont dérivés quand `H` les fournit : ils comparent alors tout, historique compris, comme aujourd’hui ; `Repository::new()` reste le dépôt en mémoire des contrats. Le service et la ligne de commande emploient un alias, `type Depot = Repository<SqliteHistory>`.

**Ce que produit une opération.** `execute` garde sa signature et son comportement observable, mais ne modifie plus seul son état interne. Il procède en trois temps :
1. `plan` (lecture seule, peut lire l’historique) ; tout refus survient ici ;
2. calcul des **écritures** (`Writes`), puis `history.apply(&writes)?`, tout ou rien ;
3. mise à jour de la partie en mémoire (tête, branches, branche active), infaillible, seulement après la réussite de l’écriture.

Un échec de l’étape 2 laisse le dépôt identique (INV-TX-01), sans copie complète du dépôt : aujourd’hui, le stockage exécute chaque commande sur une copie. `Repository::prepare(&self, envelope) -> Result<Writes, Error>` est public, pour que le contrat vérifie les écritures elles-mêmes.

```rust
/// Écritures d'une opération réussie (partie 1 et partie 2), appliquées en une transaction.
pub struct Writes {
    pub commit: Option<NewCommit>,          // id, parents, motif, profondeur, contenu
    pub branches: Vec<(BranchId, BranchMeta)>,
    pub active: Option<BranchId>,
    pub operation: (OpId, Envelope, Outcome),
    pub identities: Vec<UsedId>,
    pub current: Vec<CurrentChange>,         // (branche, clé, ancienne ligne, nouvelle ligne)
}

pub enum CommitContent {
    Reference(BTreeMap<Cle, String>),       // profondeur multiple de 100
    Difference { removed: Vec<String>, added: Vec<String> },
}
```

Les annulations, rétablissements et changements de branche produisent aussi des écritures (métadonnées de branche, pile de rétablissement, état courant), sans nouveau commit.

**Implémentation en mémoire :** `MemoryHistory`, dans le domaine (il faut pouvoir écrire `Repository::new()` et `decode_repository`), conserve tout en mémoire, un instantané complet par commit comme aujourd’hui, et applique les `Writes`. Elle sert aux contrats du domaine, qui ne dépendent donc pas de SQLite, et de référence aux tests d’équivalence du stockage. `decode_repository` et `decode_repository_v2` rendent un dépôt en mémoire ; l’import l’écrit ensuite dans le schéma 2.

---

## 5. Comportement de chaque usage

| Usage | Lecture | Écritures (une transaction) | Coût |
|---|---|---|---|
| Ouverture de la base | `meta`, `branches`, `active`, `retablir`, `etat_courant` de la branche active | aucune | O(état courant) |
| Aperçu (`preview`) | état de tête en mémoire | aucune | O(état) |
| Commande ordinaire sur la branche active | état de tête ; `is_used` pour chaque identité nouvelle ; `operation(op)` | commit (différence avec la tête), tête de branche, pile de rétablissement vidée, opération, identités, lignes changées de `etat_courant` | O(état) en calcul, O(changement) en écriture |
| Commande critique depuis main (D1, D13) | idem | nouvelle branche ; `etat_courant` de la nouvelle branche = copie de celui de main, plus la différence ; main inchangée | O(état) une fois par bifurcation |
| Annuler | différence du commit de tête (si c’est un état de référence : état du parent par `state`) | tête = parent, pile de rétablissement, lignes de `etat_courant` inversées | O(changement), ou O(état + 99 différences) |
| Annuler l’opération fondatrice d’une branche | — | la branche active redevient l’origine ; son `etat_courant` existe déjà | O(1) en écriture, O(état) à la relecture |
| Rétablir | différence du commit rétabli | inverse de l’annulation | O(changement) |
| Changer de branche | `etat_courant` de la branche cible | `active` | O(état) |
| Restaurer un état (D6) | `state(cible)` et état de tête | nouveau commit (différence), `etat_courant` | O(état + 99 différences) |
| Répétition d’une opération | `operation(op)` | aucune | O(1) |
| `repere` sur une note absente de la branche active | `branches_containing` (requête sur `etat_courant` par clé `n/<note>/0`) | aucune | O(branches) |
| Lister les branches (`basculer`, autocomplétion) | métadonnées de branches, en mémoire : nouvelle méthode `Repository::branches()` | aucune | O(branches) ; aujourd’hui, la ligne de commande déduit cette liste du journal complet |
| Export | tout l’historique, par parcours des tables dans l’ordre canonique | aucune | O(historique), assumé : action explicite |
| Base commune d’une fusion (étape 4) | `state(ancêtre commun)` | — | O(état + 99 différences) |

**Reconstruction d’un état ancien (`state`) :**
1. remonter les premiers parents depuis la cible jusqu’au commit de profondeur `p − p mod 100` (au plus 99 pas, par `commit`) ;
2. lire ses `lignes_reference` ;
3. appliquer dans l’ordre les `lignes_difference` des commits suivants, avec les contrôles d’intégrité de VERSION2.md (ligne retirée exactement présente, ligne ajoutée absente) ;
4. assembler et décoder (`decode_snapshot_v2`), ce qui revalide l’état.

Un petit cache des derniers états reconstruits (quatre, par exemple) est permis dans le stockage ; il ne change aucun résultat.

**Calcul d’une différence.** `difference(lignes(tête), lignes(candidat))` (incrément 5.1) suffit : son coût est proportionnel à l’état courant, ce qui respecte l’objectif. Une optimisation qui ne recalcule que les clés touchées est permise plus tard, à condition de produire exactement la même différence (test d’égalité avec le calcul complet).

---

## 6. Écriture et durabilité

- **Une transaction par commande :** toutes les tables touchées par les `Writes` (commits, lignes, branches, `active`, `retablir`, `operations`, `identites`, `etat_courant`) sont écrites dans la même transaction, puis validées. La tête en mémoire n’est remplacée qu’après la validation.
- **Arrêt brutal (INV-DUR-01) :** après réouverture, la base correspond à l’état d’avant ou d’après la commande, jamais à un mélange. Le test existant (processus tués pendant l’écriture) est conservé et étendu au schéma 2.
- **Répétition (INV-RETRY-01) :** une opération déjà présente dans `operations` n’écrit rien et rend le résultat enregistré.
- **Aucune suppression (D6) :** seules les lignes de `etat_courant` sont remplacées ou retirées, puisque c’est l’état courant ; rien n’est jamais effacé des tables de l’historique. Une branche abandonnée (étape 4) perd ses lignes `etat_courant`, reconstructibles par `state(tête)`.

---

## 7. Migration du schéma 1 vers le schéma 2

1. **Détection :** à l’ouverture, `meta.schema = '1'` (ou table `fragments` présente) ; l’exécutable refuse d’écrire dans une base de schéma 1 et propose la migration ; une base d’un schéma plus récent que l’exécutable est refusée avec un motif.
2. **Confirmation :** `memoire migrer` (ou une question de l’interface) ; jamais automatique.
3. **Sauvegarde :** copie cohérente vers `sauvegardes/avant-migration-AAAAMMJJ-HHMMSSZ.db` (mécanisme de `sauvegarder`, R9). Si elle échoue, rien ne se passe.
4. **Conversion,** dans un **nouveau fichier** temporaire du même répertoire : lecture complète par le chemin existant (fragments, puis `decode_repository`), puis écriture du schéma 2 : profondeurs, états de référence tous les cent commits, différences, `etat_courant` de chaque tête de branche, journal, identités.
5. **Contrôle :** dans le nouveau fichier, reconstruire chaque commit par `state` et le comparer à l’état d’origine ; comparer `etat_courant` de chaque branche à l’état de sa tête ; comparer le journal. Au moindre écart, le fichier temporaire est supprimé et la base d’origine reste intacte.
6. **Remplacement :** `fsync` du nouveau fichier, renommage atomique sur l’ancien, `fsync` du répertoire (même protocole que l’export).

---

## 8. Changements d’interface à consigner (« contrat modifié »)

| Avant | Après | Raison |
|---|---|---|
| `Repository::snapshot_at(&self, c) -> Result<&Snapshot, Error>` | `-> Result<Arc<Snapshot>, Error>` | l’état est reconstruit, non plus emprunté |
| `Repository::branches_containing(n) -> Vec<BranchId>` | `-> Result<Vec<BranchId>, Error>` | lecture de l’état courant des autres branches |
| `Repository::commit_reason(c) -> Result<&str, Error>` | `-> Result<String, Error>` | le motif est lu, non plus emprunté |
| `Repository::journal()` (itérateur sur tout le journal en mémoire) | réservé au dépôt en mémoire (contrat) ; `History::operation(op)` pour la répétition ; parcours complet pour l’export | le journal n’est plus chargé |
| la ligne de commande liste les branches par le journal | `Repository::branches()`, métadonnées en mémoire | même raison |
| `Store::execute` sur une copie du dépôt | `Repository::execute` avec `SqliteHistory` : écriture, puis mise à jour de la mémoire | atomicité sans copie |
| tests du stockage et du service : `repository() == …` | comparaison des textes canoniques v2 complets, ou avec l’oracle en mémoire (D20) | un dépôt SQLite n’a ni `Clone` ni `PartialEq` |
| `Fragments` (schéma 1) | remplacé par les tables du schéma 2 pour l’écriture ; conservé, avec son contrat, pour lire les bases de schéma 1 lors de la migration | le stockage n’enregistre plus le texte complet de chaque commit |

Les autres méthodes publiques, `execute` comprise, gardent leur signature et leur comportement. Ordre de grandeur : une dizaine de lignes du contrat du domaine et une vingtaine des contrats du stockage et du service, dans un commit « Contrat modifié » isolé ; aucune assertion n’est affaiblie.

---

## 9. Tests et vérification

1. **Oracle : l’implémentation actuelle.** Conserver le `Repository` actuel, entièrement en mémoire, comme modèle de référence : copie figée du domaine de `v0.5.0-alpha.1` dans une crate de test temporaire (`crates/memoire-reference`, `publish = false`, sans dépendance), retirée une fois 5.2c vérifié. Une copie dans `tests/` ne suffirait pas : elle n’aurait pas accès aux types internes du domaine. Cette référence est indépendante de `MemoryHistory`, qui est réécrit en 5.2b. Le test de propriété principal exécute les mêmes suites aléatoires de commandes (celles de `tests/proprietes.rs`, y compris annuler, rétablir, changer de branche, restaurer) sur l’oracle et sur la nouvelle implémentation, avec `MemoryHistory` puis avec le stockage SQLite. Après **chaque** commande : même résultat, même état de tête, mêmes métadonnées de branches, et, pour un échantillon de commits, `state(c)` égal à l’instantané de l’oracle.
2. **Cohérence des deux parties :** après chaque commande, `etat_courant` de chaque branche égale `state(tête)`.
3. **Contrats existants :** tous verts, aux changements de la section 8 près, chacun consigné.
4. **Stockage :** réouverture à l’identique ; arrêt brutal ; répétition ; sauvegarde ; clés textuelles dans l’ordre canonique (propriété).
5. **Migration :** une base de schéma 1 construite par le code actuel (scénario S-REF-01 et dépôts aléatoires) migre, se relit et donne le même dépôt ; écart provoqué → base d’origine intacte ; sauvegarde présente.
6. **Contrôle profond :** commande `memoire verifier` (lecture seule) qui reconstruit chaque commit, compare chaque `etat_courant` à `state(tête)` et vérifie le journal ; utilisable après une migration et pour le diagnostic.
7. **Écritures et erreurs (contrat du domaine) :** une commande acceptée produit exactement les écritures attendues (`prepare`) ; une commande refusée ou répétée n’en produit aucune ; un historique d’essai dont `apply` ou une lecture échoue laisse le dépôt identique, avec `Error::Historique`.
8. **Indépendance de la longueur de l’historique, sans chronomètre :** `SqliteHistory` compte les lignes qu’il lit ; une commande ordinaire et `statut` en lisent le même nombre à 100, 1 000 et 5 000 opérations. Contrairement à une durée, ce nombre est déterministe : il peut figurer dans le contrat du stockage.
9. **Corruption d’un commit ancien** (D21) : ouverture acceptée, lecture de cet état refusée avec un motif, `memoire verifier` refusé.
10. **Mutation :** sans survivant, comme d’habitude. Garder le contrat rapide (2 à 3 s) : les tests de performance (section 10) sont hors contrat.

---

## 10. Protocole de mesure de la performance

- Exécutable en mode release, machine de développement, base sur disque local.
- Programme de mesure versionné : `crates/memoire-store/examples/mesure.rs` (hors contrat et hors mutation).
- Dépôt synthétique : 1 500 notes reliées (chaînes et étoiles), changements de statut, quelques bifurcations et annulations, jusqu’à 5 000 opérations, construit par un programme d’essai reproductible (graine fixée), pas par la ligne de commande (trop lent avec l’ancienne version).
- Mesures à 1 000, 2 500 et 5 000 opérations : `memoire statut`, une création de note, une modification de statut, `annuler`, `basculer`, et la taille du fichier.
- Résultats consignés dans le journal de vérification de l’incrément, avec les mêmes mesures pour l’ancienne version (schéma 1) jusqu’où elle reste praticable.
- Critères : section 1.

---

## 11. Découpage proposé

Numérotation alignée sur REPRISE.md, section 7, où 5.3 désigne les opérations groupées et 5.4 la fusion.

| Incrément | Contenu | Livraison |
|---|---|---|
| 5.2a | programme de mesure (section 10) ; mesure de référence de la version actuelle | chiffres de départ consignés |
| 5.2b | domaine : `History`, `MemoryHistory`, `Writes`, `prepare`, tête seule en mémoire, `Error::Historique`, `Repository::branches()` ; oracle et propriétés d’équivalence ; le stockage reste au schéma 1, branché sur `MemoryHistory` | contrats verts, mutation ; aucun changement visible pour l’utilisateur |
| 5.2c | stockage schéma 2, `SqliteHistory`, ouverture rapide, arrêt brutal, comptage des lectures, `memoire verifier` | contrats du stockage, du service et de la ligne de commande verts, mutation |
| 5.2d | migration schéma 1 → 2 avec sauvegarde, confirmation et contrôle ; bouton de l’interface | essai sur une base réelle de l’utilisateur, avec son accord |
| 5.2e | mesure selon la section 10, optimisations si un critère n’est pas atteint ; STOCKAGE.md, FORMAT.md et INVARIANTS.md à jour ; livraison `v0.5.0-alpha.2` | rapport chiffré, contrôle complet |

5.2b laisse le logiciel dans un état intermédiaire livrable et sans risque pour les données : c’est le point d’arrêt naturel si une difficulté de conception apparaît.

---

## 12. Risques

- **Deux copies de l’état courant** (partie 1 et reconstruction par la partie 2) : maîtrisé par la transaction unique, la propriété de cohérence (section 9, point 2) et `memoire verifier`.
- **Refactorisation du cœur du noyau :** maîtrisée par l’oracle ; ne supprimer l’ancienne implémentation (`memoire-reference`) qu’après l’incrément 5.2c vérifié.
- **Migration des données réelles :** sauvegarde préalable, nouveau fichier, contrôle complet avant remplacement, base d’origine intacte en cas d’écart.
- **Validation de l’état à chaque ouverture** (`decode_snapshot_v2` revalide l’état) : son coût est proportionnel à l’état courant ; à mesurer en 5.2e ; ne pas la supprimer sans décision consignée.
- **Divergence entre `MemoryHistory` et `SqliteHistory`** sur un point que le contrat du domaine ne voit pas : couverte par l’équivalence après chaque commande, par la mutation de `memoire-store` contre ces tests, et par les propriétés.
- **Paramètre de type `Repository<H>`** : il se propage dans les signatures du service ; caché par un alias. Risque faible, à surveiller pour la lisibilité.

---

## 13. Décisions validées

- **D20 — Égalité des dépôts et place du contrat** (DECISIONS.md) : `Repository<H = MemoryHistory>` ; le contrat du domaine reste écrit sur l’historique en mémoire, avec son égalité complète ; l’historique SQLite est vérifié par équivalence avec lui. Options écartées : égalité par texte canonique partout (contrat plus lent, diagnostics moins lisibles) ; égalité limitée à la partie en mémoire (affaiblit le contrat en silence).
- **D21 — Vérifications à l’ouverture et vérification complète** (DECISIONS.md) : l’ouverture valide les métadonnées et l’état courant de la branche active, plus tout l’historique ; un état ancien est validé à chaque lecture ; la vérification complète a lieu à l’import, après la migration, à l’export et par `memoire verifier`. Elle révise le principe de STOCKAGE.md, section 2 (« une seule voie de lecture et de validation »).

---

## 14. Précisions d’implémentation (incrément 5.2b, 6 novembre 2026)

Écarts de forme par rapport aux sections 4 et 8, sans changement de principe :

- **Trait `History`** : trois lectures s’ajoutent pour l’ouverture (`active`, `branches`, `commit_count`), car `Repository::open(history)` charge la partie en mémoire depuis l’historique. `difference` n’est pas dans le trait : le domaine n’en a pas besoin (annuler lit l’état du parent par `state`) ; l’historique SQLite l’emploiera en interne. `CommitInfo` porte les parents et le motif, sans la profondeur, dont seul le stockage a l’usage.
- **`Writes`** : enveloppe, résultat, commit publié (`NewCommit` : identité, parent, motif, état), branches créées ou modifiées dans leur état après, branche active, identité nouvelle, état de la tête active après. Pas de champ « lignes de l’état courant changées » : l’historique SQLite les calculera lui-même à partir des états (`difference.rs`), le domaine n’ayant pas à connaître la représentation de `etat_courant`.
- **`Prepared`** (`Repeated`, `NoEffect`, `Write`) : rendu par `Repository::prepare`, pour que le contrat vérifie les écritures et l’absence d’écriture d’un refus, d’une répétition ou d’une commande sans effet.
- **`snapshot_at`, `commit_reason` et `branches_containing`** passent toujours par l’historique, y compris pour la tête active : une lecture en échec est donc signalée même pour elle.
- **`journal()`** reste disponible sur le dépôt en mémoire seulement (`impl Repository<MemoryHistory>`).
- **Oracle** : la crate `memoire-reference` et son test d’équivalence (48 graines, 80 commandes, comparaison après chaque commande) sont verts après la refonte.

## 15. Précisions d’implémentation (incrément 5.2c, 6 novembre 2026)

- **Correction de la section 3 (forme textuelle des clés).** Les préfixes `n/`, `a/` et `s/` rangeaient les ancrages avant les notes (`a` < `n`), contrairement à l’ordre canonique. Les clés sont `1/<note>/0`, `1/<note>/1/<rang>`, `1/<note>/2`, `2/<ancrage>`, `3/<portée>/0`, `3/<portée>/1/<note>`, `3/<portée>/2/<relation>`, `3/<portée>/3/<dépendant>/<cause>` ; le rang est écrit sur 20 chiffres (longueur de `u64::MAX`). Un test du contrat vérifie que leur ordre est l’ordre du texte.
- **Une seule définition du format.** Le stockage n’écrit que des morceaux produits par le domaine (`state_lines`, `encode_active`, `encode_branch_v2`, `encode_identity`, `encode_commit_v2`, `encode_operation_v2`, `commit_records`) ; `encode_repository_v2` est assemblé des mêmes morceaux, et le texte rassemblé depuis les tables est comparé à celui de l’oracle dans les tests.
- **Vérification complète à mémoire bornée** : `verify_repository_v2` applique les règles de `decode_repository_v2` sans garder l’état de chaque commit ; les lignes d’un commit sont libérées dès que son dernier enfant est reconstruit (une seule table pour un historique linéaire, quelle que soit sa longueur). `Store::verify` compare en plus l’état courant de chaque branche à celui que redonne son historique.
- **Schéma 2 effectif** : tables de la section 3, avec `identites(cle, ligne)` (clé d’ordre canonique et ligne `identite`), `retablir(branche, rang, sorte, cible, tete)`, un index sur `etat_courant(cle)` et un autre sur `branches(tete)`. L’état d’un commit tête de branche se lit dans `etat_courant` ; les autres, par la référence la plus proche et au plus 99 différences.
- **Import** : le texte est décodé en mémoire (v1 ou v2), écrit dans une base neuve, puis la base relue doit redonner exactement son texte v2, sinon le fichier est retiré.
- **Ligne de commande** : `memoire verifier` ; un historique illisible sort en code 3.
- **Tests** : contrat du stockage réécrit (D20) et étendu ; propriétés aléatoires à travers SQLite (24 graines, 80 commandes, comparaison après chaque commande, réouvertures aléatoires).

---

## 16. Précisions d’implémentation (incrément 5.2d, 6 novembre 2026)

Elles précisent la section 7 sans la changer.

- **Bases concernées.** Le schéma 1 (tables `meta` et `fragments`, format `memoire-depot 1`) est celui de toutes les versions de `v0.3.0` à `v0.5.0-alpha.1`. Une base que `v0.5.0-alpha.1` ouvrait est migrable ; la lecture applique exactement ses règles : catégories connues, texte rassemblé lu par `decode_repository`, fragments rangés sous leurs clés canoniques.
- **Répartition.**
  - *Stockage :* `Store::schema(chemin)` lit la version du schéma sans rien modifier. `Store::migrate(source, sauvegarde, cible)` ouvre la source **sans jamais y écrire**, refuse tout schéma autre que 1 (sans rien créer), refuse une sauvegarde ou une cible existantes (sans rien créer), écrit la sauvegarde (`VACUUM INTO`, cible neuve), lit le dépôt, l’écrit dans `cible` (fichier neuf) par le chemin de l’import, puis fait la vérification complète de la cible : texte v2 rassemblé identique à celui du dépôt lu, et `verify` (D21 : chaque commit reconstruit, état courant de chaque branche confirmé). Au moindre écart, la cible est retirée ; la sauvegarde, déjà écrite, est conservée. La connexion à la source est ouverte en lecture-écriture mais n’écrit rien : en lecture seule, SQLite laisserait des fichiers `-wal` et `-shm` à côté d’une base en mode WAL (constaté avec SQLite 3.51) ; à la fermeture, il intègre un éventuel journal laissé par un arrêt brutal de l’ancienne version, comme celle-ci l’aurait fait à l’ouverture. Une base fermée proprement reste identique octet pour octet.
  - *Service :* `Service::migrate(base, sauvegarde)` enchaîne : schéma déjà 2 → rien à faire, aucune sauvegarde ; sinon `Store::migrate` vers un fichier temporaire du même répertoire, synchronisation du fichier, renommage atomique sur la base, synchronisation du répertoire (protocole de l’export). Avant le renommage, toutes les connexions de la migration étant fermées, la présence d’un journal `-wal` à côté de la base signale qu’un autre processus l’a ouverte : la migration est alors refusée et le fichier temporaire retiré. Un renommage sur une base en cours d’usage pourrait faire rejouer ce journal sur le nouveau fichier.
  - *Ligne de commande :* `memoire migrer`, sans argument ; la commande est la confirmation (section 7, point 2). Sauvegarde dans `sauvegardes/avant-migration-AAAAMMJJ-HHMMSSZ.db` à côté de la base. Sorties : « Sauvegarde écrite : … » puis « Base migrée au schéma 2 : n commits, m branches, vérifiée. » ; une base déjà au schéma 2 donne « Base déjà au schéma 2 : rien à migrer. » (code 0). Tout échec sort en code 3, la base d’origine intacte.
  - *Interface :* le bouton `migrer` vient de l’aide, comme les autres. Quand une commande échoue avec le message du schéma 1, l’interface propose la migration par une question (oui / non) ; jamais automatique.
- **Mémoire.** La lecture d’une base de schéma 1 charge tout le dépôt en mémoire, comme l’ouverture de `v0.5.0-alpha.1` : coût unique, du même ordre que cette ouverture.
- **Tests.**
  - *Exemple figé* `fixtures/schema1/` : base écrite par l’exécutable de `v0.5.0-alpha.1` en rejouant S-REF-01 par la ligne de commande (recette de l’incrément 4.2, branche, annulations, pile de rétablissement), avec l’export v2 que cette même version en donnait ; empreintes SHA-256. La base migrée doit redonner exactement cet export : la migration est jugée par l’ancienne version, non par le code qui migre.
  - *Dépôts aléatoires :* bases de schéma 1 écrites comme `v0.5.0-alpha.1` les écrivait (fragments du domaine), migrées, comparées au texte de l’oracle.
  - *Refus :* source corrompue ou non canonique, schéma inconnu, base déjà au schéma 2 (stockage), sauvegarde ou cible existantes. Après chaque refus, la base d’origine est identique octet pour octet, et le fichier temporaire absent.
  - *Service, ligne de commande et interface :* migration de l’exemple figé, refus d’ouvrir avant, commandes ordinaires après, seconde migration sans effet, sauvegarde présente ; aide figée (31 commandes).
- **Oracle retiré** (comme prévu par la section 9, point 1) : `crates/memoire-reference` est supprimé une fois 5.2c vérifié (contrôle complet du commit 77febbe). L’équivalence reste garantie par `MemoryHistory` (D20) et par les propriétés aléatoires à travers SQLite (`crates/memoire-store/tests/proprietes.rs`), comparées après chaque commande.
