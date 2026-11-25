"""Les terminaux de l'interface (TERMINAUX.md, sections 1 et 2 ; CONSOLE.md ; D29, D32 ;
INV-TERM-03, INV-TERM-04).

Tous les terminaux sont les tuiles d'une console unique, panneau « console » du plan de travail
(gui/console.py). La gestion tient le registre (gui/terminaux.py) et le modèle de la mosaïque
(gui/mosaique.py), nomme les terminaux d'après ce qu'ils font, porte les gestes (créer, rouvrir,
ranger, fermer, agrandir, déplacer le point milieu, pleine page) et conserve le tout.
"""
from dataclasses import replace
import os
import time
import tkinter as tk

from console import Console
from mosaique import Mosaique
from notification import SEUIL_PAR_DEFAUT
from plan_de_travail import Defaut
from terminaux import chemin_des_terminaux, etat_court, etat_lisible, lire, noms

PERIODE_DE_MISE_A_JOUR_MS = 300
PERIODE_DU_POULS_S = 0.7
SEUIL_MAX = 3600


class GestionDesTerminaux:
    """Registre, mosaïque, console, noms et conservation ; `app` est la fenêtre principale."""

    def __init__(self, app):
        self.app = app
        self.plan = app.plan
        self.chemin = chemin_des_terminaux(os.environ)
        self.registre, avertissement = lire(self.chemin)
        if avertissement:
            app.signaler(avertissement)
        conserve = self.registre.console or {}
        self.mosaique, avertissement = Mosaique.de_json(conserve.get("mosaique"),
                                                        self.registre.noms_internes())
        if avertissement:
            app.signaler(avertissement)
        seuil = conserve.get("seuil")
        valide = isinstance(seuil, (int, float)) and not isinstance(seuil, bool) \
            and 0 <= seuil <= SEUIL_MAX
        self._seuil = float(seuil) if valide else SEUIL_PAR_DEFAUT
        self.console = None
        self._mise_a_jour = None
        self._battement = True
        # Sans bandeau : la rangée de la console porte « Pleine page », « Détacher » et « ⋯ ».
        self.plan.ajouter("console", "Console", Defaut(zone="droite", rang=0, part=0.7),
                          self._construire, sans_bandeau=True)

    def _construire(self, cadre):
        self.console = Console(cadre, self.app, self)
        return self.console

    def demarrer(self, _racine=None):
        """Après le démarrage du plan de travail : noms et mise à jour périodique."""
        self._mettre_a_jour()

    # ----- Lecture -----

    def noms_internes(self):
        return self.registre.noms_internes()

    def terminal(self, nom):
        return self.console.terminaux[nom]

    def _maison(self):
        return os.environ.get("HOME", "")

    def _dossier_affiche(self, dossier):
        """Dossier ramené sous HOME tel qu'il est écrit, si HOME est un lien vers lui."""
        maison = self._maison()
        if dossier and maison:
            reel = os.path.realpath(maison)
            if reel != maison and (dossier == reel or dossier.startswith(reel.rstrip("/") + "/")):
                return maison.rstrip("/") + dossier[len(reel.rstrip("/")):]
        return dossier

    def _etats(self):
        etats = []
        for nom in self.noms_internes():
            etat = self.terminal(nom).etat()
            etats.append(replace(etat, dossier=self._dossier_affiche(etat.dossier)))
        return etats

    def en_cours(self):
        """(nom interne, commande) des terminaux qui exécutent une commande (barre d'état)."""
        return [(nom, etat.commande) for nom, etat in zip(self.noms_internes(), self._etats())
                if etat.commande is not None and etat.termine is None]

    def place(self, nom):
        """« page » ou « arrière-plan »."""
        return "page" if nom in self.mosaique.page else "arrière-plan"

    def lignes(self):
        """(nom interne, nom, état lisible, place) de chaque terminal."""
        etats = self._etats()
        maintenant = time.monotonic()
        return [(nom, titre, etat_lisible(etat, maintenant), self.place(nom))
                for nom, titre, etat in zip(self.noms_internes(), noms(etats, self._maison()), etats)]

    def etats_courts(self, maintenant=None):
        """(nom interne, nom, état, genre) de chaque terminal, pour les tuiles et les pastilles."""
        etats = self._etats()
        maintenant = time.monotonic() if maintenant is None else maintenant
        return [(nom, titre, etat, etat_court(etat, maintenant)[1])
                for nom, titre, etat in zip(self.noms_internes(), noms(etats, self._maison()), etats)]

    def montres(self):
        """Terminaux qu'on voit dans la console, d'après la place disponible."""
        return self.mosaique.montres(self.console.capacite())

    def en_attente(self):
        """Terminaux qu'on ne voit pas : ils attendent en pastilles."""
        return self.mosaique.en_attente(self.console.capacite())

    def montre(self, nom):
        """Ce terminal est-il à l'écran ?"""
        return self.plan.visible("console") and nom in self.montres()

    # ----- Seuil de notification, commun à la console (CONSOLE.md, section 4) -----

    @property
    def seuil(self):
        return self._seuil

    @seuil.setter
    def seuil(self, valeur):
        self._seuil = max(0.0, min(float(SEUIL_MAX), float(valeur)))
        if self.console is not None:
            self.console.seuil_change()

    def changer_le_seuil(self, pas):
        self.seuil = self._seuil + pas
        self.enregistrer()

    # ----- Gestes sur les terminaux -----

    def _suivre(self, enregistrer=True, glisser=False):
        """Le modèle a changé : l'écran suit, puis la conservation. `glisser` : au milieu d'un
        geste, seule la mosaïque bouge, rien n'est écrit (CONSOLE.md, section 11)."""
        self.console.disposer(glisser)
        if enregistrer and not glisser:
            self.enregistrer()

    def nouveau(self):
        """Crée un terminal, sur la page de la console (qui se montre) ; rend son nom interne."""
        nom = self.registre.nouveau(os.getcwd())
        self.console.ajouter(nom, os.getcwd())
        self.afficher(nom)
        return nom

    def afficher(self, nom):
        """Met le terminal sur la page et le rend actif ; montre la console si elle est masquée ;
        place le curseur dans sa saisie."""
        terminal = self.terminal(nom)
        self.mosaique.placer(nom)
        terminal.marque = ""
        if not self.plan.visible("console"):
            self.plan.montrer("console")
        self._suivre()
        terminal.saisie.focus_set()

    def activer(self, nom):
        """Le terminal où l'on tape : liseré de sa tuile, cible de « page pleine »."""
        if nom in self.mosaique.page and self.mosaique.actif != nom:
            self.mosaique.activer(nom)
            self._suivre(enregistrer=False)

    def ranger(self, nom):
        """Passe le terminal en arrière-plan : son shell continue."""
        self.mosaique.ranger(nom)
        self._suivre()

    def masquer(self, nom):
        self.ranger(nom)

    def agrandir(self, nom):
        """Les deux tiers de la page pour ce terminal, qui reçoit le curseur ; une seconde fois,
        parts égales."""
        self.mosaique.agrandir(nom, self.console.capacite())
        self._suivre()
        self.terminal(nom).saisie.focus_set()

    def egaliser(self):
        self.mosaique.egaliser()
        self._suivre()

    def deplacer_le_point(self, x=None, y=None, glisser=False):
        """Déplace le point milieu ; `glisser` : pendant le geste, avant le lâcher."""
        self.mosaique.deplacer(x, y)
        self._suivre(glisser=glisser)

    def fermer(self, nom):
        """Arrête le shell et retire la tuile ; confirmation si une commande tourne."""
        terminal = self.terminal(nom)
        en_cours = terminal.suivi.en_cours() if terminal.session is not None else None
        if en_cours and terminal.session.en_vie():
            question = (f"La commande « {en_cours[0]} » tourne encore dans ce terminal. "
                        "Le fermer l'arrêtera. Fermer quand même ?")
            if not self.app.demander("Fermer le terminal", question):
                return False
        self.registre.fermer(nom)
        self.mosaique.retirer(nom)
        self.console.retirer(nom)
        self._suivre()
        return True

    # ----- Place de la console (CONSOLE.md, section 1) -----

    def en_page(self):
        return self.plan.disposition.etat("console").etat == "page"

    def pleine_page(self):
        """La console prend toute la carte."""
        self.plan.mettre_en_page("console")
        self._curseur()

    def reduire(self):
        """La console en pleine page revient où elle était : sur le côté, ou en fenêtre."""
        self.plan.quitter_la_page("console")

    def basculer(self):
        """Bouton de l'en-tête : montre la console là où elle était, curseur dans la saisie ; la
        masque si elle est visible ; en pleine page, la réduit."""
        if self.en_page():
            self.reduire()
        elif self.plan.visible("console"):
            self.plan.masquer("console")
        else:
            if not self.noms_internes():
                self.nouveau()
                return
            self.plan.montrer("console")
            self._curseur()

    def _curseur(self):
        actif = self.mosaique.actif
        if actif is not None:
            self.terminal(actif).saisie.focus_set()

    # ----- Mise à jour, conservation -----

    def _mettre_a_jour(self):
        self._mise_a_jour = None
        maintenant = time.monotonic()
        self._battement = int(maintenant / PERIODE_DU_POULS_S) % 2 == 0
        try:
            self.console.actualiser(self.etats_courts(maintenant), self._battement, maintenant)
        except tk.TclError:
            return
        for nom in self.noms_internes():
            self.registre.noter_dossier(nom, self.terminal(nom).etat().dossier)
        self._mise_a_jour = self.app.racine.after(PERIODE_DE_MISE_A_JOUR_MS, self._mettre_a_jour)

    def enregistrer(self):
        for nom in self.noms_internes():
            self.registre.noter_dossier(nom, self.terminal(nom).etat().dossier)
        self.registre.console = {"seuil": self._seuil, "mosaique": self.mosaique.vers_json()}
        try:
            self.registre.ecrire(self.chemin)
        except OSError as erreur:
            self.app.signaler(f"liste des terminaux non enregistrée : {erreur}")

    def fermer_tout(self):
        """À la fermeture du logiciel : liste conservée, mise à jour arrêtée, shells arrêtés."""
        self._annuler_la_mise_a_jour()
        self.enregistrer()
        for nom in self.noms_internes():
            self.terminal(nom).arreter()

    def arreter_sans_enregistrer(self):
        """Fenêtre détruite sans fermeture ordinaire : mise à jour annulée, shells arrêtés."""
        self._annuler_la_mise_a_jour()
        for nom in self.noms_internes():
            self.terminal(nom).arreter()

    def _annuler_la_mise_a_jour(self):
        if self._mise_a_jour is not None:
            try:
                self.app.racine.after_cancel(self._mise_a_jour)
            except tk.TclError:
                pass
            self._mise_a_jour = None
