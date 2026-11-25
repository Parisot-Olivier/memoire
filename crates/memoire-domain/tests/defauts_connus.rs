//! Registre des défauts connus de memoire-domain.
//!
//! Chaque test décrit un comportement ACTUEL jugé incorrect ; il passe tant que le défaut existe
//! et ne compte pas comme protection. Protocole : lorsqu'un correctif fait échouer un de ces tests,
//! le supprimer ici et ajouter, dans un commit dédié, le test du comportement correct dans
//! tests/contrat.rs, en citant l'invariant et la décision ; reporter le défaut dans la table
//! « Défauts corrigés » d'INVARIANTS.md.
//!
//! Portés de memoire-core avec le harnais. Corrigés : S2, S7 et S9 (incrément 2.1),
//! S1, S3 et S10 (incrément 2.2), S5 et S6 (incrément 2.3a), S4 et S11 (incrément 2.4),
//! S8 (incrément 2.6a) ; voir tests/contrat.rs. Aucun défaut actif : le fichier reste le lieu où
//! consigner un défaut découvert, selon le même protocole.
