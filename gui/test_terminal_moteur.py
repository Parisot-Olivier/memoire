"""Contrat du moteur du terminal (TERMINAL.md, sections 3 et 4 ; D27 ; INV-TERM-01, INV-TERM-02).
Écrit avant le code. Sans Tkinter ; la session réelle demande bash (Linux, macOS).

Exécution : python3 -m unittest discover -s gui ; `MEMOIRE=…/memoire` active l'essai de `memoire`
dans le terminal.
"""
import os
from pathlib import Path
import select
import shutil
import tempfile
import time
import unittest

from terminal_moteur import (MARQUE_DE_FIN, Fin, Historique, Session, SuiviDesCommandes, Tampon,
                             arreter_toutes_les_sessions, environnement, fichier_de_demarrage,
                             geometrie)

FIN = MARQUE_DE_FIN.encode()


class LeTampon(unittest.TestCase):
    def test_lignes_et_ligne_en_cours(self):
        t = Tampon()
        c = t.ajouter(b"un\ndeux\ntro")
        self.assertEqual(c.ajoutees, ["un", "deux"])
        self.assertEqual(c.courante, "tro")
        self.assertEqual(c.retirees, 0)
        c = t.ajouter(b"is\r\n")
        self.assertEqual(c.ajoutees, ["trois"])
        self.assertEqual(c.courante, "")
        self.assertEqual(t.texte(), "un\ndeux\ntrois\n")

    def test_retour_chariot_et_retour_arriere(self):
        t = Tampon()
        t.ajouter(b"10 %\r20 %\r30 %")
        self.assertEqual(t.texte(), "30 %")
        t.ajouter(b"\nabc\x08\x08d")
        self.assertEqual(t.texte(), "30 %\nad")
        # Un retour chariot suivi d'une fin de ligne ne vide pas la ligne.
        t.ajouter(b"\r\n")
        self.assertEqual(t.texte(), "30 %\nad\n")

    def test_sequences_ansi_retirees_meme_coupees(self):
        t = Tampon()
        t.ajouter(b"\x1b[1;31mrouge\x1b[0m \x1b[")
        t.ajouter(b"2Kfin\x1b]0;titre de fen\xc3")
        t.ajouter(b"\xaatre\x07 ok\x1b]2;autre\x1b\\!")
        self.assertEqual(t.texte(), "rouge fin ok!")

    def test_utf8_coupe_et_octet_invalide(self):
        t = Tampon()
        t.ajouter("é".encode()[:1])
        self.assertEqual(t.texte(), "")
        t.ajouter("é".encode()[1:] + b" \xff x")
        self.assertEqual(t.texte(), "é � x")

    def test_marque_de_fin_reperee_et_retiree(self):
        t = Tampon()
        c = t.ajouter(b"sortie\n" + FIN + b"2\x07$ ")
        self.assertEqual(c.fins, [2])
        self.assertEqual(t.texte(), "sortie\n$ ")
        # Coupée en morceaux, avec deux fins dans une lecture.
        t.ajouter(FIN[:3])
        c1 = t.ajouter(FIN[3:] + b"0")
        c2 = t.ajouter(b"\x07x" + FIN + b"130\x07")
        self.assertEqual(c1.fins + c2.fins, [0, 130])
        self.assertNotIn("7771", t.texte())
        self.assertTrue(t.texte().endswith("$ x"))

    def test_limite_de_lignes(self):
        t = Tampon(limite=3)
        c = t.ajouter(b"1\n2\n3\n4\n5\n")
        self.assertEqual(t.texte(), "3\n4\n5\n")
        self.assertEqual(c.retirees, 2)

    def test_tabulation_conservee_autres_controles_retires(self):
        t = Tampon()
        t.ajouter(b"a\tb\x07c\x00d")
        self.assertEqual(t.texte(), "a\tbcd")


class LeDossierDansLaMarque(unittest.TestCase):
    """6.9 (TERMINAUX.md, section 3) : la marque de fin porte aussi le dossier du shell."""

    def test_marque_avec_dossier_meme_coupee(self):
        t = Tampon()
        self.assertIsNone(t.dossier)
        c = t.ajouter(b"x\n" + FIN + b"2;/home/essai/projet")
        self.assertEqual((c.fins, c.dossier), ([], None))
        c = t.ajouter(b" rtos\x07$ ")
        self.assertEqual((c.fins, c.dossier), ([2], "/home/essai/projet rtos"))
        self.assertEqual(t.dossier, "/home/essai/projet rtos")
        self.assertEqual(t.texte(), "x\n$ ")
        # Ancienne forme, sans dossier : le dernier dossier connu reste.
        c = t.ajouter(FIN + b"0\x07")
        self.assertEqual((c.fins, c.dossier), ([0], None))
        self.assertEqual(t.dossier, "/home/essai/projet rtos")
        # Un dossier contenant « ; » est lu en entier.
        c = t.ajouter(FIN + b"1;/a;b\x07")
        self.assertEqual((c.fins, c.dossier), ([1], "/a;b"))

    def test_fichier_de_demarrage_transmet_le_dossier(self):
        self.assertIn('"$PWD"', fichier_de_demarrage())

    def test_commande_en_cours(self):
        s = SuiviDesCommandes()
        self.assertIsNone(s.en_cours())
        s.terminee(0, 1.0)  # invite initiale
        s.envoyee("make", 10.0)
        self.assertEqual(s.en_cours(), ("make", 10.0))
        s.terminee(0, 12.0)
        self.assertIsNone(s.en_cours())


class LHistorique(unittest.TestCase):
    def test_navigation(self):
        h = Historique()
        for c in ["make", "make", "  ", "", "ls"]:
            h.ajouter(c)
        self.assertEqual(h.commandes(), ["make", "ls"])
        self.assertEqual(h.precedente("en cours"), "ls")
        self.assertEqual(h.precedente("ignoré"), "make")
        self.assertEqual(h.precedente("ignoré"), "make")
        self.assertEqual(h.suivante(), "ls")
        self.assertEqual(h.suivante(), "en cours")
        self.assertEqual(h.suivante(), "en cours")
        h.ajouter("git status")
        self.assertEqual(h.precedente(""), "git status")

    def test_historique_vide(self):
        h = Historique()
        self.assertEqual(h.precedente("x"), "x")
        self.assertEqual(h.suivante(), "x")


class LeSuivi(unittest.TestCase):
    def test_fins_et_durees(self):
        s = SuiviDesCommandes()
        # La première invite ne termine aucune commande.
        self.assertIsNone(s.terminee(0, 5.0))
        s.envoyee("make", 10.0)
        self.assertEqual(s.terminee(2, 25.5), Fin("make", 2, 15.5))
        self.assertIsNone(s.terminee(0, 30.0))
        s.envoyee("ls", 40.0)
        self.assertEqual(s.terminee(0, 41.0), Fin("ls", 0, 1.0))


    def test_commande_envoyee_avant_l_invite_initiale(self):
        """Ajouté avec le panneau (6.7c) : une commande envoyée dès le démarrage, avant que la
        marque de l'invite initiale ne soit lue, ne lui est pas attribuée."""
        s = SuiviDesCommandes()
        s.envoyee("sleep 1; false", 1.0)
        self.assertIsNone(s.terminee(0, 1.1))
        self.assertEqual(s.terminee(1, 2.5), Fin("sleep 1; false", 1, 1.5))


class LEnvironnementEtLaGeometrie(unittest.TestCase):
    def test_environnement(self):
        env = environnement("/p/.memoire/depot.db", "/opt/memoire/bin/memoire",
                            {"PATH": "/usr/bin", "HOME": "/home/x", "TERM": "xterm-256color"})
        self.assertEqual(env["PATH"], "/opt/memoire/bin" + os.pathsep + "/usr/bin")
        self.assertEqual(env["MEMOIRE_BASE"], "/p/.memoire/depot.db")
        self.assertEqual((env["TERM"], env["PAGER"], env["GIT_PAGER"]), ("dumb", "cat", "cat"))
        self.assertEqual(env["HOME"], "/home/x")
        # Contrat modifié (6.7a) : sans dossier pour l'exécutable, le PATH reste tel quel ; une
        # entrée vide en tête désignerait le dossier courant, ce qui serait un risque.
        self.assertEqual(environnement("b", "memoire", {"PATH": "/usr/bin"})["PATH"], "/usr/bin")
        self.assertEqual(environnement("b", "memoire", {})["PATH"], "")

    def test_geometrie(self):
        self.assertEqual(geometrie(800, 400, 8, 16), (100, 25))
        self.assertEqual(geometrie(805, 415, 8, 16), (100, 25))
        self.assertEqual(geometrie(100, 20, 8, 16), (40, 6))
        self.assertEqual(geometrie(800, 400, 0, 0), (40, 6))

    def test_fichier_de_demarrage(self):
        texte = fichier_de_demarrage()
        self.assertIn('[ -f ~/.bashrc ] && . ~/.bashrc', texte)
        self.assertIn("7771;fin;", texte)
        self.assertIn("PROMPT_COMMAND", texte)
        # La fonction de fin passe avant celles de l'utilisateur, pour lire son code de retour.
        self.assertLess(texte.index(". ~/.bashrc"), texte.index("PROMPT_COMMAND"))


@unittest.skipUnless(shutil.which("bash"), "bash absent")
class UneSessionReelle(unittest.TestCase):
    """Un vrai bash sous pseudo-terminal, avec un HOME d'essai et son .bashrc."""

    def setUp(self):
        self.dossier = tempfile.TemporaryDirectory()
        maison = Path(self.dossier.name) / "maison"
        maison.mkdir()
        (maison / ".bashrc").write_text("PS1='$ '\nexport ESSAI_BASHRC=charge\n", encoding="utf-8")
        env = environnement(str(Path(self.dossier.name) / "depot.db"),
                            os.environ.get("MEMOIRE", "memoire"),
                            {"PATH": os.environ.get("PATH", ""), "HOME": str(maison)})
        self.session = Session(shutil.which("bash"), self.dossier.name, env, colonnes=80, lignes=24)
        self.tampon = Tampon()
        self.suivi = SuiviDesCommandes()
        self.fins = []
        self.session.demarrer()
        self.attendre(lambda: self.tampon.texte().endswith("$ "))

    def tearDown(self):
        self.session.arreter()
        self.dossier.cleanup()

    def lire(self, delai):
        if select.select([self.session.descripteur], [], [], delai)[0]:
            changements = self.tampon.ajouter(self.session.lire_disponible())
            for code in changements.fins:
                fin = self.suivi.terminee(code, time.monotonic())
                if fin:
                    self.fins.append(fin)

    def attendre(self, condition, delai=10.0):
        limite = time.monotonic() + delai
        while not condition():
            if time.monotonic() > limite:
                self.fail(f"délai dépassé ; sortie : {self.tampon.texte()!r}")
            self.lire(0.05)

    def commande(self, texte):
        nombre = len(self.fins)
        self.suivi.envoyee(texte, time.monotonic())
        self.session.ecrire(texte)
        self.attendre(lambda: len(self.fins) > nombre)
        return self.fins[-1]

    def test_cd_persistant_et_bashrc_charge(self):
        self.assertTrue(self.session.suivi_des_fins)
        self.commande("mkdir -p sous && cd sous")
        self.commande("pwd; echo $ESSAI_BASHRC")
        texte = self.tampon.texte()
        self.assertIn(str(Path(self.dossier.name).resolve() / "sous"), texte)
        self.assertIn("charge", texte)
        self.assertNotIn("7771", texte)

    def test_le_dossier_suit_cd(self):
        """6.9 : le tampon connaît le dossier du shell après chaque commande."""
        self.commande("mkdir -p ailleurs && cd ailleurs")
        self.assertEqual(self.tampon.dossier, str(Path(self.dossier.name).resolve() / "ailleurs"))

    def test_codes_de_retour_et_duree(self):
        self.assertEqual(self.commande("false").code, 1)
        self.assertEqual(self.commande("true").code, 0)
        fin = self.commande("sleep 1")
        self.assertEqual(fin.commande, "sleep 1")
        self.assertGreaterEqual(fin.duree, 0.9)

    def test_redimensionner(self):
        self.session.redimensionner(132, 40)
        self.commande("stty size")
        self.assertIn("40 132", self.tampon.texte())

    def test_interrompre_sans_tuer_le_shell(self):
        self.suivi.envoyee("sleep 30", time.monotonic())
        self.session.ecrire("sleep 30")
        time.sleep(0.3)
        self.session.interrompre()
        self.attendre(lambda: self.fins)
        self.assertEqual(self.fins[-1].code, 130)
        self.assertLess(self.fins[-1].duree, 10)
        self.assertTrue(self.session.en_vie())
        self.assertEqual(self.commande("echo vivant").code, 0)

    def test_echo_coupe_detecte(self):
        self.assertTrue(self.session.echo_actif())
        self.commande("stty -echo")
        self.assertFalse(self.session.echo_actif())
        self.commande("stty echo")
        self.assertTrue(self.session.echo_actif())

    def test_exit_puis_arret_sans_reste(self):
        demarrage = self.session.fichier_de_demarrage
        self.assertTrue(Path(demarrage).is_file())
        self.assertEqual(oct(Path(demarrage).stat().st_mode & 0o777), "0o600")
        pid = self.session.pid
        self.session.ecrire("exit 3")
        self.attendre(lambda: not self.session.en_vie())
        self.assertEqual(self.session.code_de_sortie, 3)
        self.session.arreter()
        self.assertFalse(Path(demarrage).exists())
        self.assertFalse(Path(demarrage).parent.exists())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)

    def test_arret_d_un_shell_occupe(self):
        self.session.ecrire("sleep 60")
        time.sleep(0.2)
        pid = self.session.pid
        self.session.arreter()
        self.assertFalse(self.session.en_vie())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)
        # Arrêter deux fois ne fait rien.
        self.session.arreter()

    def test_sessions_oubliees_arretees_a_la_sortie(self):
        """Ajouté en 6.8 : filet de sécurité à la sortie de Python (atexit), pour une session dont
        `arreter` n'aurait pas été appelé."""
        demarrage = Path(self.session.fichier_de_demarrage)
        pid = self.session.pid
        arreter_toutes_les_sessions()
        self.assertFalse(demarrage.parent.exists())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)

    @unittest.skipUnless(Path(os.environ.get("MEMOIRE", "")).is_file(), "exécutable memoire absent")
    def test_memoire_vise_la_base_donnee(self):
        self.assertEqual(self.commande("memoire init").code, 0)
        self.assertEqual(self.commande("memoire statut").code, 0)
        self.assertIn("Branche active", self.tampon.texte())
        self.assertTrue((Path(self.dossier.name) / "depot.db").is_file())


if __name__ == "__main__":
    unittest.main()


class LaCompletion(unittest.TestCase):
    """APPARENCE.md, section 11 (8.4) : complétion du dernier mot d'après le dossier du shell ou
    le PATH ; sans affichage, sans shell."""

    def setUp(self):
        import tempfile
        self.temporaire = tempfile.TemporaryDirectory()
        racine = Path(self.temporaire.name)
        for dossier in ("Documents", "Docker", "src/noyau", "avec espace", ".cache"):
            (racine / dossier).mkdir(parents=True)
        for fichier in ("Dockerfile", "driver.c", "src/main.c", "src/noyau/tache.c"):
            (racine / fichier).write_text("", encoding="utf-8")
        (racine / "bin").mkdir()
        for commande in ("memoire", "memtest"):
            chemin = racine / "bin" / commande
            chemin.write_text("#!/bin/sh\n", encoding="utf-8")
            chemin.chmod(0o755)
        self.dossier = str(racine)
        self.chemins = [str(racine / "bin")]

    def tearDown(self):
        self.temporaire.cleanup()

    def completer(self, ligne):
        from terminal_moteur import completer
        return completer(ligne, self.dossier, chemins=self.chemins, maison=self.dossier)

    def test_cd_ne_propose_que_des_dossiers(self):
        resultat = self.completer("cd Doc")
        self.assertEqual(resultat.ligne, "cd Doc")
        self.assertEqual(resultat.candidats, ["Docker/", "Documents/"])
        self.assertEqual(self.completer("cd Docu").ligne, "cd Documents/")
        self.assertEqual(self.completer("cd Docu").candidats, ["Documents/"])

    def test_une_autre_commande_propose_fichiers_et_dossiers(self):
        self.assertEqual(self.completer("cat Dock").candidats, ["Docker/", "Dockerfile"])
        self.assertEqual(self.completer("cat dr").ligne, "cat driver.c ")

    def test_le_prefixe_commun_est_complete(self):
        self.assertEqual(self.completer("cd D").ligne, "cd Doc")
        self.assertEqual(self.completer("cat src/noyau/t").ligne, "cat src/noyau/tache.c ")
        self.assertEqual(self.completer("cd src/").candidats, ["noyau/"])

    def test_sans_mot_tout_le_dossier_sauf_les_caches(self):
        self.assertEqual(self.completer("cd ").candidats,
                         ["Docker/", "Documents/", "avec espace/", "bin/", "src/"])
        self.assertEqual(self.completer("cd .").candidats, [".cache/"])

    def test_les_espaces_et_la_maison(self):
        self.assertEqual(self.completer("cd av").ligne, "cd avec\\ espace/")
        self.assertEqual(self.completer("cd ~/Docu").ligne, "cd ~/Documents/")

    def test_le_premier_mot_est_une_commande_du_path(self):
        resultat = self.completer("mem")
        self.assertEqual(resultat.candidats, ["memoire", "memtest"])
        self.assertEqual(self.completer("memo").ligne, "memoire ")
        self.assertEqual(self.completer("./dr").ligne, "./driver.c ")

    def test_rien_a_proposer(self):
        resultat = self.completer("cd zzz")
        self.assertEqual((resultat.ligne, resultat.candidats), ("cd zzz", []))
