"""Modèle de la mosaïque de la console (CONSOLE.md, sections 2, 3 et 5 ; D32 ; INV-TERM-04).

Aucune importation de tkinter. La `Mosaique` est la seule source de vérité sur la place des
terminaux dans la console : sur la page (quatre au plus, dans l'ordre) ou en arrière-plan, lequel
est actif, et où se trouve le point milieu. La réalisation (gui/console.py) applique ses
changements. Les positions sont des fractions de la page ; `en_pixels` les convertit.

Dispositions (terminaux montrés, dans l'ordre) : un seul, toute la page ; deux, côte à côte ;
trois, le premier à gauche sur toute la hauteur et les deux autres à droite ; quatre, le quatrième
sous le premier. Un terminal ajouté ne déplace donc pas les autres.
"""
from dataclasses import dataclass, field

CAPACITE = 4
PART_MIN, PART_MAX = 0.25, 0.75
PART_AGRANDIE = 2 / 3
# En dessous de deux fois ces tailles, une rangée ou une colonne de deux tuiles serait illisible.
LARGEUR_MIN, HAUTEUR_MIN = 330, 170
# Papier laissé entre deux tuiles, en pixels.
ECART = 12
# Demi-côté de la zone où l'on prend le point milieu, autour du croisement.
PRISE_DU_POINT = 22


def capacite(largeur, hauteur):
    """Nombre de terminaux que la place permet de montrer : 1, 2 (côte à côte) ou 4."""
    if largeur < 2 * LARGEUR_MIN:
        return 1
    if hauteur < 2 * HAUTEUR_MIN:
        return 2
    return CAPACITE


def _borner(part):
    return min(PART_MAX, max(PART_MIN, float(part)))


def en_pixels(rectangles, largeur, hauteur, ecart=ECART):
    """[(nom, (x, y, largeur, hauteur))] en pixels entiers : bords de la page exacts, `ecart`
    pixels de papier entre deux tuiles, centrés sur les séparations."""
    def bord(fraction, total, debut):
        if fraction <= 0:
            return 0
        if fraction >= 1:
            return total
        return round(fraction * total) - ecart // 2 + (ecart if debut else 0)

    resultat = []
    for nom, (x0, y0, x1, y1) in rectangles:
        gauche, haut = bord(x0, largeur, True), bord(y0, hauteur, True)
        resultat.append((nom, (gauche, haut, bord(x1, largeur, False) - gauche,
                               bord(y1, hauteur, False) - haut)))
    return resultat


@dataclass
class Mosaique:
    """Terminaux de la page et de l'arrière-plan, terminal actif, point milieu (x, y)."""

    page: list = field(default_factory=list)
    reserve: list = field(default_factory=list)
    actif: str = None
    x: float = 0.5
    y: float = 0.5
    agrandi: str = None

    @classmethod
    def par_defaut(cls, noms):
        """Les premiers terminaux sur la page, les autres en arrière-plan, parts égales."""
        noms = list(noms)
        return cls(noms[:CAPACITE], noms[CAPACITE:], noms[0] if noms else None)

    # ----- Lecture -----

    def montres(self, capacite=CAPACITE):
        """Terminaux de la page que la place permet de montrer ; l'actif en fait toujours partie."""
        if len(self.page) <= capacite:
            return list(self.page)
        debut = min(self.page.index(self.actif), len(self.page) - capacite)
        return self.page[debut:debut + capacite]

    def en_attente(self, capacite=CAPACITE):
        """Terminaux qu'on ne voit pas : ceux de la page qui ne tiennent pas, puis l'arrière-plan."""
        montres = self.montres(capacite)
        return [nom for nom in self.page if nom not in montres] + list(self.reserve)

    def rectangles(self, capacite=CAPACITE):
        """[(nom, (x0, y0, x1, y1))] des terminaux montrés, en fractions de la page."""
        montres = self.montres(capacite)
        x, y = self.x, self.y
        formes = {
            0: [],
            1: [(0, 0, 1, 1)],
            2: [(0, 0, x, 1), (x, 0, 1, 1)],
            3: [(0, 0, x, 1), (x, 0, 1, y), (x, y, 1, 1)],
            4: [(0, 0, x, y), (x, 0, 1, y), (x, y, 1, 1), (0, y, x, 1)],
        }[len(montres)]
        return list(zip(montres, formes))

    def point(self, capacite=CAPACITE):
        """(x, y) du point milieu ; None en dessous de trois terminaux montrés."""
        return (self.x, self.y) if len(self.montres(capacite)) >= 3 else None

    def poignees(self, capacite=CAPACITE):
        """[(axe, x, y)] : milieu de chaque séparation ; `axe` est celui où elle se tire."""
        nombre = len(self.montres(capacite))
        x, y = self.x, self.y
        if nombre == 2:
            return [("x", x, 0.5)]
        if nombre < 3:
            return []
        poignees = [("x", x, y / 2), ("x", x, (1 + y) / 2), ("y", (1 + x) / 2, y)]
        if nombre == 4:
            poignees.append(("y", x / 2, y))
        return poignees

    def prise(self, px, py, largeur, hauteur, capacite=CAPACITE, ecart=ECART):
        """Ce qu'on prend au pixel (px, py) d'une page de `largeur` × `hauteur` : « xy » (le point
        milieu), « x » ou « y » (une séparation, tirée sur cet axe), ou None."""
        nombre = len(self.montres(capacite))
        if nombre < 2:
            return None
        xs, ys = round(self.x * largeur), round(self.y * hauteur)
        marge = ecart / 2 + 2
        if nombre >= 3 and abs(px - xs) <= PRISE_DU_POINT and abs(py - ys) <= PRISE_DU_POINT:
            return "xy"
        if abs(px - xs) <= marge:
            return "x"
        if nombre >= 3 and abs(py - ys) <= marge and (nombre == 4 or px > xs):
            return "y"
        return None

    # ----- Opérations -----

    def _connu(self, nom):
        if nom not in self.page and nom not in self.reserve:
            raise ValueError(f"terminal inconnu de la mosaïque : {nom}")

    def placer(self, nom):
        """Met sur la page un terminal nouveau ou en arrière-plan, et le rend actif. Page pleine :
        il prend la place de l'actif, qui passe en tête de l'arrière-plan."""
        if nom in self.page:
            self.actif = nom
            return
        if nom in self.reserve:
            self.reserve.remove(nom)
        if len(self.page) < CAPACITE:
            self.page.append(nom)
        else:
            rang = self.page.index(self.actif)
            self.reserve.insert(0, self.page[rang])
            self.page[rang] = nom
        self.actif = nom
        self.agrandi = None

    def _sortir_de_la_page(self, nom):
        rang = self.page.index(nom)
        self.page.remove(nom)
        if self.actif == nom:
            self.actif = self.page[max(0, rang - 1)] if self.page else None
        self.agrandi = None

    def ranger(self, nom):
        """Passe un terminal de la page en arrière-plan ; le voisin d'avant devient actif."""
        if nom not in self.page:
            raise ValueError(f"terminal absent de la page : {nom}")
        self._sortir_de_la_page(nom)
        self.reserve.append(nom)

    def retirer(self, nom):
        """Oublie un terminal fermé."""
        self._connu(nom)
        if nom in self.page:
            self._sortir_de_la_page(nom)
        else:
            self.reserve.remove(nom)

    def activer(self, nom):
        if nom not in self.page:
            raise ValueError(f"terminal absent de la page : {nom}")
        self.actif = nom

    def deplacer(self, x=None, y=None):
        """Déplace le point milieu (un axe ou les deux) ; aucune part ne passe sous PART_MIN."""
        if x is not None:
            self.x = _borner(x)
        if y is not None:
            self.y = _borner(y)
        self.agrandi = None

    def egaliser(self):
        self.x = self.y = 0.5
        self.agrandi = None

    def agrandir(self, nom, capacite=CAPACITE):
        """Donne à un terminal montré les deux tiers de la page dans chaque direction qu'il
        partage ; s'il est déjà agrandi, rend des parts égales."""
        montres = self.montres(capacite)
        if nom not in montres or len(montres) < 2:
            return
        if self.agrandi == nom:
            self.egaliser()
            return
        rang = montres.index(nom)
        self.x = PART_AGRANDIE if rang in (0, 3) else 1 - PART_AGRANDIE
        if len(montres) == 4 or (len(montres) == 3 and rang > 0):
            self.y = PART_AGRANDIE if rang in (0, 1) else 1 - PART_AGRANDIE
        self.agrandi = nom
        self.actif = nom

    # ----- Conservation -----

    def vers_json(self):
        return {"page": list(self.page), "reserve": list(self.reserve), "actif": self.actif,
                "x": self.x, "y": self.y, "agrandi": self.agrandi}

    @classmethod
    def de_json(cls, contenu, noms):
        """Mosaïque relue et accordée aux terminaux `noms` du registre, et un avertissement si le
        contenu est invalide (mosaïque par défaut). Contenu absent : par défaut, sans
        avertissement. Nom inconnu ignoré ; terminal oublié en arrière-plan."""
        noms = list(noms)
        if contenu is None:
            return cls.par_defaut(noms), None
        try:
            mosaique = cls._lire(contenu)
        except (ValueError, TypeError, KeyError) as erreur:
            return cls.par_defaut(noms), f"mosaïque de la console invalide ({erreur}) : par défaut"
        mosaique._accorder(noms)
        return mosaique, None

    @classmethod
    def _lire(cls, contenu):
        if not isinstance(contenu, dict):
            raise ValueError("forme inconnue")
        page, reserve = contenu["page"], contenu["reserve"]
        for liste in (page, reserve):
            if not isinstance(liste, list) or not all(isinstance(nom, str) for nom in liste):
                raise ValueError("liste de terminaux")
        if len(set(page + reserve)) != len(page + reserve) or len(page) > CAPACITE:
            raise ValueError("terminaux de la page")
        actif, agrandi = contenu["actif"], contenu.get("agrandi")
        if (actif in page) != bool(page) or (not page and actif is not None):
            raise ValueError("terminal actif")
        if agrandi is not None and agrandi not in page:
            raise ValueError("terminal agrandi")
        parts = []
        for cle in ("x", "y"):
            part = contenu[cle]
            if isinstance(part, bool) or not isinstance(part, (int, float)) \
                    or not PART_MIN <= part <= PART_MAX:
                raise ValueError(f"part {cle}")
            parts.append(float(part))
        return cls(list(page), list(reserve), actif, parts[0], parts[1], agrandi)

    def _accorder(self, noms):
        self.page = [nom for nom in self.page if nom in noms]
        self.reserve = [nom for nom in self.reserve if nom in noms]
        self.reserve += [nom for nom in noms if nom not in self.page and nom not in self.reserve]
        if not self.page and self.reserve:
            self.page.append(self.reserve.pop(0))
        if self.actif not in self.page:
            self.actif = self.page[0] if self.page else None
        if self.agrandi not in self.page:
            self.agrandi = None
