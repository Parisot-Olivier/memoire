//! Refus et cas limites des commandes de l'incrément 4.2 (CLI.md, section 8).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use memoire_app::SequenceIds;
use memoire_cli::{executer, Environnement};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Essai {
    dir: PathBuf,
    appels: u64,
}

impl Essai {
    fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("memoire-refus-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut essai = Self { dir, appels: 0 };
        essai.ok(&["init"]);
        essai
    }

    fn lancer(&mut self, args: &[&str]) -> (i32, String, String) {
        self.appels += 1;
        let environnement = Environnement {
            base: Some(self.dir.join("depot.db")),
            repertoire: self.dir.clone(),
        };
        let tout: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let (mut sortie, mut erreurs) = (Vec::new(), Vec::new());
        let code = executer(
            &tout,
            &environnement,
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

    fn ok(&mut self, args: &[&str]) -> String {
        let (code, out, err) = self.lancer(args);
        assert_eq!(code, 0, "{args:?}\nsortie : {out}\nerreurs : {err}");
        out
    }

    /// Code et message d'erreur.
    fn refus(&mut self, args: &[&str]) -> (i32, String) {
        let (code, _, err) = self.lancer(args);
        assert_ne!(code, 0, "{args:?} aurait dû être refusé");
        (code, err)
    }

    fn id(&mut self, args: &[&str], debut: &str) -> String {
        let out = self.ok(args);
        out.lines()
            .find_map(|l| l.strip_prefix(debut))
            .unwrap_or_else(|| panic!("« {debut} » absent de : {out}"))
            .split(' ')
            .next()
            .unwrap()
            .to_owned()
    }

    fn fichier(&self, nom: &str, contenu: &str) -> String {
        let chemin = self.dir.join(nom);
        std::fs::write(&chemin, contenu).unwrap();
        chemin.display().to_string()
    }

    fn export(&mut self) -> String {
        let chemin = self.dir.join("export.txt");
        self.ok(&["exporter", chemin.to_str().unwrap()]);
        std::fs::read_to_string(chemin).unwrap()
    }
}

impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn exchanges_and_passages() {
    let mut e = Essai::new("passages");
    let r = e.fichier("r.txt", "abc abc — fin");
    let q = e.fichier("q.txt", "question ?");
    let absent = e.dir.join("absent.txt").display().to_string();
    assert_eq!(e.refus(&["echange", "coller", "--titre", "t"]).0, 2);
    assert_eq!(
        e.refus(&[
            "echange",
            "coller",
            "--titre",
            "t",
            "--reponse-fichier",
            &absent
        ])
        .0,
        3
    );
    assert_eq!(
        e.refus(&[
            "echange",
            "coller",
            "--titre",
            "t",
            "--reponse-fichier",
            &r,
            "--precedent",
            "ffffffff"
        ])
        .0,
        2
    );
    // Échange incomplet, puis question ajoutée une fois ; une seconde fois : refus du domaine.
    let x = e.id(
        &["echange", "coller", "--titre", "t", "--reponse-fichier", &r],
        "Échange créé : ",
    );
    assert_eq!(
        e.refus(&["citer", &x, "--texte", "question", "--partie", "question"])
            .0,
        2
    );
    assert_eq!(
        e.ok(&["echange", "question", &x, "--fichier", &q]),
        "Question enregistrée.\n"
    );
    assert_eq!(e.refus(&["echange", "question", &x, "--fichier", &q]).0, 1);
    assert_eq!(
        e.refus(&["echange", "question", &x, "--fichier", &absent])
            .0,
        3
    );
    // Passage dans la question. Contrat modifié (5.5b, D18, D24) : « passage » devient « citer » ;
    // la citation est suivie de la relation qui la relie à sa source, d'où « contains ».
    let out = e.ok(&["citer", &x, "--texte", "question", "--partie", "question"]);
    assert!(out.contains("(octets 0 à 8)\nRelation créée : "), "{out}");
    // Plusieurs occurrences : refus sans --occurrence, choix explicite sinon.
    let (code, err) = e.refus(&["citer", &x, "--texte", "abc"]);
    assert_eq!(code, 2);
    assert!(err.contains("présent 2 fois"), "{err}");
    assert!(e
        .ok(&["citer", &x, "--texte", "abc", "--occurrence", "1"])
        .contains("(octets 0 à 3)\nRelation créée : "));
    assert!(e
        .ok(&["citer", &x, "--texte", "abc", "--occurrence", "2"])
        .contains("(octets 4 à 7)\nRelation créée : "));
    for mauvais in ["0", "3", "deux", "-1"] {
        assert_eq!(
            e.refus(&["citer", &x, "--texte", "abc", "--occurrence", mauvais])
                .0,
            2,
            "{mauvais}"
        );
    }
    // Texte introuvable, vide, partie inconnue ; octets d'un texte accentué.
    assert_eq!(e.refus(&["citer", &x, "--texte", "absent"]).0, 2);
    assert_eq!(e.refus(&["citer", &x, "--texte", ""]).0, 2);
    assert_eq!(
        e.refus(&["citer", &x, "--texte", "abc", "--partie", "corps"])
            .0,
        2
    );
    assert!(e
        .ok(&["citer", &x, "--texte", "— fin"])
        .contains("(octets 8 à 15)\nRelation créée : "));
    // Un rôle réservé à sa propre commande n'est pas créable par « note creer ».
    assert_eq!(
        e.refus(&["note", "creer", "--role", "echange", "--titre", "t"])
            .0,
        2
    );
}

#[test]
fn anchors_are_extracted_exactly_and_never_written() {
    let mut e = Essai::new("ancrages");
    let code = "int a;\nvoid f(void) {\n  a = 1;\n}\nvoid g(void) {\n}\nvoid f(void) {\n}\n";
    let unique = "int b;\nvoid h(int x) {\n  b = x;\n}\n";
    let double = e.fichier("double.c", code);
    let simple = e.fichier("simple.c", unique);
    let n = e.id(
        &["note", "creer", "--role", "implementation", "--titre", "n"],
        "Note créée : ",
    );
    let absent = e.dir.join("absent.c").display().to_string();
    for args in [
        vec![
            "ancrer",
            &n,
            "--fichier",
            &double,
            "--fonction",
            "void f(void)",
        ],
        vec![
            "ancrer",
            &n,
            "--fichier",
            &simple,
            "--fonction",
            "void z(void)",
        ],
        vec!["ancrer", &n, "--fichier", &simple],
        vec![
            "ancrer",
            &n,
            "--fichier",
            &simple,
            "--fonction",
            "void h(int x)",
            "--lignes",
            "1-2",
        ],
        vec!["ancrer", &n, "--fichier", &simple, "--lignes", "0-1"],
        vec!["ancrer", &n, "--fichier", &simple, "--lignes", "3-2"],
        vec!["ancrer", &n, "--fichier", &simple, "--lignes", "1-5"],
        vec!["ancrer", &n, "--fichier", &simple, "--lignes", "a-b"],
        vec!["ancrer", &n, "--fichier", &simple, "--lignes", "2"],
    ] {
        assert_eq!(e.refus(&args).0, 2, "{args:?}");
    }
    assert_eq!(
        e.refus(&["ancrer", &n, "--fichier", &absent, "--lignes", "1-1"])
            .0,
        3
    );
    let sans_fin = e.fichier("sans_fin.c", "void k(void) {\n  x;");
    assert_eq!(
        e.refus(&[
            "ancrer",
            &n,
            "--fichier",
            &sans_fin,
            "--fonction",
            "void k(void)"
        ])
        .0,
        2
    );
    let out = e.ok(&[
        "ancrer",
        &n,
        "--fichier",
        &simple,
        "--fonction",
        "void h(int x)",
        "--git",
        "abc1234",
    ]);
    assert!(out.starts_with("Ancrage créé : "), "{out}");
    assert!(
        out.ends_with(&format!("Repère à copier : // @design: {n}\n")),
        "{out}"
    );
    e.ok(&["ancrer", &n, "--fichier", &simple, "--lignes", "1-2"]);
    e.ok(&["ancrer", &n, "--fichier", &simple, "--lignes", "4-4"]);
    let export = e.export();
    assert!(
        export.contains("extrait \"void h(int x) {\\n  b = x;\\n}\" empreinte"),
        "{export}"
    );
    assert!(export.contains("git \"abc1234\""), "{export}");
    assert!(
        export.contains("extrait \"int b;\\nvoid h(int x) {\\n\""),
        "{export}"
    );
    assert!(export.contains("extrait \"}\\n\""), "{export}");
    assert_eq!(std::fs::read_to_string(&simple).unwrap(), unique);
}

#[test]
fn reserves_relations_and_statuses() {
    let mut e = Essai::new("reserves");
    let r = e.fichier("r.txt", "attention au réveil perdu");
    let x = e.id(
        &["echange", "coller", "--titre", "t", "--reponse-fichier", &r],
        "Échange créé : ",
    );
    let p = e.id(
        &["citer", &x, "--texte", "réveil perdu"],
        "Citation créée : ",
    );
    let a = e.id(
        &["note", "creer", "--role", "raisonnement", "--titre", "a"],
        "Note créée : ",
    );
    let b = e.id(
        &["note", "creer", "--role", "implementation", "--titre", "b"],
        "Note créée : ",
    );
    let res = e.id(
        &["reserve", "creer", &p, "--titre", "réserve"],
        "Réserve créée : ",
    );
    assert_eq!(e.refus(&["reserve", "creer", &p]).0, 2);
    assert!(e
        .ok(&["reserve", "concerne", &res, &b])
        .starts_with("Relation créée : "));
    assert_eq!(e.refus(&["reserve", "traiter", &b, &res]).0, 2);
    assert!(e
        .ok(&["reserve", "traiter", &b, &res, "--motif", "proposition"])
        .starts_with("Relation créée : "));
    assert_eq!(
        e.refus(&["reserve", "etat", &res, "fermee", "--motif", "m"])
            .0,
        2
    );
    assert_eq!(
        e.ok(&["reserve", "etat", &res, "resolue", "--motif", "vérifiée"]),
        "Statut enregistré.\n"
    );
    // Restrictions, désaccords, implémentation, révision.
    assert_eq!(e.refus(&["restreindre", &b, &a]).0, 2);
    assert!(e
        .ok(&["restreindre", &b, &a, "--condition", "monocœur"])
        .starts_with("Relation créée : "));
    assert_eq!(e.refus(&["contredire", &b, &a]).0, 2);
    assert!(e
        .ok(&["contredire", &b, &a, "--justification", "désaccord"])
        .starts_with("Relation créée : "));
    assert_eq!(
        e.refus(&["implementation", &b, "finie", "--motif", "m"]).0,
        2
    );
    assert_eq!(
        e.ok(&["implementation", &b, "implementee", "--motif", "fait"]),
        "Statut enregistré.\n"
    );
    assert_eq!(e.refus(&["note", "reviser", &a, "--texte", "x"]).0, 2);
    assert_eq!(
        e.refus(&["note", "reviser", &a, "--texte", "x", "--nature", "moyenne"])
            .0,
        2
    );
    assert_eq!(
        e.ok(&["note", "reviser", &a, "--texte", "coquille", "--nature", "mineure"]),
        "Révision enregistrée.\n"
    );
    // Une révision substantielle d'une note dont une autre dépend est critique.
    e.ok(&["lier", &b, "depend-de", &a]);
    let (code, _) = e.refus(&[
        "note",
        "reviser",
        &a,
        "--texte",
        "autre sens",
        "--nature",
        "substantielle",
    ]);
    assert_eq!(code, 4);
    let out = e.ok(&[
        "note",
        "reviser",
        &a,
        "--texte",
        "autre sens",
        "--nature",
        "substantielle",
        "--oui",
    ]);
    assert!(
        out.contains("Révision enregistrée.\nBranche créée : "),
        "{out}"
    );
    // Acquitter : une note qui n'est pas cause est refusée par le domaine ; une vraie cause, non.
    assert_eq!(
        e.refus(&["acquitter", &b, &p, "--motif", "m", "--oui"]).0,
        1
    );
    assert_eq!(e.refus(&["acquitter", &b, &a, "--oui"]).0, 2);
    let out = e.ok(&["acquitter", &b, &a, "--motif", "toujours valable", "--oui"]);
    assert!(out.ends_with("Acquittement enregistré.\n"), "{out}");
}

#[test]
fn markers_are_resolved_or_refused() {
    let mut e = Essai::new("reperes");
    let n = e.id(
        &[
            "note",
            "creer",
            "--role",
            "raisonnement",
            "--titre",
            "seule",
        ],
        "Note créée : ",
    );
    assert_eq!(e.refus(&["repere", "// pas de repère ici"]).0, 1);
    assert_eq!(
        e.refus(&["repere", "// @design: 7e000000-0000-0000-0000-000000000001"])
            .0,
        1
    );
    assert_eq!(e.refus(&["repere"]).0, 2);
    e.ok(&["verdict", &n, "verifiee", "--motif", "test"]);
    let out = e.ok(&["repere", &format!("// @design: {n}")]);
    assert_eq!(
        out,
        format!(
            "Note : seule ({n})\nRôle : raisonnement\nDécision : proposee\nVérification : verifiee\nImplémentation : non-commencee\n\
             S'appuie sur :\n  (aucune)\nDésaccords :\n  (aucune)\nRéserves de la note :\n  (aucune)\n\
             Réserves de ses catégories :\n  (aucune)\nAncrages :\n  (aucune)\n"
        )
    );
}

/// Sortie exacte de `repere` sur un voisinage choisi : seules les relations actives du bon type
/// comptent ; les réserves de catégorie sont celles qui proviennent d'un échange de la catégorie ou
/// qui visent une note de la catégorie, hors réserves de la note (compléments après mutation).
#[test]
fn repere_selects_exactly_the_relevant_neighbours() {
    let mut e = Essai::new("voisinage");
    let note = |e: &mut Essai, role: &str, titre: &str| {
        e.id(
            &["note", "creer", "--role", role, "--titre", titre],
            "Note créée : ",
        )
    };
    let c1 = note(&mut e, "categorie", "C1");
    let c2 = note(&mut e, "categorie", "C2");
    let n = note(&mut e, "implementation", "n");
    let o = note(&mut e, "implementation", "o");
    let z = note(&mut e, "implementation", "z");
    let m = note(&mut e, "raisonnement", "m");
    let k = note(&mut e, "raisonnement", "k");
    let j = note(&mut e, "raisonnement", "j");
    let fichier = e.fichier("r.txt", "réserve possible ici");
    let echange = |e: &mut Essai, titre: &str| {
        e.id(
            &[
                "echange",
                "coller",
                "--titre",
                titre,
                "--reponse-fichier",
                &fichier,
            ],
            "Échange créé : ",
        )
    };
    let x = echange(&mut e, "x");
    let y = echange(&mut e, "y");
    let w = echange(&mut e, "w");
    for (de, vers) in [(&n, &c1), (&o, &c1), (&x, &c1), (&z, &c2), (&y, &c2)] {
        e.ok(&["lier", de, "classe-dans", vers]);
    }
    e.ok(&["lier", &n, "depend-de", &m]);
    e.ok(&["lier", &n, "lien", &j]);
    e.ok(&["contredire", &n, &k, "--justification", "désaccord assumé"]);
    let reserve = |e: &mut Essai, echange: &str, titre: &str| {
        let p = e.id(
            &["citer", echange, "--texte", "réserve"],
            "Citation créée : ",
        );
        e.id(
            &["reserve", "creer", &p, "--titre", titre],
            "Réserve créée : ",
        )
    };
    let rd = reserve(&mut e, &w, "rd");
    let rx = reserve(&mut e, &x, "rx");
    let ro = reserve(&mut e, &w, "ro");
    let rz = reserve(&mut e, &y, "rz");
    let _ru = reserve(&mut e, &w, "ru");
    e.ok(&["reserve", "concerne", &rd, &n]);
    e.ok(&["reserve", "concerne", &ro, &o]);
    e.ok(&["reserve", "concerne", &rz, &z]);
    let out = e.ok(&["repere", &format!("// @design: {n}")]);
    let attendu = [
        format!("Note : n ({n})"),
        "Rôle : implementation".to_owned(),
        "Décision : proposee".to_owned(),
        "Vérification : non-verifiee".to_owned(),
        "Implémentation : non-commencee".to_owned(),
        "S'appuie sur :".to_owned(),
        format!("  - m ({m})"),
        "Désaccords :".to_owned(),
        format!("  - k ({k}) — désaccord assumé"),
        "Réserves de la note :".to_owned(),
        format!("  - rd ({rd}) — ouverte"),
        "Réserves de ses catégories :".to_owned(),
        format!("  - rx ({rx}) — ouverte"),
        format!("  - ro ({ro}) — ouverte"),
        "Ancrages :".to_owned(),
        "  (aucune)".to_owned(),
    ]
    .join("\n")
        + "\n";
    assert_eq!(out, attendu);
}
