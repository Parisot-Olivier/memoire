"""Réalisation Tkinter du plan de travail (PLAN-DE-TRAVAIL.md ; D28 ; INV-PLAN-01).

Autour d'une zone centrale fixe, quatre zones d'ancrage (PanedWindow) reçoivent les panneaux ; un
panneau peut aussi flotter (`wm manage`) ou être masqué. La `Disposition` (gui/plan_de_travail.py)
est la seule source de vérité : chaque action la modifie, puis `appliquer` met l'écran en accord.

Un panneau est un cadre enfant de la fenêtre principale, jamais recréé : ancré, il est placé dans
une case de sa zone (`pack -in`, permis puisque la case descend de son parent) ; flottant, il devient
une fenêtre de premier niveau ; masqué, il n'est plus placé. Son contenu (un shell) reste vivant.
"""
from dataclasses import dataclass
import tkinter as tk
from tkinter import ttk

from plan_de_travail import ZONES, Defaut, Disposition, ecrire, lire
import menus

ARTICLES = {"haut": "en haut", "bas": "en bas", "gauche": "à gauche", "droite": "à droite"}
ORDRE_DES_FENETRES = (0, 1, 2)  # entrées du menu « ⋯ » actives pour un panneau flottant
FLECHES = {"haut": "horizontal", "bas": "horizontal", "gauche": "vertical", "droite": "vertical"}


@dataclass
class Panneau:
    nom: str
    titre: str
    defaut: Defaut
    construire: object
    cadre: object = None
    contenu: object = None
    bouton: object = None
    marqueur: object = None
    gere: bool = False
    case: object = None
    etiquette: object = None
    visible_d_abord: bool = False
    menu: object = None
    bouton_menu: object = None
    sans_bandeau: bool = False


def _panneaux_conserves(chemin):
    """Noms des panneaux de la disposition conservée (aucun si le fichier manque ou est illisible)."""
    import json
    from pathlib import Path
    try:
        contenu = json.loads(Path(chemin).read_text(encoding="utf-8"))
        return set(contenu.get("panneaux", {}))
    except (OSError, ValueError, AttributeError):
        return set()


class PlanDeTravail:
    """Zones d'ancrage autour de `self.centre`, panneaux et leur menu."""

    def __init__(self, racine, chemin, signaler=lambda texte: None, parent=None):
        """`parent` : cadre qui reçoit les zones (la carte papier, APPARENCE.md) ; la racine
        sinon. Les panneaux restent des enfants de la racine (fenêtres flottantes)."""
        self.racine = racine
        self.chemin = chemin
        self.signaler = signaler
        self.panneaux = {}
        self.disposition = None
        self.menu = None
        self._suivi_flottant = {}
        self.vertical = ttk.PanedWindow(parent if parent is not None else racine,
                                        orient="vertical")
        self.vertical.pack(fill="both", expand=True)
        self.milieu = ttk.PanedWindow(self.vertical, orient="horizontal")
        self.centre = ttk.Frame(self.milieu)
        parents = {"haut": self.vertical, "bas": self.vertical,
                   "gauche": self.milieu, "droite": self.milieu}
        self.zones = {z: ttk.PanedWindow(parents[z], orient=FLECHES[z]) for z in ZONES}
        self.cases = {z: [] for z in ZONES}
        # Appelées après chaque mise en accord de l'écran (la fenêtre principale suit la console).
        self.quand_applique = []
        for fenetre in (self.vertical, self.milieu):
            fenetre.bind("<ButtonRelease-1>", lambda _e: self._noter_les_parts(), add="+")

    # ----- Inscription -----

    def ajouter(self, nom, titre, defaut, construire, visible_d_abord=False, sans_bandeau=False):
        """Inscrit un panneau ; `construire(cadre)` rend son contenu, appelé une seule fois ;
        `visible_d_abord` : montré au premier lancement (absent de la disposition conservée),
        puis à la place que l'utilisateur lui donne ; `sans_bandeau` : le contenu porte lui-même
        ses actions de panneau (la console, CONSOLE.md)."""
        self.panneaux[nom] = Panneau(nom, titre, defaut, construire,
                                     visible_d_abord=visible_d_abord, sans_bandeau=sans_bandeau)

    def demarrer(self, menu_parent):
        """Lit la disposition conservée, crée les panneaux et leur menu, puis applique."""
        ecran = (self.racine.winfo_screenwidth(), self.racine.winfo_screenheight())
        definitions = {nom: p.defaut for nom, p in self.panneaux.items()}
        self.disposition, avertissement = lire(self.chemin, definitions, ecran=ecran)
        if avertissement:
            self.signaler(avertissement)
        conserves = _panneaux_conserves(self.chemin)
        for nom, panneau in self.panneaux.items():
            if panneau.visible_d_abord and nom not in conserves:
                self.disposition.montrer(nom)
        for panneau in self.panneaux.values():
            self._creer(panneau)
        self.menu = menus.Menu(menu_parent)
        self._reconstruire_le_menu()
        self.appliquer()

    def _reconstruire_le_menu(self):
        """Une entrée par panneau, sous son titre actuel, puis « Disposition par défaut »."""
        self.menu.delete(0, "end")
        for panneau in self.panneaux.values():
            self.menu.add_command(label=panneau.titre,
                                  command=lambda n=panneau.nom: self.basculer(n))
        self.menu.add_separator()
        self.menu.add_command(label="Disposition par défaut", command=self.disposition_par_defaut)

    # ----- Panneaux dynamiques (TERMINAUX.md, section 3) -----

    def ajouter_panneau(self, nom, titre, defaut, construire):
        """Ajoute un panneau en cours de session, masqué à sa place par défaut."""
        self.disposition.ajouter(nom, defaut)
        panneau = Panneau(nom, titre, defaut, construire)
        self.panneaux[nom] = panneau
        self._creer(panneau)
        self._reconstruire_le_menu()
        self.appliquer()
        self.enregistrer()

    def retirer_panneau(self, nom):
        """Retire un panneau et détruit son cadre (son contenu s'arrête à sa destruction)."""
        panneau = self.panneaux.pop(nom)
        self.disposition.retirer(nom)
        if panneau.gere:
            self.racine.tk.call("wm", "forget", panneau.cadre)
        panneau.cadre.destroy()
        self._reconstruire_le_menu()
        self.appliquer()
        self.enregistrer()

    def renommer(self, nom, titre):
        """Change le titre d'un panneau : bandeau, fenêtre flottante, menu."""
        panneau = self.panneaux[nom]
        if panneau.titre == titre:
            return
        panneau.titre = titre
        panneau.etiquette.configure(text=titre)
        if panneau.gere:
            self.racine.tk.call("wm", "title", panneau.cadre, f"memoire — {titre}")
        self._reconstruire_le_menu()

    def titre(self, nom):
        return self.panneaux[nom].titre

    def _creer(self, panneau):
        # Apparence (APPARENCE.md) : cadre sur le papier, bandeau aux petites pastilles (thème).
        cadre = tk.Frame(self.racine, borderwidth=0, highlightthickness=0)
        nom = panneau.nom
        # Une rangée (8.2b) : le titre (le nom d'un terminal, qui change, raccourci si besoin) et
        # son marqueur ; à droite « Détacher » / « Ancrer » et « ⋯ », qui ouvre le reste.
        rangee = ttk.Frame(cadre, padding=(8, 6, 6, 4))
        if not panneau.sans_bandeau:
            rangee.pack(fill="x")
        panneau.bouton_menu = ttk.Button(rangee, text="⋯", width=2, style="Bandeau.TButton",
                                         command=lambda: self._ouvrir_sous(panneau.bouton_menu,
                                                                           panneau.menu))
        panneau.bouton_menu.pack(side="right")
        panneau.bouton = ttk.Button(rangee, text="Détacher", style="Bandeau.TButton",
                                    command=lambda: self._basculer_flottant(nom))
        panneau.bouton.pack(side="right", padx=(0, 6))
        panneau.etiquette = ttk.Label(rangee, text=panneau.titre, style="Question.TLabel")
        panneau.etiquette.pack(side="left")
        panneau.marqueur = ttk.Label(rangee, text="", style="Marqueur.TLabel")
        panneau.marqueur.pack(side="left", padx=6)
        # Ordre d'une fenêtre flottante (section 10) : c'est l'utilisateur qui décide ; ces entrées
        # ne sont actives que pour un panneau flottant.
        panneau.case = tk.BooleanVar(value=False)
        panneau.menu = menu = menus.Menu(rangee)
        menu.add_checkbutton(label="Toujours au premier plan", variable=panneau.case,
                             command=lambda: self.premier_plan(nom, panneau.case.get()))
        menu.add_command(label="Mettre devant", command=lambda: self.mettre_devant(nom))
        menu.add_command(label="Mettre derrière", command=lambda: self.mettre_derriere(nom))
        menu.add_separator()
        for zone in ZONES:
            menu.add_command(label=f"Ancrer {ARTICLES[zone]}",
                             command=lambda z=zone: self.ancrer(nom, z))
        menu.add_separator()
        menu.add_command(label="Monter", command=lambda: self.monter(nom))
        menu.add_command(label="Descendre", command=lambda: self.descendre(nom))
        menu.add_separator()
        if panneau.sans_bandeau:
            # Sans bandeau, pas de bouton « Détacher » : il est ici (inactif pour une fenêtre).
            menu.add_command(label="Détacher", command=lambda: self.detacher(nom))
        menu.add_command(label="Masquer", command=lambda: self.masquer(nom))
        corps = ttk.Frame(cadre)
        corps.pack(fill="both", expand=True)
        panneau.cadre = cadre
        panneau.contenu = panneau.construire(corps)
        cadre.bind("<Configure>", lambda e, n=nom: self._configure(n, e), add="+")

    @staticmethod
    def _ouvrir_sous(bouton, menu):
        menu.tk_popup(bouton.winfo_rootx(), bouton.winfo_rooty() + bouton.winfo_height() + 4)

    # ----- Lecture -----

    def cadre(self, nom):
        return self.panneaux[nom].cadre

    def contenu(self, nom):
        return self.panneaux[nom].contenu

    def visible(self, nom):
        return self.disposition.etat(nom).etat != "masque"

    def marque(self, nom):
        return str(self.panneaux[nom].marqueur.cget("text"))

    def marquer(self, nom, texte):
        self.panneaux[nom].marqueur.configure(text=texte)

    def texte_du_bouton(self, nom):
        return str(self.panneaux[nom].bouton.cget("text"))

    def etat_du_menu_plan(self, nom):
        """État des entrées de l'ordre d'une fenêtre (menu « ⋯ ») : actives si le panneau flotte."""
        return self.panneaux[nom].menu.entrycget(ORDRE_DES_FENETRES[0], "state")

    def case_premier_plan(self, nom):
        return bool(self.panneaux[nom].case.get())

    # ----- Actions : la disposition change, puis l'écran suit -----

    def _changer(self, operation, *arguments):
        operation(*arguments)
        self.appliquer()
        self.enregistrer()

    def montrer(self, nom):
        self.marquer(nom, "")
        self._changer(self.disposition.montrer, nom)

    def masquer(self, nom):
        self._changer(self.disposition.masquer, nom)

    def basculer(self, nom):
        if self.visible(nom):
            self.masquer(nom)
        else:
            self.montrer(nom)

    def ancrer(self, nom, zone=None, rang=None):
        self._changer(self.disposition.ancrer, nom, zone, rang)

    def detacher(self, nom, geometrie=None):
        self._changer(self.disposition.detacher, nom, geometrie)

    def mettre_en_page(self, nom):
        """Le panneau prend toute la carte, par-dessus les zones (CONSOLE.md, section 1)."""
        self._changer(self.disposition.mettre_en_page, nom)

    def quitter_la_page(self, nom):
        self._changer(self.disposition.quitter_la_page, nom)

    def monter(self, nom):
        self._changer(self.disposition.monter, nom)

    def descendre(self, nom):
        self._changer(self.disposition.descendre, nom)

    def premier_plan(self, nom, actif):
        self._changer(self.disposition.premier_plan, nom, actif)

    def mettre_devant(self, nom):
        """Amène une fois la fenêtre flottante devant les autres. KWin ignore la demande `raise`
        d'une application (prévention du vol de focus) ; retirer puis réafficher la fenêtre la
        place devant, à la même géométrie (essai sous KWin, 10 novembre 2026)."""
        etat = self.disposition.etat(nom)
        if etat.etat != "flottant":
            return
        cadre = self.panneaux[nom].cadre
        self.racine.tk.call("wm", "withdraw", cadre)
        self.racine.tk.call("wm", "geometry", cadre, etat.geometrie)
        self.racine.tk.call("wm", "deiconify", cadre)
        self.racine.tk.call("raise", cadre)

    def mettre_derriere(self, nom):
        """Place une fois la fenêtre flottante derrière la fenêtre principale, et décoche
        « Toujours au premier plan »."""
        if self.disposition.etat(nom).etat != "flottant":
            return
        if self.disposition.etat(nom).premier_plan:
            self.premier_plan(nom, False)
        self.racine.tk.call("lower", self.panneaux[nom].cadre, self.racine)

    def disposition_par_defaut(self):
        definitions = {nom: p.defaut for nom, p in self.panneaux.items()}
        self.disposition = Disposition.par_defaut(definitions)
        self.appliquer()
        self.enregistrer()

    def _basculer_flottant(self, nom):
        if self.disposition.etat(nom).etat == "flottant":
            self.ancrer(nom)
        else:
            self.detacher(nom)

    def enregistrer(self):
        try:
            ecrire(self.chemin, self.disposition)
        except OSError as erreur:
            self.signaler(f"disposition des panneaux non enregistrée : {erreur}")

    def fermer(self):
        for attente in self._suivi_flottant.values():
            try:
                self.racine.after_cancel(attente)
            except tk.TclError:
                pass
        self._suivi_flottant.clear()
        self._noter_les_parts()
        self.enregistrer()

    # ----- Mise en accord de l'écran -----

    def appliquer(self):
        for panneau in self.panneaux.values():
            panneau.cadre.pack_forget()
            panneau.cadre.place_forget()
        for zone in ZONES:
            for case in self.cases[zone]:
                case.destroy()
            self.cases[zone] = []
        for fenetre in (self.vertical, self.milieu):
            for enfant in fenetre.panes():
                fenetre.forget(enfant)
        visibles = self.disposition.zones_visibles()
        if "haut" in visibles:
            self.vertical.add(self.zones["haut"], weight=1)
        self.vertical.add(self.milieu, weight=4)
        if "bas" in visibles:
            self.vertical.add(self.zones["bas"], weight=1)
        if "gauche" in visibles:
            self.milieu.add(self.zones["gauche"], weight=1)
        self.milieu.add(self.centre, weight=4)
        if "droite" in visibles:
            self.milieu.add(self.zones["droite"], weight=1)
        cases = {}
        for zone in visibles:
            for nom in self.disposition.panneaux_de(zone):
                case = ttk.Frame(self.zones[zone])
                part = self.disposition.etat(nom).part
                self.zones[zone].add(case, weight=max(1, int(part * 100)))
                self.cases[zone].append(case)
                cases[nom] = case
        for nom, panneau in self.panneaux.items():
            etat = self.disposition.etat(nom)
            if etat.etat == "flottant":
                self._faire_flotter(panneau, etat)
            else:
                if panneau.gere:
                    self.racine.tk.call("wm", "forget", panneau.cadre)
                    panneau.gere = False
                if etat.etat == "ancre":
                    panneau.cadre.pack(in_=cases[nom], fill="both", expand=True)
                    panneau.cadre.lift()
                elif etat.etat == "page":
                    # Pleine page : posé sur tout le plan de travail ; rien n'est défait dessous.
                    panneau.cadre.place(in_=self.vertical, x=0, y=0, relwidth=1, relheight=1)
            panneau.bouton.configure(text="Ancrer" if etat.etat == "flottant" else "Détacher")
            for rang in ORDRE_DES_FENETRES:
                panneau.menu.entryconfigure(rang, state="normal" if etat.etat == "flottant"
                                            else "disabled")
            panneau.case.set(etat.premier_plan)
        en_page = self.disposition.en_page()
        if en_page is not None:
            self.panneaux[en_page].cadre.lift()
        self.racine.update_idletasks()
        self._poser_les_parts(visibles)
        for nom, panneau in self.panneaux.items():
            if self.visible(nom) and hasattr(panneau.contenu, "quand_visible"):
                panneau.contenu.quand_visible()
        for suite in self.quand_applique:
            suite()

    def _faire_flotter(self, panneau, etat):
        cadre = panneau.cadre
        if not panneau.gere:
            cadre.pack_forget()
            cadre.place_forget()
            self.racine.tk.call("wm", "manage", cadre)
            self.racine.tk.call("wm", "protocol", cadre, "WM_DELETE_WINDOW",
                                self.racine.register(lambda n=panneau.nom: self.masquer(n)))
            panneau.gere = True
        self.racine.tk.call("wm", "title", cadre, f"memoire — {panneau.titre}")
        self.racine.tk.call("wm", "geometry", cadre, etat.geometrie)
        self.racine.tk.call("wm", "attributes", cadre, "-topmost", int(etat.premier_plan))
        self.racine.tk.call("wm", "deiconify", cadre)

    def _poser_les_parts(self, visibles):
        largeur = self.milieu.winfo_width()
        hauteur = self.vertical.winfo_height()
        if largeur <= 1 or hauteur <= 1:
            return
        z = self.disposition.part_de_zone
        try:
            if "gauche" in visibles:
                self.milieu.sashpos(0, int(largeur * z("gauche")))
            if "droite" in visibles:
                self.milieu.sashpos(len(self.milieu.panes()) - 2, int(largeur * (1 - z("droite"))))
            if "haut" in visibles:
                self.vertical.sashpos(0, int(hauteur * z("haut")))
            if "bas" in visibles:
                self.vertical.sashpos(len(self.vertical.panes()) - 2, int(hauteur * (1 - z("bas"))))
        except tk.TclError:
            pass

    def _noter_les_parts(self):
        """Parts des zones d'après les séparations déplacées à la souris."""
        if self.disposition is None:
            return
        visibles = self.disposition.zones_visibles()
        largeur, hauteur = self.milieu.winfo_width(), self.vertical.winfo_height()
        if largeur <= 1 or hauteur <= 1:
            return
        try:
            if "gauche" in visibles:
                self.disposition.redimensionner_zone("gauche", self.milieu.sashpos(0) / largeur)
            if "droite" in visibles:
                position = self.milieu.sashpos(len(self.milieu.panes()) - 2)
                self.disposition.redimensionner_zone("droite", 1 - position / largeur)
            if "haut" in visibles:
                self.disposition.redimensionner_zone("haut", self.vertical.sashpos(0) / hauteur)
            if "bas" in visibles:
                position = self.vertical.sashpos(len(self.vertical.panes()) - 2)
                self.disposition.redimensionner_zone("bas", 1 - position / hauteur)
        except tk.TclError:
            return
        self.enregistrer()

    def _configure(self, nom, evenement):
        """Une fenêtre flottante déplacée ou redimensionnée : sa géométrie est notée, puis
        enregistrée une demi-seconde après le dernier mouvement."""
        panneau = self.panneaux[nom]
        if evenement.widget is not panneau.cadre or not panneau.gere:
            return
        geometrie = self.racine.tk.call("wm", "geometry", panneau.cadre)
        try:
            self.disposition.deplacer(nom, str(geometrie))
        except ValueError:
            return
        if nom in self._suivi_flottant:
            self.racine.after_cancel(self._suivi_flottant[nom])
        self._suivi_flottant[nom] = self.racine.after(500, self.enregistrer)
