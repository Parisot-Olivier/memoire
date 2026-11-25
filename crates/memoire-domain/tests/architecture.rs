//! Garde d'architecture (INV-CODE-04, D9) : le domaine n'a ni dépendance externe ni entrée-sortie.
//! Il ne peut donc ni écrire dans le code de l'utilisateur, ni restaurer ses fichiers, ni dépendre
//! d'un fournisseur. Ce test lit les sources ; il ne compte pas dans le test de mutation.

use std::fs;
use std::path::Path;

const FORBIDDEN: [&str; 6] = [
    "std::fs",
    "std::net",
    "std::process",
    "std::env",
    "std::io",
    "File::",
];

#[test]
fn the_domain_performs_no_input_output() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    for entry in fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for token in FORBIDDEN {
            assert!(!text.contains(token), "{} emploie {token}", path.display());
        }
        checked += 1;
    }
    assert!(checked >= 6, "sources du domaine introuvables");
}

#[test]
fn the_domain_has_no_dependency() {
    let manifest =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let after = manifest.split("[dependencies]").nth(1).unwrap();
    let declared = after
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .count();
    assert_eq!(declared, 0, "le domaine doit rester sans dépendance");
    assert!(!manifest.contains("[dev-dependencies]") || manifest.contains("proptest"));
}
