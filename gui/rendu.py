"""Rendu du visionneur (VISIONNEUR.md, section 4 ; D25 ; INV-VIEW-01, INV-SOURCE-02).

Python pur, sans Tkinter : le texte d'une source devient un texte affiché, des plages de style et,
pour chaque caractère affiché, la plage en octets UTF-8 du caractère de la source dont il provient.
Le rendu masque des marques (`**`, `#`, accents graves, `>`, clôtures), il n'invente aucun
caractère : seules les puces de liste et les séparateurs sont affichés par un caractère dédié,
rattaché à la marque qu'il remplace. Ce qui sort du sous-ensemble reconnu s'affiche tel quel.
"""
from dataclasses import dataclass
import re

PUCE = "•"
TRAIT = "—"

_TITRE = re.compile(r"(#{1,6}) ")
_SEPARATEUR = re.compile(r"(-{3,}|\*{3,}|_{3,})[ \t]*$")
_PUCE = re.compile(r"([ \t]*)([-*]) ")
_NUMERO = re.compile(r"[ \t]*\d+\. ")


@dataclass
class Rendu:
    """Texte affiché, origine de chaque caractère affiché, plages de style."""

    affiche: str
    # Pour chaque caractère affiché : (début, fin) en octets dans la source.
    spans: list
    # (début, fin, style), en caractères affichés, triés.
    styles: list

    def plage_source(self, debut, fin):
        """Plus petite plage de la source, en octets, qui contient les caractères affichés
        [debut, fin) ; aucune pour une sélection vide (règle 2)."""
        if debut >= fin:
            return None
        return (self.spans[debut][0], self.spans[fin - 1][1])

    def plages_affichees(self, debut, fin):
        """Plages affichées dont chaque caractère provient de la plage source [debut, fin), en
        octets (règle 3) ; les caractères affichés consécutifs sont réunis."""
        plages = []
        for i, (a, b) in enumerate(self.spans):
            if debut <= a and b <= fin:
                if plages and plages[-1][1] == i:
                    plages[-1] = (plages[-1][0], i + 1)
                else:
                    plages.append((i, i + 1))
        return plages


class _Rendeur:
    def __init__(self, texte):
        self.texte = texte
        self.octets = [0]
        for car in texte:
            self.octets.append(self.octets[-1] + len(car.encode("utf-8")))
        self.affiche = []
        self.spans = []
        self.marques = []

    def emettre(self, k, styles=frozenset(), car=None):
        """Affiche le caractère k de la source (ou `car` à sa place), avec ses styles."""
        self.affiche.append(self.texte[k] if car is None else car)
        self.spans.append((self.octets[k], self.octets[k + 1]))
        self.marques.append(styles)

    def lignes(self):
        debut = 0
        for ligne in self.texte.splitlines(keepends=True):
            yield debut, debut + len(ligne), ligne
            debut += len(ligne)

    def rendre(self):
        dans_bloc = False
        for debut, fin, ligne in self.lignes():
            contenu = ligne.rstrip("\r\n")
            fin_contenu = debut + len(contenu)
            if contenu.lstrip().startswith("```"):
                # Clôture d'un bloc de code : ligne masquée, retour à la ligne compris.
                dans_bloc = not dans_bloc
                continue
            if dans_bloc:
                for k in range(debut, fin):
                    self.emettre(k, frozenset({"bloc"}))
                continue
            self.bloc(debut, fin_contenu, contenu)
            for k in range(fin_contenu, fin):
                self.emettre(k)
        return self.resultat()

    def bloc(self, debut, fin, contenu):
        """Une ligne hors d'un bloc de code, sans son retour à la ligne."""
        titre = _TITRE.match(contenu)
        if titre:
            niveau = len(titre.group(1))
            self.en_ligne(debut + titre.end(), fin, frozenset({f"titre{niveau}"}))
            return
        if _SEPARATEUR.match(contenu):
            marques = len(contenu.rstrip(" \t"))
            for k in range(debut, debut + marques):
                self.emettre(k, frozenset({"separateur"}), TRAIT)
            for k in range(debut + marques, fin):
                self.emettre(k)
            return
        if contenu.startswith(">"):
            cache = 2 if contenu.startswith("> ") else 1
            self.en_ligne(debut + cache, fin, frozenset({"citation"}))
            return
        puce = _PUCE.match(contenu)
        if puce:
            retrait = len(puce.group(1))
            for k in range(debut, debut + retrait):
                self.emettre(k)
            self.emettre(debut + retrait, frozenset({"puce"}), PUCE)
            self.en_ligne(debut + retrait + 1, fin, frozenset())
            return
        numero = _NUMERO.match(contenu)
        if numero:
            for k in range(debut, debut + numero.end()):
                self.emettre(k)
            self.en_ligne(debut + numero.end(), fin, frozenset())
            return
        self.en_ligne(debut, fin, frozenset())

    def en_ligne(self, debut, fin, styles):
        """Éléments en ligne de [debut, fin) : code, gras, italique, lien ; sinon tel quel."""
        t = self.texte
        k = debut
        while k < fin:
            car = t[k]
            if car == "`":
                ferme = t.find("`", k + 1, fin)
                if ferme > k + 1:
                    for j in range(k + 1, ferme):
                        self.emettre(j, styles | {"code"})
                    k = ferme + 1
                    continue
            elif car == "*" and t.startswith("**", k):
                ferme = t.find("**", k + 2, fin)
                if ferme > k + 2:
                    self.en_ligne(k + 2, ferme, styles | {"gras"})
                    k = ferme + 2
                    continue
            if car in "*_":
                ferme = self.fin_italique(k, fin)
                if ferme is not None:
                    self.en_ligne(k + 1, ferme, styles | {"italique"})
                    k = ferme + 1
                    continue
            elif car == "[":
                milieu = t.find("](", k + 1, fin)
                ferme = t.find(")", milieu + 2, fin) if milieu > k + 1 else -1
                if ferme != -1:
                    self.en_ligne(k + 1, milieu, styles | {"lien"})
                    k = ferme + 1
                    continue
            self.emettre(k, styles)
            k += 1

    def fin_italique(self, k, fin):
        """Clôture d'une mise en italique ouverte en k (`*` ou `_` seul), ou aucune : marque
        précédée d'un début de mot, contenu non vide sans espace aux bords, clôture suivie d'une
        fin de mot. Ainsi `nom_de_variable` reste tel quel."""
        t = self.texte
        marque = t[k]
        if k > 0 and t[k - 1].isalnum():
            return None
        if k + 1 >= fin or t[k + 1].isspace() or t[k + 1] == marque:
            return None
        j = k + 1
        while j < fin:
            if t[j] == marque and not t[j - 1].isspace():
                if j + 1 >= len(t) or not t[j + 1].isalnum():
                    return j
            j += 1
        return None

    def resultat(self):
        styles = []
        noms = sorted({s for marques in self.marques for s in marques})
        for nom in noms:
            debut = None
            for i, marques in enumerate(self.marques + [frozenset()]):
                if nom in marques and debut is None:
                    debut = i
                elif nom not in marques and debut is not None:
                    styles.append((debut, i, nom))
                    debut = None
        styles.sort()
        return Rendu("".join(self.affiche), self.spans, styles)


def rendre(texte):
    """Rendu Markdown du sous-ensemble reconnu (VISIONNEUR.md, section 4)."""
    return _Rendeur(texte).rendre()


def brut(texte):
    """Texte exact, octet pour octet, sans aucun style."""
    rendeur = _Rendeur(texte)
    for k in range(len(texte)):
        rendeur.emettre(k)
    return Rendu(texte, rendeur.spans, [])
