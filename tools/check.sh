#!/usr/bin/env bash
# Contrôle unique du noyau. S'arrête au premier écart ; aucun contrôle n'est sauté en silence.
#
# Codes de sortie : 0 = complet et réussi ; 2 = outil manquant ;
# 3 = contrôle incomplet (mutation, ou étapes rapides, sautées à la demande) ; autre = échec.
#
# CHECK_SKIP_MUTANTS=1 saute la mutation. CHECK_MUTATION_SEULE=1 saute les étapes rapides
# (formatage, documents, dépendances, audit, lints, tests, interface) et ne lance que la mutation
# et les régressions ciblées : utile quand l'environnement interrompt les longues exécutions. Le
# contrôle complet reste l'exécution sans ces variables.
#
# La cohérence documentaire vérifie que chaque identifiant cité (INV, scénario, décision, défaut) est défini.
# Le test de mutation vise memoire-domain (contrat : tests internes + tests/contrat.rs ; le registre
# des défauts connus ne compte pas comme protection) puis memoire-store (tests/stockage.rs).
# Les dépendances sont contrôlées (D14) et auditées contre la base RustSec.
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n== %s\n' "$1"; }
require() {
    if ! command -v "$1" >/dev/null 2>&1 && ! cargo "$2" --version >/dev/null 2>&1; then
        echo "ERREUR : $1 introuvable. $3" >&2
        exit 2
    fi
}

command -v cargo >/dev/null 2>&1 || { echo "ERREUR : cargo introuvable (Rust 1.91 minimum)." >&2; exit 2; }
require rustfmt fmt "Installer le composant rustfmt."
require cargo-clippy clippy "Installer le composant clippy."
# Les essais de la fiche de reprise (6.5b, D26) créent des dépôts Git d'essai.
command -v git >/dev/null 2>&1 || { echo "ERREUR : git introuvable (essais de la fiche de reprise, D26)." >&2; exit 2; }

mutation_seule="${CHECK_MUTATION_SEULE:-0}"
if [[ "$mutation_seule" == "1" ]]; then
    echo "Étapes rapides sautées (CHECK_MUTATION_SEULE=1) : mutation et régressions ciblées seules."
else

step "Formatage"
cargo fmt --check

step "Cohérence documentaire : invariants, scénarios, décisions, défauts"
python3 tools/check_docs.py
# Base de schéma 1 écrite par v0.5.0-alpha.1 : figée octet pour octet (migration, 5.2d).
(cd fixtures/schema1 && sha256sum --check --quiet SHA256SUMS)

step "Dépendances : politique D14 (versions figées, publiées depuis au moins 14 jours)"
python3 tools/deps.py

step "Sécurité : alertes RustSec (cargo audit, avertissements bloquants)"
require cargo-audit audit "Installer avec : cargo install --locked cargo-audit --version 0.22.2"
# L'audit télécharge la base d'alertes : sans réseau, il échoue, et le contrôle aussi. Un audit
# non exécuté n'est jamais compté comme réussi.
if ! cargo audit --deny warnings; then
    echo "ÉCHEC de l'audit : vulnérabilité ou avertissement signalé, ou base d'alertes inaccessible." >&2
    exit 1
fi

step "Lints : clippy, avertissements bloquants"
cargo clippy --workspace --all-targets --offline -- -D warnings

step "Tests : debug"
cargo test --workspace --offline

step "Tests : release"
cargo test --workspace --release --offline

fi

if [[ "${CHECK_SKIP_MUTANTS:-0}" == "1" ]]; then
    echo
    echo "INCOMPLET : test de mutation sauté (CHECK_SKIP_MUTANTS=1). Ne pas considérer ce contrôle comme validé." >&2
    exit 3
fi

require cargo-mutants mutants "Installer avec : cargo install --locked cargo-mutants"

if [[ "$mutation_seule" != "1" ]]; then
step "Interface : lecture de l'aide (sans Tkinter), puis essai graphique sous affichage virtuel"
# Le moteur du terminal (6.7, D27) lance de vrais shells et l'exécutable : il est construit d'abord.
cargo build --release --locked --offline -p memoire-cli
(cd gui && MEMOIRE="$(pwd)/../target/release/memoire" python3 -m unittest -v test_schema test_rendu \
    test_voisinage test_terminal_moteur test_notification test_plan_de_travail test_terminaux \
    test_actions test_dessin test_apparence test_mosaique)
# Sous KWin virtuel s'il est installé (un vrai gestionnaire de fenêtres : ordre des fenêtres
# flottantes, 6.8) ; sinon sous Xvfb, où l'essai de l'ordre des fenêtres est sauté.
if command -v kwin_wayland >/dev/null && python3 -c "import tkinter" 2>/dev/null; then
    MEMOIRE="$(pwd)/target/release/memoire" tools/essais-sous-kwin.sh
elif command -v xvfb-run >/dev/null && python3 -c "import tkinter" 2>/dev/null; then
    echo "KWin absent : essais sous Xvfb, sans gestionnaire de fenêtres (ordre des fenêtres non vérifié)."
    (cd gui && MEMOIRE="$(pwd)/../target/release/memoire" xvfb-run -a python3 -m unittest -v test_gui)
else
    echo "Essai graphique ignoré : xvfb-run ou Tkinter absent ; la lecture de l'aide reste vérifiée."
fi
fi

# Les copies de travail de cargo-mutants restent dans le répertoire temporaire quand un passage
# est interrompu (plusieurs centaines de Mo chacune) ; elles ont déjà rempli le disque. Elles sont
# supprimées avant chaque passage, sauf si CHECK_GARDER_TMP=1.
if [[ "${CHECK_GARDER_TMP:-0}" != "1" ]]; then
    rm -rf "${TMPDIR:-/tmp}"/cargo-mutants-*
fi

# Pendant la mutation, chaque binaire de test est lancé sous un plafond d'espace d'adressage
# (tools/lanceur-mutation.sh) : un mutant qui fait grossir la mémoire sans fin a déjà épuisé la
# mémoire de la machine (24,7 Go) avant son délai. Le compilateur n'est pas plafonné. Sur une
# machine dont /tmp est en mémoire vive, placer aussi TMPDIR sur disque : les copies de travail
# de cargo-mutants y sont créées.
# Les bases d'essai, elles, restent dans CHECK_TMPDIR_TESTS (/tmp par défaut) : voir
# tools/lanceur-mutation.sh.
cible=$(rustc -vV | sed -n 's/^host: //p' | tr 'a-z-' 'A-Z_')
export "CARGO_TARGET_${cible}_RUNNER=$(pwd)/tools/lanceur-mutation.sh"
export CHECK_TMPDIR_TESTS="${CHECK_TMPDIR_TESTS:-/tmp}"

step "Mutation de memoire-domain : contrat seul, aucun mutant survivant toléré (plusieurs minutes)"
# Reprise possible : la mutation est découpée en CHECK_MUTANTS_SHARDS parts (1 par défaut), qui
# couvrent ensemble tous les mutants. Une part réussie est consignée dans mutants.parts/ sous une
# clé calculée sur tout ce qui peut changer son résultat (code, tests, données d'entrée, verrou
# Cargo) ; elle n'est sautée que si cette clé est identique. Toute modification relance tout.
# CHECK_MUTANTS_DIR permet de conserver les parts réussies hors de la copie de travail.
shards="${CHECK_MUTANTS_SHARDS:-1}"
# La clé couvre exactement ce que la mutation exécute : sources, contrat (tests/contrat.rs et son
# harnais), manifeste, données d'entrée et verrou Cargo. Les autres tests ne sont pas exécutés par
# la mutation et ne peuvent pas changer son résultat.
# Le domaine n'a aucune dépendance (tests/architecture.rs le vérifie) : seule son entrée dans
# Cargo.lock compte, et l'ajout d'une dépendance ailleurs ne relance pas sa mutation.
key=$( (grep -A1 '^name = "memoire-domain"$' Cargo.lock; cat crates/memoire-domain/Cargo.toml;
        find crates/memoire-domain/src crates/memoire-domain/tests/contrat.rs \
             crates/memoire-domain/tests/harness fixtures -type f | LC_ALL=C sort | xargs cat) |
      sha256sum | cut -c1-16)
parts="${CHECK_MUTANTS_DIR:-mutants.parts}"
mkdir -p "$parts"
# Toutes les parts sont exécutées, même après un échec, pour rassembler tous les survivants en un
# seul passage ; le contrôle échoue ensuite si une part a échoué.
failed=0
for ((k = 0; k < shards; k++)); do
    marker="$parts/$key-$k-sur-$shards.ok"
    if [[ -f "$marker" ]]; then
        echo "Part $k/$shards déjà réussie pour la clé $key : $(cat "$marker")"
        continue
    fi
    rm -rf "$parts/sortie-$k"
    if cargo mutants -p memoire-domain --jobs "${CHECK_JOBS:-4}" --timeout 60 --shard "$k/$shards" \
        -o "$parts/sortie-$k" -- --offline --lib --test contrat |
        tee "$parts/sortie-$k.txt"; then
        grep "mutants tested" "$parts/sortie-$k.txt" > "$marker"
    else
        echo "ÉCHEC de la part $k/$shards : survivants ci-dessus (lignes MISSED)." >&2
        failed=1
    fi
done
if [[ "$failed" -ne 0 ]]; then
    echo "Mutation : au moins une part en échec ; les parts réussies sont consignées." >&2
    exit 1
fi

step "Mutation de memoire-store : contrat du stockage, aucun mutant survivant toléré"
# Une seule part : la crate est courte. Sa clé couvre son code, ses tests, le code du domaine dont
# elle dépend, la base figée de schéma 1 que lisent ses tests et le verrou Cargo.
store_key=$( (cat Cargo.lock crates/memoire-store/Cargo.toml;
              find crates/memoire-store/src crates/memoire-store/tests crates/memoire-domain/src fixtures/schema1 \
                   -type f | LC_ALL=C sort | xargs cat) | sha256sum | cut -c1-16)
store_marker="$parts/stockage-$store_key.ok"
if [[ -f "$store_marker" ]]; then
    echo "Mutation du stockage déjà réussie pour la clé $store_key : $(cat "$store_marker")"
else
    rm -rf "$parts"/sortie-stockage
    cargo mutants -p memoire-store --jobs "${CHECK_JOBS:-4}" --timeout 120 \
        -o "$parts"/sortie-stockage -- --offline --lib --test stockage |
        tee "$parts"/sortie-stockage.txt
    grep "mutants tested" "$parts"/sortie-stockage.txt > "$store_marker"
fi

step "Mutation de memoire-app : contrat de la couche service, aucun mutant survivant toléré"
app_key=$( (cat Cargo.lock crates/memoire-app/Cargo.toml;
            find crates/memoire-app/src crates/memoire-app/tests crates/memoire-store/src fixtures/schema1 \
                 crates/memoire-domain/src -type f | LC_ALL=C sort | xargs cat) | sha256sum | cut -c1-16)
app_marker="$parts/service-$app_key.ok"
if [[ -f "$app_marker" ]]; then
    echo "Mutation du service déjà réussie pour la clé $app_key : $(cat "$app_marker")"
else
    rm -rf "$parts"/sortie-service
    # Exclusion justifiée (VERIFICATION-PHASE3.txt) : sync_directory ne fait que synchroniser un
    # répertoire sur disque après le renommage ; son effet ne se manifeste qu'en cas de coupure
    # d'alimentation, impossible à provoquer ici. La remplacer par Ok(()) ne change rien
    # d'observable : un test qui prétendrait la vérifier serait trompeur.
    cargo mutants -p memoire-app --jobs "${CHECK_JOBS:-4}" --timeout 120 \
        --exclude-re 'sync_directory' \
        -o "$parts"/sortie-service -- --offline --lib --test service |
        tee "$parts"/sortie-service.txt
    grep "mutants tested" "$parts"/sortie-service.txt > "$app_marker"
fi

step "Mutation de memoire-cli : tous ses tests (contrat, recette, refus), aucun mutant survivant toléré"
cli_key=$( (cat Cargo.lock crates/memoire-cli/Cargo.toml;
            find crates/memoire-cli/src crates/memoire-cli/tests crates/memoire-app/src fixtures/schema1 \
                 crates/memoire-store/src crates/memoire-domain/src -type f | LC_ALL=C sort |
            xargs cat) | sha256sum | cut -c1-16)
# En plusieurs parts, consignées une à une : un passage interrompu reprend là où il s'est arrêté.
cli_shards="${CHECK_CLI_SHARDS:-4}"
cli_failed=0
for ((k = 0; k < cli_shards; k++)); do
    cli_marker="$parts/cli-$cli_key-$k-sur-$cli_shards.ok"
    if [[ -f "$cli_marker" ]]; then
        echo "Part $k/$cli_shards de la ligne de commande déjà réussie pour la clé $cli_key : $(cat "$cli_marker")"
        continue
    fi
    rm -rf "$parts/sortie-cli-$k"
    if cargo mutants -p memoire-cli --jobs "${CHECK_JOBS:-4}" --timeout 120 --shard "$k/$cli_shards" \
        -o "$parts/sortie-cli-$k" -- --offline --tests |
        tee "$parts/sortie-cli-$k.txt"; then
        grep "mutants tested" "$parts/sortie-cli-$k.txt" > "$cli_marker"
    else
        echo "ÉCHEC de la part $k/$cli_shards de la ligne de commande : survivants ci-dessus." >&2
        cli_failed=1
    fi
done
if [[ "$cli_failed" -ne 0 ]]; then
    exit 1
fi

step "Régressions ciblées historiques"
python3 tools/check_mutations.py

if [[ "$mutation_seule" == "1" ]]; then
    echo
    echo "INCOMPLET : mutation et régressions ciblées réussies, mais étapes rapides sautées (CHECK_MUTATION_SEULE=1). Ne pas considérer ce contrôle comme validé." >&2
    exit 3
fi
printf '\nCONTRÔLE COMPLET : réussi.\n'
