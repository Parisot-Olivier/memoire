"""Thème Tkinter de l'interface (APPARENCE.md, sections 4 et 5 ; D31).

Polices choisies parmi celles présentes, thème ttk « memoire » (dérivé de « clam ») dont les
éléments sont des images de `gui/dessin.py` découpées en neuf, options des widgets classiques
(texte, listes, menus) et zones de texte arrondies. Aucune règle : seulement l'apparence.
"""
import base64
import math
from collections import namedtuple
import tkinter as tk
import tkinter.font as tkfont
from tkinter import ttk

import apparence
import dessin
from apparence import COULEURS as C
from dessin import rgba

NOM_DU_THEME = "memoire"
Forme = namedtuple("Forme", "image marges")
COURTE, PASTILLE = (3, 4, 3), (5, 7, 4)


def ombre(geometrie, opacite):
    """Ombre décalée de l'encre ; une opacité nulle garde la géométrie (images de même taille)."""
    return (*geometrie, rgba(C["encre"], opacite))


OMBRE_COURTE = ombre(COURTE, .14)
SANS_OMBRE = ombre(COURTE, 0)


def photo(maitre, image, fond=None):
    """PhotoImage Tk d'une image de `dessin` ; avec `fond`, aplatie sur cette couleur."""
    p = tk.PhotoImage(master=maitre, data=base64.b64encode(image.png).decode("ascii"),
                      format="png")
    return aplatir(maitre, p, fond) if fond else p


def aplatir(maitre, image, fond):
    """Copie opaque d'une PhotoImage posée sur la couleur `fond`.

    Optimisation (APPARENCE.md, section 5) : Tk mélange une image à transparence avec ce qui est
    dessous à chaque fois qu'il la dessine, en relisant l'écran ; sous XWayland, c'est environ
    huit fois plus lent qu'une image opaque. Chaque élément du thème est posé sur une surface
    connue : on le compose une fois, ici, dans Tk (en C)."""
    plate = tk.PhotoImage(master=maitre, width=image.width(), height=image.height())
    plate.put(fond, to=(0, 0, image.width(), image.height()))
    plate.tk.call(plate, "copy", image, "-compositingrule", "overlay")
    return plate


class Theme:
    """Polices, images et styles d'une racine Tk ; un seul par racine (`appliquer`)."""

    def __init__(self, racine):
        self.racine = racine
        self.images = []
        # Images aplaties (opaques) : celles des éléments du thème (essai d'opacité).
        self.images_d_elements = []
        self._icones = {}
        self._formes_des_elements = {}
        self.polices()
        self.style = ttk.Style(racine)
        self.ttk()
        self.classiques()
        racine.configure(background=C["papier"])

    # ----- Outils -----

    def _photo(self, image, fond=None):
        p = photo(self.racine, image, fond)
        self.images.append(p)
        if fond:
            self.images_d_elements.append(p)
        return p

    def _marge(self, image, droite, gauche=0, fond=None):
        """Copie d'une PhotoImage avec des colonnes à gauche et à droite : transparentes, ou de
        la couleur `fond` (copie alors opaque)."""
        copie = tk.PhotoImage(master=self.racine, width=image.width() + gauche + droite,
                              height=image.height())
        if fond:
            copie.put(fond, to=(0, 0, copie.width(), copie.height()))
        else:
            copie.blank()
        copie.tk.call(copie, "copy", image, "-to", gauche, 0, "-compositingrule", "overlay")
        self.images.append(copie)
        if fond:
            self.images_d_elements.append(copie)
        return copie

    def icone(self, nom, taille=18, couleur=C["encre"], fond=None):
        """Pictogramme en PhotoImage (gardé en mémoire, partagé) ; aplati sur `fond` s'il est
        donné."""
        cle = (nom, taille, couleur, fond)
        if cle not in self._icones:
            self._icones[cle] = self._photo(dessin.icone(nom, taille, rgba(couleur)), fond)
        return self._icones[cle]

    @staticmethod
    def melange(fond, dessus, opacite):
        """Couleur « #RRGGBB » de `dessus`, à cette opacité, posé sur `fond`."""
        f, d = rgba(fond), rgba(dessus)
        return "#%02X%02X%02X" % tuple(round(f[i] + (d[i] - f[i]) * opacite) for i in range(3))

    def point(self, couleur, fond, anneau=False, opacite=1.0):
        """Point d'état de 8 pixels (plein, ou en anneau), aplati sur `fond` ; gardé en mémoire."""
        cle = ("point", couleur, fond, anneau, opacite)
        if cle not in self._icones:
            if anneau:
                image = dessin.forme(8, 8, 4, rgba(couleur, 0), bord=rgba(couleur, opacite),
                                     epaisseur=1.5)
            else:
                image = dessin.forme(8, 8, 4, rgba(couleur, opacite))
            self._icones[cle] = self._photo(image, fond)
        return self._icones[cle]

    def _forme(self, largeur, hauteur, rayon, **options):
        """Forme et ses marges (gauche, haut, droite, bas) autour du rectangle arrondi."""
        image = dessin.forme(largeur, hauteur, rayon, **options)
        return Forme(image, (image.x, image.y, image.largeur - image.x - largeur,
                             image.hauteur - image.y - hauteur))

    def _element(self, nom, formes, rayon, marge_interieure, sticky="nsew", fond=C["papier"]):
        """Élément image : `formes` = [(états, Forme)], la première sans état, toutes de même
        géométrie ; découpe en neuf hors des coins arrondis et de l'ombre ; chaque image est
        aplatie sur `fond`, la surface où l'élément est posé."""
        gauche, haut, droite, bas = formes[0][1].marges
        coin = int(rayon) + 1
        image = formes[0][1].image
        if (image.largeur - gauche - droite - 2 * coin < 2
                or image.hauteur - haut - bas - 2 * coin < 2):
            raise ValueError(f"{nom} : pas de milieu à répéter entre les coins")
        mx, my = marge_interieure
        self._formes_des_elements[nom] = (formes, rayon, marge_interieure, sticky)
        specs = [self._photo(formes[0][1].image, fond)]
        for etats, forme_ in formes[1:]:
            if forme_.marges != formes[0][1].marges:
                raise ValueError(f"{nom} : états de géométries différentes")
            specs.append((*etats, self._photo(forme_.image, fond)))
        self.style.element_create(nom, "image", *specs,
                                  border=(gauche + coin, haut + coin, droite + coin, bas + coin),
                                  padding=(gauche + mx, haut + my, droite + mx, bas + my),
                                  sticky=sticky)

    def decliner(self, style, element, fond, prefixe):
        """Style `prefixe+style` : le même que `style`, son élément `element` aplati sur la
        surface `fond` (une pastille posée sur un encadré, une bascule, une barre)."""
        nouveau = f"{prefixe}{element}"
        if nouveau not in self._formes_des_elements:
            formes, rayon, marge, sticky = self._formes_des_elements[element]
            self._element(nouveau, formes, rayon, marge, sticky=sticky, fond=fond)

        def remplacer(disposition):
            return [(nouveau if nom == element else nom,
                     {**options, **({"children": remplacer(options["children"])}
                                    if "children" in options else {})})
                    for nom, options in disposition]

        s = self.style
        s.layout(prefixe + style, remplacer(s.layout(style)))
        options = {k: v for k, v in s.configure(style).items()} if s.configure(style) else {}
        s.configure(prefixe + style, **options, background=fond)
        etats = s.map(style)
        if etats:
            s.map(prefixe + style, **etats)
        return prefixe + style

    # ----- Polices -----

    def polices(self):
        presentes = set(tkfont.families(self.racine))
        sans = apparence.choisir_police(presentes, apparence.POLICES_SANS)
        mono = apparence.choisir_police(presentes, apparence.POLICES_MONO)
        self.sans, self.mono = sans, mono
        for nom in ("TkDefaultFont", "TkTextFont", "TkMenuFont", "TkHeadingFont", "TkCaptionFont",
                    "TkSmallCaptionFont", "TkIconFont", "TkTooltipFont"):
            police = tkfont.nametofont(nom, root=self.racine)
            if sans:
                police.configure(family=sans)
            police.configure(size=10 if nom != "TkSmallCaptionFont" else 9)
        if mono:
            tkfont.nametofont("TkFixedFont", root=self.racine).configure(family=mono, size=10)
        famille = sans or tkfont.nametofont("TkDefaultFont", root=self.racine).cget("family")
        definitions = {
            "MemoireTitre": dict(family=famille, size=17, weight="bold"),
            "MemoireSousTitre": dict(family=famille, size=11, weight="bold"),
            "MemoireGras": dict(family=famille, size=10, weight="bold"),
            "MemoireEtiquette": dict(family=famille, size=8, weight="bold"),
            "MemoirePetite": dict(family=famille, size=9),
            "MemoireMarque": dict(family=famille, size=14, weight="bold"),
            "MemoireAction": dict(family=famille, size=11, weight="bold"),
            "MemoireMono": dict(family=mono or "TkFixedFont", size=10),
            "MemoireMonoPetite": dict(family=mono or "TkFixedFont", size=9),
        }
        # Les polices nommées vivent tant que leur objet Python : on les garde.
        self.polices_nommees = {}
        for nom, options in definitions.items():
            if nom in tkfont.names(self.racine):
                police = tkfont.nametofont(nom, root=self.racine)
                police.configure(**options)
            else:
                police = tkfont.Font(root=self.racine, name=nom, **options)
            self.polices_nommees[nom] = police

    # ----- ttk -----

    def ttk(self):
        s = self.style
        if NOM_DU_THEME not in s.theme_names():
            s.theme_create(NOM_DU_THEME, parent="clam")
        s.theme_use(NOM_DU_THEME)
        papier, champ, encre = C["papier"], C["champ"], C["encre"]
        s.configure(".", background=papier, foreground=encre, font="TkDefaultFont",
                    bordercolor=C["separation"], lightcolor=papier, darkcolor=papier,
                    troughcolor=papier, focuscolor=C["indigo"], selectbackground=C["indigo_doux"],
                    selectforeground=encre, fieldbackground=champ, insertcolor=encre,
                    insertwidth=1, arrowcolor=encre)
        s.map(".", foreground=[("disabled", "#9A8E9C")])
        s.configure("TLabel", background=papier, foreground=encre)
        s.configure("Second.TLabel", foreground=C["texte_second"])
        s.configure("Titre.TLabel", font="MemoireTitre")
        s.configure("SousTitre.TLabel", font="MemoireSousTitre")
        s.configure("Etiquette.TLabel", font="MemoireEtiquette", foreground=C["etiquette"])
        s.configure("Marqueur.TLabel", foreground=C["ambre"], font="MemoireGras")
        s.configure("Mono.TLabel", font="MemoireMono")
        s.configure("Question.TLabel", font="MemoireGras")
        s.configure("TFrame", background=papier)
        s.configure("TSeparator", background=C["separation"])

        # Boutons : pastille au liseré d'encre ; survol blanc à ombre courte.
        def pastille(largeur, hauteur, rayon, fond, bord, ombre, **autres):
            return self._forme(largeur, hauteur, rayon, fond=fond, bord=bord, ombre=ombre, **autres)

        transparent = rgba(papier, 0)
        self._element("Pastille.bordure", [
            ((), pastille(48, 32, 14, transparent, rgba(encre, .18), SANS_OMBRE)),
            (("disabled",), pastille(48, 32, 14, transparent, rgba(encre, .08), SANS_OMBRE)),
            (("pressed",), pastille(48, 32, 14, rgba("#F6E0D4"), rgba(encre, .22), SANS_OMBRE)),
            (("active",), pastille(48, 32, 14, rgba("#FFFFFF"), rgba(encre, .12), OMBRE_COURTE)),
            (("focus",), pastille(48, 32, 14, transparent, rgba(C["indigo"], .9), SANS_OMBRE)),
        ], 14, (14, 6))
        disposition_bouton = [("Pastille.bordure", {"sticky": "nsew", "children": [
            ("Button.padding", {"sticky": "nsew", "children": [
                ("Button.label", {"sticky": "nsew"})]})]})]
        s.layout("TButton", disposition_bouton)
        s.configure("TButton", padding=(2, 1), anchor="center", foreground=encre)

        # Action principale : orange à grain, ombre décalée.
        orange = rgba(C["orange"])
        self._element("Action.bordure", [
            ((), pastille(150, 40, 17, orange, rgba("#FFFFFF", .35), ombre(PASTILLE, .22),
                          grain=(.05, 6328))),
            (("disabled",), pastille(150, 40, 17, rgba(C["orange"], .40), None,
                                     ombre(PASTILLE, 0), grain=(.05, 6328))),
            (("pressed",), pastille(150, 40, 17, rgba("#EE8F2C"), rgba("#FFFFFF", .30),
                                    ombre(PASTILLE, .10), grain=(.05, 6328))),
            (("active",), pastille(150, 40, 17, rgba("#FAAA52"), rgba("#FFFFFF", .45),
                                   ombre(PASTILLE, .26), grain=(.05, 6328))),
        ], 17, (20, 8))
        s.layout("Action.TButton", [("Action.bordure", {"sticky": "nsew", "children": [
            ("Button.padding", {"sticky": "nsew", "children": [
                ("Button.label", {"sticky": "nsew"})]})]})])
        s.configure("Action.TButton", font="MemoireAction", padding=(2, 1), foreground=encre)

        # Petit bouton (bandeaux des panneaux).
        self._element("Petit.bordure", [
            ((), pastille(36, 24, 10, transparent, rgba(encre, .16), SANS_OMBRE)),
            (("disabled",), pastille(36, 24, 10, transparent, rgba(encre, .06), SANS_OMBRE)),
            (("pressed",), pastille(36, 24, 10, rgba("#F6E0D4"), rgba(encre, .2), SANS_OMBRE)),
            (("active",), pastille(36, 24, 10, rgba("#FFFFFF"), rgba(encre, .12), OMBRE_COURTE)),
        ], 10, (9, 3))
        for nom, classe in (("Bandeau.TButton", "Button"), ("Bandeau.TMenubutton", "Menubutton")):
            enfants = [(f"{classe}.label", {"sticky": "nsew"})]
            if classe == "Menubutton":
                enfants = [("Pastille.chevron", {"side": "right", "sticky": ""}),
                           (f"{classe}.label", {"side": "left", "sticky": "nsew"})]
            s.layout(nom, [("Petit.bordure", {"sticky": "nsew", "children": [
                (f"{classe}.padding", {"sticky": "nsew", "children": enfants})]})])
            s.configure(nom, font="MemoirePetite", padding=(1, 0), foreground=encre)

        s.layout("BandeauMenu.TButton", [("Petit.bordure", {"sticky": "nsew", "children": [
            ("Button.padding", {"sticky": "nsew", "children": [
                ("Pastille.chevron", {"side": "right", "sticky": ""}),
                ("Button.label", {"side": "left", "sticky": "nsew"})]})]})])
        s.configure("BandeauMenu.TButton", font="MemoirePetite", padding=(1, 0), foreground=encre)

        # Liste déroulante de menu.
        self.style.element_create("Pastille.chevron", "image",
                                  self._marge(self.icone("chevron", 14), 0, gauche=6, fond=papier),
                                  ("active", self._marge(self.icone("chevron", 14), 0, gauche=6,
                                                         fond="#FFFFFF")), sticky="")
        self.style.element_create("Champ.chevron", "image",
                                  self._marge(self.icone("chevron", 14), 0, gauche=6, fond=champ),
                                  sticky="")
        s.layout("TMenubutton", [("Pastille.bordure", {"sticky": "nsew", "children": [
            ("Menubutton.padding", {"sticky": "nsew", "children": [
                ("Pastille.chevron", {"side": "right", "sticky": ""}),
                ("Menubutton.label", {"side": "left", "sticky": "nsew"})]})]})])
        s.configure("TMenubutton", padding=(2, 1), foreground=encre)

        # Sombre (terminal).
        nuit = C["terminal"]
        nuit_t = rgba(nuit, 0)
        self._element("Sombre.bordure", [
            ((), pastille(36, 26, 11, nuit_t, rgba("#FFFFFF", .28), SANS_OMBRE)),
            (("disabled",), pastille(36, 26, 11, nuit_t, rgba("#FFFFFF", .10), SANS_OMBRE)),
            (("pressed",), pastille(36, 26, 11, rgba("#FFFFFF", .22), rgba("#FFFFFF", .4), SANS_OMBRE)),
            (("active",), pastille(36, 26, 11, rgba("#FFFFFF", .14), rgba("#FFFFFF", .45), SANS_OMBRE)),
        ], 11, (10, 4), fond=nuit)
        s.layout("Sombre.TButton", [("Sombre.bordure", {"sticky": "nsew", "children": [
            ("Button.padding", {"sticky": "nsew", "children": [
                ("Button.label", {"sticky": "nsew"})]})]})])
        s.configure("Sombre.TButton", background=nuit, foreground=C["terminal_texte"],
                    font="MemoirePetite", padding=(1, 0))
        s.map("Sombre.TButton", foreground=[("disabled", "#7D7598")])
        # Onglets des terminaux (8.4) : l'actif en verre clair, les autres sans liseré.
        self._element("Onglet.actif", [
            ((), pastille(36, 26, 11, rgba("#FFFFFF", .16), rgba("#FFFFFF", .40), SANS_OMBRE)),
        ], 11, (10, 4), fond=nuit)
        self._element("Onglet.autre", [
            ((), pastille(36, 26, 11, nuit_t, None, SANS_OMBRE)),
            (("active",), pastille(36, 26, 11, rgba("#FFFFFF", .10), None, SANS_OMBRE)),
        ], 11, (10, 4), fond=nuit)
        for nom, element, couleur in (("OngletActif.TButton", "Onglet.actif", "#FFFFFF"),
                                      ("Onglet.TButton", "Onglet.autre", "#CFC7E6")):
            s.layout(nom, [(element, {"sticky": "nsew", "children": [
                ("Button.padding", {"sticky": "nsew", "children": [
                    ("Button.label", {"sticky": "nsew"})]})]})])
            s.configure(nom, background=nuit, foreground=couleur, font="MemoirePetite",
                        padding=(1, 0))
        # Tuiles de la console (8.5, CONSOLE.md, section 4) : boutons ronds de l'en-tête, nom,
        # état ; pastilles des terminaux en arrière-plan.
        self._element("Tuile.bouton", [
            ((), pastille(28, 28, 12, nuit_t, None, SANS_OMBRE)),
            (("pressed",), pastille(28, 28, 12, rgba("#FFFFFF", .22), None, SANS_OMBRE)),
            (("active",), pastille(28, 28, 12, rgba("#FFFFFF", .14), None, SANS_OMBRE)),
        ], 12, (6, 6), fond=nuit)
        s.layout("Tuile.TButton", [("Tuile.bouton", {"sticky": "nsew", "children": [
            ("Button.label", {"sticky": "nsew"})]})])
        s.configure("Tuile.TButton", background=nuit, padding=0)
        s.configure("NomDeTuile.TLabel", background=nuit, foreground=C["terminal_texte"],
                    font="MemoireMonoPetite")
        for nom, couleur in (("EtatDeTuile.TLabel", C["terminal_second"]),
                             ("EchecDeTuile.TLabel", C["terminal_echec"]),
                             ("MarqueDeTuile.TLabel", C["terminal_invite"])):
            s.configure(nom, background=nuit, foreground=couleur, font="MemoirePetite")
        s.layout("Attente.TButton", [("Petit.bordure", {"sticky": "nsew", "children": [
            ("Button.padding", {"sticky": "nsew", "children": [
                ("Button.label", {"sticky": "nsew"})]})]})])
        s.configure("Attente.TButton", font="MemoireMonoPetite", padding=(1, 0), foreground=encre)
        s.configure("Sombre.TFrame", background=nuit)
        s.configure("Sombre.TLabel", background=nuit, foreground=C["terminal_texte"],
                    font="MemoirePetite")
        s.configure("Invite.TLabel", background=nuit, foreground=C["terminal_invite"],
                    font="MemoireMono")

        # Champs : arrondis, fond clair ; en saisie, liseré indigo.
        # Milieu large (CONSOLE.md, section 11) : ttk remplit un élément en répétant le milieu
        # de son image ; un milieu de quelques pixels, c'est des milliers de copies par redessin.
        def champ_(bord, fond=rgba(champ), epaisseur=1.0, taille=(200, 34)):
            return self._forme(*taille, 12, fond=fond, bord=bord, epaisseur=epaisseur)

        self._element("Champ.champ", [
            ((), champ_(rgba(encre, .16))),
            (("disabled",), champ_(rgba(encre, .08), rgba(champ, .5))),
            (("focus",), champ_(rgba(C["indigo"]), epaisseur=1.6)),
        ], 12, (11, 6))
        s.layout("TEntry", [("Champ.champ", {"sticky": "nsew", "children": [
            ("Entry.padding", {"sticky": "nsew", "children": [
                ("Entry.textarea", {"sticky": "nsew"})]})]})])
        s.configure("TEntry", padding=(1, 1), foreground=encre, fieldbackground=champ)
        s.layout("TCombobox", [("Champ.champ", {"sticky": "nsew", "children": [
            ("Champ.chevron", {"side": "right", "sticky": ""}),
            ("Combobox.padding", {"sticky": "nsew", "children": [
                ("Combobox.textarea", {"sticky": "nsew"})]})]})])
        s.configure("TCombobox", padding=(1, 1), foreground=encre, fieldbackground=champ,
                    arrowsize=14)
        s.map("TCombobox", fieldbackground=[("readonly", champ)],
              selectbackground=[("readonly", champ)], selectforeground=[("readonly", encre)])

        nuit_champ = C["terminal_champ"]
        self._element("Sombre.champ", [
            ((), self._forme(200, 30, 11, fond=rgba(nuit_champ), bord=rgba("#FFFFFF", .16))),
            (("focus",), self._forme(200, 30, 11, fond=rgba(nuit_champ), bord=rgba("#FFB38A", .9),
                                     epaisseur=1.4)),
        ], 11, (10, 5), fond=nuit)
        s.layout("Sombre.TEntry", [("Sombre.champ", {"sticky": "nsew", "children": [
            ("Entry.padding", {"sticky": "nsew", "children": [
                ("Entry.textarea", {"sticky": "nsew"})]})]})])
        s.configure("Sombre.TEntry", background=nuit, foreground=C["terminal_texte"],
                    fieldbackground=nuit_champ, insertcolor=C["terminal_texte"],
                    selectbackground="#4A4380", selectforeground="#FFFFFF", padding=(1, 1))
        s.configure("Sombre.TSpinbox", background=nuit, foreground=C["terminal_texte"],
                    fieldbackground=nuit_champ, insertcolor=C["terminal_texte"],
                    bordercolor="#4A4568", lightcolor=nuit_champ, darkcolor=nuit_champ,
                    arrowcolor=C["terminal_texte"], arrowsize=10, padding=(6, 2))

        # Cadres arrondis (zones de texte, listes) et cartes (« Avant de continuer »…).
        cadre = (200, 160)
        self._element("Champ.cadre", [
            ((), champ_(rgba(encre, .14), taille=cadre)),
            (("focus",), champ_(rgba(C["indigo"]), epaisseur=1.6, taille=cadre)),
        ], 12, (6, 5))
        s.layout("Champ.TFrame", [("Champ.cadre", {"sticky": "nsew"})])
        s.configure("Champ.TFrame", background=papier)
        s.layout("ChampActif.TFrame", [("Champ.cadre", {"sticky": "nsew"})])
        self._element("Champ.actif", [((), champ_(rgba(C["indigo"]), epaisseur=1.6, taille=cadre))],
                      12, (6, 5))
        s.layout("ChampActif.TFrame", [("Champ.actif", {"sticky": "nsew"})])
        s.configure("ChampActif.TFrame", background=papier)
        self._element("Sombre.cadre", [((), self._forme(240, 240, 18, fond=rgba(nuit),
                                                         bord=rgba("#FFFFFF", .10)))], 18, (8, 8))
        s.layout("SombreCadre.TFrame", [("Sombre.cadre", {"sticky": "nsew"})])
        s.configure("SombreCadre.TFrame", background=papier)

        self._element("Carte.bordure", [((), self._forme(
            200, 200, 20, fond=rgba(papier), bord=rgba(encre, .10), ombre=ombre(COURTE, .07)))],
            20, (14, 12))
        s.layout("TLabelframe", [("Carte.bordure", {"sticky": "nsew"})])
        s.configure("TLabelframe", background=papier, labelmargins=(16, 14, 0, 0),
                    labeloutside=False)
        s.configure("TLabelframe.Label", background=papier, foreground=C["etiquette"],
                    font="MemoireGras")
        s.layout("Carte.TFrame", [("Carte.bordure", {"sticky": "nsew"})])
        s.configure("Carte.TFrame", background=papier)
        # Encadré clair (« Ce que ça va changer », maquette) : fond #FFF8F4, liseré fin, ombre
        # courte ; ce qu'il contient est sur ce même fond.
        self._element("Encadre.bordure", [((), self._forme(
            200, 200, 20, fond=rgba(champ), bord=rgba(encre, .08), ombre=ombre(COURTE, .06)))],
            20, (14, 12))
        s.layout("Encadre.TFrame", [("Encadre.bordure", {"sticky": "nsew"})])
        s.configure("Encadre.TFrame", background=papier)
        s.configure("EncadreInterieur.TFrame", background=champ)
        s.configure("Encadre.TLabel", background=champ, foreground=encre)
        s.configure("EncadreEtiquette.TLabel", background=champ, foreground=C["etiquette"],
                    font="MemoireEtiquette")
        s.configure("EncadreSecond.TLabel", background=champ, foreground=C["texte_second"],
                    font="MemoirePetite")

        # Lecture (8.2c) : encart de la question ; pastilles d'étiquette (genre, états).
        self._element("Encart.cadre", [((), self._forme(200, 200, 18, fond=rgba(C["encart"])))],
                      18, (14, 12))
        s.layout("Encart.TFrame", [("Encart.cadre", {"sticky": "nsew"})])
        s.configure("Encart.TFrame", background=papier)
        s.configure("Encart.TLabel", background=C["encart"], foreground=C["etiquette"],
                    font="MemoireEtiquette")
        for nom, fond_, texte_ in (("Genre", C["genre"], encre),
                                   ("PuceBleue", C["puce_bleue"], C["texte_bleu"]),
                                   ("PuceAmbre", C["puce_ambre"], C["texte_ambre"])):
            self._element(f"{nom}.fond", [((), self._forme(36, 26, 11, fond=rgba(fond_),
                                                            bord=rgba(encre, .08)))], 11, (10, 3))
            s.layout(f"{nom}.TLabel", [(f"{nom}.fond", {"sticky": "nsew", "children": [
                ("Label.padding", {"sticky": "nsew", "children": [
                    ("Label.label", {"sticky": "nsew"})]})]})])
            s.configure(f"{nom}.TLabel", background=fond_, foreground=texte_,
                        font="MemoireGras" if nom == "Genre" else "MemoirePetite", padding=(1, 0))
        # Bascule (Rendu | Brut, versions) : le choix en pastille blanche à ombre courte.
        self._element("Choisi.bordure", [
            ((), pastille(36, 24, 10, rgba("#FFFFFF"), rgba(encre, .14), OMBRE_COURTE)),
        ], 10, (9, 3))
        s.layout("Choisi.TButton", [("Choisi.bordure", {"sticky": "nsew", "children": [
            ("Button.padding", {"sticky": "nsew", "children": [
                ("Button.label", {"sticky": "nsew"})]})]})])
        s.configure("Choisi.TButton", font="MemoireGras", padding=(1, 0), foreground=encre)

        # Cases « Si besoin » : puces ; cochées, encre pleine.
        def puce(fond, bord, ombre=SANS_OMBRE):
            return self._forme(44, 30, 12, fond=fond, bord=bord, ombre=ombre)

        self._element("Puce.fond", [
            ((), puce(transparent, rgba(encre, .24))),
            (("disabled",), puce(transparent, rgba(encre, .08))),
            (("selected", "active"), puce(rgba("#3A3346"), rgba("#3A3346"), OMBRE_COURTE)),
            (("selected",), puce(rgba(encre), rgba(encre))),
            (("active",), puce(rgba("#FFFFFF"), rgba(encre, .16), OMBRE_COURTE)),
        ], 12, (9, 4))
        self.style.element_create(
            "Puce.signe", "image", self._marge(self.icone("plus", 14), 6, fond=papier),
            ("selected", "active", self._marge(self.icone("coche", 14, C["creme"]), 6,
                                               fond="#3A3346")),
            ("selected", self._marge(self.icone("coche", 14, C["creme"]), 6, fond=encre)),
            ("active", self._marge(self.icone("plus", 14), 6, fond="#FFFFFF")), sticky="")
        s.layout("TCheckbutton", [("Puce.fond", {"sticky": "nsew", "children": [
            ("Checkbutton.padding", {"sticky": "nsew", "children": [
                ("Puce.signe", {"side": "left", "sticky": ""}),
                ("Checkbutton.label", {"side": "left", "sticky": "nsew"})]})]})])
        s.configure("TCheckbutton", padding=(1, 1), foreground=encre)
        s.map("TCheckbutton", foreground=[("disabled", "#9A8E9C"), ("selected", C["creme"])])

        # Choix : pastilles ; choisie, blanche à ombre courte et point indigo.
        self._element("Choix.fond", [
            ((), puce(transparent, rgba(encre, .16))),
            (("disabled",), puce(transparent, rgba(encre, .06))),
            (("selected",), puce(rgba("#FFFFFF"), rgba(encre, .18), OMBRE_COURTE)),
            (("active",), puce(rgba("#FFFAF7"), rgba(encre, .16))),
        ], 12, (10, 5))
        vide = self._photo(dessin.forme(12, 12, 6, rgba(champ, 0), bord=rgba(encre, .35),
                                        epaisseur=1.3))
        plein = self._photo(dessin.forme(12, 12, 6, rgba(C["indigo"])))
        self.style.element_create("Choix.point", "image", self._marge(vide, 8, fond=papier),
                                  ("selected", self._marge(plein, 8, fond="#FFFFFF")),
                                  ("active", self._marge(vide, 8, fond="#FFFAF7")), sticky="")
        s.layout("TRadiobutton", [("Choix.fond", {"sticky": "nsew", "children": [
            ("Radiobutton.padding", {"sticky": "nsew", "children": [
                ("Choix.point", {"side": "left", "sticky": ""}),
                ("Radiobutton.label", {"side": "left", "sticky": "nsew"})]})]})])
        s.configure("TRadiobutton", padding=(1, 1), foreground=encre)

        # Barres de défilement fines : un jeu d'images par surface, chacune aplatie dessus.
        for prefixe, fond in (("", papier), ("Champ.", champ), ("Sombre.", nuit)):
            pouce = rgba("#FFFFFF" if prefixe == "Sombre." else encre, .22)
            pouce_actif = rgba("#FFFFFF" if prefixe == "Sombre." else encre, .38)
            rail = self._photo(dessin.forme(10, 10, 0, rgba(fond)), fond)
            self.style.element_create(f"{prefixe}Barre.rail", "image", rail, sticky="nsew",
                                      border=1, padding=(2, 2))
            for orient, (l, h) in (("Vertical", (8, 36)), ("Horizontal", (36, 8))):
                normal = self._photo(dessin.forme(l, h, 4, pouce), fond)
                actif = self._photo(dessin.forme(l, h, 4, pouce_actif), fond)
                self.style.element_create(f"{prefixe}Barre.{orient}.pouce", "image", normal,
                                          ("pressed", actif), ("active", actif),
                                          border=(3, 4) if orient == "Vertical" else (4, 3),
                                          sticky="nsew")
                cote = "ns" if orient == "Vertical" else "ew"
                s.layout(f"{prefixe}{orient}.TScrollbar", [(f"{prefixe}Barre.rail", {
                    "sticky": cote, "children": [(f"{prefixe}Barre.{orient}.pouce", {
                        "expand": "1", "sticky": "nsew"})]})])
                s.configure(f"{prefixe}{orient}.TScrollbar", background=fond, troughcolor=fond)

        # Arbres (liste des terminaux).
        s.layout("Treeview", [("Champ.champ", {"sticky": "nsew", "children": [
            ("Treeview.padding", {"sticky": "nsew", "children": [
                ("Treeview.treearea", {"sticky": "nsew"})]})]})])
        s.configure("Treeview", background=champ, fieldbackground=champ, foreground=encre,
                    rowheight=28, borderwidth=0, padding=(0, 0))
        s.map("Treeview", background=[("selected", C["indigo_doux"])],
              foreground=[("selected", encre)])
        s.configure("Treeview.Heading", background=champ, foreground=C["etiquette"],
                    font="MemoireEtiquette", relief="flat", borderwidth=0, padding=(6, 4))
        s.map("Treeview.Heading", background=[("active", champ)])

        # Surfaces de la lecture (8.3) : bascule groupée, barre de sélection, encadré clair.
        segment, pilule = C["segment"], C["survol"]
        self._element("Segment.fond", [((), self._forme(40, 32, 13, fond=rgba(segment)))], 13, (3, 3))
        s.layout("Segment.TFrame", [("Segment.fond", {"sticky": "nsew"})])
        s.configure("Segment.TFrame", background=papier)
        self.decliner("Choisi.TButton", "Choisi.bordure", segment, "Segment")
        self._element("Discret.bordure", [
            ((), pastille(36, 24, 10, rgba(segment, 0), None, SANS_OMBRE)),
            (("active",), pastille(36, 24, 10, rgba("#FFFFFF", .55), None, SANS_OMBRE)),
        ], 10, (9, 3), fond=segment)
        s.layout("SegmentAutre.TButton", [("Discret.bordure", {"sticky": "nsew", "children": [
            ("Button.padding", {"sticky": "nsew", "children": [
                ("Button.label", {"sticky": "nsew"})]})]})])
        s.configure("SegmentAutre.TButton", font="MemoireGras", padding=(1, 0),
                    foreground=C["texte_second"], background=segment)
        self._element("Pilule.fond", [((), self._forme(
            80, 56, 26, fond=rgba(pilule), bord=rgba(encre, .08), ombre=ombre(PASTILLE, .10)))],
            26, (10, 8))
        s.layout("Pilule.TFrame", [("Pilule.fond", {"sticky": "nsew"})])
        s.configure("Pilule.TFrame", background=papier)
        s.configure("PiluleInterieur.TFrame", background=pilule)
        s.configure("PiluleSecond.TLabel", background=pilule, foreground=C["texte_second"])
        self.decliner("Action.TButton", "Action.bordure", pilule, "Pilule")
        self.decliner("Bandeau.TButton", "Petit.bordure", pilule, "Pilule")
        self.decliner("Bandeau.TButton", "Petit.bordure", C["champ"], "Encadre")

        # Séparations des panneaux : une poignée visible au milieu (on la tire pour redimensionner,
        # le journal par exemple).
        # « Horizontal.Sash » sépare des panneaux empilés : la poignée y est horizontale.
        for nom, (l, h), cote in (("Horizontal.Sash", (40, 5), "horizontale"),
                                  ("Vertical.Sash", (5, 40), "verticale")):
            poignee = self._photo(dessin.forme(l, h, 2.5, rgba(encre, .22)), papier)
            poignee_active = self._photo(dessin.forme(l, h, 2.5, rgba(C["indigo"], .75)), papier)
            self.style.element_create(f"Poignee.{cote}", "image", poignee,
                                      ("active", poignee_active), ("pressed", poignee_active),
                                      sticky="")
            s.layout(nom, [(f"Poignee.{cote}", {"sticky": ""})])
            s.configure(nom, background=papier)
        s.configure("Sash", sashthickness=12, gripcount=0, background=papier,
                    lightcolor=papier, darkcolor=papier, bordercolor=papier)
        s.configure("TPanedwindow", background=papier)

    # ----- Widgets classiques -----

    def classiques(self):
        o = self.racine.option_add
        champ, encre = C["champ"], C["encre"]
        for cle, valeur in {
            "*Text.background": champ, "*Text.foreground": encre, "*Text.relief": "flat",
            "*Text.borderWidth": 0, "*Text.highlightThickness": 0,
            "*Text.selectBackground": C["indigo_doux"], "*Text.selectForeground": encre,
            "*Text.inactiveSelectBackground": C["indigo_doux"],
            "*Text.insertBackground": encre, "*Text.padX": 8, "*Text.padY": 6,
            "*Listbox.background": C["survol"], "*Listbox.foreground": encre,
            "*Listbox.selectBackground": C["indigo_doux"], "*Listbox.selectForeground": encre,
            "*Listbox.relief": "flat", "*Listbox.borderWidth": 0,
            "*Listbox.highlightThickness": 1, "*Listbox.highlightBackground": C["separation"],
            "*Listbox.highlightColor": C["separation"], "*Listbox.activeStyle": "none",
            "*Listbox.font": "TkDefaultFont",
            "*Menu.background": C["survol"], "*Menu.foreground": encre,
            "*Menu.activeBackground": C["indigo_doux"], "*Menu.activeForeground": encre,
            "*Menu.relief": "flat", "*Menu.borderWidth": 1, "*Menu.activeBorderWidth": 0,
            "*Menu.selectColor": C["indigo"], "*Menu.font": "TkMenuFont",
            "*Toplevel.background": C["papier"], "*Frame.background": C["papier"],
            "*Canvas.background": C["papier"], "*Canvas.highlightThickness": 0,
        }.items():
            o(cle, valeur)


def forme_etiree(maitre, largeur, hauteur, rayon, **options):
    """Grande forme sans dégradé ni grain, étirée par Tk : (PhotoImage, dessin.Image sans PNG).

    Optimisation (redimensionnement) : une petite forme est calculée, juste assez grande pour ses
    coins, son liseré, son halo et son ombre ; Tk recopie ses coins et répète sa colonne et sa
    rangée du milieu, identiques dans la grande (le calcul rapide de `dessin.forme` repose sur la
    même propriété). Essai : test_une_grande_forme_etiree_est_celle_du_calcul_complet."""
    dx = dy = flou = 0
    if options.get("ombre"):
        dx, dy, flou, _ = options["ombre"]
    taille_halo = options["halo"][0] if options.get("halo") else 0
    epaisseur = options.get("epaisseur", 1.0) if options.get("bord") else 0
    marge = math.ceil(rayon + max(taille_halo, epaisseur, flou)) + 2
    l0 = min(largeur, 2 * (marge + abs(dx)) + 3)
    h0 = min(hauteur, 2 * (marge + abs(dy)) + 3)
    petite = dessin.forme(l0, h0, rayon, **options)
    source = photo(maitre, petite)
    if (l0, h0) == (largeur, hauteur):
        return source, petite
    plus_l, plus_h = largeur - l0, hauteur - h0
    grande = tk.PhotoImage(master=maitre, width=petite.largeur + plus_l,
                           height=petite.hauteur + plus_h)
    xs, ys = petite.x + l0 // 2, petite.y + h0 // 2
    colonnes = ((0, xs, 0, xs), (xs, xs + 1, xs, xs + 1 + plus_l),
                (xs + 1, petite.largeur, xs + 1 + plus_l, petite.largeur + plus_l))
    rangees = ((0, ys, 0, ys), (ys, ys + 1, ys, ys + 1 + plus_h),
               (ys + 1, petite.hauteur, ys + 1 + plus_h, petite.hauteur + plus_h))
    for sx0, sx1, gx0, gx1 in colonnes:
        for sy0, sy1, gy0, gy1 in rangees:
            if sx1 > sx0 and sy1 > sy0:
                grande.tk.call(grande, "copy", source, "-from", sx0, sy0, sx1, sy1,
                               "-to", gx0, gy0, gx1, gy1, "-compositingrule", "set")
    return grande, dessin.Image(petite.largeur + plus_l, petite.hauteur + plus_h, b"",
                                petite.x, petite.y)


def police(taille=10, *styles, fixe=False):
    """Police (famille, taille, styles) de la famille choisie par le thème."""
    nom = "TkFixedFont" if fixe else "TkDefaultFont"
    try:
        famille = tkfont.nametofont(nom).cget("family")
    except (tk.TclError, RuntimeError):
        famille = nom
    return (famille, taille, *styles)


def appliquer(racine):
    """Applique le thème à la racine (une fois) et le rend."""
    theme = getattr(racine, "_theme_memoire", None)
    if theme is None:
        theme = Theme(racine)
        racine._theme_memoire = theme
        import menus  # les listes déroulantes s'ouvrent dans une carte (8.2a)
        menus.installer(racine)
    return theme


def theme_de(widget):
    """Le thème de la racine d'un widget (appliqué au besoin)."""
    return appliquer(widget.winfo_toplevel()._root())


class ZoneDeTexte(tk.Text):
    """Zone de texte dans un cadre arrondi, avec une barre de défilement fine ; se place comme
    un widget unique (`pack`, `grid`, `place` portent sur le cadre), comme `ScrolledText`."""

    def __init__(self, master=None, sombre=False, defilement=True, **options):
        self.frame = ttk.Frame(master, style="SombreCadre.TFrame" if sombre else "Champ.TFrame",
                               padding=(8, 6) if sombre else (6, 5))
        self._sombre = sombre
        fond = C["terminal"] if sombre else C["champ"]
        options.setdefault("background", fond)
        options.setdefault("borderwidth", 0)
        options.setdefault("highlightthickness", 0)
        options.setdefault("relief", "flat")
        if sombre:
            options.setdefault("foreground", C["terminal_texte"])
            options.setdefault("insertbackground", C["terminal_texte"])
            options.setdefault("selectbackground", "#4A4380")
            options.setdefault("selectforeground", "#FFFFFF")
        self.vbar = None
        if defilement:
            self.vbar = ttk.Scrollbar(self.frame, orient="vertical",
                                      style="Sombre.Vertical.TScrollbar" if sombre
                                      else "Champ.Vertical.TScrollbar")
            options["yscrollcommand"] = self.vbar.set
        super().__init__(self.frame, **options)
        tk.Text.pack(self, side="left", fill="both", expand=True)
        if self.vbar is not None:
            self.vbar.pack(side="right", fill="y")
            self.vbar["command"] = self.yview
        if not sombre:
            self.bind("<FocusIn>", lambda _e: self.frame.configure(style="ChampActif.TFrame"),
                      add="+")
            self.bind("<FocusOut>", lambda _e: self.frame.configure(style="Champ.TFrame"),
                      add="+")
        methodes_du_texte = vars(tk.Text).keys()
        geometrie = vars(tk.Pack).keys() | vars(tk.Grid).keys() | vars(tk.Place).keys()
        for nom in geometrie - methodes_du_texte:
            if nom[0] != "_" and nom not in ("config", "configure"):
                setattr(self, nom, getattr(self.frame, nom))


class Flux(ttk.Frame):
    """Cadre qui range ses enfants de gauche à droite et passe à la ligne quand la place manque
    (puces « Si besoin », choix) ; ses enfants sont créés avec lui pour parent."""

    def __init__(self, master=None, ecart=8, interligne=6, **options):
        super().__init__(master, width=200, height=1, **options)
        self.ecart, self.interligne = ecart, interligne
        self.enfants = []
        self.bind("<Configure>", lambda _e: self.ranger(), add="+")

    def ajouter(self, widget):
        self.enfants.append(widget)
        # Un enfant qui change de taille (rempli après coup) fait ranger de nouveau, au repos.
        widget.bind("<Configure>", lambda _e: self._ranger_bientot(), add="+")
        self.ranger()
        return widget

    def _ranger_bientot(self):
        if not getattr(self, "_attente", None):
            self._attente = self.after_idle(self._ranger_maintenant)

    def _ranger_maintenant(self):
        self._attente = None
        self.ranger()

    def ranger(self):
        largeur = self.winfo_width()
        if largeur <= 1:
            largeur = 640
        x = y = hauteur_de_rangee = 0
        for widget in self.enfants:
            l, h = widget.winfo_reqwidth(), widget.winfo_reqheight()
            if x > 0 and x + l > largeur:
                x, y = 0, y + hauteur_de_rangee + self.interligne
                hauteur_de_rangee = 0
            widget.place(x=x, y=y)
            x += l + self.ecart
            hauteur_de_rangee = max(hauteur_de_rangee, h)
        hauteur = y + hauteur_de_rangee
        if int(self.cget("height")) != hauteur:
            self.configure(height=max(1, hauteur))


class ZoneDefilante(ttk.Frame):
    """Cadre dont le contenu (`interieur`) défile verticalement quand il dépasse."""

    def __init__(self, master=None, **options):
        super().__init__(master, **options)
        fond = ttk.Style(self).lookup("TFrame", "background") or C["papier"]
        self.canevas = tk.Canvas(self, highlightthickness=0, borderwidth=0, background=fond)
        self.barre = ttk.Scrollbar(self, orient="vertical", command=self.canevas.yview)
        self.canevas.configure(yscrollcommand=self._barre_si_besoin)
        self.interieur = ttk.Frame(self.canevas)
        self._fenetre = self.canevas.create_window(0, 0, window=self.interieur, anchor="nw")
        self.canevas.pack(side="left", fill="both", expand=True)
        self.interieur.bind("<Configure>", lambda _e: self._region(), add="+")
        self.canevas.bind("<Configure>", lambda e: (
            self.canevas.itemconfigure(self._fenetre, width=e.width), self._region()), add="+")
        self.bind_all("<Button-4>", lambda e: self._molette(e, -1), add="+")
        self.bind_all("<Button-5>", lambda e: self._molette(e, 1), add="+")
        self.bind_all("<MouseWheel>", lambda e: self._molette(e, -1 if e.delta > 0 else 1), add="+")

    def _region(self):
        self.canevas.configure(scrollregion=self.canevas.bbox("all") or (0, 0, 0, 0))

    def _barre_si_besoin(self, debut, fin):
        self.barre.set(debut, fin)
        if float(debut) <= 0.0 and float(fin) >= 1.0:
            self.barre.pack_forget()
        elif not self.barre.winfo_manager():
            self.barre.pack(side="right", fill="y")

    def _molette(self, evenement, pas):
        """La molette fait défiler la zone sous le pointeur, sauf un widget qui défile déjà."""
        try:
            widget = self.winfo_containing(evenement.x_root, evenement.y_root)
        except (KeyError, tk.TclError):
            return
        if widget is None or isinstance(widget, (tk.Text, tk.Listbox)):
            return
        chemin = str(widget)
        if chemin.startswith(str(self)) and self.barre.winfo_manager():
            self.canevas.yview_scroll(pas * 2, "units")
