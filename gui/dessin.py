"""Dessin des formes de l'interface, en Python pur (APPARENCE.md, section 5 ; D31 ; INV-LOOK-01).

Aucune importation de tkinter, aucune dépendance : chaque forme est une image PNG (RGBA, 8 bits)
calculée ici, que Tkinter compose ensuite sur le canevas ou dans le thème ttk.

Une forme est un rectangle arrondi décrit par sa distance signée : bord adouci d'un pixel, liseré
intérieur, halo intérieur, ombre décalée et adoucie (cachée sous la forme, même translucide, comme
`box-shadow`). Le calcul rapide ne calcule qu'une fois les rangées identiques et, dans une rangée,
le milieu identique ; il rend les mêmes octets que le calcul pixel par pixel (essai).
"""
from dataclasses import dataclass
import math
import random
import struct
import zlib

# Arrêts des dégradés (position de 0 à 1, couleur) : ceux de la maquette validée.
DEGRADE_DE_FOND = (
    (0.00, (0x4B, 0x4A, 0x9E, 1.0)),
    (0.17, (0x8D, 0x87, 0xB4, 1.0)),
    (0.37, (0xE8, 0xDC, 0xCF, 1.0)),
    (0.54, (0xEB, 0xC3, 0xDC, 1.0)),
    (0.68, (0xDE, 0x8F, 0xDA, 1.0)),
    (0.84, (0xB0, 0x7F, 0xE3, 1.0)),
    (1.00, (0x9A, 0x6B, 0xE0, 1.0)),
)
DEGRADE_SIGNATURE = (
    (0.00, (0x34, 0x2F, 0xCD, 1.0)),
    (0.26, (0x6E, 0x3F, 0xBF, 1.0)),
    (0.52, (0xFF, 0x74, 0x77, 1.0)),
    (0.78, (0x6E, 0x3F, 0xBF, 1.0)),
    (1.00, (0x34, 0x2F, 0xCD, 1.0)),
)


def rgba(hexa, alpha=1.0):
    """Couleur « #RRGGBB » et son opacité (0 à 1)."""
    hexa = hexa.lstrip("#")
    return (int(hexa[0:2], 16), int(hexa[2:4], 16), int(hexa[4:6], 16), float(alpha))


@dataclass(frozen=True)
class Image:
    """Une image PNG ; (x, y) : coin haut gauche de la forme dans l'image (marge de l'ombre)."""

    largeur: int
    hauteur: int
    png: bytes
    x: int = 0
    y: int = 0


# ----- PNG -----

def _morceau(genre, contenu):
    return (struct.pack(">I", len(contenu)) + genre + contenu
            + struct.pack(">I", zlib.crc32(genre + contenu) & 0xFFFFFFFF))


def png(largeur, hauteur, lignes):
    """PNG RGBA 8 bits à partir des lignes de pixels (4 octets par pixel)."""
    lignes = list(lignes)
    if len(lignes) != hauteur or any(len(ligne) != 4 * largeur for ligne in lignes):
        raise ValueError("dimensions des lignes incohérentes")
    brut = b"".join(b"\x00" + bytes(ligne) for ligne in lignes)
    entete = struct.pack(">IIBBBBB", largeur, hauteur, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + _morceau(b"IHDR", entete)
            + _morceau(b"IDAT", zlib.compress(brut, 6)) + _morceau(b"IEND", b""))


def lire_png(donnees):
    """(largeur, hauteur, lignes) d'un PNG écrit par `png` (RGBA 8 bits, sans filtre)."""
    rang, idat, largeur, hauteur = 8, b"", 0, 0
    while rang < len(donnees):
        (longueur,) = struct.unpack(">I", donnees[rang:rang + 4])
        genre = donnees[rang + 4:rang + 8]
        contenu = donnees[rang + 8:rang + 8 + longueur]
        if genre == b"IHDR":
            largeur, hauteur = struct.unpack(">II", contenu[:8])
        elif genre == b"IDAT":
            idat += contenu
        rang += 12 + longueur
    brut = zlib.decompress(idat)
    pas = 1 + 4 * largeur
    return largeur, hauteur, [brut[y * pas + 1:(y + 1) * pas] for y in range(hauteur)]


# ----- Outils -----

def _borne(v):
    return 0.0 if v < 0.0 else 1.0 if v > 1.0 else v


def _lisse(bas, haut, v):
    if v <= bas:
        return 0.0
    if v >= haut:
        return 1.0
    t = (v - bas) / (haut - bas)
    return t * t * (3.0 - 2.0 * t)


def couleur_du_degrade(arrets, t):
    """Couleur (r, g, b, a) à la position t (0 à 1) d'un dégradé."""
    if t <= arrets[0][0]:
        return arrets[0][1]
    for (p0, c0), (p1, c1) in zip(arrets, arrets[1:]):
        if t <= p1:
            u = (t - p0) / (p1 - p0) if p1 > p0 else 0.0
            return tuple(c0[i] + (c1[i] - c0[i]) * u for i in range(4))
    return arrets[-1][1]


def _sur(dessus, dessous):
    """Composition « par-dessus » en alpha droit : (r, g, b, a) flottants."""
    r1, g1, b1, a1 = dessus
    if a1 <= 0.0:
        return dessous
    r0, g0, b0, a0 = dessous
    a = a1 + a0 * (1.0 - a1)
    if a <= 0.0:
        return (0.0, 0.0, 0.0, 0.0)
    k = a0 * (1.0 - a1)
    return ((r1 * a1 + r0 * k) / a, (g1 * a1 + g0 * k) / a, (b1 * a1 + b0 * k) / a, a)


def _octets(c):
    r, g, b, a = c
    if a <= 0.0:
        return b"\x00\x00\x00\x00"
    return bytes((min(255, max(0, round(r))), min(255, max(0, round(g))),
                  min(255, max(0, round(b))), min(255, max(0, round(a * 255)))))


def _distance(px, py, demi_l, demi_h, rayon):
    """Distance signée au rectangle arrondi centré en 0 (négative à l'intérieur)."""
    qx = abs(px) - (demi_l - rayon)
    qy = abs(py) - (demi_h - rayon)
    return math.hypot(max(qx, 0.0), max(qy, 0.0)) + min(max(qx, qy), 0.0) - rayon


# ----- Formes -----

def forme(largeur, hauteur, rayon, fond, bord=None, epaisseur=1.0, ombre=None, halo=None,
          degrade=None, grain=None, rapide=True):
    """Rectangle arrondi de `largeur` × `hauteur` (rayon borné à la demi-hauteur).

    - `fond` : (r, g, b, a) ; `degrade` : arrêts d'un dégradé horizontal qui remplace sa teinte ;
    - `bord`, `epaisseur` : liseré intérieur ;
    - `halo` : (taille, couleur), lueur intérieure le long du bord ;
    - `ombre` : (dx, dy, flou, couleur), décalée, adoucie, cachée sous la forme ;
    - `grain` : (force, graine), bruit déterministe sur la teinte (petites images seulement).
    """
    rayon = max(0.0, min(float(rayon), largeur / 2, hauteur / 2))
    dx = dy = flou = 0
    couleur_ombre = None
    if ombre:
        dx, dy, flou, couleur_ombre = ombre
    etendue = math.ceil(flou) + 1 if ombre else 0
    gauche = etendue + max(0, -dx) if ombre else 0
    haut = etendue + max(0, -dy) if ombre else 0
    droite = etendue + max(0, dx) if ombre else 0
    bas = etendue + max(0, dy) if ombre else 0
    larg_img, haut_img = largeur + gauche + droite, hauteur + haut + bas
    demi_l, demi_h = largeur / 2, hauteur / 2
    cx, cy = gauche + demi_l, haut + demi_h
    taille_halo, couleur_halo = halo if halo else (0, None)
    hasard = random.Random(grain[1]) if grain else None
    teintes = []
    for x in range(larg_img):
        if degrade:
            t = _borne((x + 0.5 - gauche) / largeur)
            r, g, b, _ = couleur_du_degrade(degrade, t)
            teintes.append((r, g, b, fond[3]))
        else:
            teintes.append(fond)

    def facteurs(x, y):
        px, py = x + 0.5 - cx, y + 0.5 - cy
        d = _distance(px, py, demi_l, demi_h, rayon)
        couvre = _borne(0.5 - d)
        liseré = (couvre - _borne(0.5 - d - epaisseur)) if bord else 0.0
        lueur = (1.0 - _lisse(0.0, taille_halo, -d)) * couvre if halo else 0.0
        portee = 0.0
        if ombre and couvre < 1.0:
            ds = _distance(px - dx, py - dy, demi_l, demi_h, rayon)
            portee = (1.0 - _lisse(-flou, flou, ds)) * (1.0 - couvre) if flou else \
                _borne(0.5 - ds) * (1.0 - couvre)
        return (couvre, liseré, lueur, portee)

    def composer(f, teinte):
        couvre, liseré, lueur, portee = f
        c = (0.0, 0.0, 0.0, 0.0)
        if portee > 0.0:
            c = _sur((couleur_ombre[0], couleur_ombre[1], couleur_ombre[2],
                      couleur_ombre[3] * portee), c)
        if couvre > 0.0:
            r, g, b, a = teinte
            if hasard is not None:
                n = (hasard.random() * 2.0 - 1.0) * grain[0] * 255.0
                r, g, b = r + n, g + n, b + n
            c = _sur((r, g, b, a * couvre), c)
        if lueur > 0.0:
            c = _sur((couleur_halo[0], couleur_halo[1], couleur_halo[2],
                      couleur_halo[3] * lueur), c)
        if liseré > 0.0:
            c = _sur((bord[0], bord[1], bord[2], bord[3] * liseré), c)
        return _octets(c)

    if not rapide or grain:
        lignes = [b"".join(composer(facteurs(x, y), teintes[x]) for x in range(larg_img))
                  for y in range(haut_img)]
        return Image(larg_img, haut_img, png(larg_img, haut_img, lignes), gauche, haut)

    # Zones où rien ne dépend de x (resp. y) : loin des coins, du liseré, du halo et de l'ombre.
    marge = math.ceil(rayon + max(taille_halo, epaisseur if bord else 0, flou)) + 2
    x0 = gauche + max(0, dx) + marge
    x1 = gauche + largeur + min(0, dx) - marge
    y0 = haut + max(0, dy) + marge
    y1 = haut + hauteur + min(0, dy) - marge

    def ligne(y):
        if x0 >= x1:
            return b"".join(composer(facteurs(x, y), teintes[x]) for x in range(larg_img))
        f_milieu = facteurs(x0, y)
        morceaux = [composer(facteurs(x, y), teintes[x]) for x in range(x0)]
        if degrade:
            morceaux += [composer(f_milieu, teintes[x]) for x in range(x0, x1)]
        else:
            morceaux.append(composer(f_milieu, teintes[x0]) * (x1 - x0))
        morceaux += [composer(facteurs(x, y), teintes[x]) for x in range(x1, larg_img)]
        return b"".join(morceaux)

    lignes = []
    milieu = None
    for y in range(haut_img):
        if y0 <= y < y1:
            if milieu is None:
                milieu = ligne(y)
            lignes.append(milieu)
        else:
            lignes.append(ligne(y))
    return Image(larg_img, haut_img, png(larg_img, haut_img, lignes), gauche, haut)


def degrade(largeur, hauteur, arrets, angle):
    """Dégradé linéaire à la manière de CSS (`angle` en degrés, 90 : de gauche à droite) ; les
    coins extrêmes tombent sur le premier et le dernier arrêt."""
    a = math.radians(angle)
    ux, uy = math.sin(a), -math.cos(a)
    longueur = abs(largeur * ux) + abs(hauteur * uy)
    # Couleurs précalculées sur 1024 positions : le dégradé ne se calcule qu'une fois par teinte.
    pas = 1023
    table = [_octets(couleur_du_degrade(arrets, k / pas)) for k in range(pas + 1)]
    lignes = []
    for y in range(hauteur):
        py = y + 0.5 - hauteur / 2
        depart = (0.5 - largeur / 2) * ux + py * uy
        lignes.append(b"".join(
            table[min(pas, max(0, round(((depart + x * ux) / longueur + 0.5) * pas)))]
            for x in range(largeur)))
    return Image(largeur, hauteur, png(largeur, hauteur, lignes))


# ----- Pictogrammes -----

def _cercle(cx, cy, r, debut=0.0, fin=360.0, pas=16):
    n = max(2, round(pas * abs(fin - debut) / 360.0))
    return [(cx + r * math.cos(math.radians(debut + (fin - debut) * k / n)),
             cy + r * math.sin(math.radians(debut + (fin - debut) * k / n))) for k in range(n + 1)]


def _gelule(cx, cy, longueur, r, angle):
    """Contour d'une gélule (deux demi-cercles reliés), tournée de `angle` degrés."""
    a = math.radians(angle)
    ux, uy = math.cos(a), math.sin(a)
    demi = longueur / 2 - r
    ax, ay, bx, by = cx - ux * demi, cy - uy * demi, cx + ux * demi, cy + uy * demi
    points = _cercle(bx, by, r, angle - 90, angle + 90, 16) + _cercle(ax, ay, r, angle + 90, angle + 270, 16)
    return points + [points[0]]


# Traits dans une grille de 24 unités, comme les pictogrammes de la maquette.
ICONES = {
    "ecrire": [[(4, 20), (8, 20), (19, 9), (15, 5), (4, 16), (4, 20)], [(13.5, 6.5), (17.5, 10.5)]],
    "relier": [_gelule(9, 15, 11, 3.2, -45), _gelule(15, 9, 11, 3.2, -45)],
    "juger": [[(12, 4), (12, 20)], [(7.5, 20), (16.5, 20)], [(4.5, 7), (19.5, 7)],
              [(5, 7), (2.5, 13)], [(5, 7), (7.5, 13)], _cercle(5, 13, 2.5, 0, 180, 10),
              [(19, 7), (16.5, 13)], [(19, 7), (21.5, 13)], _cercle(19, 13, 2.5, 0, 180, 10)],
    "lire": [[(12, 6.5), (12, 20)], [(12, 6.5), (9, 5), (3.5, 5), (3.5, 18), (9, 18), (12, 20)],
             [(12, 6.5), (15, 5), (20.5, 5), (20.5, 18), (15, 18), (12, 20)]],
    "code": [[(8, 7), (3, 12), (8, 17)], [(16, 7), (21, 12), (16, 17)], [(14, 5), (10, 19)]],
    "pistes": [[(6, 3.5), (6, 15)], _cercle(6, 18, 2.5), _cercle(18, 6, 2.5),
               [(18, 8.5), (17.6, 11), (15.4, 13.2), (11, 14.6), (6, 15.5)]],
    "reprise": [[(5, 21), (5, 3.5)], [(5, 4), (18, 4), (15, 8), (18, 12), (5, 12)]],
    "projet": [[(3, 7), (3, 18), (4, 19), (20, 19), (21, 18), (21, 9), (20, 8), (12, 8),
                (10, 6), (4, 6), (3, 7)]],
    "expert": [[(4, 7), (9, 12), (4, 17)], [(12, 18), (20, 18)]],
    "chercher": [_cercle(10.5, 10.5, 6.5, pas=24), [(15.5, 15.5), (20, 20)]],
    "terminaux": [[(3, 5), (21, 5), (21, 19), (3, 19), (3, 5)], [(7, 9.5), (10, 12), (7, 14.5)],
                  [(12, 15), (17, 15)]],
    "panneaux": [[(3, 4), (21, 4), (21, 20), (3, 20), (3, 4)], [(9.5, 4), (9.5, 20)],
                 [(9.5, 12), (21, 12)]],
    "reglages": [[(4, 7), (13.5, 7)], [(18.5, 7), (20, 7)], _cercle(16, 7, 2.5),
                 [(4, 17), (5.5, 17)], [(10.5, 17), (20, 17)], _cercle(8, 17, 2.5)],
    "chevron": [[(6, 9), (12, 15), (18, 9)]],
    "coche": [[(5, 12.5), (10, 17.5), (19.5, 7)]],
    "plus": [[(12, 5), (12, 19)], [(5, 12), (19, 12)]],
    "fermer": [[(6.5, 6.5), (17.5, 17.5)], [(17.5, 6.5), (6.5, 17.5)]],
    # Tuiles de la console (8.5) : ranger en arrière-plan, agrandir, parts égales.
    "ranger": [[(6, 18), (18, 18)]],
    "agrandir": [[(14.5, 4), (20, 4), (20, 9.5)], [(20, 4), (13.5, 10.5)],
                 [(9.5, 20), (4, 20), (4, 14.5)], [(4, 20), (10.5, 13.5)]],
    "egaliser": [[(14, 4.5), (14, 10), (19.5, 10)], [(14, 10), (20, 4)],
                 [(10, 19.5), (10, 14), (4.5, 14)], [(10, 14), (4, 20)]],
    "voisinage": [_cercle(12, 12, 2.4), _cercle(5, 5, 2.2), _cercle(19, 5, 2.2),
                  _cercle(12, 20, 2.2), [(6.7, 6.7), (10.3, 10.3)], [(17.3, 6.7), (13.7, 10.3)],
                  [(12, 14.4), (12, 17.8)]],
}


def _distance_au_segment(px, py, ax, ay, bx, by):
    vx, vy = bx - ax, by - ay
    longueur = vx * vx + vy * vy
    t = 0.0 if longueur == 0 else _borne(((px - ax) * vx + (py - ay) * vy) / longueur)
    return math.hypot(px - ax - t * vx, py - ay - t * vy)


def icone(nom, taille, couleur, epaisseur=2.2):
    """Pictogramme `nom` en `taille` pixels, traits de `epaisseur` unités (grille de 24)."""
    traits = ICONES[nom]
    echelle = taille / 24.0
    segments = [(a[0] * echelle, a[1] * echelle, b[0] * echelle, b[1] * echelle)
                for trait in traits for a, b in zip(trait, trait[1:])]
    demi = epaisseur * echelle / 2
    lignes = []
    for y in range(taille):
        ligne = bytearray()
        for x in range(taille):
            px, py = x + 0.5, y + 0.5
            d = min(_distance_au_segment(px, py, *s) for s in segments)
            couvre = _borne(demi + 0.5 - d)
            ligne += _octets((couleur[0], couleur[1], couleur[2], couleur[3] * couvre))
        lignes.append(bytes(ligne))
    return Image(taille, taille, png(taille, taille, lignes))
