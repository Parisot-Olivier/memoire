"""Contrôle ciblé : deux régressions historiques doivent déclencher les tests prévus.

Elles sont injectées dans memoire-domain, la crate en développement ; le prototype
memoire-core a été retiré. Exécuter avec python3 tools/check_mutations.py ; Cargo doit
être disponible. Aucun fichier source du projet n'est modifié.
"""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile

project = Path(__file__).resolve().parents[1]
cargo = os.environ.get("CARGO", "cargo")
crate = "crates/memoire-domain"
mutations = [
    ("publication_dans_branche_origine", f"{crate}/src/repository.rs",
     # Point d'injection adapté à l'incrément 5.2b (écritures préparées) : même régression,
     # la bifurcation n'a jamais lieu et la publication reste dans la branche d'origine.
     "let (destination, branch) = if forks {", "let (destination, branch) = if false {",
     "critical_change_forks_before_publish"),
    # Point d'injection adapté à l'incrément 5.5 : la règle de propagation est factorisée dans
    # Relation::propagates, que suivent le réexamen et les orphelines.
    ("propagation_liens_thematiques", f"{crate}/src/model.rs",
     "matches!(self, Relation::DependsOn | Relation::DevelopedFrom)",
     "matches!(self, Relation::DependsOn | Relation::DevelopedFrom | Relation::Related)",
     "invalidating_a_subidea_keeps_architecture_category_and_sibling"),
]
report = ["Contrôle ciblé de sensibilité des tests (memoire-domain)",
          "Deux régressions sont injectées dans des copies temporaires, jamais dans le projet livré.", ""]
for name, path, before, after, test in mutations:
    original = (project / path).read_text()
    if original.count(before) != 1:
        raise RuntimeError(f"Point d'injection à adapter explicitement après refonte : {name}")
    with tempfile.TemporaryDirectory(prefix="memoire-mutation-") as directory:
        candidate = Path(directory) / "project"
        shutil.copytree(project, candidate, ignore=shutil.ignore_patterns("target", "mutants.out*", ".git", "__pycache__"))
        (candidate / path).write_text(original.replace(before, after))
        run = subprocess.run([cargo, "test", "--offline", "-p", "memoire-domain", "--test", "contrat", test],
                             cwd=candidate, text=True, capture_output=True)
        output = run.stdout + run.stderr
        if not (run.returncode != 0 and "test result: FAILED" in output and f"{test} ... FAILED" in output):
            raise RuntimeError(f"La régression n'a pas été détectée par le test attendu : {name}\n{output}")
        report += [f"{name} : régression détectée par {test}.", f"Code de sortie : {run.returncode}", ""]
(project / "VERIFICATION-MUTATIONS.txt").write_text("\n".join(report))
print("\n".join(report))
