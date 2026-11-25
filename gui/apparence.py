"""Apparence de l'interface, sans Tkinter (APPARENCE.md ; D31 ; INV-LOOK-02).

Couleurs du thème et couples texte / surface vérifiés par test (contraste WCAG), polices
préférées, nom du projet, lecture de l'état pour la barre d'état, recherche des actions. Aucune
règle métier : l'état vient des sorties de `memoire statut` et `memoire branches --tsv`.
"""
from dataclasses import dataclass
from pathlib import Path
import unicodedata

import dessin

COULEURS = {
    # Surfaces
    "papier": "#FFEDE2",
    "champ": "#FFF8F4",
    "survol": "#FFFAF7",
    "separation": "#EBDAD2",
    "terminal": "#1D1A33",
    "terminal_champ": "#2A2650",
    # Textes
    "encre": "#241F2E",
    "texte_second": "#4A3F4E",
    "etiquette": "#5E5262",
    "blanc": "#FFFFFF",
    "creme": "#FFEDE2",
    "terminal_texte": "#E9E4F5",
    "terminal_invite": "#FFB38A",
    # Console en mosaïque (8.5, CONSOLE.md) : état d'une tuile, liseré de la tuile active.
    "terminal_second": "#B9B2D6",
    "terminal_echec": "#FF9A5C",
    "terminal_vert": "#3DDCA8",
    "terminal_icone": "#CFC8E8",
    "lavande": "#8F88F2",
    # Signature et action
    "indigo": "#342FCD",
    "corail": "#FF7477",
    "orange": "#F89C39",
    "indigo_doux": "#E6E3F8",
    # Couleurs de sens (Okabe-Ito, comme la vue locale ; ambre pour « à revoir »)
    "bleu": "#0072B2",
    "vermillon": "#D55E00",
    "vert": "#009E73",
    "ambre": "#E07B00",
    "citation": "#FCD9B4",
    # Lecture (8.2c) : encart de la question, pastille du genre, puces d'état.
    "encart": "#FBE2D4",
    "genre": "#EFE9F1",
    "segment": "#F2E1D7",
    "puce_bleue": "#DCEBF4",
    "texte_bleu": "#0B4A75",
    "puce_ambre": "#FDE3C4",
    "texte_ambre": "#6B3A00",
}

# Verres : (couleur, opacité), posés sur le dégradé.
VERRE_GIVRE = ("#FFF6F0", 0.60)
VERRE_TEINTE = ("#1C1248", 0.36)
VERRE_CLAIR = ("#FFFFFF", 0.10)

DEGRADE_DE_FOND = tuple((p, "#%02X%02X%02X" % c[:3]) for p, c in dessin.DEGRADE_DE_FOND)
DEGRADE_SIGNATURE = tuple((p, "#%02X%02X%02X" % c[:3]) for p, c in dessin.DEGRADE_SIGNATURE)

# Couples texte / surface déclarés par le thème : (nom, texte, surface, grand texte).
COUPLES = (
    ("texte sur papier", COULEURS["encre"], COULEURS["papier"], False),
    ("texte second sur papier", COULEURS["texte_second"], COULEURS["papier"], False),
    ("étiquette sur papier", COULEURS["etiquette"], COULEURS["papier"], False),
    ("texte dans un champ", COULEURS["encre"], COULEURS["champ"], False),
    ("texte second dans un champ", COULEURS["texte_second"], COULEURS["champ"], False),
    ("étiquette dans un encadré clair", COULEURS["etiquette"], COULEURS["champ"], False),
    ("texte sur pastille blanche", COULEURS["encre"], COULEURS["blanc"], False),
    ("texte sur l'action principale", COULEURS["encre"], COULEURS["orange"], False),
    ("puce cochée", COULEURS["creme"], COULEURS["encre"], False),
    ("sélection dans une liste", COULEURS["encre"], COULEURS["indigo_doux"], False),
    ("citation surlignée", COULEURS["encre"], COULEURS["citation"], False),
    ("terminal", COULEURS["terminal_texte"], COULEURS["terminal"], False),
    ("invite du terminal", COULEURS["terminal_invite"], COULEURS["terminal"], False),
    ("saisie du terminal", COULEURS["terminal_texte"], COULEURS["terminal_champ"], False),
    ("état d'une tuile", COULEURS["terminal_second"], COULEURS["terminal"], False),
    ("échec dans une tuile", COULEURS["terminal_echec"], COULEURS["terminal"], False),
    ("marque sur le dégradé", COULEURS["blanc"], DEGRADE_DE_FOND[0][1], True),
    ("lien bleu sur papier", COULEURS["bleu"], COULEURS["papier"], False),
    ("indigo sur papier", COULEURS["indigo"], COULEURS["papier"], False),
    ("étiquette dans l'encart de la question", COULEURS["etiquette"], COULEURS["encart"], False),
    ("texte dans l'encart de la question", COULEURS["encre"], COULEURS["encart"], False),
    ("pastille du genre", COULEURS["encre"], COULEURS["genre"], False),
    ("bascule : choix non retenu", COULEURS["texte_second"], COULEURS["segment"], False),
    ("barre de sélection", COULEURS["texte_second"], COULEURS["survol"], False),
    ("puce « à jour »", COULEURS["texte_bleu"], COULEURS["puce_bleue"], False),
    ("puce « source révisée »", COULEURS["texte_ambre"], COULEURS["puce_ambre"], False),
)

POLICES_SANS = ("Geist", "Inter", "Adwaita Sans", "IBM Plex Sans", "Cantarell", "Noto Sans",
                "DejaVu Sans")
POLICES_MONO = ("Geist Mono", "JetBrains Mono", "Adwaita Mono", "IBM Plex Mono", "Fira Code",
                "Noto Sans Mono", "DejaVu Sans Mono")

# Point de couleur de chaque rôle de note (suggestions, voisinage) ; le rôle se lit aussi au mot.
COULEUR_DU_ROLE = {
    "raisonnement": COULEURS["indigo"],
    "implementation": COULEURS["vermillon"],
    "note": "#8A7F8D",
    "echange": "#8A7F8D",
    "categorie": COULEURS["vert"],
    "reserve": "#B0408F",
    "citation": COULEURS["orange"],
}

# Pictogramme de chaque sujet du rail, dans l'ordre du catalogue (ACTIONS.md, section 4).
ICONE_DU_SUJET = {
    "Écrire": "ecrire",
    "Relier": "relier",
    "Juger": "juger",
    "Lire": "lire",
    "Code": "code",
    "Histoire et pistes": "pistes",
    "Où j’en suis": "reprise",
    "Projet": "projet",
}


def _rgb(couleur):
    if isinstance(couleur, str):
        c = couleur.lstrip("#")
        return (int(c[0:2], 16), int(c[2:4], 16), int(c[4:6], 16))
    return tuple(couleur[:3])


def _luminance(couleur):
    def canal(v):
        v /= 255
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (canal(v) for v in _rgb(couleur))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contraste(premiere, seconde):
    """Rapport de contraste WCAG entre deux couleurs (« #RRGGBB » ou (r, g, b))."""
    a, b = sorted((_luminance(premiere), _luminance(seconde)), reverse=True)
    return (a + 0.05) / (b + 0.05)


def melange(dessus, dessous):
    """(r, g, b) d'une couleur translucide (couleur, opacité) posée sur une couleur opaque."""
    couleur, alpha = dessus
    haut, bas = _rgb(couleur), _rgb(dessous)
    return tuple(round(h * alpha + b * (1 - alpha)) for h, b in zip(haut, bas))


def couleur_du_degrade(arrets, t):
    """(r, g, b) à la position t d'un dégradé de couleurs « #RRGGBB »."""
    c = dessin.couleur_du_degrade(tuple((p, dessin.rgba(h)) for p, h in arrets), t)
    return tuple(round(v) for v in c[:3])


def hexa(couleur):
    """« #RRGGBB » d'un (r, g, b)."""
    return "#%02X%02X%02X" % tuple(_rgb(couleur))


def choisir_police(presentes, preferees):
    """Première police préférée présente (sans tenir compte des majuscules), sinon None."""
    par_nom = {nom.lower(): nom for nom in presentes}
    for nom in preferees:
        if nom.lower() in par_nom:
            return par_nom[nom.lower()]
    return None


def nom_du_projet(base):
    """Nom affiché du projet : le dossier qui contient `.memoire`, ou celui d'un `depot.db`,
    sinon le nom du fichier de base."""
    if not base:
        return "sans projet"
    chemin = Path(base)
    if chemin.parent.name == ".memoire":
        return chemin.parent.parent.name or chemin.stem
    if chemin.stem == "depot" and chemin.parent.name:
        return chemin.parent.name
    return chemin.stem


@dataclass(frozen=True)
class Etat:
    """Ce que montre la barre d'état : nom de la piste active, notes à revoir, notes sans appui."""

    piste: str
    a_revoir: int
    sans_appui: int


def _section(statut, titre):
    lignes, dedans = [], False
    for ligne in statut.splitlines():
        if not ligne.startswith(" "):
            dedans = ligne.strip() == titre
            continue
        if dedans and ligne.strip().startswith("- "):
            lignes.append(ligne)
    return lignes


def lire_etat(statut, branches):
    """État du projet d'après `memoire statut` et `memoire branches --tsv`."""
    piste = ""
    for ligne in branches.splitlines()[1:]:
        champs = (ligne.split("\t") + [""] * 5)[:5]
        if champs[1] == "oui":
            piste = champs[3]
    return Etat(piste, len(_section(statut, "À réexaminer :")), len(_section(statut, "Orphelines :")))


def libelle_a_revoir(nombre):
    if nombre == 0:
        return "rien à revoir"
    return f"{nombre} note{'s' if nombre > 1 else ''} à revoir"


def libelle_sans_appui(nombre):
    return f"{nombre} note{'s' if nombre > 1 else ''} sans appui"


def sans_accents(texte):
    """Texte en minuscules, sans accents ni cédilles."""
    decompose = unicodedata.normalize("NFD", texte.lower())
    return "".join(c for c in decompose if unicodedata.category(c) != "Mn")


def actions_trouvees(requete, actions):
    """Actions dont le titre contient la requête (majuscules et accents ignorés), dans l'ordre."""
    cle = sans_accents(requete.strip())
    if not cle:
        return []
    return [a for a in actions if cle in sans_accents(a.titre)]
