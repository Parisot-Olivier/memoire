"""Catalogue des actions de l'interface (ACTIONS.md ; D30 ; INV-UI-01).

Aucune importation de tkinter. Chaque action est une intention en mots simples : son groupe dans le
menu, sa phrase d'aide, ses questions (le minimum), ses sections à cocher (le reste), les commandes
qu'elle produit à partir des réponses, et ses suites « Et ensuite ? ». La ligne de commande garde
ses termes ; la table VOCABULAIRE relie les mots de l'interface aux valeurs des commandes.
"""
from dataclasses import dataclass, field

GROUPES = ("Écrire", "Relier", "Juger", "Lire", "Code", "Histoire et pistes", "Où j’en suis",
           "Projet")

# Commandes laissées au seul mode expert.
EXPERT_SEULEMENT = ("aide",)

# Mots de l'interface pour les valeurs de la ligne de commande (ACTIONS.md, section 3).
VOCABULAIRE = {
    "role": {"categorie": "thème", "raisonnement": "idée", "implementation": "solution technique",
             "note": "note libre", "echange": "échange avec une IA", "citation": "passage cité",
             "reserve": "doute à suivre"},
    "relation": {"classe-dans": "rangée dans le thème", "depend-de": "s’appuie sur",
                 "developpe-a-partir-de": "vient de", "lien": "en rapport avec",
                 "remplace": "remplace", "restreint": "ne vaut que si", "contredit": "contredit",
                 "concerne": "concerne", "traite": "répond à"},
    "verdict": {"non-verifiee": "pas encore vérifiée", "verifiee": "vérifiée (c’est vrai)",
                "refutee": "réfutée (c’est faux)"},
    "decision": {"proposee": "proposée", "retenue": "retenue", "plus-retenue": "mise de côté",
                 "remplacee": "remplacée"},
    "implementation": {"non-commencee": "pas commencé", "en-cours": "en cours",
                       "implementee": "fait"},
    "reserve": {"ouverte": "ouvert", "traitee-a-verifier": "traité, à vérifier",
                "resolue": "levé"},
}


def mot(categorie, valeur):
    """Mot de l'interface pour une valeur de la ligne de commande ; la valeur elle-même sinon."""
    return VOCABULAIRE.get(categorie, {}).get(valeur, valeur)


def _choix(categorie, valeurs):
    return tuple((mot(categorie, v), v) for v in valeurs)


@dataclass(frozen=True)
class Question:
    """Une question : `genre` texte, texte_long, note, notes, choix, dossier, fichier, piste,
    etape, reprise ou interne (rempli par un parcours guidé, jamais affiché). `si` : (clé,
    valeur) d'une autre réponse qui la fait apparaître. `vers_fichier` : l'interface écrit la
    réponse dans un fichier temporaire et passe son chemin."""

    cle: str
    libelle: str
    genre: str = "texte"
    obligatoire: bool = True
    choix: tuple = ()
    exemple: str = "x"
    si: tuple = None
    vers_fichier: bool = False


@dataclass(frozen=True)
class Section:
    """Une case à cocher (« La ranger dans un thème ») et les questions qu'elle fait apparaître."""

    cle: str
    libelle: str
    questions: tuple = ()


@dataclass(frozen=True)
class Suite:
    """Une suite « Et ensuite ? » : action proposée et réponses pré-remplies (clé de la question
    de l'action proposée → « objet » ou clé d'une réponse de l'action faite)."""

    action: str
    libelle: str
    remplir: dict = field(default_factory=dict)


@dataclass(frozen=True)
class Action:
    cle: str
    groupe: str
    titre: str
    aide: str
    questions: tuple = ()
    sections: tuple = ()
    construire: object = None
    importante: bool = False
    suites: tuple = ()
    objet: str = None
    special: str = None
    emploie: tuple = ()

    def toutes_les_questions(self):
        return self.questions + tuple(q for s in self.sections for q in s.questions)

    def question(self, cle):
        return next(q for q in self.toutes_les_questions() if q.cle == cle)

    def visibles(self, reponses):
        """Questions principales à afficher, d'après les réponses déjà données."""
        return [q for q in self.questions if q.genre != "interne"
                and (q.si is None or reponses.get(q.si[0]) == q.si[1])]


class _Reponses:
    """Accès aux réponses retenues : question visible, section cochée, valeur non vide."""

    def __init__(self, action, reponses, sections):
        self._valeurs = {}
        for q in action.questions:
            if q.si is None or reponses.get(q.si[0]) == q.si[1]:
                self._valeurs[q.cle] = reponses.get(q.cle)
        for s in action.sections:
            if s.cle in sections:
                self._valeurs[s.cle] = True
                for q in s.questions:
                    self._valeurs[q.cle] = reponses.get(q.cle)

    def __call__(self, cle):
        valeur = self._valeurs.get(cle)
        if valeur is None or (isinstance(valeur, str) and not valeur.strip()):
            return None
        return valeur

    def option(self, nom, cle):
        valeur = self(cle)
        return [nom, valeur] if valeur is not None else []

    def drapeau(self, nom, cle):
        return [nom] if self(cle) else []


def commandes_de(action, reponses, sections):
    """Lignes de commande (sans « memoire » ni « --base ») produites par les réponses."""
    if action.construire is None:
        return []
    return action.construire(_Reponses(action, reponses, set(sections)))


def exemples(action):
    """Réponses d'essai pour chaque question, toutes sections cochées (essais, INV-UI-01)."""
    reponses = {}
    for q in action.toutes_les_questions():
        reponses[q.cle] = q.choix[0][1] if q.choix else q.exemple
    return reponses, {s.cle for s in action.sections}


APERCU = (
    ("Opération critique :", "Ce que ça va changer :"),
    ("  - une branche sera créée depuis main",
     "  - ce sera fait sur une nouvelle piste, pour ne pas toucher à la piste principale"),
    ("  - n'est plus à réexaminer : ", "  - n’est plus à revoir : "),
    ("  - à réexaminer : ", "  - à revoir : "),
)


def traduire_apercu(texte):
    """L'aperçu d'une opération importante, dans le vocabulaire de l'interface."""
    lignes = []
    for ligne in texte.splitlines(keepends=True):
        for avant, apres in APERCU:
            if ligne.startswith(avant):
                ligne = apres + ligne[len(avant):]
                break
        lignes.append(ligne)
    return "".join(lignes)


# ----- Questions courantes -----

NOTE = "aaaaaaaa"


def _note(cle="note", libelle="Quelle note ?", obligatoire=True):
    return Question(cle, libelle, "note", obligatoire, exemple=NOTE)


def _notes(cle, libelle="Lesquelles ?"):
    return Question(cle, libelle, "notes", exemple=f"{NOTE},bbbbbbbb")


def _pourquoi(libelle="Pourquoi ?", cle="motif"):
    return Question(cle, libelle)


def _section_pourquoi():
    return Section("pourquoi", "Noter pourquoi", (_pourquoi(),))


GENRES = (("Idée", "raisonnement"), ("Solution technique", "implementation"),
          ("Note libre", "note"), ("Thème (pour ranger)", "categorie"))

LIENS = (("s’appuie sur", "depend-de"), ("vient de", "developpe-a-partir-de"),
         ("est rangée dans le thème", "classe-dans"), ("est en rapport avec", "lien"),
         ("ne vaut que si…", "ne-vaut-que-si"), ("contredit", "contredit"))


def _relier(r):
    lien, origine, destination = r("lien"), r("origine"), r("destination")
    motif = r.option("--motif", "motif")
    if lien == "ne-vaut-que-si":
        return [["restreindre", origine, destination, "--condition", r("condition")] + motif]
    if lien == "contredit":
        return [["contredire", origine, destination, "--justification", r("raison")] + motif]
    return [["lier", origine, lien, destination] + motif]


def _ancrer(r):
    endroit = r.option("--fonction", "fonction") if r("endroit") == "fonction" else \
        r.option("--lignes", "lignes")
    return [["ancrer", r("note"), "--fichier", r("fichier")] + endroit + r.option("--git", "git")]


def _adopter(r):
    commandes = [["basculer", r("pour")]] if r("pour") else []
    commandes.append(["fusionner", r("piste"), "--plan", r("plan")] + r.option("--cible", "cible")
                     + r.option("--source", "source") + r.option("--desactiver", "desactiver")
                     + r.option("--motif", "motif"))
    return commandes


ACTIONS = (
    # ----- Écrire -----
    Action(
        "nouvelle_note", "Écrire", "Nouvelle note",
        "Une idée, une solution technique, une note libre, ou un thème pour ranger les autres.",
        (Question("titre", "Titre ?"),
         Question("genre", "Quel genre de note ?", "choix", choix=GENRES)),
        (Section("texte", "Ajouter un texte", (Question("texte", "Le texte", "texte_long"),)),
         Section("appuis", "Elle s’appuie sur d’autres notes", (_notes("appuis"),)),
         Section("origine", "Elle vient d’une note ou d’un échange",
                 (_notes("origine", "Laquelle ?"),)),
         Section("theme", "La ranger dans un thème", (_notes("theme", "Quel thème ?"),)),
         Section("rapport", "En rapport avec d’autres notes", (_notes("rapport"),))),
        lambda r: [["note", "creer", "--role", r("genre"), "--titre", r("titre")]
                   + r.option("--texte", "texte") + r.option("--depend-de", "appuis")
                   + r.option("--developpe-a-partir-de", "origine")
                   + r.option("--classe-dans", "theme") + r.option("--lien", "rapport")],
        suites=(Suite("lier_au_code", "La lier au code", {"note": "objet"}),
                Suite("relier", "La relier à une autre note", {"origine": "objet"}),
                Suite("lire", "La lire", {"note": "objet"})),
        objet="Note créée : "),
    Action(
        "nouvel_echange", "Écrire", "Nouvel échange avec une IA",
        "Une question posée à une IA et sa réponse, gardées telles quelles. Collez simplement le "
        "texte.",
        (Question("titre", "Titre de l’échange ?"),
         Question("reponse", "La réponse de l’IA (collez-la)", "texte_long", vers_fichier=True)),
        (Section("question", "Ajouter la question posée",
                 (Question("question", "La question (collez-la)", "texte_long",
                           vers_fichier=True),)),
         Section("precedent", "C’est la suite d’un autre échange",
                 (_note("precedent", "Quel échange ?"),))),
        lambda r: [["echange", "coller", "--titre", r("titre"), "--reponse-fichier", r("reponse")]
                   + r.option("--question-fichier", "question")
                   + r.option("--precedent", "precedent")],
        suites=(Suite("citer", "Citer un passage de cet échange", {"note": "objet"}),
                Suite("nouvelle_note", "Une note qui en vient", {"origine": "objet"}),
                Suite("lire", "Le lire", {"note": "objet"})),
        objet="Échange créé : "),
    Action(
        "citer", "Écrire", "Citer un passage",
        "Garder un extrait exact d’une note ou d’un échange. Le plus simple : le sélectionner en "
        "lisant (bouton « Choisir en lisant »).",
        (_note("note", "Dans quelle note ou quel échange ?"),
         Question("texte", "Le passage exact")),
        (Section("dans_la_question", "Le passage est dans la question"),
         Section("soutient", "Il appuie des notes", (_notes("soutient"),)),
         Section("inspire", "Des notes en viennent", (_notes("inspire"),)),
         Section("theme", "Le ranger dans un thème", (_notes("theme", "Quel thème ?"),)),
         Section("doute", "En tirer un doute à suivre", (Question("doute", "Le doute ?"),))),
        lambda r: [["citer", r("note"), "--texte", r("texte")]
                   + (["--partie", "question"] if r("dans_la_question") else [])
                   + r.option("--soutient", "soutient") + r.option("--inspire", "inspire")
                   + r.option("--classe-dans", "theme") + r.option("--reserve", "doute")],
        suites=(Suite("nouvelle_note", "Une note qui en vient", {"origine": "objet"}),
                Suite("nouveau_doute", "En tirer un doute à suivre", {"passage": "objet"})),
        objet="Citation créée : ", special="citer"),
    Action(
        "reviser", "Écrire", "Modifier le texte d’une note",
        "Le texte change ; l’ancienne version reste dans l’histoire.",
        (_note(), Question("texte", "Le nouveau texte", "texte_long"),
         Question("nature", "Ce changement est…", "choix",
                  choix=(("une retouche (le sens ne change pas)", "mineure"),
                         ("un changement de sens", "substantielle")))),
        (_section_pourquoi(),),
        lambda r: [["note", "reviser", r("note"), "--texte", r("texte"), "--nature", r("nature")]
                   + r.option("--motif", "motif")],
        suites=(Suite("lire", "La lire", {"note": "note"}),), objet="reponse:note"),
    Action(
        "completer_question", "Écrire", "Ajouter la question d’un échange",
        "Pour un échange enregistré sans sa question.",
        (_note("echange", "Quel échange ?"),
         Question("question", "La question (collez-la)", "texte_long", vers_fichier=True)),
        construire=lambda r: [["echange", "question", r("echange"), "--fichier", r("question")]],
        suites=(Suite("lire", "Le lire", {"note": "echange"}),), objet="reponse:echange"),
    Action(
        "nouveau_doute", "Écrire", "Nouveau doute à suivre",
        "Un avertissement tiré d’un passage cité d’un échange, à garder en tête jusqu’à le lever.",
        (_note("passage", "De quel passage cité ?"), Question("titre", "Le doute, en quelques mots ?")),
        (Section("cible", "Préciser ce qu’il vise", (Question("cible", "Ce qu’il vise ?"),)),),
        lambda r: [["reserve", "creer", r("passage"), "--titre", r("titre")]
                   + r.option("--cible", "cible")],
        suites=(Suite("doute_concerne", "Il concerne une note", {"doute": "objet"}),),
        objet="Réserve créée : "),
    # ----- Relier -----
    Action(
        "relier", "Relier", "Relier deux notes",
        "« Cette note … l’autre ». S’appuie sur et vient de : si l’autre tombe, celle-ci devra être "
        "revue.",
        (_note("origine", "Quelle note ?"),
         Question("lien", "Comment ?", "choix", choix=LIENS),
         _note("destination", "Quelle autre note ?"),
         Question("condition", "À quelle condition ?", si=("lien", "ne-vaut-que-si")),
         Question("raison", "Pourquoi ?", si=("lien", "contredit"))),
        (_section_pourquoi(),), _relier,
        suites=(Suite("voisinage", "Voir son voisinage", {"note": "origine"}),
                Suite("relier", "La relier encore", {"origine": "origine"})),
        objet="reponse:origine", emploie=("restreindre", "contredire")),
    Action(
        "ranger", "Relier", "Ranger une note dans un thème", "Un thème regroupe des notes.",
        (_note(), _note("theme", "Quel thème ?")), (_section_pourquoi(),),
        lambda r: [["lier", r("note"), "classe-dans", r("theme")] + r.option("--motif", "motif")],
        suites=(Suite("voisinage", "Voir son voisinage", {"note": "note"}),),
        objet="reponse:note"),
    Action(
        "doute_concerne", "Relier", "Ce doute concerne une note",
        "Le doute reste ouvert ; il est maintenant rattaché à la note.",
        (_note("doute", "Quel doute ?"), _note()), (_section_pourquoi(),),
        lambda r: [["reserve", "concerne", r("doute"), r("note")] + r.option("--motif", "motif")]),
    Action(
        "repond_au_doute", "Relier", "Cette note répond à un doute",
        "Le doute passe à « traité, à vérifier ».",
        (_note(), _note("doute", "Quel doute ?"), _pourquoi("Comment y répond-elle ?")),
        construire=lambda r: [["reserve", "traiter", r("note"), r("doute"), "--motif", r("motif")]]),
    # ----- Juger -----
    Action(
        "verifier", "Juger", "Vérifier une note (c’est vrai)",
        "La note est confirmée.", (_note(), _pourquoi("Comment l’avez-vous vérifiée ?")),
        construire=lambda r: [["verdict", r("note"), "verifiee", "--motif", r("motif")]],
        importante=True, objet="reponse:note",
        suites=(Suite("ou_en_est_le_projet", "Où en est le projet ?"),)),
    Action(
        "refuter", "Juger", "Réfuter une note (c’est faux)",
        "La note est fausse ; ce qui s’appuie dessus devra être revu. Rien n’est effacé.",
        (_note(), _pourquoi("Pourquoi est-elle fausse ?")),
        construire=lambda r: [["verdict", r("note"), "refutee", "--motif", r("motif")]],
        importante=True, objet="reponse:note",
        suites=(Suite("ou_en_est_le_projet", "Revoir les notes concernées"),
                Suite("garder_quand_meme", "Garder une note quand même", {"cause": "note"}),
                Suite("aller_sur_piste", "Revenir sur la piste principale"))),
    Action(
        "remettre_a_verifier", "Juger", "Remettre une note « à vérifier »",
        "Annule un « vérifiée » ou un « réfutée » donné trop tôt.",
        (_note(), _pourquoi()),
        construire=lambda r: [["verdict", r("note"), "non-verifiee", "--motif", r("motif")]],
        importante=True, objet="reponse:note"),
    Action(
        "decider", "Juger", "Retenir ou mettre de côté une idée",
        "Mise de côté : écartée, mais gardée en mémoire ; ce qui s’appuie dessus devra être revu.",
        (_note("note", "Quelle idée ou solution ?"),
         Question("decision", "Elle est…", "choix",
                  choix=_choix("decision", ("retenue", "proposee", "plus-retenue"))),
         _pourquoi()),
        construire=lambda r: [["decision", r("note"), r("decision"), "--motif", r("motif")]],
        importante=True, objet="reponse:note",
        suites=(Suite("ou_en_est_le_projet", "Où en est le projet ?"),)),
    Action(
        "avancement", "Juger", "Avancement d’une solution technique",
        "Pas commencé, en cours ou fait.",
        (_note("note", "Quelle solution ?"),
         Question("etat", "Où en est-elle ?", "choix",
                  choix=_choix("implementation", ("non-commencee", "en-cours", "implementee"))),
         _pourquoi()),
        construire=lambda r: [["implementation", r("note"), r("etat"), "--motif", r("motif")]],
        importante=True, objet="reponse:note"),
    Action(
        "garder_quand_meme", "Juger", "Garder une note quand même",
        "Une note à revoir reste valable malgré la note qui a changé.",
        (_note("dependant", "Quelle note garder ?"), _note("cause", "Malgré quelle note ?"),
         _pourquoi("Pourquoi reste-t-elle valable ?")),
        construire=lambda r: [["acquitter", r("dependant"), r("cause"), "--motif", r("motif")]],
        importante=True, suites=(Suite("ou_en_est_le_projet", "Où en est le projet ?"),)),
    Action(
        "retirer", "Juger", "Retirer une note des notes en vigueur",
        "Elle n’est plus utilisée, mais reste dans l’histoire ; ce qui s’appuie dessus devra être "
        "revu.",
        (_note(),), (_section_pourquoi(),),
        lambda r: [["retirer", r("note")] + r.option("--motif", "motif")],
        importante=True, suites=(Suite("ou_en_est_le_projet", "Où en est le projet ?"),)),
    Action(
        "etat_du_doute", "Juger", "Lever ou rouvrir un doute",
        "Un doute est ouvert, traité (à vérifier) ou levé.",
        (_note("doute", "Quel doute ?"),
         Question("etat", "Il est…", "choix",
                  choix=_choix("reserve", ("resolue", "traitee-a-verifier", "ouverte"))),
         _pourquoi()),
        construire=lambda r: [["reserve", "etat", r("doute"), r("etat"), "--motif", r("motif")]],
        importante=True),
    # ----- Lire -----
    Action("ou_en_est_le_projet", "Lire", "Où en est le projet ?",
           "Les notes en vigueur, celles à revoir et celles sans appui, sur la piste actuelle.",
           construire=lambda r: [["statut"]]),
    Action("chercher", "Lire", "Chercher une note", "Par un mot de son titre.",
           (Question("texte", "Quel mot chercher ?"),),
           construire=lambda r: [["chercher", r("texte")]]),
    Action("lire", "Lire", "Lire une note ou un échange",
           "Le texte mis en forme, les passages cités surlignés ; on peut y citer une sélection.",
           (_note("note", "Quelle note ou quel échange ?"),), special="lire",
           emploie=("source", "citations")),
    Action("voisinage", "Lire", "Voisinage d’une note",
           "Les notes reliées à celle-ci, et comment ; un clic pour se déplacer.",
           (_note(),), special="voisinage", emploie=("voisinage",)),
    # ----- Code -----
    Action(
        "lier_au_code", "Code", "Lier une note au code",
        "Donne le commentaire à coller dans votre code ; votre fichier n’est jamais modifié.",
        (_note(), Question("fichier", "Quel fichier ?", "fichier", exemple="src/a.c"),
         Question("endroit", "Quel endroit ?", "choix",
                  choix=(("une fonction", "fonction"), ("des lignes", "lignes"))),
         Question("fonction", "La signature de la fonction ?", si=("endroit", "fonction")),
         Question("lignes", "Quelles lignes ? (ex. 40-62)", si=("endroit", "lignes"),
                  exemple="40-62")),
        (Section("git", "Noter la version du code", (Question("git", "Quel commit Git ?"),)),),
        _ancrer, special="copier_repere", objet="reponse:note",
        suites=(Suite("verifier_code", "Vérifier les liens avec le code"),)),
    Action(
        "notes_du_code", "Code", "Quelles notes concernent ce code ?",
        "Donnez un fichier, et si vous voulez une ligne ou un extrait : les notes liées et les "
        "marques. Rien n’est modifié.",
        (Question("fichier", "Quel fichier ?", "fichier", exemple="src/a.c"),),
        (Section("ligne", "Une ligne précise",
                 (Question("ligne", "Quelle ligne ?", exemple="42"),)),
         Section("extrait", "Un extrait collé",
                 (Question("extrait", "Collez l’extrait", "texte_long"),))),
        lambda r: [["code", "notes", r("fichier")]
                   + (r.option("--ligne", "ligne") or r.option("--extrait", "extrait"))],
        special="notes_du_code"),
    Action("que_designe", "Code", "Que désigne cette marque ?",
           "Collez une ligne de votre code qui contient une marque @design, quel que soit le "
           "commentaire.",
           (Question("ligne", "La ligne avec la marque ?",
                     exemple=f"// @design: {NOTE}-0000-0000-0000-000000000000"),),
           construire=lambda r: [["repere", r("ligne")]]),
    Action(
        "verifier_code", "Code", "Vérifier les liens avec le code",
        "Parcourt votre code : marques retrouvées, déplacées, perdues… Rien n’est modifié.",
        (Question("dossier", "Quel dossier de code ?", "dossier", exemple="src"),),
        (Section("extensions", "Seulement certains fichiers",
                 (Question("extensions", "Quelles extensions ? (ex. c,h)"),)),),
        lambda r: [["indexer", r("dossier")] + r.option("--extensions", "extensions")]),
    # ----- Histoire et pistes -----
    Action("annuler", "Histoire et pistes", "Annuler la dernière action",
           "Revient juste avant la dernière action de la piste actuelle.",
           construire=lambda r: [["annuler"]], suites=(Suite("retablir", "Rétablir"),)),
    Action("retablir", "Histoire et pistes", "Rétablir l’action annulée",
           "Refait la dernière action annulée.", construire=lambda r: [["retablir"]]),
    Action("etapes", "Histoire et pistes", "Les dernières étapes",
           "Ce qui a été fait sur la piste actuelle, du plus récent au plus ancien.",
           construire=lambda r: [["historique"]]),
    Action(
        "revenir_a_une_etape", "Histoire et pistes", "Revenir à une étape",
        "Tout le projet, ou seulement certaines notes, tels qu’à une étape passée. Rien n’est "
        "effacé, et on peut annuler.",
        (Question("etape", "Quelle étape ?", "etape", exemple="aaaaaaaa"), _pourquoi()),
        (Section("notes", "Seulement certaines notes", (_notes("notes"),)),),
        lambda r: [["restaurer", "--depuis", r("etape")] + r.option("--notes", "notes")
                   + r.option("--motif", "motif")],
        importante=True, emploie=("restauration plan", "historique"),
        suites=(Suite("ou_en_est_le_projet", "Où en est le projet ?"),
                Suite("annuler", "Annuler ce retour"))),
    Action("pistes", "Histoire et pistes", "Les pistes",
           "Les versions parallèles du projet ; * marque celle où vous êtes.",
           construire=lambda r: [["branches"]]),
    Action("aller_sur_piste", "Histoire et pistes", "Aller sur une piste",
           "Travailler sur une autre piste ; rien n’est perdu.",
           (Question("piste", "Quelle piste ?", "piste", exemple="aaaaaaaa"),),
           construire=lambda r: [["basculer", r("piste")]]),
    Action("mettre_piste_de_cote", "Histoire et pistes", "Mettre une piste de côté",
           "Elle disparaît de la liste, sans rien supprimer ; on peut la reprendre.",
           (Question("piste", "Quelle piste ?", "piste", exemple="aaaaaaaa"),),
           (_section_pourquoi(),),
           lambda r: [["abandonner", r("piste")] + r.option("--motif", "motif")]),
    Action("reprendre_piste", "Histoire et pistes", "Reprendre une piste mise de côté",
           "Elle revient dans la liste des pistes.",
           (Question("piste", "Quelle piste ?", "piste", exemple="aaaaaaaa"),),
           construire=lambda r: [["reactiver", r("piste")]]),
    Action(
        "adopter_piste", "Histoire et pistes", "Adopter une piste",
        "Les changements d’une piste deviennent la version retenue d’une autre, souvent la "
        "principale. Une sauvegarde est faite avant.",
        (Question("piste", "Quelle piste adopter ?", "piste", exemple="aaaaaaaa"),
         Question("pour", "Pour quelle piste ?", "piste", obligatoire=False, exemple="bbbbbbbb"),
         Question("plan", "Code du plan", "interne", exemple="abcd1234abcd1234"),
         Question("cible", "Désaccords gardés ici", "interne", obligatoire=False, exemple="1"),
         Question("source", "Désaccords pris dans la piste", "interne", obligatoire=False,
                  exemple="2"),
         Question("desactiver", "Relations désactivées", "interne", obligatoire=False,
                  exemple="1"),
         Question("motif", "Pourquoi ?", obligatoire=False)),
        construire=_adopter, importante=True, special="adopter",
        emploie=("fusion plan", "branches"),
        suites=(Suite("mettre_piste_de_cote", "Mettre la piste adoptée de côté", {"piste": "piste"}),
                Suite("ou_en_est_le_projet", "Où en est le projet ?"))),
    # ----- Où j'en suis -----
    Action(
        "noter_ou_j_en_suis", "Où j’en suis", "Noter où j’en suis",
        "Pour reprendre facilement : ce que vous faites, la prochaine étape, et l’état de votre "
        "code (lu dans Git, sans rien y écrire).",
        (Question("objectif", "Sur quoi travaillez-vous ?"),
         Question("action", "Quelle est la prochaine chose à faire ?")),
        (Section("bloque", "Quelque chose bloque", (Question("question", "Qu’est-ce qui bloque ?"),)),
         Section("etat", "Où vous en êtes arrivé", (Question("etat", "Où en êtes-vous ?"),)),
         Section("resultat", "Le dernier résultat (test, compilation…)",
                 (Question("resultat", "Quel résultat ?"),)),
         Section("hypothese", "Votre hypothèse du moment", (Question("hypothese", "Laquelle ?"),)),
         Section("ouverts", "Fichiers ou fonctions ouverts", (Question("ouverts", "Lesquels ?"),)),
         Section("notes", "Notes concernées", (_notes("notes"),)),
         Section("ailleurs", "Le code est dans un autre dossier",
                 (Question("code", "Quel dossier de code ?", "dossier", exemple="src"),)),
         Section("sans_code", "Ne pas noter l’état du code")),
        lambda r: [["reprise", "noter", "--objectif", r("objectif"), "--action", r("action")]
                   + r.option("--question", "question") + r.option("--etat", "etat")
                   + r.option("--resultat", "resultat") + r.option("--hypothese", "hypothese")
                   + r.option("--ouverts", "ouverts") + r.option("--notes", "notes")
                   + (r.option("--code", "code") if not r("sans_code") else [])
                   + r.drapeau("--sans-code", "sans_code")]),
    Action("ou_en_etais_je", "Où j’en suis", "Où en étais-je ?",
           "Le dernier point noté, et ce qui a changé depuis (projet et code).",
           construire=lambda r: [["reprise", "voir"]]),
    Action("mes_points", "Où j’en suis", "Mes points de reprise",
           "Tous les points notés sur la piste actuelle.", construire=lambda r: [["reprise", "liste"]]),
    Action("recuperer_code", "Où j’en suis", "Récupérer le code non enregistré noté",
           "Le travail non commité noté avec un point de reprise, à réappliquer par git apply.",
           (Question("point", "Quel point de reprise ?", "reprise", exemple="aaaaaaaa"),),
           construire=lambda r: [["reprise", "code", r("point")]]),
    # ----- Projet -----
    Action("creer_projet", "Projet", "Créer le projet",
           "Crée le fichier du projet, s’il n’existe pas encore.", construire=lambda r: [["init"]]),
    Action("sauvegarder", "Projet", "Sauvegarder le projet",
           "Une copie datée, dans « sauvegardes » à côté du projet.",
           sections=(Section("ailleurs", "Choisir le fichier de sauvegarde",
                             (Question("fichier", "Quel fichier ?", "fichier", exemple="copie.db"),)),),
           construire=lambda r: [["sauvegarder"] + ([r("fichier")] if r("fichier") else [])]),
    Action("exporter", "Projet", "Exporter le projet en texte",
           "Tout le projet dans un fichier texte (pour Git ou un autre ordinateur).",
           (Question("fichier", "Vers quel fichier ?", "fichier", exemple="projet.txt"),),
           construire=lambda r: [["exporter", r("fichier")]]),
    Action("importer", "Projet", "Importer un projet exporté",
           "Crée le projet à partir d’un export, vérifié en entier avant.",
           (Question("fichier", "Quel fichier ?", "fichier", exemple="projet.txt"),),
           construire=lambda r: [["importer", r("fichier")]]),
    Action("verifier_projet", "Projet", "Vérifier le projet en entier",
           "Relit toute l’histoire pour s’assurer que rien n’est abîmé.",
           construire=lambda r: [["verifier"]]),
    Action("mettre_a_jour_base", "Projet", "Mettre à jour une ancienne base",
           "Convertit un projet d’une ancienne version, après une sauvegarde.",
           construire=lambda r: [["migrer"]]),
)

_PAR_CLE = {a.cle: a for a in ACTIONS}


def action(cle):
    """L'action de cette clé ; KeyError si elle n'existe pas."""
    return _PAR_CLE[cle]


# ----- Ce que ça va changer, dernières étapes (APPARENCE.md, section 9.3 ; 8.2b) -----

# Lignes de commande en lecture seule : rien ne change.
LECTURE = {"statut", "chercher", "historique", "branches", "repere", "indexer", "verifier",
           "source", "citations", "voisinage", ("code", "notes"), ("reprise", "voir"),
           ("reprise", "liste"), ("reprise", "code"), ("fusion", "plan"),
           ("restauration", "plan")}
# Rôles au masculin (« thème créé ») ; les autres au féminin (« idée créée »).
ROLES_MASCULINS = {"categorie", "echange", "citation", "reserve"}
SIGNES_DE_VERDICT = {"verifiee": "✓", "refutee": "✕", "non-verifiee": "?"}
SIGNES_DE_DECISION = {"retenue": "★", "proposee": "+", "plus-retenue": "–", "remplacee": "⇄"}
LIENS_DE_NOTE = (("--depend-de", "→", "s’appuie sur"), ("--developpe-a-partir-de", "←", "vient de"),
                 ("--classe-dans", "◇", "rangée dans le thème"), ("--lien", "~", "en rapport avec"))


def _decouper(argv):
    """(positions, options) d'une ligne de commande ; chaque option a une valeur."""
    positions, options, rang = [], {}, 0
    while rang < len(argv):
        mot_ = argv[rang]
        if isinstance(mot_, str) and mot_.startswith("--"):
            options[mot_] = argv[rang + 1] if rang + 1 < len(argv) else None
            rang += 2
        else:
            positions.append(mot_)
            rang += 1
    return positions, options


def decrire(argv, noms=None):
    """« Ce que ça va changer » : (signe, quoi, objet) pour une ligne de commande du catalogue.

    Une note s'écrit « nom · id8 » dans les réponses : seul le nom est gardé ; `noms` donne le
    nom affiché d'un identifiant (piste, étape) ; une réponse manquante s'écrit « … ». C'est ce
    que le formulaire demandera, pas un calcul du moteur : celui-ci ne donne d'aperçu que pour
    une opération critique."""
    noms = noms or {}

    def nom(valeur):
        if valeur is None or not str(valeur).strip():
            return "…"
        valeur = noms.get(str(valeur), str(valeur)).strip()
        return valeur.rsplit(" · ", 1)[0] if " · " in valeur else valeur

    def plusieurs(valeur):
        if valeur is None or not str(valeur).strip():
            return ["…"]
        return [nom(v) for v in str(valeur).split(",") if v.strip()]

    if not argv:
        return [("·", "rien ne change", "aucune commande")]
    positions, options = _decouper(list(argv))
    tete = positions[0]
    paire = (tete, positions[1]) if len(positions) > 1 else None
    if tete in LECTURE or paire in LECTURE:
        return [("·", "rien ne change", "lecture seule")]
    p = positions + [None] * 4
    lignes = []
    if paire == ("note", "creer"):
        role = options.get("--role")
        genre = "créé" if role in ROLES_MASCULINS else "créée"
        lignes.append(("+", f"{mot('role', role) if role else 'note'} {genre}",
                       nom(options.get("--titre"))))
        if options.get("--texte"):
            lignes.append(("¶", "avec un texte", f"{len(str(options['--texte']))} caractères"))
        for option, signe, quoi in LIENS_DE_NOTE:
            if option in options:
                lignes += [(signe, quoi, n) for n in plusieurs(options[option])]
    elif paire == ("echange", "coller"):
        lignes.append(("+", "échange avec une IA créé", nom(options.get("--titre"))))
        if options.get("--question-fichier"):
            lignes.append(("¶", "avec la question posée", ""))
        if "--precedent" in options:
            lignes.append(("←", "suite de", nom(options["--precedent"])))
    elif tete == "citer":
        texte = options.get("--texte")
        lignes.append(("+", "passage cité", f"« {texte} »" if texte else "…"))
        partie = "la question de" if options.get("--partie") == "question" else "dans"
        lignes.append(("·", partie, nom(p[1])))
        lignes += [("→", "s’appuie sur ce passage", n) for n in plusieurs(options["--soutient"])
                   ] if "--soutient" in options else []
        lignes += [("←", "vient de ce passage", n) for n in plusieurs(options["--inspire"])
                   ] if "--inspire" in options else []
        lignes += [("◇", "rangée dans le thème", n) for n in plusieurs(options["--classe-dans"])
                   ] if "--classe-dans" in options else []
        lignes += [("~", "en rapport avec", n) for n in plusieurs(options["--lien"])
                   ] if "--lien" in options else []
        if "--note" in options:
            role = options.get("--role")
            genre = "créé" if role in ROLES_MASCULINS else "créée"
            lignes.append(("+", f"{mot('role', role) if role else 'note'} {genre}",
                           nom(options["--note"])))
        if "--reserve" in options:
            lignes.append(("!", "doute à suivre créé", nom(options["--reserve"])))
    elif paire == ("note", "reviser"):
        nature = {"mineure": " (retouche)", "substantielle": " (changement de sens)"}.get(
            options.get("--nature"), "")
        lignes.append(("✎", f"texte modifié{nature}", nom(p[2])))
    elif paire == ("echange", "question"):
        lignes.append(("✎", "question ajoutée", nom(p[2])))
    elif paire == ("reserve", "creer"):
        lignes.append(("!", "doute à suivre créé", nom(options.get("--titre"))))
        lignes.append(("·", "à partir du passage", nom(p[2])))
    elif paire == ("reserve", "concerne"):
        lignes.append(("!", f"le doute « {nom(p[2])} » concerne", nom(p[3])))
    elif paire == ("reserve", "traiter"):
        lignes.append(("✓", f"répond au doute « {nom(p[2])} »", nom(p[3])))
    elif paire == ("reserve", "etat"):
        lignes.append(("!", f"doute : {mot('reserve', p[3])}", nom(p[2])))
    elif tete == "lier":
        lignes.append(("→", mot("relation", p[2]) if p[2] else "relation",
                       f"{nom(p[1])} → {nom(p[3])}"))
    elif tete == "verdict":
        lignes.append((SIGNES_DE_VERDICT.get(p[2], "?"), mot("verdict", p[2]), nom(p[1])))
    elif tete == "decision":
        lignes.append((SIGNES_DE_DECISION.get(p[2], "·"), f"décision : {mot('decision', p[2])}",
                       nom(p[1])))
    elif tete == "implementation":
        lignes.append(("⚙", f"avancement : {mot('implementation', p[2])}", nom(p[1])))
    elif tete == "acquitter":
        lignes.append(("✓", "gardée quand même", nom(p[1])))
        lignes.append(("·", "malgré", nom(p[2])))
    elif tete == "retirer":
        lignes.append(("–", "retirée des notes en vigueur", nom(p[1])))
    elif tete == "ancrer":
        lignes.append(("‹›", "liée au code", f"{nom(p[1])} · {nom(options.get('--fichier'))}"))
    elif tete == "annuler":
        lignes.append(("↶", "la dernière action est annulée", ""))
    elif tete == "retablir":
        lignes.append(("↷", "l’action annulée est rétablie", ""))
    elif tete == "restaurer":
        lignes.append(("↶", "revenir à l’étape", nom(options.get("--depuis"))))
        if "--notes" in options:
            lignes += [("·", "seulement", n) for n in plusieurs(options["--notes"])]
    elif tete == "basculer":
        lignes.append(("⇄", "aller sur la piste", nom(p[1])))
    elif tete == "abandonner":
        lignes.append(("–", "piste mise de côté", nom(p[1])))
    elif tete == "reactiver":
        lignes.append(("+", "piste reprise", nom(p[1])))
    elif tete == "fusionner":
        lignes.append(("⇄", "adopter la piste", nom(p[1])))
    elif paire == ("reprise", "noter"):
        lignes.append(("⚑", "point « où j’en suis » noté", nom(options.get("--objectif"))))
    elif tete == "init":
        lignes.append(("+", "projet créé", ""))
    elif tete == "sauvegarder":
        lignes.append(("⎘", "copie de sauvegarde", nom(p[1])))
    elif tete == "exporter":
        lignes.append(("⎘", "export en texte", nom(p[1])))
    elif tete == "importer":
        lignes.append(("+", "projet importé", nom(p[1])))
    elif tete == "migrer":
        lignes.append(("⇧", "base mise à jour", ""))
    else:
        lignes.append(("·", f"commande {tete}", " ".join(nom(x) for x in positions[1:])))
    if options.get("--motif"):
        lignes.append(("“", "pourquoi", str(options["--motif"])))
    return lignes


@dataclass(frozen=True)
class Etape:
    """Une étape de `memoire historique` : identifiant, commande, motif (vide s'il n'y en a pas)."""

    identifiant: str
    commande: str
    motif: str


def lire_historique(sortie):
    """Étapes de « ID — commande — motif », de la plus récente à la plus ancienne."""
    etapes = []
    for ligne in sortie.splitlines():
        morceaux = ligne.split(" — ")
        if len(morceaux) < 2 or len(morceaux[0].strip()) != 36:
            continue
        motif = " — ".join(morceaux[2:]).strip()
        etapes.append(Etape(morceaux[0].strip(), morceaux[1].strip(),
                            "" if motif == "(sans motif)" else motif))
    return etapes


ETAPES = {
    "annuler": "Annulation", "retablir": "Rétablissement", "basculer": "Changement de piste",
    "abandonner": "Piste mise de côté", "reactiver": "Piste reprise",
    "fusionner": "Piste adoptée", "lot": "Plusieurs changements", "reprise": "Où j’en suis",
    "creer-note": "Nouvelle note", "reviser": "Texte modifié", "inclure": "Inclusion",
    "copier-contexte": "Contexte copié", "lier": "Nouveau lien", "verdict": "Vérification",
    "decision": "Décision", "implementation": "Avancement",
    "retirer": "Retrait des notes en vigueur", "reattacher": "Rattachement",
    "remplacer": "Remplacement", "restreindre": "Restriction", "contredire": "Contradiction",
    "creer-echange": "Nouvel échange avec une IA", "completer-question": "Question ajoutée",
    "citer": "Passage cité", "creer-reserve": "Nouveau doute à suivre",
    "traiter": "Réponse à un doute", "etat-reserve": "État d’un doute",
    "acquitter": "Gardée quand même", "detacher": "Détachement", "ancrer": "Lien au code",
    "retirer-ancrage": "Lien au code retiré", "restaurer": "Retour à une étape",
    "restaurer-notes": "Retour de notes à une étape", "racine": "Début du projet",
}


def libelle_d_etape(commande):
    """Nom d'une étape dans le vocabulaire de l'interface."""
    return ETAPES.get(commande, commande.replace("-", " ").capitalize())
