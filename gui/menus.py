"""Menus, listes déroulantes et suggestions dans une carte arrondie (APPARENCE.md, section 9.3 ;
8.2a ; D31).

Une `Carte` est posée dans la fenêtre (un canevas placé par-dessus) : son image de fond est le
morceau du fond de la fenêtre qu'elle couvre (ou le papier), sur lequel est composée une carte
arrondie à ombre courte ; l'image est opaque (dessin rapide, voir theme.aplatir). Elle ne peut pas
déborder de la fenêtre : si la place manque, elle s'ouvre au-dessus ou se décale.

- `Menu` offre ce que le code utilise de `tk.Menu` (entrées, coches, séparateurs, `tk_popup`).
- Les listes des `ttk.Combobox` passent par `ttk::combobox::Post`, remplacée ici (`installer`).
- Les suggestions de notes (gui/champs.py) utilisent une carte non modale : la frappe continue.
Aucune règle : seulement l'affichage.
"""
from dataclasses import dataclass
import tkinter as tk
import tkinter.font as tkfont

import dessin
from apparence import COULEURS as C
from dessin import rgba
from theme import forme_etiree, photo, theme_de

HAUTEUR_DE_LIGNE = 32
HAUTEUR_DE_SEPARATEUR = 11
LIGNES_VISIBLES = 10
MARGE = 6
FOND_DE_CARTE = "#FFFAF7"
# Carte sombre (possibilités du terminal) : fond, texte, texte second.
FOND_SOMBRE, TEXTE_SOMBRE, SECOND_SOMBRE = C["terminal_champ"], C["terminal_texte"], "#CFC7E6"
# Cartes ouvertes : une seule à la fois ; un clic ailleurs les ferme (voir `installer`).
_OUVERTES = []
OMBRE = (3, 6, 6, rgba(C["encre"], .18))


@dataclass
class Ligne:
    """Une ligne de carte : texte, puis au besoin texte second (gris), texte à droite (chasse
    fixe), point de couleur, coche ; `genre` : « commande », « case » ou « separateur »."""

    texte: str = ""
    genre: str = "commande"
    second: str = ""
    droite: str = ""
    point: str = None
    coche: bool = False
    actif: bool = True


def marges_de_l_ombre():
    """(gauche, haut, droite, bas) : ce que l'ombre ajoute autour d'une carte."""
    image = dessin.forme(40, 40, 14, rgba(FOND_DE_CARTE), ombre=OMBRE)
    return (image.x, image.y, image.largeur - image.x - 40, image.hauteur - image.y - 40)


def fond_de_la_fenetre(fenetre):
    """Image du fond d'une fenêtre (celle de la fenêtre principale), ou None."""
    return getattr(fenetre, "_fond_memoire", None)


class Carte:
    """Carte flottante de lignes ; `quand_choisi(rang)` reçoit la ligne choisie."""

    def __init__(self, maitre, modale=True, sombre=False):
        self.maitre = maitre
        self.modale = modale
        self.sombre = sombre
        self._vient_d_ouvrir = False
        self.canevas = None
        self.image_de_fond = None
        self.lignes = []
        self.rang = None
        self.debut = 0
        self.quand_choisi = None
        self._focus_avant = None
        self._images = {}

    # ----- Lecture -----

    def visible(self):
        return self.canevas is not None and bool(self.canevas.winfo_manager())

    def libelles(self):
        return [ligne.texte for ligne in self.lignes if ligne.genre != "separateur"]

    def _choisissables(self):
        return [r for r, ligne in enumerate(self.lignes)
                if ligne.genre != "separateur" and ligne.actif]

    # ----- Ouverture -----

    def montrer(self, lignes, x_ecran, y_ecran, largeur_min=0, quand_choisi=None, rang=None,
                au_dessus=None):
        """Ouvre la carte au point (x_ecran, y_ecran) de l'écran ; `au_dessus` : ordonnée écran
        où la poser au-dessus si elle ne tient pas dessous (bord haut d'un champ)."""
        # Une seule carte ouverte à la fois : celle-ci ferme les autres.
        for autre in list(_OUVERTES):
            if autre is not self:
                autre.fermer()
        if self not in _OUVERTES:
            _OUVERTES.append(self)
        self.lignes = list(lignes)
        self.quand_choisi = quand_choisi
        self.debut = 0
        fenetre = self.maitre.winfo_toplevel()
        if self.canevas is None or self.canevas.winfo_toplevel() is not fenetre:
            self._creer(fenetre)
        police = tkfont.nametofont("TkDefaultFont", root=self.maitre)
        mono = tkfont.nametofont("MemoireMonoPetite", root=self.maitre)
        avec_marque = any(l.genre == "case" or l.point or l.coche for l in self.lignes)
        self._decalage_texte = 14 + (22 if avec_marque else 0)
        largeur = largeur_min
        for ligne in self.lignes:
            if ligne.genre == "separateur":
                continue
            l = self._decalage_texte + police.measure(ligne.texte) + 18
            if ligne.second:
                l += police.measure(" — " + ligne.second)
            if ligne.droite:
                l += mono.measure(ligne.droite) + 24
            largeur = max(largeur, l + 2 * MARGE)
        fenetre.update_idletasks()
        largeur = min(largeur, max(160, fenetre.winfo_width() - 24))
        visibles = self.lignes[:LIGNES_VISIBLES] if len(self.lignes) > LIGNES_VISIBLES else self.lignes
        hauteur = 2 * MARGE + sum(HAUTEUR_DE_SEPARATEUR if l.genre == "separateur"
                                  else HAUTEUR_DE_LIGNE for l in visibles)
        self.largeur, self.hauteur = largeur, hauteur
        # Position dans la fenêtre, sans déborder, ombre comprise.
        gauche, haut_, droite, bas = (0, 0, 0, 0) if self.sombre else marges_de_l_ombre()
        fx, fy = fenetre.winfo_rootx(), fenetre.winfo_rooty()
        fl, fh = fenetre.winfo_width(), fenetre.winfo_height()
        x = min(max(gauche + 2, x_ecran - fx), fl - largeur - droite - 2)
        y = y_ecran - fy
        if y + hauteur + bas > fh - 2:
            haut = (au_dessus - fy if au_dessus is not None else y) - hauteur - 4
            y = haut if haut >= haut_ + 2 else max(haut_ + 2, fh - hauteur - bas - 2)
        # Une carte sombre (possibilités du terminal) s'ouvre de préférence au-dessus de la saisie,
        # dans le terminal : ses coins et son ombre restent sur le fond sombre.
        if self.sombre and au_dessus is not None and au_dessus - fy - hauteur - 4 >= haut_ + 2:
            y = au_dessus - fy - hauteur - 4
        y = max(haut_ + 2, min(y, fh - hauteur - bas - 2))
        self._fond(fenetre, x, y, largeur, hauteur)
        self.rang = rang if rang is not None else (None if not self.modale else
                                                   next(iter(self._choisissables()), None))
        if self.rang is not None and self.rang >= LIGNES_VISIBLES:
            self.debut = self.rang - LIGNES_VISIBLES + 1
        self.dessiner()
        c = self.canevas
        c.place(x=x - self._marges[0], y=y - self._marges[1])
        c.tk.call("raise", c._w)  # Canvas.lift lève un élément, pas le widget
        # Pas de capture des clics : un clic ailleurs ferme la carte et agit normalement
        # (`installer`). Le clic qui vient de l'ouvrir ne la referme pas.
        self._vient_d_ouvrir = True
        c.after_idle(self._ouverte)
        if self.modale:
            self._focus_avant = fenetre.focus_get()
            c.focus_set()

    def _ouverte(self):
        self._vient_d_ouvrir = False

    def _creer(self, fenetre):
        if self.canevas is not None:
            self.canevas.destroy()
        c = tk.Canvas(fenetre, highlightthickness=0, borderwidth=0, background=C["papier"])
        self.canevas = c
        c.bind("<Motion>", self._mouvement)
        c.bind("<ButtonPress-1>", self._clic)
        c.bind("<Up>", lambda _e: self.deplacer(-1) or "break")
        c.bind("<Down>", lambda _e: self.deplacer(1) or "break")
        c.bind("<Return>", lambda _e: self.valider() or "break")
        c.bind("<Escape>", lambda _e: self.fermer() or "break")
        c.bind("<Button-4>", lambda _e: self._defiler(-1))
        c.bind("<Button-5>", lambda _e: self._defiler(1))
        c.bind("<MouseWheel>", lambda e: self._defiler(-1 if e.delta > 0 else 1))

    def _fond(self, fenetre, x, y, largeur, hauteur):
        """Image opaque : le fond de la fenêtre sous la carte (ou le papier), puis la carte."""
        if self.sombre:
            # Coins presque droits, sans ombre : la carte peut chevaucher le terminal et le
            # papier, aucune couleur de coin ne conviendrait aux deux.
            p, image = forme_etiree(self.maitre, largeur, hauteur, 3, fond=rgba(FOND_SOMBRE),
                                    bord=rgba("#FFFFFF", .28))
        else:
            p, image = forme_etiree(self.maitre, largeur, hauteur, 14, fond=rgba(FOND_DE_CARTE),
                                    bord=rgba(C["encre"], .10), ombre=OMBRE)
        self._marges = (image.x, image.y)
        il, ih = image.largeur, image.hauteur
        fond = tk.PhotoImage(master=self.maitre, width=il, height=ih)
        # Une carte sombre s'ouvre sur le terminal : ses coins prennent sa couleur.
        fond.put(FOND_SOMBRE if self.sombre else C["papier"], to=(0, 0, il, ih))
        source = None if self.sombre else fond_de_la_fenetre(fenetre)
        x0, y0 = x - image.x, y - image.y
        if source is not None:
            fx0, fy0 = max(0, x0), max(0, y0)
            fx1, fy1 = min(source.width(), x0 + il), min(source.height(), y0 + ih)
            if fx1 > fx0 and fy1 > fy0:
                fond.tk.call(fond, "copy", source, "-from", fx0, fy0, fx1, fy1,
                             "-to", fx0 - x0, fy0 - y0, "-compositingrule", "set")
        fond.tk.call(fond, "copy", p, "-compositingrule", "overlay")
        self.image_de_fond = fond
        self.canevas.configure(width=il, height=ih)

    def _image(self, cle, fabrique):
        if cle not in self._images:
            self._images[cle] = fabrique()
        return self._images[cle]

    # ----- Dessin -----

    def dessiner(self):
        c = self.canevas
        c.delete("all")
        c.create_image(0, 0, image=self.image_de_fond, anchor="nw")
        mx, my = self._marges
        y = my + MARGE
        theme = theme_de(self.maitre)
        survol = self._image(("survol", self.largeur), lambda: photo(
            self.maitre, dessin.forme(self.largeur - 2 * MARGE, HAUTEUR_DE_LIGNE - 4, 11,
                                      rgba("#FFFFFF", .16) if self.sombre
                                      else rgba(C["indigo"], .10)),
            FOND_SOMBRE if self.sombre else FOND_DE_CARTE))
        encre = TEXTE_SOMBRE if self.sombre else C["encre"]
        second = SECOND_SOMBRE if self.sombre else C["texte_second"]
        self._positions = {}
        for rang in range(self.debut, len(self.lignes)):
            ligne = self.lignes[rang]
            h = HAUTEUR_DE_SEPARATEUR if ligne.genre == "separateur" else HAUTEUR_DE_LIGNE
            if y + h > my + self.hauteur - MARGE + 1:
                break
            if ligne.genre == "separateur":
                c.create_line(mx + 14, y + h / 2, mx + self.largeur - 14, y + h / 2,
                              fill=C["separation"])
            else:
                self._positions[rang] = (y, y + h)
                if rang == self.rang and ligne.actif:
                    c.create_image(mx + MARGE, y + 2, image=survol, anchor="nw")
                milieu = y + h / 2
                couleur = encre if ligne.actif else "#9A8E9C"
                if ligne.coche:
                    c.create_image(mx + 16, milieu, anchor="w",
                                   image=theme.icone("coche", 14, C["indigo"]))
                elif ligne.point:
                    point = self._image(("point", ligne.point), lambda co=ligne.point: photo(
                        self.maitre, dessin.forme(8, 8, 4, rgba(co))))
                    c.create_image(mx + 19, milieu, image=point, anchor="w")
                texte = c.create_text(mx + self._decalage_texte, milieu, text=ligne.texte,
                                      anchor="w", fill=couleur, font="TkDefaultFont")
                if ligne.second:
                    _, _, fin, _ = c.bbox(texte)
                    c.create_text(fin, milieu, text=f" — {ligne.second}", anchor="w",
                                  fill=second, font="TkDefaultFont")
                if ligne.droite:
                    c.create_text(mx + self.largeur - 16, milieu, text=ligne.droite, anchor="e",
                                  fill=second, font="MemoireMonoPetite")
            y += h
        if len(self.lignes) - self.debut > len(self._positions) + self._separateurs_vus():
            c.create_text(mx + self.largeur - 12, my + self.hauteur - 4, text="⌄", anchor="se",
                          fill=C["texte_second"])

    def _separateurs_vus(self):
        return sum(1 for r in range(self.debut, len(self.lignes))
                   if self.lignes[r].genre == "separateur")

    # ----- Gestes -----

    def _rang_en(self, y):
        for rang, (haut, bas) in self._positions.items():
            if haut <= y < bas:
                return rang
        return None

    def _mouvement(self, evenement):
        rang = self._rang_en(evenement.y)
        if rang is not None and self.lignes[rang].actif and rang != self.rang:
            self.rang = rang
            self.dessiner()

    def _clic(self, evenement):
        dedans = (0 <= evenement.x < self.canevas.winfo_width()
                  and 0 <= evenement.y < self.canevas.winfo_height())
        rang = self._rang_en(evenement.y) if dedans else None
        if rang is not None:
            self._choisir_ligne(rang)
        elif not dedans or self.modale:
            if not dedans:
                self.fermer()
        return "break"

    def _defiler(self, pas):
        if len(self.lignes) <= LIGNES_VISIBLES:
            return
        self.debut = max(0, min(len(self.lignes) - LIGNES_VISIBLES, self.debut + pas))
        self.dessiner()

    def deplacer(self, pas):
        """Ligne choisissable suivante (pas = 1) ou précédente (pas = -1)."""
        rangs = self._choisissables()
        if not rangs or not self.visible():
            return None
        if self.rang not in rangs:
            self.rang = rangs[0] if pas > 0 else rangs[-1]
        else:
            self.rang = rangs[max(0, min(len(rangs) - 1, rangs.index(self.rang) + pas))]
        if self.rang < self.debut:
            self.debut = self.rang
        elif self.rang >= self.debut + LIGNES_VISIBLES:
            self.debut = self.rang - LIGNES_VISIBLES + 1
        self.dessiner()
        return "break"

    def valider(self):
        if self.rang is None or not self.visible():
            return None
        return self._choisir_ligne(self.rang)

    def choisir(self, rang):
        """Choisit la `rang`-ième entrée (séparateurs non comptés, comme `libelles`) et ferme ;
        une entrée inactive ne se choisit pas."""
        entrees = [r for r, ligne in enumerate(self.lignes) if ligne.genre != "separateur"]
        if not 0 <= rang < len(entrees):
            return None
        return self._choisir_ligne(entrees[rang])

    def _choisir_ligne(self, rang):
        """Choisit la ligne `rang` (séparateurs comptés) ; `quand_choisi` reçoit ce rang."""
        if not 0 <= rang < len(self.lignes):
            return None
        ligne = self.lignes[rang]
        if ligne.genre == "separateur" or not ligne.actif:
            return None
        self.fermer()
        if self.quand_choisi is not None:
            return self.quand_choisi(rang)
        return None

    def detruire(self):
        """Ferme la carte et détruit son canevas (le champ qui la portait disparaît)."""
        self.fermer()
        if self.canevas is not None:
            try:
                self.canevas.destroy()
            except tk.TclError:
                pass
            self.canevas = None

    def fermer(self):
        if self in _OUVERTES:
            _OUVERTES.remove(self)
        if self.canevas is None:
            return
        try:
            self.canevas.place_forget()
        except tk.TclError:
            return
        if self.modale and self._focus_avant is not None:
            try:
                self._focus_avant.focus_set()
            except (tk.TclError, KeyError):
                pass
        self._focus_avant = None


class Menu:
    """Remplaçant de `tk.Menu` pour ce que l'interface en utilise : entrées, coches, séparateurs,
    lecture des entrées (essais), `tk_popup`. `postcommand` est appelé avant chaque ouverture."""

    def __init__(self, maitre=None, tearoff=False, postcommand=None, **_options):
        self.maitre = maitre
        self.postcommand = postcommand
        self.entrees = []
        self.liste = Carte(maitre)

    def add_command(self, label="", command=None, accelerator="", state="normal"):
        self.entrees.append({"type": "command", "label": label, "command": command,
                             "accelerator": accelerator, "state": state})

    def add_checkbutton(self, label="", variable=None, command=None, state="normal"):
        self.entrees.append({"type": "checkbutton", "label": label, "command": command,
                             "variable": variable, "state": state, "accelerator": ""})

    def add_separator(self):
        self.entrees.append({"type": "separator", "label": "", "state": "normal"})

    def _rang(self, index):
        if index == "end" or index == "last":
            return len(self.entrees) - 1
        return int(index)

    def index(self, index):
        if not self.entrees:
            return None
        return self._rang(index)

    def delete(self, debut, fin=None):
        if not self.entrees:
            return
        a = self._rang(debut)
        b = self._rang(fin) if fin is not None else a
        del self.entrees[a:b + 1]

    def type(self, index):
        return self.entrees[self._rang(index)]["type"]

    def entrycget(self, index, option):
        return self.entrees[self._rang(index)].get(option, "")

    def entryconfigure(self, index, **options):
        self.entrees[self._rang(index)].update(options)

    entryconfig = entryconfigure

    def invoke(self, index):
        entree = self.entrees[self._rang(index)]
        if entree["type"] == "separator" or entree.get("state") == "disabled":
            return None
        if entree["type"] == "checkbutton" and entree.get("variable") is not None:
            entree["variable"].set(not entree["variable"].get())
        if entree.get("command"):
            return entree["command"]()
        return None

    def tk_popup(self, x, y, entry=None):
        if self.postcommand is not None:
            self.postcommand()
        lignes = []
        for entree in self.entrees:
            if entree["type"] == "separator":
                lignes.append(Ligne(genre="separateur"))
                continue
            coche = (entree["type"] == "checkbutton" and entree.get("variable") is not None
                     and bool(entree["variable"].get()))
            lignes.append(Ligne(entree["label"], genre="case" if entree["type"] == "checkbutton"
                                else "commande", droite=entree.get("accelerator", ""),
                                coche=coche, actif=entree.get("state") != "disabled"))
        self.liste.montrer(lignes, x, y, quand_choisi=self.invoke)

    post = tk_popup

    def unpost(self):
        self.liste.fermer()


def liste_de(combobox):
    """La carte qui sert de liste à une `ttk.Combobox`."""
    carte = getattr(combobox, "_carte_memoire", None)
    if carte is None:
        carte = Carte(combobox)
        combobox._carte_memoire = carte
    return carte


def _poster(racine, chemin):
    """Remplace `ttk::combobox::Post` : la liste s'ouvre dans une carte."""
    try:
        liste = racine.nametowidget(chemin)
    except KeyError:
        return
    if liste.instate(["disabled"]):
        return
    commande = str(liste.cget("postcommand"))
    if commande:
        racine.tk.eval(commande)
    valeurs = [str(v) for v in racine.tk.splitlist(liste.cget("values"))]
    if not valeurs:
        return
    courant = liste.get()
    lignes = [Ligne(v, coche=(v == courant)) for v in valeurs]
    rang = valeurs.index(courant) if courant in valeurs else None

    def choisi(r):
        liste.set(valeurs[r])
        liste.icursor("end")
        liste.selection_clear()
        liste.focus_set()
        liste.event_generate("<<ComboboxSelected>>")

    liste_de(liste).montrer(lignes, liste.winfo_rootx(), liste.winfo_rooty() + liste.winfo_height() + 4,
                            largeur_min=liste.winfo_width(), quand_choisi=choisi, rang=rang,
                            au_dessus=liste.winfo_rooty())


def installer(racine):
    """Les listes déroulantes de cette racine s'ouvrent dans une carte (une fois par racine)."""
    if getattr(racine, "_menus_memoire", False):
        return
    racine._menus_memoire = True
    # La procédure doit exister avant d'être remplacée : une liste créée la charge au besoin.
    if not racine.tk.call("info", "procs", "ttk::combobox::Post"):
        from tkinter import ttk
        ttk.Combobox(racine).destroy()
    # Un clic ailleurs que sur une carte ouverte la ferme ; le clic agit ensuite normalement.
    def clic_ailleurs(evenement):
        for carte in list(_OUVERTES):
            if evenement.widget is not carte.canevas and not carte._vient_d_ouvrir:
                carte.fermer()

    racine.bind_all("<ButtonPress-1>", clic_ailleurs, add="+")
    racine.createcommand("memoire_poster_liste", lambda chemin: _poster(racine, chemin))
    racine.tk.eval("proc ::ttk::combobox::Post {cb} {memoire_poster_liste $cb}")
