"""Panneau « Aperçu » (APPARENCE.md, section 9.3 ; 8.2b) : ce que ça va changer, dernières étapes.

« Ce que ça va changer » traduit les commandes que le formulaire lancera (`actions.decrire`) et se
met à jour pendant la saisie (« prévu ») ; pour une opération critique, l'aperçu du moteur
s'affiche après « Faire » (« à confirmer ») ; après une réussite, « fait ». « Dernières étapes »
lit `memoire historique`. Aucune règle ; le moteur n'est pas sollicité pour l'aperçu prévu.
"""
import re
import tkinter as tk
import tkinter.font as tkfont
from tkinter import ttk

import dessin
from actions import libelle_d_etape
from apparence import COULEURS as C
from dessin import rgba
from theme import photo

COULEUR_DU_SIGNE = {
    "+": C["indigo"], "→": C["bleu"], "←": C["bleu"], "◇": C["vert"], "✓": C["bleu"],
    "✕": C["vermillon"], "!": "#B0408F", "⇄": C["indigo"], "↶": C["indigo"], "↷": C["indigo"],
    "★": C["ambre"], "⚑": C["ambre"],
}
TEXTE_DU_SIGNE = {"→": "#0B4A75", "←": "#0B4A75", "✓": "#0B4A75", "◇": "#0B4F3D",
                  "✕": "#8A3A00", "+": "#2A26A8", "!": "#6E1F57"}
NEUTRE = "#8A7F8D"
ETAPES_MONTREES = 5


class PanneauApercu:
    """Contenu du panneau « Aperçu », dans le cadre `cadre` du plan de travail."""

    def __init__(self, cadre, app):
        self.app = app
        self.cadre = cadre
        self.lignes = []
        self.texte_du_moteur = ""
        self.etapes = []
        self._images = {}
        cadre.configure(padding=(10, 6, 10, 10))
        # Encadré clair, comme la maquette.
        self.carte = carte = ttk.Frame(cadre, style="Encadre.TFrame", padding=(18, 16, 18, 14))
        carte.pack(fill="x")
        haut = ttk.Frame(carte, style="EncadreInterieur.TFrame")
        haut.pack(fill="x")
        ttk.Label(haut, text="CE QUE ÇA VA CHANGER", style="EncadreEtiquette.TLabel").pack(
            side="left")
        self.etat = ttk.Label(haut, text="", style="EncadreSecond.TLabel")
        self.etat.pack(side="right")
        self.changements = tk.Canvas(carte, height=40, highlightthickness=0, borderwidth=0,
                                     background=C["champ"])
        self.changements.pack(fill="x", pady=(10, 0))
        self.note = ttk.Label(carte, text="", style="EncadreSecond.TLabel", justify="left")
        self.note.pack(fill="x", pady=(8, 0))
        ttk.Label(cadre, text="DERNIÈRES ÉTAPES", style="Etiquette.TLabel").pack(
            anchor="w", padx=6, pady=(16, 6))
        self.liste = tk.Canvas(cadre, height=60, highlightthickness=0, borderwidth=0,
                               background=C["papier"])
        self.liste.pack(fill="both", expand=True)
        self.changements.bind("<Configure>", lambda _e: self._dessiner_changements(), add="+")
        self.liste.bind("<Configure>", lambda _e: self._dessiner_etapes(), add="+")
        self.note.bind("<Configure>", lambda e: self.note.configure(
            wraplength=max(120, e.width - 4)), add="+")
        self.liste.bind("<Motion>", self._survol)
        self.liste.bind("<Leave>", lambda _e: self._survol(None))
        self.liste.bind("<Button-1>", self._clic)
        self._zones = []
        self._survolee = None
        self.montrer_prevu([], "Choisir une action : ce qu’elle changera s’affiche ici.")

    # ----- Ce que ça va changer -----

    def etat_du_texte(self):
        return str(self.etat.cget("text"))

    def montrer_prevu(self, lignes, note="Rien n’est écrit avant « Faire »."):
        self.lignes = list(lignes)
        self.texte_du_moteur = ""
        self.etat.configure(text="prévu" if lignes else "")
        self.note.configure(text=note)
        self._dessiner_changements()

    def montrer_moteur(self, texte):
        """Aperçu du moteur d'une opération critique, dans le vocabulaire de l'interface."""
        self.texte_du_moteur = texte
        self.etat.configure(text="à confirmer")
        # Les identifiants complets entre parenthèses sont retirés : le nom suffit ici.
        sans_identifiant = re.sub(r" \([0-9a-f]{8}-[0-9a-f-]{27}\)", "", texte)
        lignes = [l.strip()[2:] if l.strip().startswith("- ") else l.strip()
                  for l in sans_identifiant.splitlines()[1:] if l.strip()]
        self.note.configure(text="Avant de continuer :\n" + "\n".join(f"• {l}" for l in lignes))

    def montrer_fait(self, lignes):
        self.lignes = list(lignes)
        self.texte_du_moteur = ""
        self.etat.configure(text="fait")
        self.note.configure(text="C’est enregistré : une nouvelle étape, qu’on peut annuler.")
        self._dessiner_changements()

    def _pastille(self, signe):
        couleur = COULEUR_DU_SIGNE.get(signe, NEUTRE)
        cle = ("signe", couleur)
        if cle not in self._images:
            self._images[cle] = photo(self.cadre, dessin.forme(26, 26, 13, rgba(couleur, .14)),
                                      C["champ"])
        return self._images[cle]

    def _dessiner_changements(self):
        c = self.changements
        c.delete("all")
        largeur = max(160, c.winfo_width())
        police = tkfont.nametofont("TkDefaultFont", root=c)
        y = 0
        if not self.lignes:
            c.configure(height=1)
            return
        for signe, quoi, objet in self.lignes:
            c.create_image(0, y, image=self._pastille(signe), anchor="nw")
            c.create_text(13, y + 13, text=signe, fill=TEXTE_DU_SIGNE.get(signe, C["encre"]),
                          font="MemoireMonoPetite")
            c.create_text(38, y + 2, text=quoi, anchor="nw", fill=C["texte_second"],
                          font="MemoirePetite", width=largeur - 40)
            haut = y + 2 + police.metrics("linespace") - 2
            if objet:
                texte = c.create_text(38, haut, text=objet, anchor="nw", fill=C["encre"],
                                      font="MemoireGras", width=largeur - 40)
                y = max(y + 30, c.bbox(texte)[3] + 8)
            else:
                y += 32
        c.configure(height=y)

    # ----- Dernières étapes -----

    def montrer_etapes(self, etapes):
        self.etapes = list(etapes)[:ETAPES_MONTREES]
        self._dessiner_etapes()

    def libelles_des_etapes(self):
        return [libelle_d_etape(e.commande) for e in self.etapes]

    def _dessiner_etapes(self):
        c = self.liste
        c.delete("all")
        self._zones = []
        largeur = max(160, c.winfo_width())
        y = 0
        mono = tkfont.nametofont("MemoireMonoPetite", root=c)
        puce_l = mono.measure("00000000") + 14
        cle = ("puce", puce_l)
        if cle not in self._images:
            self._images[cle] = photo(c, dessin.forme(puce_l, 22, 11, rgba(C["indigo"], .10)),
                                      C["papier"])
        survol = ("survol", largeur)
        if survol not in self._images:
            self._images[survol] = photo(c, dessin.forme(largeur, 46, 14, rgba("#FFFFFF", .7)),
                                         C["papier"])
        for rang, etape in enumerate(self.etapes):
            haut = y
            fond_img = self._images[survol]
            fond = c.create_image(0, y, image=fond_img, anchor="nw", state="hidden")
            c.create_image(6, y + 6, image=self._images[cle], anchor="nw")
            c.create_text(6 + puce_l / 2, y + 17, text=etape.identifiant[:8],
                          fill="#2A26A8", font="MemoireMonoPetite")
            libelle = c.create_text(puce_l + 16, y + 7, text=libelle_d_etape(etape.commande),
                                    anchor="nw", fill=C["encre"], font="TkDefaultFont",
                                    width=largeur - puce_l - 22)
            bas = c.bbox(libelle)[3]
            if etape.motif:
                motif = c.create_text(puce_l + 16, bas + 1, text=etape.motif, anchor="nw",
                                      fill=C["texte_second"], font="MemoirePetite",
                                      width=largeur - puce_l - 22)
                bas = c.bbox(motif)[3]
            y = max(y + 36, bas + 10)
            self._zones.append((haut, y, rang, fond))
        c.configure(height=max(40, y))

    def _survol(self, evenement):
        zone = None
        if evenement is not None:
            zone = next((z for z in self._zones if z[0] <= evenement.y < z[1]), None)
        if zone is self._survolee:
            return
        if self._survolee is not None:
            self.liste.itemconfigure(self._survolee[3], state="hidden")
        self._survolee = zone
        if zone is not None:
            self.liste.itemconfigure(zone[3], state="normal")
        self.liste.configure(cursor="hand2" if zone is not None else "")

    def _clic(self, evenement):
        zone = next((z for z in self._zones if z[0] <= evenement.y < z[1]), None)
        if zone is not None:
            self.choisir_etape(zone[2])

    def choisir_etape(self, rang):
        """« Revenir à une étape », pré-rempli avec cette étape."""
        return self.app.ouvrir_action("revenir_a_une_etape",
                                      {"etape": self.etapes[rang].identifiant})
