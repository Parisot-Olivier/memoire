"""La console : rangée, mosaïque de tuiles, poignées et point milieu (CONSOLE.md ; D32 ;
INV-TERM-04).

Contenu du panneau « console » du plan de travail. Réalisation Tkinter du modèle
(gui/mosaique.py), qui reste la seule source de vérité : chaque geste le modifie par la gestion
des terminaux, puis `disposer` met l'écran en accord. Une tuile est un cadre enfant de la zone,
placé au pixel ou retiré de l'écran, jamais recréé : son shell survit à tout changement de place.

Une tuile est un cadre classique au fond du terminal (CONSOLE.md, section 11) : le serveur peint
ce fond dès qu'une partie de la tuile est découverte, avant que Tk redessine, et rien n'est à
étirer. Ses coins arrondis sont quatre petites images, son liseré est sa bordure.

Un widget Tk est un rectangle opaque : l'image du point milieu reproduit donc ce qu'il recouvre
(le papier et les coins des tuiles voisines, recopiés de la même forme), puis porte la pastille.
"""
import tkinter as tk
from tkinter import ttk

import dessin
import theme as theme_
from apparence import COULEURS as C
from dessin import rgba
from mosaique import ECART, capacite, en_pixels
from terminal import PanneauTerminal
from terminaux import nom_court
from theme import Flux

BOITE_DU_POINT = 44
POIGNEE = (4, 36)          # épaisseur et longueur de la gélule d'une séparation
PRISE_DE_POIGNEE = 48      # longueur de la zone où on la prend
TUILE_MIN_POUR_POIGNEE = 90
RAYON_DE_TUILE = 18
LISERE = 2                 # bordure d'une tuile, en pixels
NOM_DE_PASTILLE = (16, 26)  # caractères : sur le côté, en large
ZONE_MIN = (240, 200)      # place demandée pour la mosaïque, quelle que soit la rangée


class Console:
    """Rangée de la console et mosaïque ; `gestion` tient le registre et le modèle."""

    def __init__(self, cadre, app, gestion):
        self.app, self.gestion, self.cadre = app, gestion, cadre
        self.theme = theme_.theme_de(cadre)
        papier = C["papier"]
        cadre.configure(padding=(14, 10, 14, 12))
        self.rangee = ttk.Frame(cadre)
        self.rangee.pack(fill="x", pady=(0, 8))
        self.zone = tk.Frame(cadre, background=papier, borderwidth=0, highlightthickness=0,
                             width=ZONE_MIN[0], height=ZONE_MIN[1])
        self.zone.pack(fill="both", expand=True)
        self._images = {}
        # Liseré d'une tuile : à peine visible, lavande pour la tuile active.
        self._liseres = {False: self.theme.melange(C["terminal"], "#FFFFFF", .10),
                         True: C["lavande"]}
        self.tuiles, self.terminaux, self._coins, self._liserees = {}, {}, {}, {}
        for nom in gestion.registre.noms_internes():
            self.ajouter(nom, gestion.registre.dossier(nom))
        self._montres = None
        # Nombre de fois où la rangée a été relue (essai : jamais pendant un glisser).
        self.lectures_de_la_rangee = 0
        self._structure = None
        self._capacite = None
        self._prise = None
        self._signature = None
        self._elements, self._pastilles = [], {}
        # Page sans terminal : le dire, et offrir d'en ouvrir un.
        self.vide = ttk.Frame(self.zone)
        ttk.Label(self.vide, text="Aucun terminal sur la page.", style="Second.TLabel").pack(
            pady=(0, 12))
        self.bouton_de_la_page_vide = ttk.Button(self.vide, text="Nouveau terminal",
                                                 style="Action.TButton", command=gestion.nouveau)
        self.bouton_de_la_page_vide.pack()
        self.seuil_saisi = tk.StringVar(value=self._seuil_lisible())
        self.poignees = [self._creer_une_poignee() for _ in range(4)]
        self.point = tk.Label(self.zone, borderwidth=0, highlightthickness=0, background=papier,
                              cursor="fleur", takefocus=False)
        self._survol_du_point = False
        self._cle_du_point = None
        self._gestes(self.point, lambda _e: "xy")
        self.point.bind("<Enter>", lambda _e: self._survoler_le_point(True), add="+")
        self.point.bind("<Leave>", lambda _e: self._survoler_le_point(False), add="+")
        # Dans le papier entre deux tuiles : la séparation se prend sur toute sa longueur.
        self._gestes(self.zone, self._prise_dans_la_zone)
        self.zone.bind("<Motion>", self._curseur_de_la_zone, add="+")
        self.zone.bind("<Configure>", lambda _e: self.disposer(), add="+")

    # ----- Tuiles -----

    def ajouter(self, nom, dossier):
        """Crée la tuile d'un terminal (hors de l'écran tant que `disposer` ne la place pas)."""
        nuit, lisere = C["terminal"], self._liseres[False]
        tuile = tk.Frame(self.zone, background=nuit, borderwidth=0, highlightthickness=LISERE,
                         highlightbackground=lisere, highlightcolor=lisere)
        self.tuiles[nom] = tuile
        self.terminaux[nom] = PanneauTerminal(tuile, self.app, nom, dossier)
        # Les quatre coins arrondis, par-dessus la bordure ; leur fond de fenêtre est celui du
        # terminal, comme tout ce que porte la tuile.
        self._coins[nom] = []
        for coin, (relx, rely, ancre) in enumerate(((0, 0, "nw"), (1, 0, "ne"), (1, 1, "se"),
                                                    (0, 1, "sw"))):
            image = tk.Label(tuile, image=self._image_de_coin(False, coin), background=nuit,
                             borderwidth=0, highlightthickness=0, takefocus=False)
            image.place(relx=relx, rely=rely, anchor=ancre, bordermode="outside")
            self._coins[nom].append(image)
        self._liserees[nom] = False
        return self.terminaux[nom]

    def _liserer(self, nom, active):
        """Liseré lavande de la tuile active : sa bordure et ses quatre coins."""
        if self._liserees[nom] == active:
            return
        self._liserees[nom] = active
        couleur = self._liseres[active]
        self.tuiles[nom].configure(highlightbackground=couleur, highlightcolor=couleur)
        for coin, image in enumerate(self._coins[nom]):
            image.configure(image=self._image_de_coin(active, coin))

    def retirer(self, nom):
        """Détruit la tuile d'un terminal fermé : son shell s'arrête à sa destruction."""
        self.terminaux.pop(nom)
        self._coins.pop(nom)
        self._liserees.pop(nom)
        self.tuiles.pop(nom).destroy()

    def tuile(self, nom):
        return self.tuiles[nom]

    def capacite(self):
        """Nombre de terminaux que la place permet de montrer ; avant le premier affichage : un
        seul sur le côté, la mosaïque en pleine page ou en fenêtre."""
        if self._capacite is not None:
            return self._capacite
        return 1 if self.app.plan.disposition.etat("console").etat in ("ancre", "masque") else 4

    def quand_visible(self):
        self.disposer()

    # ----- Mise en accord de l'écran -----

    def disposer(self, glisser=False):
        """Place les tuiles montrées, les poignées et le point milieu d'après le modèle.
        `glisser` : pendant qu'on tire une séparation, seule la mosaïque bouge ; la rangée et les
        shells à lancer attendent le lâcher."""
        largeur, hauteur = self.zone.winfo_width(), self.zone.winfo_height()
        if largeur > 1 and hauteur > 1:
            self._capacite = capacite(largeur, hauteur)
            self._placer(largeur, hauteur, glisser)
        if not glisser:
            self.actualiser_la_rangee()

    def _placer(self, largeur, hauteur, glisser):
        m, cap = self.gestion.mosaique, self._capacite
        places = dict(en_pixels(m.rectangles(cap), largeur, hauteur))
        for nom, tuile in self.tuiles.items():
            if nom not in places:
                tuile.place_forget()
                continue
            x, y, l, h = places[nom]
            tuile.place(x=x, y=y, width=l, height=h)
            self._liserer(nom, nom == m.actif and len(places) > 1)
            self.terminaux[nom].montrer_agrandir(len(places) > 1, m.agrandi == nom)
        # Poignées et point au-dessus des tuiles : à refaire quand les tuiles montrées changent
        # (une tuile neuve est créée au-dessus de tout).
        dessus = list(places) != self._montres
        self._montres = list(places)
        if places:
            self.vide.place_forget()
        else:
            self.vide.place(relx=0.5, rely=0.5, anchor="center")
        self._placer_les_poignees(m.poignees(cap), places, largeur, hauteur, dessus)
        if m.point(cap) is None:
            self.point.place_forget()
        else:
            montres = list(places)
            cle = (len(montres), montres.index(m.actif) if m.actif in montres else None,
                   self._survol_du_point)
            if cle != self._cle_du_point:
                self._cle_du_point = cle
                self.point.configure(image=self._image_du_point(*cle))
            demi = BOITE_DU_POINT // 2
            self.point.place(x=round(m.x * largeur) - demi, y=round(m.y * hauteur) - demi,
                             width=BOITE_DU_POINT, height=BOITE_DU_POINT)
            if dessus:
                self.point.lift()
        if not glisser and self.app.plan.visible("console"):
            for nom in places:
                self.terminaux[nom].quand_visible()

    def _placer_les_poignees(self, positions, places, largeur, hauteur, dessus):
        petite = min((min(l, h) for _x, _y, l, h in places.values()), default=0)
        for rang, poignee in enumerate(self.poignees):
            if rang >= len(positions) or petite < TUILE_MIN_POUR_POIGNEE:
                poignee.place_forget()
                continue
            axe, fx, fy = positions[rang]
            if poignee.axe != axe:
                poignee.axe = axe
                poignee.configure(image=self._image_de_poignee(axe, False),
                                  cursor="sb_h_double_arrow" if axe == "x" else "sb_v_double_arrow")
            cx, cy = round(fx * largeur), round(fy * hauteur)
            l, h = (ECART, PRISE_DE_POIGNEE) if axe == "x" else (PRISE_DE_POIGNEE, ECART)
            poignee.place(x=cx - l // 2, y=cy - h // 2, width=l, height=h)
            if dessus:
                poignee.lift()

    # ----- Glisser : séparations et point milieu -----

    def _creer_une_poignee(self):
        poignee = tk.Label(self.zone, borderwidth=0, highlightthickness=0,
                           background=C["papier"], takefocus=False)
        poignee.axe = None
        self._gestes(poignee, lambda _e, p=poignee: p.axe)
        poignee.bind("<Enter>", lambda _e, p=poignee: p.configure(
            image=self._image_de_poignee(p.axe, True)), add="+")
        poignee.bind("<Leave>", lambda _e, p=poignee: p.configure(
            image=self._image_de_poignee(p.axe, False)), add="+")
        return poignee

    def _gestes(self, widget, axe_de):
        """Prendre, tirer, lâcher ; double-clic : parts égales. `axe_de(evenement)` dit ce qu'on
        prend (« x », « y », « xy » ou None)."""
        widget.bind("<ButtonPress-1>", lambda e: self._prendre(axe_de(e)), add="+")
        widget.bind("<B1-Motion>", self._tirer, add="+")
        widget.bind("<ButtonRelease-1>", self._lacher, add="+")
        widget.bind("<Double-Button-1>", lambda e: self._egaliser(axe_de(e)), add="+")

    def _prise_dans_la_zone(self, evenement):
        return self.gestion.mosaique.prise(evenement.x, evenement.y, self.zone.winfo_width(),
                                           self.zone.winfo_height(), self.capacite())

    def _curseur_de_la_zone(self, evenement):
        axe = self._prise or self._prise_dans_la_zone(evenement)
        curseur = {"x": "sb_h_double_arrow", "y": "sb_v_double_arrow", "xy": "fleur"}.get(axe, "")
        if str(self.zone.cget("cursor")) != curseur:
            self.zone.configure(cursor=curseur)

    def _prendre(self, axe):
        self._prise = axe

    def _tirer(self, evenement):
        if self._prise is None:
            return
        largeur, hauteur = self.zone.winfo_width(), self.zone.winfo_height()
        if largeur <= 1 or hauteur <= 1:
            return
        x = (evenement.x_root - self.zone.winfo_rootx()) / largeur
        y = (evenement.y_root - self.zone.winfo_rooty()) / hauteur
        self.gestion.deplacer_le_point(x if "x" in self._prise else None,
                                       y if "y" in self._prise else None, glisser=True)

    def _lacher(self, _evenement):
        if self._prise is not None:
            self._prise = None
            # Le geste est fini : la rangée, les shells à lancer et la conservation suivent.
            self.disposer()
            self.gestion.enregistrer()

    def _egaliser(self, axe):
        if axe is not None:
            self._prise = None
            self.gestion.egaliser()

    def _survoler_le_point(self, dessus):
        if dessus != self._survol_du_point:
            self._survol_du_point = dessus
            self.disposer()

    # ----- Images -----

    def _image(self, cle, fabrique):
        if cle not in self._images:
            self._images[cle] = fabrique()
        return self._images[cle]

    def _image_de_poignee(self, axe, survol):
        l, h = POIGNEE if axe == "x" else POIGNEE[::-1]
        couleur = rgba(C["indigo"]) if survol else rgba(C["encre"], .30)
        return self._image(("poignee", axe, survol), lambda: theme_.photo(
            self.zone, dessin.forme(l, h, 2, couleur), fond=C["papier"]))

    def _forme_de_tuile(self, active):
        """Une tuile entière, petite : fond du terminal, liseré, coins arrondis, sur le papier.
        Les coins des tuiles et l'image du point milieu en sont des morceaux."""
        return self._image(("tuile", active), lambda: theme_.photo(self.zone, dessin.forme(
            64, 64, RAYON_DE_TUILE, rgba(C["terminal"]), bord=rgba(self._liseres[active]),
            epaisseur=LISERE), fond=C["papier"]))

    def _image_de_coin(self, active, coin):
        """Coin arrondi d'une tuile : 0 haut gauche, puis dans le sens des aiguilles."""
        def decouper():
            source, r = self._forme_de_tuile(active), RAYON_DE_TUILE
            cote = source.width()
            x, y = ((0, 0), (cote - r, 0), (cote - r, cote - r), (0, cote - r))[coin]
            image = tk.PhotoImage(master=self.zone, width=r, height=r)
            image.tk.call(image, "copy", source, "-from", x, y, x + r, y + r)
            return image
        return self._image(("coin", active, coin), decouper)

    def _image_du_point(self, nombre, actif, survol):
        """Le point milieu sur ce qu'il recouvre : le papier, puis le coin (ou le bord) de chaque
        tuile voisine, recopié de l'image du thème ; `actif` : rang de la tuile au liseré."""
        return self._image(("point", nombre, actif, survol),
                           lambda: self._composer_le_point(nombre, actif, survol))

    def _composer_le_point(self, nombre, actif, survol):
        boite, centre, demi_ecart = BOITE_DU_POINT, BOITE_DU_POINT // 2, ECART // 2
        coin = centre - demi_ecart
        image = tk.PhotoImage(master=self.zone, width=boite, height=boite)
        image.put(C["papier"], to=(0, 0, boite, boite))

        def source(rang):
            return self._forme_de_tuile(rang == actif)

        def copier(rang, de, vers):
            image.tk.call(image, "copy", source(rang), "-from", *de, "-to", *vers)

        cote = source(0).width()
        loin, pres = cote - coin, centre + demi_ecart
        if nombre == 4:
            copier(0, (loin, loin, cote, cote), (0, 0))
            copier(3, (loin, 0, cote, coin), (0, pres))
        else:
            # Trois tuiles : à gauche, le bord droit d'une seule tuile, sur toute la hauteur.
            copier(0, (loin, cote // 2, cote, cote // 2 + 1), (0, 0, coin, boite))
        copier(1, (0, loin, coin, cote), (pres, 0))
        copier(2, (0, 0, coin, coin), (pres, pres))

        def poser(forme_, x, y):
            photo = theme_.photo(self.zone, forme_)
            image.tk.call(image, "copy", photo, "-to", x - forme_.x, y - forme_.y,
                          "-compositingrule", "overlay")

        if survol:
            poser(dessin.forme(32, 32, 16, rgba(C["indigo"], .28)), centre - 16, centre - 16)
        poser(dessin.forme(26, 26, 13, rgba("#FFFFFF"), bord=rgba(C["encre"], .10),
                           ombre=(3, 4, 4, rgba(C["encre"], .34))), centre - 13, centre - 13)
        poser(dessin.forme(8, 8, 4, rgba(C["indigo"])), centre - 4, centre - 4)
        return image

    # ----- Rangée de la console -----

    def _seuil_lisible(self):
        seuil = self.gestion.seuil
        return str(int(seuil)) if float(seuil).is_integer() else str(seuil)

    def seuil_change(self):
        self.seuil_saisi.set(self._seuil_lisible())

    def _attente(self):
        """(nom interne, nom, genre) des terminaux qu'on ne voit pas."""
        en_attente = self.gestion.en_attente()
        par_nom = {nom: (titre, genre) for nom, titre, _e, genre in self.gestion.etats_courts()}
        return [(nom, *par_nom[nom]) for nom in en_attente]

    def actualiser_la_rangee(self):
        """Met la rangée en accord avec la console. Elle n'est reconstruite que si sa structure
        change (place de la console, place disponible, liste des terminaux en attente) ; un nom ou
        un état qui change se corrige sur place, sans rien détruire."""
        self.lectures_de_la_rangee += 1
        etat = self.app.plan.disposition.etat("console").etat
        large = self.capacite() > 1
        attente = self._attente()
        signature = (etat, large, tuple(attente))
        if signature == self._signature:
            return
        self._signature = signature
        structure = (etat, large, tuple(nom for nom, _titre, _genre in attente))
        if structure != self._structure:
            self._structure = structure
            self._construire_la_rangee(etat, large, [nom for nom, _t, _g in attente])
        papier = C["papier"]
        couleurs = {"cours": (C["vert"], False), "echec": (C["vermillon"], False)}
        for nom, titre, genre in attente:
            couleur, anneau = couleurs.get(genre, (C["etiquette"], True))
            self._pastilles[nom].configure(
                text=nom_court(titre, NOM_DE_PASTILLE[large]),
                image=(self.theme.point(couleur, papier, anneau=anneau), "active",
                       self.theme.point(couleur, "#FFFFFF", anneau=anneau)))

    def _construire_la_rangee(self, etat, large, en_attente):
        for enfant in self.rangee.winfo_children():
            enfant.destroy()
        self._elements, self._pastilles = [], {}
        tete = ttk.Frame(self.rangee)
        tete.pack(fill="x")
        outils = ttk.Frame(tete)
        outils.pack(side="right", anchor="n")
        etiquette = ttk.Label(tete, text="CONSOLE", style="Etiquette.TLabel")
        etiquette.pack(side="left", anchor="n", padx=(2, 10), pady=(5, 0))
        self._elements.append(etiquette)
        # Large : tout sur une ligne. Étroite (sur le côté) : les terminaux sous le titre.
        flux = Flux(tete if large else self.rangee, ecart=6, interligne=6)
        if large:
            flux.pack(side="left", fill="x", expand=True)
        else:
            flux.pack(fill="x", pady=(6, 0))

        def bouton(parent, texte, action, style="Bandeau.TButton", **options):
            b = ttk.Button(parent, text=texte, style=style, command=action, takefocus=False,
                           **options)
            self._elements.append(b)
            return b

        def etiquette_(parent, texte, style="Second.TLabel", **options):
            e = ttk.Label(parent, text=texte, style=style, **options)
            self._elements.append(e)
            return e

        flux.ajouter(bouton(flux, "+ Nouveau terminal" if large else "+ Nouveau",
                            self.gestion.nouveau))
        if en_attente and large:
            flux.ajouter(etiquette_(flux, "EN ARRIÈRE-PLAN", "Etiquette.TLabel",
                                    padding=(8, 5, 0, 0)))
        for nom in en_attente:
            # Texte et point d'état : posés par `actualiser_la_rangee`, puis tenus à jour.
            pastille = bouton(flux, "", lambda n=nom: self.gestion.afficher(n),
                              style="Attente.TButton", compound="left")
            self._pastilles[nom] = pastille
            flux.ajouter(pastille)
        if large:
            # Un seul réglage pour la console : la valeur, et − / + par pas de cinq secondes.
            seuil = ttk.Frame(outils)
            seuil.pack(side="left", padx=(0, 12))
            etiquette_(seuil, "Notifier après", font="MemoirePetite").pack(side="left", padx=(0, 6))
            bouton(seuil, "−", lambda: self.gestion.changer_le_seuil(-5), width=2).pack(side="left")
            ttk.Label(seuil, textvariable=self.seuil_saisi, font="MemoireGras", width=4,
                      anchor="e").pack(side="left")
            etiquette_(seuil, "s", font="MemoirePetite").pack(side="left", padx=(3, 6))
            bouton(seuil, "+", lambda: self.gestion.changer_le_seuil(5), width=2).pack(side="left")
        if etat == "page":
            bouton(outils, "Réduire", self.gestion.reduire).pack(side="left", padx=(0, 6))
        else:
            bouton(outils, "Pleine page", self.gestion.pleine_page).pack(side="left", padx=(0, 6))
        # Sur le côté, la place est comptée : « Détacher » est alors dans le menu « ⋯ ».
        if etat == "flottant" and large:
            bouton(outils, "Ancrer", lambda: self.app.plan.ancrer("console")).pack(
                side="left", padx=(0, 6))
        elif large:
            bouton(outils, "Détacher", lambda: self.app.plan.detacher("console")).pack(
                side="left", padx=(0, 6))
        menu = bouton(outils, "⋯", lambda: self._ouvrir_le_menu(menu), width=2)
        menu.pack(side="left")

    def _ouvrir_le_menu(self, bouton):
        """Le menu ordinaire d'un panneau : ordre de la fenêtre, zone, masquer."""
        plan = self.app.plan
        plan._ouvrir_sous(bouton, plan.panneaux["console"].menu)

    def textes_de_la_rangee(self):
        """Textes de la rangée, dans l'ordre (étiquettes, boutons, pastilles)."""
        self.actualiser_la_rangee()
        return [str(element.cget("text")) for element in self._elements]

    def bouton(self, texte):
        """Le bouton de la rangée qui porte exactement ce texte."""
        self.actualiser_la_rangee()
        for element in self._elements:
            if isinstance(element, ttk.Button) and str(element.cget("text")) == texte:
                return element
        raise KeyError(texte)

    def pastille(self, nom):
        """La pastille d'un terminal en attente."""
        self.actualiser_la_rangee()
        return self._pastilles[nom]

    def actualiser(self, etats, battement, maintenant):
        """Mise à jour périodique : en-têtes des tuiles, pastilles, rangée."""
        for nom, titre, etat, _genre in etats:
            self.terminaux[nom].actualiser(titre, etat, battement, maintenant)
        self.actualiser_la_rangee()
