#!/usr/bin/env python3
"""Contrôle de la politique de dépendances (D14).

Sans option : contrôle hors ligne. Chaque paquet du registre présent dans Cargo.lock doit figurer,
avec la même version, dans DEPENDANCES.md, et y avoir été publié au moins 14 jours avant la date
d'audit inscrite. Le domaine (memoire-domain) reste sans dépendance : tests/architecture.rs le
vérifie.

Avec --refresh : interroge crates.io (réseau requis), relève la date de publication de chaque paquet
verrouillé et réécrit DEPENDANCES.md. Un paquet trop récent est signalé ; il faut alors le ramener à
une version plus ancienne (cargo update -p nom@version --precise ancienne) et relancer.
"""
from datetime import date, timedelta
from pathlib import Path
import json
import re
import sys
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
LOCK = ROOT / "Cargo.lock"
REGISTER = ROOT / "DEPENDANCES.md"
MIN_AGE = timedelta(days=14)


def locked():
    packages = []
    for block in LOCK.read_text().split("[[package]]"):
        name = re.search(r'^name = "([^"]+)"', block, re.M)
        version = re.search(r'^version = "([^"]+)"', block, re.M)
        if name and version and "source = \"registry+" in block:
            packages.append((name.group(1), version.group(1)))
    return sorted(packages)


def refresh():
    today = date.today()
    rows, young = [], []
    for name, version in locked():
        request = urllib.request.Request(
            f"https://crates.io/api/v1/crates/{name}/{version}",
            headers={"User-Agent": "memoire-deps-audit"},
        )
        published = json.load(urllib.request.urlopen(request, timeout=20))["version"]["created_at"][:10]
        rows.append((name, version, published))
        if today - date.fromisoformat(published) < MIN_AGE:
            young.append(f"{name} {version} ({published})")
    lines = [
        "# Dépendances verrouillées — registre d’audit (D14)",
        "",
        f"Date d’audit : {today.isoformat()}. Généré par `python3 tools/deps.py --refresh` ; contrôlé hors ligne par `python3 tools/deps.py`.",
        "",
        "| Paquet | Version | Publication |",
        "|---|---|---|",
    ] + [f"| {n} | {v} | {d} |" for n, v, d in rows]
    REGISTER.write_text("\n".join(lines) + "\n")
    if young:
        sys.exit("Publiés depuis moins de 14 jours : " + ", ".join(young))
    print(f"{len(rows)} paquets audités, tous publiés depuis au moins 14 jours.")


def check():
    text = REGISTER.read_text()
    audit = date.fromisoformat(re.search(r"Date d’audit : (\d{4}-\d{2}-\d{2})", text).group(1))
    listed = {(n, v): d for n, v, d in re.findall(r"^\| ([^ |]+) \| ([^ |]+) \| (\d{4}-\d{2}-\d{2}) \|$", text, re.M)}
    errors = []
    for name, version in locked():
        published = listed.get((name, version))
        if published is None:
            errors.append(f"{name} {version} absent du registre d’audit")
        elif audit - date.fromisoformat(published) < MIN_AGE:
            errors.append(f"{name} {version} publié le {published}, moins de 14 jours avant l’audit")
    extra = set(listed) - set(locked())
    errors += [f"{n} {v} au registre mais plus verrouillé" for n, v in sorted(extra)]
    if errors:
        sys.exit("\n".join(errors))
    print(f"{len(listed)} dépendances conformes à D14 (audit du {audit}).")


if __name__ == "__main__":
    refresh() if "--refresh" in sys.argv else check()
