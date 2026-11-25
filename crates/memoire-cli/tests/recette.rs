//! Test de recette de l'incrément 4.2 (CLI.md, section 8) : S-REF-01 rejoué en entier par la ligne
//! de commande, sur les sources réelles figées. Écrit avant le code des commandes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use memoire_app::SequenceIds;
use memoire_cli::{executer, Environnement};

static COUNTER: AtomicU64 = AtomicU64::new(0);
const DOC: &str = include_str!("../../../fixtures/rtos-2026-10/doc.md");
const DRIVER: &str = include_str!("../../../fixtures/rtos-2026-10/driver.c");

struct Essai {
    dir: PathBuf,
    appels: u64,
}

impl Essai {
    fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("memoire-recette-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir, appels: 0 }
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

    /// Identifiant affiché après « debut » dans la sortie d'une commande réussie.
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
}

impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Même découpage que le contrat du domaine : marqueurs « ** question » et « ** réponse ».
fn split_exchanges(doc: &str) -> Vec<(&str, &str)> {
    let mut marks = Vec::new();
    let mut offset = 0;
    for line in doc.split_inclusive('\n') {
        let question = match line.trim_end_matches('\n') {
            "** question" => Some(true),
            "** réponse" | "** reponse" => Some(false),
            _ => None,
        };
        if let Some(question) = question {
            marks.push((offset, offset + line.len(), question));
        }
        offset += line.len();
    }
    marks
        .chunks(2)
        .enumerate()
        .map(|(k, pair)| {
            let next = marks.get(2 * k + 2).map_or(doc.len(), |m| m.0);
            (&doc[pair[0].1..pair[1].0], &doc[pair[1].1..next])
        })
        .collect()
}

#[test]
fn the_reference_scenario_replays_through_the_command_line() {
    let mut e = Essai::new("sref01");
    e.ok(&["init"]);

    // Étape 1 : cinq échanges chaînés, octet pour octet.
    let mut echanges: Vec<String> = Vec::new();
    for (k, (question, reponse)) in split_exchanges(DOC).into_iter().enumerate() {
        let q = e.fichier(&format!("q{k}.txt"), question);
        let r = e.fichier(&format!("r{k}.txt"), reponse);
        let mut args = vec![
            "echange",
            "coller",
            "--titre",
            "rôle du driver",
            "--question-fichier",
            &q,
            "--reponse-fichier",
            &r,
        ];
        let precedent = echanges.last().cloned();
        if let Some(p) = &precedent {
            args.extend(["--precedent", p.as_str()]);
        }
        echanges.push(e.id(&args, "Échange créé : "));
    }
    assert_eq!(echanges.len(), 5);
    let [e1, e2, e3, e4, e5] = [0, 1, 2, 3, 4].map(|i| echanges[i].clone());

    // Étape 2 : catégories ; un cycle de catégories est refusé par le domaine.
    let c_drv = e.id(
        &[
            "note",
            "creer",
            "--role",
            "categorie",
            "--titre",
            "Driver GPIO : réveil des tâches",
        ],
        "Note créée : ",
    );
    let c_exp = e.id(
        &[
            "note",
            "creer",
            "--role",
            "categorie",
            "--titre",
            "Exploration / évolution",
        ],
        "Note créée : ",
    );
    let c_mc = e.id(
        &[
            "note",
            "creer",
            "--role",
            "categorie",
            "--titre",
            "Exploration multicœur",
        ],
        "Note créée : ",
    );
    e.ok(&["lier", &c_mc, "classe-dans", &c_exp]);
    for (x, c) in [
        (&e1, &c_drv),
        (&e2, &c_drv),
        (&e3, &c_drv),
        (&e3, &c_mc),
        (&e4, &c_mc),
        (&e5, &c_mc),
    ] {
        e.ok(&["lier", x, "classe-dans", c]);
    }
    assert_eq!(e.lancer(&["lier", &c_exp, "classe-dans", &c_mc]).0, 1);

    // Étape 3 : restriction de périmètre, sans verdict.
    let p_e3c = e.id(&["citer", &e3, "--texte", "En multicœur, cependant, une écriture atomique de l’état ne suffit pas à garantir ce transfert."], "Citation créée : ");
    let p_e4 = e.id(
        &[
            "citer",
            &e4,
            "--texte",
            "Ma réponse précédente supposait plusieurs cœurs capables d’ordonnancer simultanément.",
        ],
        "Citation créée : ",
    );
    e.ok(&[
        "restreindre",
        &p_e4,
        &p_e3c,
        "--condition",
        "plusieurs ordonnanceurs",
    ]);

    // Étape 4 : décisions, sources citées, désaccord assumé.
    let p_e2a = e.id(
        &[
            "citer",
            &e2,
            "--texte",
            "Ton idée d’organiser les attentes par port, puis par broche, est pertinente.",
        ],
        "Citation créée : ",
    );
    let p_e3a = e.id(&["citer", &e3, "--texte", "Une tâche bloquée peut être référencée directement dans les structures d’attente du périphérique, puis rendue à l’ordonnanceur lors du réveil."], "Citation créée : ");
    let p_e3b = e.id(&["citer", &e3, "--texte", "le gestionnaire d’attente identifie la tâche à réveiller, puis une primitive commune réalise le transfert vers PENDING."], "Citation créée : ");
    let n1 = e.id(
        &[
            "note",
            "creer",
            "--role",
            "implementation",
            "--titre",
            "N-DRV-1",
            "--texte",
            "Le driver réveille les tâches par écriture directe de l’état",
        ],
        "Note créée : ",
    );
    e.ok(&[
        "decision",
        &n1,
        "retenue",
        "--motif",
        "choix pour le monocœur",
    ]);
    e.ok(&["lier", &n1, "classe-dans", &c_drv]);
    e.ok(&["lier", &n1, "depend-de", &p_e2a]);
    e.ok(&["lier", &n1, "depend-de", &p_e3a]);
    e.ok(&[
        "contredire",
        &n1,
        &p_e3b,
        "--justification",
        "monocœur, section critique courte",
    ]);
    let n2 = e.id(
        &[
            "note",
            "creer",
            "--role",
            "implementation",
            "--titre",
            "N-DRV-2",
            "--texte",
            "Un événement arrivé avant l’attente est conservé",
        ],
        "Note créée : ",
    );
    e.ok(&["decision", &n2, "retenue", "--motif", "poignée de main"]);
    e.ok(&[
        "implementation",
        &n2,
        "en-cours",
        "--motif",
        "refresh() consomme l’événement sans vérifier qu’une tâche attend",
    ]);
    e.ok(&["lier", &n2, "classe-dans", &c_drv]);
    e.ok(&["lier", &n2, "depend-de", &n1]);

    // Étape 5 : réserves.
    let p_lost = e.id(
        &[
            "citer",
            &e1,
            "--texte",
            "C’est là que peut se produire un réveil perdu.",
        ],
        "Citation créée : ",
    );
    let p_ack = e.id(
        &[
            "citer",
            &e1,
            "--texte",
            "sur ce composant, GPIOICR acquitte les bits auxquels on écrit 1",
        ],
        "Citation créée : ",
    );
    let p_tick = e.id(&["citer", &e1, "--texte", "une tâche peut attendre presque une période de tick après l’interruption avant d’être rendue prête"], "Citation créée : ");
    let r1 = e.id(
        &[
            "reserve",
            "creer",
            &p_lost,
            "--titre",
            "Réveil perdu entre vérification et blocage",
        ],
        "Réserve créée : ",
    );
    e.ok(&["reserve", "concerne", &r1, &n2]);
    // Contrat corrigé : traiter change le statut de la réserve, le motif est donc obligatoire
    // (D12) ; la section 8 de CLI.md l'omettait.
    e.ok(&[
        "reserve",
        "traiter",
        &n2,
        &r1,
        "--motif",
        "protocole de poignée de main",
    ]);
    let r2 = e.id(
        &[
            "reserve",
            "creer",
            &p_ack,
            "--titre",
            "Acquittement GPIOICR : bits écrits à 1",
            "--cible",
            "startup.s (à créer)",
        ],
        "Réserve créée : ",
    );
    let r3 = e.id(
        &[
            "reserve",
            "creer",
            &p_tick,
            "--titre",
            "Latence d’un tick : paramètre à fixer",
        ],
        "Réserve créée : ",
    );
    e.ok(&["reserve", "concerne", &r3, &n1]);

    // Étape 6 : ancrages ; le fichier du driver n'est pas modifié.
    let driver = e.fichier("driver.c", DRIVER);
    let ancrage = e.ok(&[
        "ancrer",
        &n1,
        "--fichier",
        &driver,
        "--fonction",
        "void wake_up(int port, int pin)",
    ]);
    let repere = ancrage
        .lines()
        .find_map(|l| l.strip_prefix("Repère à copier : "))
        .unwrap()
        .to_owned();
    assert_eq!(repere, format!("// @design: {n1}"));
    e.ok(&[
        "ancrer",
        &n2,
        "--fichier",
        &driver,
        "--fonction",
        "void refresh()",
    ]);
    assert_eq!(std::fs::read_to_string(&driver).unwrap(), DRIVER);

    // Étape 7 : reprise depuis le repère de wake_up(), tel qu'il apparaîtrait dans le code.
    let reprise = e.ok(&["repere", &format!("    {repere}")]);
    for attendu in [
        format!("Note : N-DRV-1 ({n1})"),
        "Décision : retenue".to_owned(),
        format!("« Ton idée d’organiser les attentes par port, puis par broche, est pertinente. » ({p_e2a})"),
        format!("({p_e3a})"),
        format!("({p_e3b}) — monocœur, section critique courte"),
        format!("Latence d’un tick : paramètre à fixer ({r3}) — ouverte"),
        format!("Réveil perdu entre vérification et blocage ({r1}) — traitée à vérifier"),
        format!("Acquittement GPIOICR : bits écrits à 1 ({r2}) — ouverte — cible : startup.s (à créer)"),
        format!("{} (actif)", driver),
    ] {
        assert!(reprise.contains(&attendu), "« {attendu} » absent de :\n{reprise}");
    }
    let r3_ligne = reprise.find(&format!("({r3})")).unwrap();
    let categories = reprise.find("Réserves de ses catégories :").unwrap();
    assert!(
        r3_ligne < categories,
        "R-3 vise la note elle-même :\n{reprise}"
    );

    // Étape 8 : changement d'avis, critique : rien sans --oui, puis branche créée.
    let (code, _, _) = e.lancer(&[
        "decision",
        &n1,
        "plus-retenue",
        "--motif",
        "primitive commune",
    ]);
    assert_eq!(code, 4);
    let branche = e.id(
        &[
            "decision",
            &n1,
            "plus-retenue",
            "--motif",
            "primitive commune",
            "--oui",
        ],
        "Branche créée : ",
    );
    let statut = e.ok(&["statut"]);
    assert!(
        statut.contains(&format!("Branche active : {branche}")),
        "{statut}"
    );
    assert!(
        statut.contains(&format!("N-DRV-2 ({n2}) — cause : N-DRV-1 ({n1})")),
        "{statut}"
    );

    // Étape 9 : acquittement, révision substantielle de la cause, puis trois annulations.
    e.ok(&[
        "acquitter",
        &n2,
        &n1,
        "--motif",
        "toujours valable avec la primitive commune",
        "--oui",
    ]);
    assert!(e.ok(&["statut"]).contains("À réexaminer :\n  (aucune)"));
    e.ok(&[
        "note",
        "reviser",
        &n1,
        "--texte",
        "Primitive commune du noyau",
        "--nature",
        "substantielle",
        "--oui",
    ]);
    assert!(e
        .ok(&["statut"])
        .contains(&format!("N-DRV-2 ({n2}) — cause : N-DRV-1 ({n1})")));
    for _ in 0..3 {
        e.ok(&["annuler"]);
    }
    let statut = e.ok(&["statut"]);
    assert!(statut.contains("(main)"), "{statut}");
    assert!(statut.contains("À réexaminer :\n  (aucune)"), "{statut}");
    assert!(e.ok(&["repere", &repere]).contains("Décision : retenue"));
}
