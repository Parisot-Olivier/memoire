"""Lecture de l'aide de `memoire` et construction des lignes de commande (INTERFACE.md, section 2).

Module sans dépendance graphique : il se teste seul (gui/test_schema.py).
"""
from dataclasses import dataclass
import re


@dataclass(frozen=True)
class Champ:
    """Un champ de formulaire.

    nature : « position », « option » ou « drapeau » ; nom : NOM en majuscules pour une position,
    --option sinon ; choix : valeurs permises, vide pour un texte libre ; requis : champ
    obligatoire ; fichier : bouton « Parcourir » ; exclusif : membre d'une alternative (l'un ou
    l'autre).
    """

    nature: str
    nom: str
    choix: tuple
    requis: bool
    fichier: bool
    exclusif: bool


@dataclass(frozen=True)
class Commande:
    nom: str
    champs: tuple
    resume: str


MAJUSCULES = re.compile(r"^[A-Z][A-Z0-9_-]*$")
MOT = re.compile(r"^[a-z][a-z0-9-]*$")


def _groupes(usage):
    """Découpe l'usage en jetons ; un groupe entre crochets ou parenthèses reste un seul jeton."""
    jetons, courant, profondeur = [], "", 0
    for c in usage:
        if c in "[(":
            profondeur += 1
        elif c in "])":
            profondeur -= 1
        if c == " " and profondeur == 0:
            if courant:
                jetons.append(courant)
            courant = ""
        else:
            courant += c
    if courant:
        jetons.append(courant)
    if profondeur != 0:
        raise ValueError(f"crochets ou parenthèses non appariés : {usage}")
    return jetons


def _valeur(spec):
    """Choix permis d'une valeur : liste si « a|b », vide pour un texte libre « NOM »."""
    if "|" in spec:
        choix = tuple(spec.split("|"))
        if not all(MOT.match(c) for c in choix):
            raise ValueError(f"choix invalide : {spec}")
        return choix
    if MAJUSCULES.match(spec):
        return ()
    raise ValueError(f"valeur invalide : {spec}")


def _option(nom, spec, requis, exclusif):
    if not nom.startswith("--") or not MOT.match(nom[2:]):
        raise ValueError(f"option invalide : {nom}")
    if spec is None:
        return Champ("drapeau", nom, (), False, False, False)
    return Champ("option", nom, _valeur(spec), requis, "fichier" in nom, exclusif)


def _position(nom, requis, resume):
    if not MAJUSCULES.match(nom):
        raise ValueError(f"argument de position invalide : {nom}")
    liste = re.search(rf"\b{re.escape(nom)} : ([a-z0-9-]+(?:, [a-z0-9-]+)+)", resume)
    choix = tuple(liste.group(1).split(", ")) if liste else ()
    return Champ("position", nom, choix, requis, "FICHIER" in nom, False)


def _champs(usage, resume):
    jetons = _groupes(usage)
    champs = []
    i = 0
    while i < len(jetons):
        jeton = jetons[i]
        if jeton.startswith("[") and jeton.endswith("]"):
            interieur = jeton[1:-1].split(" ")
            if interieur[0].startswith("--"):
                if len(interieur) > 2:
                    raise ValueError(f"option facultative invalide : {jeton}")
                spec = interieur[1] if len(interieur) == 2 else None
                champs.append(_option(interieur[0], spec, False, False))
            elif len(interieur) == 1:
                champs.append(_position(interieur[0], False, resume))
            else:
                raise ValueError(f"groupe facultatif invalide : {jeton}")
        elif jeton.startswith("(") and jeton.endswith(")"):
            for alternative in jeton[1:-1].split(" | "):
                morceaux = alternative.split(" ")
                if len(morceaux) != 2:
                    raise ValueError(f"alternative invalide : {jeton}")
                champs.append(_option(morceaux[0], morceaux[1], False, True))
        elif jeton.startswith("--"):
            if i + 1 >= len(jetons):
                raise ValueError(f"valeur manquante pour {jeton}")
            champs.append(_option(jeton, jetons[i + 1], True, False))
            i += 1
        elif "|" in jeton:
            champs.append(Champ("position", "VALEUR", _valeur(jeton), True, False, False))
        else:
            champs.append(_position(jeton, True, resume))
        i += 1
    noms = [c.nom for c in champs]
    if len(set(noms)) != len(noms):
        raise ValueError(f"champs en double : {usage}")
    return tuple(champs)


def lire_aide(texte):
    """Commandes décrites par la sortie de `memoire aide`, dans l'ordre."""
    commandes = []
    for ligne in texte.splitlines():
        if not ligne.startswith("  "):
            continue
        gauche, separateur, resume = ligne[2:].partition(" — ")
        if not separateur:
            raise ValueError(f"ligne d'aide sans résumé : {ligne}")
        mots = gauche.split(" ")
        n = 0
        while n < len(mots) and n < 2 and MOT.match(mots[n]) and "|" not in mots[n]:
            n += 1
        if n == 0:
            raise ValueError(f"nom de commande invalide : {ligne}")
        commandes.append(Commande(" ".join(mots[:n]), _champs(" ".join(mots[n:]), resume), resume))
    return commandes


def arguments(commande, valeurs):
    """Ligne de commande (liste d'arguments, sans « --base ») d'après les valeurs du formulaire.

    Lève ValueError pour un champ obligatoire vide, une valeur hors des choix, ou une alternative
    sans exactement un membre renseigné.
    """
    resultat = commande.nom.split(" ")
    exclusifs = [c for c in commande.champs if c.exclusif]
    renseignes = [c for c in exclusifs if str(valeurs.get(c.nom, "")).strip()]
    if exclusifs and len(renseignes) != 1:
        noms = " ou ".join(c.nom for c in exclusifs)
        raise ValueError(f"préciser exactement l'un de {noms}")
    for champ in commande.champs:
        if champ.nature == "drapeau":
            if valeurs.get(champ.nom) is True:
                resultat.append(champ.nom)
            continue
        valeur = str(valeurs.get(champ.nom, ""))
        if not valeur.strip():
            if champ.requis:
                raise ValueError(f"champ obligatoire vide : {champ.nom}")
            continue
        if champ.choix and valeur not in champ.choix:
            raise ValueError(f"{champ.nom} : valeur hors des choix ({', '.join(champ.choix)})")
        if champ.nature == "option":
            resultat.append(champ.nom)
        resultat.append(valeur)
    return resultat


# Champs qui désignent une note : l'interface leur propose l'autocomplétion par nom (R3).
REFERENCES = {"NOTE", "ORIGINE", "DESTINATION", "DEPENDANT", "CAUSE", "PASSAGE", "RESERVE",
              "ECHANGE", "--precedent"}

# Champs qui désignent plusieurs notes, séparées par des virgules (NOTES dans l'aide ; 5.3).
# 6.2 (CENTRE.md) : options de lien de « citer ».
# 6.4 (RESTAURATION.md) : sélection d'une restauration.
MULTIPLES = {"--classe-dans", "--depend-de", "--developpe-a-partir-de", "--lien", "--soutient",
             "--inspire", "--notes"}


def est_reference(champ):
    """Vrai si le champ désigne une ou plusieurs notes, donc s'il accepte une recherche par nom."""
    return (champ.nom in REFERENCES or champ.nom in MULTIPLES) and not champ.choix


# Champs qui désignent un dossier : DOSSIER d'« indexer » (6.3), --code de la fiche de reprise (6.5).
DOSSIERS = {"DOSSIER", "--code"}


def est_dossier(champ):
    """Vrai si le champ désigne un dossier : il se choisit par un dialogue de dossier."""
    return champ.nom in DOSSIERS and not champ.choix


def est_multiple(champ):
    """Vrai si le champ accepte plusieurs notes séparées par des virgules."""
    return champ.nom in MULTIPLES and not champ.choix


def dernier_element(valeur):
    """Texte de la dernière note d'une liste séparée par des virgules, sans espaces autour."""
    return valeur.rsplit(",", 1)[-1].strip()


def remplacer_dernier(valeur, identifiant):
    """Liste où la dernière note est remplacée par l'identifiant choisi ; les précédentes restent."""
    if "," not in valeur:
        return identifiant
    return f"{valeur.rsplit(',', 1)[0]},{identifiant}"


def lire_resultats(sortie):
    """Résultats de `memoire chercher` : (identifiant, libellé « désignation — rôle — identifiant »)."""
    resultats = []
    for ligne in sortie.splitlines():
        identifiant, role, designation = ligne.split("\t", 2)
        resultats.append((identifiant, f"{designation} — {role} — {identifiant}"))
    return resultats


def analyser(commandes, argv):
    """Relit une ligne de commande comme le ferait l'exécutable, d'après l'aide (INV-UI-01) :
    rend la commande et les valeurs, ou lève ValueError (commande, option, choix, argument
    manquant ou en trop, alternative)."""
    candidates = [c for c in commandes if argv[:len(c.nom.split(" "))] == c.nom.split(" ")]
    if not candidates:
        raise ValueError(f"commande inconnue : {' '.join(argv[:2])}")
    commande = max(candidates, key=lambda c: len(c.nom.split(" ")))
    reste = argv[len(commande.nom.split(" ")):]
    options = {c.nom: c for c in commande.champs if c.nature in ("option", "drapeau")}
    positions = [c for c in commande.champs if c.nature == "position"]
    valeurs, rang, i = {}, 0, 0
    while i < len(reste):
        jeton = reste[i]
        if jeton.startswith("--"):
            champ = options.get(jeton)
            if champ is None:
                raise ValueError(f"option inconnue pour {commande.nom} : {jeton}")
            if champ.nature == "drapeau":
                valeurs[jeton] = True
                i += 1
                continue
            if i + 1 >= len(reste):
                raise ValueError(f"valeur manquante pour {jeton}")
            valeur = reste[i + 1]
            if champ.choix and valeur not in champ.choix:
                raise ValueError(f"{jeton} : choix invalide « {valeur} »")
            valeurs[jeton] = valeur
            i += 2
            continue
        if rang >= len(positions):
            raise ValueError(f"argument en trop pour {commande.nom} : {jeton}")
        champ = positions[rang]
        if champ.choix and jeton not in champ.choix:
            raise ValueError(f"{champ.nom} : choix invalide « {jeton} »")
        valeurs[champ.nom] = jeton
        rang += 1
        i += 1
    manquants = [c.nom for c in commande.champs if c.requis and c.nom not in valeurs]
    if manquants:
        raise ValueError(f"{commande.nom} : manque {', '.join(manquants)}")
    exclusifs = [c.nom for c in commande.champs if c.exclusif]
    if exclusifs and sum(nom in valeurs for nom in exclusifs) != 1:
        raise ValueError(f"{commande.nom} : une et une seule de {', '.join(exclusifs)}")
    return commande, valeurs


def affichage_de_note(titre, identifiant):
    """Une note choisie, telle qu'affichée dans un champ : « nom · id8 » (RETROUVER.md, section 1)."""
    titre = " ".join(titre.replace("\t", " ").split())
    return f"{titre} · {identifiant[:8]}"


_AFFICHEE = re.compile(r"·\s*([0-9a-f]{8})\s*$")
_TAPE = re.compile(r"^[0-9a-f][0-9a-f-]{7,35}$")


def identifiants(texte, connus):
    """Identifiants à passer à la commande, séparés par des virgules : pour une note affichée
    « nom · id8 », l'identifiant complet s'il est connu (`connus` : id8 → identifiant), sinon le
    préfixe ; un identifiant ou un préfixe tapé à la main passe tel quel. Sans aucune note affichée,
    le texte passe tel quel ; sinon, un morceau qui n'est ni l'un ni l'autre appartient au nom
    d'une note (un nom peut contenir une virgule)."""
    elements = [e.strip() for e in texte.split(",")] if texte.strip() else []
    if not any(_AFFICHEE.search(e) for e in elements):
        return ",".join(elements)
    resultat = []
    for element in elements:
        affichee = _AFFICHEE.search(element)
        if affichee:
            resultat.append(connus.get(affichee.group(1), affichee.group(1)))
        elif _TAPE.match(element):
            resultat.append(element)
    return ",".join(resultat)
