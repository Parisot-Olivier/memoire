"""Champs partagés par les fenêtres de l'interface (R3, 5.3, 6.2, 7.2).

Champ de note avec recherche par nom (RETROUVER.md, section 1) : dès deux lettres tapées, une liste
de suggestions s'ouvre sous le champ (`memoire chercher`, sans tenir compte des majuscules ni des
accents) ; ↑ ↓ et Entrée, ou un clic, choisissent. La note choisie s'affiche « nom · id8 » ;
`schema.identifiants` rend l'identifiant à la commande. Dans un champ à plusieurs notes, séparées
par des virgules, seule la dernière est cherchée et remplacée. La liste déroulante du champ
fonctionne aussi. Aucune règle : la recherche est celle de l'exécutable.
"""
import subprocess
from tkinter import ttk

from apparence import COULEUR_DU_ROLE
from menus import Carte, Ligne
from schema import affichage_de_note, dernier_element, remplacer_dernier

DELAI_DE_FRAPPE_MS = 250
LETTRES_MINIMALES = 2


def _role_lisible(role):
    try:
        from actions import mot
    except ImportError:  # pragma: no cover
        return role
    return mot("role", role)


def liste_de_notes(parent, memoire, base, variable, multiple=False, largeur=78, connus=None):
    """Champ de note à placer par l'appelant ; `base` rend le chemin de la base ; `connus`
    (id8 → identifiant complet) reçoit chaque note choisie."""
    connus = {} if connus is None else connus
    liste = ttk.Combobox(parent, textvariable=variable, width=largeur)
    liste.suggestions = []
    liste.connus = connus
    resultats = []
    etat = {"attente": None, "avant": ""}
    # Les suggestions s'ouvrent dans une carte non modale : la frappe continue (8.2a).
    carte = Carte(liste, modale=False)
    liste.carte = carte

    def chercher(texte):
        resultat = subprocess.run([memoire, "--base", base(), "chercher", texte],
                                  capture_output=True, text=True, encoding="utf-8")
        trouves = []
        for ligne in resultat.stdout.splitlines() if resultat.returncode == 0 else []:
            identifiant, role, titre = (ligne.split("\t", 2) + ["", ""])[:3]
            trouves.append((identifiant, titre, role))
        return trouves

    def proposer_maintenant():
        etat["attente"] = None
        etat["avant"] = variable.get()
        texte = dernier_element(etat["avant"]) if multiple else variable.get().strip()
        resultats.clear()
        if len(texte) >= LETTRES_MINIMALES and "·" not in texte:
            resultats.extend(chercher(texte))
        liste.suggestions = [f"{titre} — {_role_lisible(role)}" for _, titre, role in resultats]
        liste.configure(values=liste.suggestions)
        montrer()

    def montrer():
        if not liste.suggestions or not liste.winfo_ismapped():
            carte.fermer()
            return
        lignes = [Ligne(titre, second=_role_lisible(role), droite=identifiant[:8],
                        point=COULEUR_DU_ROLE.get(role, "#8A7F8D"))
                  for identifiant, titre, role in resultats]
        carte.montrer(lignes, liste.winfo_rootx(), liste.winfo_rooty() + liste.winfo_height() + 4,
                      largeur_min=max(liste.winfo_width(), 200), quand_choisi=choisir_suggestion,
                      rang=0, au_dessus=liste.winfo_rooty())

    def cacher(_evenement=None):
        carte.fermer()

    def choisir_suggestion(rang):
        if not 0 <= rang < len(resultats):
            return
        identifiant, titre, _ = resultats[rang]
        connus[identifiant[:8]] = identifiant
        affiche = affichage_de_note(titre, identifiant)
        if multiple:
            affiche = remplacer_dernier(etat["avant"], f" {affiche}" if "," in etat["avant"]
                                        else affiche)
        variable.set(affiche)
        liste.icursor("end")
        cacher()

    def frappe(evenement):
        if evenement.keysym in ("Up", "Down", "Return", "Escape", "Tab"):
            return
        if etat["attente"] is not None:
            liste.after_cancel(etat["attente"])
        etat["attente"] = liste.after(DELAI_DE_FRAPPE_MS, proposer_maintenant)

    def deplacer(pas):
        if not carte.visible():
            return None
        carte.deplacer(pas)
        return "break"

    def valider(_evenement):
        if not carte.visible():
            return None
        choisir_suggestion(carte.rang if carte.rang is not None else 0)
        return "break"

    def depuis_la_liste(_evenement=None):
        """Choix dans la liste déroulante du champ."""
        libelle = variable.get()
        if libelle in liste.suggestions:
            choisir_suggestion(liste.suggestions.index(libelle))

    liste.proposer_maintenant = proposer_maintenant
    liste.choisir_suggestion = choisir_suggestion
    liste.configure(postcommand=lambda: (proposer_maintenant(), cacher()))
    liste.bind("<<ComboboxSelected>>", depuis_la_liste)
    liste.bind("<KeyRelease>", frappe, add="+")
    liste.bind("<Down>", lambda _e: deplacer(1))
    liste.bind("<Up>", lambda _e: deplacer(-1))
    liste.bind("<Return>", valider, add="+")
    liste.bind("<Escape>", cacher, add="+")
    liste.bind("<FocusOut>", lambda _e: liste.after(150, cacher), add="+")
    liste.bind("<Destroy>", lambda _e: carte.detruire(), add="+")
    return liste
