"""Cohérence documentaire : chaque identifiant cité doit être défini.

Les défauts S1 à S11 sont soit actifs (registre de memoire-domain), soit corrigés
(table « Défauts corrigés » d'INVARIANTS.md), jamais les deux ni aucun des deux.

- INV-… cités dans SCENARIOS.md, DECISIONS.md, TRACABILITE.md et les tests : définis dans le registre d'INVARIANTS.md ;
- S-… cités : définis par un titre de SCENARIOS.md ;
- D1 à D9 cités : définis par une fiche de DECISIONS.md ;
- défauts S1 à S11 cités : présents dans le registre des défauts connus.
Sortie non nulle au premier écart.
"""
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1]
read = lambda name: (root / name).read_text(encoding="utf-8")
inv_doc, scen, dec, trace = read("INVARIANTS.md"), read("SCENARIOS.md"), read("DECISIONS.md"), read("TRACABILITE.md")
tests = "".join(p.read_text(encoding="utf-8") for p in sorted(root.glob("crates/*/tests/*.rs")))
defects = read("crates/memoire-domain/tests/defauts_connus.rs")

defined_inv = set(re.findall(r"^\| (INV-[A-Z]+-\d+) \|", inv_doc, re.M))
defined_scen = set(re.findall(r"^#{2,3} (S-[A-Z]+-\d+)", scen, re.M))
defined_dec = set(re.findall(r"^## (D\d+) ", dec, re.M))
defined_defects = set(re.findall(r"fn s(\d+)_", defects))
active_defects = set(defined_defects)
fixed_section = inv_doc.split("### Défauts corrigés", 1)[1].split("\n### ", 1)[0] if "### Défauts corrigés" in inv_doc else ""
fixed_defects = set(re.findall(r"^\| S(\d+) \|", fixed_section, re.M))
defined_defects = active_defects | fixed_defects

errors = []
for name, text in [("SCENARIOS.md", scen), ("DECISIONS.md", dec), ("TRACABILITE.md", trace), ("tests", tests), ("INVARIANTS.md", inv_doc)]:
    for inv in sorted(set(re.findall(r"INV-[A-Z]+-\d+", text)) - defined_inv):
        errors.append(f"{name} : {inv} non défini dans le registre")
    for sc in sorted(set(re.findall(r"S-[A-Z]+-\d+", text)) - defined_scen):
        errors.append(f"{name} : scénario {sc} non défini")
    for d in sorted(set(re.findall(r"\bD(\d+)\b", text)) - {x[1:] for x in defined_dec}):
        errors.append(f"{name} : décision D{d} non définie")
for name, text in [("INVARIANTS.md", inv_doc), ("DECISIONS.md", dec), ("SCENARIOS.md", scen)]:
    for n in sorted(set(re.findall(r"\bS(\d+)\b", text)) - defined_defects, key=int):
        errors.append(f"{name} : défaut S{n} absent du registre des défauts connus")

for n in sorted(active_defects & fixed_defects, key=int):
    errors.append(f"défaut S{n} à la fois actif et corrigé")
missing = set(map(str, range(1, max(map(int, defined_defects)) + 1))) - defined_defects
for n in sorted(missing, key=int):
    errors.append(f"défaut S{n} ni actif ni corrigé : il a disparu du suivi")

print(f"{len(defined_inv)} invariants, {len(defined_scen)} scénarios, {len(defined_dec)} décisions, "
      f"{len(active_defects)} défauts actifs, {len(fixed_defects)} corrigés.")
if errors:
    print("\n".join(errors))
    sys.exit(1)
print("Références documentaires cohérentes.")
