"""Contrat de la partie pure de la vue locale (VOISINAGE.md, section 4) : lecture de la forme
`voisinage --tsv`, disposition, libellés, styles. Écrit avant le code.

Exécution : python3 -m unittest discover -s gui (sans Tkinter).
"""
import unittest

from voisinage import (ENTETE_TSV, STYLES, Lien, bord, disposer, entete, ligne_de_liste,
                       lire_voisinage, tronquer)

TSV = (
    "relation\tsens\ttype\torigine\tvoisine\trole\tnom\tsource\tetat\n"
    "r1\tsortant\tdepend-de\tnote\tn1\timplementation\tPrimitive commune\t-\tecartee\n"
    "r2\tentrant\tlien\tnote\tn2\tnote\tHypothèse\t-\tcourante\n"
    "r3\tsortant\tdepend-de\tcitation\tc1\tcitation\tle tick réveille\te1\tcourante\n"
    "r4\tsortant\tclasse-dans\tcategorie\tk1\tcategorie\tScheduler\t-\tcourante\n"
)


class LectureDuVoisinage(unittest.TestCase):
    def test_lecture_de_la_forme_tsv(self):
        liens = lire_voisinage(TSV)
        self.assertEqual(len(liens), 4)
        self.assertEqual(
            liens[2],
            Lien("r3", "sortant", "depend-de", "citation", "c1", "citation", "le tick réveille", "e1",
                 "courante"),
        )
        self.assertIsNone(liens[0].source)
        self.assertEqual(ENTETE_TSV, TSV.splitlines()[0])
        self.assertEqual(lire_voisinage(ENTETE_TSV + "\n"), [])

    def test_une_sortie_inattendue_est_refusee(self):
        with self.assertRaises(ValueError):
            lire_voisinage("autre\nentête\n")
        with self.assertRaises(ValueError):
            lire_voisinage(ENTETE_TSV + "\nr1\tsortant\n")
        with self.assertRaises(ValueError):
            lire_voisinage(ENTETE_TSV + "\nr1\tde-biais\tlien\tnote\tn\tnote\tx\t-\tcourante\n")
        with self.assertRaises(ValueError):
            lire_voisinage(ENTETE_TSV + "\nr1\tsortant\tlien\tautre\tn\tnote\tx\t-\tcourante\n")

    def test_entete_de_la_sortie_lisible(self):
        self.assertEqual(
            entete("Voisinage de Réveil perdu (abc) — raisonnement, à réexaminer\nNote entière :\n"),
            "Réveil perdu (abc) — raisonnement, à réexaminer",
        )
        with self.assertRaises(ValueError):
            entete("autre chose\n")


class Disposition(unittest.TestCase):
    def test_les_voisins_sont_sur_un_cercle_dans_l_ordre(self):
        """Premier voisin en haut, puis dans le sens des aiguilles d'une montre ; positions
        entières et déterministes."""
        self.assertEqual(disposer(0, 400, 300), [])
        self.assertEqual(disposer(1, 400, 300), [(200, 40)])
        self.assertEqual(disposer(4, 400, 300), [(200, 40), (310, 150), (200, 260), (90, 150)])
        self.assertEqual(disposer(4, 400, 300), disposer(4, 400, 300))
        for x, y in disposer(7, 640, 520):
            self.assertTrue(0 <= x <= 640 and 0 <= y <= 520)

    def test_un_trait_s_arrete_au_bord_d_une_etiquette(self):
        """Ajouté avec le code (6.6b) : la flèche ne disparaît pas sous l'étiquette."""
        self.assertEqual(bord(100, 100, 300, 100, 40, 10), (140, 100))
        self.assertEqual(bord(100, 100, 100, 0, 40, 10), (100, 90))
        self.assertEqual(bord(0, 0, 30, 40, 100, 4), (3, 4))
        self.assertEqual(bord(5, 5, 5, 5, 10, 10), (5, 5))
        self.assertEqual(bord(0, 0, 3, 0, 10, 10), (3, 0))

    def test_libelles(self):
        self.assertEqual(tronquer("court"), "court")
        self.assertEqual(tronquer("x" * 28), "x" * 28)
        self.assertEqual(tronquer("y" * 29), "y" * 27 + "…")
        liens = lire_voisinage(TSV)
        self.assertEqual(ligne_de_liste(liens[0]), "→ depend-de Primitive commune (écartée) — note entière")
        self.assertEqual(ligne_de_liste(liens[1]), "← lien Hypothèse — note entière")
        self.assertEqual(ligne_de_liste(liens[2]), "→ depend-de « le tick réveille » — citation")
        self.assertEqual(ligne_de_liste(liens[3]), "→ classe-dans Scheduler — catégorie")

    def test_chaque_origine_a_sa_couleur_et_son_trait(self):
        """Couleur doublée d'un style de trait (VOISINAGE.md, section 1)."""
        self.assertEqual(set(STYLES), {"note", "citation", "categorie"})
        self.assertEqual(STYLES["note"], ("#0072B2", ()))
        self.assertEqual(STYLES["citation"], ("#D55E00", (2, 4)))
        self.assertEqual(STYLES["categorie"], ("#009E73", (8, 4)))


if __name__ == "__main__":
    unittest.main()


class LeTitreDuCentre(unittest.TestCase):
    """8.2c : le titre et le rôle de la note lue viennent de l'en-tête de `voisinage`."""

    def test_titre_et_role(self):
        from voisinage import titre_et_role
        self.assertEqual(
            titre_et_role("Le noyau (b2e84c4c-0734-46c1-a2ea-9acaadd96b03) — raisonnement"),
            ("Le noyau", "raisonnement"))
        self.assertEqual(titre_et_role("Titre (avec) parenthèses (b2e84c4c-0734-46c1-a2ea-"
                                       "9acaadd96b03) — echange"),
                         ("Titre (avec) parenthèses", "echange"))
        self.assertEqual(titre_et_role("illisible"), ("illisible", ""))


class LaDispositionAuxCoins(unittest.TestCase):
    """8.3 : jusqu'à quatre voisins aux quatre coins (haut gauche, haut droite, bas gauche, bas
    droite), comme la maquette ; au-delà, sur une ellipse."""

    def test_aux_coins(self):
        from voisinage import disposer_en_coins
        (hg, hd, bg, bd) = disposer_en_coins(4, 360, 250)
        self.assertLess(hg[0], 180)
        self.assertLess(hg[1], 125)
        self.assertGreater(hd[0], 180)
        self.assertLess(hd[1], 125)
        self.assertLess(bg[0], 180)
        self.assertGreater(bg[1], 125)
        self.assertGreater(bd[0], 180)
        self.assertGreater(bd[1], 125)
        self.assertEqual(disposer_en_coins(2, 360, 250), [hg, hd])

    def test_au_dela_de_quatre(self):
        from voisinage import disposer_en_coins, disposer_en_ellipse
        self.assertEqual(disposer_en_coins(6, 360, 250), disposer_en_ellipse(6, 360, 250))
