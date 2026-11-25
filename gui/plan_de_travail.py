"""Modèle du plan de travail modulaire (PLAN-DE-TRAVAIL.md, sections 1 à 3 ; D28 ; INV-PLAN-01).

Aucune importation de tkinter : la `Disposition` est la seule source de vérité sur la place de
chaque panneau (ancré dans une zone, flottant, masqué) ; la réalisation (gui/panneaux.py) applique
ses changements. La disposition se conserve dans la configuration de l'utilisateur, hors du dépôt.
"""
from dataclasses import dataclass, field, replace
import json
import os
from pathlib import Path
import re
import tempfile

ZONES = ("haut", "bas", "gauche", "droite")
# « page » (8.5, CONSOLE.md) : le panneau prend toute la carte ; un seul à la fois.
ETATS = ("ancre", "flottant", "masque", "page")
RETOURS = ("ancre", "flottant")
VERSION = 1
PARTS_PAR_DEFAUT = {"haut": 0.25, "bas": 0.25, "gauche": 0.20, "droite": 0.30}
PART_MIN, PART_MAX = 0.05, 0.9
GEOMETRIE_PAR_DEFAUT = "720x420+120+120"
GEOMETRIE = re.compile(r"^(\d+)x(\d+)\+?(-?\d+)\+?(-?\d+)$")


@dataclass(frozen=True)
class Defaut:
    """Place par défaut d'un panneau."""

    zone: str
    rang: int
    part: float = 0.5


@dataclass(frozen=True)
class EtatPanneau:
    """Place d'un panneau. `retour` : état rendu par « montrer » quand il est masqué, et par
    « quitter la page » quand il est en pleine page."""

    etat: str
    zone: str
    rang: int
    part: float = 0.5
    geometrie: str = None
    premier_plan: bool = False
    retour: str = "ancre"


def _valider_geometrie(geometrie):
    if geometrie is not None and not (isinstance(geometrie, str) and GEOMETRIE.match(geometrie)):
        raise ValueError(f"géométrie invalide : {geometrie!r}")


def ramener_dans_l_ecran(geometrie, largeur, hauteur):
    """Géométrie « LxH+X+Y » ramenée entière dans un écran de la taille donnée."""
    l, h, x, y = (int(v) for v in GEOMETRIE.match(geometrie).groups())
    l, h = min(l, largeur), min(h, hauteur)
    x = min(max(0, x), largeur - l)
    y = min(max(0, y), hauteur - h)
    return f"{l}x{h}+{x}+{y}"


@dataclass
class Disposition:
    """Place de chaque panneau connu et part de chaque zone dans la fenêtre."""

    panneaux: dict = field(default_factory=dict)
    zones: dict = field(default_factory=lambda: dict(PARTS_PAR_DEFAUT))

    @classmethod
    def par_defaut(cls, definitions):
        """Chaque panneau masqué, prêt à reprendre sa place par défaut quand il est montré."""
        return cls({nom: EtatPanneau("masque", d.zone, d.rang, d.part)
                    for nom, d in definitions.items()})

    # ----- Panneaux dynamiques (TERMINAUX.md, section 3) -----

    def ajouter(self, nom, defaut):
        """Inscrit un panneau en cours de session, masqué à sa place par défaut."""
        if nom in self.panneaux:
            raise ValueError(f"panneau déjà présent : {nom}")
        if defaut.zone not in ZONES:
            raise ValueError(f"zone inconnue : {defaut.zone}")
        self.panneaux[nom] = EtatPanneau("masque", defaut.zone, defaut.rang, defaut.part)

    def retirer(self, nom):
        """Retire un panneau ; sa zone est renumérotée."""
        self._sortir(nom)
        del self.panneaux[nom]

    # ----- Lecture -----

    def etat(self, nom):
        if nom not in self.panneaux:
            raise ValueError(f"panneau inconnu : {nom}")
        return self.panneaux[nom]

    def panneaux_de(self, zone):
        """Panneaux ancrés dans la zone, par rang."""
        ancres = [(e.rang, nom) for nom, e in self.panneaux.items()
                  if e.etat == "ancre" and e.zone == zone]
        return [nom for _, nom in sorted(ancres)]

    def zones_visibles(self):
        return [z for z in ZONES if self.panneaux_de(z)]

    def part_de_zone(self, zone):
        return self.zones[zone]

    # ----- Opérations -----

    def _poser(self, nom, **changements):
        self.panneaux[nom] = replace(self.etat(nom), **changements)

    def _renumeroter(self, zone, ordre=None):
        for rang, nom in enumerate(ordre if ordre is not None else self.panneaux_de(zone)):
            self._poser(nom, rang=rang)

    def ancrer(self, nom, zone=None, rang=None):
        """Ancre le panneau dans `zone` (par défaut sa dernière zone), au rang donné ou à la fin."""
        etat = self.etat(nom)
        zone = etat.zone if zone is None else zone
        if zone not in ZONES:
            raise ValueError(f"zone inconnue : {zone}")
        self._sortir(nom)
        ordre = [n for n in self.panneaux_de(zone) if n != nom]
        ordre.insert(len(ordre) if rang is None else max(0, min(rang, len(ordre))), nom)
        self._poser(nom, etat="ancre", zone=zone, retour="ancre")
        self._renumeroter(zone, ordre)

    def detacher(self, nom, geometrie=None):
        """Fait flotter le panneau ; sans géométrie, il reprend la dernière connue."""
        _valider_geometrie(geometrie)
        etat = self.etat(nom)
        self._sortir(nom)
        self._poser(nom, etat="flottant", retour="flottant",
                    geometrie=geometrie or etat.geometrie or GEOMETRIE_PAR_DEFAUT)

    def deplacer(self, nom, geometrie):
        _valider_geometrie(geometrie)
        self._poser(nom, geometrie=geometrie)

    def premier_plan(self, nom, actif):
        self._poser(nom, premier_plan=bool(actif))

    def masquer(self, nom):
        etat = self.etat(nom)
        if etat.etat == "masque":
            return
        self._sortir(nom)
        self._poser(nom, etat="masque", retour=etat.retour if etat.etat == "page" else etat.etat)

    def montrer(self, nom):
        """Rend au panneau masqué sa dernière place ; sans effet s'il est visible."""
        etat = self.etat(nom)
        if etat.etat != "masque":
            return
        if etat.retour == "flottant":
            self._poser(nom, etat="flottant")
        else:
            self.ancrer(nom, etat.zone, etat.rang)

    # ----- Pleine page (CONSOLE.md, section 1) -----

    def en_page(self):
        """Nom du panneau en pleine page, ou None."""
        return next((nom for nom, e in self.panneaux.items() if e.etat == "page"), None)

    def mettre_en_page(self, nom):
        """Le panneau prend toute la carte ; celui qui y était revient à sa place."""
        etat = self.etat(nom)
        if etat.etat == "page":
            return
        autre = self.en_page()
        if autre is not None:
            self.quitter_la_page(autre)
        self._sortir(nom)
        self._poser(nom, etat="page", retour=etat.etat if etat.etat in RETOURS else etat.retour)

    def quitter_la_page(self, nom):
        """Rend au panneau en pleine page la place qu'il avait avant ; sans effet sinon."""
        etat = self.etat(nom)
        if etat.etat != "page":
            return
        if etat.retour == "flottant":
            self._poser(nom, etat="flottant", geometrie=etat.geometrie or GEOMETRIE_PAR_DEFAUT)
        else:
            self.ancrer(nom, etat.zone, etat.rang)

    def _sortir(self, nom):
        """Renumérote la zone d'un panneau ancré comme s'il n'y était plus ; l'appelant lui
        donne ensuite son nouvel état."""
        etat = self.etat(nom)
        if etat.etat == "ancre":
            self._renumeroter(etat.zone, [n for n in self.panneaux_de(etat.zone) if n != nom])

    def _deplacer_dans_la_zone(self, nom, pas):
        etat = self.etat(nom)
        if etat.etat != "ancre":
            return
        ordre = self.panneaux_de(etat.zone)
        i = ordre.index(nom)
        j = max(0, min(i + pas, len(ordre) - 1))
        ordre[i], ordre[j] = ordre[j], ordre[i]
        self._renumeroter(etat.zone, ordre)

    def monter(self, nom):
        self._deplacer_dans_la_zone(nom, -1)

    def descendre(self, nom):
        self._deplacer_dans_la_zone(nom, 1)

    def redimensionner_zone(self, zone, part):
        if zone not in ZONES:
            raise ValueError(f"zone inconnue : {zone}")
        self.zones[zone] = min(PART_MAX, max(PART_MIN, float(part)))

    def redimensionner_panneau(self, nom, part):
        self._poser(nom, part=min(1.0, max(PART_MIN, float(part))))

    # ----- Conservation -----

    def vers_json(self):
        return {
            "version": VERSION,
            "panneaux": {nom: {"etat": e.etat, "zone": e.zone, "rang": e.rang, "part": e.part,
                               "geometrie": e.geometrie, "premier_plan": e.premier_plan,
                               "retour": e.retour}
                         for nom, e in sorted(self.panneaux.items())},
            "zones": dict(self.zones),
        }

    @classmethod
    def de_json(cls, contenu, definitions):
        """Disposition relue, et un avertissement si le contenu est invalide (la disposition par
        défaut est alors rendue). Panneau inconnu ignoré ; panneau nouveau à sa place par défaut."""
        defaut = cls.par_defaut(definitions)
        try:
            disposition = cls._lire(contenu, definitions, defaut)
        except (ValueError, TypeError, KeyError, AttributeError) as erreur:
            return defaut, f"disposition des panneaux invalide ({erreur}) : disposition par défaut"
        return disposition, None

    @classmethod
    def _lire(cls, contenu, definitions, defaut):
        if not isinstance(contenu, dict) or contenu.get("version") != VERSION:
            raise ValueError("version inconnue")
        zones = dict(PARTS_PAR_DEFAUT)
        for zone, part in contenu["zones"].items():
            if zone not in ZONES or isinstance(part, bool) or not isinstance(part, (int, float)) \
                    or not PART_MIN <= part <= PART_MAX:
                raise ValueError(f"zone {zone}")
            zones[zone] = float(part)
        panneaux = dict(defaut.panneaux)
        for nom, e in contenu["panneaux"].items():
            if nom not in definitions:
                continue
            etat = EtatPanneau(e["etat"], e["zone"], e["rang"], e["part"], e["geometrie"],
                               e["premier_plan"], e["retour"])
            if (etat.etat not in ETATS or etat.zone not in ZONES or etat.retour not in RETOURS
                    or isinstance(etat.rang, bool) or not isinstance(etat.rang, int)
                    or etat.rang < 0 or isinstance(etat.part, bool)
                    or not isinstance(etat.part, (int, float)) or not 0 < etat.part <= 1
                    or not isinstance(etat.premier_plan, bool)):
                raise ValueError(f"panneau {nom}")
            _valider_geometrie(etat.geometrie)
            panneaux[nom] = etat
        disposition = cls(panneaux, zones)
        if sum(1 for e in panneaux.values() if e.etat == "page") > 1:
            raise ValueError("plusieurs panneaux en pleine page")
        for zone in ZONES:
            disposition._renumeroter(zone)
        return disposition


def chemin_de_configuration(env=os.environ):
    """`$XDG_CONFIG_HOME/memoire/plan-de-travail.json`, sinon `~/.config/memoire/…`."""
    base = env.get("XDG_CONFIG_HOME") or os.path.join(env.get("HOME", ""), ".config")
    return Path(base) / "memoire" / "plan-de-travail.json"


def lire(chemin, definitions, ecran=None):
    """Disposition conservée et avertissement éventuel ; fichier absent : disposition par défaut,
    sans avertissement. Avec `ecran` (largeur, hauteur), les panneaux flottants y sont ramenés."""
    chemin = Path(chemin)
    if not chemin.exists():
        return Disposition.par_defaut(definitions), None
    try:
        contenu = json.loads(chemin.read_text(encoding="utf-8"))
    except (OSError, ValueError) as erreur:
        return (Disposition.par_defaut(definitions),
                f"disposition des panneaux illisible ({erreur}) : disposition par défaut")
    disposition, avertissement = Disposition.de_json(contenu, definitions)
    if ecran is not None:
        for nom, e in disposition.panneaux.items():
            if e.geometrie is not None:
                disposition.panneaux[nom] = replace(e, geometrie=ramener_dans_l_ecran(
                    e.geometrie, *ecran))
    return disposition, avertissement


def ecrire(chemin, disposition):
    """Écriture atomique : fichier temporaire dans le même dossier, puis renommage."""
    chemin = Path(chemin)
    chemin.parent.mkdir(parents=True, exist_ok=True)
    fd, temporaire = tempfile.mkstemp(dir=chemin.parent, prefix=".plan-de-travail-", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fichier:
            json.dump(disposition.vers_json(), fichier, ensure_ascii=False, indent=1)
            fichier.write("\n")
        os.replace(temporaire, chemin)
    except BaseException:
        Path(temporaire).unlink(missing_ok=True)
        raise
