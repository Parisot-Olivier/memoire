//! Couche service (SERVICE.md) : seule interface de la future ligne de commande.
//!
//! Le service attribue les identités (D3), construit les enveloppes avec l'état attendu et le
//! motif (D12), et passe par le stockage pour toute écriture. Une intention est d'abord préparée
//! (identités attribuées, aperçu calculé, rien n'est écrit), puis appliquée ; si le dépôt a changé
//! entre-temps, le domaine refuse l'enveloppe périmée et rien n'est écrit.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use memoire_domain::{
    AnchorId, Command, EdgeId, Envelope, Expected, NodeId, OpId, Outcome, Preview, ScopeId,
    Verified,
};
use memoire_store::{Store, SCHEMA_VERSION};
pub use memoire_store::{StoreError, StoredRepository};

/// Résultat d'une migration de base (ETAT-HISTORIQUE.md, sections 7 et 16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Migration {
    /// Base déjà au schéma courant : rien n'a été fait, aucune sauvegarde écrite.
    AlreadyCurrent,
    /// Base convertie, vérifiée en entier, puis mise à la place de l'ancienne.
    Migrated(Verified),
}

/// Échec d'une migration ; la base d'origine n'a pas été modifiée.
#[derive(Debug)]
pub enum MigrationError {
    /// Refus ou erreur du stockage (schéma, lecture, écriture, vérification).
    Store(StoreError),
    /// Erreur d'entrée-sortie pendant la mise en place du nouveau fichier.
    Io(io::Error),
    /// La base est ouverte par un autre processus : elle n'est pas remplacée.
    InUse,
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(e) => e.fmt(f),
            Self::Io(e) => write!(f, "mise en place de la base migrée impossible : {e}"),
            Self::InUse => f.write_str(
                "base ouverte par un autre processus : la fermer, puis relancer la migration \
                 (la base n'a pas été modifiée)",
            ),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<StoreError> for MigrationError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<io::Error> for MigrationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Source d'identités de 128 bits ; ne rend jamais l'identifiant nul.
pub trait IdSource {
    /// Identité suivante.
    fn next(&mut self) -> [u8; 16];
}

/// Une source d'identités en boîte reste une source d'identités (interfaces qui choisissent leur
/// source à l'exécution, comme la ligne de commande).
impl<T: IdSource + ?Sized> IdSource for Box<T> {
    fn next(&mut self) -> [u8; 16] {
        (**self).next()
    }
}

/// UUID version 4, aléatoires : usage réel.
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomIds;

impl IdSource for RandomIds {
    fn next(&mut self) -> [u8; 16] {
        uuid::Uuid::new_v4().into_bytes()
    }
}

/// Suite déterministe pour les tests : un préfixe sur huit octets, puis un compteur qui commence à 1.
#[derive(Clone, Copy, Debug)]
pub struct SequenceIds {
    prefix: u64,
    counter: u64,
}

impl SequenceIds {
    /// Suite de préfixe donné.
    #[must_use]
    pub fn new(prefix: u64) -> Self {
        Self { prefix, counter: 0 }
    }
}

impl IdSource for SequenceIds {
    fn next(&mut self) -> [u8; 16] {
        self.counter += 1;
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.prefix.to_be_bytes());
        bytes[8..].copy_from_slice(&self.counter.to_be_bytes());
        bytes
    }
}

/// Identités nouvelles attribuées pendant la préparation d'une commande.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Assigned {
    /// Notes nouvelles, dans l'ordre d'attribution.
    pub nodes: Vec<NodeId>,
    /// Relations nouvelles.
    pub edges: Vec<EdgeId>,
    /// Contextes nouveaux.
    pub scopes: Vec<ScopeId>,
    /// Ancrages nouveaux.
    pub anchors: Vec<AnchorId>,
}

/// Fournisseur d'identités prêté à la construction d'une commande ; garde trace de ce qu'il donne.
pub struct Fresh<'a> {
    ids: &'a mut dyn IdSource,
    assigned: Assigned,
}

impl Fresh<'_> {
    /// Identité d'une note nouvelle.
    pub fn node(&mut self) -> NodeId {
        let id = NodeId::from_bytes(self.ids.next());
        self.assigned.nodes.push(id);
        id
    }

    /// Identité d'une relation nouvelle.
    pub fn edge(&mut self) -> EdgeId {
        let id = EdgeId::from_bytes(self.ids.next());
        self.assigned.edges.push(id);
        id
    }

    /// Identité d'un contexte nouveau.
    pub fn scope(&mut self) -> ScopeId {
        let id = ScopeId::from_bytes(self.ids.next());
        self.assigned.scopes.push(id);
        id
    }

    /// Identité d'un ancrage nouveau.
    pub fn anchor(&mut self) -> AnchorId {
        let id = AnchorId::from_bytes(self.ids.next());
        self.assigned.anchors.push(id);
        id
    }
}

/// Commande préparée : identités attribuées, état attendu figé, aperçu calculé, rien d'écrit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    /// Identifiant de l'opération.
    pub op: OpId,
    /// État sur lequel l'aperçu a été calculé ; l'application l'exige encore.
    pub expected: Expected,
    /// Commande construite.
    pub command: Command,
    /// Effets et bifurcation annoncés.
    pub preview: Preview,
    /// Identités attribuées.
    pub assigned: Assigned,
}

/// Commande appliquée : résultat, et enveloppe exacte pour une répétition éventuelle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    /// Résultat publié.
    pub outcome: Outcome,
    /// Enveloppe exécutée ; la rejouer rend le même résultat (INV-RETRY-01).
    pub envelope: Envelope,
}

/// Service adossé à un fichier de dépôt.
pub struct Service<I: IdSource> {
    store: Store,
    ids: I,
}

impl<I: IdSource> Service<I> {
    /// Crée un dépôt neuf dans un fichier qui ne doit pas exister.
    ///
    /// # Errors
    /// Erreur du stockage.
    pub fn create(path: &Path, ids: I) -> Result<Self, StoreError> {
        Ok(Self {
            store: Store::create(path)?,
            ids,
        })
    }

    /// Ouvre un dépôt existant.
    ///
    /// # Errors
    /// Erreur du stockage.
    pub fn open(path: &Path, ids: I) -> Result<Self, StoreError> {
        Ok(Self {
            store: Store::open(path)?,
            ids,
        })
    }

    /// Crée une base à partir du texte d'un export ; le texte est validé en entier avant toute
    /// création de fichier (SERVICE.md, section 5).
    ///
    /// # Errors
    /// Texte invalide, base existante ou erreur SQLite.
    pub fn import(text: &str, path: &Path, ids: I) -> Result<Self, StoreError> {
        Ok(Self {
            store: Store::import(path, text)?,
            ids,
        })
    }

    /// Écrit le texte canonique du dépôt complet, au format version 2 (VERSION2.md, section 6),
    /// dans un fichier, de façon atomique : fichier
    /// temporaire du même répertoire, synchronisé, renommé à la place de la cible, puis
    /// synchronisation du répertoire. Un export interrompu laisse l'ancien fichier ou le nouveau.
    ///
    /// # Errors
    /// Erreur d'entrée-sortie ; le fichier temporaire est alors supprimé.
    pub fn export(&self, path: &Path) -> io::Result<()> {
        // Export vérifié en entier (D21) : un écart est refusé comme donnée invalide.
        let text = self
            .store
            .export()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let (directory, temporary) = temporary_path(path)?;
        let written = (|| {
            let mut file = File::create(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            sync_directory(&directory)
        })();
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        written
    }

    /// Migre une base de schéma 1 en place (ETAT-HISTORIQUE.md, sections 7 et 16) : sauvegarde
    /// dans `backup` (répertoire créé au besoin), conversion vers un fichier temporaire du même
    /// répertoire, vérification complète, puis renommage atomique sur la base et synchronisation
    /// du répertoire. Une base déjà au schéma courant n'est pas touchée.
    ///
    /// # Errors
    /// Base absente ou illisible, schéma inconnu, sauvegarde existante, écart de vérification,
    /// base ouverte par un autre processus, erreur d'entrée-sortie : la base d'origine reste
    /// intacte et le fichier temporaire est retiré.
    pub fn migrate(base: &Path, backup: &Path) -> Result<Migration, MigrationError> {
        if Store::schema(base)? == SCHEMA_VERSION {
            return Ok(Migration::AlreadyCurrent);
        }
        if let Some(directory) = backup.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(directory)?;
        }
        let (directory, temporary) = temporary_path(base)?;
        let verified = Store::migrate(base, backup, &temporary)?;
        let placed = (|| {
            // Toutes les connexions de la migration sont fermées : un journal encore présent
            // appartient à un autre processus, qui pourrait le rejouer sur le nouveau fichier.
            if side_file(base, "-wal").exists() {
                return Err(MigrationError::InUse);
            }
            File::open(&temporary)?.sync_all()?;
            fs::rename(&temporary, base)?;
            sync_directory(&directory)?;
            Ok(())
        })();
        if placed.is_err() {
            for suffix in ["", "-wal", "-shm"] {
                // Retrait au mieux du fichier temporaire : l'erreur rendue est celle du placement.
                let _ = fs::remove_file(side_file(&temporary, suffix));
            }
        }
        placed.map(|()| Migration::Migrated(verified))
    }

    /// Écrit une copie cohérente de la base dans un nouveau fichier (R9).
    ///
    /// # Errors
    /// Cible existante ou erreur du stockage.
    pub fn backup(&self, path: &Path) -> Result<(), StoreError> {
        self.store.backup(path)
    }

    /// Dépôt courant.
    #[must_use]
    pub fn repository(&self) -> &StoredRepository {
        self.store.repository()
    }

    /// Texte canonique v2 complet du dépôt, rassemblé depuis la base, sans vérification.
    ///
    /// # Errors
    /// [`StoreError::Historique`] si la base est illisible.
    pub fn text(&self) -> Result<String, StoreError> {
        self.store.text()
    }

    /// Vérification complète de la base (D21) : tout l'historique relu et validé, l'état courant
    /// de chaque branche confirmé.
    ///
    /// # Errors
    /// Écart trouvé ou base illisible.
    pub fn verify(&self) -> Result<Verified, StoreError> {
        self.store.verify()
    }

    /// Prépare une commande : la fonction reçoit un fournisseur d'identités nouvelles et rend la
    /// commande ; le service fige l'état attendu et calcule l'aperçu. Rien n'est écrit.
    ///
    /// # Errors
    /// [`StoreError::Domain`] si la commande est déjà refusée par le domaine.
    pub fn plan(
        &mut self,
        build: impl FnOnce(&mut Fresh<'_>) -> Command,
    ) -> Result<Planned, StoreError> {
        let mut fresh = Fresh {
            ids: &mut self.ids,
            assigned: Assigned::default(),
        };
        let command = build(&mut fresh);
        let assigned = fresh.assigned;
        let op = OpId::from_bytes(self.ids.next());
        let repository = self.store.repository();
        let preview = repository.preview(&command).map_err(|e| match e {
            memoire_domain::Error::Historique(message) => StoreError::Historique(message),
            other => StoreError::Domain(other),
        })?;
        Ok(Planned {
            op,
            expected: repository.expected(),
            command,
            preview,
            assigned,
        })
    }

    /// Applique une commande préparée avec son motif. Si le dépôt a changé depuis la préparation,
    /// le domaine la refuse comme périmée ; rien n'est écrit.
    ///
    /// # Errors
    /// Refus du domaine (motif manquant, enveloppe périmée…) ou erreur du stockage.
    pub fn apply(&mut self, planned: Planned, reason: &str) -> Result<Applied, StoreError> {
        let envelope = Envelope {
            op: planned.op,
            expected: planned.expected,
            reason: reason.to_owned(),
            command: planned.command,
        };
        let outcome = self.store.execute(envelope.clone())?;
        Ok(Applied { outcome, envelope })
    }

    /// Rejoue une enveloppe déjà appliquée, par exemple après une interruption : même résultat,
    /// aucun double effet (INV-RETRY-01).
    ///
    /// # Errors
    /// Refus du domaine ou erreur du stockage.
    pub fn retry(&mut self, envelope: Envelope) -> Result<Outcome, StoreError> {
        self.store.execute(envelope)
    }
}

/// Répertoire de la cible et fichier temporaire de l'export, dans ce même répertoire : le
/// renommage final n'est atomique qu'entre deux chemins d'un même répertoire.
fn temporary_path(path: &Path) -> io::Result<(PathBuf, PathBuf)> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chemin sans nom de fichier"))?;
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let temporary = directory.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    Ok((directory, temporary))
}

/// Fichier annexe de SQLite à côté d'une base (`-wal`, `-shm`), ou la base elle-même.
fn side_file(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Rend durable le renommage dans le répertoire (systèmes Unix ; ailleurs, sans objet).
fn sync_directory(directory: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(directory)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = directory;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Le fichier temporaire est toujours dans le répertoire de la cible ; un nom seul désigne le
    // répertoire courant ; un chemin sans nom de fichier est refusé.
    #[test]
    fn the_temporary_file_sits_next_to_its_target() {
        let pid = std::process::id();
        let (directory, temporary) = temporary_path(Path::new("/a/b/export.txt")).unwrap();
        assert_eq!(directory, PathBuf::from("/a/b"));
        assert_eq!(
            temporary,
            PathBuf::from(format!("/a/b/.export.txt.{pid}.tmp"))
        );
        let (directory, temporary) = temporary_path(Path::new("export.txt")).unwrap();
        assert_eq!(directory, PathBuf::from("."));
        assert_eq!(temporary, PathBuf::from(format!("./.export.txt.{pid}.tmp")));
        let refused = temporary_path(Path::new("/")).unwrap_err();
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
    }
}
