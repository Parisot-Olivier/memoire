"""Une tuile de la console : un terminal (TERMINAL.md, sections 1 à 4 ; CONSOLE.md, section 4 ;
D27, D32 ; INV-TERM-01).

Vue Tkinter du moteur (gui/terminal_moteur.py) : en-tête (état, nom, ranger, agrandir, fermer),
sortie, ligne de saisie avec historique et complétion, interruption. Le shell est lancé à la
première fois que la tuile est montrée, et survit à tout changement de place (le cadre n'est jamais
recréé).
"""
import os
import threading
import time
import tkinter as tk
import tkinter.font as tkfont
from tkinter import ttk

from apparence import COULEURS as C
from notification import a_notifier, texte_de_fin
from menus import Carte, Ligne
from terminal_moteur import (Historique, Session, SuiviDesCommandes, Tampon, _dernier_mot,
                             completer, environnement, geometrie)
from terminaux import EtatDuTerminal, etat_court, nom_court
from theme import theme_de

PERIODE_DE_LECTURE_MS = 30
# La taille d'une tuile change à chaque pixel pendant qu'on tire le point milieu : elle n'est
# transmise au shell qu'une fois le geste calmé.
DELAI_DE_TAILLE_MS = 120
STYLES_D_ETAT = {"echec": "EchecDeTuile.TLabel"}


class PanneauTerminal:
    """Un terminal de la console, tuile `nom` (« terminal-N », TERMINAUX.md) ; `app` fournit
    racine, base, exécutable, terminaux (la gestion) et notifier ; le shell démarre dans `dossier`,
    sinon dans le répertoire courant."""

    def __init__(self, cadre, app, nom="terminal-1", dossier=None):
        self.app = app
        self.nom = nom
        self.dossier_de_depart = dossier
        self.dernier_code = None
        self.cadre = cadre
        self.session = None
        self.tampon = Tampon()
        self.historique = Historique()
        self.suivi = SuiviDesCommandes()
        self.fins = []
        # « ● terminé, code N » quand une commande longue a fini hors de la vue, jusqu'au retour.
        self.marque = ""
        self.nom_affiche = ""
        self.etat_affiche = ""
        self._lecture = None
        self._taille = (100, 30)
        self._envoi_de_taille = None
        self._fin_annoncee = False
        self._agrandi = False
        self.police = tkfont.nametofont("TkFixedFont")
        self.theme = theme_de(cadre)
        # Apparence (APPARENCE.md, section 4) : tuile sombre, encre indigo. Les cadres sont des
        # cadres classiques au fond du terminal (CONSOLE.md, section 11) : le serveur peint ce
        # fond sous les widgets ttk qu'ils portent, avant que Tk les redessine ; un cadre ttk y
        # laisserait voir le papier pendant un redimensionnement.
        nuit = C["terminal"]
        survol = self.theme.melange(nuit, "#FFFFFF", .14)
        # Le contenu reste à 18 pixels des bords : hors des quatre coins arrondis de la tuile.
        corps = tk.Frame(cadre, background=nuit, borderwidth=0, highlightthickness=0)
        corps.pack(fill="both", expand=True, padx=16, pady=(8, 10))
        # En-tête : point d'état, nom (D29), état court, puis ranger, agrandir, fermer.
        tete = tk.Frame(corps, background=nuit, borderwidth=0, highlightthickness=0)
        tete.pack(fill="x", pady=(0, 4))
        tete.columnconfigure(1, weight=1)
        self.point = ttk.Label(tete, style="Sombre.TLabel")
        self.point.grid(row=0, column=0, padx=(2, 8))
        self.etiquette = ttk.Label(tete, style="NomDeTuile.TLabel", anchor="w", width=1)
        self.etiquette.grid(row=0, column=1, sticky="ew")
        self.etiquette_d_etat = ttk.Label(tete, style="EtatDeTuile.TLabel")
        self.etiquette_d_etat.grid(row=0, column=2, padx=(8, 6))
        self._icones = {icone: (self.theme.icone(icone, 16, C["terminal_icone"], fond=nuit),
                                "active", self.theme.icone(icone, 16, "#FFFFFF", fond=survol))
                        for icone in ("ranger", "agrandir", "egaliser", "fermer")}
        self.boutons = {}
        for colonne, (cle, action) in enumerate((("ranger", self.ranger),
                                                 ("agrandir", self.agrandir),
                                                 ("fermer", self.fermer)), start=3):
            bouton = ttk.Button(tete, style="Tuile.TButton", image=self._icones[cle],
                                command=action, takefocus=False)
            bouton.grid(row=0, column=colonne)
            self.boutons[cle] = bouton
        self.etiquette.bind("<Button-1>", lambda _e: self.saisie.focus_set())
        self.etiquette.bind("<Double-Button-1>", lambda _e: self.agrandir())
        self.etiquette.bind("<Configure>", lambda _e: self._ecrire_le_nom(), add="+")
        self.sortie = tk.Text(corps, wrap="char", font=self.police, height=3, width=20,
                              state="disabled", background=nuit, foreground="#E9E4F5",
                              insertbackground="#E9E4F5", selectbackground="#4A4380",
                              selectforeground="#FFFFFF", borderwidth=0, highlightthickness=0,
                              padx=2, pady=4)
        self.sortie.bind("<Configure>", self._ajuster_la_taille)
        # Cliquer dans la sortie rend la tuile active sans prendre le curseur : on peut y choisir
        # du texte et le copier.
        self.sortie.bind("<Button-1>", lambda _e: self._activer(), add="+")
        # La saisie garde sa place en bas, même dans une tuile basse : c'est la sortie qui
        # rétrécit. À sa droite, un seul bouton : « Interrompre » pendant une commande, « Effacer »
        # sinon.
        rangee = tk.Frame(corps, background=nuit, borderwidth=0, highlightthickness=0)
        ttk.Label(rangee, text="$", style="Invite.TLabel").pack(side="left")
        self.bouton_action = ttk.Button(rangee, text="Effacer", style="Sombre.TButton",
                                        command=self._action, takefocus=False)
        self.bouton_action.pack(side="right", padx=(8, 0))
        self.ligne = tk.StringVar()
        self.saisie = ttk.Entry(rangee, textvariable=self.ligne, font=self.police,
                                style="Sombre.TEntry", width=4)
        self.saisie.pack(side="left", fill="x", expand=True, padx=(6, 0))
        rangee.pack(side="bottom", fill="x", pady=(6, 0))
        self.sortie.pack(fill="both", expand=True)
        # Complétion par Tab (APPARENCE.md, section 11) : carte des possibilités au second Tab.
        self.completions = Carte(self.saisie, modale=False, sombre=True)
        self._candidats = []
        self._ligne_au_dernier_tab = None
        self.saisie.bind("<Tab>", lambda _e: self.completer_la_saisie() or "break")
        self.saisie.bind("<Return>", lambda _e: self._entree())
        self.saisie.bind("<Up>", lambda _e: self._fleche(-1))
        self.saisie.bind("<Down>", lambda _e: self._fleche(1))
        self.saisie.bind("<Escape>", lambda _e: self.completions.fermer())
        self.saisie.bind("<KeyRelease>", self._touche, add="+")
        self._fermeture_des_completions = None
        self.saisie.bind("<FocusOut>", lambda _e: self._fermer_bientot_les_completions(), add="+")
        self.saisie.bind("<Destroy>", lambda _e: self.completions.detruire(), add="+")
        self.saisie.bind("<Control-c>", lambda _e: self.interrompre() or "break")
        # Ctrl+L efface la sortie, comme dans un shell.
        self.saisie.bind("<Control-l>", lambda _e: self.effacer() or "break")
        self.saisie.bind("<FocusIn>", lambda _e: self._revenu())
        cadre.bind("<Destroy>", self._detruit, add="+")
        self.actualiser(nom, self.etat(), True)

    # ----- En-tête de la tuile -----

    @property
    def seuil(self):
        """Seuil de notification : celui de la console (CONSOLE.md, section 4)."""
        return self.app.terminaux.seuil

    @seuil.setter
    def seuil(self, valeur):
        self.app.terminaux.seuil = valeur

    def actualiser(self, titre, etat, battement, maintenant=None):
        """Met l'en-tête en accord avec le terminal : nom, état court, point d'état (qui pulse
        pendant une commande), bouton « Interrompre » ou « Effacer »."""
        texte, genre = etat_court(etat, time.monotonic() if maintenant is None else maintenant)
        style = STYLES_D_ETAT.get(genre, "EtatDeTuile.TLabel")
        if self.marque:
            texte, style = self.marque, "MarqueDeTuile.TLabel"
        if titre != self.nom_affiche:
            self.nom_affiche = titre
            self._ecrire_le_nom()
        if (texte, style) != (self.etat_affiche, str(self.etiquette_d_etat.cget("style"))):
            self.etat_affiche = texte
            self.etiquette_d_etat.configure(text=texte, style=style)
        nuit = C["terminal"]
        if genre == "cours":
            image = self.theme.point(C["terminal_vert"], nuit, opacite=1.0 if battement else .35)
        elif genre == "echec":
            image = self.theme.point(C["terminal_echec"], nuit)
        else:
            image = self.theme.point(C["terminal_second"], nuit, anneau=True)
        if str(self.point.cget("image")) != str(image):
            self.point.configure(image=image)
        action = "Interrompre" if genre == "cours" else "Effacer"
        if str(self.bouton_action.cget("text")) != action:
            self.bouton_action.configure(text=action)

    def _ecrire_le_nom(self):
        """Le nom, raccourci par « … » s'il dépasse la place."""
        texte, largeur = self.nom_affiche, self.etiquette.winfo_width() - 6
        police = tkfont.nametofont("MemoireMonoPetite")
        if largeur > 10 and police.measure(texte) > largeur:
            for longueur in range(len(texte) - 1, 3, -1):
                texte = nom_court(self.nom_affiche, longueur)
                if police.measure(texte) <= largeur:
                    break
        if str(self.etiquette.cget("text")) != texte:
            self.etiquette.configure(text=texte)

    def montrer_agrandir(self, possible, agrandi):
        """Le bouton « agrandir » n'existe qu'à plusieurs ; agrandie, la tuile propose le retour
        à des parts égales."""
        bouton = self.boutons["agrandir"]
        if possible and not bouton.winfo_manager():
            bouton.grid()
        elif not possible and bouton.winfo_manager():
            bouton.grid_remove()
        if agrandi != self._agrandi:
            self._agrandi = agrandi
            bouton.configure(image=self._icones["egaliser" if agrandi else "agrandir"])

    def ranger(self):
        """Passe ce terminal en arrière-plan : il continue de tourner."""
        self.app.terminaux.ranger(self.nom)

    def agrandir(self):
        """Donne à cette tuile les deux tiers de la page ; une seconde fois, parts égales."""
        self.app.terminaux.agrandir(self.nom)

    def fermer(self):
        """Ferme ce terminal (confirmation si une commande tourne)."""
        return self.app.terminaux.fermer(self.nom)

    def _activer(self):
        gestion = getattr(self.app, "terminaux", None)
        if gestion is not None:
            gestion.activer(self.nom)

    def _revenu(self):
        """Le curseur est dans la saisie : la tuile est active, la marque de fin est vue."""
        self.marque = ""
        self._activer()

    def _action(self):
        if str(self.bouton_action.cget("text")) == "Interrompre":
            self.interrompre()
        else:
            self.effacer()

    # ----- Complétion par Tab -----

    def completer_la_saisie(self):
        """Tab : complète ce qui est sans ambiguïté ; un second Tab montre les possibilités."""
        ligne = self.ligne.get()
        dossier = self.tampon.dossier or self.dossier_de_depart or os.getcwd()
        resultat = completer(ligne, dossier)
        self._candidats = resultat.candidats
        if resultat.ligne != ligne:
            self.ligne.set(resultat.ligne)
            self.saisie.icursor("end")
            self.completions.fermer()
            self._ligne_au_dernier_tab = None
            return "break"
        if len(resultat.candidats) > 1 and self._ligne_au_dernier_tab == ligne:
            self.completions.montrer(
                [Ligne(c) for c in resultat.candidats], self.saisie.winfo_rootx(),
                self.saisie.winfo_rooty() + self.saisie.winfo_height() + 4,
                largeur_min=min(260, max(160, self.saisie.winfo_width())),
                quand_choisi=self._choisir_la_completion, rang=0,
                au_dessus=self.saisie.winfo_rooty())
        self._ligne_au_dernier_tab = ligne
        return "break"

    def _choisir_la_completion(self, rang):
        avant, mot = _dernier_mot(self.ligne.get())
        tete = mot.rpartition("/")[0] + "/" if "/" in mot else ""
        candidat = self._candidats[rang]
        nom = candidat.replace(" ", "\\ ")
        self.ligne.set(avant + tete + nom + ("" if candidat.endswith("/") else " "))
        self.saisie.icursor("end")
        self.saisie.focus_set()
        self._ligne_au_dernier_tab = None

    def _fermer_bientot_les_completions(self):
        """Le curseur a quitté la saisie : la carte se ferme, après le clic qui l'aurait choisie."""
        def fermer():
            self._fermeture_des_completions = None
            self.completions.fermer()
        if self._fermeture_des_completions is None:
            self._fermeture_des_completions = self.cadre.after(150, fermer)

    def _entree(self):
        if self.completions.visible():
            self.completions.valider()
            return "break"
        return self.executer(self.ligne.get())

    def _fleche(self, pas):
        if self.completions.visible():
            self.completions.deplacer(pas)
            return "break"
        return self._rappeler(self.historique.precedente(self.ligne.get()) if pas < 0
                              else self.historique.suivante())

    def _touche(self, evenement):
        """Toute frappe autre que Tab ou les flèches referme les possibilités."""
        if evenement.keysym not in ("Tab", "Up", "Down", "Return", "Shift_L", "Shift_R"):
            self.completions.fermer()
            self._ligne_au_dernier_tab = None

    # ----- Shell -----

    def quand_visible(self):
        """La tuile est montrée : le shell démarre la première fois."""
        if self.session is None:
            self.demarrer()

    def demarrer(self):
        executable = self.app.memoire
        if os.path.dirname(executable):
            executable = os.path.abspath(executable)
        env = environnement(self.app.base.get(), executable, os.environ)
        shell = os.environ.get("SHELL") or "/bin/bash"
        colonnes, lignes = self._taille
        dossier = self.dossier_de_depart
        if not dossier or not os.path.isdir(dossier):
            dossier = os.getcwd()
        self.session = Session(shell, dossier, env, colonnes=colonnes, lignes=lignes)
        self.session.demarrer()
        self._fin_annoncee = False
        if not self.session.suivi_des_fins:
            self._ajouter_texte("[fin de commande non suivie pour ce shell]\n")
        self._planifier()

    def _planifier(self):
        self._lecture = self.cadre.after(PERIODE_DE_LECTURE_MS, self._lire)

    def _lire(self):
        self._lecture = None
        if self.session is None:
            return
        donnees = self.session.lire_disponible()
        if donnees:
            changements = self.tampon.ajouter(donnees)
            self._appliquer(changements)
            for code in changements.fins:
                self.dernier_code = code
                fin = self.suivi.terminee(code, time.monotonic())
                if fin is not None:
                    self._fin(fin)
        if self.session.descripteur is not None and not self.session.en_vie():
            if not self._fin_annoncee:
                self._fin_annoncee = True
                self._ajouter_texte(f"\n[shell terminé, code {self.session.code_de_sortie}] "
                                    "— Entrée pour relancer\n")
            return None
        try:
            self.saisie.configure(show="" if self.session.echo_actif() else "•")
        except OSError:
            pass
        self._planifier()

    def _fin(self, fin):
        self.fins.append(fin)
        regarde = self.regarde()
        if a_notifier(fin, self.seuil, regarde):
            titre, corps = texte_de_fin(fin)
            threading.Thread(target=self.app.notifier, args=(titre, corps), daemon=True).start()
            self.marque = f"● terminé, code {fin.code}"

    def regarde(self):
        """La tuile est montrée et sa fenêtre a le focus."""
        if not self.app.terminaux.montre(self.nom):
            return False
        focus = self.app.racine.focus_get()
        return focus is not None and str(focus.winfo_toplevel()) == str(self.cadre.winfo_toplevel())

    def etat(self):
        """Ce que l'on sait du terminal, pour son nom et sa ligne dans la liste (D29)."""
        shell = os.environ.get("SHELL") or "/bin/bash"
        dossier = self.tampon.dossier or self.dossier_de_depart
        termine = None
        if self.session is not None and self.session.code_de_sortie is not None:
            termine = self.session.code_de_sortie
        en_cours = self.suivi.en_cours() if self.session is not None else None
        commande, debut = en_cours if en_cours else (None, None)
        return EtatDuTerminal(shell=shell, dossier=dossier, commande=commande, debut=debut,
                              dernier_code=self.dernier_code, termine=termine)

    def executer(self, texte):
        if self.session is None or not self.session.en_vie():
            if self.session is not None:
                self.session.arreter()
            self.demarrer()
        # Une taille en attente d'envoi part avant la commande : elle s'exécute à la bonne taille.
        if self._envoi_de_taille is not None:
            self.cadre.after_cancel(self._envoi_de_taille)
            self._transmettre_la_taille()
        if self.session.echo_actif():
            self.historique.ajouter(texte)
            self.suivi.envoyee(texte, time.monotonic())
        self.session.ecrire(texte)
        self.ligne.set("")
        return "break"

    def interrompre(self):
        if self.session is not None and self.session.en_vie():
            self.session.interrompre()

    def arreter(self):
        for attente in (self._lecture, self._envoi_de_taille, self._fermeture_des_completions):
            if attente is not None:
                try:
                    self.cadre.after_cancel(attente)
                except tk.TclError:
                    pass
        self._lecture = self._envoi_de_taille = self._fermeture_des_completions = None
        if self.session is not None:
            self.session.arreter()

    def _detruit(self, evenement):
        if evenement.widget is self.cadre:
            self.arreter()

    # ----- Affichage -----

    def texte(self):
        return self.sortie.get("1.0", "end-1c")

    def effacer(self):
        self.tampon = Tampon()
        self.sortie.configure(state="normal")
        self.sortie.delete("1.0", "end")
        self.sortie.configure(state="disabled")

    def _appliquer(self, changements):
        """Remplace la ligne en cours par les lignes terminées puis la nouvelle ligne en cours,
        retire les lignes de tête en excès (Changements, gui/terminal_moteur.py)."""
        self.sortie.configure(state="normal")
        self.sortie.delete("end-1c linestart", "end-1c")
        self.sortie.insert("end-1c", "\n".join(changements.ajoutees + [changements.courante]))
        if changements.retirees:
            self.sortie.delete("1.0", f"{changements.retirees + 1}.0")
        self.sortie.configure(state="disabled")
        self.sortie.see("end")

    def _ajouter_texte(self, texte):
        self._appliquer(self.tampon.ajouter(texte.encode("utf-8")))

    def _rappeler(self, commande):
        self.ligne.set(commande)
        self.saisie.icursor("end")
        return "break"

    def _ajuster_la_taille(self, evenement):
        """La taille de la zone de sortie, en caractères, est transmise au shell, une fois le
        redimensionnement calmé."""
        taille = geometrie(evenement.width, evenement.height, self.police.measure("0"),
                           self.police.metrics("linespace"))
        if taille == self._taille:
            return
        self._taille = taille
        # La fin de la sortie reste visible quand la tuile change de taille.
        self.sortie.see("end")
        if self._envoi_de_taille is None:
            self._envoi_de_taille = self.cadre.after(DELAI_DE_TAILLE_MS, self._transmettre_la_taille)

    def _transmettre_la_taille(self):
        self._envoi_de_taille = None
        if self.session is not None and self.session.en_vie():
            self.session.redimensionner(*self._taille)
