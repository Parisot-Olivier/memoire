#!/usr/bin/env python3
"""Interface graphique de memoire (INTERFACE.md ; D17, D30).

À gauche, le menu des actions, rangées par sujet et dites simplement (gui/actions.py) ; à droite, le
formulaire de l'action choisie (gui/formulaire.py) et la sortie de l'exécutable. Le mode expert
garde un formulaire par commande de `memoire aide`. Aucune règle ici : tout passe par l'exécutable.
"""
import argparse
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tkinter as tk
from tkinter import filedialog, messagebox, ttk

sys.path.insert(0, str(Path(__file__).resolve().parent))
from champs import liste_de_notes  # noqa: E402
import theme  # noqa: E402
from notification import envoyer  # noqa: E402
from panneaux import PlanDeTravail  # noqa: E402
from plan_de_travail import chemin_de_configuration  # noqa: E402
from schema import (arguments, est_dossier, est_multiple, est_reference, identifiants,  # noqa: E402
                    lire_aide)
from gestion_des_terminaux import GestionDesTerminaux  # noqa: E402
from actions import GROUPES  # noqa: E402
import apparence  # noqa: E402
import formulaire  # noqa: E402
from habillage import MODE_EXPERT, Habillage, ListeDesActions, Recherche  # noqa: E402
from apercu import PanneauApercu  # noqa: E402
from voisinage import PanneauVoisinage  # noqa: E402
from actions import lire_historique  # noqa: E402
from plan_de_travail import Defaut  # noqa: E402
from visionneur import interroger  # noqa: E402

CONFIRMATION_REQUISE = 4
ERREUR_DE_STOCKAGE = 3
# Indication donnée par l'exécutable pour une base de schéma 1 (ETAT-HISTORIQUE.md, section 16).
MIGRATION_PROPOSEE = "memoire migrer"


def lancer(memoire, base, args):
    """Lance l'exécutable ; rend le code de retour et le texte à afficher."""
    ligne = [memoire, "--base", base, *args]
    resultat = subprocess.run(ligne, capture_output=True, text=True, encoding="utf-8")
    texte = f"$ memoire {shlex.join(args)}\n{resultat.stdout}{resultat.stderr}[code {resultat.returncode}]\n\n"
    return resultat.returncode, texte


class Application:
    def __init__(self, racine, memoire, base):
        self.racine = racine
        self.memoire = memoire
        # Apparence (APPARENCE.md, D31) : thème appliqué avant tout widget.
        self.theme = theme.appliquer(racine)
        aide = subprocess.run([memoire, "aide"], capture_output=True, text=True, encoding="utf-8")
        if aide.returncode != 0:
            raise RuntimeError(f"« memoire aide » a échoué : {aide.stderr}")
        self.commandes = lire_aide(aide.stdout)
        self.courante = None
        # Question oui / non ; remplaçable dans les essais.
        self.demander = messagebox.askyesno
        self.valeurs = {}
        self.a_confirmer = None
        # Notes choisies par leur nom : id8 → identifiant complet (RETROUVER.md, section 1).
        self.notes_connues = {}
        racine.title("memoire — mémoire de conception")
        racine.geometry("1400x860")
        racine.minsize(1040, 660)
        self.base = tk.StringVar(value=base)
        self.notifier = envoyer
        self.etat = apparence.Etat("", 0, 0)
        self.sujet_courant = GROUPES[0]
        self._relecture = None
        # Apparence (APPARENCE.md, section 3) : fond, en-tête, rail, carte, barre d'état ; le plan
        # de travail (PLAN-DE-TRAVAIL.md, D28) est dans la carte.
        self.habillage = Habillage(self)
        self.plan = PlanDeTravail(racine, chemin_de_configuration(os.environ), signaler=self.signaler,
                                  parent=self.habillage.carte)
        centre = self.plan.centre
        # Les actions du sujet choisi (ACTIONS.md, section 4 ; D30) ; le rail choisit le sujet.
        self.liste = ListeDesActions(centre, self)
        self.liste.canevas.pack(side="left", fill="y")
        self._separation = tk.Frame(centre, width=1, background=apparence.COULEURS["separation"])
        self._separation.pack(side="left", fill="y", pady=18)
        self.action_courante = None

        # À droite : l'action (qui défile si elle est longue), puis le journal ; la séparation
        # entre les deux se déplace.
        self.colonne = ttk.PanedWindow(centre, orient="vertical")
        self.colonne.pack(side="left", fill="both", expand=True)
        # La lecture d'une note prend la place de l'action (APPARENCE.md, section 9.3).
        self.zone_de_lecture = ttk.Frame(centre)
        self.lecture = None
        self.vue_de_la_carte = "action"
        # Panneaux de chaque vue (APPARENCE.md, section 10) : l'Aperçu pour les actions, le
        # Voisinage pour la lecture ; le choix de l'utilisateur dans chaque vue est retenu.
        self._apercu_voulu = True
        self._voisinage_voulu = True
        defilante = theme.ZoneDefilante(self.colonne)
        self.colonne.add(defilante, weight=4)
        droite = defilante.interieur
        droite.configure(padding=(30, 22, 26, 10))
        self.titre = ttk.Label(droite, text="Choisir une action à gauche", style="Titre.TLabel")
        self.titre.pack(anchor="w")
        self.resume = ttk.Label(droite, text="", wraplength=760, justify="left", style="Second.TLabel")
        self.resume.pack(anchor="w", pady=(4, 10))
        droite.bind("<Configure>", lambda e: self.resume.configure(
            wraplength=max(300, e.width - 60)), add="+")
        self.formulaire = ttk.Frame(droite)
        self.formulaire.pack(fill="x")
        actions = ttk.Frame(droite)
        actions.pack(fill="x", pady=(10, 0))
        self.bouton_executer = ttk.Button(actions, text="Exécuter", command=self.executer, state="disabled",
                                          style="Action.TButton")
        self.bouton_executer.pack(side="left")
        self.bouton_confirmer = ttk.Button(actions, text="Confirmer (--oui)", command=self.confirmer, state="disabled")
        self.bouton_confirmer.pack(side="left", padx=10)
        # Les boutons du mode expert n'apparaissent qu'avec une commande choisie.
        self.bouton_executer.pack_forget()
        self.bouton_confirmer.pack_forget()
        bas = ttk.Frame(self.colonne, padding=(30, 8, 26, 12))
        self.colonne.add(bas, weight=1)
        journal = ttk.Frame(bas)
        journal.pack(fill="x", pady=(0, 6))
        # Les boutons sont placés d'abord : quand la colonne est étroite, c'est l'indication qui
        # cède la place, pas eux.
        ttk.Button(journal, text="Effacer", style="Bandeau.TButton",
                   command=lambda: self.sortie.delete("1.0", "end")).pack(side="right")
        # Agrandir : le journal prend la plus grande part ; Réduire : la taille d'avant.
        self.bouton_journal = ttk.Button(journal, text="Agrandir", style="Bandeau.TButton",
                                         command=self.basculer_journal)
        self.bouton_journal.pack(side="right", padx=6)
        ttk.Label(journal, text="JOURNAL", style="Etiquette.TLabel").pack(side="left")
        ttk.Label(journal, text="tirer la poignée", style="Second.TLabel",
                  font="MemoirePetite").pack(side="left", padx=10)
        self._journal_avant = None
        self.sortie = theme.ZoneDeTexte(bas, height=6, wrap="word", font="MemoireMono")
        self.sortie.pack(fill="both", expand=True)
        self._journal_place = False
        self.colonne.bind("<Configure>", self._placer_le_journal, add="+")
        self.afficher_reprise()
        # Terminaux intégrés (TERMINAL.md, TERMINAUX.md, CONSOLE.md ; D27, D29, D32) : la console,
        # un seul panneau, ancrée en haut à droite par défaut ; ses tuiles sont les terminaux.
        self._apercu_attente = None
        self._formulaire_de_l_apercu = None
        # Panneau « Aperçu » (APPARENCE.md, section 9.3) : à droite, sous les terminaux (rang 99),
        # montré au premier lancement.
        self.plan.ajouter("apercu", "Aperçu", Defaut(zone="droite", rang=99, part=0.3),
                          lambda cadre: PanneauApercu(cadre, self), visible_d_abord=True)
        # Panneau « Voisinage » : il suit la note lue dans la carte (8.2c).
        self.plan.ajouter("voisinage", "Voisinage", Defaut(zone="droite", rang=98, part=0.45),
                          lambda cadre: PanneauVoisinage(cadre, self))
        self.terminaux = GestionDesTerminaux(self)
        self.plan.demarrer(racine)
        # Au démarrage, vue des actions : le Voisinage attend la première lecture.
        if self.plan.visible("voisinage"):
            self.plan.masquer("voisinage")
        self.terminaux.demarrer(racine)
        # Le bouton rond de la console suit sa visibilité (pastille de papier quand on la voit).
        self._console_montree = self.plan.visible("console")
        self.plan.quand_applique.append(self._suivre_la_console)
        self.recherche = Recherche(self)
        self.liste.afficher(self.sujet_courant)
        self.rafraichir_etat(dessiner=False)
        self.habillage.dessiner()
        self._suivi_des_terminaux = None
        self._terminaux_vus = []
        self._suivre_les_terminaux()
        self._ferme = False
        racine.protocol("WM_DELETE_WINDOW", self.fermer)
        racine.bind("<Destroy>", self._detruite, add="+")

    def _detruite(self, evenement):
        """Fenêtre détruite sans « fermer » : mise à jour annulée et shells arrêtés quand même."""
        if evenement.widget is self.racine and not self._ferme:
            self._ferme = True
            self._annuler_les_attentes()
            self.terminaux.arreter_sans_enregistrer()

    def _annuler_les_attentes(self):
        for attente in (self._relecture, self._suivi_des_terminaux,
                        getattr(self, "_apercu_attente", None)):
            if attente is not None:
                try:
                    self.racine.after_cancel(attente)
                except tk.TclError:
                    pass
        self._relecture = self._suivi_des_terminaux = self._apercu_attente = None
        self.habillage.arreter()

    # ----- Menu : rail des sujets et liste des actions (APPARENCE.md, section 3) -----

    def sujets(self):
        """Les sujets du rail, dans l'ordre : ceux du catalogue, puis le mode expert."""
        return [*GROUPES, MODE_EXPERT]

    def choisir_sujet(self, sujet):
        """Montre les actions d'un sujet (ou les commandes du mode expert) dans la carte."""
        if sujet not in self.sujets():
            raise ValueError(f"sujet inconnu : {sujet}")
        self.reduire_la_console()
        if sujet != self.sujet_courant:
            self.sujet_courant = sujet
            self.liste.afficher(sujet)
            self.habillage.redessiner()

    def actions_affichees(self):
        """Titres des actions (ou noms des commandes) de la liste, dans l'ordre."""
        return list(self.liste.titres)

    def choisir_dans_la_liste(self, rang):
        return self.liste.choisir(rang)

    def nom_du_projet(self):
        return apparence.nom_du_projet(self.base.get())

    # ----- Barre d'état -----

    def rafraichir_etat(self, dessiner=True):
        """Relit l'état du projet (`statut`, `branches --tsv` : lecture seule) et redessine."""
        self._relecture = None
        code, statut, _ = interroger(self.memoire, self.base.get(), ["statut"])
        code_b, branches, _ = interroger(self.memoire, self.base.get(), ["branches", "--tsv"])
        self.etat = apparence.lire_etat(statut if code == 0 else "", branches if code_b == 0 else "")
        apercu = self.apercu
        if apercu is not None:
            code_h, historique, _ = interroger(self.memoire, self.base.get(),
                                               ["historique", "--nombre", "5"])
            apercu.montrer_etapes(lire_historique(historique) if code_h == 0 else [])
        self.racine.title(f"memoire — {self.nom_du_projet()}")
        if dessiner:
            self.habillage.redessiner()
        return self.etat

    def _etat_a_relire(self):
        """Après chaque commande : relecture de l'état dès que l'interface est au repos."""
        if self._relecture is None and not getattr(self, "_ferme", False):
            self._relecture = self.racine.after_idle(self.rafraichir_etat)

    # ----- Panneau « Aperçu » -----

    @property
    def apercu(self):
        plan = getattr(self, "plan", None)
        if plan is None or "apercu" not in plan.panneaux or plan.panneaux["apercu"].contenu is None:
            return None
        return plan.contenu("apercu")

    def apercu_a_jour(self, formulaire):
        """Le formulaire a changé : « Ce que ça va changer » sera recalculé au repos."""
        self._formulaire_de_l_apercu = formulaire
        if self._apercu_attente is None and not getattr(self, "_ferme", False):
            self._apercu_attente = self.racine.after_idle(self._apercu_maintenant)

    def _apercu_maintenant(self):
        self._apercu_attente = None
        formulaire = self._formulaire_de_l_apercu
        if self.apercu is None or formulaire is None or formulaire is not self.action_courante:
            return
        if formulaire._a_confirmer:
            return
        self.apercu.montrer_prevu(formulaire.apercu_prevu())

    def apercu_du_moteur(self, texte):
        if self.apercu is not None:
            self.apercu.montrer_moteur(texte)

    def apercu_fait(self, lignes):
        if self.apercu is not None:
            self.apercu.montrer_fait(lignes)

    def textes_de_la_barre(self):
        return self.habillage.textes_de_la_barre()

    def terminaux_en_cours(self):
        terminaux = getattr(self, "terminaux", None)
        return terminaux.en_cours() if terminaux is not None else []

    def _suivre_les_terminaux(self):
        """Une commande lancée ou finie dans un terminal change la barre d'état."""
        self._suivi_des_terminaux = None
        if self._ferme_ou_detruit():
            return
        try:
            vus = self.terminaux_en_cours()
        except (tk.TclError, AttributeError):
            vus = []
        if vus != self._terminaux_vus:
            self._terminaux_vus = vus
            self.habillage.redessiner()
        self._suivi_des_terminaux = self.racine.after(1000, self._suivre_les_terminaux)

    def _placer_le_journal(self, evenement):
        """Une fois, au premier affichage : le journal prend environ 190 pixels en bas."""
        if self._journal_place or evenement.height < 300:
            return
        self._journal_place = True
        try:
            self.colonne.sashpos(0, evenement.height - 190)
        except tk.TclError:
            pass

    def basculer_journal(self):
        """Agrandit le journal (environ les trois quarts de la colonne), ou rend sa taille d'avant."""
        hauteur = self.colonne.winfo_height()
        if self._journal_avant is None:
            self._journal_avant = self.colonne.sashpos(0)
            self.colonne.sashpos(0, max(120, int(hauteur * 0.22)))
            self.bouton_journal.configure(text="Réduire")
        else:
            self.colonne.sashpos(0, self._journal_avant)
            self._journal_avant = None
            self.bouton_journal.configure(text="Agrandir")

    def _ferme_ou_detruit(self):
        return getattr(self, "_ferme", False)

    def ouvrir_action(self, cle, prerempli=None):
        """Formulaire d'une action du catalogue, éventuellement pré-rempli (« Et ensuite ? »)."""
        from actions import action as action_de
        action = action_de(cle)
        self.reduire_la_console()
        self.retour_a_l_action()
        # Le rail et la liste montrent l'action ouverte, d'où qu'elle vienne.
        if action.groupe != self.sujet_courant:
            self.choisir_sujet(action.groupe)
        self.liste.marquer(action.titre)
        self.courante = None
        self.valeurs = {}
        self.titre.configure(text=action.titre)
        self.resume.configure(text=action.aide)
        # Les boutons du mode expert n'ont pas de sens pour une action : masqués.
        self.bouton_executer.pack_forget()
        self.bouton_confirmer.pack_forget()
        self.action_courante = formulaire.ouvrir(self, self.formulaire, cle, prerempli)
        return self.action_courante

    def afficher(self, texte):
        """Ajoute un texte à la zone de sortie ; l'état du projet sera relu."""
        self.sortie.insert("end", texte)
        self.sortie.see("end")
        self._etat_a_relire()

    def signaler(self, texte):
        """Message du logiciel lui-même dans la zone de sortie."""
        self.sortie.insert("end", f"[{texte}]\n")

    @property
    def terminal(self):
        """Le premier terminal ouvert, ou None s'il n'y en a plus."""
        noms = self.terminaux.noms_internes()
        return self.terminaux.terminal(noms[0]) if noms else None

    def basculer_terminal(self):
        """Bouton de l'en-tête : montre ou masque la console ; en pleine page, la réduit
        (CONSOLE.md, section 1) ; ouvre un terminal s'il n'y en a plus."""
        self.terminaux.basculer()

    def _suivre_la_console(self):
        montree = self.plan.visible("console")
        if montree != self._console_montree:
            self._console_montree = montree
            self.habillage.redessiner()

    def reduire_la_console(self):
        """Ce qu'on demande à voir dans la carte (un sujet, une action, une note) passe devant la
        console en pleine page : elle se réduit, ses terminaux restent à portée."""
        terminaux = getattr(self, "terminaux", None)
        if terminaux is not None and terminaux.console is not None and terminaux.en_page():
            terminaux.reduire()

    def fermer(self):
        """Disposition enregistrée, shell arrêté, fenêtre fermée (INV-TERM-01)."""
        if self._ferme:
            return
        # La disposition enregistrée est celle de la vue des actions.
        try:
            self.retour_a_l_action()
        except tk.TclError:
            pass
        self._ferme = True
        self._annuler_les_attentes()
        self.plan.fermer()
        self.terminaux.fermer_tout()
        try:
            self.racine.destroy()
        except tk.TclError:
            pass

    def afficher_reprise(self):
        """À l'ouverture, la dernière fiche de reprise de la branche active (FICHE-REPRISE.md,
        section 6) ; rien sans base, sans fiche ou en cas d'erreur."""
        code, texte = lancer(self.memoire, self.base.get(), ["reprise", "voir"])
        if code == 0 and "\nPoint de reprise " in texte:
            self.sortie.insert("end", texte)

    def choisir_base(self):
        chemin = filedialog.asksaveasfilename(title="Base memoire", initialfile="depot.db",
                                              confirmoverwrite=False)
        if chemin:
            self.base.set(chemin)
            self.rafraichir_etat()

    def choisir(self, commande):
        """Affiche le formulaire de la commande ; une commande sans champ s'exécute aussitôt."""
        self.courante = commande
        self.action_courante = None
        self.reduire_la_console()
        self.retour_a_l_action()
        if self.sujet_courant != MODE_EXPERT:
            self.choisir_sujet(MODE_EXPERT)
        self.liste.marquer(commande.nom)
        if self.apercu is not None:
            self.apercu.montrer_prevu([("·", "mode expert", "la commande telle quelle")],
                                      "Son compte rendu s’affiche dans le journal.")
        if not self.bouton_executer.winfo_manager():
            self.bouton_executer.pack(side="left")
            self.bouton_confirmer.pack(side="left", padx=6)
        self.titre.configure(text=commande.nom)
        self.resume.configure(text=commande.resume)
        for enfant in self.formulaire.winfo_children():
            enfant.destroy()
        self.valeurs = {}
        for ligne, champ in enumerate(commande.champs):
            libelle = champ.nom + ("" if champ.requis or champ.nature == "drapeau" else " (facultatif)")
            if champ.exclusif:
                libelle = champ.nom + " (l'un ou l'autre)"
            ttk.Label(self.formulaire, text=libelle, style="Question.TLabel").grid(
                row=ligne, column=0, sticky="w", padx=(0, 12), pady=3)
            if champ.nature == "drapeau":
                variable = tk.BooleanVar(value=False)
                ttk.Checkbutton(self.formulaire, variable=variable).grid(row=ligne, column=1, sticky="w")
            elif champ.choix:
                variable = tk.StringVar(value=champ.choix[0] if champ.requis else "")
                valeurs = list(champ.choix) if champ.requis else ["", *champ.choix]
                ttk.Combobox(self.formulaire, textvariable=variable, values=valeurs, state="readonly",
                             width=30).grid(row=ligne, column=1, sticky="w")
            elif est_reference(champ):
                variable = tk.StringVar()
                self.champ_reference(ligne, variable, est_multiple(champ))
            else:
                variable = tk.StringVar()
                ttk.Entry(self.formulaire, textvariable=variable, width=30).grid(row=ligne, column=1, sticky="we",
                                                                                pady=3)
                if champ.fichier:
                    ttk.Button(self.formulaire, text="Parcourir…",
                               command=lambda v=variable: self.choisir_fichier(v)).grid(row=ligne, column=2, padx=4)
                elif est_dossier(champ):
                    ttk.Button(self.formulaire, text="Parcourir…",
                               command=lambda v=variable: self.choisir_dossier(v)).grid(row=ligne, column=2, padx=4)
            self.valeurs[champ.nom] = variable
        self.formulaire.columnconfigure(1, weight=1)
        self.bouton_executer.configure(state="normal")
        self.bouton_confirmer.configure(state="disabled")
        if not commande.champs:
            self.executer()

    def champ_reference(self, ligne, variable, multiple=False):
        """Champ désignant une note, avec autocomplétion par nom (R3 ; gui/champs.py)."""
        liste = liste_de_notes(self.formulaire, self.memoire, self.base.get, variable, multiple,
                               connus=self.notes_connues, largeur=30)
        liste.grid(row=ligne, column=1, sticky="we", pady=3)
        ttk.Label(self.formulaire, text="un nom suffit", style="Second.TLabel").grid(
            row=ligne, column=2, padx=8)
        if not multiple:
            ttk.Button(self.formulaire, text="Voir",
                       command=lambda: self.voir(variable.get().strip())).grid(row=ligne, column=3)
            ttk.Button(self.formulaire, text="Voisinage",
                       command=lambda: self.voisinage(variable.get().strip())).grid(row=ligne, column=4)

    def voir(self, note):
        """Lit une note ou un échange dans la carte (VISIONNEUR.md, section 5 ; APPARENCE.md,
        section 9.3) ; le panneau « Voisinage » la suit."""
        if not note:
            return None
        from visionneur import Visionneur
        self.reduire_la_console()
        if self.lecture is not None:
            self.lecture.detruire()
        self.lecture = Visionneur(self, note, parent=self.zone_de_lecture)
        if self.sujet_courant != "Lire":
            self.choisir_sujet("Lire")
        self.liste.marquer("Lire une note ou un échange")
        if self.vue_de_la_carte != "lecture":
            # La lecture prend la place de la liste des actions et de l'action (maquette).
            self.liste.canevas.pack_forget()
            self._separation.pack_forget()
            self.colonne.pack_forget()
            self.zone_de_lecture.pack(side="left", fill="both", expand=True)
            self.vue_de_la_carte = "lecture"
            self._apercu_voulu = self.plan.visible("apercu")
            if self._apercu_voulu:
                self.plan.masquer("apercu")
            if self._voisinage_voulu and not self.plan.visible("voisinage"):
                self.plan.montrer("voisinage")
            self._elargir_la_droite()
        panneau = self.plan.contenu("voisinage")
        if panneau is not None:
            panneau.suivre(note)
        self.habillage.redessiner()
        return self.lecture

    def voir_dans_une_fenetre(self, note):
        from visionneur import Visionneur
        return Visionneur(self, note) if note else None

    def _elargir_la_droite(self):
        """Pendant la lecture, la colonne de droite (le voisinage) a au moins 380 pixels, comme
        la maquette ; la part choisie pour la vue des actions revient au retour (le plan de
        travail la repose depuis sa disposition)."""
        milieu = self.plan.milieu
        self.racine.update_idletasks()
        largeur = milieu.winfo_width()
        if "droite" not in self.plan.disposition.zones_visibles() or largeur < 600:
            return
        voulue = int(min(largeur * 0.42, max(380, largeur * 0.32)))
        rang = len(milieu.panes()) - 2
        try:
            if largeur - milieu.sashpos(rang) < voulue:
                milieu.sashpos(rang, largeur - voulue)
        except tk.TclError:
            pass

    def retour_a_l_action(self):
        """La carte montre de nouveau l'action (le formulaire et le journal)."""
        if self.vue_de_la_carte == "action":
            return
        if self.lecture is not None:
            self.lecture.fermer_les_cartes()
        self.zone_de_lecture.pack_forget()
        self.liste.canevas.pack(side="left", fill="y")
        self._separation.pack(side="left", fill="y", pady=18)
        self.colonne.pack(side="left", fill="both", expand=True)
        self.vue_de_la_carte = "action"
        self._voisinage_voulu = self.plan.visible("voisinage")
        if self._voisinage_voulu:
            self.plan.masquer("voisinage")
        if self._apercu_voulu and not self.plan.visible("apercu"):
            self.plan.montrer("apercu")
        self.habillage.redessiner()

    def voisinage(self, note):
        """Ouvre la vue locale d'une note (VOISINAGE.md, section 1)."""
        if not note:
            return None
        from voisinage import Voisinage
        return Voisinage(self, note)

    def voisinage_d_une_note(self):
        """Bouton du haut : l'action « Voisinage d'une note », par son nom (RETROUVER.md)."""
        return self.ouvrir_action("voisinage")

    def voir_une_note(self):
        """Bouton du haut : l'action « Lire une note ou un échange », par son nom (RETROUVER.md)."""
        return self.ouvrir_action("lire")

    def choisir_fichier(self, variable):
        chemin = filedialog.askopenfilename(title="Fichier") or filedialog.asksaveasfilename(title="Fichier")
        if chemin:
            variable.set(chemin)

    def choisir_dossier(self, variable):
        chemin = filedialog.askdirectory(title="Dossier du code")
        if chemin:
            variable.set(chemin)

    def executer(self):
        if self.courante is None:
            return
        valeurs = {nom: variable.get() for nom, variable in self.valeurs.items()}
        for champ in self.courante.champs:
            if est_reference(champ) and isinstance(valeurs.get(champ.nom), str):
                valeurs[champ.nom] = identifiants(valeurs[champ.nom], self.notes_connues)
        try:
            args = arguments(self.courante, valeurs)
        except ValueError as erreur:
            messagebox.showwarning("Formulaire incomplet", str(erreur))
            return
        code = self.executer_args(args)
        a_oui = any(c.nom == "--oui" for c in self.courante.champs)
        if code == CONFIRMATION_REQUISE and a_oui:
            self.a_confirmer = [*args, "--oui"]
            self.bouton_confirmer.configure(state="normal")
        else:
            self.a_confirmer = None
            self.bouton_confirmer.configure(state="disabled")

    def confirmer(self):
        if self.a_confirmer:
            self.executer_args(self.a_confirmer)
            self.a_confirmer = None
            self.bouton_confirmer.configure(state="disabled")

    def executer_args(self, args):
        code, texte = lancer(self.memoire, self.base.get(), args)
        self.afficher(texte)
        if code == ERREUR_DE_STOCKAGE and MIGRATION_PROPOSEE in texte and args != ["migrer"]:
            self.proposer_migration()
        return code

    def proposer_migration(self):
        """Base de schéma 1 : la migration est proposée, jamais lancée sans accord."""
        if self.demander(
            "Migration de la base",
            "Cette base a été écrite par une version antérieure (schéma 1). La convertir au "
            "schéma 2 ? Une sauvegarde est faite avant, dans sauvegardes/ à côté de la base ; "
            "la base convertie est vérifiée en entier avant de remplacer l'ancienne.",
        ):
            self.executer_args(["migrer"])


def principal():
    parseur = argparse.ArgumentParser(description="Interface graphique minimale de memoire")
    parseur.add_argument("--memoire", default="memoire", help="chemin de l'exécutable memoire")
    defaut = os.environ.get("MEMOIRE_BASE") or str(Path.cwd() / ".memoire" / "depot.db")
    parseur.add_argument("--base", default=defaut, help="fichier de base (défaut : MEMOIRE_BASE ou .memoire/depot.db)")
    options = parseur.parse_args()
    racine = tk.Tk()
    try:
        Application(racine, options.memoire, options.base)
    except (OSError, RuntimeError, ValueError) as erreur:
        messagebox.showerror("memoire", f"Impossible de démarrer : {erreur}")
        racine.destroy()
        return 1
    racine.mainloop()
    return 0


if __name__ == "__main__":
    sys.exit(principal())
