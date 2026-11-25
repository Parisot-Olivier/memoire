//! Intégrité des données d'entrée du scénario S-REF-01 (SCENARIOS.md).
//! Ce fichier ne teste pas le noyau : il garantit que les passages cités par le scénario
//! existent, une seule fois, dans la source brute figée. Toute modification du jeu d'essai
//! le fait échouer ; les empreintes SHA-256 sont dans fixtures/rtos-2026-10/SHA256SUMS.

const DOC: &str = include_str!("../../../fixtures/rtos-2026-10/doc.md");
const DRIVER: &str = include_str!("../../../fixtures/rtos-2026-10/driver.c");

/// Passages de S-REF-01 : nom symbolique et texte exact dans la source brute.
pub const PASSAGES: [(&str, &str); 9] = [
    ("P-E1-4", "une tâche peut attendre presque une période de tick après l’interruption avant d’être rendue prête"),
    ("P-E1-5", "C’est là que peut se produire un réveil perdu."),
    ("P-E1-ack", "sur ce composant, GPIOICR acquitte les bits auxquels on écrit 1"),
    ("P-E1-cite", "01-LM3S9B92-DataSheet.pdf\u{2060}\u{fffc}"),
    ("P-E2a", "Ton idée d’organiser les attentes par port, puis par broche, est pertinente."),
    ("P-E3a", "Une tâche bloquée peut être référencée directement dans les structures d’attente du périphérique, puis rendue à l’ordonnanceur lors du réveil."),
    ("P-E3b", "le gestionnaire d’attente identifie la tâche à réveiller, puis une primitive commune réalise le transfert vers PENDING."),
    ("P-E3c", "En multicœur, cependant, une écriture atomique de l’état ne suffit pas à garantir ce transfert."),
    ("P-E4", "Ma réponse précédente supposait plusieurs cœurs capables d’ordonnancer simultanément."),
];

#[test]
fn raw_sources_are_frozen() {
    assert_eq!(DOC.len(), 20_392);
    assert_eq!(DRIVER.len(), 4_263);
    // Artefacts de copier-coller que la conservation octet pour octet doit garder (INV-SOURCE-03).
    assert_eq!(DOC.matches('\t').count(), 30);
    assert_eq!(DOC.matches('\u{a0}').count(), 7);
    assert_eq!(DOC.matches('\u{fffc}').count(), 1);
    assert_eq!(DOC.matches('\u{2060}').count(), 1);
    assert!(!DOC.contains('\r'));
}

#[test]
fn exchanges_markers_are_ambiguous_as_documented() {
    // Cinq échanges ; orthographe variable du marqueur de réponse.
    assert_eq!(DOC.matches("** question").count(), 5);
    assert_eq!(DOC.matches("** réponse").count(), 1);
    assert_eq!(DOC.matches("** reponse").count(), 4);
    // Le marqueur de titre « * » est aussi celui des listes à puces des réponses (INV-SOURCE-04).
    let star_lines = DOC.lines().filter(|l| l.starts_with("* ")).count();
    assert_eq!(star_lines, 16); // 1 titre + 15 puces
}

#[test]
fn every_scenario_passage_occurs_exactly_once() {
    for (name, text) in PASSAGES {
        assert_eq!(DOC.matches(text).count(), 1, "{name}");
    }
}

#[test]
fn code_anchors_of_the_scenario_exist() {
    for symbol in ["void wake_up(int port, int pin)", "void refresh()"] {
        assert_eq!(DRIVER.matches(symbol).count(), 1, "{symbol}");
    }
    // Repère manuel actuel : il recopie le titre de la conversation (INV-CODE-02 le remplace).
    assert!(DRIVER.contains("//voir dans doc.md pourquoi cette decision"));
}
