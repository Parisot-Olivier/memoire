"""Formulaire progressif d'une action du catalogue (ACTIONS.md, sections 2, 5 et 6 ; D30).

Les questions minimales d'abord ; une section cochée ajoute les siennes ; une question
conditionnelle n'apparaît que pour le choix qui la demande. « Faire » lance les commandes de
l'action ; une opération importante montre d'abord « Ce que ça va changer », puis attend la
confirmation ; une réussite propose « Et ensuite ? ». Aucune règle ici : les commandes sont
celles du catalogue (gui/actions.py), l'exécutable fait le reste.
"""
import os
import tempfile
import tkinter as tk
from tkinter import filedialog, messagebox, ttk

from actions import action as action_de, commandes_de, decrire, mot, traduire_apercu
from champs import liste_de_notes
from schema import analyser, identifiants
from theme import Flux, ZoneDeTexte
from visionneur import interroger

CONFIRMATION_REQUISE = 4
NOMS_DE_CHAMP = {"verdict": "vérification", "decision": "décision", "implementation": "avancement",
                 "reserve": "état du doute", "visibilite": "en vigueur", "texte": "texte"}


def _valeur_lisible(champ, valeur):
    categorie = {"implementation": "implementation", "reserve": "reserve"}.get(champ, champ)
    return mot(categorie, valeur)


class FormulaireAction:
    """Le formulaire d'une action, dans le cadre `cadre` de la fenêtre principale `app`."""

    def __init__(self, app, cadre, action, prerempli=None):
        self.app = app
        self.cadre = cadre
        self.action = action
        self.objet = None
        self.apercu = None
        self.notes_trouvees = []
        self._commandes_lancees = []
        self._a_confirmer = None
        self._derniere = {}
        self.variables = {}
        self.textes = {}
        self.sections = {}
        self.rangees = {}
        self.choix_affiches = {}
        for enfant in cadre.winfo_children():
            enfant.destroy()
        self._construire()
        for cle, valeur in (prerempli or {}).items():
            for section in action.sections:
                if any(q.cle == cle for q in section.questions):
                    self.cocher(section.cle)
            self.remplir({cle: valeur})
        self._rafraichir()
        self._lignes_lancees = []

    # ----- Construction -----

    def _construire(self):
        haut = ttk.Frame(self.cadre)
        haut.pack(fill="x")
        haut.columnconfigure(1, weight=1)
        rang = 0
        for question in self.action.questions:
            if question.genre == "interne":
                self.variables[question.cle] = tk.StringVar()
                continue
            self._question(haut, rang, question)
            rang += 1
        if self.action.sections:
            ttk.Label(self.cadre, text="Si besoin :", style="Question.TLabel").pack(
                anchor="w", pady=(12, 4))
            puces = Flux(self.cadre)
            puces.pack(fill="x")
        for rang_, section in enumerate(self.action.sections):
            cadre = ttk.Frame(self.cadre)
            cadre.pack(fill="x")
            case = tk.BooleanVar(value=False)
            # Les cases sont des puces côte à côte ; leurs questions viennent dessous.
            puces.ajouter(ttk.Checkbutton(puces, text=section.libelle, variable=case,
                                          command=self._rafraichir))
            interieur = ttk.Frame(cadre, padding=(0, 6, 0, 0))
            interieur.columnconfigure(1, weight=1)
            for rang, question in enumerate(section.questions):
                self._question(interieur, rang, question)
            self.sections[section.cle] = (case, interieur)
        self.boutons = ttk.Frame(self.cadre, padding=(0, 14, 0, 0))
        self.boutons.pack(fill="x")
        self.bouton_faire = ttk.Button(self.boutons, text="Faire", command=self.faire,
                                       style="Action.TButton")
        self.bouton_faire.pack(side="left")
        if self.action.special == "citer":
            ttk.Button(self.boutons, text="Choisir en lisant",
                       command=lambda: self.app.voir(self.valeur("note"))).pack(side="left", padx=10)
        self.zone_apercu = ttk.Frame(self.cadre)
        self.zone_apercu.pack(fill="x")
        self.zone_suites = ttk.Frame(self.cadre)
        self.zone_suites.pack(fill="x")

    def _question(self, parent, rang, question):
        etiquette = ttk.Label(parent, text=question.libelle, style="Question.TLabel")
        etiquette.grid(row=rang, column=0, sticky="nw", padx=(0, 12), pady=(9, 4))
        if question.genre == "texte_long":
            zone = ZoneDeTexte(parent, height=6, width=70, wrap="word", font="TkTextFont")
            zone.grid(row=rang, column=1, sticky="we", pady=4)
            zone.bind("<KeyRelease>", lambda _e: self._changement(), add="+")
            self.textes[question.cle] = zone
            self.rangees[question.cle] = (etiquette, zone)
            return
        variable = tk.StringVar()
        self.variables[question.cle] = variable
        variable.trace_add("write", lambda *_a: self._changement())
        if question.genre == "choix":
            cadre = Flux(parent)
            for libelle, valeur in question.choix:
                cadre.ajouter(ttk.Radiobutton(cadre, text=libelle, value=valeur, variable=variable,
                                              command=self._rafraichir))
            variable.set(question.choix[0][1])
            widget = cadre
        elif question.genre in ("note", "notes"):
            widget = liste_de_notes(parent, self.app.memoire, self.app.base.get, variable,
                                    question.genre == "notes", largeur=60,
                                    connus=self.app.notes_connues)
        elif question.genre in ("piste", "etape", "reprise"):
            choix = self.choix_de(question.cle, question.genre)
            self.choix_affiches[question.cle] = choix
            widget = ttk.Combobox(parent, values=[nom for nom, _ in choix], state="readonly", width=60)
            widget.bind("<<ComboboxSelected>>",
                        lambda _e, w=widget, v=variable, c=question.cle: v.set(
                            dict(self.choix_affiches[c]).get(w.get(), "")))
            if question.cle == "pour":
                active = next(((n, i) for n, i in choix if n.startswith("★ ")), None)
                if active:
                    widget.set(active[0])
                    variable.set(active[1])
        else:
            cadre = ttk.Frame(parent)
            ttk.Entry(cadre, textvariable=variable, width=60).pack(side="left", fill="x", expand=True)
            if question.genre in ("fichier", "dossier"):
                choisir = (filedialog.askdirectory if question.genre == "dossier"
                           else filedialog.askopenfilename)
                ttk.Button(cadre, text="Parcourir…",
                           command=lambda: variable.set(choisir() or variable.get())).pack(side="left")
            widget = cadre
        widget.grid(row=rang, column=1, sticky="we", pady=4)
        self.rangees[question.cle] = (etiquette, widget)

    def choix_de(self, cle, genre=None):
        """(nom affiché, valeur) des pistes, étapes ou points de reprise proposés."""
        genre = genre or self.action.question(cle).genre
        if genre == "piste":
            code, sortie, _ = interroger(self.app.memoire, self.app.base.get(), ["branches", "--tsv"])
            choix = []
            for ligne in sortie.splitlines()[1:] if code == 0 else []:
                piste, active, _, nom, _motif = (ligne.split("\t") + [""] * 5)[:5]
                choix.append((("★ " if active == "oui" else "") + nom, piste))
            return choix
        if genre == "etape":
            commande = ["historique", "--nombre", "50"]
        else:
            commande = ["reprise", "liste"]
        code, sortie, _ = interroger(self.app.memoire, self.app.base.get(), commande)
        choix = []
        for ligne in sortie.splitlines() if code == 0 else []:
            identifiant, _, reste = ligne.partition(" — ")
            if len(identifiant) == 36:
                choix.append((f"{identifiant[:8]} — {reste}", identifiant))
        return choix

    # ----- Réponses -----

    def _rafraichir(self):
        reponses = self._reponses_brutes()
        for question in self.action.questions:
            if question.si is None or question.cle not in self.rangees:
                continue
            visible = reponses.get(question.si[0]) == question.si[1]
            for widget in self.rangees[question.cle]:
                if visible:
                    widget.grid()
                else:
                    widget.grid_remove()
        for case, interieur in self.sections.values():
            if case.get():
                interieur.pack(fill="x")
            else:
                interieur.pack_forget()
        self._changement()

    def _changement(self):
        """Une réponse a changé : « Ce que ça va changer » sera recalculé (panneau Aperçu)."""
        signaler = getattr(self.app, "apercu_a_jour", None)
        if signaler is not None and hasattr(self, "sections"):
            signaler(self)

    def apercu_prevu(self):
        """Ce que les commandes de l'action changeront, d'après les réponses affichées."""
        if self.action.special in ("lire", "voisinage"):
            return [("·", "rien ne change", "lecture seule")]
        reponses = self._reponses_brutes()
        noms = {valeur: nom.removeprefix("★ ") for choix in self.choix_affiches.values()
                for nom, valeur in choix}
        try:
            commandes = commandes_de(self.action, reponses, self.sections_cochees())
        except (TypeError, AttributeError, KeyError, ValueError):
            return [("·", "à compléter", "")]
        return [ligne for argv in commandes for ligne in decrire(argv, noms)]

    def _reponses_brutes(self):
        reponses = {cle: v.get() for cle, v in self.variables.items()}
        reponses.update({cle: z.get("1.0", "end-1c") for cle, z in self.textes.items()})
        return reponses

    def remplir(self, valeurs):
        for cle, valeur in valeurs.items():
            if cle in self.textes:
                self.textes[cle].delete("1.0", "end")
                self.textes[cle].insert("1.0", valeur)
            elif cle in self.variables:
                self.variables[cle].set(valeur)
                noms = {v: n for n, v in self.choix_affiches.get(cle, [])}
                if valeur in noms and isinstance(self.rangees[cle][1], ttk.Combobox):
                    self.rangees[cle][1].set(noms[valeur])
        self._rafraichir()

    def valeur(self, cle):
        """Réponse telle qu'affichée (une note : « nom · id8 »)."""
        return self._reponses_brutes().get(cle, "")

    def reponse(self, cle):
        """Réponse telle que la commande la reçoit (une note : son identifiant)."""
        return self._pour_la_commande(self._reponses_brutes()).get(cle, "")

    def _pour_la_commande(self, reponses):
        reponses = dict(reponses)
        for question in self.action.toutes_les_questions():
            if question.genre in ("note", "notes") and reponses.get(question.cle):
                reponses[question.cle] = identifiants(reponses[question.cle],
                                                      self.app.notes_connues)
        return reponses

    def cocher(self, section, actif=True):
        self.sections[section][0].set(actif)
        self._rafraichir()

    def libelles_visibles(self):
        """Libellés des questions affichées, dans l'ordre."""
        reponses = self._reponses_brutes()
        libelles = [q.libelle for q in self.action.visibles(reponses)]
        for section in self.action.sections:
            if self.sections[section.cle][0].get():
                libelles += [q.libelle for q in section.questions]
        return libelles

    def sections_cochees(self):
        return {cle for cle, (case, _) in self.sections.items() if case.get()}

    # ----- Exécution -----

    def _manquantes(self, reponses):
        questions = list(self.action.visibles(reponses))
        for section in self.action.sections:
            if section.cle in self.sections_cochees():
                questions += list(section.questions)
        return [q.libelle for q in questions if q.obligatoire and not reponses.get(q.cle, "").strip()]

    def faire(self):
        """Lance l'action ; rend le code de la dernière commande (4 : à confirmer)."""
        if self.action.special == "lire":
            return 0 if self.app.voir(self.reponse("note")) else None
        if self.action.special == "voisinage":
            return 0 if self.app.voisinage(self.reponse("note")) else None
        reponses = self._reponses_brutes()
        manquantes = self._manquantes(reponses)
        if manquantes:
            messagebox.showwarning("Il manque une réponse", "\n".join(manquantes))
            return None
        self._lignes_lancees = self.apercu_prevu()
        fichiers = []
        try:
            for question in self.action.toutes_les_questions():
                if question.vers_fichier and reponses.get(question.cle):
                    fd, chemin = tempfile.mkstemp(prefix="memoire-saisie-", suffix=".txt")
                    with os.fdopen(fd, "w", encoding="utf-8") as fichier:
                        fichier.write(reponses[question.cle])
                    fichiers.append(chemin)
                    reponses[question.cle] = chemin
            reponses = self._pour_la_commande(reponses)
            self._derniere = reponses
            return self._lancer(commandes_de(self.action, reponses, self.sections_cochees()))
        finally:
            for chemin in fichiers:
                os.unlink(chemin)

    def _accepte_oui(self, argv):
        try:
            commande = analyser(self.app.commandes, argv)[0]
        except ValueError:
            return False
        return any(champ.nom == "--oui" for champ in commande.champs)

    def _lancer(self, commandes, oui=False):
        """Lance les commandes dans l'ordre ; à la première qui demande confirmation, montre
        l'aperçu et garde les commandes restantes pour « Confirmer »."""
        code = 0
        self._commandes_lancees = commandes
        for rang, argv in enumerate(commandes):
            effective = argv + ["--oui"] if oui and self._accepte_oui(argv) else argv
            code, sortie, erreurs = interroger(self.app.memoire, self.app.base.get(), effective)
            self.app.afficher(f"$ memoire {' '.join(effective)}\n{sortie}{erreurs}[code {code}]\n\n")
            if code == CONFIRMATION_REQUISE:
                self._montrer_apercu(sortie, commandes[rang:])
                return code
            if code != 0:
                return code
            self._noter_objet(sortie)
        self._reussite()
        return code

    def _montrer_apercu(self, sortie, restantes):
        self.apercu = traduire_apercu(sortie)
        self._a_confirmer = restantes
        montrer = getattr(self.app, "apercu_du_moteur", None)
        if montrer is not None:
            montrer(self.apercu)
        for enfant in self.zone_apercu.winfo_children():
            enfant.destroy()
        cadre = ttk.LabelFrame(self.zone_apercu, text="Avant de continuer", padding=6)
        cadre.pack(fill="x", pady=10)
        plan = getattr(self.app, "plan", None)
        if plan is not None and "apercu" in plan.panneaux and plan.visible("apercu"):
            # Le détail est dans le panneau « Aperçu » : ici, une ligne et les boutons (8.2b).
            texte = "Opération importante : le détail est à droite, dans « Ce que ça va changer »."
        else:
            texte = self.apercu
        etiquette = ttk.Label(cadre, text=texte, justify="left", wraplength=600)
        etiquette.pack(anchor="w", fill="x")
        cadre.bind("<Configure>", lambda e: etiquette.configure(wraplength=max(200, e.width - 40)),
                   add="+")
        boutons = ttk.Frame(cadre)
        boutons.pack(anchor="w", pady=(10, 0))
        ttk.Button(boutons, text="Confirmer", command=self.confirmer,
                   style="Action.TButton").pack(side="left")
        ttk.Button(boutons, text="Ne rien faire", command=self._abandonner).pack(side="left", padx=10)

    def _abandonner(self):
        self._a_confirmer = None
        self._changement()
        for enfant in self.zone_apercu.winfo_children():
            enfant.destroy()

    def confirmer(self):
        restantes, self._a_confirmer = self._a_confirmer, None
        if not restantes:
            return None
        for enfant in self.zone_apercu.winfo_children():
            enfant.destroy()
        return self._lancer(restantes, oui=True)

    def _noter_objet(self, sortie):
        if self.action.special == "copier_repere":
            for ligne in sortie.splitlines():
                if ligne.startswith("Repère à copier : "):
                    self._montrer_repere(ligne[len("Repère à copier : "):])
        objet = self.action.objet
        if objet is None:
            return
        if objet.startswith("reponse:"):
            self.objet = self._derniere.get(objet.split(":", 1)[1]) or self.objet
            return
        for ligne in sortie.splitlines():
            if ligne.startswith(objet):
                self.objet = ligne[len(objet):].split(" ")[0]
                return

    def _montrer_repere(self, repere):
        """Le commentaire à coller dans le code, et un bouton pour le copier."""
        self.repere = repere
        cadre = ttk.LabelFrame(self.zone_apercu, text="À coller dans votre code", padding=6)
        cadre.pack(fill="x", pady=6)
        ttk.Label(cadre, text=repere, style="Mono.TLabel").pack(side="left")

        def copier():
            self.app.racine.clipboard_clear()
            self.app.racine.clipboard_append(repere)

        ttk.Button(cadre, text="Copier", command=copier).pack(side="left", padx=8)

    def _montrer_les_notes_du_code(self):
        """« Quelles notes concernent ce code ? » : chaque note trouvée, avec « Lire » et
        « Voisinage » (RETROUVER.md, section 2)."""
        argv = self._commandes_lancees[-1] + ["--tsv"]
        code, sortie, _ = interroger(self.app.memoire, self.app.base.get(), argv)
        self.notes_trouvees = []
        for ligne in sortie.splitlines() if code == 0 else []:
            champs = ligne.split("\t")
            if champs[0] in ("ancrage", "proche"):
                trouvee = (champs[1], champs[2])
            elif champs[0] == "marque" and champs[4] == "connue":
                trouvee = (champs[2], champs[3])
            else:
                continue
            if trouvee not in self.notes_trouvees:
                self.notes_trouvees.append(trouvee)
        cadre = ttk.LabelFrame(self.zone_apercu, text="Notes trouvées", padding=6)
        cadre.pack(fill="x", pady=6)
        if not self.notes_trouvees:
            ttk.Label(cadre, text="Aucune note ne concerne ce code.").pack(anchor="w")
        for rang, (_note, titre) in enumerate(self.notes_trouvees):
            ligne = ttk.Frame(cadre)
            ligne.pack(anchor="w", fill="x")
            ttk.Label(ligne, text=titre).pack(side="left")
            ttk.Button(ligne, text="Lire", command=lambda r=rang: self.lire_note(r)).pack(
                side="left", padx=6)
            ttk.Button(ligne, text="Voisinage",
                       command=lambda r=rang: self.app.voisinage(self.notes_trouvees[r][0])).pack(
                side="left")

    def lire_note(self, rang):
        return self.app.voir(self.notes_trouvees[rang][0])

    def _reussite(self):
        self.app.afficher(f"✓ Fait : {self.action.titre}\n\n")
        fait = getattr(self.app, "apercu_fait", None)
        if fait is not None:
            fait(self._lignes_lancees or self.apercu_prevu())
        if self.action.special == "notes_du_code":
            self._montrer_les_notes_du_code()
        for enfant in self.zone_suites.winfo_children():
            enfant.destroy()
        if not self.action.suites:
            return
        ttk.Label(self.zone_suites, text="Et ensuite ?", style="SousTitre.TLabel").pack(
            anchor="w", pady=(14, 6))
        for rang, (libelle, _suite) in enumerate(self.suites()):
            ttk.Button(self.zone_suites, text=libelle,
                       command=lambda r=rang: self.suivre(r)).pack(anchor="w", pady=2)

    def suites(self):
        return [(s.libelle, s) for s in self.action.suites]

    def suivre(self, rang):
        """Ouvre l'action suivante, pré-remplie avec l'objet de celle-ci."""
        suite = self.action.suites[rang]
        prerempli = {}
        for cle, source in suite.remplir.items():
            valeur = self.objet if source == "objet" else self._derniere.get(source)
            if valeur:
                prerempli[cle] = valeur
        return self.app.ouvrir_action(suite.action, prerempli)


class FormulaireAdoption(FormulaireAction):
    """« Adopter une piste » (ACTIONS.md, section 5) : piste à adopter, piste qui l'adopte, phrase
    de résumé, un choix par désaccord, puis la fusion avec le code du plan et les numéros."""

    def __init__(self, app, cadre, action, prerempli=None):
        self.desaccords = []
        self.resume_du_plan = ""
        self._plan = {}
        self._choix_des_desaccords = {}
        self._candidate = tk.StringVar()
        super().__init__(app, cadre, action, prerempli)
        self.bouton_faire.configure(text="Adopter", state="disabled")
        ttk.Button(self.boutons, text="Voir le plan", command=self.voir_le_plan).pack(side="left",
                                                                                      padx=6)
        self.zone_plan = ttk.Frame(self.cadre)
        self.zone_plan.pack(fill="x", before=self.zone_apercu)

    def _nom(self, piste):
        return next((nom.removeprefix("★ ") for nom, valeur in self.choix_de("piste", "piste")
                     if valeur == piste), piste)

    def voir_le_plan(self):
        piste, pour = self.valeur("piste"), self.valeur("pour")
        if not piste:
            messagebox.showwarning("Il manque une réponse", "Quelle piste adopter ?")
            return None
        active = next((v for n, v in self.choix_de("piste", "piste") if n.startswith("★ ")), None)
        if pour and pour != active:
            code, sortie, erreurs = interroger(self.app.memoire, self.app.base.get(),
                                               ["basculer", pour])
            self.app.afficher(f"$ memoire basculer {pour}\n{sortie}{erreurs}[code {code}]\n\n")
            if code != 0:
                return code
        cible = pour or active
        code, sortie, erreurs = interroger(self.app.memoire, self.app.base.get(),
                                           ["fusion", "plan", piste, "--tsv"])
        for enfant in self.zone_plan.winfo_children():
            enfant.destroy()
        self.desaccords, self._choix_des_desaccords = [], {}
        if code != 0:
            self.app.afficher(erreurs)
            return code
        lignes = [l.split("\t") for l in sortie.splitlines()]
        if lignes and lignes[0][0] == "a-jour":
            self.resume_du_plan = (f"« {self._nom(piste)} » est déjà la version retenue de "
                                   f"« {self._nom(cible)} » : rien à adopter.")
            ttk.Label(self.zone_plan, text=self.resume_du_plan, wraplength=700).pack(anchor="w")
            return 0
        self._plan = {l[0]: l[1:] for l in lignes if l[0] in ("code", "base", "sans-conflit")}
        self.resume_du_plan = (f"Les changements de « {self._nom(piste)} » deviendront la version "
                               f"retenue de la {self._nom(cible)}." if cible and
                               self._nom(cible) == "piste principale" else
                               f"Les changements de « {self._nom(piste)} » deviendront la version "
                               f"retenue de « {self._nom(cible)} ».")
        cadre = ttk.LabelFrame(self.zone_plan, text="Le plan", padding=6)
        cadre.pack(fill="x", pady=6)
        ttk.Label(cadre, text=self.resume_du_plan, wraplength=700, justify="left").pack(anchor="w")
        ttk.Label(cadre, text=f"Repris sans difficulté : {self._plan['sans-conflit'][0]} "
                  "changement(s).").pack(anchor="w")
        candidates = []
        a_revoir = []
        for ligne in lignes:
            if ligne[0] == "conflit":
                rang, _genre, _note, titre, champ, ici, autre = ligne[1:8]
                nom_du_champ = NOMS_DE_CHAMP.get(champ, champ)
                texte = (f"« {titre} » — {nom_du_champ} : {_valeur_lisible(champ, ici)} ici, "
                         f"{_valeur_lisible(champ, autre)} dans la piste adoptée.")
                self.desaccords.append(texte)
                choix = tk.StringVar(value="ici")
                self._choix_des_desaccords[int(rang)] = choix
                bloc = ttk.Frame(cadre)
                bloc.pack(anchor="w", pady=(6, 0))
                ttk.Label(bloc, text=texte, wraplength=700).pack(anchor="w")
                ttk.Radiobutton(bloc, text="Garder ce qui est ici", value="ici",
                                variable=choix).pack(anchor="w")
                ttk.Radiobutton(bloc, text="Garder ce que propose la piste adoptée", value="piste",
                                variable=choix).pack(anchor="w")
            elif ligne[0] == "candidate":
                candidates.append(ligne)
            elif ligne[0] == "a-revoir":
                a_revoir.append(ligne[2])
        if candidates:
            ttk.Label(cadre, text="Ces relations formeraient une boucle ; laquelle désactiver ?",
                      wraplength=700).pack(anchor="w", pady=(6, 0))
            self._candidate.set(candidates[0][1])
            for ligne in candidates:
                rang, _relation, origine, genre, destination = ligne[1:6]
                ttk.Radiobutton(cadre, text=f"« {origine} » {mot('relation', genre)} « {destination} »",
                                value=rang, variable=self._candidate).pack(anchor="w")
        if a_revoir:
            ttk.Label(cadre, text="À revoir ensuite : " + ", ".join(a_revoir),
                      wraplength=700).pack(anchor="w", pady=(6, 0))
        self.bouton_faire.configure(state="normal")
        return 0

    def trancher(self, rang, cote):
        self._choix_des_desaccords[rang].set(cote)

    def faire(self):
        if not self._plan:
            if self.voir_le_plan() != 0 or not self._plan:
                return None
        ici = [str(r) for r, v in self._choix_des_desaccords.items() if v.get() == "ici"]
        piste = [str(r) for r, v in self._choix_des_desaccords.items() if v.get() == "piste"]
        self.remplir({"plan": self._plan["code"][0], "cible": ",".join(ici),
                      "source": ",".join(piste)})
        if self._candidate.get():
            self.variables["desactiver"].set(self._candidate.get())
        reponses = self._reponses_brutes()
        reponses["pour"] = ""  # déjà sur la piste qui adopte (voir_le_plan)
        self._derniere = reponses
        return self._lancer(commandes_de(self.action, reponses, set()))


def ouvrir(app, cadre, cle, prerempli=None):
    action = action_de(cle)
    classe = FormulaireAdoption if action.special == "adopter" else FormulaireAction
    return classe(app, cadre, action, prerempli)
