"""Contrat du nommage et du registre des terminaux (TERMINAUX.md, sections 1 à 3 ; D29 ;
INV-TERM-03). Écrit avant le code. Sans Tkinter.

Exécution : python3 -m unittest discover -s gui
"""
import json
from pathlib import Path
import tempfile
import unittest

from terminaux import (EtatDuTerminal, Terminaux, abreger, etat_court, etat_lisible, lire, nom,
                       nom_court, noms)

MAISON = "/home/essai"


class LeNommage(unittest.TestCase):
    def test_abreger(self):
        self.assertEqual(abreger("/home/essai", MAISON), "~")
        self.assertEqual(abreger("/home/essai/projets/rtos", MAISON), "~/projets/rtos")
        self.assertEqual(abreger("/home/essaiX/p", MAISON), "/home/essaiX/p")
        self.assertEqual(abreger("/tmp/p", None), "/tmp/p")
        self.assertEqual(abreger("/tmp/p", ""), "/tmp/p")

    def test_chaque_situation(self):
        inactif = EtatDuTerminal(shell="/bin/bash", dossier="/home/essai/projets/rtos")
        self.assertEqual(nom(inactif, MAISON), "bash — ~/projets/rtos")
        self.assertEqual(nom(EtatDuTerminal(shell="bash"), MAISON), "bash")
        en_cours = EtatDuTerminal(shell="bash", dossier="/tmp", commande="make && ./run-tests",
                                  debut=10.0)
        self.assertEqual(nom(en_cours, MAISON), "make && ./run-tests")
        longue = EtatDuTerminal(shell="bash", commande="x" * 50, debut=0.0)
        self.assertEqual(nom(longue, MAISON), "x" * 39 + "…")
        echec = EtatDuTerminal(shell="bash", dossier="/tmp", dernier_code=2)
        self.assertEqual(nom(echec, MAISON), "bash — /tmp")
        termine = EtatDuTerminal(shell="bash", dossier="/tmp", commande="exit", termine=0)
        self.assertEqual(nom(termine, MAISON), "terminé (code 0)")

    def test_doublons_numerotes_dans_l_ordre(self):
        a = EtatDuTerminal(shell="bash", dossier="/tmp")
        b = EtatDuTerminal(shell="bash", dossier="/tmp")
        c = EtatDuTerminal(shell="bash", dossier="/srv")
        d = EtatDuTerminal(shell="bash", dossier="/tmp")
        self.assertEqual(noms([a, b, c, d], MAISON),
                         ["bash — /tmp", "bash — /tmp (2)", "bash — /srv", "bash — /tmp (3)"])
        self.assertEqual(noms([], MAISON), [])

    def test_etats_lisibles(self):
        self.assertEqual(etat_lisible(EtatDuTerminal(shell="bash", commande="make", debut=10.0),
                                      maintenant=143.0), "en cours depuis 2 min 13 s")
        self.assertEqual(etat_lisible(EtatDuTerminal(shell="bash"), maintenant=0.0), "inactif")
        self.assertEqual(etat_lisible(EtatDuTerminal(shell="bash", dernier_code=0), maintenant=0.0),
                         "inactif")
        self.assertEqual(etat_lisible(EtatDuTerminal(shell="bash", dernier_code=2), maintenant=0.0),
                         "dernière commande : code 2")
        self.assertEqual(etat_lisible(EtatDuTerminal(shell="bash", termine=1), maintenant=0.0),
                         "terminé (code 1)")


class LeRegistre(unittest.TestCase):
    def test_numeros_jamais_reemployes(self):
        t = Terminaux()
        self.assertEqual(t.nouveau("/p"), "terminal-1")
        self.assertEqual(t.nouveau(None), "terminal-2")
        t.fermer("terminal-1")
        self.assertEqual(t.nouveau("/q"), "terminal-3")
        self.assertEqual(t.noms_internes(), ["terminal-2", "terminal-3"])
        self.assertIsNone(t.dossier("terminal-2"))
        t.noter_dossier("terminal-2", "/r")
        self.assertEqual(t.dossier("terminal-2"), "/r")
        with self.assertRaises(ValueError):
            t.fermer("terminal-1")

    def test_conservation(self):
        t = Terminaux()
        t.nouveau("/p")
        t.nouveau("/q")
        t.fermer("terminal-1")
        relu, avertissement = Terminaux.de_json(json.loads(json.dumps(t.vers_json())))
        self.assertIsNone(avertissement)
        self.assertEqual(relu, t)
        self.assertEqual(relu.nouveau(None), "terminal-3")

    def test_fichier(self):
        with tempfile.TemporaryDirectory() as dossier:
            chemin = Path(dossier) / "memoire" / "terminaux.json"
            # Absent : un seul terminal, sans dossier, sans avertissement.
            t, avertissement = lire(chemin)
            self.assertEqual((t.noms_internes(), avertissement), (["terminal-1"], None))
            t.nouveau("/q")
            t.ecrire(chemin)
            self.assertEqual(sorted(p.name for p in chemin.parent.iterdir()), ["terminaux.json"])
            relu, avertissement = lire(chemin)
            self.assertEqual((relu, avertissement), (t, None))
            for contenu in ["{pas du json", json.dumps({"version": 9}),
                            json.dumps({"version": 1, "prochain": 1, "terminaux": [{"nom": 3}]}),
                            json.dumps({"version": 1, "prochain": 1,
                                        "terminaux": [{"nom": "terminal-1", "dossier": None},
                                                      {"nom": "terminal-1", "dossier": None}]})]:
                chemin.write_text(contenu, encoding="utf-8")
                relu, avertissement = lire(chemin)
                self.assertEqual(relu.noms_internes(), ["terminal-1"], contenu)
                self.assertIsNotNone(avertissement, contenu)



class LesNomsDansLaConsole(unittest.TestCase):
    """8.5 (CONSOLE.md, section 4) : état court d'une tuile, nom raccourci d'une pastille."""

    def test_etat_court(self):
        shell = "/bin/bash"
        self.assertEqual(etat_court(EtatDuTerminal(shell, "/p"), 100.0), ("", "repos"))
        self.assertEqual(etat_court(EtatDuTerminal(shell, "/p", commande="make", debut=88.0), 100.0),
                         ("depuis 12 s", "cours"))
        self.assertEqual(etat_court(EtatDuTerminal(shell, "/p", commande="make"), 100.0),
                         ("en cours", "cours"))
        self.assertEqual(etat_court(EtatDuTerminal(shell, "/p", dernier_code=2), 100.0),
                         ("code 2", "echec"))
        self.assertEqual(etat_court(EtatDuTerminal(shell, "/p", dernier_code=0), 100.0),
                         ("", "repos"))
        self.assertEqual(etat_court(EtatDuTerminal(shell, "/p", commande="make", termine=0), 100.0),
                         ("terminé", "termine"))

    def test_un_nom_raccourci_garde_ce_qui_distingue(self):
        # D'un dossier, la fin (et le numéro d'un doublon) ; d'une commande, le début.
        self.assertEqual(nom_court("bash — ~/notes", 20), "bash — ~/notes")
        self.assertEqual(nom_court("bash — ~/Desktop/projets/logiciel/gui (2)", 20),
                         "bash — …ciel/gui (2)")
        self.assertEqual(nom_court("make && ./run-tests --tout", 12), "make && ./r…")
        self.assertEqual(nom_court("zsh — ~/un/dossier", 9), "zsh — ~/…")
        for longueur in range(6, 40):
            self.assertLessEqual(len(nom_court("bash — ~/Desktop/projets/logiciel", longueur)),
                                 longueur)


class LaConsoleDansLeRegistre(unittest.TestCase):
    """8.5 (CONSOLE.md, section 5 ; D32) : le registre conserve aussi l'état de la console (seuil
    de notification, mosaïque), sous une clé `console` ; son contenu est validé par son lecteur."""

    def test_la_console_est_conservee_avec_les_terminaux(self):
        t = Terminaux()
        t.nouveau("/p")
        self.assertIsNone(t.console)
        self.assertNotIn("console", t.vers_json())
        t.console = {"seuil": 20, "mosaique": {"page": ["terminal-1"], "reserve": [],
                                               "actif": "terminal-1", "x": 0.6, "y": 0.4}}
        relu, avertissement = Terminaux.de_json(json.loads(json.dumps(t.vers_json())))
        self.assertIsNone(avertissement)
        self.assertEqual(relu, t)
        self.assertEqual(relu.console["mosaique"]["x"], 0.6)

    def test_une_console_illisible_ne_fait_pas_perdre_les_terminaux(self):
        t = Terminaux()
        t.nouveau("/p")
        t.nouveau("/q")
        contenu = {**t.vers_json(), "console": 3}
        relu, avertissement = Terminaux.de_json(contenu)
        self.assertIsNone(avertissement)
        self.assertEqual(relu.noms_internes(), ["terminal-1", "terminal-2"])
        self.assertIsNone(relu.console)


if __name__ == "__main__":
    unittest.main()
