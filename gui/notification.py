"""Notification de fin de commande (TERMINAL.md, section 4 ; D27).

Texte et règle de notification : fonctions pures. Envoi : service de notifications du bureau par
D-Bus (`gdbus`), sinon `notify-send`, sinon rien ; jamais d'erreur pour l'utilisateur.
"""
import subprocess

TITRE = "memoire — commande terminée"
LONGUEUR_DE_COMMANDE = 80
SEUIL_PAR_DEFAUT = 10.0
DUREE_D_AFFICHAGE_MS = 10000


def duree_lisible(secondes):
    """« 43 s », « 2 min 13 s », « 1 h 05 min »."""
    total = int(round(secondes))
    heures, reste = divmod(total, 3600)
    minutes, secondes = divmod(reste, 60)
    if heures:
        return f"{heures} h" + (f" {minutes:02d} min" if minutes else "")
    if minutes:
        return f"{minutes} min" + (f" {secondes} s" if secondes else "")
    return f"{secondes} s"


def texte_de_fin(fin):
    """Titre et corps de la notification d'une commande terminée."""
    commande = fin.commande
    if len(commande) > LONGUEUR_DE_COMMANDE:
        commande = commande[:LONGUEUR_DE_COMMANDE - 1] + "…"
    issue = "réussite" if fin.code == 0 else f"échec (code {fin.code})"
    return TITRE, f"{commande} — {issue} en {duree_lisible(fin.duree)}"


def a_notifier(fin, seuil=SEUIL_PAR_DEFAUT, regarde=False):
    """Une commande d'au moins `seuil` secondes, terminée quand le terminal n'est pas regardé."""
    return fin.duree >= seuil and not regarde


def variant_chaine(texte):
    """Chaîne au format texte des variants GLib, pour `gdbus call`."""
    return "'" + texte.replace("\\", "\\\\").replace("'", "\\'") + "'"


def envoyer(titre, corps, lanceur=subprocess.run):
    """Envoie la notification ; vrai si un moyen l'a acceptée. Aucune exception ne sort."""
    essais = [
        ["gdbus", "call", "--session", "--dest", "org.freedesktop.Notifications",
         "--object-path", "/org/freedesktop/Notifications",
         "--method", "org.freedesktop.Notifications.Notify",
         variant_chaine("memoire"), "0", variant_chaine(""), variant_chaine(titre),
         variant_chaine(corps), "[]", "{}", str(DUREE_D_AFFICHAGE_MS)],
        ["notify-send", "--app-name=memoire", titre, corps],
    ]
    for argv in essais:
        try:
            resultat = lanceur(argv, capture_output=True, timeout=5)
        except (OSError, subprocess.SubprocessError):
            continue
        if resultat.returncode == 0:
            return True
    return False
