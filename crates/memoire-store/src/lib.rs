//! Stockage SQLite d'un dépôt `memoire-domain`, schéma 2 (ETAT-HISTORIQUE.md ; D4, D14, D20, D21).
//!
//! La base fait autorité. Elle a deux parties, écrites dans la même transaction :
//! - l'**état courant** de la tête de chaque branche, ligne canonique par ligne canonique ;
//! - l'**historique** : commits (état de référence tous les cent commits, différence avec le
//!   premier parent ailleurs), journal des opérations, identités employées, branches.
//!
//! [`SqliteHistory`] implémente le trait [`History`] du domaine : le domaine décide, le stockage
//! rend durable. Toutes les lignes stockées sont des morceaux du texte canonique v2, produits par
//! le domaine ; le texte complet se rassemble depuis les tables ([`Store::text`]) et se vérifie
//! en entier sur demande ([`Store::verify`]). L'ouverture ne lit que les métadonnées, les
//! branches et l'état courant de la branche active (D21).

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use memoire_domain::{
    commit_records, decode_operation_v2, decode_repository, decode_repository_any, encode_active,
    encode_branch_v2, encode_commit_parents_v2, encode_identity, encode_operation_v2,
    encode_repository_v2, holds_reference, identity_key, line_key, state_difference,
    state_from_lines, state_lines, verify_repository_v2, Branch, BranchId, Category, CommitBlock,
    CommitContent, CommitId, CommitInfo, DecodeError, Envelope, Error, Fragments, History, NodeId,
    OpId, Outcome, Redo, Repository, Snapshot, UsedId, Verified, Writes, REPOSITORY_HEADER,
    REPOSITORY_HEADER_V2, SNAPSHOT_HEADER_V2,
};
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Params, Row, TransactionBehavior,
};

/// Version du schéma de la base.
pub const SCHEMA_VERSION: &str = "2";

const SCHEMA: &str = "
CREATE TABLE meta (
    cle TEXT PRIMARY KEY,
    valeur TEXT NOT NULL
) STRICT;
CREATE TABLE branches (
    id TEXT PRIMARY KEY,
    tete TEXT NOT NULL,
    origine TEXT,
    base TEXT NOT NULL,
    etat TEXT NOT NULL CHECK (etat IN ('active', 'abandonnee'))
) STRICT;
CREATE TABLE active (
    unique_ligne INTEGER PRIMARY KEY CHECK (unique_ligne = 1),
    branche TEXT NOT NULL
) STRICT;
CREATE TABLE retablir (
    branche TEXT NOT NULL,
    rang INTEGER NOT NULL,
    sorte TEXT NOT NULL CHECK (sorte IN ('commit', 'entrer')),
    cible TEXT NOT NULL,
    tete TEXT,
    PRIMARY KEY (branche, rang)
) STRICT;
CREATE TABLE identites (
    cle TEXT PRIMARY KEY,
    ligne TEXT NOT NULL
) STRICT, WITHOUT ROWID;
CREATE TABLE commits (
    id TEXT PRIMARY KEY,
    parent1 TEXT,
    parent2 TEXT,
    motif TEXT NOT NULL,
    profondeur INTEGER NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('reference', 'difference'))
) STRICT;
CREATE TABLE lignes_reference (
    commit_id TEXT NOT NULL,
    cle TEXT NOT NULL,
    ligne TEXT NOT NULL,
    PRIMARY KEY (commit_id, cle)
) STRICT, WITHOUT ROWID;
CREATE TABLE lignes_difference (
    commit_id TEXT NOT NULL,
    rang INTEGER NOT NULL,
    sens TEXT NOT NULL CHECK (sens IN ('-', '+')),
    cle TEXT NOT NULL,
    ligne TEXT NOT NULL,
    PRIMARY KEY (commit_id, rang)
) STRICT, WITHOUT ROWID;
CREATE TABLE etat_courant (
    branche TEXT NOT NULL,
    cle TEXT NOT NULL,
    ligne TEXT NOT NULL,
    PRIMARY KEY (branche, cle)
) STRICT, WITHOUT ROWID;
CREATE INDEX etat_courant_cle ON etat_courant (cle);
CREATE INDEX branches_tete ON branches (tete);
CREATE TABLE operations (
    op TEXT PRIMARY KEY,
    bloc TEXT NOT NULL
) STRICT;
";

/// Erreur du stockage.
#[derive(Debug)]
pub enum StoreError {
    /// Le fichier existe déjà : `create` ne l'écrase jamais.
    AlreadyExists,
    /// Commande refusée par le domaine ; rien n'a été écrit.
    Domain(Error),
    /// Texte illisible au regard du format (import, vérification) ; rien n'est chargé.
    Decode(DecodeError),
    /// Schéma, version ou contenu non conformes ; le dépôt n'est pas chargé.
    Schema(String),
    /// L'historique n'a pu être lu ou écrit : ligne manquante ou illisible, différence
    /// incohérente, erreur SQLite pendant une lecture ou une écriture. Rien n'a été modifié.
    Historique(String),
    /// Erreur de SQLite ; en écriture, la transaction est annulée et le dépôt reste inchangé.
    Sqlite(rusqlite::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => f.write_str("le fichier existe déjà"),
            Self::Domain(e) => write!(f, "commande refusée : {e}"),
            Self::Decode(e) => write!(f, "contenu illisible : {e}"),
            Self::Schema(m) => write!(f, "base non conforme : {m}"),
            Self::Historique(m) => write!(f, "historique illisible ou non écrit : {m}"),
            Self::Sqlite(e) => write!(f, "erreur SQLite : {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

/// Erreur du domaine rendue par le stockage : un échec de l'historique n'est pas un refus.
fn from_domain(error: Error) -> StoreError {
    match error {
        Error::Historique(message) => StoreError::Historique(message),
        other => StoreError::Domain(other),
    }
}

fn fault(message: impl fmt::Display) -> Error {
    Error::Historique(message.to_string())
}

/// Identifiant relu dans la base, en forme canonique.
fn parse<T>(text: &str, make: fn([u8; 16]) -> T) -> Result<T, Error> {
    NodeId::parse(text)
        .map(|id| make(id.to_bytes()))
        .ok_or_else(|| fault(format!("identifiant invalide : {text}")))
}

/// Lignes d'une requête, comptées dans `read`.
fn query<T, P: Params>(
    connection: &Connection,
    read: &Cell<u64>,
    sql: &str,
    params: P,
    map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, Error> {
    let mut statement = connection.prepare_cached(sql).map_err(fault)?;
    let rows = statement.query_map(params, map).map_err(fault)?;
    let mut found = Vec::new();
    for row in rows {
        found.push(row.map_err(fault)?);
    }
    read.set(read.get() + u64::try_from(found.len()).unwrap_or(u64::MAX));
    Ok(found)
}

/// Première ligne d'une requête, s'il y en a une.
fn first<T, P: Params>(
    connection: &Connection,
    read: &Cell<u64>,
    sql: &str,
    params: P,
    map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Option<T>, Error> {
    Ok(query(connection, read, sql, params, map)?
        .into_iter()
        .next())
}

/// Lignes de l'état courant d'une branche, par clé.
fn current_lines(
    connection: &Connection,
    read: &Cell<u64>,
    branch: BranchId,
) -> Result<BTreeMap<String, String>, Error> {
    Ok(query(
        connection,
        read,
        "SELECT cle, ligne FROM etat_courant WHERE branche = ?1 ORDER BY cle",
        [branch.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?
    .into_iter()
    .collect())
}

/// Applique à des lignes la différence d'un commit : chaque ligne retirée doit être exactement
/// présente, chaque ligne ajoutée absente ; tout écart est refusé, jamais ignoré.
fn apply_difference(
    connection: &Connection,
    read: &Cell<u64>,
    lines: &mut BTreeMap<String, String>,
    commit: CommitId,
) -> Result<(), Error> {
    let rows: Vec<(String, String, String)> = query(
        connection,
        read,
        "SELECT sens, cle, ligne FROM lignes_difference WHERE commit_id = ?1 ORDER BY rang",
        [commit.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    for (sign, key, line) in rows {
        let consistent = match sign.as_str() {
            "-" => lines.remove(&key).as_ref() == Some(&line),
            _ => lines.insert(key, line).is_none(),
        };
        if !consistent {
            return Err(fault(format!("différence incohérente au commit {commit}")));
        }
    }
    Ok(())
}

/// Lignes de l'état d'un commit, par clé, non encore validées : l'état courant d'une branche
/// tenue (non abandonnée) dont il est la tête, sinon l'état de référence le plus proche le long
/// des premiers parents, suivi d'au plus 99 différences.
fn commit_lines(
    connection: &Connection,
    read: &Cell<u64>,
    commit: CommitId,
) -> Result<BTreeMap<String, String>, Error> {
    // L'état courant d'une branche abandonnée n'est pas tenu (VERSION2.md, section 3).
    let head_of = first(
        connection,
        read,
        "SELECT id FROM branches WHERE tete = ?1 AND etat = 'active' ORDER BY id LIMIT 1",
        [commit.to_string()],
        |r| r.get::<_, String>(0),
    )?;
    if let Some(branch) = head_of {
        return current_lines(connection, read, parse(&branch, BranchId::from_bytes)?);
    }
    let mut chain = Vec::new();
    let mut current = commit;
    let reference = loop {
        let row: Option<(Option<String>, String)> = first(
            connection,
            read,
            "SELECT parent1, nature FROM commits WHERE id = ?1",
            [current.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let Some((parent, nature)) = row else {
            return Err(if chain.is_empty() {
                Error::MissingCommit(commit)
            } else {
                fault(format!("parent {current} absent"))
            });
        };
        if nature == "reference" {
            break current;
        }
        chain.push(current);
        if chain.len() >= 100 {
            return Err(fault(format!(
                "plus de 99 différences avant un état de référence depuis {commit}"
            )));
        }
        let parent = parent.ok_or_else(|| fault(format!("différence sans parent : {current}")))?;
        current = parse(&parent, CommitId::from_bytes)?;
    };
    let mut lines: BTreeMap<String, String> = query(
        connection,
        read,
        "SELECT cle, ligne FROM lignes_reference WHERE commit_id = ?1 ORDER BY cle",
        [reference.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?
    .into_iter()
    .collect();
    for step in chain.iter().rev() {
        apply_difference(connection, read, &mut lines, *step)?;
    }
    Ok(lines)
}

fn decode_lines<'a>(
    lines: impl IntoIterator<Item = &'a String>,
    commit: CommitId,
) -> Result<Arc<Snapshot>, Error> {
    state_from_lines(lines.into_iter().map(String::as_str))
        .map(Arc::new)
        .map_err(|e| fault(format!("état du commit {commit} illisible : {e}")))
}

/// Historique d'un dépôt dans une base SQLite au schéma 2. Il compte les lignes qu'il lit, pour
/// vérifier que l'usage ordinaire ne dépend pas de la longueur de l'historique.
#[derive(Debug)]
pub struct SqliteHistory {
    connection: Connection,
    read: Cell<u64>,
}

impl SqliteHistory {
    /// Nombre de lignes lues depuis l'ouverture.
    #[must_use]
    pub fn lines_read(&self) -> u64 {
        self.read.get()
    }

    fn query<T, P: Params>(
        &self,
        sql: &str,
        params: P,
        map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, Error> {
        query(&self.connection, &self.read, sql, params, map)
    }

    fn first<T, P: Params>(
        &self,
        sql: &str,
        params: P,
        map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Option<T>, Error> {
        first(&self.connection, &self.read, sql, params, map)
    }
}

impl History for SqliteHistory {
    fn active(&self) -> Result<BranchId, Error> {
        let branch = self
            .first("SELECT branche FROM active", [], |r| r.get::<_, String>(0))?
            .ok_or_else(|| fault("branche active absente"))?;
        parse(&branch, BranchId::from_bytes)
    }

    fn branches(&self) -> Result<BTreeMap<BranchId, Branch>, Error> {
        let rows: Vec<(String, String, Option<String>, String, String)> = self.query(
            "SELECT id, tete, origine, base, etat FROM branches ORDER BY id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;
        let mut branches = BTreeMap::new();
        for (id, head, origin, base, state) in rows {
            branches.insert(
                parse(&id, BranchId::from_bytes)?,
                Branch {
                    head: parse(&head, CommitId::from_bytes)?,
                    redo: Vec::new(),
                    origin: origin
                        .map(|o| parse(&o, BranchId::from_bytes))
                        .transpose()?,
                    base: parse(&base, CommitId::from_bytes)?,
                    abandoned: state == "abandonnee",
                },
            );
        }
        let entries: Vec<(String, i64, String, String, Option<String>)> = self.query(
            "SELECT branche, rang, sorte, cible, tete FROM retablir ORDER BY branche, rang",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;
        for (branch, rank, kind, target, head) in entries {
            let entry = match (kind.as_str(), head) {
                ("commit", None) => Redo::Commit(parse(&target, CommitId::from_bytes)?),
                ("entrer", Some(head)) => Redo::Enter {
                    branch: parse(&target, BranchId::from_bytes)?,
                    head: parse(&head, CommitId::from_bytes)?,
                },
                _ => return Err(fault(format!("entrée de rétablissement invalide : {kind}"))),
            };
            let stack = &mut branches
                .get_mut(&parse(&branch, BranchId::from_bytes)?)
                .ok_or_else(|| fault(format!("rétablissement d'une branche inconnue : {branch}")))?
                .redo;
            if usize::try_from(rank).ok() != Some(stack.len()) {
                return Err(fault(format!(
                    "pile de rétablissement discontinue : {branch}"
                )));
            }
            stack.push(entry);
        }
        Ok(branches)
    }

    fn commit_count(&self) -> Result<usize, Error> {
        let count = self
            .first("SELECT count(*) FROM commits", [], |r| r.get::<_, i64>(0))?
            .unwrap_or(0);
        usize::try_from(count).map_err(fault)
    }

    fn state(&self, commit: CommitId) -> Result<Arc<Snapshot>, Error> {
        let lines = commit_lines(&self.connection, &self.read, commit)?;
        decode_lines(lines.values(), commit)
    }

    fn commit(&self, commit: CommitId) -> Result<Option<CommitInfo>, Error> {
        let row: Option<(Option<String>, Option<String>, String)> = self.first(
            "SELECT parent1, parent2, motif FROM commits WHERE id = ?1",
            [commit.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        row.map(|(first, second, reason)| {
            Ok(CommitInfo {
                parents: first
                    .into_iter()
                    .chain(second)
                    .map(|p| parse(&p, CommitId::from_bytes))
                    .collect::<Result<_, _>>()?,
                reason,
            })
        })
        .transpose()
    }

    fn operation(&self, op: OpId) -> Result<Option<(Envelope, Outcome)>, Error> {
        let block = self.first(
            "SELECT bloc FROM operations WHERE op = ?1",
            [op.to_string()],
            |r| r.get::<_, String>(0),
        )?;
        block
            .map(|b| decode_operation_v2(&b).map_err(|e| fault(format!("opération {op} : {e}"))))
            .transpose()
    }

    fn is_used(&self, id: UsedId) -> Result<bool, Error> {
        Ok(self
            .first(
                "SELECT 1 FROM identites WHERE cle = ?1",
                [identity_key(id)],
                |r| r.get::<_, i64>(0),
            )?
            .is_some())
    }

    fn branches_containing(&self, node: NodeId) -> Result<Vec<BranchId>, Error> {
        let key = line_key(&format!("contenu {node}"))
            .ok_or_else(|| fault("clé de contenu introuvable"))?;
        self.query(
            "SELECT DISTINCT branche FROM etat_courant WHERE cle = ?1 ORDER BY branche",
            [key],
            |r| r.get::<_, String>(0),
        )?
        .iter()
        .map(|b| parse(b, BranchId::from_bytes))
        .collect()
    }

    fn apply(&mut self, writes: &Writes) -> Result<(), Error> {
        let read = &self.read;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(fault)?;
        write(&tx, read, writes)?;
        tx.commit().map_err(fault)
    }
}

/// Écritures d'une opération, dans une transaction ouverte : commit et ses lignes, branches,
/// pile de rétablissement, état courant des têtes déplacées, branche active, identité, journal.
fn write(connection: &Connection, read: &Cell<u64>, writes: &Writes) -> Result<(), Error> {
    let run = |sql: &str, params: &[&dyn rusqlite::ToSql]| -> Result<(), Error> {
        connection.execute(sql, params).map(|_| ()).map_err(fault)
    };
    let before = writes.envelope.expected.branch;
    let after_lines = state_lines(&writes.state);
    // Lignes de l'état courant de la branche active avant l'opération, lues une seule fois.
    let mut before_lines: Option<BTreeMap<String, String>> = None;
    if let Some(commit) = &writes.commit {
        let parent_depth = first(
            connection,
            read,
            "SELECT profondeur FROM commits WHERE id = ?1",
            [commit.parent.to_string()],
            |r| r.get::<_, i64>(0),
        )?
        .ok_or_else(|| fault(format!("parent {} absent", commit.parent)))?;
        let depth = usize::try_from(parent_depth).map_err(fault)? + 1;
        let reference = holds_reference(depth);
        let nature = if reference { "reference" } else { "difference" };
        run(
            "INSERT INTO commits (id, parent1, parent2, motif, profondeur, nature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                &commit.id.to_string(),
                &commit.parent.to_string(),
                &commit.merged.map(|m| m.to_string()),
                &commit.reason,
                &i64::try_from(depth).map_err(fault)?,
                &nature,
            ],
        )?;
        let lines = state_lines(&commit.state);
        if reference {
            for (key, line) in &lines {
                run(
                    "INSERT INTO lignes_reference (commit_id, cle, ligne) VALUES (?1, ?2, ?3)",
                    &[&commit.id.to_string(), key, line],
                )?;
            }
        } else {
            let parent_lines = current_lines(connection, read, before)?;
            let changes = state_difference(&parent_lines, &lines);
            let rows = changes
                .removed
                .iter()
                .map(|(k, l)| ("-", *k, *l))
                .chain(changes.added.iter().map(|(k, l)| ("+", *k, *l)));
            for (rank, (sign, key, line)) in (0_i64..).zip(rows) {
                run(
                    "INSERT INTO lignes_difference (commit_id, rang, sens, cle, ligne)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    &[&commit.id.to_string(), &rank, &sign, &key, &line],
                )?;
            }
            before_lines = Some(parent_lines);
        }
    }
    for (id, branch) in &writes.branches {
        let old: Option<(String, String)> = first(
            connection,
            read,
            "SELECT tete, etat FROM branches WHERE id = ?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let was_abandoned = old.as_ref().is_some_and(|(_, state)| state == "abandonnee");
        let old_head = old.map(|(head, _)| head);
        // Réactivation : l'état de la tête est reconstruit et validé tant que la branche est
        // encore abandonnée dans la base, donc depuis son historique (VERSION2.md, section 3).
        let rebuilt = if was_abandoned && !branch.abandoned {
            let lines = commit_lines(connection, read, branch.head)?;
            Some(state_lines(&*decode_lines(lines.values(), branch.head)?))
        } else {
            None
        };
        run(
            "INSERT INTO branches (id, tete, origine, base, etat) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET tete = excluded.tete, origine = excluded.origine,
                                            base = excluded.base, etat = excluded.etat",
            &[
                &id.to_string(),
                &branch.head.to_string(),
                &branch.origin.map(|o| o.to_string()),
                &branch.base.to_string(),
                &branch_state(branch),
            ],
        )?;
        write_redo(connection, *id, branch)?;
        if branch.abandoned {
            // Abandon : l'état courant n'est plus tenu ; la tête et l'historique restent.
            run(
                "DELETE FROM etat_courant WHERE branche = ?1",
                &[&id.to_string()],
            )?;
            continue;
        }
        if let Some(lines) = rebuilt {
            for (key, line) in &lines {
                run(
                    "INSERT INTO etat_courant (branche, cle, ligne) VALUES (?1, ?2, ?3)",
                    &[&id.to_string(), key, line],
                )?;
            }
            continue;
        }
        if old_head.as_deref() == Some(branch.head.to_string().as_str()) {
            continue;
        }
        // La tête a bougé ou la branche est nouvelle : son état courant devient l'état après
        // l'opération. Seule la branche active peut changer de tête.
        if *id != writes.active {
            return Err(fault(format!(
                "tête déplacée hors de la branche active : {id}"
            )));
        }
        let current = match (&old_head, &before_lines) {
            (None, _) => BTreeMap::new(),
            // Lignes du parent du commit, déjà lues : l'état courant de la branche active avant
            // l'opération. Un commit ne s'ajoute qu'à cette branche, ou à une branche neuve.
            (Some(_), Some(lines)) => {
                if *id != before {
                    return Err(fault(format!(
                        "commit publié sur {id}, hors de la branche de son parent {before}"
                    )));
                }
                lines.clone()
            }
            (Some(_), None) => current_lines(connection, read, *id)?,
        };
        let changes = state_difference(&current, &after_lines);
        for (key, _) in &changes.removed {
            run(
                "DELETE FROM etat_courant WHERE branche = ?1 AND cle = ?2",
                &[&id.to_string(), key],
            )?;
        }
        for (key, line) in &changes.added {
            run(
                "INSERT INTO etat_courant (branche, cle, ligne) VALUES (?1, ?2, ?3)",
                &[&id.to_string(), key, line],
            )?;
        }
    }
    run(
        "UPDATE active SET branche = ?1 WHERE unique_ligne = 1",
        &[&writes.active.to_string()],
    )?;
    for &id in &writes.identities {
        run(
            "INSERT INTO identites (cle, ligne) VALUES (?1, ?2)",
            &[&identity_key(id), &encode_identity(id)],
        )?;
    }
    run(
        "INSERT INTO operations (op, bloc) VALUES (?1, ?2)",
        &[
            &writes.envelope.op.to_string(),
            &encode_operation_v2(&writes.envelope, &writes.outcome),
        ],
    )
}

/// Valeur de la colonne `etat` d'une branche.
fn branch_state(branch: &Branch) -> &'static str {
    if branch.abandoned {
        "abandonnee"
    } else {
        "active"
    }
}

/// Remplace la pile de rétablissement d'une branche.
fn write_redo(connection: &Connection, id: BranchId, branch: &Branch) -> Result<(), Error> {
    connection
        .execute("DELETE FROM retablir WHERE branche = ?1", [id.to_string()])
        .map_err(fault)?;
    for (rank, entry) in (0_i64..).zip(&branch.redo) {
        let (kind, target, head) = match entry {
            Redo::Commit(commit) => ("commit", commit.to_string(), None),
            Redo::Enter { branch, head } => ("entrer", branch.to_string(), Some(head.to_string())),
        };
        connection
            .execute(
                "INSERT INTO retablir (branche, rang, sorte, cible, tete) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id.to_string(), rank, kind, target, head],
            )
            .map_err(fault)?;
    }
    Ok(())
}

/// Écrit un dépôt en mémoire complet dans une base neuve, dans une transaction ouverte.
fn write_repository(connection: &Connection, repo: &Repository) -> Result<(), Error> {
    let run = |sql: &str, params: &[&dyn rusqlite::ToSql]| -> Result<(), Error> {
        connection.execute(sql, params).map(|_| ()).map_err(fault)
    };
    run(
        "INSERT INTO meta (cle, valeur) VALUES ('schema', ?1), ('format', ?2)",
        &[&SCHEMA_VERSION, &REPOSITORY_HEADER_V2],
    )?;
    let branches = repo.history().branches()?;
    for (id, branch) in &branches {
        run(
            "INSERT INTO branches (id, tete, origine, base, etat) VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                &id.to_string(),
                &branch.head.to_string(),
                &branch.origin.map(|o| o.to_string()),
                &branch.base.to_string(),
                &branch_state(branch),
            ],
        )?;
        write_redo(connection, *id, branch)?;
        // Aucun état courant n'est tenu pour une branche abandonnée.
        if branch.abandoned {
            continue;
        }
        for (key, line) in state_lines(&*repo.snapshot_at(branch.head)?) {
            run(
                "INSERT INTO etat_courant (branche, cle, ligne) VALUES (?1, ?2, ?3)",
                &[&id.to_string(), &key, &line],
            )?;
        }
    }
    run(
        "INSERT INTO active (unique_ligne, branche) VALUES (1, ?1)",
        &[&repo.active_branch().to_string()],
    )?;
    for id in repo.history().used_ids() {
        run(
            "INSERT INTO identites (cle, ligne) VALUES (?1, ?2)",
            &[&identity_key(id), &encode_identity(id)],
        )?;
    }
    for record in commit_records(repo) {
        let nature = match record.content {
            CommitContent::Reference(_) => "reference",
            CommitContent::Difference { .. } => "difference",
        };
        run(
            "INSERT INTO commits (id, parent1, parent2, motif, profondeur, nature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                &record.id.to_string(),
                &record.parent.map(|p| p.to_string()),
                &record.merged.map(|m| m.to_string()),
                &record.reason,
                &i64::try_from(record.depth).map_err(fault)?,
                &nature,
            ],
        )?;
        match &record.content {
            CommitContent::Reference(lines) => {
                for (key, line) in lines {
                    run(
                        "INSERT INTO lignes_reference (commit_id, cle, ligne) VALUES (?1, ?2, ?3)",
                        &[&record.id.to_string(), key, line],
                    )?;
                }
            }
            CommitContent::Difference { removed, added } => {
                let rows = removed
                    .iter()
                    .map(|(k, l)| ("-", k, l))
                    .chain(added.iter().map(|(k, l)| ("+", k, l)));
                for (rank, (sign, key, line)) in (0_i64..).zip(rows) {
                    run(
                        "INSERT INTO lignes_difference (commit_id, rang, sens, cle, ligne)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        &[&record.id.to_string(), &rank, &sign, key, line],
                    )?;
                }
            }
        }
    }
    for (envelope, outcome) in repo.journal() {
        run(
            "INSERT INTO operations (op, bloc) VALUES (?1, ?2)",
            &[
                &envelope.op.to_string(),
                &encode_operation_v2(envelope, outcome),
            ],
        )?;
    }
    Ok(())
}

/// Dépôt dont l'historique est une base SQLite.
pub type StoredRepository = Repository<SqliteHistory>;

/// Dépôt adossé à un fichier SQLite au schéma 2.
#[derive(Debug)]
pub struct Store {
    repository: StoredRepository,
}

fn configure(connection: &Connection) -> Result<(), StoreError> {
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(())
}

/// Retire une base et ses fichiers annexes de SQLite.
fn remove_base(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        // Retrait au mieux : un fichier absent n'est pas une erreur.
        let _ = std::fs::remove_file(name);
    }
}

/// Lit le dépôt d'une base de schéma 1 (tables `meta` et `fragments`), avec les règles de
/// `v0.5.0-alpha.1` : catégories connues, texte rassemblé lu par `decode_repository`, fragments
/// rangés sous leurs clés canoniques.
fn read_schema_1(connection: &Connection) -> Result<Repository, StoreError> {
    let mut stored = Fragments::default();
    for category in Category::ALL {
        stored.parts.insert(category, BTreeMap::new());
    }
    let mut statement = connection.prepare("SELECT categorie, cle, texte FROM fragments")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (word, key, text) = row?;
        let category = Category::from_word(&word)
            .ok_or_else(|| StoreError::Schema(format!("catégorie « {word} » inconnue")))?;
        stored.parts.entry(category).or_default().insert(key, text);
    }
    let repository = decode_repository(&stored.assemble()).map_err(StoreError::Decode)?;
    if Fragments::of(&repository) != stored {
        return Err(StoreError::Schema(
            "fragments rangés sous des clés non canoniques".to_owned(),
        ));
    }
    Ok(repository)
}

/// Métadonnée de la base ; une ligne absente est une base non conforme, une erreur de SQLite
/// (fichier étranger, table absente) reste une erreur de SQLite.
fn meta(connection: &Connection, key: &str) -> Result<String, StoreError> {
    connection
        .query_row(
            "SELECT valeur FROM meta WHERE cle = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| StoreError::Schema(format!("métadonnée « {key} » absente")))
}

impl Store {
    /// Crée un dépôt neuf dans un fichier qui ne doit pas exister.
    ///
    /// # Errors
    /// [`StoreError::AlreadyExists`] si le fichier existe ; erreur SQLite sinon.
    pub fn create(path: &Path) -> Result<Self, StoreError> {
        Self::create_from(path, &Repository::new())
    }

    /// Crée une base à partir du texte canonique d'un dépôt complet, en version 1 (FORMAT.md,
    /// section 12) ou 2 (section 13) selon son en-tête. Le texte est lu et validé en entier avant
    /// toute création : s'il est refusé, aucun fichier n'est créé.
    ///
    /// # Errors
    /// [`StoreError::Decode`] pour un texte invalide ; [`StoreError::AlreadyExists`] si le
    /// fichier existe ; erreur SQLite sinon.
    pub fn import(path: &Path, text: &str) -> Result<Self, StoreError> {
        let repository = decode_repository_any(text).map_err(StoreError::Decode)?;
        Self::create_from(path, &repository)
    }

    /// Écrit un dépôt en mémoire dans une base neuve, puis vérifie que la base relue redonne
    /// exactement son texte canonique. Au moindre échec, le fichier est retiré.
    fn create_from(path: &Path, repository: &Repository) -> Result<Self, StoreError> {
        // Limite connue : entre ce contrôle et l'ouverture, un autre processus pourrait créer le
        // fichier. L'usage est mono-utilisateur ; aucune garantie n'est revendiquée au-delà.
        if path.exists() {
            return Err(StoreError::AlreadyExists);
        }
        let created = Self::write_new(path, repository);
        if created.is_err() {
            remove_base(path);
        }
        created
    }

    fn write_new(path: &Path, repository: &Repository) -> Result<Self, StoreError> {
        let mut connection = Connection::open(path)?;
        configure(&connection)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA)?;
        write_repository(&tx, repository).map_err(from_domain)?;
        tx.commit()?;
        drop(connection);
        let store = Self::open(path)?;
        if store.text()? != encode_repository_v2(repository) {
            return Err(StoreError::Schema(
                "la base écrite ne redonne pas le dépôt".to_owned(),
            ));
        }
        Ok(store)
    }

    /// Version du schéma d'une base existante, lue sans rien écrire ni créer.
    ///
    /// # Errors
    /// Fichier absent ou étranger, métadonnée absente.
    pub fn schema(path: &Path) -> Result<String, StoreError> {
        // Lecture-écriture sans création ni écriture : en lecture seule, SQLite laisserait des
        // fichiers -wal et -shm à côté d'une base en mode WAL.
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        meta(&connection, "schema")
    }

    /// Migre une base de schéma 1 vers une base neuve de schéma 2 (ETAT-HISTORIQUE.md, sections
    /// 7 et 16) : sauvegarde de la source (`VACUUM INTO`), lecture complète avec les règles de
    /// `v0.5.0-alpha.1`, écriture de `target` par le chemin de l'import, puis vérification
    /// complète de `target` (D21). La source n'est jamais écrite.
    ///
    /// # Errors
    /// [`StoreError::AlreadyExists`] si la sauvegarde ou la cible existent, [`StoreError::Schema`]
    /// pour un schéma ou un format autres que ceux du schéma 1 : rien n'est alors créé. Après la
    /// sauvegarde, une source illisible ([`StoreError::Decode`], [`StoreError::Schema`]) ou un
    /// écart de la cible : la cible est retirée, la sauvegarde conservée.
    pub fn migrate(source: &Path, backup: &Path, target: &Path) -> Result<Verified, StoreError> {
        if backup.exists() || target.exists() {
            return Err(StoreError::AlreadyExists);
        }
        let saved = backup
            .to_str()
            .ok_or_else(|| StoreError::Schema("chemin de sauvegarde non UTF-8".to_owned()))?;
        // Aucune écriture par cette connexion ; voir `schema` pour le mode d'ouverture. À sa
        // fermeture, SQLite intègre un éventuel journal laissé par un arrêt brutal, comme
        // l'ancienne version l'aurait fait à l'ouverture.
        let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        let schema = meta(&connection, "schema")?;
        if schema == SCHEMA_VERSION {
            return Err(StoreError::Schema(format!(
                "base déjà au schéma {SCHEMA_VERSION} : rien à migrer"
            )));
        }
        if schema != "1" {
            return Err(StoreError::Schema(format!("schéma {schema} inconnu")));
        }
        let format = meta(&connection, "format")?;
        if format != REPOSITORY_HEADER {
            return Err(StoreError::Schema(format!(
                "format « {format} » inconnu pour le schéma 1"
            )));
        }
        connection.execute("VACUUM INTO ?1", params![saved])?;
        let repository = read_schema_1(&connection)?;
        drop(connection);
        let store = Self::create_from(target, &repository)?;
        let verified = store.verify();
        if verified.is_err() {
            drop(store);
            remove_base(target);
        }
        verified
    }

    /// Ouvre un dépôt existant : schéma et format, branches, branche active et état courant de
    /// la branche active, lu et validé ; l'historique n'est lu qu'à la demande (D21).
    ///
    /// # Errors
    /// Schéma ou format inconnus (une base de schéma 1 doit d'abord être migrée), historique
    /// illisible, erreur SQLite.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        // Schéma et format vérifiés avant tout réglage : passer le journal en mode WAL réécrirait
        // l'en-tête d'une base qui doit être refusée intacte.
        let schema = meta(&connection, "schema")?;
        if schema == "1" {
            return Err(StoreError::Schema(
                "base au schéma 1 : lancer « memoire migrer » pour la convertir \
                 (une sauvegarde est faite avant)"
                    .to_owned(),
            ));
        }
        if schema != SCHEMA_VERSION {
            return Err(StoreError::Schema(format!("schéma {schema} inconnu")));
        }
        let format = meta(&connection, "format")?;
        if format != REPOSITORY_HEADER_V2 {
            return Err(StoreError::Schema(format!("format « {format} » inconnu")));
        }
        configure(&connection)?;
        let history = SqliteHistory {
            connection,
            read: Cell::new(0),
        };
        let repository = Repository::open(history).map_err(|e| match e {
            Error::Historique(message) => StoreError::Historique(message),
            other => StoreError::Historique(other.to_string()),
        })?;
        Ok(Self { repository })
    }

    /// Écrit une copie cohérente de la base dans un nouveau fichier (R9), par SQLite lui-même
    /// (`VACUUM INTO`) : la copie reflète un état validé, même pendant une écriture.
    ///
    /// # Errors
    /// [`StoreError::AlreadyExists`] si la cible existe ; erreur SQLite sinon.
    pub fn backup(&self, path: &Path) -> Result<(), StoreError> {
        if path.exists() {
            return Err(StoreError::AlreadyExists);
        }
        let cible = path
            .to_str()
            .ok_or_else(|| StoreError::Schema("chemin de sauvegarde non UTF-8".to_owned()))?;
        self.history()
            .connection
            .execute("VACUUM INTO ?1", params![cible])?;
        Ok(())
    }

    /// Dépôt courant, tel qu'il est écrit dans le fichier.
    #[must_use]
    pub fn repository(&self) -> &StoredRepository {
        &self.repository
    }

    fn history(&self) -> &SqliteHistory {
        self.repository.history()
    }

    /// Nombre de lignes lues dans la base depuis l'ouverture.
    #[must_use]
    pub fn lines_read(&self) -> u64 {
        self.history().lines_read()
    }

    /// Exécute une commande et la rend durable, en une transaction, avant de la publier en
    /// mémoire.
    ///
    /// # Errors
    /// [`StoreError::Domain`] si le domaine refuse ; [`StoreError::Historique`] si l'historique
    /// ne peut être lu ou écrit. Dans les deux cas, ni le fichier ni le dépôt en mémoire ne
    /// changent.
    pub fn execute(&mut self, envelope: Envelope) -> Result<Outcome, StoreError> {
        self.repository.execute(envelope).map_err(from_domain)
    }

    /// Texte canonique v2 complet du dépôt, rassemblé depuis les tables, sans vérification.
    ///
    /// # Errors
    /// [`StoreError::Historique`] si une table est illisible.
    pub fn text(&self) -> Result<String, StoreError> {
        let history = self.history();
        let mut out = String::new();
        out.push_str(REPOSITORY_HEADER_V2);
        out.push('\n');
        out.push_str(&encode_active(history.active().map_err(from_domain)?));
        for (id, branch) in history.branches().map_err(from_domain)? {
            out.push_str(&encode_branch_v2(id, &branch));
        }
        for line in history
            .query("SELECT ligne FROM identites ORDER BY cle", [], |r| {
                r.get::<_, String>(0)
            })
            .map_err(from_domain)?
        {
            out.push_str(&line);
        }
        type Row = (String, Option<String>, Option<String>, String, String);
        let commits: Vec<Row> = history
            .query(
                "SELECT id, parent1, parent2, motif, nature FROM commits ORDER BY id",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(from_domain)?;
        for (id, first, second, reason, nature) in commits {
            let commit = parse(&id, CommitId::from_bytes).map_err(from_domain)?;
            let parents: Vec<CommitId> = first
                .into_iter()
                .chain(second)
                .map(|p| parse(&p, CommitId::from_bytes))
                .collect::<Result<_, _>>()
                .map_err(from_domain)?;
            let block = if nature == "reference" {
                let lines: Vec<String> = history
                    .query(
                        "SELECT ligne FROM lignes_reference WHERE commit_id = ?1 ORDER BY cle",
                        [&id],
                        |r| r.get(0),
                    )
                    .map_err(from_domain)?;
                let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
                encode_commit_parents_v2(commit, &parents, &reason, CommitBlock::Reference(&lines))
            } else {
                let rows: Vec<(String, String)> = history
                    .query(
                        "SELECT sens, ligne FROM lignes_difference WHERE commit_id = ?1 ORDER BY rang",
                        [&id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .map_err(from_domain)?;
                let removed: Vec<&str> = rows
                    .iter()
                    .filter(|(s, _)| s == "-")
                    .map(|(_, l)| l.as_str())
                    .collect();
                let added: Vec<&str> = rows
                    .iter()
                    .filter(|(s, _)| s == "+")
                    .map(|(_, l)| l.as_str())
                    .collect();
                encode_commit_parents_v2(
                    commit,
                    &parents,
                    &reason,
                    CommitBlock::Difference {
                        removed: &removed,
                        added: &added,
                    },
                )
            };
            out.push_str(&block);
        }
        for block in history
            .query("SELECT bloc FROM operations ORDER BY op", [], |r| {
                r.get::<_, String>(0)
            })
            .map_err(from_domain)?
        {
            out.push_str(&block);
        }
        out.push_str("fin\n");
        Ok(out)
    }

    /// Vérification complète (D21) : le texte rassemblé est relu en entier par le domaine,
    /// chaque état reconstruit et validé, et l'état courant de chaque branche comparé à celui
    /// que redonne son historique. Rend le texte vérifié.
    fn verified_text(&self) -> Result<(String, Verified), StoreError> {
        let text = self.text()?;
        let verified = verify_repository_v2(&text).map_err(StoreError::Decode)?;
        let history = self.history();
        let branches = history.branches().map_err(from_domain)?;
        for (branch, head) in &verified.heads {
            let lines =
                current_lines(&history.connection, &history.read, *branch).map_err(from_domain)?;
            // Une branche abandonnée ne tient aucun état courant (VERSION2.md, section 3).
            let abandoned = branches.get(branch).is_some_and(|b| b.abandoned);
            let mut current = String::new();
            current.push_str(SNAPSHOT_HEADER_V2);
            current.push('\n');
            for line in lines.values() {
                current.push_str(line);
                current.push('\n');
            }
            current.push_str("fin\n");
            let consistent = if abandoned {
                lines.is_empty()
            } else {
                current == *head
            };
            if !consistent {
                return Err(StoreError::Schema(format!(
                    "état courant de la branche {branch} différent de son historique"
                )));
            }
        }
        Ok((text, verified))
    }

    /// Vérification complète de la base (commande `memoire verifier`).
    ///
    /// # Errors
    /// [`StoreError::Decode`] si le texte rassemblé est refusé ; [`StoreError::Schema`] si l'état
    /// courant d'une branche diverge de son historique ; [`StoreError::Historique`] si une table
    /// est illisible.
    pub fn verify(&self) -> Result<Verified, StoreError> {
        self.verified_text().map(|(_, verified)| verified)
    }

    /// Texte canonique v2 complet, après vérification complète (export, D21).
    ///
    /// # Errors
    /// Les mêmes que [`Store::verify`].
    pub fn export(&self) -> Result<String, StoreError> {
        self.verified_text().map(|(text, _)| text)
    }
}
