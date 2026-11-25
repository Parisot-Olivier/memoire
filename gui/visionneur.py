"""Visionneur d'une note ou d'un échange (VISIONNEUR.md, sections 1 et 5 ; D25).

Lecture rendue ou brute, surlignage des citations, citation d'une sélection, versions d'une note.
Aucune règle ici : le texte vient de `memoire source`, les citations de `memoire citations`, et
une citation se crée par `memoire citer --plage`, dans la fenêtre principale qui en affiche le
compte rendu. La correspondance entre l'affichage et la source est celle de `rendu.py`.
"""
import subprocess
import tkinter as tk
from tkinter import ttk

from actions import mot
from apparence import COULEURS
from champs import liste_de_notes
from theme import police
from rendu import brut, rendre
from voisinage import entete, titre_et_role

CONFIRMATION_REQUISE = 4

# Carte « Relier la sélection » (CENTRE.md, section 4) : option de `citer` et libellé.
LIENS = [
    ("--soutient", "Soutient"),
    ("--inspire", "Inspire"),
    ("--classe-dans", "Classe dans"),
    ("--lien", "Lien"),
]
ROLES = ["raisonnement", "implementation", "note", "categorie"]

# Styles du rendu, préfixés pour ne pas se confondre avec l'étiquette des citations.
STYLES = {
    "titre1": {"font": ("TkDefaultFont", 16, "bold")},
    "titre2": {"font": ("TkDefaultFont", 14, "bold")},
    "titre3": {"font": ("TkDefaultFont", 12, "bold")},
    "titre4": {"font": ("TkDefaultFont", 11, "bold")},
    "titre5": {"font": ("TkDefaultFont", 10, "bold")},
    "titre6": {"font": ("TkDefaultFont", 10, "bold italic")},
    "gras": {"font": ("TkDefaultFont", 10, "bold")},
    "italique": {"font": ("TkDefaultFont", 10, "italic")},
    "code": {"font": ("TkFixedFont", 10), "background": "#F6E6DD"},
    "bloc": {"font": ("TkFixedFont", 10), "background": "#F8ECE5"},
    "citation": {"foreground": "#555555", "lmargin1": 18, "lmargin2": 18},
    "lien": {"foreground": "#1a55a8", "underline": True},
    "puce": {},
    "separateur": {"foreground": COULEURS["etiquette"]},
}


def interroger(memoire, base, args):
    """Lance l'exécutable et rend son code, sa sortie et ses erreurs, sans rien afficher."""
    resultat = subprocess.run([memoire, "--base", base, *args], capture_output=True, text=True,
                              encoding="utf-8")
    return resultat.returncode, resultat.stdout, resultat.stderr


def lire_citations(sortie):
    """Lignes de `memoire citations` : (identifiant, début, fin, statut)."""
    citations = []
    for ligne in sortie.splitlines():
        identifiant, debut, fin, statut = ligne.split("\t")
        citations.append((identifiant, int(debut), int(fin), statut))
    return citations


def position(caracteres):
    """Index Tk d'une position en caractères affichés."""
    return f"1.0 + {caracteres} chars"


class Zone:
    """Une zone de lecture : une partie d'un échange, ou le texte d'une note. La question d'un
    échange est dans un encart crème ; le reste est sur le papier, à largeur de lecture."""

    def __init__(self, parent, titre, partie, encart=False):
        self.partie = partie
        fond = COULEURS["encart"] if encart else COULEURS["papier"]
        self.cadre = ttk.Frame(parent, style="Encart.TFrame" if encart else "TFrame",
                               padding=(18, 12) if encart else (0, 6))
        self.cadre.pack(fill="x" if encart else "both", expand=not encart, pady=(0, 12))
        ttk.Label(self.cadre, text=titre.upper(),
                  style="Encart.TLabel" if encart else "Etiquette.TLabel").pack(anchor="w")
        # Dans l'encart, le fond est celui de l'encart (un cadre classique, de sa couleur).
        rangee = tk.Frame(self.cadre, background=fond) if encart else ttk.Frame(self.cadre)
        rangee.pack(fill="both", expand=True, pady=(4, 0))
        # Largeur de lecture : environ 72 caractères ; la sélection reste quand on clique ailleurs.
        self.texte = tk.Text(rangee, wrap="word", height=4 if encart else 12, width=72,
                             font=police(11), spacing1=3, spacing3=5, background=fond,
                             borderwidth=0, highlightthickness=0, padx=2, pady=4,
                             exportselection=False, cursor="xterm")
        if not encart:
            defilement = ttk.Scrollbar(rangee, orient="vertical", command=self.texte.yview)
            self.texte.configure(yscrollcommand=defilement.set)
            defilement.pack(side="right", fill="y")
        self.texte.pack(side="left", fill="both", expand=True)
        for nom, options in STYLES.items():
            options = dict(options)
            if "font" in options:
                famille, taille, *reste = options["font"]
                options["font"] = police(taille + 1, *reste, fixe=famille == "TkFixedFont")
            self.texte.tag_configure(f"style-{nom}", **options)
        self.texte.tag_configure("citation", background=COULEURS["citation"], underline=True)
        self.texte.tag_raise("citation")
        self.texte.tag_raise("sel")
        self.rendu = None
        self.citations = []

    def afficher(self, rendu, citations):
        self.rendu = rendu
        self.citations = citations
        self.texte.configure(state="normal")
        self.texte.delete("1.0", "end")
        self.texte.insert("1.0", rendu.affiche)
        for debut, fin, style in rendu.styles:
            self.texte.tag_add(f"style-{style}", position(debut), position(fin))
        for _, debut, fin, _ in citations:
            for a, b in rendu.plages_affichees(debut, fin):
                self.texte.tag_add("citation", position(a), position(b))
        # Lecture seule : la sélection reste possible.
        self.texte.configure(state="disabled")

    def caractere(self, index):
        """Position en caractères affichés d'un index Tk."""
        return len(self.texte.get("1.0", index))

    def citation_en(self, caractere):
        """Citation surlignée à cette position affichée, s'il y en a une."""
        for citation in self.citations:
            _, debut, fin, _ = citation
            if any(a <= caractere < b for a, b in self.rendu.plages_affichees(debut, fin)):
                return citation
        return None


def _placer_pres(cadre, widget, x, y, limite=None, haut_de_l_ancre=0):
    """Pose `cadre` (enfant de la fenêtre de `widget`) au point (x, y) de `widget`, sans
    déborder de la fenêtre, ni du bas de `limite` s'il est donné (la zone de lecture)."""
    fenetre = cadre.master
    fenetre.update_idletasks()
    gx = widget.winfo_rootx() - fenetre.winfo_rootx() + x
    gy = widget.winfo_rooty() - fenetre.winfo_rooty() + y
    largeur, hauteur = cadre.winfo_reqwidth(), cadre.winfo_reqheight()
    gx = max(8, min(gx, fenetre.winfo_width() - largeur - 8))
    bas = fenetre.winfo_height() - 8
    if limite is not None and limite.winfo_ismapped():
        bas = min(bas, limite.winfo_rooty() - fenetre.winfo_rooty() + limite.winfo_height() - 4)
    if gy + hauteur > bas:
        dessus = widget.winfo_rooty() - fenetre.winfo_rooty() + haut_de_l_ancre - hauteur - 6
        gy = dessus if dessus >= 8 else max(8, fenetre.winfo_height() - hauteur - 8)
    cadre.place(x=gx, y=gy)
    cadre.lift()


class BarreDeSelection:
    """Barre de la sélection (8.3) : une pastille au bas de l'article, montrée quand du texte est
    choisi : citer, soutient, inspire, nouvelle note ; « ⋯ » : classe dans, lien, doute."""

    def __init__(self, vue, echange, parent):
        import menus
        self.vue = vue
        self.cadre = ttk.Frame(parent, style="Pilule.TFrame", padding=(18, 10, 14, 12))
        self.boutons = []
        self.etiquette = ttk.Label(self.cadre, text="Sélection :", style="PiluleSecond.TLabel")
        self.etiquette.pack(side="left", padx=(0, 8))
        bouton = ttk.Button(self.cadre, text="Citer la sélection", style="PiluleAction.TButton",
                            command=vue.citer_selection)
        bouton.pack(side="left", padx=(0, 6))
        self.boutons.append(bouton)
        for libelle, cible in (("Soutient…", "--soutient"), ("Inspire…", "--inspire"),
                               ("Nouvelle note…", "note")):
            bouton = ttk.Button(self.cadre, text=libelle, style="PiluleBandeau.TButton",
                                command=lambda c=cible: self.ouvrir(c))
            bouton.pack(side="left", padx=(0, 6))
            self.boutons.append(bouton)
        self.menu_autres = menus.Menu(self.cadre)
        autres = [("Classe dans…", "--classe-dans"), ("Lien…", "--lien")]
        if echange:
            autres.append(("Doute à suivre…", "reserve"))
        for libelle, cible in autres:
            self.menu_autres.add_command(label=libelle, command=lambda c=cible: self.ouvrir(c))
        self.bouton_autres = ttk.Button(self.cadre, text="⋯", width=2, style="PiluleBandeau.TButton")
        self.bouton_autres.configure(command=lambda: self.menu_autres.tk_popup(
            self.bouton_autres.winfo_rootx(), self.bouton_autres.winfo_rooty() - 4
            - 32 * len(autres) - 24))
        self.bouton_autres.pack(side="left")
        self.boutons.append(self.bouton_autres)

    def libelles(self):
        return [str(b.cget("text")) for b in self.boutons]

    def visible(self):
        return self.cadre.winfo_manager() == "pack"

    def montrer(self, _zone=None):
        if not self.visible():
            self.cadre.pack(side="left", anchor="w")
        # Colonne étroite : le mot « Sélection : » cède la place, les boutons restent entiers.
        self.cadre.update_idletasks()
        place = self.cadre.master.winfo_width()
        if place > 1 and self.cadre.winfo_reqwidth() > place and self.etiquette.winfo_manager():
            self.etiquette.pack_forget()

    def cacher(self):
        self.cadre.pack_forget()

    def ouvrir(self, cible):
        """Ouvre la carte « Relier la sélection » sur le champ de `cible`."""
        self.vue.ouvrir_relier(cible)


class CarteDeCitation:
    """Carte d'un passage cité (8.2c) : identifiant, octets, état ; près du clic."""

    def __init__(self, vue):
        self.vue = vue
        self.cadre = ttk.Frame(vue.fenetre, style="Carte.TFrame", padding=(16, 12))
        haut = ttk.Frame(self.cadre)
        haut.pack(fill="x")
        self.titre = ttk.Label(haut, text="Passage cité", style="Question.TLabel")
        self.titre.pack(side="left")
        ttk.Button(haut, text="✕", width=2, style="Bandeau.TButton",
                   command=self.fermer).pack(side="right")
        self.reference = ttk.Label(self.cadre, text="", style="Mono.TLabel", font="MemoireMonoPetite")
        self.reference.pack(anchor="w", pady=(6, 6))
        self.puces = ttk.Frame(self.cadre)
        self.puces.pack(anchor="w")
        self.etats = []

    def textes(self):
        return [str(self.titre.cget("text")), str(self.reference.cget("text"))] + \
            [str(p.cget("text")) for p in self.etats]

    def visible(self):
        return bool(self.cadre.winfo_manager())

    def montrer(self, citation, zone, index):
        identifiant, debut, fin, statut = citation
        verdict, *autres = statut.split(",")
        self.reference.configure(text=f"{identifiant[:8]} · octets {debut} à {fin}")
        for puce in self.etats:
            puce.destroy()
        revisee = "source-revisee" in autres
        self.etats = [
            ttk.Label(self.puces, text=mot("verdict", verdict), style="Genre.TLabel",
                      font="MemoirePetite"),
            ttk.Label(self.puces, text="source révisée" if revisee else "à jour",
                      style="PuceAmbre.TLabel" if revisee else "PuceBleue.TLabel"),
        ]
        for puce in self.etats:
            puce.pack(side="left", padx=(0, 6))
        boite = zone.texte.bbox(index) or (0, 0, 0, 16)
        _placer_pres(self.cadre, zone.texte, boite[0] + 20, boite[1] + boite[3] + 8)

    def fermer(self):
        self.cadre.place_forget()


class Visionneur:
    """Lecture d'une note ou d'un échange (VISIONNEUR.md ; APPARENCE.md, section 9.3).

    Avec `parent`, la lecture se construit dans ce cadre (la carte de la fenêtre principale) ;
    sans, dans une fenêtre à elle (« Ouvrir dans une fenêtre »)."""

    def __init__(self, app, note, parent=None):
        self.app = app
        self.note = note
        self.brut = False
        self.version = None
        self.a_confirmer = None
        self._versions = None
        self.dans_la_carte = parent is not None
        if parent is None:
            self.fenetre = tk.Toplevel(app.racine)
            self.fenetre.title(f"memoire — {note}")
            self.fenetre.geometry("1000x860")
            self.cadre = ttk.Frame(self.fenetre, padding=(26, 18, 26, 14))
            self.cadre.pack(fill="both", expand=True)
        else:
            self.fenetre = parent.winfo_toplevel()
            self.cadre = ttk.Frame(parent, padding=(30, 20, 26, 12))
            self.cadre.pack(fill="both", expand=True)
        code, _, _ = self.interroger(["source", note, "--partie", "reponse"])
        self.echange = code == 0
        code_v, lisible, _ = self.interroger(["voisinage", note])
        titre, role = (titre_et_role(entete(lisible)) if code_v == 0 and
                       lisible.startswith("Voisinage de ") else (note, ""))
        self._entete(titre, role)
        # Le bas de l'article (barre de sélection, confirmation, état) est réservé d'abord.
        self.bas = ttk.Frame(self.cadre)
        self.bas.pack(side="bottom", fill="x", pady=(8, 0))
        self.rangee_de_la_barre = ttk.Frame(self.bas)
        self.rangee_de_la_barre.pack(fill="x")
        corps = ttk.Frame(self.cadre)
        corps.pack(fill="both", expand=True, pady=(14, 0))
        if self.echange:
            self.zones = [Zone(corps, "Question", "question", encart=True),
                          Zone(corps, "Réponse", "reponse")]
        else:
            self.zones = [Zone(corps, "Texte", None)]
        for zone in self.zones:
            zone.texte.tag_bind("citation", "<Button-1>",
                                lambda _e, z=zone: self.montrer_citation(z, "current"))
            zone.texte.bind("<<Selection>>", lambda _e, z=zone: self.selection_changee(z),
                            add="+")
        self.confirmation = ttk.Frame(self.bas, style="Carte.TFrame", padding=(14, 10))
        ttk.Label(self.confirmation, text="Opération importante : le détail est dans le journal "
                  "et l’Aperçu.", style="Second.TLabel").pack(side="left")
        self.bouton_confirmer = ttk.Button(self.confirmation, text="Confirmer", state="disabled",
                                           style="Action.TButton", command=self.confirmer)
        self.bouton_confirmer.pack(side="right")
        self.etat = ttk.Label(self.bas, text="", wraplength=860, justify="left",
                              style="Second.TLabel")
        self.etat.pack(fill="x", pady=(6, 0))
        self.panneau(self.echange)
        self.barre = BarreDeSelection(self, self.echange, self.rangee_de_la_barre)
        self.carte_citation = CarteDeCitation(self)
        self.fenetre.bind("<Escape>", lambda _e: self.fermer_les_cartes(), add="+")
        self.charger()

    # ----- En-tête -----

    def _entete(self, titre, role):
        # Une rangée (8.3) : à gauche le retour, le genre et l'identifiant ; à droite la bascule
        # « Rendu | Brut » groupée, les versions, et la fenêtre.
        haut = ttk.Frame(self.cadre)
        haut.pack(fill="x")
        if self.dans_la_carte:
            ttk.Button(haut, text="‹ Retour", style="Bandeau.TButton",
                       command=self.app.retour_a_l_action).pack(side="left", padx=(0, 12))
        genre = "Échange avec une IA" if self.echange else (mot("role", role).capitalize()
                                                             if role else "Note")
        self.genre = ttk.Label(haut, text=genre, style="Genre.TLabel")
        self.genre.pack(side="left")
        ttk.Label(haut, text=self.note[:8], style="Second.TLabel",
                  font="MemoireMonoPetite").pack(side="left", padx=(10, 0))
        if self.dans_la_carte:
            ttk.Button(haut, text="⧉", width=2, style="Bandeau.TButton",
                       command=self.ouvrir_dans_une_fenetre).pack(side="right", padx=(8, 0))
        else:
            ttk.Button(haut, text="Fermer", style="Bandeau.TButton",
                       command=self.fenetre.destroy).pack(side="right", padx=(8, 0))
            ttk.Button(haut, text="Voisinage", style="Bandeau.TButton",
                       command=lambda: self.app.voisinage(self.note)).pack(side="right", padx=8)
        bascule = ttk.Frame(haut, style="Segment.TFrame", padding=(3, 3))
        bascule.pack(side="right")
        self.bouton_rendu = ttk.Button(bascule, text="Rendu", style="SegmentChoisi.TButton",
                                       command=lambda: self._brut(False))
        self.bouton_rendu.pack(side="left")
        self.bouton_brut = ttk.Button(bascule, text="Brut", style="SegmentAutre.TButton",
                                      command=lambda: self._brut(True))
        self.bouton_brut.pack(side="left", padx=(2, 0))
        self.boutons_de_version = []
        if not self.echange:
            versions = self.versions()
            if len(versions) > 1:
                self.versions_cadre = ttk.Frame(haut, style="Segment.TFrame", padding=(3, 3))
                self.versions_cadre.pack(side="right", padx=(0, 8))
                for version in versions:
                    bouton = ttk.Button(self.versions_cadre, text=version, width=2,
                                        style="SegmentAutre.TButton",
                                        command=lambda v=version: self.choisir_version(v))
                    bouton.pack(side="left", padx=(0, 2))
                    self.boutons_de_version.append(bouton)
                ttk.Label(haut, text="versions", style="Second.TLabel",
                          font="MemoirePetite").pack(side="right", padx=(0, 6))
        self.titre_de_la_note = ttk.Label(self.cadre, text=titre, style="Titre.TLabel",
                                          wraplength=820, justify="left")
        self.titre_de_la_note.pack(anchor="w", pady=(14, 0))
        self.titre_de_la_note.bind("<Configure>", lambda e: self.titre_de_la_note.configure(
            wraplength=max(200, e.width)), add="+")

    def ouvrir_dans_une_fenetre(self):
        return Visionneur(self.app, self.note)

    # ----- Relier la sélection (CENTRE.md) : une carte qui s'ouvre depuis la barre -----

    def panneau(self, echange):
        """Carte « Relier la sélection » : relations, note nouvelle, réserve (CENTRE.md)."""
        cadre = ttk.Frame(self.fenetre, style="Carte.TFrame", padding=(16, 14))
        self.relier = cadre
        haut = ttk.Frame(cadre)
        haut.grid(row=0, column=0, columnspan=4, sticky="we", pady=(0, 8))
        ttk.Label(haut, text="Relier la sélection", style="Question.TLabel").pack(side="left")
        ttk.Button(haut, text="✕", width=2, style="Bandeau.TButton",
                   command=self.fermer_relier).pack(side="right")
        self.liens = {}
        self.listes = {}
        self.champs_de_relier = {}
        for ligne, (option, libelle) in enumerate(LIENS, start=1):
            ttk.Label(cadre, text=libelle).grid(row=ligne, column=0, sticky="w", padx=(0, 8))
            variable = tk.StringVar()
            liste = liste_de_notes(cadre, self.app.memoire, self.app.base.get, variable,
                                   multiple=True, largeur=40, connus=self.app.notes_connues)
            liste.grid(row=ligne, column=1, columnspan=3, sticky="we", pady=2)
            self.liens[option] = variable
            self.listes[option] = liste
            self.champs_de_relier[option] = liste
        rang = len(LIENS) + 1
        ttk.Label(cadre, text="Nouvelle note (titre)").grid(row=rang, column=0, sticky="w")
        self.nouvelle_note = tk.StringVar()
        champ = ttk.Entry(cadre, textvariable=self.nouvelle_note, width=30)
        champ.grid(row=rang, column=1, sticky="we", pady=2)
        self.champs_de_relier["note"] = champ
        self.role = tk.StringVar(value=ROLES[0])
        ttk.Combobox(cadre, textvariable=self.role, values=ROLES, state="readonly",
                     width=16).grid(row=rang, column=2, padx=4)
        ttk.Label(cadre, text="Réserve (titre)").grid(row=rang + 1, column=0, sticky="w")
        self.reserve = tk.StringVar()
        # Une réserve ne naît que d'une citation d'échange (D24).
        self.champ_reserve = ttk.Entry(cadre, textvariable=self.reserve, width=30,
                                       state="normal" if echange else "disabled")
        self.champ_reserve.grid(row=rang + 1, column=1, sticky="we", pady=2)
        self.champs_de_relier["reserve"] = self.champ_reserve
        ttk.Button(cadre, text="Citer la sélection", style="Action.TButton",
                   command=self.citer_selection).grid(row=rang + 2, column=0, columnspan=4,
                                                      sticky="w", pady=(10, 0))
        cadre.columnconfigure(1, weight=1)

    def relier_visible(self):
        return bool(self.relier.winfo_manager())

    def ouvrir_relier(self, cible):
        """Montre la carte « Relier la sélection » sous la barre, curseur dans le champ voulu."""
        barre = self.barre.cadre
        if barre.winfo_manager():
            # Au-dessus de la barre (elle est au bas de l'article).
            _placer_pres(self.relier, barre, 0, barre.winfo_height() + 6, limite=self.cadre,
                         haut_de_l_ancre=0)
        else:
            _placer_pres(self.relier, self.zones[-1].texte, 40, 40)
        champ = self.champs_de_relier.get(cible)
        if champ is not None:
            champ.focus_set()

    def fermer_relier(self):
        self.relier.place_forget()

    def fermer_les_cartes(self):
        self.fermer_relier()
        self.barre.cacher()
        self.carte_citation.fermer()

    def selection_changee(self, zone):
        """La barre suit la sélection ; elle reste tant que la carte « Relier » est ouverte."""
        if zone.texte.tag_ranges("sel"):
            self.carte_citation.fermer()
            self.barre.montrer(zone)
        elif not self.relier_visible():
            self.barre.cacher()

    def arguments_de_liaison(self):
        """Options de `citer` pour ce qui est rempli dans le panneau."""
        args = []
        from schema import identifiants
        for option, variable in self.liens.items():
            valeur = identifiants(variable.get().strip().strip(","), self.app.notes_connues)
            if valeur:
                args += [option, valeur]
        if self.nouvelle_note.get().strip():
            args += ["--note", self.nouvelle_note.get().strip(), "--role", self.role.get()]
        if self.reserve.get().strip():
            args += ["--reserve", self.reserve.get().strip()]
        return args

    def vider_panneau(self):
        for variable in self.liens.values():
            variable.set("")
        self.nouvelle_note.set("")
        self.reserve.set("")

    def interroger(self, args):
        return interroger(self.app.memoire, self.app.base.get(), args)

    def arguments_de_source(self, zone):
        """Partie d'un échange, ou version affichée d'une note (la courante par défaut)."""
        if zone.partie:
            return ["--partie", zone.partie]
        if self.version is not None:
            return ["--version", self.version]
        return []

    def charger(self):
        """Relit chaque zone : texte de la source, rendu, citations surlignées."""
        for zone in self.zones:
            args = self.arguments_de_source(zone)
            code, texte, _ = self.interroger(["source", self.note, *args])
            if code != 0:
                texte = ""
            rendu = brut(texte) if self.brut else rendre(texte)
            code, sortie, _ = self.interroger(["citations", self.note, *args])
            zone.afficher(rendu, lire_citations(sortie) if code == 0 else [])
        self.bouton_rendu.configure(style="SegmentAutre.TButton" if self.brut
                                    else "SegmentChoisi.TButton")
        self.bouton_brut.configure(style="SegmentChoisi.TButton" if self.brut
                                   else "SegmentAutre.TButton")
        courante = self.version or (self._versions[-1] if self._versions else None)
        for bouton in self.boutons_de_version:
            bouton.configure(style="SegmentChoisi.TButton" if str(bouton.cget("text")) == courante
                             else "SegmentAutre.TButton")

    def versions(self):
        """Versions d'une note : 0 pour le texte de création, puis une par révision."""
        if self._versions is None:
            self._versions = []
            while self.interroger(["source", self.note, "--version", str(len(self._versions))])[0] == 0:
                self._versions.append(str(len(self._versions)))
        return self._versions

    def choisir_version(self, version):
        self.version = version
        self.charger()

    def _brut(self, brut_):
        if self.brut != brut_:
            self.basculer_brut()

    def basculer_brut(self):
        self.brut = not self.brut
        self.charger()

    def selection(self):
        """Zone sélectionnée et plage affichée de la sélection, en caractères."""
        for zone in self.zones:
            plages = zone.texte.tag_ranges("sel")
            if plages:
                debut = zone.caractere(plages[0])
                return zone, debut, debut + len(zone.texte.get(plages[0], plages[1]))
        return None

    def citer_selection(self):
        """Cite la plus petite plage de la source qui contient la sélection (règle 2) ; rend le
        code de la commande, ou None sans sélection."""
        selection = self.selection()
        if selection is None:
            return None
        zone, debut, fin = selection
        plage = zone.rendu.plage_source(debut, fin)
        if plage is None:
            return None
        args = ["citer", self.note, "--plage", f"{plage[0]}-{plage[1]}",
                *self.arguments_de_source(zone), *self.arguments_de_liaison()]
        code = self.app.executer_args(args)
        if code == CONFIRMATION_REQUISE:
            self.a_confirmer = [*args, "--oui"]
            self.bouton_confirmer.configure(state="normal")
            self.confirmation.pack(fill="x", pady=(8, 0), before=self.etat)
            self.etat.configure(text="Opération critique : aperçu dans la fenêtre principale ; "
                                     "« Confirmer » pour l'appliquer.")
        else:
            self.a_confirmer = None
            self.bouton_confirmer.configure(state="disabled")
            self.terminer(code)
        return code

    def confirmer(self):
        if not self.a_confirmer:
            return None
        code = self.app.executer_args(self.a_confirmer)
        self.a_confirmer = None
        self.bouton_confirmer.configure(state="disabled")
        self.confirmation.pack_forget()
        self.terminer(code)
        return code

    def terminer(self, code):
        if code == 0:
            self.etat.configure(text="Citation créée : compte rendu dans le journal.")
            self.vider_panneau()
            self.fermer_les_cartes()
            self.charger()
        else:
            self.etat.configure(text=f"Refus (code {code}) : voir le journal.")

    def montrer_citation(self, zone, index):
        """Affiche la citation surlignée à cet endroit : carte de citation et ligne d'état."""
        citation = zone.citation_en(zone.caractere(index))
        if citation is None:
            return
        identifiant, debut, fin, statut = citation
        verdict, *autres = statut.split(",")
        texte = f"Citation {identifiant} : octets {debut} à {fin}, {verdict}"
        if "source-revisee" in autres:
            texte += ", source révisée"
        self.etat.configure(text=texte)
        self.carte_citation.montrer(citation, zone, index)

    def detruire(self):
        """La lecture quitte la carte : ses cartes flottantes partent avec elle."""
        for cadre in (self.carte_citation.cadre, self.relier):
            try:
                cadre.destroy()
            except tk.TclError:
                pass
        self.cadre.destroy()
