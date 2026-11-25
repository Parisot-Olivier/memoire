"""Moteur du terminal intégré (TERMINAL.md, sections 3 et 4 ; D27 ; INV-TERM-01, INV-TERM-02).

Aucune importation de tkinter : le shell de l'utilisateur tourne sous un pseudo-terminal, le tampon
transforme ses octets en texte à afficher et repère la marque de fin de chaque commande. La vue
(gui/terminal.py) branche la lecture sur sa boucle d'événements et applique les changements.
Linux et macOS : modules pty, termios, fcntl.
"""
import atexit
import codecs
from dataclasses import dataclass, field
import fcntl
import os
import pty
import shutil
import signal
import struct
import tempfile
import termios
import time

# Marque écrite par la fonction ajoutée à PROMPT_COMMAND : ESC ] 7771 ; fin ; <code> BEL.
MARQUE_DE_FIN = "\x1b]7771;fin;"
_CONTENU_DE_FIN = "7771;fin;"
LIMITE_DE_LIGNES = 5000
COLONNES_MIN, LIGNES_MIN = 40, 6


@dataclass(frozen=True)
class Fin:
    """Commande terminée : texte envoyé, code de retour, durée en secondes."""

    commande: str
    code: int
    duree: float


@dataclass
class Changements:
    """Ce que la vue doit appliquer, dans cet ordre : ajouter les lignes terminées `ajoutees` à la
    place de la ligne en cours, afficher `courante` comme nouvelle ligne en cours, puis retirer
    `retirees` lignes en tête. `fins` : codes des commandes terminées pendant cette lecture ;
    `dossier` : dossier du shell transmis par la dernière marque de fin de cette lecture."""

    ajoutees: list = field(default_factory=list)
    courante: str = ""
    retirees: int = 0
    fins: list = field(default_factory=list)
    dossier: str = None


class Tampon:
    """Texte à afficher : UTF-8 incrémental, séquences ANSI retirées (même coupées entre deux
    lectures), retours chariot et retours arrière, limite de lignes, marques de fin repérées."""

    TEXTE, ECHAP, CSI, OSC, OSC_ECHAP, UN_DE_PLUS = range(6)

    def __init__(self, limite=LIMITE_DE_LIGNES):
        self.limite = limite
        self.lignes = []
        self.courante = ""
        self.dossier = None
        self._retour = False
        self._etat = self.TEXTE
        self._osc = []
        self._decodeur = codecs.getincrementaldecoder("utf-8")(errors="replace")

    def texte(self):
        tete = "\n".join(self.lignes) + "\n" if self.lignes else ""
        return tete + self.courante

    def ajouter(self, octets):
        changements = Changements()
        for c in self._decodeur.decode(octets):
            self._caractere(c, changements)
        changements.courante = self.courante
        exces = len(self.lignes) - self.limite
        if exces > 0:
            del self.lignes[:exces]
            changements.retirees = exces
        return changements

    def _caractere(self, c, changements):
        etat = self._etat
        if etat == self.TEXTE:
            self._texte(c, changements)
        elif etat == self.ECHAP:
            self._etat = {"[": self.CSI, "]": self.OSC}.get(c, self.TEXTE)
            if c in "()#%":
                self._etat = self.UN_DE_PLUS
            self._osc = []
        elif etat == self.CSI:
            # Octet final d'une séquence CSI : de @ à ~.
            if "@" <= c <= "~":
                self._etat = self.TEXTE
        elif etat == self.UN_DE_PLUS:
            self._etat = self.TEXTE
        elif etat == self.OSC:
            if c == "\x07":
                self._fin_osc(changements)
            elif c == "\x1b":
                self._etat = self.OSC_ECHAP
            else:
                self._osc.append(c)
        else:  # OSC_ECHAP : ESC \ termine la séquence.
            self._fin_osc(changements)
            if c != "\\":
                self._caractere(c, changements)

    def _fin_osc(self, changements):
        self._etat = self.TEXTE
        contenu = "".join(self._osc)
        if contenu.startswith(_CONTENU_DE_FIN):
            # « <code> » ou, depuis 6.9, « <code>;<dossier> » (le dossier peut contenir « ; »).
            code, separateur, dossier = contenu[len(_CONTENU_DE_FIN):].partition(";")
            try:
                changements.fins.append(int(code))
            except ValueError:
                return
            if separateur:
                changements.dossier = self.dossier = dossier

    def _texte(self, c, changements):
        if c == "\x1b":
            self._etat = self.ECHAP
        elif c == "\n":
            self.lignes.append(self.courante)
            changements.ajoutees.append(self.courante)
            self.courante = ""
            self._retour = False
        elif c == "\r":
            self._retour = True
        elif c == "\b":
            self.courante = self.courante[:-1]
        elif c == "\t" or (c >= " " and c != "\x7f"):
            if self._retour:
                self.courante = ""
                self._retour = False
            self.courante += c


class Historique:
    """Commandes saisies pendant la session, pour ↑ et ↓ ; sans ligne vide ni doublon consécutif."""

    def __init__(self):
        self._commandes = []
        self._position = None
        self._en_cours = ""

    def commandes(self):
        return list(self._commandes)

    def ajouter(self, commande):
        if commande.strip() and (not self._commandes or self._commandes[-1] != commande):
            self._commandes.append(commande)
        self._position = None

    def precedente(self, en_cours):
        if self._position is None:
            self._en_cours = en_cours
            if not self._commandes:
                return en_cours
            self._position = len(self._commandes) - 1
        else:
            self._position = max(0, self._position - 1)
        return self._commandes[self._position]

    def suivante(self):
        if self._position is None:
            return self._en_cours
        self._position += 1
        if self._position >= len(self._commandes):
            self._position = None
            return self._en_cours
        return self._commandes[self._position]


class SuiviDesCommandes:
    """Associe chaque marque de fin à la commande envoyée. La première marque est toujours celle de
    l'invite initiale : elle ne termine aucune commande, même envoyée avant qu'elle n'arrive."""

    def __init__(self):
        self._en_attente = None
        self._invite_initiale = True

    def envoyee(self, commande, instant):
        self._en_attente = (commande, instant)

    def en_cours(self):
        """Commande envoyée et pas encore terminée, avec son instant d'envoi ; sinon None."""
        return self._en_attente

    def terminee(self, code, instant):
        if self._invite_initiale:
            self._invite_initiale = False
            return None
        if self._en_attente is None:
            return None
        commande, debut = self._en_attente
        self._en_attente = None
        return Fin(commande, code, instant - debut)


def environnement(base, executable, env):
    """Variables du shell : `memoire` en tête du PATH et sur la base de la fenêtre ; pas de pager
    ni de terminal évolué (pas d'émulateur complet)."""
    resultat = dict(env)
    dossier = os.path.dirname(executable)
    chemin = env.get("PATH", "")
    if dossier:
        chemin = dossier + os.pathsep + chemin if chemin else dossier
    resultat["PATH"] = chemin
    resultat["MEMOIRE_BASE"] = base
    resultat.update(TERM="dumb", PAGER="cat", GIT_PAGER="cat")
    return resultat


def geometrie(largeur_px, hauteur_px, largeur_car, hauteur_car):
    """Colonnes et lignes d'une zone, d'après la taille d'un caractère ; minimums de la section 1."""
    if largeur_car <= 0 or hauteur_car <= 0:
        return COLONNES_MIN, LIGNES_MIN
    return max(COLONNES_MIN, largeur_px // largeur_car), max(LIGNES_MIN, hauteur_px // hauteur_car)


def fichier_de_demarrage():
    """Fichier de démarrage de bash : le ~/.bashrc de l'utilisateur, puis la marque de fin ajoutée
    en tête de PROMPT_COMMAND, pour lire le code de retour de la commande."""
    return (
        "[ -f ~/.bashrc ] && . ~/.bashrc\n"
        "__memoire_fin() { local code=$?; printf '\\033]7771;fin;%s;%s\\007' \"$code\" \"$PWD\"; }\n"
        'if [[ "$(declare -p PROMPT_COMMAND 2>/dev/null)" == "declare -a"* ]]; then\n'
        '    PROMPT_COMMAND=(__memoire_fin "${PROMPT_COMMAND[@]}")\n'
        "else\n"
        '    PROMPT_COMMAND="__memoire_fin${PROMPT_COMMAND:+;$PROMPT_COMMAND}"\n'
        "fi\n"
    )


# Sessions ouvertes, tenues par une référence forte jusqu'à `arreter` : arrêtées à la sortie de
# Python même si `arreter` n'a pas été appelé et que plus rien d'autre ne les référence
# (INV-TERM-01 : ni processus ni fichier de démarrage ne restent).
_SESSIONS = set()


def arreter_toutes_les_sessions():
    for session in list(_SESSIONS):
        session.arreter(delai=0.2)


atexit.register(arreter_toutes_les_sessions)


def _taille(colonnes, lignes):
    return struct.pack("HHHH", lignes, colonnes, 0, 0)


class Session:
    """Le shell de l'utilisateur sous un pseudo-terminal, dans son propre groupe de processus."""

    def __init__(self, shell, dossier, env, colonnes=100, lignes=30):
        self.shell = shell
        self.dossier = dossier
        self.env = env
        self.colonnes, self.lignes = colonnes, lignes
        self.pid = None
        self.descripteur = None
        self.fichier_de_demarrage = None
        self.suivi_des_fins = os.path.basename(shell) == "bash"
        self._code = None

    def demarrer(self):
        argv = [self.shell, "-i"]
        if self.suivi_des_fins:
            dossier = tempfile.mkdtemp(prefix="memoire-terminal-")
            chemin = os.path.join(dossier, "demarrage.bash")
            fd = os.open(chemin, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, "w", encoding="utf-8") as fichier:
                fichier.write(fichier_de_demarrage())
            self.fichier_de_demarrage = chemin
            # Sans readline : la saisie est celle de la vue, et l'écho du terminal reste fiable
            # pour masquer un mot de passe.
            argv = [self.shell, "--noediting", "--rcfile", chemin, "-i"]
        pid, fd = pty.fork()
        if pid == 0:  # pragma: no cover - processus enfant
            try:
                fcntl.ioctl(0, termios.TIOCSWINSZ, _taille(self.colonnes, self.lignes))
                os.chdir(self.dossier)
                os.execvpe(self.shell, argv, self.env)
            finally:
                os._exit(127)
        self.pid, self.descripteur = pid, fd
        os.set_blocking(fd, False)
        _SESSIONS.add(self)

    def lire_disponible(self):
        """Octets arrivés depuis la dernière lecture, sans bloquer ; vide à la fin du shell."""
        morceaux = []
        while self.descripteur is not None:
            try:
                morceau = os.read(self.descripteur, 65536)
            except BlockingIOError:
                break
            except OSError:  # EIO : le shell est terminé.
                break
            if not morceau:
                break
            morceaux.append(morceau)
        return b"".join(morceaux)

    def ecrire(self, texte):
        donnees = (texte + "\n").encode("utf-8")
        while donnees:
            try:
                ecrits = os.write(self.descripteur, donnees)
            except BlockingIOError:
                time.sleep(0.01)
                continue
            donnees = donnees[ecrits:]

    def interrompre(self):
        """Caractère d'interruption du terminal : SIGINT à la commande au premier plan."""
        caractere = termios.tcgetattr(self.descripteur)[6][termios.VINTR]
        os.write(self.descripteur, caractere if isinstance(caractere, bytes) else bytes([caractere]))

    def redimensionner(self, colonnes, lignes):
        self.colonnes, self.lignes = colonnes, lignes
        fcntl.ioctl(self.descripteur, termios.TIOCSWINSZ, _taille(colonnes, lignes))

    def echo_actif(self):
        return bool(termios.tcgetattr(self.descripteur)[3] & termios.ECHO)

    def en_vie(self):
        if self.pid is None or self._code is not None:
            return False
        try:
            pid, statut = os.waitpid(self.pid, os.WNOHANG)
        except ChildProcessError:
            self._code = -1
            return False
        if pid == 0:
            return True
        self._code = os.waitstatus_to_exitcode(statut)
        return False

    @property
    def code_de_sortie(self):
        return self._code

    def arreter(self, delai=1.0):
        """SIGHUP au shell et à la commande au premier plan, SIGKILL après `delai` ; aucun
        processus ni fichier de démarrage ne reste. Sans effet la seconde fois."""
        if self.pid is not None and self.en_vie():
            groupes = {self.pid}
            try:
                groupes.add(os.tcgetpgrp(self.descripteur))
            except OSError:
                pass
            self._signaler(groupes, signal.SIGHUP)
            limite = time.monotonic() + delai
            while self.en_vie() and time.monotonic() < limite:
                time.sleep(0.02)
            if self.en_vie():
                self._signaler(groupes, signal.SIGKILL)
                try:
                    _, statut = os.waitpid(self.pid, 0)
                    self._code = os.waitstatus_to_exitcode(statut)
                except ChildProcessError:
                    self._code = -1
        if self.descripteur is not None:
            os.close(self.descripteur)
            self.descripteur = None
        if self.fichier_de_demarrage is not None:
            shutil.rmtree(os.path.dirname(self.fichier_de_demarrage), ignore_errors=True)
        _SESSIONS.discard(self)

    @staticmethod
    def _signaler(groupes, numero):
        for groupe in groupes:
            try:
                os.killpg(groupe, numero)
            except (ProcessLookupError, PermissionError):
                pass


# ----- Complétion par Tab (APPARENCE.md, section 11 ; 8.4) -----

COMMANDES_DE_DOSSIER = ("cd", "pushd", "rmdir")


@dataclass(frozen=True)
class Completion:
    """Ligne complétée de ce qui est sans ambiguïté, et possibilités (un dossier finit par « / »)."""

    ligne: str
    candidats: list


def _dernier_mot(ligne):
    """(avant, mot) : le dernier mot commence après la dernière espace non précédée de « \\ »."""
    rang = len(ligne)
    while rang > 0:
        if ligne[rang - 1] == " " and not (rang > 1 and ligne[rang - 2] == "\\"):
            break
        rang -= 1
    return ligne[:rang], ligne[rang:]


def _prefixe_commun(noms):
    if not noms:
        return ""
    premier, dernier = min(noms), max(noms)
    rang = 0
    while rang < len(premier) and rang < len(dernier) and premier[rang] == dernier[rang]:
        rang += 1
    return premier[:rang]


def completer(ligne, dossier, chemins=None, maison=None):
    """Complète le dernier mot de `ligne`, sans rien exécuter : fichiers et dossiers de `dossier`
    (le dossier courant du shell), dossiers seuls après `cd`, `pushd`, `rmdir` ; commandes des
    `chemins` (le PATH) pour un premier mot sans « / ». Les noms cachés ne sont proposés que si
    le mot commence par un point ; une espace s'écrit « \\  »."""
    avant, mot = _dernier_mot(ligne)
    brut = mot.replace("\\ ", " ")
    premier_mot = not avant.strip()
    if premier_mot and "/" not in brut:
        noms = set()
        for chemin in chemins if chemins is not None else os.environ.get("PATH", "").split(os.pathsep):
            try:
                for nom in os.listdir(chemin):
                    if nom.startswith(brut) and os.access(os.path.join(chemin, nom), os.X_OK) \
                            and not os.path.isdir(os.path.join(chemin, nom)):
                        noms.add(nom)
            except OSError:
                continue
        candidats = sorted(noms) if brut else []
        suffixes = {nom: " " for nom in candidats}
        tete = ""
    else:
        tete, _, debut = brut.rpartition("/")
        tete = tete + "/" if "/" in brut else ""
        reel = tete
        if reel.startswith("~"):
            reel = (maison if maison is not None else os.path.expanduser("~")) + reel[1:]
        base = reel if os.path.isabs(reel) else os.path.join(dossier or ".", reel)
        commande = avant.split()[0] if avant.split() else ""
        candidats, suffixes = [], {}
        try:
            entrees = sorted(os.listdir(base))
        except OSError:
            entrees = []
        for nom in entrees:
            if not nom.startswith(debut) or (nom.startswith(".") and not debut.startswith(".")):
                continue
            est_dossier = os.path.isdir(os.path.join(base, nom))
            if commande in COMMANDES_DE_DOSSIER and not est_dossier:
                continue
            candidats.append(nom)
            suffixes[nom] = "/" if est_dossier else " "
    if not candidats:
        return Completion(ligne, [])
    affiches = [nom + ("/" if suffixes[nom] == "/" else "") for nom in candidats]
    if len(candidats) == 1:
        complet = candidats[0].replace(" ", "\\ ") + suffixes[candidats[0]]
    else:
        commun = _prefixe_commun(candidats)
        debut_tape = brut[len(tete):] if not (premier_mot and "/" not in brut) else brut
        complet = (commun if len(commun) > len(debut_tape) else debut_tape).replace(" ", "\\ ")
    return Completion(avant + tete.replace(" ", "\\ ") + complet, affiches)
