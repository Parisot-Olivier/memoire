"""Contrat de la lecture de l'aide par l'interface (INTERFACE.md, section 2). Écrit avant le code.

Exécution : python3 -m unittest discover -s gui (sans Tkinter).
"""
from pathlib import Path
import unittest

from schema import Champ, arguments, lire_aide

AIDE = (Path(__file__).resolve().parent.parent / "crates/memoire-cli/tests/aide.txt").read_text(encoding="utf-8")


def commande(nom):
    return next(c for c in lire_aide(AIDE) if c.nom == nom)


class LectureDeLAide(unittest.TestCase):
    def test_chaque_commande_se_transcrit(self):
        commandes = lire_aide(AIDE)
        noms = [c.nom for c in commandes]
        attendus = [l.split(" — ")[0].strip() for l in AIDE.splitlines() if l.startswith("  ")]
        attendus = [a.split(" [")[0].split(" --")[0].split(" (")[0] for a in attendus]
        # Contrat modifié (R9, R3) : 29 commandes depuis l'ajout de « sauvegarder » et « chercher » ;
        # contrat modifié (5.2c, D21) : 30 avec « verifier » ; (5.2d) : 31 avec « migrer » ;
        # (5.4a, D23) : 33 avec « abandonner » et « reactiver » ; (5.4c) : 35 avec « fusion plan »
        # et « fusionner » ; (5.5b, D24) : 36, « passage » devenant « citer » et « retirer » s'ajoutant ;
        # (6.1a, D25) : 38 avec « source » et « citations » ; (6.3b) : 39 avec « indexer » ;
        # (6.4b) : 42 avec « historique », « restauration plan » et « restaurer » ; (6.5b) : 46
        # avec « reprise noter », « reprise voir », « reprise liste » et « reprise code » ; (6.6a) : 47
        # avec « voisinage » ; (7.2a) : 48 avec « code notes ».
        self.assertEqual(len(commandes), 48)
        for nom in noms:
            self.assertTrue(any(a.startswith(nom) for a in attendus), nom)
        self.assertEqual(noms[0], "init")
        self.assertIn("note creer", noms)
        self.assertIn("reserve etat", noms)

    def test_formes_de_champs(self):
        self.assertEqual(commande("init").champs, ())
        # Contrat modifié (5.3, LOT.md) : « note creer » reçoit les options de relation, le motif
        # et la confirmation ; ses trois premiers champs sont inchangés.
        role, titre, texte, *liens, motif, oui = commande("note creer").champs
        self.assertEqual(role, Champ("option", "--role", ("categorie", "raisonnement", "implementation", "note"), True, False, False))
        self.assertEqual(titre, Champ("option", "--titre", (), True, False, False))
        self.assertEqual(texte, Champ("option", "--texte", (), False, False, False))
        self.assertEqual(
            liens,
            [Champ("option", nom, (), False, False, False)
             for nom in ("--classe-dans", "--depend-de", "--developpe-a-partir-de", "--lien")],
        )
        self.assertEqual(motif, Champ("option", "--motif", (), False, False, False))
        self.assertEqual(oui, Champ("drapeau", "--oui", (), False, False, False))
        note, valeur, motif, oui = commande("verdict").champs
        self.assertEqual(note, Champ("position", "NOTE", (), True, False, False))
        self.assertEqual(valeur, Champ("position", "VALEUR", ("non-verifiee", "verifiee", "refutee"), True, False, False))
        self.assertEqual(motif, Champ("option", "--motif", (), True, False, False))
        self.assertEqual(oui, Champ("drapeau", "--oui", (), False, False, False))

    def test_choix_donnes_par_le_resume(self):
        _, genre, _, _, _ = commande("lier").champs
        self.assertEqual(genre, Champ("position", "TYPE", ("classe-dans", "depend-de", "developpe-a-partir-de", "lien"), True, False, False))

    def test_fichiers_et_alternatives(self):
        titre, reponse, question, precedent = commande("echange coller").champs
        self.assertEqual(reponse, Champ("option", "--reponse-fichier", (), True, True, False))
        self.assertEqual(question, Champ("option", "--question-fichier", (), False, True, False))
        self.assertEqual(precedent, Champ("option", "--precedent", (), False, False, False))
        note, fichier, fonction, lignes, git = commande("ancrer").champs
        self.assertEqual(fichier, Champ("option", "--fichier", (), True, True, False))
        self.assertEqual(fonction, Champ("option", "--fonction", (), False, False, True))
        self.assertEqual(lignes, Champ("option", "--lignes", (), False, False, True))
        self.assertEqual(git, Champ("option", "--git", (), False, False, False))
        self.assertEqual(commande("exporter").champs, (Champ("position", "FICHIER", (), True, True, False),))
        self.assertEqual(commande("aide").champs, (Champ("position", "COMMANDE", (), False, False, False),))

    def test_usage_hors_grammaire_refuse(self):
        with self.assertRaises(ValueError):
            lire_aide("Commandes :\n  bizarre {x} — inconnu\n")


class ConstructionDesArguments(unittest.TestCase):
    def test_ordre_et_valeurs(self):
        verdict = commande("verdict")
        self.assertEqual(
            arguments(verdict, {"NOTE": "abcd1234", "VALEUR": "refutee", "--motif": "contre-exemple établi", "--oui": False}),
            ["verdict", "abcd1234", "refutee", "--motif", "contre-exemple établi"],
        )
        self.assertEqual(
            arguments(verdict, {"NOTE": "abcd1234", "VALEUR": "refutee", "--motif": "m", "--oui": True}),
            ["verdict", "abcd1234", "refutee", "--motif", "m", "--oui"],
        )
        self.assertEqual(
            arguments(commande("note creer"), {"--role": "note", "--titre": "t", "--texte": ""}),
            ["note", "creer", "--role", "note", "--titre", "t"],
        )

    def test_champs_obligatoires_et_alternatives(self):
        verdict = commande("verdict")
        with self.assertRaisesRegex(ValueError, "--motif"):
            arguments(verdict, {"NOTE": "abcd1234", "VALEUR": "refutee", "--motif": "  "})
        with self.assertRaisesRegex(ValueError, "VALEUR"):
            arguments(verdict, {"NOTE": "abcd1234", "VALEUR": "autre", "--motif": "m"})
        ancrer = commande("ancrer")
        base = {"NOTE": "abcd1234", "--fichier": "driver.c"}
        with self.assertRaises(ValueError):
            arguments(ancrer, base)
        with self.assertRaises(ValueError):
            arguments(ancrer, {**base, "--fonction": "void f()", "--lignes": "1-2"})
        self.assertEqual(
            arguments(ancrer, {**base, "--lignes": "1-2"}),
            ["ancrer", "abcd1234", "--fichier", "driver.c", "--lignes", "1-2"],
        )


if __name__ == "__main__":
    unittest.main()


class References(unittest.TestCase):
    """R3 : champs qui désignent une note, proposés avec autocomplétion."""

    def test_champs_de_reference(self):
        from schema import est_reference
        self.assertTrue(est_reference(commande("verdict").champs[0]))
        origine, genre, destination, _, _ = commande("lier").champs
        self.assertTrue(est_reference(origine))
        self.assertFalse(est_reference(genre))
        self.assertTrue(est_reference(destination))
        self.assertTrue(est_reference(commande("echange coller").champs[3]))  # --precedent
        self.assertFalse(est_reference(commande("note creer").champs[1]))  # --titre
        self.assertFalse(est_reference(commande("exporter").champs[0]))

    def test_champ_dossier(self):
        """6.3 (INDEXEUR.md, section 5) : un champ DOSSIER se choisit par un dialogue de dossier."""
        from schema import est_dossier
        self.assertTrue(est_dossier(commande("indexer").champs[0]))
        self.assertFalse(est_dossier(commande("ancrer").champs[0]))
        self.assertFalse(est_dossier(commande("ancrer").champs[1]))

    def test_champs_de_la_fiche_de_reprise(self):
        """6.5 (FICHE-REPRISE.md, section 6) : --code se choisit par un dialogue de dossier,
        --notes est un champ à plusieurs notes, --sans-code une case à cocher."""
        from schema import est_dossier, est_multiple
        champs = {c.nom: c for c in commande("reprise noter").champs}
        self.assertTrue(champs["--objectif"].requis)
        self.assertTrue(champs["--action"].requis)
        self.assertTrue(est_dossier(champs["--code"]))
        self.assertTrue(est_multiple(champs["--notes"]))
        self.assertEqual(champs["--sans-code"].nature, "drapeau")
        self.assertTrue(est_dossier(next(c for c in commande("reprise voir").champs if c.nom == "--code")))
        self.assertFalse(est_dossier(champs["--objectif"]))

    def test_notes_affichees_par_leur_nom(self):
        """7.2 (RETROUVER.md, section 1) : une note choisie s'affiche « nom · id8 » ; la commande
        reçoit l'identifiant complet quand il est connu, le préfixe sinon ; un identifiant tapé
        à la main passe tel quel."""
        from schema import affichage_de_note, identifiants
        complet = "9b1e9e5d-3ba6-4eeb-b66d-d121ac0676fd"
        self.assertEqual(affichage_de_note("Réveil perdu", complet), "Réveil perdu · 9b1e9e5d")
        self.assertEqual(affichage_de_note("a\tb", complet), "a b · 9b1e9e5d")
        connus = {"9b1e9e5d": complet}
        self.assertEqual(identifiants("Réveil perdu · 9b1e9e5d", connus), complet)
        self.assertEqual(identifiants("Réveil perdu · 9b1e9e5d, File · 3f2a91c0", connus),
                         f"{complet},3f2a91c0")
        self.assertEqual(identifiants("Titre, avec virgule · 12345678, File · 3f2a91c0", {}),
                         "12345678,3f2a91c0")
        self.assertEqual(identifiants("9b1e9e5d", connus), "9b1e9e5d")
        self.assertEqual(identifiants(" 9b1e9e5d,3f2a91c0 ", connus), "9b1e9e5d,3f2a91c0")
        self.assertEqual(identifiants("", connus), "")

    def test_lecture_des_resultats(self):
        from schema import lire_resultats
        sortie = "abc\traisonnement\tréveil perdu\ndef\tpassage\t« phrase »\n"
        self.assertEqual(
            lire_resultats(sortie),
            [("abc", "réveil perdu — raisonnement — abc"), ("def", "« phrase » — passage — def")],
        )
        self.assertEqual(lire_resultats(""), [])

    def test_champs_a_plusieurs_notes(self):
        """5.3 (LOT.md, section 5) : un champ NOTES accepte plusieurs notes séparées par des
        virgules ; l'autocomplétion porte sur la dernière, et le choix ne remplace qu'elle."""
        from schema import dernier_element, est_multiple, est_reference, remplacer_dernier
        champs = {c.nom: c for c in commande("note creer").champs}
        for nom in ("--classe-dans", "--depend-de", "--developpe-a-partir-de", "--lien"):
            self.assertTrue(est_reference(champs[nom]), nom)
            self.assertTrue(est_multiple(champs[nom]), nom)
        self.assertFalse(est_multiple(champs["--titre"]))
        self.assertFalse(est_multiple(commande("lier").champs[0]))
        # 6.2 (CENTRE.md, section 4) : les options de lien de « citer » aussi ; pas ses titres.
        champs_citer = {c.nom: c for c in commande("citer").champs}
        for nom in ("--soutient", "--inspire", "--classe-dans", "--lien"):
            self.assertTrue(est_multiple(champs_citer[nom]), nom)
        for nom in ("--note", "--reserve", "--texte-note", "--cible"):
            self.assertFalse(est_reference(champs_citer[nom]), nom)
        # 6.4 (RESTAURATION.md, section 4) : la sélection d'une restauration aussi.
        champs_restaurer = {c.nom: c for c in commande("restaurer").champs}
        self.assertTrue(est_multiple(champs_restaurer["--notes"]))
        self.assertFalse(est_reference(champs_restaurer["--depuis"]))
        self.assertEqual(dernier_element(""), "")
        self.assertEqual(dernier_element("reveil"), "reveil")
        self.assertEqual(dernier_element("3f2a91c0, reve"), "reve")
        self.assertEqual(dernier_element("3f2a91c0,"), "")
        self.assertEqual(remplacer_dernier("reve", "ab12cd34"), "ab12cd34")
        self.assertEqual(remplacer_dernier("3f2a91c0, reve", "ab12cd34"), "3f2a91c0,ab12cd34")
        self.assertEqual(remplacer_dernier("3f2a91c0,", "ab12cd34"), "3f2a91c0,ab12cd34")
