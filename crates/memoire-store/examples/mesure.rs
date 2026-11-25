//! Programme de mesure de la performance (ETAT-HISTORIQUE.md, section 10 ; incrément 5.2a).
//!
//! Hors contrat et hors mutation : il produit des chiffres, pas un verdict. Il construit en
//! mémoire un dépôt synthétique reproductible (graine fixée) : notes reliées en chaînes
//! (« dépend de ») et en étoiles (« classé dans » une catégorie), changements de statut,
//! bifurcations aussitôt quittées par annulation, quelques annulations et rétablissements. À
//! chaque palier d'opérations, le dépôt est importé dans une base neuve, puis mesuré :
//! - par la bibliothèque : ouverture de la base, exécution d'une commande ordinaire ;
//! - par l'exécutable, comme l'utilisateur : `statut`, `note creer`, `decision`, `annuler`,
//!   `basculer` vers une branche puis retour sur main.
//!
//! Usage (depuis la racine du dépôt, après `cargo build --release -p memoire-cli`) :
//!
//! ```text
//! cargo run --release -p memoire-store --example mesure -- \
//!     [--paliers 1000,2500,5000] [--notes 1500] [--graine 1] \
//!     [--repertoire target/mesure] [--memoire target/release/memoire] [--plafond-ouverture 120] \
//!     [--etat-constant]
//! ```
//!
//! Les bases sont écrites sur disque (`target/mesure` par défaut, ignoré par Git), jamais dans
//! un répertoire temporaire en mémoire. La mesure s'arrête au premier palier dont l'ouverture
//! dépasse le plafond, en secondes : l'ancienne version cesse alors d'être praticable.

use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::time::{Duration, Instant};

use memoire_domain::{
    encode_repository_v2, BranchId, Change, Command, Decision, EdgeId, Envelope, Implementation,
    Kind, NodeId, OpId, Relation, Repository, Verdict, DEFAULT_SCOPE, MAIN,
};
use memoire_store::Store;

type Resultat<T> = Result<T, Box<dyn StdError>>;

// ----- Paramètres -----

struct Parametres {
    paliers: Vec<usize>,
    notes: usize,
    graine: u64,
    repertoire: PathBuf,
    memoire: PathBuf,
    plafond_ouverture: f64,
    /// État constant : toutes les notes visées dès le premier palier, puis plus aucune note ni
    /// relation nouvelle ; seul l'historique s'allonge.
    etat_constant: bool,
}

fn parametres() -> Resultat<Parametres> {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut p = Parametres {
        paliers: vec![1000, 2500, 5000],
        notes: 1500,
        graine: 1,
        repertoire: racine.join("target/mesure"),
        memoire: racine.join("target/release/memoire"),
        plafond_ouverture: 120.0,
        etat_constant: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(option) = args.next() {
        if option == "--etat-constant" {
            p.etat_constant = true;
            continue;
        }
        let valeur = args
            .next()
            .ok_or_else(|| format!("valeur manquante après {option}"))?;
        match option.as_str() {
            "--paliers" => {
                p.paliers = valeur
                    .split(',')
                    .map(str::parse)
                    .collect::<Result<_, _>>()?;
                p.paliers.sort_unstable();
                p.paliers.dedup();
            }
            "--notes" => p.notes = valeur.parse()?,
            "--graine" => p.graine = valeur.parse()?,
            "--repertoire" => p.repertoire = valeur.into(),
            "--memoire" => p.memoire = valeur.into(),
            "--plafond-ouverture" => p.plafond_ouverture = valeur.parse()?,
            autre => return Err(format!("option inconnue : {autre}").into()),
        }
    }
    if p.paliers.is_empty() || p.paliers[0] == 0 {
        return Err("--paliers : au moins un palier non nul".into());
    }
    Ok(p)
}

// ----- Générateur reproductible -----

/// SplitMix64 : reproductible et sans dépendance.
struct Hasard(u64);

impl Hasard {
    fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn sous(&mut self, n: usize) -> usize {
        usize::try_from(self.suivant() % n as u64).unwrap_or(0)
    }
}

/// Dépôt synthétique en construction : il ne propose que des commandes valides, et compte les
/// commandes refusées (aucune n'est attendue, mais un refus n'interrompt pas la génération).
struct Generateur {
    hasard: Hasard,
    graine: u64,
    compteur: u64,
    depot: Repository,
    /// Notes affirmatives (raisonnement, implémentation), dans l'ordre de création.
    affirmations: Vec<NodeId>,
    categories: Vec<NodeId>,
    branches: Vec<BranchId>,
    refus: usize,
}

impl Generateur {
    fn new(graine: u64) -> Self {
        Self {
            hasard: Hasard(graine),
            graine,
            compteur: 0,
            depot: Repository::new(),
            affirmations: Vec::new(),
            categories: Vec::new(),
            branches: Vec::new(),
            refus: 0,
        }
    }

    /// Identifiant nouveau et non nul : préfixe, graine, compteur.
    fn identite(&mut self) -> [u8; 16] {
        self.compteur += 1;
        let mut octets = [0u8; 16];
        octets[0] = 0x6d;
        octets[1..8].copy_from_slice(&self.graine.to_be_bytes()[1..]);
        octets[8..].copy_from_slice(&self.compteur.to_be_bytes());
        octets
    }

    fn operations(&self) -> usize {
        self.depot.journal().count()
    }

    fn notes(&self) -> usize {
        self.affirmations.len() + self.categories.len()
    }

    fn executer(&mut self, command: Command) -> Option<memoire_domain::Outcome> {
        let envelope = Envelope {
            op: OpId::from_bytes(self.identite()),
            expected: self.depot.expected(),
            reason: "mesure".into(),
            command,
        };
        match self.depot.execute(envelope) {
            Ok(outcome) => Some(outcome),
            Err(_) => {
                self.refus += 1;
                None
            }
        }
    }

    fn creer(&mut self, kind: Kind) {
        let id = NodeId::from_bytes(self.identite());
        let rang = self.notes() + 1;
        let command = Command::Change(Change::CreateNode {
            scope: DEFAULT_SCOPE,
            id,
            kind,
            title: format!("note {rang}"),
            text: format!(
                "Texte de la note {rang} : une hypothèse de conception, sa justification et ses \
                 limites, en quelques phrases ordinaires pour un usage réel."
            ),
        });
        if self.executer(command).is_some() {
            if kind == Kind::Category {
                self.categories.push(id);
            } else {
                self.affirmations.push(id);
            }
        }
    }

    fn lier(&mut self, from: NodeId, to: NodeId, relation: Relation) {
        let id = EdgeId::from_bytes(self.identite());
        self.executer(Command::Change(Change::Link {
            scope: DEFAULT_SCOPE,
            id,
            from,
            to,
            relation,
        }));
    }

    /// Un pas du générateur : une commande proposée, acceptée sauf refus imprévu. Avec
    /// `figer`, une fois les notes visées atteintes, plus aucune note ni relation nouvelle.
    fn pas(&mut self, notes_visees: usize, figer: bool) {
        let operations = self.operations();
        // Toutes les 250 opérations (la première à la 50e), une bifurcation : réfutation d'une
        // note dont d'autres dépendent, sur main, aussitôt quittée par l'annulation de son
        // opération fondatrice.
        if operations % 250 == 49 && self.depot.active_branch() == MAIN {
            if let Some(&cible) = self.affirmations.first() {
                let refuter = Command::Change(Change::SetVerdict {
                    scope: DEFAULT_SCOPE,
                    node: cible,
                    verdict: Verdict::Refuted,
                });
                if let Some(outcome) = self.executer(refuter) {
                    if outcome.forked {
                        self.branches.push(outcome.branch);
                        self.executer(Command::Undo);
                    }
                }
                return;
            }
        }
        let peut_creer = self.notes() < notes_visees;
        let tirage = match self.hasard.sous(100) {
            // État figé : les créations et les liens deviennent des changements de statut.
            t if figer && !peut_creer && t < 60 => 60 + t % 36,
            t => t,
        };
        match tirage {
            0..=29 if peut_creer || self.affirmations.len() < 2 => {
                let kind = match self.hasard.sous(10) {
                    0 if self.categories.len() < 50 => Kind::Category,
                    1..=3 => Kind::Implementation,
                    _ => Kind::Reasoning,
                };
                self.creer(kind);
            }
            0..=59 if self.affirmations.len() >= 2 => {
                // Chaîne : la note la plus récente dépend d'une note plus ancienne (pas de
                // cycle) ; étoile : une note est classée dans une catégorie.
                let n = self.affirmations.len();
                let origine = self.affirmations[n - 1 - self.hasard.sous(n.min(20))];
                if tirage < 45 || self.categories.is_empty() {
                    let rang = self
                        .affirmations
                        .iter()
                        .position(|a| *a == origine)
                        .unwrap_or(0);
                    if rang > 0 {
                        let destination = self.affirmations[self.hasard.sous(rang)];
                        self.lier(origine, destination, Relation::DependsOn);
                    }
                } else {
                    let categorie = self.categories[self.hasard.sous(self.categories.len())];
                    self.lier(origine, categorie, Relation::ClassifiedIn);
                }
            }
            0..=95 if !self.affirmations.is_empty() => {
                let note = self.affirmations[self.hasard.sous(self.affirmations.len())];
                let change = if self.hasard.sous(2) == 0 {
                    let decision = [Decision::Proposed, Decision::Retained][self.hasard.sous(2)];
                    Change::SetDecision {
                        scope: DEFAULT_SCOPE,
                        node: note,
                        decision,
                    }
                } else {
                    let implementation = [
                        Implementation::NotStarted,
                        Implementation::InProgress,
                        Implementation::Implemented,
                    ][self.hasard.sous(3)];
                    Change::SetImplementation {
                        scope: DEFAULT_SCOPE,
                        node: note,
                        implementation,
                    }
                };
                self.executer(Command::Change(change));
            }
            _ => {
                // Annulation puis rétablissement : la pile de rétablissement est exercée.
                if self.executer(Command::Undo).is_some() {
                    self.executer(Command::Redo);
                }
            }
        }
    }

    fn jusqua(&mut self, operations: usize, notes_visees: usize, figer: bool) {
        while self.operations() < operations {
            self.pas(notes_visees, figer);
        }
    }
}

// ----- Mesures -----

fn secondes(duree: Duration) -> f64 {
    duree.as_secs_f64()
}

fn mediane(mut valeurs: Vec<f64>) -> f64 {
    valeurs.sort_by(f64::total_cmp);
    valeurs[valeurs.len() / 2]
}

fn taille_mo(base: &Path) -> f64 {
    ["", "-wal"]
        .iter()
        .filter_map(|suffixe| std::fs::metadata(format!("{}{suffixe}", base.display())).ok())
        .map(|m| m.len() as f64)
        .sum::<f64>()
        / 1e6
}

fn supprimer(base: &Path) {
    for suffixe in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffixe}", base.display()));
    }
}

/// Lance l'exécutable sur la base ; rend la durée et la sortie standard.
fn lancer(memoire: &Path, base: &Path, args: &[&str]) -> Resultat<(f64, String)> {
    let debut = Instant::now();
    let sortie = Process::new(memoire)
        .arg("--base")
        .arg(base)
        .args(args)
        .output()?;
    let duree = secondes(debut.elapsed());
    if !sortie.status.success() {
        return Err(format!(
            "memoire {args:?} : code {:?}, {}",
            sortie.status.code(),
            String::from_utf8_lossy(&sortie.stderr)
        )
        .into());
    }
    Ok((duree, String::from_utf8(sortie.stdout)?))
}

struct Ligne {
    operations: usize,
    notes: usize,
    branches: usize,
    base_mo: f64,
    texte_v2_mo: f64,
    ouverture: f64,
    lignes_ouverture: u64,
    execution: f64,
    cli: Option<[Option<f64>; 6]>,
}

fn mesurer(p: &Parametres, g: &Generateur, base: &Path) -> Resultat<Ligne> {
    supprimer(base);
    let texte_v2 = encode_repository_v2(&g.depot);
    drop(Store::import(base, &texte_v2)?);
    let base_mo = taille_mo(base);
    let texte_v2_mo = texte_v2.len() as f64 / 1e6;
    // Ouverture : médiane de trois.
    let mut ouvertures = Vec::new();
    for _ in 0..3 {
        let debut = Instant::now();
        drop(Store::open(base)?);
        ouvertures.push(secondes(debut.elapsed()));
    }
    let ouverture = mediane(ouvertures);
    let lignes_ouverture = Store::open(base)?.lines_read();
    // Exécution d'une commande ordinaire, base déjà ouverte.
    let mut store = Store::open(base)?;
    let note = *g.affirmations.last().ok_or("aucune note")?;
    let decision = match store
        .repository()
        .snapshot()
        .graph(DEFAULT_SCOPE)?
        .node(note)?
        .decision()
    {
        Some(Decision::Retained) => Decision::Proposed,
        _ => Decision::Retained,
    };
    let mut op = [0u8; 16];
    op[0] = 0x6e;
    op[8..].copy_from_slice(&(g.operations() as u64).to_be_bytes());
    let envelope = Envelope {
        op: OpId::from_bytes(op),
        expected: store.repository().expected(),
        reason: "mesure".into(),
        command: Command::Change(Change::SetDecision {
            scope: DEFAULT_SCOPE,
            node: note,
            decision,
        }),
    };
    let debut = Instant::now();
    store.execute(envelope)?;
    let execution = secondes(debut.elapsed());
    drop(store);
    let cli = if p.memoire.exists() {
        let statut = mediane(
            (0..3)
                .map(|_| lancer(&p.memoire, base, &["statut"]).map(|(d, _)| d))
                .collect::<Resultat<_>>()?,
        );
        let (creation, sortie) = lancer(
            &p.memoire,
            base,
            &[
                "note",
                "creer",
                "--role",
                "raisonnement",
                "--titre",
                "mesure",
            ],
        )?;
        let id = sortie
            .trim()
            .strip_prefix("Note créée : ")
            .ok_or_else(|| format!("sortie inattendue de note creer : {sortie}"))?
            .to_owned();
        let (statut_change, _) = lancer(
            &p.memoire,
            base,
            &["decision", &id, "retenue", "--motif", "mesure", "--oui"],
        )?;
        let (annulation, _) = lancer(&p.memoire, base, &["annuler"])?;
        // Bascule vers la dernière branche créée, puis retour : seulement si une branche existe.
        let (bascule, retour) = match g.branches.last() {
            Some(branche) => (
                Some(lancer(&p.memoire, base, &["basculer", &branche.to_string()])?.0),
                Some(lancer(&p.memoire, base, &["basculer", &MAIN.to_string()])?.0),
            ),
            None => (None, None),
        };
        Some([
            Some(statut),
            Some(creation),
            Some(statut_change),
            Some(annulation),
            bascule,
            retour,
        ])
    } else {
        None
    };
    Ok(Ligne {
        operations: g.operations(),
        notes: g.notes(),
        branches: g.depot.branch_count(),
        base_mo,
        texte_v2_mo,
        ouverture,
        lignes_ouverture,
        execution,
        cli,
    })
}

fn main() -> Resultat<()> {
    let p = parametres()?;
    std::fs::create_dir_all(&p.repertoire)?;
    if !p.memoire.exists() {
        eprintln!(
            "Exécutable absent ({}) : mesures de la ligne de commande sautées. \
             Construire avec : cargo build --release -p memoire-cli",
            p.memoire.display()
        );
    }
    println!(
        "Mesure memoire — graine {}, {} notes visées{}, paliers {:?}, plafond d'ouverture {} s",
        p.graine,
        p.notes,
        if p.etat_constant {
            " (état constant)"
        } else {
            ""
        },
        p.paliers,
        p.plafond_ouverture
    );
    println!();
    println!(
        "| Opérations | Notes | Branches | Base (Mo) | Texte v2 (Mo) | Ouverture (s) \
         | Lignes lues à l'ouverture | Exécution (s) | statut (s) | note creer (s) | decision (s) \
         | annuler (s) | basculer (s) | retour main (s) |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    // Les notes sont créées au rythme des paliers : la cible est atteinte au dernier.
    let dernier = *p.paliers.last().unwrap_or(&1);
    let mut g = Generateur::new(p.graine);
    let debut = Instant::now();
    for &palier in &p.paliers {
        let notes_visees = if p.etat_constant {
            p.notes
        } else {
            p.notes * palier / dernier
        };
        g.jusqua(palier, notes_visees, p.etat_constant);
        let base = p.repertoire.join(format!("palier-{palier}.db"));
        let l = mesurer(&p, &g, &base)?;
        let cli = l.cli.map_or_else(
            || "— | — | — | — | — | —".to_owned(),
            |c| {
                c.iter()
                    .map(|d| d.map_or_else(|| "—".to_owned(), |d| format!("{d:.3}")))
                    .collect::<Vec<_>>()
                    .join(" | ")
            },
        );
        println!(
            "| {} | {} | {} | {:.1} | {:.2} | {:.3} | {} | {:.3} | {cli} |",
            l.operations,
            l.notes,
            l.branches,
            l.base_mo,
            l.texte_v2_mo,
            l.ouverture,
            l.lignes_ouverture,
            l.execution
        );
        if l.ouverture > p.plafond_ouverture {
            println!();
            println!(
                "Arrêt : ouverture de {:.1} s au-delà du plafond de {} s ; paliers suivants non mesurés.",
                l.ouverture, p.plafond_ouverture
            );
            break;
        }
    }
    println!();
    println!(
        "Durée totale : {:.0} s ; commandes refusées par le domaine pendant la génération : {}.",
        secondes(debut.elapsed()),
        g.refus
    );
    Ok(())
}
