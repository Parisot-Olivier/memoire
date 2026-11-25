"""Composition de la fenêtre principale (APPARENCE.md, section 3 ; D31).

Un canevas occupe la fenêtre : dégradé, en-tête (marque, pastille signature, boutons ronds), rail
des sujets en verre givré, plaque de verre, ombre de la carte, barre d'état en pastilles. La carte
papier est un cadre posé dessus ; elle reçoit le plan de travail. La liste des actions du sujet
choisi et la recherche (Ctrl K) sont ici aussi. Aucune règle : les actions sont celles du catalogue,
l'état vient de l'exécutable.
"""
import subprocess
import tkinter as tk
import tkinter.font as tkfont
from pathlib import Path
from tkinter import ttk

import apparence
import dessin
from actions import ACTIONS, GROUPES, mot
from apparence import COULEURS as C
from dessin import rgba
import menus
from theme import forme_etiree, photo  # noqa: F401 (forme_etiree : essais)

MODE_EXPERT = "Toutes les commandes (mode expert)"
LETTRES_MINIMALES = 2
DELAI_DE_FRAPPE_MS = 200

# Mesures (pixels).
MARGE_X, MARGE_HAUT, MARGE_BAS = 28, 20, 16
ENTETE_H, ECART, RAIL_L, BARRE_H = 64, 22, 210, 44
RAYON_CARTE, DEBORD_PLAQUE, RETRAIT_CARTE = 28, 16, 10
ELEMENT_H = 44

VERRE = rgba(*apparence.VERRE_GIVRE)
LISERE_VERRE = rgba("#FFFFFF", .70)
OMBRE_PASTILLE = (5, 7, 4, rgba(C["encre"], .22))
OMBRE_COURTE = (3, 4, 3, rgba(C["encre"], .14))


class Images:
    """PhotoImages calculées à la demande, gardées par clé (une seule version par rôle)."""

    def __init__(self, maitre):
        self.maitre = maitre
        self._par_role = {}
        self._fixes = {}

    def role(self, role, cle, fabrique, fond=None):
        """Image d'un rôle (« carte », « rail »…) : recalculée seulement si sa clé change ;
        aplatie sur `fond` s'il est donné (dessin plus rapide, voir theme.aplatir)."""
        ancienne = self._par_role.get(role)
        if ancienne is not None and ancienne[0] == cle:
            return ancienne[1], ancienne[2]
        image = fabrique()
        p = photo(self.maitre, image, fond)
        self._par_role[role] = (cle, p, image)
        return p, image

    def etiree(self, role, cle, largeur, hauteur, rayon, **options):
        """Grande forme d'un rôle, étirée par Tk (voir `forme_etiree`)."""
        ancienne = self._par_role.get(role)
        if ancienne is not None and ancienne[0] == cle:
            return ancienne[1], ancienne[2]
        p, image = forme_etiree(self.maitre, largeur, hauteur, rayon, **options)
        self._par_role[role] = (cle, p, image)
        return p, image

    def agrandie(self, role, cle, petite, echelle):
        """Copie agrandie (`echelle` fois) d'une PhotoImage, gardée comme un rôle."""
        ancienne = self._par_role.get(role)
        if ancienne is not None and ancienne[0] == cle:
            return ancienne[1]
        grande = tk.PhotoImage(master=self.maitre)
        grande.tk.call(grande, "copy", petite, "-zoom", echelle, echelle)
        self._par_role[role] = (cle, grande, None)
        return grande

    def fixe(self, cle, fabrique):
        """Image qui ne change pas (pictogrammes, points, pastilles de taille fixe)."""
        if cle not in self._fixes:
            image = fabrique()
            self._fixes[cle] = (photo(self.maitre, image), image)
        return self._fixes[cle]


def tronquer(texte, police, largeur):
    """Texte raccourci par « … » pour tenir dans `largeur` pixels."""
    if police.measure(texte) <= largeur:
        return texte
    while texte and police.measure(texte + "…") > largeur:
        texte = texte[:-1]
    return texte.rstrip() + "…"


class Habillage:
    """Fond et cadre de la fenêtre principale de `app` (memoire_gui.Application).

    Optimisation : tout le fond est UNE image opaque (`travail`), composée dans Tk (en C). La
    couche statique (dégradé, plaque, carte, rail, signature, boutons ronds, pictogrammes) n'est
    recomposée qu'au redimensionnement ; la couche d'état (sujet choisi, badge, barre d'état,
    piste) l'est à chaque relecture, par une copie de la couche statique. Le survol et les clics
    passent par des zones calculées au dessin : aucune liaison par élément."""

    def __init__(self, app):
        self.app = app
        racine = app.racine
        self.racine = racine
        self.images = Images(racine)
        self.canevas = tk.Canvas(racine, highlightthickness=0, borderwidth=0,
                                 background=apparence.DEGRADE_DE_FOND[2][1])
        self.canevas.place(x=0, y=0, relwidth=1, relheight=1)
        self.carte = ttk.Frame(racine, padding=(4, 4))
        self.geometrie = {}
        self.zones = []
        self._zone = None
        self._statique = None
        self._cle_statique = None
        self.travail = tk.PhotoImage(master=racine, width=1, height=1)
        # Les cartes de menu posées dans la fenêtre prennent leur fond dans cette image.
        racine._fond_memoire = self.travail
        self.canevas.create_image(0, 0, image=self.travail, anchor="nw", tags=("fond",))
        self._taille = None
        self._attente = None
        self._pouls = None
        self._pouls_allume = True
        self._points_du_pouls = {}
        self.segments = []
        self.police_gras = tkfont.nametofont("MemoireGras", root=racine)
        self.police = tkfont.nametofont("TkDefaultFont", root=racine)
        self.police_mono = tkfont.nametofont("MemoireMonoPetite", root=racine)
        racine.bind("<Configure>", self._configure, add="+")
        self.canevas.bind("<Motion>", self._mouvement)
        self.canevas.bind("<Leave>", lambda _e: self._survoler(None))
        self.canevas.bind("<Button-1>", self._clic)
        self.menu_projet = menus.Menu(racine)
        self.menu_projet.add_command(label="Changer de projet…", command=app.choisir_base)
        self.menu_projet.add_separator()
        self.menu_projet.add_command(label="Où en est le projet ?",
                                     command=lambda: app.ouvrir_action("ou_en_est_le_projet"))
        self.menu_projet.add_command(label="Sauvegarder le projet",
                                     command=lambda: app.ouvrir_action("sauvegarder"))

    # ----- Géométrie -----

    def _configure(self, evenement):
        """Redimensionnement : un seul dessin, 120 ms après le dernier changement de taille."""
        if evenement.widget is not self.racine:
            return
        taille = (evenement.width, evenement.height)
        if taille == self._taille:
            return
        self._taille = taille
        if self._attente is not None:
            self.racine.after_cancel(self._attente)
        self._attente = self.racine.after(120, self.dessiner)

    def disposer(self, largeur, hauteur):
        """Rectangles (x, y, l, h) de chaque partie pour une fenêtre de cette taille."""
        corps_y = MARGE_HAUT + ENTETE_H + ECART
        barre_y = hauteur - MARGE_BAS - BARRE_H
        carte_bas = barre_y - 18
        carte_x = MARGE_X + RAIL_L + 24
        carte = (carte_x, corps_y, max(200, largeur - MARGE_X - DEBORD_PLAQUE - carte_x),
                 max(200, carte_bas - corps_y))
        signature_l = max(560, min(800, largeur - 2 * 250))
        return {
            "signature": ((largeur - signature_l) // 2, MARGE_HAUT, signature_l, ENTETE_H),
            "rail": (MARGE_X, corps_y, RAIL_L, carte[3]),
            "carte": carte,
            "plaque": (carte_x + 44, corps_y + 84, largeur - 14 - carte_x - 44,
                       hauteur - 10 - corps_y - 84),
            "barre": (carte_x + 56, barre_y, largeur - carte_x - 56 - 34, BARRE_H),
            "taille": (largeur, hauteur),
        }

    # ----- Dessin -----

    def dessiner(self):
        """Redessine tout pour la taille actuelle et replace la carte."""
        self._attente = None
        largeur, hauteur = self.racine.winfo_width(), self.racine.winfo_height()
        if largeur < 50 or hauteur < 50:
            largeur, hauteur = 1400, 860
        self.geometrie = g = self.disposer(largeur, hauteur)
        x, y, l, h = g["carte"]
        self.carte.place(x=x + RETRAIT_CARTE, y=y + RETRAIT_CARTE, width=l - 2 * RETRAIT_CARTE,
                         height=h - 2 * RETRAIT_CARTE)
        self._couche_statique()
        self.dessiner_etat()

    def redessiner(self):
        """Après une relecture de l'état : seule la couche d'état change."""
        if self.geometrie:
            self.dessiner_etat()

    @staticmethod
    def _composer(cible, photo_, image, x, y):
        """Pose une image de forme sur `cible` : (x, y) est le coin de la forme, hors ombre."""
        cible.tk.call(cible, "copy", photo_, "-to", int(x - image.x), int(y - image.y),
                      "-compositingrule", "overlay")

    def _couche_statique(self):
        g = self.geometrie
        largeur, hauteur = g["taille"]
        cle = (largeur, hauteur)
        c = self.canevas
        c.delete("statique")
        if cle != self._cle_statique:
            base = tk.PhotoImage(master=self.racine, width=largeur, height=hauteur)
            # Le dégradé est lisse : calculé au huitième, agrandi par Tk.
            echelle = 8
            petit, _ = self.images.role("fond-petit", (largeur // echelle + 1,
                                                       hauteur // echelle + 1),
                                        lambda: dessin.degrade(largeur // echelle + 1,
                                                               hauteur // echelle + 1,
                                                               dessin.DEGRADE_DE_FOND, 128))
            base.tk.call(base, "copy", petit, "-zoom", echelle, echelle)
            for nom, fabrique in self._formes_statiques():
                photo_, image, x, y = fabrique()
                self._composer(base, photo_, image, x, y)
            self._statique, self._cle_statique = base, cle
        # Textes et zones de la couche statique.
        self._textes_statiques()

    def _formes_statiques(self):
        g = self.geometrie
        largeur, _ = g["taille"]
        formes = []

        def ajouter(nom, role, cle, fabrique, x, y):
            def faire():
                p, image = self.images.role(role, cle, fabrique)
                return p, image, x, y
            formes.append((nom, faire))

        def etirer(role, rect, rayon, **options):
            x, y, l, h = rect
            formes.append((role, lambda: (*self.images.etiree(role, (l, h), l, h, rayon,
                                                               **options), x, y)))

        etirer("plaque", g["plaque"], 30, fond=rgba("#FFFFFF", .10), bord=rgba("#FFFFFF", .55))
        etirer("carte", g["carte"], RAYON_CARTE, fond=rgba(C["papier"]), bord=rgba("#FFFFFF", .9),
               ombre=(0, 18, 26, rgba("#3C1E6E", .30)))
        etirer("rail", g["rail"], 26, fond=VERRE, bord=LISERE_VERRE, ombre=OMBRE_PASTILLE)
        rx, ry, rl, rh = g["rail"]
        ajouter("marque", "marque", (), lambda: dessin.forme(
            28, 28, 14, rgba(C["indigo"]), degrade=dessin.DEGRADE_SIGNATURE,
            bord=rgba("#FFFFFF", .55), ombre=OMBRE_COURTE), MARGE_X,
            MARGE_HAUT + ENTETE_H // 2 - 14)
        sx, sy, sl, sh = g["signature"]
        ajouter("signature", "signature", (sl, sh), lambda: dessin.forme(
            sl, sh, sh / 2, rgba(C["indigo"]), degrade=dessin.DEGRADE_SIGNATURE,
            bord=rgba("#FFFFFF", .45), halo=(22, rgba("#FFFFFF", .28)), ombre=OMBRE_PASTILLE),
            sx, sy)
        # Recherche : entre le projet (à gauche) et la piste (à droite), largeur fixe.
        ra, rb = self._recherche_x()
        milieu = sy + sh / 2
        ajouter("recherche", "recherche", (rb - ra,), lambda: dessin.forme(
            rb - ra, 48, 24, rgba(*apparence.VERRE_TEINTE), bord=rgba("#FFFFFF", .55)),
            ra, milieu - 24)
        touche_l = self.police_mono.measure("Ctrl K") + 20
        ajouter("touche", "touche", (touche_l,), lambda: dessin.forme(
            touche_l, 26, 13, rgba("#FFFFFF", .16), bord=rgba("#FFFFFF", .45)),
            rb - 10 - touche_l, milieu - 13)
        blanc = C["blanc"]
        ajouter("loupe", "loupe", (), lambda: dessin.icone("chercher", 18, rgba(blanc)),
                ra + 18, milieu - 9)
        # Boutons ronds et leurs pictogrammes ; pictogrammes du rail.
        for rang, (nom, _titre) in enumerate(self._outils()):
            ox = largeur - MARGE_X - 46 - 56 * (2 - rang)
            oy = MARGE_HAUT + (ENTETE_H - 46) // 2
            ajouter(f"rond-{nom}", "rond", (), lambda: dessin.forme(
                46, 46, 23, VERRE, bord=LISERE_VERRE, ombre=OMBRE_PASTILLE), ox, oy)
            ajouter(f"outil-{nom}", f"icone-{nom}", (), lambda n=nom: dessin.icone(
                n, 20, rgba(C["encre"])), ox + 13, oy + 13)
        for rang, (sujet, haut) in enumerate(zip(self.app.sujets(), self._positions_du_rail())):
            icone = apparence.ICONE_DU_SUJET.get(sujet, "expert")
            ajouter(f"icone-sujet-{rang}", f"icone-sujet-{icone}", (), lambda i=icone: dessin.icone(
                i, 18, rgba(C["encre"])), rx + 26, haut + ELEMENT_H / 2 - 9)
        return formes

    def _recherche_x(self):
        sx, _sy, sl, _sh = self.geometrie["signature"]
        largeur = min(sl - 2 * 170, 380)
        debut = sx + (sl - largeur) // 2 - 40
        return debut, debut + largeur

    def _outils(self):
        return [("terminaux", "Console"), ("panneaux", "Panneaux"), ("reglages", "Projet")]

    def _positions_du_rail(self):
        x, y, l, h = self.geometrie["rail"]
        positions = [y + 12 + rang * ELEMENT_H for rang in range(len(GROUPES))]
        positions.append(y + h - 12 - ELEMENT_H)
        return positions

    def _textes_statiques(self):
        c = self.canevas
        g = self.geometrie
        largeur, _ = g["taille"]
        blanc = C["blanc"]
        y = MARGE_HAUT + ENTETE_H // 2
        c.create_text(MARGE_X + 38, y, text="memoire", anchor="w", fill=blanc,
                      font="MemoireMarque", tags=("statique",))
        ra, rb = self._recherche_x()
        touche_l = self.police_mono.measure("Ctrl K") + 20
        # Le texte de la recherche dépend de la vue : il est dans la couche d'état.
        c.create_text(rb - 10 - touche_l / 2, y, text="Ctrl K", fill=blanc,
                      font="MemoireMonoPetite", tags=("statique", "recherche"))
        x, _y, l, _h = g["rail"]
        positions = self._positions_du_rail()
        c.create_line(x + 22, positions[-1] - 8, x + l - 22, positions[-1] - 8,
                      fill="#D9C5CF", width=1, tags=("statique",))

    # ----- Couche d'état -----

    def dessiner_etat(self):
        """Sujet choisi, badge, piste, barre d'état : copie de la couche statique, formes
        d'état composées dessus, textes et zones refaits."""
        g = self.geometrie
        largeur, hauteur = g["taille"]
        c = self.canevas
        self._arreter_le_pouls()
        c.delete("etat")
        self.zones = []
        self._zone = None
        travail = self.travail
        if travail.width() != largeur or travail.height() != hauteur:
            travail.configure(width=largeur, height=hauteur)
        travail.tk.call(travail, "copy", self._statique, "-compositingrule", "set")
        self._signature_etat()
        self._texte_de_la_recherche()
        self._zones_statiques()
        self._outils_etat()
        self._rail_etat()
        self._barre(g["barre"])
        c.tag_lower("fond")

    def _signature_etat(self):
        c = self.canevas
        sx, sy, sl, sh = self.geometrie["signature"]
        milieu = sy + sh / 2
        blanc = C["blanc"]
        chevron, ci = self.images.fixe(("chevron", blanc), lambda: dessin.icone(
            "chevron", 14, rgba(blanc)))
        ra, rb = self._recherche_x()
        projet = tronquer(self.app.nom_du_projet(), self.police_gras, ra - sx - 26 - 34)
        largeur_projet = self.police_gras.measure(projet)
        c.create_text(sx + 26, milieu, text=projet, anchor="w", fill=blanc, font="MemoireGras",
                      tags=("etat", "projet"))
        self._composer(self.travail, chevron, ci, sx + 26 + largeur_projet + 6, milieu - 7)
        self.zones.append((sx + 10, sy + 8, ra - 6, sy + sh - 8,
                           lambda e: self.menu_projet.tk_popup(e.x_root, e.y_root), None))
        piste = self.app.etat.piste or "projet à créer"
        droite = sx + sl - 26
        piste = tronquer(piste, self.police_gras, droite - 20 - 16 - rb - 14)
        largeur_piste = self.police_gras.measure(piste)
        point, pi = self.images.fixe(("point", blanc), lambda: dessin.forme(8, 8, 4, rgba(blanc)))
        self._composer(self.travail, chevron, ci, droite - 14, milieu - 7)
        c.create_text(droite - 20, milieu, text=piste, anchor="e", fill=blanc, font="MemoireGras",
                      tags=("etat", "piste", "piste-texte"))
        self._composer(self.travail, point, pi, droite - 20 - largeur_piste - 14, milieu - 4)
        self.zones.append((rb + 6, sy + 8, sx + sl - 10, sy + sh - 8,
                           lambda _e: self.app.ouvrir_action("aller_sur_piste"), None))

    def _texte_de_la_recherche(self):
        """« Chercher… » ; pendant la lecture, « Lire › titre de la note » (maquette)."""
        _sx, sy, _sl, sh = self.geometrie["signature"]
        ra, rb = self._recherche_x()
        touche_l = self.police_mono.measure("Ctrl K") + 20
        texte = "Chercher une note, une action…"
        lecture = getattr(self.app, "lecture", None)
        if getattr(self.app, "vue_de_la_carte", "action") == "lecture" and lecture is not None:
            texte = f"Lire › {lecture.titre_de_la_note.cget('text')}"
        self.canevas.create_text(ra + 46, sy + sh / 2,
                                 text=tronquer(texte, self.police, rb - ra - 70 - touche_l),
                                 anchor="w", fill=C["blanc"], font="TkDefaultFont",
                                 tags=("etat", "recherche", "texte-recherche"))

    def texte_de_la_recherche(self):
        return " ".join(self.canevas.itemcget(i, "text")
                        for i in self.canevas.find_withtag("texte-recherche"))

    def _zones_statiques(self):
        sx, sy, sl, sh = self.geometrie["signature"]
        ra, rb = self._recherche_x()
        milieu = sy + sh / 2
        self.zones.append((ra, milieu - 24, rb, milieu + 24,
                           lambda _e: self.app.recherche.ouvrir(), None))
        largeur, _ = self.geometrie["taille"]
        # Le premier bouton montre ou masque la console (CONSOLE.md, section 1) : un clic.
        menus = {"terminaux": lambda _e: self.app.basculer_terminal(),
                 "panneaux": lambda e: self._menu(self.app.plan.menu, e),
                 "reglages": lambda e: self.menu_projet.tk_popup(e.x_root, e.y_root)}
        for rang, (nom, _titre) in enumerate(self._outils()):
            ox = largeur - MARGE_X - 46 - 56 * (2 - rang)
            oy = MARGE_HAUT + (ENTETE_H - 46) // 2
            self.zones.append((ox, oy, ox + 46, oy + 46, menus[nom], None))

    def outils_choisis(self):
        """Boutons ronds de l'en-tête dessinés en pastille de papier : celui de la console quand
        elle est visible (CONSOLE.md, section 11)."""
        try:
            montree = getattr(self.app, "terminaux", None) is not None \
                and self.app.plan.visible("console")
        except (AttributeError, ValueError):
            montree = False
        return ["terminaux"] if montree else []

    def _outils_etat(self):
        """L'élément choisi ressort du verre : pastille de papier, pictogramme indigo."""
        largeur, _ = self.geometrie["taille"]
        choisis = self.outils_choisis()
        for rang, (nom, _titre) in enumerate(self._outils()):
            if nom not in choisis:
                continue
            ox = largeur - MARGE_X - 46 - 56 * (2 - rang)
            oy = MARGE_HAUT + (ENTETE_H - 46) // 2
            p, image = self.images.fixe("rond-choisi", lambda: dessin.forme(
                46, 46, 23, rgba(C["papier"]), bord=rgba("#FFFFFF", .9)))
            self._composer(self.travail, p, image, ox, oy)
            pi, pii = self.images.fixe(("outil-choisi", nom), lambda n=nom: dessin.icone(
                n, 20, rgba(C["indigo"])))
            self._composer(self.travail, pi, pii, ox + 13, oy + 13)

    def _menu(self, menu, evenement):
        if menu is not None:
            menu.tk_popup(evenement.x_root, evenement.y_root + 20)

    # ----- Survol et clics -----

    def _zone_en(self, x, y):
        for zone in self.zones:
            if zone[0] <= x < zone[2] and zone[1] <= y < zone[3]:
                return zone
        return None

    def _survoler(self, zone):
        if zone is self._zone:
            return
        c = self.canevas
        if self._zone is not None and self._zone[5] is not None:
            c.itemconfigure(self._zone[5], state="hidden")
        self._zone = zone
        if zone is not None and zone[5] is not None:
            c.itemconfigure(zone[5], state="normal")
        c.configure(cursor="hand2" if zone is not None else "")

    def _mouvement(self, evenement):
        self._survoler(self._zone_en(evenement.x, evenement.y))

    def _clic(self, evenement):
        zone = self._zone_en(evenement.x, evenement.y)
        if zone is not None:
            zone[4](evenement)

    # ----- Rail des sujets -----

    def _rail_etat(self):
        x, y, l, h = self.geometrie["rail"]
        c = self.canevas
        choisi, ci = self.images.fixe("rail-choisi", lambda: dessin.forme(
            l - 20, ELEMENT_H - 4, 20, rgba(C["papier"]), bord=rgba("#FFFFFF", .9),
            ombre=OMBRE_COURTE))
        survol, _si = self.images.fixe("rail-survol", lambda: dessin.forme(
            l - 20, ELEMENT_H - 4, 20, rgba("#FFFFFF", .40)))
        for rang, (sujet, haut) in enumerate(zip(self.app.sujets(), self._positions_du_rail())):
            tag = f"sujet-{rang}"
            courant = sujet == self.app.sujet_courant
            fond = None
            if courant:
                # La pastille couvre le pictogramme de la couche statique : on le repose dessus.
                self._composer(self.travail, choisi, ci, x + 10, haut + 2)
                icone = apparence.ICONE_DU_SUJET.get(sujet, "expert")
                pi, pii = self.images.role(f"icone-sujet-{icone}", (), lambda: dessin.icone(
                    icone, 18, rgba(C["encre"])))
                self._composer(self.travail, pi, pii, x + 26, haut + ELEMENT_H / 2 - 9)
            else:
                fond = c.create_image(x + 10, haut + 2, image=survol, anchor="nw",
                                      state="hidden", tags=("etat", tag))
            milieu = haut + ELEMENT_H / 2
            police = "MemoireGras" if courant else "TkDefaultFont"
            if sujet == MODE_EXPERT:
                c.create_text(x + 54, milieu - 7, text="Toutes les commandes", anchor="w",
                              fill=C["encre"], font=police, tags=("etat", tag))
                c.create_text(x + 54, milieu + 9, text="mode expert", anchor="w",
                              fill=C["texte_second"], font="MemoirePetite", tags=("etat", tag))
            else:
                c.create_text(x + 54, milieu, text=sujet, anchor="w", fill=C["encre"],
                              font=police, tags=("etat", tag))
            if sujet == "Où j’en suis" and self.app.etat.a_revoir:
                nombre = str(self.app.etat.a_revoir)
                largeur_badge = max(22, 12 + 8 * len(nombre))
                b, bi = self.images.fixe(("badge", largeur_badge), lambda: dessin.forme(
                    largeur_badge, 22, 11, rgba(C["orange"])))
                bx = x + l - 22 - largeur_badge
                self._composer(self.travail, b, bi, bx, milieu - 11)
                c.create_text(bx + largeur_badge / 2, milieu, text=nombre, fill=C["encre"],
                              font="MemoireGras", tags=("etat", tag))
            self.zones.append((x + 10, haut + 2, x + l - 10, haut + ELEMENT_H - 2,
                               lambda _e, s=sujet: self.app.choisir_sujet(s), fond))

    # ----- Barre d'état -----

    def segments_de_la_barre(self):
        """(texte, couleur du point, geste) de chaque pastille de la barre d'état."""
        etat = self.app.etat
        segments = [(etat.piste or "projet à créer", C["indigo"],
                     lambda: self.app.ouvrir_action("aller_sur_piste")),
                    (apparence.libelle_a_revoir(etat.a_revoir),
                     C["ambre"] if etat.a_revoir else C["vert"],
                     lambda: self.app.ouvrir_action("ou_en_est_le_projet"))]
        if etat.sans_appui:
            segments.append((apparence.libelle_sans_appui(etat.sans_appui), "#B0408F",
                             lambda: self.app.ouvrir_action("ou_en_est_le_projet")))
        for nom, commande in self.app.terminaux_en_cours():
            segments.append((f"terminal · {commande}", C["vert"],
                             lambda n=nom: self.app.terminaux.afficher(n)))
        return segments

    def _barre(self, rect):
        x, y, l, h = rect
        c = self.canevas
        self.segments = self.segments_de_la_barre()
        self._points_du_pouls = {}
        textes = [tronquer(t, self.police, 260) for t, _, _ in self.segments]
        largeurs = [24 + 8 + self.police.measure(t) for t in textes]
        total = sum(largeurs) + 12
        p, image = self.images.etiree("barre", (total,), total, h, h / 2, fond=VERRE,
                                      bord=LISERE_VERRE, ombre=OMBRE_PASTILLE)
        self._composer(self.travail, p, image, x, y)
        cx = x + 6
        pouls = False
        for rang, ((texte, couleur, geste), court, largeur) in enumerate(
                zip(self.segments, textes, largeurs)):
            tag = f"segment-{rang}"
            s, _si = self.images.role(f"survol-{rang}", (largeur,), lambda w=largeur: dessin.forme(
                w, h - 8, (h - 8) / 2, rgba("#FFFFFF", .45)))
            fond = c.create_image(cx, y + 4, image=s, anchor="nw", state="hidden",
                                  tags=("etat", tag))
            point, pi = self.images.fixe(("point", couleur), lambda co=couleur: dessin.forme(
                8, 8, 4, rgba(co)))
            if texte.startswith("terminal · "):
                # Le point d'un terminal en cours pulse : il reste un élément à part, qui
                # s'atténue sans disparaître.
                item = c.create_image(cx + 14, y + h / 2, image=point, anchor="w",
                                      tags=("etat", tag, "pouls"))
                attenue, _ai = self.images.fixe(("point-attenue", couleur), lambda co=couleur:
                                                dessin.forme(8, 8, 4, rgba(co, .35)))
                self._points_du_pouls[item] = (point, attenue)
                pouls = True
            else:
                self._composer(self.travail, point, pi, cx + 14, y + h / 2 - 4)
            c.create_text(cx + 30, y + h / 2, text=court, anchor="w", fill=C["encre"],
                          font="TkDefaultFont", tags=("etat", tag, "texte-barre"))
            self.zones.append((cx, y, cx + largeur, y + h, lambda _e, g=geste: g(), fond))
            cx += largeur
        # À droite : le fichier de base.
        nom = Path(self.app.base.get()).name or "aucune base"
        largeur = self.police_mono.measure(nom) + 40
        d, di = self.images.role("base", (largeur,), lambda: dessin.forme(
            largeur, h, h / 2, VERRE, bord=LISERE_VERRE, ombre=OMBRE_PASTILLE))
        dx = x + l - largeur
        if dx > cx + 20:
            self._composer(self.travail, d, di, dx, y)
            c.create_text(dx + largeur / 2, y + h / 2, text=nom, fill=C["encre"],
                          font="MemoireMonoPetite", tags=("etat", "base", "texte-barre"))
            self.zones.append((dx, y, dx + largeur, y + h,
                               lambda e: self.menu_projet.tk_popup(e.x_root, e.y_root), None))
        if pouls:
            self._battre()

    def textes_de_la_barre(self):
        return [self.canevas.itemcget(i, "text") for i in self.canevas.find_withtag("texte-barre")]

    def _battre(self):
        """Le point d'un terminal en cours pulse."""
        def battement():
            self._pouls = None
            items = self.canevas.find_withtag("pouls")
            if not items:
                return
            self._pouls_allume = not self._pouls_allume
            for item in items:
                plein, attenue = self._points_du_pouls[item]
                self.canevas.itemconfigure(item, image=plein if self._pouls_allume else attenue)
            self._pouls = self.racine.after(700, battement)

        self._pouls = self.racine.after(700, battement)

    def _arreter_le_pouls(self):
        if self._pouls is not None:
            try:
                self.racine.after_cancel(self._pouls)
            except tk.TclError:
                pass
            self._pouls = None

    def arreter(self):
        self._arreter_le_pouls()
        if self._attente is not None:
            try:
                self.racine.after_cancel(self._attente)
            except tk.TclError:
                pass
            self._attente = None


class ListeDesActions:
    """Les actions du sujet choisi, dans la carte : la choisie est une pastille blanche."""

    LARGEUR = 262

    def __init__(self, parent, app):
        self.app = app
        self.canevas = tk.Canvas(parent, width=self.LARGEUR, highlightthickness=0, borderwidth=0,
                                 background=C["papier"])
        self.titres = []
        self.cibles = []
        self.sujet = None
        self.choisi = None
        self._images = Images(self.canevas)
        self.canevas.bind("<Configure>", lambda _e: self.dessiner(), add="+")
        for sequence, pas in (("<Button-4>", -1), ("<Button-5>", 1)):
            self.canevas.bind(sequence, lambda _e, p=pas: self.canevas.yview_scroll(p * 2, "units"))
        self.canevas.bind("<MouseWheel>", lambda e: self.canevas.yview_scroll(
            -1 if e.delta > 0 else 1, "units"))

    def afficher(self, sujet):
        """Montre les actions de `sujet` (ou les commandes du mode expert)."""
        self.sujet = sujet
        if sujet == MODE_EXPERT:
            self.cibles = [("commande", c.nom) for c in self.app.commandes]
            self.titres = [c.nom for c in self.app.commandes]
        else:
            actions = [a for a in ACTIONS if a.groupe == sujet]
            self.cibles = [("action", a.cle) for a in actions]
            self.titres = [a.titre for a in actions]
        if self.choisi not in self.titres:
            self.choisi = None
        self.canevas.yview_moveto(0)
        self.dessiner()

    def marquer(self, titre):
        self.choisi = titre
        self.dessiner()

    def dessiner(self):
        c = self.canevas
        c.delete("all")
        if self.sujet is None:
            return
        largeur = max(self.LARGEUR, c.winfo_width())
        etiquette = "TOUTES LES COMMANDES" if self.sujet == MODE_EXPERT else self.sujet.upper()
        c.create_text(28, 30, text=etiquette, anchor="w", fill=C["etiquette"],
                      font="MemoireEtiquette")
        y = 50
        texte_l = largeur - 2 * 16 - 32
        police = "MemoireMonoPetite" if self.sujet == MODE_EXPERT else "TkDefaultFont"
        for rang, titre in enumerate(self.titres):
            courant = titre == self.choisi
            tag = f"action-{rang}"
            texte = c.create_text(32, 0, text=titre, anchor="nw", width=texte_l,
                                  fill=C["encre"], tags=(tag,),
                                  font="MemoireGras" if courant else police)
            _, haut, _, bas = c.bbox(texte)
            hauteur = (bas - haut) + 18
            c.coords(texte, 32, y + 9)
            image_l = largeur - 2 * 16
            if courant:
                p, image = self._images.role("choisi", (image_l, hauteur), lambda: dessin.forme(
                    image_l, hauteur, 15, rgba(C["survol"]), bord=rgba(C["encre"], .10),
                    ombre=OMBRE_COURTE), fond=C["papier"])
                fond = c.create_image(16 - image.x, y - image.y, image=p, anchor="nw", tags=(tag,))
                c.tag_lower(fond, texte)
            else:
                p, image = self._images.role(f"survol-{hauteur}", (image_l, hauteur),
                                             lambda: dessin.forme(image_l, hauteur, 15,
                                                                  rgba("#FFFFFF", .55)),
                                             fond=C["papier"])
                fond = c.create_image(16, y, image=p, anchor="nw", tags=(tag,), state="hidden")
                c.tag_lower(fond, texte)
                c.tag_bind(tag, "<Enter>", lambda _e, f=fond: (
                    c.itemconfigure(f, state="normal"), c.configure(cursor="hand2")))
                c.tag_bind(tag, "<Leave>", lambda _e, f=fond: (
                    c.itemconfigure(f, state="hidden"), c.configure(cursor="")))
            c.tag_bind(tag, "<Button-1>", lambda _e, r=rang: self.choisir(r))
            y += hauteur + 4
        c.configure(scrollregion=(0, 0, largeur, y + 20))

    def choisir(self, rang):
        genre, cible = self.cibles[rang]
        self.choisi = self.titres[rang]
        if genre == "commande":
            commande = next(c for c in self.app.commandes if c.nom == cible)
            self.dessiner()
            return self.app.choisir(commande)
        return self.app.ouvrir_action(cible)


class Recherche:
    """Recherche des actions et des notes (Ctrl K) : une carte posée sur la carte papier."""

    def __init__(self, app):
        self.app = app
        racine = app.racine
        self.resultats = []
        self._attente = None
        self.cadre = ttk.Frame(racine, style="Carte.TFrame", padding=(16, 14))
        haut = ttk.Frame(self.cadre)
        haut.pack(fill="x")
        ttk.Label(haut, text="Chercher une action ou une note", style="Question.TLabel").pack(
            side="left")
        ttk.Button(haut, text="Fermer · Échap", style="Bandeau.TButton",
                   command=self.fermer).pack(side="right")
        self.texte = tk.StringVar()
        self.champ = ttk.Entry(self.cadre, textvariable=self.texte, font="MemoireSousTitre")
        self.champ.pack(fill="x", pady=(10, 8))
        self.liste = tk.Listbox(self.cadre, height=9, activestyle="none", exportselection=False,
                                font="TkDefaultFont")
        self.liste.pack(fill="both", expand=True)
        ttk.Label(self.cadre, text="↑ ↓ choisir · Entrée ouvrir · une action s’ouvre, une note "
                                   "s’ouvre dans le visionneur", style="Second.TLabel",
                  font="MemoirePetite").pack(anchor="w", pady=(8, 0))
        self.champ.bind("<KeyRelease>", self._frappe)
        self.champ.bind("<Down>", lambda _e: self._deplacer(1))
        self.champ.bind("<Up>", lambda _e: self._deplacer(-1))
        self.champ.bind("<Return>", lambda _e: self._valider())
        self.champ.bind("<Escape>", lambda _e: self.fermer())
        self.liste.bind("<Escape>", lambda _e: self.fermer())
        self.liste.bind("<Return>", lambda _e: self._valider())
        self.liste.bind("<Double-Button-1>", lambda _e: self._valider())
        self.liste.bind("<ButtonRelease-1>", lambda e: self.choisir(self.liste.nearest(e.y))
                        if self.resultats else None)
        for sequence in ("<Control-k>", "<Control-K>"):
            racine.bind(sequence, lambda _e: self.ouvrir() or "break", add="+")

    def visible(self):
        return bool(self.cadre.winfo_manager())

    def ouvrir(self):
        g = self.app.habillage.geometrie.get("carte")
        if g is None:
            self.app.habillage.dessiner()
            g = self.app.habillage.geometrie["carte"]
        x, y, l, _h = g
        largeur = min(660, l - 60)
        self.cadre.place(x=x + (l - largeur) // 2, y=y + 18, width=largeur)
        self.cadre.lift()
        self.champ.focus_set()
        self.champ.select_range(0, "end")
        self.chercher(self.texte.get())

    def fermer(self):
        if self._attente is not None:
            self.cadre.after_cancel(self._attente)
            self._attente = None
        self.cadre.place_forget()

    def _frappe(self, evenement):
        if evenement.keysym in ("Up", "Down", "Return", "Escape"):
            return
        if self._attente is not None:
            self.cadre.after_cancel(self._attente)
        self._attente = self.cadre.after(DELAI_DE_FRAPPE_MS, lambda: self.chercher(self.texte.get()))

    def _notes(self, texte):
        resultat = subprocess.run([self.app.memoire, "--base", self.app.base.get(), "chercher",
                                   texte], capture_output=True, text=True, encoding="utf-8")
        trouvees = []
        for ligne in resultat.stdout.splitlines() if resultat.returncode == 0 else []:
            identifiant, role, titre = (ligne.split("\t", 2) + ["", ""])[:3]
            trouvees.append((titre, mot("role", role), identifiant))
        return trouvees

    def chercher(self, texte):
        """Actions dont le titre contient `texte`, puis notes trouvées par `memoire chercher`."""
        self._attente = None
        if self.texte.get() != texte:
            self.texte.set(texte)
        self.resultats = [("action", a.titre, a.cle)
                          for a in apparence.actions_trouvees(texte, ACTIONS)]
        if len(texte.strip()) >= LETTRES_MINIMALES:
            self.resultats += [("note", f"{titre} — {role} · {identifiant[:8]}", identifiant)
                               for titre, role, identifiant in self._notes(texte.strip())]
        self.liste.delete(0, "end")
        sujets = {a.cle: a.groupe for a in ACTIONS}
        for genre, libelle, cible in self.resultats:
            self.liste.insert("end", f"  {libelle}    ·  {sujets[cible]}" if genre == "action"
                              else f"  ◆ {libelle}")
        if self.resultats:
            self.liste.selection_set(0)
        return self.resultats

    def _deplacer(self, pas):
        if not self.resultats:
            return "break"
        choix = self.liste.curselection()
        rang = max(0, min((choix[0] if choix else -1) + pas, len(self.resultats) - 1))
        self.liste.selection_clear(0, "end")
        self.liste.selection_set(rang)
        self.liste.see(rang)
        return "break"

    def _valider(self):
        if not self.resultats:
            return "break"
        choix = self.liste.curselection()
        self.choisir(choix[0] if choix else 0)
        return "break"

    def choisir(self, rang):
        """Ouvre le résultat `rang` : l'action, ou la note dans le visionneur ; ferme la carte."""
        genre, _libelle, cible = self.resultats[rang]
        self.fermer()
        if genre == "action":
            return self.app.ouvrir_action(cible)
        return self.app.voir(cible)
