"""Plusieurs terminaux : nommage et registre (TERMINAUX.md, sections 1 à 3 ; D29 ; INV-TERM-03).

Aucune importation de tkinter. Un terminal est nommé par ce qu'il fait : la commande en cours,
sinon son shell et son dossier ; les doublons sont numérotés. Le registre attribue des numéros
internes jamais réemployés et conserve, entre deux sessions, la liste des terminaux et leur dernier
dossier (ni commande ni sortie).
"""
from dataclasses import dataclass, field
import json
import os
from pathlib import Path
import re
import tempfile

from notification import duree_lisible

LONGUEUR_DE_NOM = 40
VERSION = 1
NOM_INTERNE = re.compile(r"^terminal-([1-9][0-9]*)$")


@dataclass(frozen=True)
class EtatDuTerminal:
    """Ce que l'on sait d'un terminal : shell, dossier, commande en cours et son début, code de la
    dernière commande, code de sortie du shell s'il est terminé."""

    shell: str
    dossier: str = None
    commande: str = None
    debut: float = None
    dernier_code: int = None
    termine: int = None


def abreger(dossier, maison):
    """Dossier abrégé par « ~ » quand il est sous `maison`."""
    if not maison:
        return dossier
    racine = maison.rstrip("/") or "/"
    if dossier == racine:
        return "~"
    if racine != "/" and dossier.startswith(racine + "/"):
        return "~" + dossier[len(racine):]
    return dossier


def nom(etat, maison):
    """Nom d'un terminal (TERMINAUX.md, section 1)."""
    if etat.termine is not None:
        return f"terminé (code {etat.termine})"
    if etat.commande is not None:
        commande = etat.commande
        if len(commande) > LONGUEUR_DE_NOM:
            commande = commande[:LONGUEUR_DE_NOM - 1] + "…"
        return commande
    shell = os.path.basename(etat.shell) or etat.shell
    return f"{shell} — {abreger(etat.dossier, maison)}" if etat.dossier else shell


def noms(etats, maison):
    """Noms de plusieurs terminaux, dans l'ordre ; un nom déjà pris reçoit « (2) », « (3) »…"""
    resultat, vus = [], {}
    for etat in etats:
        base = nom(etat, maison)
        vus[base] = vus.get(base, 0) + 1
        resultat.append(base if vus[base] == 1 else f"{base} ({vus[base]})")
    return resultat


def etat_lisible(etat, maintenant):
    """« en cours depuis 2 min 13 s », « inactif », « dernière commande : code 2 », « terminé »."""
    if etat.termine is not None:
        return f"terminé (code {etat.termine})"
    if etat.commande is not None and etat.debut is not None:
        return f"en cours depuis {duree_lisible(maintenant - etat.debut)}"
    if etat.dernier_code:
        return f"dernière commande : code {etat.dernier_code}"
    return "inactif"


def nom_court(titre, longueur):
    """Nom raccourci à `longueur` caractères : d'un dossier (« shell — dossier »), on garde la fin,
    qui distingue deux terminaux ; d'une commande, le début."""
    if len(titre) <= longueur:
        return titre
    tete, separateur, queue = titre.partition(" — ")
    garde = longueur - len(tete) - len(separateur) - 1
    if separateur and garde >= 4:
        return f"{tete}{separateur}…{queue[-garde:]}"
    return titre[:longueur - 1] + "…"


def etat_court(etat, maintenant):
    """(texte, genre) pour l'en-tête d'une tuile et sa pastille (CONSOLE.md, section 4) : genre
    « cours », « echec », « termine » ou « repos »."""
    if etat.termine is not None:
        return "terminé", "termine"
    if etat.commande is not None:
        if etat.debut is None:
            return "en cours", "cours"
        return f"depuis {duree_lisible(maintenant - etat.debut)}", "cours"
    if etat.dernier_code:
        return f"code {etat.dernier_code}", "echec"
    return "", "repos"


@dataclass
class Terminaux:
    """Registre des terminaux ouverts : numéro interne → dernier dossier connu."""

    prochain: int = 1
    dossiers: dict = field(default_factory=dict)
    # État de la console (8.5, CONSOLE.md, section 5) : seuil de notification et mosaïque, tels
    # que leur lecteur les a écrits ; c'est lui qui les valide (gui/mosaique.py).
    console: dict = None

    @classmethod
    def par_defaut(cls):
        registre = cls()
        registre.nouveau(None)
        return registre

    def nouveau(self, dossier):
        nom_interne = f"terminal-{self.prochain}"
        self.prochain += 1
        self.dossiers[nom_interne] = dossier
        return nom_interne

    def fermer(self, nom_interne):
        if nom_interne not in self.dossiers:
            raise ValueError(f"terminal inconnu : {nom_interne}")
        del self.dossiers[nom_interne]

    def noms_internes(self):
        return list(self.dossiers)

    def dossier(self, nom_interne):
        return self.dossiers[nom_interne]

    def noter_dossier(self, nom_interne, dossier):
        if nom_interne in self.dossiers:
            self.dossiers[nom_interne] = dossier

    def vers_json(self):
        contenu = {"version": VERSION, "prochain": self.prochain,
                   "terminaux": [{"nom": n, "dossier": d} for n, d in self.dossiers.items()]}
        if self.console is not None:
            contenu["console"] = self.console
        return contenu

    @classmethod
    def de_json(cls, contenu):
        """Registre relu et avertissement ; contenu invalide : un seul terminal, sans dossier."""
        try:
            if not isinstance(contenu, dict) or contenu.get("version") != VERSION:
                raise ValueError("version inconnue")
            prochain = contenu["prochain"]
            if isinstance(prochain, bool) or not isinstance(prochain, int) or prochain < 1:
                raise ValueError("prochain")
            dossiers = {}
            for entree in contenu["terminaux"]:
                nom_interne, dossier = entree["nom"], entree["dossier"]
                correspondance = NOM_INTERNE.match(nom_interne) if isinstance(nom_interne, str) else None
                if (correspondance is None or int(correspondance.group(1)) >= prochain
                        or nom_interne in dossiers
                        or not (dossier is None or isinstance(dossier, str))):
                    raise ValueError(f"terminal {nom_interne!r}")
                dossiers[nom_interne] = dossier
            console = contenu.get("console")
            return cls(prochain, dossiers, console if isinstance(console, dict) else None), None
        except (ValueError, TypeError, KeyError) as erreur:
            return cls.par_defaut(), f"liste des terminaux invalide ({erreur}) : un seul terminal"

    def ecrire(self, chemin):
        """Écriture atomique : fichier temporaire dans le même dossier, puis renommage."""
        chemin = Path(chemin)
        chemin.parent.mkdir(parents=True, exist_ok=True)
        fd, temporaire = tempfile.mkstemp(dir=chemin.parent, prefix=".terminaux-", suffix=".tmp")
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as fichier:
                json.dump(self.vers_json(), fichier, ensure_ascii=False, indent=1)
                fichier.write("\n")
            os.replace(temporaire, chemin)
        except BaseException:
            Path(temporaire).unlink(missing_ok=True)
            raise


def chemin_des_terminaux(env=os.environ):
    """`$XDG_CONFIG_HOME/memoire/terminaux.json`, sinon `~/.config/memoire/terminaux.json`."""
    base = env.get("XDG_CONFIG_HOME") or os.path.join(env.get("HOME", ""), ".config")
    return Path(base) / "memoire" / "terminaux.json"


def lire(chemin):
    """Registre conservé et avertissement ; fichier absent : un seul terminal, sans avertissement."""
    chemin = Path(chemin)
    if not chemin.exists():
        return Terminaux.par_defaut(), None
    try:
        contenu = json.loads(chemin.read_text(encoding="utf-8"))
    except (OSError, ValueError) as erreur:
        return Terminaux.par_defaut(), f"liste des terminaux illisible ({erreur}) : un seul terminal"
    return Terminaux.de_json(contenu)
