#!/bin/sh
# Lanceur des binaires de test pendant la mutation (CARGO_TARGET_<cible>_RUNNER, posé par
# check.sh) ; le compilateur n'y passe pas.
#
# 1. Plafond d'espace d'adressage : un mutant qui fait grossir la mémoire sans fin échoue sur une
#    allocation refusée, et compte comme détecté, au lieu d'épuiser la mémoire de la machine (le
#    6 novembre 2026, un tel mutant a atteint 24,7 Go avant d'être tué par le noyau). Le plus gros
#    binaire de test réside sous 40 Mo ; le plafond, en kio, se règle par CHECK_PLAFOND_KIO.
# 2. Répertoire temporaire des tests : TMPDIR peut viser un disque pour les copies de travail de
#    cargo-mutants (plusieurs centaines de Mo chacune) ; les bases d'essai, petites mais écrites
#    avec synchronous=FULL, vont dans CHECK_TMPDIR_TESTS (posé par check.sh, /tmp par défaut),
#    sans quoi chaque commit coûte un vrai fsync (contrat du stockage : 17 s au lieu de 2 s).
ulimit -v "${CHECK_PLAFOND_KIO:-4194304}" || exit 2
if [ -n "${CHECK_TMPDIR_TESTS:-}" ]; then
    TMPDIR="$CHECK_TMPDIR_TESTS"
    export TMPDIR
fi
exec "$@"
