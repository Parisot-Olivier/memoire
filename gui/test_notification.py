"""Contrat des notifications de fin de commande (TERMINAL.md, section 4 ; D27). Écrit avant le code.
Sans Tkinter et sans toucher au vrai bureau : l'envoi passe par un lanceur remplacé.

Exécution : python3 -m unittest discover -s gui
"""
import unittest

from notification import a_notifier, duree_lisible, envoyer, texte_de_fin, variant_chaine
from terminal_moteur import Fin


class Resultat:
    def __init__(self, code):
        self.returncode = code


class LanceurDEssai:
    """Enregistre chaque appel ; rend un code par programme, ou lève l'exception donnée."""

    def __init__(self, reponses):
        self.reponses = reponses
        self.appels = []

    def __call__(self, argv, **_options):
        self.appels.append(argv)
        reponse = self.reponses[argv[0]]
        if isinstance(reponse, Exception):
            raise reponse
        return Resultat(reponse)


class LeTexte(unittest.TestCase):
    def test_durees(self):
        self.assertEqual(duree_lisible(0.4), "0 s")
        self.assertEqual(duree_lisible(42.6), "43 s")
        self.assertEqual(duree_lisible(60), "1 min")
        self.assertEqual(duree_lisible(133), "2 min 13 s")
        self.assertEqual(duree_lisible(3600), "1 h")
        self.assertEqual(duree_lisible(3900), "1 h 05 min")

    def test_reussite_et_echec(self):
        self.assertEqual(
            texte_de_fin(Fin("make && ./run-tests", 2, 133)),
            ("memoire — commande terminée", "make && ./run-tests — échec (code 2) en 2 min 13 s"),
        )
        self.assertEqual(texte_de_fin(Fin("make", 0, 12))[1], "make — réussite en 12 s")
        longue = "x" * 100
        self.assertEqual(texte_de_fin(Fin(longue, 0, 12))[1], "x" * 79 + "… — réussite en 12 s")

    def test_quand_notifier(self):
        self.assertTrue(a_notifier(Fin("make", 0, 10.0), seuil=10, regarde=False))
        self.assertFalse(a_notifier(Fin("make", 0, 9.9), seuil=10, regarde=False))
        self.assertFalse(a_notifier(Fin("make", 0, 60), seuil=10, regarde=True))


class LEnvoi(unittest.TestCase):
    def test_variant_chaine(self):
        self.assertEqual(variant_chaine("a"), "'a'")
        self.assertEqual(variant_chaine("l'échec \\ fin"), "'l\\'échec \\\\ fin'")

    def test_par_dbus_d_abord(self):
        lanceur = LanceurDEssai({"gdbus": 0, "notify-send": 0})
        self.assertTrue(envoyer("titre", "corps", lanceur=lanceur))
        (argv,) = lanceur.appels
        self.assertEqual(argv[:2], ["gdbus", "call"])
        self.assertIn("org.freedesktop.Notifications.Notify", argv)
        self.assertIn("'titre'", argv)
        self.assertIn("'corps'", argv)
        self.assertIn("'memoire'", argv)

    def test_puis_notify_send(self):
        lanceur = LanceurDEssai({"gdbus": 1, "notify-send": 0})
        self.assertTrue(envoyer("titre", "corps", lanceur=lanceur))
        self.assertEqual(lanceur.appels[1], ["notify-send", "--app-name=memoire", "titre", "corps"])
        lanceur = LanceurDEssai({"gdbus": FileNotFoundError(), "notify-send": 0})
        self.assertTrue(envoyer("titre", "corps", lanceur=lanceur))

    def test_sans_service_ni_erreur(self):
        lanceur = LanceurDEssai({"gdbus": FileNotFoundError(), "notify-send": OSError("refus")})
        self.assertFalse(envoyer("titre", "corps", lanceur=lanceur))
        lanceur = LanceurDEssai({"gdbus": 1, "notify-send": 1})
        self.assertFalse(envoyer("titre", "corps", lanceur=lanceur))


if __name__ == "__main__":
    unittest.main()
