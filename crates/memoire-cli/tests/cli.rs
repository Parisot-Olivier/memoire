//! Contrat de la ligne de commande, incrément 4.1 (CLI.md, sections 5 à 7). Écrit avant le code.
//!
//! Contrat modifié (incrément 5.2c, D20) : le texte d'une base se lit par `Store::text`, qui le
//! rassemble depuis les tables du schéma 2 ; deux bases se comparent toujours par leur texte
//! canonique complet.

use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::atomic::{AtomicU64, Ordering};

use memoire_app::SequenceIds;
use memoire_cli::{executer, Environnement};
use memoire_domain::decode_repository_v2;
use memoire_store::Store;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Répertoire temporaire propre à un test, supprimé à la fin.
struct Essai {
    dir: PathBuf,
    appels: u64,
}

impl Essai {
    fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("memoire-cli-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir, appels: 0 }
    }

    fn base(&self) -> PathBuf {
        self.dir.join("depot.db")
    }

    /// Lance la commande sur la base de l'essai ; chaque appel reçoit sa propre suite d'identités.
    fn lancer(&mut self, args: &[&str]) -> (i32, String, String) {
        self.appels += 1;
        let mut tout = vec!["--base".to_owned(), self.base().display().to_string()];
        tout.extend(args.iter().map(|a| (*a).to_owned()));
        let environnement = Environnement {
            base: None,
            repertoire: self.dir.clone(),
        };
        let (mut sortie, mut erreurs) = (Vec::new(), Vec::new());
        let code = executer(
            &tout,
            &environnement,
            // Les 8 premiers caractères des identifiants diffèrent d'un appel à l'autre, comme
            // pour des UUID aléatoires, et jamais « 00000000 », réservé à main.
            Box::new(SequenceIds::new(self.appels << 36)),
            &mut sortie,
            &mut erreurs,
        );
        (
            code,
            String::from_utf8(sortie).unwrap(),
            String::from_utf8(erreurs).unwrap(),
        )
    }

    /// Lance et exige le code attendu ; rend la sortie standard.
    fn ok(&mut self, args: &[&str]) -> String {
        let (code, out, err) = self.lancer(args);
        assert_eq!(code, 0, "{args:?}\nsortie : {out}\nerreurs : {err}");
        out
    }

    fn texte_du_depot(&self) -> String {
        Store::open(&self.base()).unwrap().text().unwrap()
    }
}

impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Identifiant affiché à la fin d'une ligne « … : <id> ».
fn identifiant(sortie: &str, debut: &str) -> String {
    sortie
        .lines()
        .find_map(|l| l.strip_prefix(debut))
        .unwrap_or_else(|| panic!("« {debut} » absent de : {sortie}"))
        .trim()
        .to_owned()
}

#[test]
fn init_creates_the_base_once() {
    let mut e = Essai::new("init");
    let out = e.ok(&["init"]);
    assert_eq!(out, format!("Dépôt créé : {}\n", e.base().display()));
    let (code, _, err) = e.lancer(&["init"]);
    assert_eq!(code, 3);
    assert!(err.contains("existe déjà"), "{err}");
}

#[test]
fn usage_errors_are_refused_with_code_2() {
    let mut e = Essai::new("usage");
    e.ok(&["init"]);
    for args in [
        &[][..],
        &["inconnue"][..],
        &["statut", "--inconnue"][..],
        &["statut", "en-trop"][..],
        &["note", "creer", "--titre"][..],
        &[
            "note", "creer", "--role", "note", "--titre", "a", "--titre", "b",
        ][..],
        &["note", "creer", "--role", "inconnu", "--titre", "a"][..],
        &["note", "creer", "--titre", "sans rôle"][..],
        &["verdict", "abcd"][..],
    ] {
        let (code, _, err) = e.lancer(args);
        assert_eq!(code, 2, "{args:?} : {err}");
        assert!(err.starts_with("erreur : "), "{err}");
    }
}

#[test]
fn help_is_generated_from_the_command_table() {
    let mut e = Essai::new("aide");
    let aide = e.ok(&["aide"]);
    for nom in [
        "init",
        "statut",
        "note creer",
        "lier",
        "verdict",
        "decision",
        "annuler",
        "retablir",
        "branches",
        "basculer",
        "exporter",
        "importer",
        "aide",
    ] {
        assert!(
            aide.lines().any(|l| l.starts_with(&format!("  {nom} "))),
            "{nom} absent de l'aide :\n{aide}"
        );
    }
    let lier = e.ok(&["aide", "lier"]);
    assert!(lier.contains("ORIGINE TYPE DESTINATION"), "{lier}");
    let (code, _, _) = e.lancer(&["aide", "inconnue"]);
    assert_eq!(code, 2);
}

#[test]
fn notes_relations_and_status_follow_the_domain() {
    let mut e = Essai::new("parcours");
    e.ok(&["init"]);
    let a = identifiant(
        &e.ok(&[
            "note",
            "creer",
            "--role",
            "raisonnement",
            "--titre",
            "hypothèse",
            "--texte",
            "le réveil est atomique",
        ]),
        "Note créée : ",
    );
    let b = identifiant(
        &e.ok(&[
            "note",
            "creer",
            "--role",
            "implementation",
            "--titre",
            "réveil",
        ]),
        "Note créée : ",
    );
    let statut = e.ok(&["statut"]);
    assert!(
        statut.contains("Branche active : 00000000-0000-0000-0000-000000000000 (main)"),
        "{statut}"
    );
    assert!(statut.contains(&format!("  - hypothèse ({a})")), "{statut}");
    assert!(statut.contains(&format!("  - réveil ({b})")), "{statut}");
    // Préfixe de 8 caractères accepté ; relation ordinaire, appliquée sans confirmation.
    let lien = e.ok(&["lier", &b[..8], "depend-de", &a]);
    assert!(lien.starts_with("Relation créée : "), "{lien}");
    // Préfixe trop court ou inconnu : refusé (usage).
    assert_eq!(e.lancer(&["lier", &b[..7], "lien", &a]).0, 2);
    assert_eq!(e.lancer(&["lier", "ffffffff", "lien", &a]).0, 2);
    // Type inconnu : usage ; relation interdite par le domaine : refus 1.
    assert_eq!(e.lancer(&["lier", &b, "inconnu", &a]).0, 2);
    assert_eq!(e.lancer(&["lier", &a, "depend-de", &a]).0, 1);
    // Motif obligatoire : option absente (usage) ou vide (refus du domaine).
    assert_eq!(e.lancer(&["verdict", &a, "refutee"]).0, 2);
    assert_eq!(
        e.lancer(&["verdict", &a, "refutee", "--motif", "  ", "--oui"])
            .0,
        1
    );
    // Opération critique sans --oui : aperçu, rien d'écrit, code 4.
    let avant = e.texte_du_depot();
    let (code, out, err) = e.lancer(&["verdict", &a, "refutee", "--motif", "contre-exemple"]);
    assert_eq!(code, 4, "{out}{err}");
    assert!(out.contains("Opération critique :"), "{out}");
    assert!(out.contains("une branche sera créée depuis main"), "{out}");
    assert!(
        out.contains(&format!("à réexaminer : réveil ({b})")),
        "{out}"
    );
    assert!(err.contains("--oui"), "{err}");
    assert_eq!(e.texte_du_depot(), avant);
    // Avec --oui : appliquée, branche créée, cause visible dans le statut.
    let applique = e.ok(&[
        "verdict",
        &a,
        "refutee",
        "--motif",
        "contre-exemple",
        "--oui",
    ]);
    let branche = identifiant(&applique, "Branche créée : ");
    assert!(applique.contains("Statut enregistré."), "{applique}");
    let statut = e.ok(&["statut"]);
    assert!(
        statut.contains(&format!("Branche active : {branche}")),
        "{statut}"
    );
    assert!(
        statut.contains(&format!("  - réveil ({b}) — cause : hypothèse ({a})")),
        "{statut}"
    );
    // Décision ordinaire sur la branche expérimentale.
    let decision = e.ok(&["decision", &b, "retenue", "--motif", "choix"]);
    assert!(decision.contains("Statut enregistré."), "{decision}");
    assert!(!decision.contains("Branche créée"), "{decision}");
    assert_eq!(e.lancer(&["decision", &b, "inconnue", "--motif", "x"]).0, 2);
}

#[test]
fn history_and_branches() {
    let mut e = Essai::new("historique");
    e.ok(&["init"]);
    let a = identifiant(
        &e.ok(&["note", "creer", "--role", "raisonnement", "--titre", "a"]),
        "Note créée : ",
    );
    let branche = identifiant(
        &e.ok(&["verdict", &a, "refutee", "--motif", "réfutée", "--oui"]),
        "Branche créée : ",
    );
    let liste = e.ok(&["branches"]);
    assert!(
        liste.contains("  00000000-0000-0000-0000-000000000000 (main)"),
        "{liste}"
    );
    assert!(liste.contains(&format!("* {branche}")), "{liste}");
    assert_eq!(e.ok(&["annuler"]), "Annulé.\n");
    assert!(e.ok(&["statut"]).contains("(main)"));
    assert_eq!(e.ok(&["retablir"]), "Rétabli.\n");
    assert!(e
        .ok(&["statut"])
        .contains(&format!("Branche active : {branche}")));
    assert_eq!(
        e.ok(&["basculer", "00000000"]),
        "Branche active : 00000000-0000-0000-0000-000000000000 (main)\n"
    );
    assert_eq!(
        e.ok(&["basculer", &branche[..8]]),
        format!("Branche active : {branche}\n")
    );
    assert_eq!(e.lancer(&["basculer", "1234567"]).0, 2);
    // Rien à rétablir : refus du domaine.
    assert_eq!(e.lancer(&["retablir"]).0, 1);
}

// Contrat modifié (incrément 5.1) : le fichier exporté est au format version 2.
#[test]
fn export_and_import() {
    let mut e = Essai::new("export");
    e.ok(&["init"]);
    e.ok(&[
        "note",
        "creer",
        "--role",
        "categorie",
        "--titre",
        "Driver GPIO",
    ]);
    let fichier = e.dir.join("depot.txt");
    let out = e.ok(&["exporter", fichier.to_str().unwrap()]);
    assert_eq!(out, format!("Exporté : {}\n", fichier.display()));
    assert_eq!(
        std::fs::read_to_string(&fichier).unwrap(),
        Store::open(&e.base()).unwrap().text().unwrap()
    );
    let mut copie = Essai::new("import");
    let out = copie.ok(&["importer", fichier.to_str().unwrap()]);
    assert_eq!(out, format!("Importé dans : {}\n", copie.base().display()));
    assert_eq!(copie.texte_du_depot(), e.texte_du_depot());
    assert_eq!(copie.lancer(&["importer", fichier.to_str().unwrap()]).0, 3);
    std::fs::write(e.dir.join("faux.txt"), "pas un dépôt\n").unwrap();
    let mut autre = Essai::new("import-faux");
    let (code, _, err) = autre.lancer(&["importer", e.dir.join("faux.txt").to_str().unwrap()]);
    assert_eq!(code, 3);
    assert!(err.contains("ligne 1"), "{err}");
    assert!(!autre.base().exists());
}

#[test]
fn a_missing_base_is_reported() {
    let mut e = Essai::new("absente");
    let (code, _, err) = e.lancer(&["statut"]);
    assert_eq!(code, 3);
    assert!(err.starts_with("erreur : "), "{err}");
}

/// Base par défaut et variable d'environnement, à travers l'exécutable lui-même.
#[test]
fn the_executable_runs_end_to_end() {
    let e = Essai::new("executable");
    let programme = env!("CARGO_BIN_EXE_memoire");
    let lancer = |args: &[&str], base: Option<&Path>| {
        let mut commande = Process::new(programme);
        commande
            .args(args)
            .current_dir(&e.dir)
            .env_remove("MEMOIRE_BASE");
        if let Some(base) = base {
            commande.env("MEMOIRE_BASE", base);
        }
        commande.output().unwrap()
    };
    // Base par défaut : .memoire/depot.db dans le répertoire courant.
    assert!(lancer(&["init"], None).status.success());
    assert!(e.dir.join(".memoire/depot.db").exists());
    // Variable d'environnement.
    let autre = e.dir.join("autre.db");
    assert!(lancer(&["init"], Some(&autre)).status.success());
    let sortie = lancer(
        &["note", "creer", "--role", "note", "--titre", "t"],
        Some(&autre),
    );
    assert!(sortie.status.success());
    assert!(String::from_utf8(sortie.stdout)
        .unwrap()
        .starts_with("Note créée : "));
    let refus = lancer(&["inconnue"], Some(&autre));
    assert_eq!(refus.status.code(), Some(2));
}

// ---------------------------------------------------------------------------
// Compléments après mutation (incrément 4.1) : aide figée, option --base, confirmation d'une
// bifurcation sans cause, import dans un répertoire à créer.
// ---------------------------------------------------------------------------

/// L'aide complète est figée : elle fait partie de l'interface (tests/aide.txt).
#[test]
fn the_help_text_is_frozen() {
    let mut e = Essai::new("aide-figee");
    assert_eq!(e.ok(&["aide"]), include_str!("aide.txt"));
}

#[test]
fn the_base_option_needs_a_value_and_a_command() {
    let e = Essai::new("option-base");
    let lancer = |args: &[&str]| {
        let environnement = Environnement {
            base: None,
            repertoire: e.dir.clone(),
        };
        let tout: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let (mut sortie, mut erreurs) = (Vec::new(), Vec::new());
        let code = executer(
            &tout,
            &environnement,
            Box::new(SequenceIds::new(1 << 36)),
            &mut sortie,
            &mut erreurs,
        );
        (code, String::from_utf8(erreurs).unwrap())
    };
    let (code, err) = lancer(&["--base"]);
    assert_eq!(code, 2);
    assert!(err.contains("valeur manquante pour --base"), "{err}");
    let (code, err) = lancer(&["--base", "x.db"]);
    assert_eq!(code, 2);
    assert!(err.contains("commande attendue"), "{err}");
}

/// Une réfutation sans dépendant ne crée aucune cause, mais bifurque depuis main : elle reste
/// critique et demande confirmation.
#[test]
fn a_fork_without_review_effect_still_needs_confirmation() {
    let mut e = Essai::new("bifurcation-seule");
    e.ok(&["init"]);
    let a = identifiant(
        &e.ok(&[
            "note",
            "creer",
            "--role",
            "raisonnement",
            "--titre",
            "seule",
        ]),
        "Note créée : ",
    );
    let avant = e.texte_du_depot();
    let (code, out, _) = e.lancer(&["verdict", &a, "refutee", "--motif", "réfutée"]);
    assert_eq!(code, 4, "{out}");
    assert!(out.contains("une branche sera créée depuis main"), "{out}");
    assert!(!out.contains("réexaminer"), "{out}");
    assert_eq!(e.texte_du_depot(), avant);
}

/// L'import crée au besoin le répertoire de la base.
#[test]
fn import_creates_the_directory_of_the_base() {
    let mut e = Essai::new("import-repertoire");
    e.ok(&["init"]);
    let fichier = e.dir.join("export.txt");
    e.ok(&["exporter", fichier.to_str().unwrap()]);
    let base = e.dir.join("sous").join("dossier").join("copie.db");
    let environnement = Environnement {
        base: Some(base.clone()),
        repertoire: e.dir.clone(),
    };
    let (mut sortie, mut erreurs) = (Vec::new(), Vec::new());
    let args = vec!["importer".to_owned(), fichier.display().to_string()];
    let code = executer(
        &args,
        &environnement,
        Box::new(SequenceIds::new(9 << 36)),
        &mut sortie,
        &mut erreurs,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&erreurs));
    assert!(base.exists());
}

// R9 : sauvegarde manuelle, vers un fichier choisi ou, par défaut, horodatée dans sauvegardes/.
#[test]
fn manual_backups() {
    let mut e = Essai::new("sauvegarde");
    e.ok(&["init"]);
    e.ok(&["note", "creer", "--role", "note", "--titre", "à garder"]);
    let cible = e.dir.join("copie.db");
    assert_eq!(
        e.ok(&["sauvegarder", cible.to_str().unwrap()]),
        format!("Sauvegarde écrite : {}\n", cible.display())
    );
    assert_eq!(
        Store::open(&cible).unwrap().text().unwrap(),
        e.texte_du_depot()
    );
    assert_eq!(e.lancer(&["sauvegarder", cible.to_str().unwrap()]).0, 3);
    let out = e.ok(&["sauvegarder"]);
    let chemin = out.strip_prefix("Sauvegarde écrite : ").unwrap().trim_end();
    let nom = Path::new(chemin)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        chemin.starts_with(e.dir.join("sauvegardes").to_str().unwrap()),
        "{chemin}"
    );
    assert!(
        nom.starts_with("depot-")
            && nom.ends_with("Z.db")
            && nom.len() == "depot-20261025-184807Z.db".len(),
        "{nom}"
    );
    assert_eq!(
        Store::open(Path::new(chemin)).unwrap().text().unwrap(),
        e.texte_du_depot()
    );
}

// R3 : recherche par nom, insensible à la casse et aux accents ; les titres qui commencent par le
// texte d'abord ; une ligne par note : identifiant, rôle, désignation, séparés par des tabulations.
#[test]
fn notes_are_found_by_name() {
    let mut e = Essai::new("chercher");
    e.ok(&["init"]);
    let id = |e: &mut Essai, role: &str, titre: &str| {
        identifiant(
            &e.ok(&["note", "creer", "--role", role, "--titre", titre]),
            "Note créée : ",
        )
    };
    let driver = id(&mut e, "categorie", "Driver GPIO : Réveil des tâches");
    let perdu = id(&mut e, "raisonnement", "réveil perdu");
    id(&mut e, "implementation", "Ordonnanceur");
    let out = e.ok(&["chercher", "REVEIL"]);
    assert_eq!(
        out,
        format!("{perdu}\traisonnement\tréveil perdu\n{driver}\tcategorie\tDriver GPIO : Réveil des tâches\n")
    );
    assert_eq!(
        e.ok(&["chercher", "taches"]),
        format!("{driver}\tcategorie\tDriver GPIO : Réveil des tâches\n")
    );
    assert_eq!(e.ok(&["chercher", "absent"]), "");
    assert_eq!(e.lancer(&["chercher"]).0, 2);
    // Un passage se cherche par son texte.
    let r = e.dir.join("r.txt");
    std::fs::write(&r, "Une phrase à citer exactement.").unwrap();
    let x = identifiant(
        &e.ok(&[
            "echange",
            "coller",
            "--titre",
            "conversation",
            "--reponse-fichier",
            r.to_str().unwrap(),
        ]),
        "Échange créé : ",
    );
    // Contrat modifié (5.5b, D18, D24) : « passage » devient « citer », et le rôle s'affiche
    // « citation ».
    let p = identifiant(
        &e.ok(&["citer", &x, "--texte", "phrase à citer"]),
        "Citation créée : ",
    );
    let p = p.split(' ').next().unwrap().to_owned();
    assert_eq!(
        e.ok(&["chercher", "phrase a"]),
        format!("{p}\tcitation\t« phrase à citer »\n")
    );
}

/// Base de l'essai, ouverte directement en SQLite, pour l'altérer hors du logiciel.
fn alterer(e: &Essai, sql: &str) {
    rusqlite::Connection::open(e.base())
        .unwrap()
        .execute_batch(sql)
        .unwrap();
}

// D21 : « verifier » relit tout l'historique et confirme l'état courant de chaque branche ; un
// écart est signalé comme une erreur de stockage (code 3), jamais comme un refus métier. Une
// lecture impossible d'un état ancien pendant une commande est aussi une erreur de stockage.
#[test]
fn full_verification_and_history_faults() {
    let mut e = Essai::new("verifier");
    e.ok(&["init"]);
    e.ok(&[
        "note",
        "creer",
        "--role",
        "raisonnement",
        "--titre",
        "première",
    ]);
    e.ok(&[
        "note",
        "creer",
        "--role",
        "raisonnement",
        "--titre",
        "seconde",
    ]);
    assert_eq!(
        e.ok(&["verifier"]),
        "Base vérifiée : 3 commits, 1 branche, état courant confirmé.\n"
    );
    assert_eq!(e.lancer(&["verifier", "en-trop"]).0, 2);
    // Différence de la première création rendue illisible : l'ouverture reste possible (D21),
    // mais la vérification et l'annulation qui relit cet état échouent, en code 3.
    alterer(
        &e,
        "UPDATE lignes_difference SET ligne = replace(ligne, 'titre', 'title')
         WHERE commit_id = (SELECT id FROM commits WHERE profondeur = 1)",
    );
    assert_eq!(e.lancer(&["statut"]).0, 0);
    let (code, _, err) = e.lancer(&["verifier"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.starts_with("erreur : contenu illisible : "), "{err}");
    let (code, _, err) = e.lancer(&["annuler"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("historique illisible"), "{err}");
}

/// Base de schéma 1 écrite par l'exécutable de `v0.5.0-alpha.1`, et son export v2.
const SCHEMA_1_DB: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/schema1/sref01-schema1.db"
);
const SCHEMA_1_EXPORT: &str = include_str!("../../../fixtures/schema1/sref01-export-v2.txt");

// 5.2d (ETAT-HISTORIQUE.md, sections 7 et 16) : une base de schéma 1 est refusée avec
// l'indication de la migration ; « migrer » la sauvegarde dans sauvegardes/, la convertit et la
// vérifie ; ensuite les commandes ordinaires fonctionnent et une seconde migration ne fait rien.
#[test]
fn migration_of_a_schema_1_base() {
    let mut e = Essai::new("migrer");
    std::fs::copy(SCHEMA_1_DB, e.base()).unwrap();
    let (code, _, err) = e.lancer(&["statut"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("memoire migrer"), "{err}");
    assert_eq!(e.lancer(&["migrer", "en-trop"]).0, 2);
    let out = e.ok(&["migrer"]);
    let mut lignes = out.lines();
    let chemin = lignes
        .next()
        .unwrap()
        .strip_prefix("Sauvegarde écrite : ")
        .unwrap()
        .to_owned();
    assert_eq!(
        lignes.next(),
        Some("Base migrée au schéma 2 : 47 commits, 2 branches, vérifiée.")
    );
    assert_eq!(lignes.next(), None);
    let nom = Path::new(&chemin).file_name().unwrap().to_str().unwrap();
    assert!(
        chemin.starts_with(e.dir.join("sauvegardes").to_str().unwrap()),
        "{chemin}"
    );
    assert!(
        nom.starts_with("avant-migration-")
            && nom.ends_with("Z.db")
            && nom.len() == "avant-migration-20261106-184807Z.db".len(),
        "{nom}"
    );
    assert_eq!(Store::schema(Path::new(&chemin)).unwrap(), "1");
    assert_eq!(e.texte_du_depot(), SCHEMA_1_EXPORT);
    assert!(e.ok(&["statut"]).contains("Branche active"));
    assert_eq!(
        e.ok(&["migrer"]),
        "Base déjà au schéma 2 : rien à migrer.\n"
    );
    assert_eq!(
        std::fs::read_dir(e.dir.join("sauvegardes"))
            .unwrap()
            .count(),
        1
    );
}

// 5.2d : sans base, ou sur une base déjà au schéma 2, « migrer » ne crée aucune sauvegarde ;
// une base illisible est refusée en code 3, intacte.
#[test]
fn migration_refusals() {
    let mut e = Essai::new("migrer-refus");
    assert_eq!(e.lancer(&["migrer"]).0, 3);
    assert!(!e.base().exists());
    assert!(!e.dir.join("sauvegardes").exists());
    e.ok(&["init"]);
    assert_eq!(
        e.ok(&["migrer"]),
        "Base déjà au schéma 2 : rien à migrer.\n"
    );
    assert!(!e.dir.join("sauvegardes").exists());
    let mut e = Essai::new("migrer-illisible");
    std::fs::copy(SCHEMA_1_DB, e.base()).unwrap();
    alterer(
        &e,
        "UPDATE fragments SET texte = replace(texte, 'titre', 'title') WHERE categorie = 'commit'",
    );
    let avant = std::fs::read(e.base()).unwrap();
    let (code, _, err) = e.lancer(&["migrer"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.starts_with("erreur : "), "{err}");
    assert_eq!(std::fs::read(e.base()).unwrap(), avant);
}

/// Crée une note et rend son identifiant.
fn creer(e: &mut Essai, role: &str, titre: &str) -> String {
    identifiant(
        &e.ok(&["note", "creer", "--role", role, "--titre", titre]),
        "Note créée : ",
    )
}

// R1, INV-LOT-01 (incrément 5.3, LOT.md, section 5) : « note creer » avec des liens crée la note
// et ses relations en une seule opération ; la note créée est l'origine de chaque relation ; un
// seul « annuler » défait le tout. Les notes se donnent par identifiant ou préfixe, séparées par
// des virgules ; les relations sont annoncées dans l'ordre des options, puis des notes.
#[test]
fn creating_a_note_with_its_links_is_one_operation() {
    let mut e = Essai::new("note-liee");
    e.ok(&["init"]);
    let categorie = creer(&mut e, "categorie", "Pilote");
    let a = creer(&mut e, "raisonnement", "Réveil perdu");
    let b = creer(&mut e, "raisonnement", "Acquittement");
    let avant = e.texte_du_depot();
    let out = e.ok(&[
        "note",
        "creer",
        "--role",
        "implementation",
        "--titre",
        "Gestionnaire d'attente",
        "--lien",
        &categorie,
        "--depend-de",
        &format!("{},{}", &a[..8], b),
        "--classe-dans",
        &categorie,
    ]);
    let lignes: Vec<&str> = out.lines().collect();
    assert_eq!(lignes.len(), 5, "{out}");
    let note = lignes[0].strip_prefix("Note créée : ").unwrap();
    let attendues = [
        format!("(classe-dans {categorie})"),
        format!("(depend-de {a})"),
        format!("(depend-de {b})"),
        format!("(lien {categorie})"),
    ];
    for (ligne, attendue) in lignes[1..].iter().zip(&attendues) {
        let reste = ligne.strip_prefix("Relation créée : ").unwrap();
        assert!(reste.ends_with(attendue.as_str()), "{ligne}");
        assert_eq!(reste.split(' ').next().unwrap().len(), 36, "{ligne}");
    }
    let statut = e.ok(&["statut"]);
    assert!(statut.contains(note), "{statut}");
    // Une seule opération : un seul « annuler » ramène le dépôt à l'état d'avant.
    e.ok(&["annuler"]);
    let apres = decode_repository_v2(&e.texte_du_depot()).unwrap();
    assert_eq!(
        apres.snapshot(),
        decode_repository_v2(&avant).unwrap().snapshot()
    );
    assert_eq!(e.ok(&["chercher", "Gestionnaire"]), "");
    // Sans option de lien, la commande est celle d'avant : une note seule.
    let out = e.ok(&["note", "creer", "--role", "note", "--titre", "seule"]);
    assert_eq!(out.lines().count(), 1, "{out}");
}

// D13, D22 : un lot critique (ici une dépendance envers une note réfutée, sur une branche) montre
// son aperçu et n'écrit rien sans --oui ; avec --oui, il s'applique en une seule opération.
#[test]
fn a_critical_note_with_links_needs_confirmation() {
    let mut e = Essai::new("note-liee-critique");
    e.ok(&["init"]);
    let a = creer(&mut e, "raisonnement", "Hypothèse");
    let out = e.ok(&[
        "verdict",
        &a,
        "refutee",
        "--motif",
        "contre-exemple",
        "--oui",
    ]);
    assert!(out.contains("Branche créée : "), "{out}");
    let avant = e.texte_du_depot();
    let commande = [
        "note",
        "creer",
        "--role",
        "implementation",
        "--titre",
        "Appui fragile",
        "--depend-de",
        &a,
    ];
    let (code, out, _) = e.lancer(&commande);
    assert_eq!(code, 4, "{out}");
    assert!(out.contains("Opération critique"), "{out}");
    assert!(out.contains("à réexaminer"), "{out}");
    assert!(!out.contains("une branche sera créée"), "{out}");
    assert_eq!(e.texte_du_depot(), avant);
    let mut confirmee = commande.to_vec();
    confirmee.push("--oui");
    let out = e.ok(&confirmee);
    assert!(out.contains("Note créée : "), "{out}");
    assert!(out.contains(&format!("(depend-de {a})")), "{out}");
    assert!(!out.contains("Branche créée"), "{out}");
}

// LOT.md, section 5 : une liste mal formée, une note inconnue, ambiguë ou citée deux fois pour le
// même type sont refusées en erreur d'usage (code 2), sans rien écrire.
#[test]
fn malformed_note_lists_are_refused() {
    let mut e = Essai::new("note-liee-refus");
    e.ok(&["init"]);
    let a = creer(&mut e, "raisonnement", "a");
    let avant = e.texte_du_depot();
    let base = ["note", "creer", "--role", "raisonnement", "--titre", "t"];
    for liste in [
        format!("{a},{a}"),
        format!("{a},"),
        format!(",{a}"),
        format!("{a},,{a}"),
        "ffffffff-ffff-ffff-ffff-ffffffffffff".to_owned(),
        "abc".to_owned(),
        String::new(),
    ] {
        let mut args = base.to_vec();
        args.extend(["--depend-de", &liste]);
        let (code, out, err) = e.lancer(&args);
        assert_eq!(code, 2, "« {liste} » : {out}{err}");
        assert_eq!(e.texte_du_depot(), avant, "« {liste} »");
    }
    // Deux types différents vers la même note : accepté.
    let mut args = base.to_vec();
    args.extend(["--depend-de", &a, "--lien", &a]);
    let out = e.ok(&args);
    assert_eq!(out.matches("Relation créée : ").count(), 2, "{out}");
}

// INV-BR-05 (incrément 5.4a ; FUSION.md, sections 9.7 et 9.9) : abandonner une branche ne
// supprime rien ; la branche active revient alors sur son origine ; une branche abandonnée
// disparaît de « branches » mais reste listée par « branches --toutes », n'est plus basculable,
// et se réactive sans devenir active. Aucune confirmation : l'abandon se défait par « reactiver ».
#[test]
fn abandoning_and_reactivating_a_branch() {
    let mut e = Essai::new("abandon");
    e.ok(&["init"]);
    let a = creer(&mut e, "raisonnement", "Hypothèse");
    let b = creer(&mut e, "implementation", "Appui");
    e.ok(&["lier", &b, "depend-de", &a]);
    let branche = identifiant(
        &e.ok(&[
            "verdict",
            &a,
            "refutee",
            "--motif",
            "contre-exemple",
            "--oui",
        ]),
        "Branche créée : ",
    );
    let main = "00000000-0000-0000-0000-000000000000 (main)";
    // Ligne de la branche dans « branches --toutes », sans la marque de la branche active.
    let tete = |e: &mut Essai| {
        let liste = e.ok(&["branches", "--toutes"]);
        liste.lines().find(|l| l.contains(&branche)).unwrap()[2..].to_owned()
    };
    let avant = tete(&mut e);
    // Abandon de la branche active : retour sur main, annoncé.
    assert_eq!(
        e.ok(&["abandonner", &branche[..8], "--motif", "piste close"]),
        format!("Branche abandonnée : {branche}\nBranche active : {main}\n")
    );
    assert!(e
        .ok(&["statut"])
        .contains(&format!("Branche active : {main}")));
    let liste = e.ok(&["branches"]);
    assert_eq!(liste, format!("* {main} — tête {}\n", tete_de_main(&liste)));
    let toutes = e.ok(&["branches", "--toutes"]);
    assert_eq!(toutes.lines().count(), 2, "{toutes}");
    let ligne = tete(&mut e);
    assert_eq!(ligne, format!("{avant} (abandonnée)"));
    // Ni basculable ni abandonnable une seconde fois : refus du domaine, rien n'est écrit.
    let texte = e.texte_du_depot();
    let (code, _, err) = e.lancer(&["basculer", &branche]);
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains(&format!("branche abandonnée : {branche}")),
        "{err}"
    );
    assert_eq!(e.lancer(&["abandonner", &branche]).0, 1);
    assert_eq!(e.texte_du_depot(), texte);
    // « annuler » ne défait pas l'abandon : il annule la dernière opération de main.
    assert_eq!(e.ok(&["annuler"]), "Annulé.\n");
    assert!(tete(&mut e).ends_with(" (abandonnée)"));
    e.ok(&["retablir"]);
    // Réactivation : la branche redevient listée, sans devenir active.
    assert_eq!(
        e.ok(&["reactiver", &branche]),
        format!("Branche réactivée : {branche}\n")
    );
    assert_eq!(tete(&mut e), avant);
    assert!(e
        .ok(&["branches"])
        .contains(&format!("  {branche} — tête ")));
    assert!(e
        .ok(&["statut"])
        .contains(&format!("Branche active : {main}")));
    assert_eq!(e.lancer(&["reactiver", &branche]).0, 1);
    // Abandon d'une branche qui n'est pas active : la branche active ne change pas.
    assert_eq!(
        e.ok(&["abandonner", &branche]),
        format!("Branche abandonnée : {branche}\n")
    );
    e.ok(&["reactiver", &branche]);
    assert_eq!(
        e.ok(&["basculer", &branche]),
        format!("Branche active : {branche}\n")
    );
}

/// Tête de main dans une liste de branches.
fn tete_de_main(liste: &str) -> String {
    identifiant(
        liste,
        "* 00000000-0000-0000-0000-000000000000 (main) — tête ",
    )
}

// main ne s'abandonne pas (refus du domaine, code 1) ; une branche inconnue, un préfixe trop
// court ou un argument manquant sont des erreurs d'usage (code 2).
#[test]
fn abandon_refusals() {
    let mut e = Essai::new("abandon-refus");
    e.ok(&["init"]);
    let (code, _, err) = e.lancer(&["abandonner", "00000000"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("main ne peut pas être abandonnée"), "{err}");
    let (code, _, err) = e.lancer(&["reactiver", "00000000"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("branche non abandonnée"), "{err}");
    for args in [
        &["abandonner"][..],
        &["abandonner", "1234567"][..],
        &["abandonner", "12345678"][..],
        &["reactiver"][..],
        &["reactiver", "00000000", "en-trop"][..],
        &["branches", "--tout"][..],
    ] {
        assert_eq!(e.lancer(args).0, 2, "{args:?}");
    }
}

/// Code du plan affiché par « fusion plan ».
fn code_du_plan(plan: &str) -> String {
    identifiant(plan, "Code du plan : ")
}

/// Sauvegardes écrites avant une fusion.
fn sauvegardes_de_fusion(e: &Essai) -> Vec<String> {
    std::fs::read_dir(e.dir.join("sauvegardes"))
        .map(|entrees| {
            entrees
                .map(|f| f.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|nom| nom.starts_with("avant-fusion-") && nom.ends_with(".db"))
                .collect()
        })
        .unwrap_or_default()
}

/// Texte courant d'une note dans la base de l'essai.
fn texte_de(e: &Essai, note: &str) -> String {
    let depot = decode_repository_v2(&e.texte_du_depot()).unwrap();
    let id = memoire_domain::NodeId::parse(note).unwrap();
    depot
        .snapshot()
        .graph(memoire_domain::DEFAULT_SCOPE)
        .unwrap()
        .node(id)
        .unwrap()
        .text()
        .to_owned()
}

// INV-MERGE-01, D19, D23 (FUSION.md, section 9.9) : « fusion plan » montre la base, ce qui est
// réuni, les conflits numérotés, les effets et le code du plan ; « fusionner » exige ce code,
// demande --oui quand la fusion change des réexamens, écrit une sauvegarde avant toute écriture,
// et publie un seul commit qu'un seul « annuler » défait.
#[test]
fn merging_a_branch_follows_its_plan_after_a_backup() {
    let mut e = Essai::new("fusion");
    e.ok(&["init"]);
    let a = creer(&mut e, "raisonnement", "Hypothèse");
    let b = creer(&mut e, "implementation", "Appui");
    e.ok(&["lier", &b, "depend-de", &a]);
    let x = identifiant(
        &e.ok(&[
            "verdict",
            &a,
            "refutee",
            "--motif",
            "contre-exemple",
            "--oui",
        ]),
        "Branche créée : ",
    );
    let c = creer(&mut e, "categorie", "Pilote");
    e.ok(&["basculer", "00000000"]);
    let main = "00000000-0000-0000-0000-000000000000 (main)";
    let plan = e.ok(&["fusion", "plan", &x[..8]]);
    assert!(
        plan.starts_with(&format!("Fusion de {x} dans {main} :\nBase : ")),
        "{plan}"
    );
    // Note c, son membre, et le verdict de la note a.
    assert!(plan.contains("Réunis sans conflit : 3\n"), "{plan}");
    assert!(plan.contains("Conflits :\n  (aucun)\n"), "{plan}");
    assert!(
        plan.contains(&format!("  - à réexaminer : Appui ({b})\n")),
        "{plan}"
    );
    let code = code_du_plan(&plan);
    assert_eq!(code.len(), 17, "{plan}");
    assert!(
        plan.ends_with(&format!("Code du plan : {code}\n")),
        "{plan}"
    );
    let avant = e.texte_du_depot();
    // Code périmé ou inventé : erreur d'usage, rien n'est écrit.
    let (status, _, err) = e.lancer(&["fusionner", &x, "--plan", "00000000-00000000", "--oui"]);
    assert_eq!(status, 2, "{err}");
    assert!(err.contains("plan périmé"), "{err}");
    // Des réexamens changent : aperçu, confirmation exigée, ni sauvegarde ni écriture.
    let (status, out, _) = e.lancer(&["fusionner", &x, "--plan", &code]);
    assert_eq!(status, 4, "{out}");
    assert!(out.contains("Opération critique"), "{out}");
    assert_eq!(e.texte_du_depot(), avant);
    assert!(sauvegardes_de_fusion(&e).is_empty());
    let out = e.ok(&["fusionner", &x, "--plan", &code, "--oui"]);
    let copie = identifiant(&out, "Sauvegarde écrite : ");
    assert_eq!(sauvegardes_de_fusion(&e).len(), 1, "{out}");
    // La sauvegarde est l'état d'avant la fusion.
    assert_eq!(
        Store::open(Path::new(&copie)).unwrap().text().unwrap(),
        avant
    );
    let fusion = identifiant(&out, "Fusion enregistrée : ");
    assert!(!out.contains("Branche créée"), "{out}");
    assert!(e.ok(&["chercher", "Pilote"]).contains(&c));
    let statut = e.ok(&["statut"]);
    assert!(
        statut.contains(&format!("Appui ({b}) — cause : Hypothèse ({a})")),
        "{statut}"
    );
    assert!(e
        .ok(&["branches"])
        .contains(&format!("* {main} — tête {fusion}\n")));
    // Un seul « annuler » défait la fusion ; rétablie, la source est déjà réunie.
    e.ok(&["annuler"]);
    assert!(!e.ok(&["statut"]).contains("cause : Hypothèse"));
    e.ok(&["retablir"]);
    assert_eq!(
        e.ok(&["fusion", "plan", &x]),
        format!("Rien à fusionner : {x} est déjà réunie dans {main}.\n")
    );
}

// FUSION.md, section 9.9 : chaque conflit se résout par son numéro, avec --cible ou --source ;
// un conflit sans résolution est un refus du domaine ; un numéro inconnu, répété ou mal formé est
// une erreur d'usage. Aucun refus n'écrit de sauvegarde.
#[test]
fn merge_conflicts_are_resolved_by_their_numbers() {
    let mut e = Essai::new("fusion-conflits");
    e.ok(&["init"]);
    let a = creer(&mut e, "raisonnement", "Hypothèse");
    let b = creer(&mut e, "implementation", "Appui");
    e.ok(&["lier", &b, "depend-de", &a]);
    let x = identifiant(
        &e.ok(&[
            "verdict",
            &a,
            "refutee",
            "--motif",
            "contre-exemple",
            "--oui",
        ]),
        "Branche créée : ",
    );
    let reviser = |e: &mut Essai, texte: &str| {
        e.ok(&[
            "note", "reviser", &b, "--texte", texte, "--nature", "mineure",
        ]);
    };
    reviser(&mut e, "côté source");
    e.ok(&["basculer", "00000000"]);
    e.ok(&["verdict", &a, "verifiee", "--motif", "preuve"]);
    reviser(&mut e, "côté cible");
    let plan = e.ok(&["fusion", "plan", &x]);
    assert!(
        plan.contains(&format!(
            "Conflits :\n  1. texte de Appui ({b}) — cible : « côté cible » ; source : « côté source »\n  2. verdict de Hypothèse ({a}) — cible : verifiee ; source : refutee\n"
        )),
        "{plan}"
    );
    assert!(plan.contains("Réunis sans conflit : 0\n"), "{plan}");
    let code = code_du_plan(&plan);
    let avant = e.texte_du_depot();
    let (status, _, err) = e.lancer(&["fusionner", &x, "--plan", &code, "--oui"]);
    assert_eq!(status, 1, "{err}");
    assert!(err.contains("2 conflit(s) non résolu(s)"), "{err}");
    for numeros in [
        &["--cible", "3"][..],
        &["--cible", "0"][..],
        &["--cible", "1", "--source", "1"][..],
        &["--cible", "1,1"][..],
        &["--cible", "un"][..],
        &["--desactiver", "1"][..],
    ] {
        let mut args = vec!["fusionner", &x, "--plan", &code, "--oui"];
        args.extend_from_slice(numeros);
        assert_eq!(e.lancer(&args).0, 2, "{numeros:?}");
    }
    assert_eq!(e.texte_du_depot(), avant);
    assert!(sauvegardes_de_fusion(&e).is_empty());
    e.ok(&[
        "fusionner",
        &x,
        "--plan",
        &code,
        "--cible",
        "1",
        "--source",
        "2",
        "--oui",
    ]);
    assert_eq!(texte_de(&e, &b), "côté cible");
    assert!(e
        .ok(&["statut"])
        .contains(&format!("Appui ({b}) — cause : Hypothèse ({a})")));
}

// INV-MERGE-03 : un conflit structurel se résout par --desactiver, numéro d'une relation
// candidate ; une fusion de la branche active dans elle-même, d'une branche abandonnée ou
// inconnue est refusée.
#[test]
fn structural_conflicts_and_merge_refusals() {
    let mut e = Essai::new("fusion-structure");
    e.ok(&["init"]);
    let p = creer(&mut e, "categorie", "Pilotes");
    let q = creer(&mut e, "categorie", "Matériel");
    let a = creer(&mut e, "raisonnement", "Hypothèse");
    let b = creer(&mut e, "implementation", "Appui");
    e.ok(&["lier", &b, "depend-de", &a]);
    let x = identifiant(
        &e.ok(&[
            "verdict",
            &a,
            "refutee",
            "--motif",
            "contre-exemple",
            "--oui",
        ]),
        "Branche créée : ",
    );
    let premiere = identifiant(&e.ok(&["lier", &q, "classe-dans", &p]), "Relation créée : ");
    e.ok(&["basculer", "00000000"]);
    let seconde = identifiant(&e.ok(&["lier", &p, "classe-dans", &q]), "Relation créée : ");
    let plan = e.ok(&["fusion", "plan", &x]);
    assert!(
        plan.contains(&format!(
            "Conflit structurel : cycle de remplacement ou de catégories\nRelations candidates :\n  1. {premiere} : Matériel ({q}) classe-dans Pilotes ({p})\n  2. {seconde} : Pilotes ({p}) classe-dans Matériel ({q})\n"
        )),
        "{plan}"
    );
    let code = code_du_plan(&plan);
    let (status, _, err) = e.lancer(&["fusionner", &x, "--plan", &code, "--oui"]);
    assert_eq!(status, 1, "{err}");
    assert!(err.contains("conflit structurel"), "{err}");
    assert_eq!(
        e.lancer(&[
            "fusionner",
            &x,
            "--plan",
            &code,
            "--desactiver",
            "3",
            "--oui"
        ])
        .0,
        2
    );
    e.ok(&[
        "fusionner",
        &x,
        "--plan",
        &code,
        "--desactiver",
        "1",
        "--oui",
    ]);
    // La relation désactivée est la première candidate ; l'autre reste active.
    let depot = decode_repository_v2(&e.texte_du_depot()).unwrap();
    let graphe = depot
        .snapshot()
        .graph(memoire_domain::DEFAULT_SCOPE)
        .unwrap();
    let active = |id: &str| {
        graphe
            .edge(memoire_domain::EdgeId::parse(id).unwrap())
            .unwrap()
            .active()
    };
    assert!(!active(&premiere));
    assert!(active(&seconde));
    // Refus.
    let (status, _, err) = e.lancer(&["fusion", "plan", "00000000"]);
    assert_eq!(status, 1, "{err}");
    assert!(err.contains("ne se fusionne pas dans elle-même"), "{err}");
    e.ok(&["abandonner", &x]);
    let (status, _, err) = e.lancer(&["fusion", "plan", &x]);
    assert_eq!(status, 1, "{err}");
    assert!(err.contains("branche abandonnée"), "{err}");
    for args in [
        &["fusion", "plan"][..],
        &["fusion", "plan", "1234567"][..],
        &["fusionner", &x][..],
        &["fusionner", "--plan", &code][..],
    ] {
        assert_eq!(e.lancer(args).0, 2, "{args:?}");
    }
}

// D18, D24 (CITATIONS.md, section 5) : « citer » cite le texte exact d'une note ou d'un échange
// et relie la citation à sa source, en une seule opération ; la citation d'une note garde sa
// version et se signale « (source révisée) » ; le rôle s'affiche « citation ».
#[test]
fn citing_a_note_or_an_exchange_links_the_citation_to_its_source() {
    let mut e = Essai::new("citer");
    e.ok(&["init"]);
    let n = identifiant(
        &e.ok(&[
            "note",
            "creer",
            "--role",
            "raisonnement",
            "--titre",
            "Invariant",
            "--texte",
            "la priorité est conservée",
        ]),
        "Note créée : ",
    );
    let out = e.ok(&["citer", &n[..8], "--texte", "priorité"]);
    let lignes: Vec<&str> = out.lines().collect();
    assert_eq!(lignes.len(), 2, "{out}");
    let citation = lignes[0]
        .strip_prefix("Citation créée : ")
        .and_then(|l| l.strip_suffix(" (octets 3 à 12)"))
        .unwrap_or_else(|| panic!("{out}"))
        .to_owned();
    let relation = lignes[1].strip_prefix("Relation créée : ").unwrap();
    assert!(
        relation.ends_with(&format!(" (developpe-a-partir-de {n})")),
        "{out}"
    );
    assert_eq!(
        e.ok(&["chercher", "priorité"]),
        format!("{citation}\tcitation\t« priorité »\n")
    );
    // Une seule opération : un « annuler » retire la citation et sa relation.
    e.ok(&["annuler"]);
    assert_eq!(e.ok(&["chercher", "priorité"]), "");
    e.ok(&["retablir"]);
    // Une note n'a pas de partie : --partie est une erreur d'usage.
    assert_eq!(
        e.lancer(&["citer", &n, "--texte", "priorité", "--partie", "question"])
            .0,
        2
    );
    // Révisée, la note citée signale la citation.
    e.ok(&[
        "note",
        "reviser",
        &n,
        "--texte",
        "la priorité change",
        "--nature",
        "mineure",
    ]);
    let statut = e.ok(&["statut"]);
    assert!(
        statut.contains(&format!("« priorité » ({citation}) (source révisée)")),
        "{statut}"
    );
    // Citer un échange : la relation va vers l'échange.
    let reponse = e.dir.join("reponse.txt");
    std::fs::write(&reponse, "le réveil peut être perdu").unwrap();
    let x = identifiant(
        &e.ok(&[
            "echange",
            "coller",
            "--titre",
            "Réveil",
            "--reponse-fichier",
            reponse.to_str().unwrap(),
        ]),
        "Échange créé : ",
    );
    let out = e.ok(&["citer", &x, "--texte", "réveil"]);
    assert!(
        out.contains(&format!(" (developpe-a-partir-de {x})\n")),
        "{out}"
    );
}

// D18, points 1, 2 et 4 ; R6 ; D24 : une note peut dépendre d'un échange ; « retirer » sort un
// échange des ressources actuelles et met ses dépendants à réexaminer ; une fois la cause
// acquittée, la note, sans autre appui, est listée parmi les orphelines de « statut ».
#[test]
fn exchanges_in_dependencies_withdrawal_and_orphans() {
    let mut e = Essai::new("orphelines");
    e.ok(&["init"]);
    let reponse = e.dir.join("reponse.txt");
    std::fs::write(&reponse, "réponse").unwrap();
    let x = identifiant(
        &e.ok(&[
            "echange",
            "coller",
            "--titre",
            "Conversation",
            "--reponse-fichier",
            reponse.to_str().unwrap(),
        ]),
        "Échange créé : ",
    );
    let a = creer(&mut e, "implementation", "Appui");
    e.ok(&["lier", &a, "depend-de", &x]);
    assert!(e.ok(&["statut"]).contains("Orphelines :\n  (aucune)\n"));
    // Un échange ne dépend pas d'une note.
    assert_eq!(e.lancer(&["lier", &x, "developpe-a-partir-de", &a]).0, 1);
    let avant = e.texte_du_depot();
    let (code, out, _) = e.lancer(&["retirer", &x, "--motif", "périmée"]);
    assert_eq!(code, 4, "{out}");
    assert_eq!(e.texte_du_depot(), avant);
    // Contrat modifié (5.5b) : l'aperçu de l'opération critique précède le compte rendu, même
    // avec --oui (CLI.md, section 5) ; « starts_with », écrit à tort, devient « contains ».
    let out = e.ok(&["retirer", &x, "--motif", "périmée", "--oui"]);
    assert!(out.contains("Retrait enregistré.\n"), "{out}");
    assert!(out.contains("Branche créée : "), "{out}");
    let statut = e.ok(&["statut"]);
    assert!(
        statut.contains(&format!("Appui ({a}) — cause : Conversation ({x})")),
        "{statut}"
    );
    e.ok(&["acquitter", &a, &x, "--motif", "vérifié ailleurs", "--oui"]);
    let statut = e.ok(&["statut"]);
    assert!(
        statut.contains(&format!(
            "À réexaminer :\n  (aucune)\nOrphelines :\n  - Appui ({a})\n"
        )),
        "{statut}"
    );
    let ressources = statut.split("À réexaminer :").next().unwrap().to_owned();
    assert!(!ressources.contains(&a), "{statut}");
    assert_eq!(e.lancer(&["retirer", "1234567"]).0, 2);
}

/// Crée une note de rôle donné avec un texte, et rend son identifiant.
fn creer_avec_texte(e: &mut Essai, role: &str, titre: &str, texte: &str) -> String {
    identifiant(
        &e.ok(&[
            "note", "creer", "--role", role, "--titre", titre, "--texte", texte,
        ]),
        "Note créée : ",
    )
}

/// Colle un échange (question et réponse depuis des fichiers de l'essai) et rend son identifiant.
fn coller(e: &mut Essai, titre: &str, question: Option<&str>, reponse: &str) -> String {
    let r = e.dir.join(format!("{titre}-reponse.txt"));
    std::fs::write(&r, reponse).unwrap();
    let mut args = vec![
        "echange".to_owned(),
        "coller".to_owned(),
        "--titre".to_owned(),
        titre.to_owned(),
        "--reponse-fichier".to_owned(),
        r.display().to_string(),
    ];
    if let Some(q) = question {
        let chemin = e.dir.join(format!("{titre}-question.txt"));
        std::fs::write(&chemin, q).unwrap();
        args.push("--question-fichier".to_owned());
        args.push(chemin.display().to_string());
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    identifiant(&e.ok(&args), "Échange créé : ")
}

// VISIONNEUR.md, section 3 (D25) : « source » rend le texte exact d'une partie d'un échange ou
// d'une version d'une note, octet pour octet, sans rien ajouter ; les arguments sans objet sont
// des erreurs d'usage.
#[test]
fn source_prints_the_exact_text() {
    let mut e = Essai::new("source");
    e.ok(&["init"]);
    let texte = "  # Titre\n**gras** é 🚀\n\n";
    let n = creer_avec_texte(&mut e, "raisonnement", "Note", texte);
    assert_eq!(e.ok(&["source", &n[..8]]), texte);
    e.ok(&[
        "note",
        "reviser",
        &n,
        "--texte",
        "version 1",
        "--nature",
        "mineure",
    ]);
    assert_eq!(e.ok(&["source", &n]), "version 1");
    assert_eq!(e.ok(&["source", &n, "--version", "0"]), texte);
    assert_eq!(e.ok(&["source", &n, "--version", "1"]), "version 1");
    let x = coller(&mut e, "Échange", Some("question ?\n"), "réponse\r\n— fin");
    assert_eq!(e.ok(&["source", &x]), "réponse\r\n— fin");
    assert_eq!(
        e.ok(&["source", &x, "--partie", "question"]),
        "question ?\n"
    );
    let incomplet = coller(&mut e, "Incomplet", None, "seule");
    for args in [
        &["source", &n, "--version", "2"][..],
        &["source", &n, "--version", "un"][..],
        &["source", &n, "--partie", "question"][..],
        &["source", &x, "--version", "0"][..],
        &["source", &x, "--partie", "corps"][..],
        &["source", &incomplet, "--partie", "question"][..],
        &["source"][..],
        &["source", "1234567"][..],
    ] {
        assert_eq!(e.lancer(args).0, 2, "{args:?}");
    }
}

// VISIONNEUR.md, section 3 : « citations » liste, pour une source (partie d'échange ou version de
// note), chaque citation : identifiant, début et fin en octets, statut, et « source-revisee »
// pour une note révisée depuis ; par début croissant.
#[test]
fn citations_list_the_cited_ranges_of_a_source() {
    let mut e = Essai::new("citations");
    e.ok(&["init"]);
    let n = creer_avec_texte(
        &mut e,
        "raisonnement",
        "Invariant",
        "la priorité est conservée",
    );
    let premiere = identifiant(
        &e.ok(&["citer", &n, "--texte", "priorité"]),
        "Citation créée : ",
    );
    let premiere = premiere.split(' ').next().unwrap().to_owned();
    let seconde = identifiant(&e.ok(&["citer", &n, "--plage", "0-2"]), "Citation créée : ");
    assert!(seconde.ends_with(" (octets 0 à 2)"), "{seconde}");
    let seconde = seconde.split(' ').next().unwrap().to_owned();
    assert_eq!(
        e.ok(&["citations", &n]),
        format!("{seconde}\t0\t2\tnon-verifiee\n{premiere}\t3\t12\tnon-verifiee\n")
    );
    e.ok(&["verdict", &premiere, "verifiee", "--motif", "relu"]);
    e.ok(&[
        "note",
        "reviser",
        &n,
        "--texte",
        "autre texte",
        "--nature",
        "mineure",
    ]);
    // La version courante n'est citée par personne ; la version 0 l'est, et a été révisée depuis.
    assert_eq!(e.ok(&["citations", &n]), "");
    assert_eq!(
        e.ok(&["citations", &n, "--version", "0"]),
        format!(
            "{seconde}\t0\t2\tnon-verifiee,source-revisee\n{premiere}\t3\t12\tverifiee,source-revisee\n"
        )
    );
    let x = coller(&mut e, "Échange", Some("la question"), "le réveil perdu");
    let p = identifiant(
        &e.ok(&["citer", &x, "--texte", "réveil"]),
        "Citation créée : ",
    );
    let p = p.split(' ').next().unwrap().to_owned();
    assert_eq!(
        e.ok(&["citations", &x]),
        format!("{p}\t3\t10\tnon-verifiee\n")
    );
    assert_eq!(e.ok(&["citations", &x, "--partie", "question"]), "");
    assert_eq!(e.lancer(&["citations", &x, "--version", "0"]).0, 2);
    assert_eq!(e.lancer(&["citations", &n, "--partie", "reponse"]).0, 2);
}

// VISIONNEUR.md, section 3 : « citer » prend aussi une plage en octets (--plage A-B, fin
// exclue), et une version d'une note (--version N, la courante par défaut) ; --texte et --plage
// s'excluent ; une plage qui coupe un caractère ou sort du texte est un refus du domaine.
#[test]
fn citing_by_byte_range_and_version() {
    let mut e = Essai::new("citer-plage");
    e.ok(&["init"]);
    let n = creer_avec_texte(
        &mut e,
        "raisonnement",
        "Invariant",
        "la priorité est conservée",
    );
    let out = e.ok(&["citer", &n, "--plage", "3-12"]);
    assert!(out.contains(" (octets 3 à 12)\nRelation créée : "), "{out}");
    assert!(e
        .ok(&["chercher", "priorité"])
        .contains("\tcitation\t« priorité »"));
    let avant = e.texte_du_depot();
    for plage in ["3-11", "12-3", "3-99"] {
        let (code, _, err) = e.lancer(&["citer", &n, "--plage", plage]);
        assert_eq!(code, 1, "{plage} : {err}");
    }
    for args in [
        &["citer", &n, "--plage", "trois-douze"][..],
        &["citer", &n, "--plage", "3"][..],
        &["citer", &n, "--plage", "3-12", "--texte", "priorité"][..],
        &["citer", &n][..],
        &["citer", &n, "--plage", "3-12", "--occurrence", "1"][..],
    ] {
        assert_eq!(e.lancer(args).0, 2, "{args:?}");
    }
    assert_eq!(e.texte_du_depot(), avant);
    // Une version ancienne se cite, par plage comme par texte.
    e.ok(&[
        "note",
        "reviser",
        &n,
        "--texte",
        "tout autre chose",
        "--nature",
        "mineure",
    ]);
    assert_eq!(e.lancer(&["citer", &n, "--texte", "priorité"]).0, 2);
    let out = e.ok(&["citer", &n, "--version", "0", "--texte", "conservée"]);
    assert!(out.contains(" (octets 17 à 27)\n"), "{out}");
    assert_eq!(
        e.ok(&["citations", &n, "--version", "0"]).lines().count(),
        2
    );
    assert_eq!(
        e.lancer(&["citer", &n, "--version", "3", "--plage", "0-2"])
            .0,
        2
    );
}

/// Identifiant en tête de la partie d'une ligne qui suit `debut`.
fn premier_mot(ligne: &str, debut: &str) -> String {
    ligne
        .strip_prefix(debut)
        .unwrap_or_else(|| panic!("« {debut} » absent de : {ligne}"))
        .split(' ')
        .next()
        .unwrap()
        .to_owned()
}

// INV-WORK-01 (CENTRE.md, sections 1 et 3) : « citer » cite, relie à la source, relie aux notes
// demandées, crée une note et une réserve, en une seule opération qu'un seul « annuler » défait ;
// une ligne de compte rendu par objet, dans l'ordre du lot.
#[test]
fn citing_and_linking_in_one_gesture() {
    let mut e = Essai::new("centre");
    e.ok(&["init"]);
    let soutenue = creer_avec_texte(&mut e, "raisonnement", "Hypothèse", "un appui");
    let nee = creer_avec_texte(&mut e, "implementation", "Gestionnaire", "un choix");
    let categorie = creer(&mut e, "categorie", "Pilotes");
    let x = coller(&mut e, "Réveil", Some("q"), "le réveil peut être perdu");
    let out = e.ok(&[
        "citer",
        &x,
        "--texte",
        "réveil",
        "--soutient",
        &soutenue[..8],
        "--inspire",
        &nee,
        "--classe-dans",
        &categorie,
        "--lien",
        &soutenue,
        "--note",
        "Idée née du réveil",
        "--role",
        "implementation",
        "--texte-note",
        "à explorer",
        "--reserve",
        "Vérifier le réveil",
        "--cible",
        "startup.s",
    ]);
    let lignes: Vec<&str> = out.lines().collect();
    assert_eq!(lignes.len(), 9, "{out}");
    let citation = premier_mot(lignes[0], "Citation créée : ");
    assert!(lignes[0].ends_with(" (octets 3 à 10)"), "{out}");
    let note = premier_mot(lignes[6], "Note créée : ");
    let attendues = [
        (1, format!(" (developpe-a-partir-de {x})")),
        (2, format!(" ({soutenue} depend-de {citation})")),
        (3, format!(" ({nee} developpe-a-partir-de {citation})")),
        (4, format!(" (classe-dans {categorie})")),
        (5, format!(" (lien {soutenue})")),
        (7, format!(" (developpe-a-partir-de {citation})")),
    ];
    for (rang, fin) in attendues {
        assert!(lignes[rang].starts_with("Relation créée : "), "{out}");
        assert!(lignes[rang].ends_with(&fin), "{rang} : {out}");
    }
    let reserve = premier_mot(lignes[8], "Réserve créée : ");
    assert!(e
        .ok(&["chercher", "Idée née"])
        .contains(&format!("{note}\timplementation\t")));
    assert!(e.ok(&["chercher", "Vérifier"]).contains(&reserve));
    assert_eq!(texte_de(&e, &note), "à explorer");
    // Une seule opération : un « annuler » défait tout. Contrat modifié (6.2a) : « réveil » trouve
    // aussi l'échange « Réveil », qui reste ; seule la citation doit disparaître.
    e.ok(&["annuler"]);
    assert_eq!(
        e.ok(&["chercher", "réveil"]),
        format!("{x}\techange\tRéveil\n")
    );
    for nom in ["Idée née", "Vérifier"] {
        assert_eq!(e.ok(&["chercher", nom]), "", "{nom}");
    }
    e.ok(&["retablir"]);
    assert!(e.ok(&["chercher", "Idée née"]).contains(&note));
    // Sans option, la citation seule, comme avant.
    let seule = e.ok(&["citer", &x, "--texte", "perdu"]);
    assert_eq!(seule.lines().count(), 2, "{seule}");
}

// CENTRE.md, section 2 : le refus d'un élément refuse tout le lot et le nomme ; un lot critique
// demande une confirmation, une fois ; les options sans objet et les listes mal formées sont des
// erreurs d'usage, sans rien écrire.
#[test]
fn citing_and_linking_refusals() {
    let mut e = Essai::new("centre-refus");
    e.ok(&["init"]);
    let n = creer_avec_texte(&mut e, "raisonnement", "Hypothèse", "un appui fragile");
    let categorie = creer(&mut e, "categorie", "Pilotes");
    let x = coller(&mut e, "Réveil", Some("q"), "le réveil peut être perdu");
    let avant = e.texte_du_depot();
    // Une catégorie ne dépend de rien : troisième élément du lot.
    let (code, _, err) = e.lancer(&["citer", &x, "--texte", "réveil", "--soutient", &categorie]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("élément 3 du lot"), "{err}");
    // Une réserve ne naît que d'une citation d'échange (D24).
    let (code, _, err) = e.lancer(&["citer", &n, "--texte", "appui", "--reserve", "Doute"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("élément 3 du lot"), "{err}");
    for args in [
        &[
            "citer",
            &x,
            "--texte",
            "réveil",
            "--soutient",
            &format!("{n},{n}"),
        ][..],
        &["citer", &x, "--texte", "réveil", "--inspire", "1234567"][..],
        &["citer", &x, "--texte", "réveil", "--lien", "abcdefgh"][..],
        &["citer", &x, "--texte", "réveil", "--role", "implementation"][..],
        &["citer", &x, "--texte", "réveil", "--texte-note", "t"][..],
        &["citer", &x, "--texte", "réveil", "--cible", "c"][..],
        &[
            "citer", &x, "--texte", "réveil", "--note", "t", "--role", "echange",
        ][..],
    ] {
        assert_eq!(e.lancer(args).0, 2, "{args:?}");
    }
    assert_eq!(e.texte_du_depot(), avant);
    // Citer une note réfutée crée une cause de réexamen : lot critique, confirmé une fois.
    e.ok(&[
        "verdict",
        &n,
        "refutee",
        "--motif",
        "contre-exemple",
        "--oui",
    ]);
    let avant = e.texte_du_depot();
    let commande = ["citer", &n, "--texte", "fragile", "--note", "Reprise"];
    let (code, out, _) = e.lancer(&commande);
    assert_eq!(code, 4, "{out}");
    assert_eq!(e.texte_du_depot(), avant);
    let mut confirmee = commande.to_vec();
    confirmee.push("--oui");
    let out = e.ok(&confirmee);
    assert!(out.contains("Note créée : "), "{out}");
    assert!(!out.contains("Branche créée"), "{out}");
}

/// Écrit un fichier sous la racine du code d'essai, dossiers compris.
fn ecrire(racine: &Path, chemin: &str, contenu: &[u8]) {
    let cible = racine.join(chemin);
    std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
    std::fs::write(cible, contenu).unwrap();
}

/// Contenu de tous les fichiers sous un dossier, par chemin, pour constater qu'aucun ne change.
fn contenus(racine: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut tout = Vec::new();
    let mut pile = vec![racine.to_path_buf()];
    while let Some(dossier) = pile.pop() {
        for entree in std::fs::read_dir(&dossier).unwrap() {
            let chemin = entree.unwrap().path();
            if chemin.is_dir() {
                pile.push(chemin);
            } else {
                let octets = std::fs::read(&chemin).unwrap();
                tout.push((chemin, octets));
            }
        }
    }
    tout.sort();
    tout
}

// INDEXEUR.md, sections 1 et 4 (INV-CODE-01, INV-CODE-04, INV-CODE-05, INV-CODE-06 ; CA-05,
// CA-30, CA-31, S-CODE-02) : « indexer » parcourt le code, saute les dossiers cachés, « target »
// et les fichiers illisibles, et rend un rapport exact, section par section ; il n'écrit ni dans
// le code ni dans le dépôt, et deux indexations rendent le même rapport.
#[test]
fn indexing_reports_the_state_of_every_marker_and_anchor() {
    let mut e = Essai::new("indexer");
    e.ok(&["init"]);
    let code = e.dir.join("code");
    let a = creer(&mut e, "implementation", "A");
    let b = creer(&mut e, "implementation", "B");
    let c = creer(&mut e, "implementation", "C");
    let d = creer(&mut e, "implementation", "D");
    let l = creer(&mut e, "implementation", "L");
    let n = creer(&mut e, "implementation", "N");
    ecrire(
        &code,
        "src/driver.c",
        format!("void a() {{}}\nvoid b() {{}}\n// @design: {a}\n// @design: {b}\n").as_bytes(),
    );
    ecrire(&code, "src/engine.c", b"void c() {}\n");
    ecrire(&code, "src/old.c", b"void l() {}\n");
    ecrire(&code, "src/y.c", b"int y;\n");
    let chemin = |nom: &str| code.join(nom).display().to_string();
    for (note, fichier, lignes) in [
        (&a, "src/driver.c", "1-1"),
        (&b, "src/driver.c", "2-2"),
        (&c, "src/engine.c", "1-1"),
        (&l, "src/old.c", "1-1"),
        (&d, "src/y.c", "1-1"),
    ] {
        e.ok(&[
            "ancrer",
            note,
            "--fichier",
            &chemin(fichier),
            "--lignes",
            lignes,
        ]);
    }
    // Ensuite le code évolue : B modifiée, engine.c renommé (S-CODE-02), old.c supprimé, D en deux
    // endroits, un repère inconnu, un repère sans ancrage, un identifiant mal formé.
    let fantome = "aaaaaaaa-0000-4000-8000-000000000000";
    ecrire(
        &code,
        "src/driver.c",
        format!("void a() {{}}\nvoid b(int) {{}}\n// @design: {a}\n// @design: {b}\n").as_bytes(),
    );
    std::fs::remove_file(code.join("src/engine.c")).unwrap();
    ecrire(
        &code,
        "src/kernel.c",
        format!("void c() {{}}\n// @design: {c}\n// @design: {d}\n").as_bytes(),
    );
    std::fs::remove_file(code.join("src/old.c")).unwrap();
    ecrire(
        &code,
        "src/x.s",
        format!("; @design: {fantome}\n").as_bytes(),
    );
    ecrire(
        &code,
        "src/y.c",
        format!("// @design: {n}\n// @design: pas-un-id\n/* @design: {d} */\n").as_bytes(),
    );
    ecrire(
        &code,
        ".git/cache.c",
        format!("// @design: {a}\n").as_bytes(),
    );
    ecrire(&code, "target/t.c", format!("// @design: {a}\n").as_bytes());
    ecrire(&code, "bin.dat", &[0xff, 0xfe, 0x00, 0x41]);
    let avant = e.texte_du_depot();
    let fichiers = contenus(&code);
    let rapport = e.ok(&["indexer", &code.display().to_string()]);
    assert_eq!(
        rapport,
        format!(
            "Repères : 8 dans 4 fichiers (fichiers lus : 4, ignorés : 1)\n\
             Retrouvés :\n  - A ({a}) — src/driver.c:3\n\
             Modifiés (le code ancré a changé) :\n  - B ({b}) — src/driver.c:4 — extrait ancré absent de src/driver.c\n\
             Déplacés (chemin de l'ancrage périmé) :\n  - C ({c}) — src/kernel.c:2 — ancré dans src/engine.c, extrait retrouvé\n\
             Ambigus (plusieurs occurrences, aucune choisie) :\n  - D ({d}) — src/kernel.c:3, src/y.c:3\n\
             Perdus (ancrage sans repère dans le code) :\n  - L ({l}) — ancré dans src/old.c\n\
             Inconnus (repère sans note dans cette branche) :\n  - {fantome} — src/x.s:1\n\
             Non ancrés (repère d'une note sans ancrage) :\n  - N ({n}) — src/y.c:1\n\
             Mal formés (identifiant illisible après « @design: ») :\n  - src/y.c:2\n"
        )
    );
    // Lecture seule (INV-CODE-04, INV-CODE-06) ; même rapport la seconde fois.
    assert_eq!(e.texte_du_depot(), avant);
    assert_eq!(contenus(&code), fichiers);
    assert_eq!(e.ok(&["indexer", &code.display().to_string()]), rapport);
    // --extensions restreint les fichiers lus ; un fichier écarté n'est pas « ignoré ».
    let restreint = e.ok(&[
        "indexer",
        &code.display().to_string(),
        "--extensions",
        "c,h",
    ]);
    assert!(
        restreint.starts_with("Repères : 7 dans 3 fichiers (fichiers lus : 3, ignorés : 0)\n"),
        "{restreint}"
    );
    assert!(restreint.contains("Inconnus (repère sans note dans cette branche) :\n  (aucun)\n"));
    // Un repère inconnu de la branche active indique les branches où sa note existe (ajouté après
    // le code, INDEXEUR.md, section 3).
    let w = creer(&mut e, "raisonnement", "W");
    let branche = identifiant(
        &e.ok(&["verdict", &w, "refutee", "--motif", "réfutée", "--oui"]),
        "Branche créée : ",
    );
    let seule = creer(&mut e, "raisonnement", "Seule sur la branche");
    e.ok(&["basculer", "00000000"]);
    ecrire(
        &code,
        "src/w.c",
        format!("// @design: {seule}\n").as_bytes(),
    );
    let rapport = e.ok(&["indexer", &code.display().to_string()]);
    assert!(
        rapport.contains(&format!(
            "  - {seule} — src/w.c:1 — présente sur la branche {branche}\n"
        )),
        "{rapport}"
    );
    // DOSSIER doit être un dossier.
    assert_eq!(e.lancer(&["indexer", &chemin("src/driver.c")]).0, 2);
    assert_eq!(e.lancer(&["indexer", &chemin("absent")]).0, 2);
    assert_eq!(e.lancer(&["indexer"]).0, 2);
}

// INDEXEUR.md, section 4 : sont ignorés, et comptés comme tels, les fichiers de plus de 4 Mo, les
// liens symboliques (jamais suivis) et les dossiers illisibles ; un gros fichier sous la limite
// est lu (ajouté après le code, pour éliminer des survivants de la mutation).
#[test]
fn indexing_skips_what_it_cannot_read() {
    use std::os::unix::fs::PermissionsExt;
    let mut e = Essai::new("indexer-ignores");
    e.ok(&["init"]);
    let a = creer(&mut e, "implementation", "A");
    let code = e.dir.join("code");
    ecrire(&code, "ok.c", format!("// @design: {a}\n").as_bytes());
    let mut gros = vec![b' '; 3 * 1024 * 1024];
    gros.extend_from_slice(format!("\n// @design: {a}\n").as_bytes());
    ecrire(&code, "gros.c", &gros);
    ecrire(&code, "trop-gros.c", &vec![b' '; 4 * 1024 * 1024 + 1]);
    std::os::unix::fs::symlink(code.join("ok.c"), code.join("lien.c")).unwrap();
    ecrire(&code, "prive/cache.c", b"int x;\n");
    let prive = code.join("prive");
    std::fs::set_permissions(&prive, std::fs::Permissions::from_mode(0o000)).unwrap();
    // Un dossier sans droits reste lisible pour le superutilisateur : le compte en dépend.
    let illisible = std::fs::read_dir(&prive).is_err();
    let rapport = e.ok(&["indexer", &code.display().to_string()]);
    std::fs::set_permissions(&prive, std::fs::Permissions::from_mode(0o755)).unwrap();
    let attendu = if illisible {
        "Repères : 2 dans 2 fichiers (fichiers lus : 2, ignorés : 3)\n"
    } else {
        "Repères : 2 dans 2 fichiers (fichiers lus : 3, ignorés : 2)\n"
    };
    assert!(rapport.starts_with(attendu), "{rapport}");
    assert!(
        rapport.contains("Non ancrés (repère d'une note sans ancrage) :\n  - A ("),
        "{rapport}"
    );
}

/// Commits listés par « historique », du plus récent au plus ancien.
fn commits(e: &mut Essai) -> Vec<String> {
    e.ok(&["historique", "--nombre", "100"])
        .lines()
        .map(|l| l.split(' ').next().unwrap().to_owned())
        .collect()
}

// RESTAURATION.md, section 3 : « historique » liste les commits de la branche active, du plus
// récent au plus ancien, avec leur commande et leur motif ; la racine en dernier.
#[test]
fn the_history_lists_the_commits_of_the_active_branch() {
    let mut e = Essai::new("historique");
    e.ok(&["init"]);
    let a = creer(&mut e, "raisonnement", "Idée");
    e.ok(&["verdict", &a, "verifiee", "--motif", "relu"]);
    let lignes: Vec<String> = e.ok(&["historique"]).lines().map(str::to_owned).collect();
    assert_eq!(lignes.len(), 3, "{lignes:?}");
    assert!(lignes[0].ends_with(" — verdict — relu"), "{lignes:?}");
    assert!(
        lignes[1].ends_with(" — creer-note — (sans motif)"),
        "{lignes:?}"
    );
    assert_eq!(lignes[2], "00000000-0000-0000-0000-000000000000 — racine");
    assert_eq!(lignes[0].split(' ').next().unwrap().len(), 36);
    assert_eq!(e.ok(&["historique", "--nombre", "1"]).lines().count(), 1);
    for nombre in ["0", "un", "-1"] {
        assert_eq!(
            e.lancer(&["historique", "--nombre", nombre]).0,
            2,
            "{nombre}"
        );
    }
}

// CA-27, INV-RESTORE-01, INV-RESTORE-02, CA-33 : une idée écartée est restaurée avec ses liens
// sans toucher le travail fait depuis ; le plan dit ce qui sera rétabli et ce qui est conservé ;
// la restauration est critique (--oui) ; un « annuler » la défait ; sans --notes, tout le réseau
// est restauré, les notes créées depuis restant présentes.
#[test]
fn restoring_an_idea_without_the_rest() {
    let mut e = Essai::new("restaurer");
    e.ok(&["init"]);
    let idee = creer(&mut e, "raisonnement", "Idée");
    let appui = creer(&mut e, "implementation", "Appui");
    e.ok(&["lier", &appui, "depend-de", &idee]);
    let cible = commits(&mut e)[0].clone();
    e.ok(&[
        "decision",
        &idee,
        "plus-retenue",
        "--motif",
        "abandonnée",
        "--oui",
    ]);
    let tard = creer(&mut e, "implementation", "Venue après");
    // Contrat modifié (6.4b) : dépendre d'une idée écartée crée une cause de réexamen, l'opération
    // est critique : --oui, oublié à l'écriture.
    e.ok(&["lier", &tard, "depend-de", &idee, "--oui"]);
    let plan = e.ok(&[
        "restauration",
        "plan",
        "--depuis",
        &cible[..8],
        "--notes",
        &format!("{idee},{appui}"),
    ]);
    assert!(
        plan.starts_with(&format!(
            "Restauration ciblée de {cible} : 2 notes\nRétabli :\n"
        )),
        "{plan}"
    );
    assert!(
        plan.contains(&format!(
            "  - decision de Idée ({idee}) : plus-retenue → proposee\n"
        )),
        "{plan}"
    );
    assert!(
        plan.contains(
            "Conservé (relations avec le reste du réseau, différentes depuis) :\n  - relation "
        ),
        "{plan}"
    );
    assert!(
        plan.contains(&format!("Venue après ({tard}) depend-de Idée ({idee})")),
        "{plan}"
    );
    let avant = e.texte_du_depot();
    let commande = [
        "restaurer",
        "--depuis",
        &cible,
        "--notes",
        &format!("{idee},{appui}"),
    ];
    let (code, out, _) = e.lancer(&commande);
    assert_eq!(code, 4, "{out}");
    assert_eq!(e.texte_du_depot(), avant);
    let mut confirmee = commande.to_vec();
    confirmee.push("--oui");
    let out = e.ok(&confirmee);
    assert!(out.contains("Restauration enregistrée : "), "{out}");
    let statut = e.ok(&["statut"]);
    let ressources = statut.split("À réexaminer :").next().unwrap().to_owned();
    assert!(ressources.contains(&format!("Idée ({idee})")), "{statut}");
    assert!(
        ressources.contains(&format!("Venue après ({tard})")),
        "{statut}"
    );
    // Un seul « annuler » défait la restauration (CA-33).
    e.ok(&["annuler"]);
    let statut = e.ok(&["statut"]);
    let ressources = statut.split("À réexaminer :").next().unwrap().to_owned();
    assert!(!ressources.contains(&format!("Idée ({idee})")), "{statut}");
    // Restauration complète : la note venue après reste présente, retirée des ressources actuelles.
    let plan = e.ok(&["restauration", "plan", "--depuis", &cible]);
    assert!(
        plan.starts_with(&format!("Restauration complète de {cible}\n")),
        "{plan}"
    );
    e.ok(&["restaurer", "--depuis", &cible, "--oui"]);
    let statut = e.ok(&["statut"]);
    let ressources = statut.split("À réexaminer :").next().unwrap().to_owned();
    assert!(ressources.contains(&format!("Idée ({idee})")), "{statut}");
    assert!(
        !ressources.contains(&format!("Venue après ({tard})")),
        "{statut}"
    );
    assert!(e.ok(&["chercher", "Venue"]).contains(&tard));
}

// RESTAURATION.md, sections 2 et 4 : commit inconnu, préfixe trop court, sélection vide ou mal
// formée : erreurs d'usage ; note créée après le commit : refus du domaine ; rien n'est écrit.
#[test]
fn restore_refusals() {
    let mut e = Essai::new("restaurer-refus");
    e.ok(&["init"]);
    let idee = creer(&mut e, "raisonnement", "Idée");
    let cible = commits(&mut e)[0].clone();
    let tard = creer(&mut e, "raisonnement", "Venue après");
    let avant = e.texte_du_depot();
    for args in [
        &["restaurer", "--depuis", "1234567", "--oui"][..],
        &["restaurer", "--depuis", "abcdef01", "--oui"][..],
        &["restaurer", "--notes", &idee, "--oui"][..],
        &["restaurer", "--depuis", &cible, "--notes", "", "--oui"][..],
        &[
            "restaurer",
            "--depuis",
            &cible,
            "--notes",
            &format!("{idee},{idee}"),
            "--oui",
        ][..],
        &["restauration", "plan"][..],
    ] {
        assert_eq!(e.lancer(args).0, 2, "{args:?}");
    }
    let (code, _, err) = e.lancer(&["restaurer", "--depuis", &cible, "--notes", &tard, "--oui"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("note absente de l'état à restaurer"), "{err}");
    assert_eq!(e.texte_du_depot(), avant);
}

// RESTAURATION.md, section 1 : le plan dit si une relation interne sera réactivée ou désactivée
// (ajouté après le code, pour éliminer des survivants de la mutation).
#[test]
fn the_restore_plan_says_how_internal_relations_change() {
    let mut e = Essai::new("restauration-relations");
    e.ok(&["init"]);
    let idee = creer(&mut e, "raisonnement", "Idée");
    let appui = creer(&mut e, "implementation", "Appui");
    let avant_lien = commits(&mut e)[0].clone();
    let lien = identifiant(
        &e.ok(&["lier", &appui, "depend-de", &idee]),
        "Relation créée : ",
    );
    let avec_lien = commits(&mut e)[0].clone();
    let selection = format!("{idee},{appui}");
    // Depuis « avant_lien », la relation interne a été créée : elle serait désactivée.
    let plan = e.ok(&[
        "restauration",
        "plan",
        "--depuis",
        &avant_lien,
        "--notes",
        &selection,
    ]);
    assert!(
        plan.contains(&format!(
            "  - relation {lien} : Appui ({appui}) depend-de Idée ({idee}) — désactivée\n"
        )),
        "{plan}"
    );
    // Une restauration complète vers « avant_lien » la désactive ; depuis « avec_lien », elle
    // serait réactivée.
    e.ok(&["restaurer", "--depuis", &avant_lien, "--oui"]);
    let plan = e.ok(&[
        "restauration",
        "plan",
        "--depuis",
        &avec_lien,
        "--notes",
        &selection,
    ]);
    assert!(
        plan.contains(&format!(
            "  - relation {lien} : Appui ({appui}) depend-de Idée ({idee}) — réactivée\n"
        )),
        "{plan}"
    );
}

// ---------------------------------------------------------------------------
// Incrément 6.5b — fiche de reprise (FICHE-REPRISE.md, D26 ; INV-RESUME-01, INV-RESUME-02 ; CA-08).
// Les essais créent un dépôt Git dans le dossier de l'essai : ils demandent l'exécutable git.
// ---------------------------------------------------------------------------

/// Git des essais, isolé de la configuration de l'utilisateur ; échoue si la commande échoue.
fn git(dossier: &Path, args: &[&str]) -> String {
    let sortie = Process::new("git")
        .arg("-C")
        .arg(dossier)
        .args([
            "-c",
            "user.name=Essai",
            "-c",
            "user.email=essai@example.invalid",
            "-c",
            "init.defaultBranch=principale",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        sortie.status.success(),
        "git {args:?} : {}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    String::from_utf8(sortie.stdout).unwrap()
}

/// Dépôt Git d'essai dans `code/` : un commit, puis un fichier modifié et un fichier non suivi.
/// Rend le dossier et le commit de tête.
fn depot_git(e: &Essai) -> (PathBuf, String) {
    let code = e.dir.join("code");
    std::fs::create_dir_all(code.join("src")).unwrap();
    git(&code, &["init", "-q"]);
    std::fs::write(code.join("src/a.c"), "int a;\n").unwrap();
    git(&code, &["add", "."]);
    git(&code, &["commit", "-q", "-m", "départ"]);
    std::fs::write(code.join("src/a.c"), "int a = 1;\n").unwrap();
    std::fs::write(code.join("b.c"), "int b;\n").unwrap();
    let tete = git(&code, &["rev-parse", "HEAD"]).trim().to_owned();
    (code, tete)
}

/// Contenu de chaque fichier d'une arborescence, `.git` compris, par chemin.
fn arborescence(dossier: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut fichiers = std::collections::BTreeMap::new();
    let mut a_lire = vec![dossier.to_path_buf()];
    while let Some(d) = a_lire.pop() {
        for entree in std::fs::read_dir(&d).unwrap() {
            let chemin = entree.unwrap().path();
            if chemin.is_dir() {
                a_lire.push(chemin);
            } else {
                fichiers.insert(chemin.clone(), std::fs::read(&chemin).unwrap());
            }
        }
    }
    fichiers
}

/// Date d'une fiche : « AAAA-MM-JJ HH:MM UTC ».
fn date_valide(date: &str) -> bool {
    let b = date.as_bytes();
    date.len() == 20
        && date.ends_with(" UTC")
        && [4, 7].iter().all(|&i| b[i] == b'-')
        && b[10] == b' '
        && b[13] == b':'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15]
            .iter()
            .all(|&i| b[i].is_ascii_digit())
}

/// Sortie de « reprise voir » : identifiant et date de l'en-tête vérifiés, puis le reste.
fn fiche_vue(sortie: &str, id: &str) -> String {
    let (entete, reste) = sortie.split_once('\n').unwrap();
    let date = entete
        .strip_prefix(&format!("Point de reprise {id} — "))
        .unwrap_or_else(|| panic!("{sortie}"));
    assert!(date_valide(date), "{entete}");
    reste.to_owned()
}

/// Note un point de reprise et rend son identifiant et la ligne « Code : … ».
fn noter(e: &mut Essai, args: &[&str]) -> (String, String) {
    let mut tout = vec!["reprise", "noter"];
    tout.extend_from_slice(args);
    let sortie = e.ok(&tout);
    let mut lignes = sortie.lines();
    let id = lignes
        .next()
        .and_then(|l| l.strip_prefix("Point de reprise enregistré : "))
        .unwrap_or_else(|| panic!("{sortie}"))
        .to_owned();
    let code = lignes.next().unwrap_or_default().to_owned();
    assert_eq!(lignes.next(), None, "{sortie}");
    (id, code)
}

// CA-08, INV-RESUME-01, INV-RESUME-02 : une fiche note objectif, question, prochaine action, notes
// et l'état du code (commit, branche, fichiers changés, travail non enregistré) ; chaque commande
// rouvre la base, et la fiche se relit à l'identique, en tête l'objectif, la question et la
// prochaine action ; le réseau ne change pas ; ni le code ni le dépôt Git ne sont modifiés.
#[test]
fn a_resume_point_is_noted_with_the_state_of_the_code() {
    let mut e = Essai::new("reprise");
    e.ok(&["init"]);
    let n = creer(&mut e, "raisonnement", "Réveil perdu");
    let (code, tete) = depot_git(&e);
    let avant_code = arborescence(&code);
    let avant_reseau = e.ok(&["statut"]);
    let dossier = code.display().to_string();
    let (id, ligne) = noter(
        &mut e,
        &[
            "--objectif",
            "isoler le réveil perdu",
            "--action",
            "instrumenter wake_task",
            "--question",
            "le tick peut-il réveiller une tâche prête ?",
            "--resultat",
            "test_wakeup échoue 1 fois sur 20",
            "--notes",
            &n,
            "--code",
            &dossier,
        ],
    );
    let resume_du_code = format!(
        "{} sur principale ; 2 fichiers changés dont 1 non suivi (listé seulement) ; travail non \
         enregistré conservé",
        &tete[..8]
    );
    assert_eq!(ligne, format!("Code : {resume_du_code}"));
    assert_eq!(arborescence(&code), avant_code, "code ou dépôt Git modifié");
    assert_eq!(e.ok(&["statut"]), avant_reseau, "réseau modifié");
    let vue = e.ok(&["reprise", "voir", "--code", &dossier]);
    assert_eq!(
        fiche_vue(&vue, &id),
        format!(
            "Objectif : isoler le réveil perdu\n\
             Question : le tick peut-il réveiller une tâche prête ?\n\
             Prochaine action : instrumenter wake_task\n\
             État atteint : —\n\
             Dernier résultat : test_wakeup échoue 1 fois sur 20\n\
             Hypothèse : —\n\
             Ouverts : —\n\
             Notes : Réveil perdu ({n})\n\
             Code noté : {resume_du_code} (memoire reprise code {})\n\
             Depuis : réseau, 0 opération ; code inchangé\n",
            &id[..8]
        )
    );
    assert_eq!(arborescence(&code), avant_code, "code ou dépôt Git modifié");
    // La fiche figure dans l'historique ; « voir » sans --depuis montre la dernière.
    assert!(e
        .ok(&["historique", "--nombre", "1"])
        .starts_with(&format!("{id} — reprise — ")));
    let par_id = e.ok(&["reprise", "voir", "--depuis", &id[..8], "--code", &dossier]);
    assert_eq!(par_id, vue);
    // Le travail non enregistré noté se réapplique sur une copie propre du commit noté.
    let patch = e.ok(&["reprise", "code", &id]);
    assert_eq!(patch, git(&code, &["diff", "HEAD", "--binary"]));
    let copie = e.dir.join("copie");
    git(
        &e.dir,
        &["clone", "-q", &dossier, &copie.display().to_string()],
    );
    std::fs::write(e.dir.join("travail.patch"), &patch).unwrap();
    git(
        &copie,
        &["apply", &e.dir.join("travail.patch").display().to_string()],
    );
    assert_eq!(
        std::fs::read_to_string(copie.join("src/a.c")).unwrap(),
        "int a = 1;\n"
    );
}

// INV-RESUME-02 : à la lecture, un nouveau commit et un travail non enregistré différent sont
// signalés ; les opérations du réseau depuis la fiche sont comptées. « annuler » juste après une
// fiche dit qu'il défait la fiche ; « restaurer --depuis » une fiche revient à son état du réseau.
#[test]
fn the_resume_point_says_what_changed_since() {
    let mut e = Essai::new("reprise-depuis");
    e.ok(&["init"]);
    let (code, tete) = depot_git(&e);
    let dossier = code.display().to_string();
    let (id, _) = noter(
        &mut e,
        &["--objectif", "o", "--action", "a", "--code", &dossier],
    );
    let depuis = |e: &mut Essai| -> String {
        let vue = e.ok(&["reprise", "voir", "--code", &dossier]);
        vue.lines().last().unwrap().to_owned()
    };
    assert_eq!(
        depuis(&mut e),
        "Depuis : réseau, 0 opération ; code inchangé"
    );
    std::fs::write(code.join("b.c"), "int b = 2;\n").unwrap();
    // Le contenu d'un fichier non suivi n'est pas noté : seule sa présence l'est.
    assert_eq!(
        depuis(&mut e),
        "Depuis : réseau, 0 opération ; code inchangé"
    );
    std::fs::write(code.join("src/a.c"), "int a = 2;\n").unwrap();
    assert_eq!(
        depuis(&mut e),
        "Depuis : réseau, 0 opération ; code, travail non enregistré différent"
    );
    git(&code, &["commit", "-q", "-am", "suite"]);
    let nouvelle = git(&code, &["rev-parse", "HEAD"]).trim().to_owned();
    creer(&mut e, "note", "après la fiche");
    creer(&mut e, "note", "encore après");
    assert_eq!(
        depuis(&mut e),
        format!(
            "Depuis : réseau, 2 opérations ; code, 1 commit depuis ({} → {}), travail non \
             enregistré différent",
            &tete[..8],
            &nouvelle[..8]
        )
    );
    // Un commit qui ne descend pas du commit noté (historique réécrit) est signalé comme tel.
    git(&code, &["checkout", "-q", "--orphan", "autre"]);
    git(&code, &["commit", "-q", "-m", "sans lien"]);
    let orpheline = git(&code, &["rev-parse", "HEAD"]).trim().to_owned();
    assert!(
        depuis(&mut e).ends_with(&format!(
            "code, commit {} → {}, qui ne descend pas du commit noté, travail non enregistré \
             différent",
            &tete[..8],
            &orpheline[..8]
        )),
        "{}",
        depuis(&mut e)
    );
    // Revenir à la fiche côté réseau : les notes venues après sont retirées des ressources.
    let restaure = e.ok(&["restaurer", "--depuis", &id, "--motif", "retour", "--oui"]);
    // Contrat modifié (6.5b) : avec --oui, l'aperçu de l'opération critique précède la ligne de
    // résultat ; « starts_with » écrit à tort.
    assert!(
        restaure.contains("\nRestauration enregistrée : "),
        "{restaure}"
    );
    // Annuler juste après une fiche défait la fiche, et le dit.
    let (_, _) = noter(
        &mut e,
        &["--objectif", "dernière", "--action", "a", "--sans-code"],
    );
    assert_eq!(
        e.ok(&["annuler"]),
        "Annulé : point de reprise « dernière ».\n"
    );
    assert_eq!(e.ok(&["annuler"]), "Annulé.\n");
}

// FICHE-REPRISE.md, section 5 : liste des fiches de la branche active, de la plus récente à la plus
// ancienne ; --nombre ; --toutes groupe par branche active, chaque branche ne montrant que les
// fiches notées depuis qu'elle a quitté main. Sans fiche, un message, code 0.
#[test]
fn resume_points_are_listed_per_branch() {
    let mut e = Essai::new("reprise-liste");
    e.ok(&["init"]);
    assert_eq!(
        e.ok(&["reprise", "voir"]),
        "Aucun point de reprise sur cette branche.\n"
    );
    assert_eq!(
        e.ok(&["reprise", "liste"]),
        "Aucun point de reprise sur cette branche.\n"
    );
    let n = creer(&mut e, "raisonnement", "hypothèse");
    let (premier, _) = noter(
        &mut e,
        &["--objectif", "premier", "--action", "a", "--sans-code"],
    );
    let (second, _) = noter(
        &mut e,
        &["--objectif", "second", "--action", "a", "--sans-code"],
    );
    let lignes = |sortie: String| -> Vec<(String, String)> {
        sortie
            .lines()
            .map(|l| {
                let morceaux: Vec<&str> = l.split(" — ").collect();
                assert_eq!(morceaux.len(), 3, "{l}");
                let id = morceaux[0].trim_start().to_owned();
                assert!(date_valide(morceaux[1]), "{l}");
                (id, morceaux[2].to_owned())
            })
            .collect()
    };
    assert_eq!(
        lignes(e.ok(&["reprise", "liste"])),
        vec![
            (second.clone(), "second".into()),
            (premier.clone(), "premier".into())
        ]
    );
    assert_eq!(
        lignes(e.ok(&["reprise", "liste", "--nombre", "1"])),
        vec![(second.clone(), "second".into())]
    );
    e.ok(&[
        "verdict",
        &n,
        "refutee",
        "--motif",
        "contre-exemple",
        "--oui",
    ]);
    let (sur_branche, _) = noter(
        &mut e,
        &[
            "--objectif",
            "sur la branche",
            "--action",
            "a",
            "--sans-code",
        ],
    );
    let branche = e
        .ok(&["statut"])
        .lines()
        .find_map(|l| l.strip_prefix("Branche active : "))
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .to_owned();
    let toutes = e.ok(&["reprise", "liste", "--toutes"]);
    let blocs: Vec<&str> = toutes.split("Branche ").filter(|b| !b.is_empty()).collect();
    assert_eq!(blocs.len(), 2, "{toutes}");
    assert!(
        blocs[0].starts_with(&format!(
            "{} (main) :\n",
            "00000000-0000-0000-0000-000000000000"
        )),
        "{toutes}"
    );
    assert!(blocs[0].contains(&format!("  {second} — ")), "{toutes}");
    assert!(blocs[0].contains(&format!("  {premier} — ")), "{toutes}");
    assert!(blocs[1].starts_with(&format!("{branche} :\n")), "{toutes}");
    assert!(
        blocs[1].contains(&format!("  {sur_branche} — ")),
        "{toutes}"
    );
    assert!(!blocs[1].contains(&second), "{toutes}");
    // Vue depuis main : la fiche de la branche n'est pas dans son historique.
    e.ok(&["basculer", "00000000"]);
    let vue = e.ok(&[
        "reprise",
        "voir",
        "--depuis",
        &sur_branche,
        "--code",
        &e.dir.display().to_string(),
    ]);
    assert!(
        vue.ends_with(
            "Depuis : réseau, fiche hors de l'historique de la branche active ; code non noté\n"
        ),
        "{vue}"
    );
}

// FICHE-REPRISE.md, section 4 : sans dépôt Git, la fiche est notée sans état du code ; --sans-code
// s'en passe ; un dépôt sans commit et une tête détachée sont décrits ; un patch hors UTF-8 n'est
// pas conservé. Les fichiers de la base, s'ils sont dans le dépôt, ne comptent pas parmi les
// changements.
#[test]
fn resume_points_describe_every_state_of_the_code() {
    let mut e = Essai::new("reprise-code");
    e.ok(&["init"]);
    let hors = e.dir.join("hors-git");
    std::fs::create_dir_all(&hors).unwrap();
    let hors_texte = hors.display().to_string();
    let (id, ligne) = noter(
        &mut e,
        &["--objectif", "o", "--action", "a", "--code", &hors_texte],
    );
    assert_eq!(
        ligne,
        format!("Code : non disponible (pas de dépôt Git dans {hors_texte})")
    );
    let vue = e.ok(&["reprise", "voir", "--code", &hors_texte]);
    assert!(
        fiche_vue(&vue, &id)
            .ends_with("Code noté : —\nDepuis : réseau, 0 opération ; code non noté\n"),
        "{vue}"
    );
    let (_, ligne) = noter(&mut e, &["--objectif", "o", "--action", "a", "--sans-code"]);
    assert_eq!(ligne, "Code : non noté (--sans-code)");
    // Dépôt sans commit.
    let vide = e.dir.join("vide");
    std::fs::create_dir_all(&vide).unwrap();
    git(&vide, &["init", "-q"]);
    std::fs::write(vide.join("x.c"), "x\n").unwrap();
    let (_, ligne) = noter(
        &mut e,
        &[
            "--objectif",
            "o",
            "--action",
            "a",
            "--code",
            &vide.display().to_string(),
        ],
    );
    assert_eq!(
        ligne,
        "Code : aucun commit sur principale ; 1 fichier changé dont 1 non suivi (listé seulement)"
    );
    // Tête détachée, propre.
    let (code, tete) = depot_git(&e);
    git(&code, &["checkout", "-q", "--", "src/a.c"]);
    std::fs::remove_file(code.join("b.c")).unwrap();
    git(&code, &["checkout", "-q", "--detach"]);
    let dossier = code.display().to_string();
    let (_, ligne) = noter(
        &mut e,
        &["--objectif", "o", "--action", "a", "--code", &dossier],
    );
    assert_eq!(
        ligne,
        format!(
            "Code : {} (tête détachée) ; aucun changement non enregistré",
            &tete[..8]
        )
    );
    // Patch hors UTF-8 : non conservé, et « reprise code » le dit (code 2).
    std::fs::write(code.join("src/a.c"), b"int \xe9;\n").unwrap();
    let (latin, ligne) = noter(
        &mut e,
        &["--objectif", "o", "--action", "a", "--code", &dossier],
    );
    assert!(
        ligne.ends_with(
            " ; 1 fichier changé ; travail non enregistré non conservé (trop grand ou hors UTF-8)"
        ),
        "{ligne}"
    );
    let (code_retour, _, erreurs) = e.lancer(&["reprise", "code", &latin]);
    assert_eq!(code_retour, 2, "{erreurs}");
    assert!(
        erreurs.contains("travail non enregistré non conservé"),
        "{erreurs}"
    );
    // La base elle-même, placée dans le dépôt, n'est pas un changement du code.
    let mut d = Essai::new("reprise-base-dans-le-depot");
    let racine = d.dir.clone();
    git(&racine, &["init", "-q"]);
    std::fs::write(racine.join("a.c"), "a\n").unwrap();
    git(&racine, &["add", "a.c"]);
    git(&racine, &["commit", "-q", "-m", "a"]);
    d.ok(&["init"]);
    let (_, ligne) = noter(
        &mut d,
        &[
            "--objectif",
            "o",
            "--action",
            "a",
            "--code",
            &racine.display().to_string(),
        ],
    );
    assert!(
        ligne.ends_with(" ; aucun changement non enregistré"),
        "{ligne}"
    );
}

// FICHE-REPRISE.md, sections 2 et 5 : refus localisés, rien d'écrit.
#[test]
fn resume_refusals() {
    let mut e = Essai::new("reprise-refus");
    e.ok(&["init"]);
    let n = creer(&mut e, "note", "n");
    let avant = e.texte_du_depot();
    let absent = e.dir.join("absent").display().to_string();
    for (args, attendu, message) in [
        (vec!["reprise", "noter", "--action", "a"], 2, "--objectif"),
        (vec!["reprise", "noter", "--objectif", "o"], 2, "--action"),
        (
            vec![
                "reprise",
                "noter",
                "--objectif",
                "  ",
                "--action",
                "a",
                "--sans-code",
            ],
            1,
            "champ obligatoire vide dans le point de reprise : objectif",
        ),
        (
            vec![
                "reprise",
                "noter",
                "--objectif",
                "o",
                "--action",
                "a",
                "--code",
                ".",
                "--sans-code",
            ],
            2,
            "--code et --sans-code",
        ),
        (
            vec![
                "reprise",
                "noter",
                "--objectif",
                "o",
                "--action",
                "a",
                "--code",
                &absent,
            ],
            2,
            "pas un dossier",
        ),
        (
            vec![
                "reprise",
                "noter",
                "--objectif",
                "o",
                "--action",
                "a",
                "--sans-code",
                "--notes",
                &format!("{n},{n}"),
            ],
            2,
            "",
        ),
        (
            vec!["reprise", "liste", "--nombre", "0"],
            2,
            "nombre invalide",
        ),
        (
            vec!["reprise", "voir", "--code", &absent],
            2,
            "pas un dossier",
        ),
    ] {
        let (code, _, erreurs) = e.lancer(&args);
        assert_eq!(code, attendu, "{args:?} : {erreurs}");
        assert!(erreurs.contains(message), "{args:?} : {erreurs}");
        assert_eq!(e.texte_du_depot(), avant, "{args:?}");
    }
    let ordinaire = commits(&mut e)[0].clone();
    let (code, _, erreurs) = e.lancer(&["reprise", "voir", "--depuis", &ordinaire]);
    assert_eq!(code, 1, "{erreurs}");
    assert!(erreurs.contains("pas un point de reprise"), "{erreurs}");
    let (code, _, erreurs) = e.lancer(&["reprise", "code", &ordinaire]);
    assert_eq!(code, 1, "{erreurs}");
    let (id, _) = noter(&mut e, &["--objectif", "o", "--action", "a", "--sans-code"]);
    let (code, _, erreurs) = e.lancer(&["reprise", "code", &id]);
    assert_eq!(code, 2, "{erreurs}");
    assert!(erreurs.contains("aucun état du code noté"), "{erreurs}");
}

// Ajoutés après la mutation de 6.5b ------------------------------------------------------------

/// Lance l'exécutable avec le PATH donné ; rend code, sortie et erreurs.
fn avec_path(e: &Essai, path: &Path, args: &[&str]) -> (i32, String, String) {
    let sortie = Process::new(env!("CARGO_BIN_EXE_memoire"))
        .arg("--base")
        .arg(e.base())
        .args(args)
        .env("PATH", path)
        .output()
        .unwrap();
    (
        sortie.status.code().unwrap(),
        String::from_utf8(sortie.stdout).unwrap(),
        String::from_utf8(sortie.stderr).unwrap(),
    )
}

// FICHE-REPRISE.md, section 4 : sans exécutable git, la fiche est notée sans état du code et la
// lecture le dit ; un git qui ne se lance pas, ou qui échoue sur un dépôt reconnu (index
// corrompu), est une erreur de code 3, sans fiche.
#[test]
fn git_absent_or_failing_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let mut e = Essai::new("reprise-git-absent");
    e.ok(&["init"]);
    let (code, _) = depot_git(&e);
    let dossier = code.display().to_string();
    let (id, _) = noter(
        &mut e,
        &["--objectif", "o", "--action", "a", "--code", &dossier],
    );
    let sans_git = e.dir.join("sans-git");
    std::fs::create_dir_all(&sans_git).unwrap();
    let noter_sans_git = [
        "reprise",
        "noter",
        "--objectif",
        "o",
        "--action",
        "a",
        "--code",
        &dossier,
    ];
    let (retour, sortie, erreurs) = avec_path(&e, &sans_git, &noter_sans_git);
    assert_eq!(retour, 0, "{erreurs}");
    assert!(
        sortie.ends_with("\nCode : non disponible (git introuvable)\n"),
        "{sortie}"
    );
    let (retour, sortie, _) = avec_path(
        &e,
        &sans_git,
        &["reprise", "voir", "--depuis", &id, "--code", &dossier],
    );
    assert_eq!(retour, 0);
    assert!(
        sortie.ends_with(" ; code non disponible (git introuvable)\n"),
        "{sortie}"
    );
    // Un « git » qui n'est pas exécutable.
    let faux = e.dir.join("faux-git");
    std::fs::create_dir_all(&faux).unwrap();
    std::fs::write(faux.join("git"), "pas un programme").unwrap();
    std::fs::set_permissions(faux.join("git"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let avant = e.texte_du_depot();
    let (retour, _, erreurs) = avec_path(&e, &faux, &noter_sans_git);
    assert_eq!(retour, 3, "{erreurs}");
    assert!(erreurs.contains("git : "), "{erreurs}");
    assert_eq!(e.texte_du_depot(), avant);
    // Index corrompu : « git status » échoue sur un dépôt reconnu.
    std::fs::write(code.join(".git/index"), "corrompu").unwrap();
    let (retour, _, erreurs) = e.lancer(&noter_sans_git);
    assert_eq!(retour, 3, "{erreurs}");
    assert!(erreurs.contains("git status"), "{erreurs}");
    assert!(erreurs.contains("a échoué"), "{erreurs}");
    assert_eq!(e.texte_du_depot(), avant);
}

// Une tête sans commit, à la fiche ou aujourd'hui : « commit aucun → … » ou « … → aucun ». Les
// fichiers de la base, dans le dépôt, sont exclus, les autres changements non.
#[test]
fn unborn_heads_and_the_base_inside_the_repository() {
    let mut e = Essai::new("reprise-tete-vide");
    e.ok(&["init"]);
    let vide = e.dir.join("vide");
    std::fs::create_dir_all(&vide).unwrap();
    git(&vide, &["init", "-q"]);
    std::fs::write(vide.join("x.c"), "x\n").unwrap();
    let dossier = vide.display().to_string();
    let (id, _) = noter(
        &mut e,
        &["--objectif", "o", "--action", "a", "--code", &dossier],
    );
    git(&vide, &["add", "x.c"]);
    git(&vide, &["commit", "-q", "-m", "premier"]);
    let premier = git(&vide, &["rev-parse", "HEAD"]).trim().to_owned();
    let vue = e.ok(&["reprise", "voir", "--depuis", &id, "--code", &dossier]);
    assert!(
        vue.ends_with(&format!(
            "Depuis : réseau, 0 opération ; code, commit aucun → {}, travail non enregistré \
             différent\n",
            &premier[..8]
        )),
        "{vue}"
    );
    let (id, _) = noter(
        &mut e,
        &["--objectif", "o", "--action", "a", "--code", &dossier],
    );
    git(&vide, &["checkout", "-q", "--orphan", "neuve"]);
    let vue = e.ok(&["reprise", "voir", "--depuis", &id, "--code", &dossier]);
    assert!(
        vue.ends_with(&format!(
            "Depuis : réseau, 0 opération ; code, commit {} → aucun, travail non enregistré \
             différent\n",
            &premier[..8]
        )),
        "{vue}"
    );
    // Base dans le dépôt, plus un vrai changement : seul celui-ci compte.
    let mut d = Essai::new("reprise-base-et-changement");
    let racine = d.dir.clone();
    git(&racine, &["init", "-q"]);
    std::fs::write(racine.join("a.c"), "a\n").unwrap();
    git(&racine, &["add", "a.c"]);
    git(&racine, &["commit", "-q", "-m", "a"]);
    d.ok(&["init"]);
    std::fs::write(racine.join("nouveau.c"), "n\n").unwrap();
    let (_, ligne) = noter(
        &mut d,
        &[
            "--objectif",
            "o",
            "--action",
            "a",
            "--code",
            &racine.display().to_string(),
        ],
    );
    assert!(
        ligne.ends_with(" ; 1 fichier changé dont 1 non suivi (listé seulement)"),
        "{ligne}"
    );
}

// ---------------------------------------------------------------------------
// Incrément 6.6a — vue locale colorée (VOISINAGE.md ; INV-VIEW-02 ; R7).
// ---------------------------------------------------------------------------

/// Réseau d'essai : un centre relié par chaque origine ; un appui réfuté (centre à réexaminer).
/// Rend (centre, appui, voisine, categorie, echange, citation).
fn voisinage_d_essai(e: &mut Essai) -> [String; 6] {
    e.ok(&["init"]);
    let centre = creer(e, "raisonnement", "Réveil perdu");
    let appui = creer(e, "implementation", "Primitive commune");
    let voisine = creer(e, "note", "Hypo\tthèse\nlongue");
    let categorie = creer(e, "categorie", "Scheduler");
    e.ok(&["lier", &centre, "depend-de", &appui]);
    e.ok(&["lier", &voisine, "lien", &centre]);
    e.ok(&["lier", &centre, "classe-dans", &categorie]);
    let echange = coller(e, "Échange 12", Some("q ?"), "le tick réveille la tâche");
    let sortie = e.ok(&[
        "citer",
        &echange,
        "--texte",
        "le tick réveille",
        "--soutient",
        &centre,
    ]);
    let citation = identifiant(&sortie, "Citation créée : ")
        .split(' ')
        .next()
        .unwrap()
        .to_owned();
    e.ok(&[
        "verdict",
        &appui,
        "refutee",
        "--motif",
        "contre-exemple",
        "--oui",
    ]);
    [centre, appui, voisine, categorie, echange, citation]
}

// INV-VIEW-02 (VOISINAGE.md, section 3) : sortie lisible groupée par origine, états de la note et
// de ses voisines ; trois sections vides pour une note sans relation ; rien n'est écrit.
#[test]
fn the_neighborhood_is_printed_by_origin() {
    let mut e = Essai::new("voisinage");
    let [centre, appui, voisine, categorie, echange, citation] = voisinage_d_essai(&mut e);
    let avant = e.texte_du_depot();
    assert_eq!(
        e.ok(&["voisinage", &centre[..8]]),
        format!(
            "Voisinage de Réveil perdu ({centre}) — raisonnement, à réexaminer\n\
             Note entière :\n\
             \x20 → depend-de Primitive commune ({appui}) (écartée)\n\
             \x20 ← lien Hypo\tthèse\nlongue ({voisine})\n\
             Citation :\n\
             \x20 → depend-de « le tick réveille » ({citation}), citation de Échange 12 ({echange})\n\
             Catégorie :\n\
             \x20 → classe-dans Scheduler ({categorie})\n"
        )
    );
    let seule = creer(&mut e, "note", "Seule");
    assert_eq!(
        e.ok(&["voisinage", &seule]),
        format!(
            "Voisinage de Seule ({seule}) — note\nNote entière :\n  (aucun)\nCitation :\n  \
             (aucun)\nCatégorie :\n  (aucun)\n"
        )
    );
    let apres = e.texte_du_depot();
    assert_ne!(apres, avant, "la note créée change le dépôt");
    e.ok(&["voisinage", &centre]);
    assert_eq!(e.texte_du_depot(), apres, "voisinage a écrit");
}

// VOISINAGE.md, section 3 : forme --tsv, contrat de l'interface ; tabulations et retours à la
// ligne d'un nom remplacés par des espaces ; source d'une citation ; états, orpheline comprise.
#[test]
fn the_neighborhood_has_a_tab_separated_form() {
    let mut e = Essai::new("voisinage-tsv");
    let [centre, appui, voisine, categorie, echange, citation] = voisinage_d_essai(&mut e);
    let relation = |e: &mut Essai, origine: &str, destination: &str| -> String {
        // Identifiant de la relation, lu dans la forme --tsv de l'origine.
        e.ok(&["voisinage", origine, "--tsv"])
            .lines()
            .find(|l| l.split('\t').nth(4) == Some(destination))
            .unwrap()
            .split('\t')
            .next()
            .unwrap()
            .to_owned()
    };
    let tsv = e.ok(&["voisinage", &centre, "--tsv"]);
    let lignes: Vec<&str> = tsv.lines().collect();
    assert_eq!(
        lignes[0],
        "relation\tsens\ttype\torigine\tvoisine\trole\tnom\tsource\tetat"
    );
    let r_appui = relation(&mut e, &centre, &appui);
    let r_voisine = relation(&mut e, &centre, &voisine);
    let r_citation = relation(&mut e, &centre, &citation);
    let r_categorie = relation(&mut e, &centre, &categorie);
    assert_eq!(
        lignes[1..],
        [
            format!("{r_appui}\tsortant\tdepend-de\tnote\t{appui}\timplementation\tPrimitive commune\t-\tecartee"),
            format!("{r_voisine}\tentrant\tlien\tnote\t{voisine}\tnote\tHypo thèse longue\t-\tcourante"),
            format!("{r_citation}\tsortant\tdepend-de\tcitation\t{citation}\tcitation\tle tick réveille\t{echange}\tcourante"),
            format!("{r_categorie}\tsortant\tclasse-dans\tcategorie\t{categorie}\tcategorie\tScheduler\t-\tcourante"),
        ]
    );
    let centre_vu = e.ok(&["voisinage", &categorie, "--tsv"]);
    assert!(
        centre_vu.ends_with(&format!(
            "\tentrant\tclasse-dans\tcategorie\t{centre}\traisonnement\tRéveil perdu\t-\ta-reexaminer\n"
        )),
        "{centre_vu}"
    );
    // Orpheline : son seul appui est réfuté et la cause acquittée.
    let orpheline = creer(&mut e, "note", "Orpheline");
    e.ok(&["lier", &orpheline, "depend-de", &appui, "--oui"]);
    e.ok(&[
        "acquitter",
        &orpheline,
        &appui,
        "--motif",
        "reste utile",
        "--oui",
    ]);
    let vu = e.ok(&["voisinage", &appui, "--tsv"]);
    assert!(
        vu.contains(&format!(
            "\tentrant\tdepend-de\tnote\t{orpheline}\tnote\tOrpheline\t-\torpheline\n"
        )),
        "{vu}"
    );
    // Refus d'usage.
    for args in [vec!["voisinage"], vec!["voisinage", "inconnue-12345678"]] {
        let (code, _, erreurs) = e.lancer(&args);
        assert_eq!(code, 2, "{args:?} : {erreurs}");
    }
}

// ---------------------------------------------------------------------------
// Incrément 7.1b — sorties pour le panneau des actions (ACTIONS.md, section 7 ; D30 ; INV-UI-01).
// ---------------------------------------------------------------------------

/// Lignes d'une sortie --tsv, découpées.
fn champs_tsv(sortie: &str) -> Vec<Vec<String>> {
    sortie
        .lines()
        .map(|l| l.split('\t').map(str::to_owned).collect())
        .collect()
}

// ACTIONS.md, sections 3 et 7 : « branches --tsv » donne chaque branche avec un nom lisible, tiré de
// l'opération qui l'a créée, et son motif ; deux noms identiques sont numérotés ; main est « piste
// principale » ; --toutes ajoute les branches abandonnées.
#[test]
fn branches_have_a_readable_name() {
    let mut e = Essai::new("branches-tsv");
    e.ok(&["init"]);
    let hypothese = creer(&mut e, "raisonnement", "Hypothèse");
    let file = creer(&mut e, "implementation", "File d'attente");
    let refutation = identifiant(
        &e.ok(&[
            "verdict",
            &hypothese,
            "refutee",
            "--motif",
            "contre-exemple",
            "--oui",
        ]),
        "Branche créée : ",
    );
    e.ok(&["basculer", "00000000"]);
    let mise_de_cote = identifiant(
        &e.ok(&[
            "decision",
            &file,
            "plus-retenue",
            "--motif",
            "trop lente",
            "--oui",
        ]),
        "Branche créée : ",
    );
    e.ok(&["basculer", "00000000"]);
    let seconde = identifiant(
        &e.ok(&[
            "decision",
            &file,
            "plus-retenue",
            "--motif",
            "doublon",
            "--oui",
        ]),
        "Branche créée : ",
    )
    .to_owned();
    let lignes = champs_tsv(&e.ok(&["branches", "--tsv"]));
    assert_eq!(
        lignes[0],
        ["piste", "active", "mise-de-cote", "nom", "motif"]
    );
    let ligne = |id: &str| lignes.iter().find(|l| l[0] == id).cloned().unwrap();
    assert_eq!(
        ligne("00000000-0000-0000-0000-000000000000"),
        [
            "00000000-0000-0000-0000-000000000000",
            "non",
            "non",
            "piste principale",
            ""
        ]
    );
    assert_eq!(
        ligne(&refutation),
        [
            refutation.as_str(),
            "non",
            "non",
            "Réfutation de « Hypothèse »",
            "contre-exemple"
        ]
    );
    // Deux branches de même nom : la seconde, dans l'ordre des identifiants, est numérotée.
    let (premiere, deuxieme) = if mise_de_cote < seconde {
        (&mise_de_cote, &seconde)
    } else {
        (&seconde, &mise_de_cote)
    };
    assert_eq!(ligne(premiere)[3], "Mise de côté de « File d'attente »");
    assert_eq!(ligne(deuxieme)[3], "Mise de côté de « File d'attente » (2)");
    assert_eq!(ligne(&seconde)[1], "oui");
    assert_eq!(lignes.len(), 5);
    // Abandonnée : absente sans --toutes, marquée avec.
    e.ok(&["abandonner", &refutation]);
    assert!(!e.ok(&["branches", "--tsv"]).contains(&refutation));
    let toutes = champs_tsv(&e.ok(&["branches", "--toutes", "--tsv"]));
    let abandonnee = toutes.iter().find(|l| l[0] == refutation).unwrap();
    assert_eq!(abandonnee[2], "oui");
    // Retour à une étape : nommé par l'étape.
    let etape = commits(&mut e)[2].clone();
    e.ok(&["basculer", "00000000"]);
    let retour = identifiant(
        &e.ok(&["restaurer", "--depuis", &etape, "--motif", "essai", "--oui"]),
        "Branche créée : ",
    );
    let lignes = champs_tsv(&e.ok(&["branches", "--tsv"]));
    let nom = &lignes.iter().find(|l| l[0] == retour).unwrap()[3];
    assert_eq!(nom, &format!("Retour à l'étape {}", &etape[..8]));
}

// ACTIONS.md, section 5 (« Adopter une piste ») : « fusion plan --tsv » donne le code du plan, la
// base, le nombre de changements sans conflit, chaque désaccord (genre, note, champ, valeur ici,
// valeur dans la branche) et les notes à revoir ; le code est celui de la sortie lisible et
// « fusionner » l'accepte ; une branche déjà réunie donne « a-jour ».
#[test]
fn the_merge_plan_has_a_tab_separated_form() {
    let mut e = Essai::new("fusion-tsv");
    e.ok(&["init"]);
    let idee = creer(&mut e, "raisonnement", "Hypo\tthèse");
    let source = identifiant(
        &e.ok(&[
            "decision",
            &idee,
            "plus-retenue",
            "--motif",
            "écartée",
            "--oui",
        ]),
        "Branche créée : ",
    );
    e.ok(&["basculer", "00000000"]);
    e.ok(&["decision", &idee, "retenue", "--motif", "gardée"]);
    let lisible = e.ok(&["fusion", "plan", &source]);
    let code = lisible
        .lines()
        .find_map(|l| l.strip_prefix("Code du plan : "))
        .unwrap()
        .to_owned();
    let lignes = champs_tsv(&e.ok(&["fusion", "plan", &source, "--tsv"]));
    assert_eq!(lignes[0], ["code", code.as_str()]);
    assert_eq!(lignes[1][0], "base");
    assert_eq!(lignes[2], ["sans-conflit", "0"]);
    assert_eq!(
        lignes[3],
        [
            "conflit",
            "1",
            "champ",
            idee.as_str(),
            "Hypo thèse",
            "decision",
            "retenue",
            "plus-retenue"
        ]
    );
    assert_eq!(lignes.len(), 4, "{lignes:?}");
    let applique = e.ok(&[
        "fusionner",
        &source,
        "--plan",
        &code,
        "--source",
        "1",
        "--motif",
        "adoptée",
        "--oui",
    ]);
    assert!(applique.contains("Fusion enregistrée"), "{applique}");
    assert_eq!(e.ok(&["fusion", "plan", &source, "--tsv"]), "a-jour\toui\n");
}

// ---------------------------------------------------------------------------
// Incrément 7.2a — retrouver les notes d'un code (RETROUVER.md ; INV-CODE-07).
// ---------------------------------------------------------------------------

/// Projet d'essai : un fichier src/sched.c, deux notes liées par des lignes, une seulement
/// marquée. Rend (code, reveil, file, seule).
fn code_d_essai(e: &mut Essai) -> (PathBuf, String, String, String) {
    e.ok(&["init"]);
    let reveil = creer(e, "implementation", "Réveil");
    let file = creer(e, "implementation", "File");
    let seule = creer(e, "raisonnement", "Seule");
    let code = e.dir.clone();
    ecrire(
        &code,
        "src/sched.c",
        format!(
            "#include <x.h>\n// @design: {seule}\nint file[8];\n\n// @design: {reveil}\n\
             void wake_task(struct task *t)\n{{\n    t->etat = PRETE;\n}}\n"
        )
        .as_bytes(),
    );
    let chemin = code.join("src/sched.c").display().to_string();
    e.ok(&["ancrer", &reveil, "--fichier", &chemin, "--lignes", "6-9"]);
    e.ok(&["ancrer", &file, "--fichier", &chemin, "--lignes", "3-3"]);
    (code, reveil, file, seule)
}

// RETROUVER.md, section 2 : notes liées au fichier et marques qu'il contient ; avec une ligne,
// celle dont l'extrait la contient d'abord et la marque la plus proche au-dessus ; forme --tsv ;
// rien n'est écrit.
#[test]
fn the_notes_of_a_code_file() {
    let mut e = Essai::new("code-notes");
    let (code, reveil, file, seule) = code_d_essai(&mut e);
    let avant = e.texte_du_depot();
    let avant_code = contenus(&code.join("src"));
    assert_eq!(
        e.ok(&["code", "notes", "src/sched.c"]),
        format!(
            "Notes liées à src/sched.c :\n\
             \x20 - Réveil ({reveil}) — « void wake_task(struct task *t) »\n\
             \x20 - File ({file}) — « int file[8]; »\n\
             Marques dans le fichier :\n\
             \x20 - ligne 2 : Seule ({seule})\n\
             \x20 - ligne 5 : Réveil ({reveil})\n"
        )
    );
    assert_eq!(
        e.ok(&[
            "code",
            "notes",
            &code.join("src/sched.c").display().to_string(),
            "--ligne",
            "8"
        ]),
        format!(
            "Notes liées à src/sched.c :\n\
             \x20 - Réveil ({reveil}) — « void wake_task(struct task *t) » — contient la ligne 8\n\
             \x20 - File ({file}) — « int file[8]; »\n\
             Marques dans le fichier :\n\
             \x20 - ligne 2 : Seule ({seule})\n\
             \x20 - ligne 5 : Réveil ({reveil}) — la plus proche au-dessus de la ligne 8\n"
        )
    );
    let tsv = e.ok(&[
        "code",
        "notes",
        "src/sched.c",
        "--extrait",
        "int file[8];",
        "--tsv",
    ]);
    assert_eq!(
        tsv,
        format!(
            "ancrage\t{file}\tFile\textrait\tsrc/sched.c\tint file[8];\n\
             ancrage\t{reveil}\tRéveil\tfichier\tsrc/sched.c\tvoid wake_task(struct task *t)\n\
             marque\t2\t{seule}\tSeule\tconnue\tnon\n\
             marque\t5\t{reveil}\tRéveil\tconnue\tnon\n"
        )
    );
    assert_eq!(e.texte_du_depot(), avant);
    assert_eq!(contenus(&code.join("src")), avant_code);
}

// RETROUVER.md, section 2 : chemin proche signalé à part ; marque inconnue et marque mal formée ;
// refus d'usage (fichier absent, ligne hors du fichier, ligne et extrait ensemble).
#[test]
fn the_notes_of_a_code_file_near_paths_and_refusals() {
    let mut e = Essai::new("code-notes-proche");
    let (code, reveil, _file, _seule) = code_d_essai(&mut e);
    ecrire(
        &code,
        "autre/sched.c",
        b"// @design: aaaaaaaa-0000-4000-8000-000000000000\n/* @design: 1234 */\n",
    );
    let sortie = e.ok(&["code", "notes", "autre/sched.c"]);
    assert_eq!(
        sortie,
        format!(
            "Notes liées à autre/sched.c :\n  (aucune)\n\
             Chemin proche :\n\
             \x20 - Réveil ({reveil}) — « void wake_task(struct task *t) » — ancré dans src/sched.c\n\
             \x20 - File ({}) — « int file[8]; » — ancré dans src/sched.c\n\
             Marques dans le fichier :\n\
             \x20 - ligne 1 : note absente de cette piste (aaaaaaaa-0000-4000-8000-000000000000)\n\
             \x20 - ligne 2 : marque mal formée\n",
            _file
        )
    );
    for args in [
        vec!["code", "notes", "absent.c"],
        vec!["code", "notes", "src/sched.c", "--ligne", "99"],
        vec!["code", "notes", "src/sched.c", "--ligne", "0"],
        vec![
            "code",
            "notes",
            "src/sched.c",
            "--ligne",
            "2",
            "--extrait",
            "x",
        ],
        vec!["code", "notes"],
    ] {
        let (code_retour, _, erreurs) = e.lancer(&args);
        assert_eq!(code_retour, 2, "{args:?} : {erreurs}");
    }
}

// RETROUVER.md, section 2 : « repere » accepte toute ligne qui contient une marque, quelle que
// soit la syntaxe ; deux marques donnent deux notes ; une ligne sans marque valable est refusée.
#[test]
fn a_marker_is_found_whatever_the_comment_syntax() {
    let mut e = Essai::new("repere-syntaxes");
    let (_code, reveil, file, _seule) = code_d_essai(&mut e);
    for ligne in [
        format!("// @design: {reveil}"),
        format!("    wake(t); // @design: {reveil}"),
        format!("# @design: {reveil}"),
        format!("/* @design: {reveil} */"),
        format!("; @design: {reveil}"),
    ] {
        let sortie = e.ok(&["repere", &ligne]);
        assert!(
            sortie.starts_with(&format!("Note : Réveil ({reveil})\n")),
            "{ligne} : {sortie}"
        );
    }
    let deux = e.ok(&[
        "repere",
        &format!("x(); // @design: {reveil} @design: {file}"),
    ]);
    let blocs: Vec<&str> = deux.split("\n\n").collect();
    assert_eq!(blocs.len(), 2, "{deux}");
    assert!(blocs[0].starts_with(&format!("Note : Réveil ({reveil})")));
    assert!(blocs[1].starts_with(&format!("Note : File ({file})")));
    let (code_retour, _, _) = e.lancer(&["repere", "/* @design: 1234 */"]);
    assert_eq!(code_retour, 1);
}

// Formes restantes de « code notes » (ajouté après la mutation de 7.2a) : marques inconnue et mal
// formée en --tsv ; extrait contenu dans un ancrage ; marque couverte par l'extrait.
#[test]
fn the_notes_of_a_code_file_other_forms() {
    let mut e = Essai::new("code-notes-formes");
    let (code, reveil, file, _seule) = code_d_essai(&mut e);
    ecrire(
        &code,
        "autre/x.c",
        b"// @design: aaaaaaaa-0000-4000-8000-000000000000\n/* @design: 1234 */\n",
    );
    assert_eq!(
        e.ok(&["code", "notes", "autre/x.c", "--tsv"]),
        "marque\t1\taaaaaaaa-0000-4000-8000-000000000000\t-\tinconnue\tnon\n\
         marque\t2\t-\t-\tmal-formee\tnon\n"
    );
    let sortie = e.ok(&[
        "code",
        "notes",
        "src/sched.c",
        "--extrait",
        "{\n    t->etat = PRETE;",
    ]);
    assert!(
        sortie.contains(&format!(
            "  - Réveil ({reveil}) — « void wake_task(struct task *t) » — contient l'extrait\n"
        )),
        "{sortie}"
    );
    assert!(
        sortie.contains(&format!("  - File ({file}) — « int file[8]; »\n")),
        "{sortie}"
    );
    let extrait = format!("\n// @design: {reveil}");
    let sortie = e.ok(&["code", "notes", "src/sched.c", "--extrait", &extrait]);
    assert!(
        sortie.contains(&format!(
            "  - ligne 5 : Réveil ({reveil}) — dans l'extrait\n"
        )),
        "{sortie}"
    );
}
