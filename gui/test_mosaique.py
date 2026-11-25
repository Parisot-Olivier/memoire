"""Contrat du modèle de la mosaïque de la console (CONSOLE.md, sections 2, 3 et 5 ; D32 ;
INV-TERM-04). Écrit avant le code. Sans Tkinter.

Exécution : python3 -m unittest discover -s gui
"""
import json
import random
import unittest

from mosaique import (CAPACITE, ECART, PART_AGRANDIE, PART_MAX, PART_MIN, Mosaique, capacite,
                      en_pixels)


def mosaique(*noms):
    m = Mosaique()
    for nom in noms:
        m.placer(nom)
    return m


class LesDispositions(unittest.TestCase):
    def test_une_disposition_par_nombre_de_terminaux(self):
        m = Mosaique()
        self.assertEqual((m.rectangles(), m.point(), m.poignees()), ([], None, []))
        m.placer("a")
        self.assertEqual(m.rectangles(), [("a", (0, 0, 1, 1))])
        self.assertEqual((m.point(), m.poignees()), (None, []))
        m.placer("b")
        self.assertEqual(m.rectangles(), [("a", (0, 0, 0.5, 1)), ("b", (0.5, 0, 1, 1))])
        # Deux terminaux : une séparation et sa poignée, pas de point milieu.
        self.assertEqual((m.point(), m.poignees()), (None, [("x", 0.5, 0.5)]))
        m.placer("c")
        self.assertEqual(m.rectangles(), [("a", (0, 0, 0.5, 1)), ("b", (0.5, 0, 1, 0.5)),
                                          ("c", (0.5, 0.5, 1, 1))])
        self.assertEqual(m.point(), (0.5, 0.5))
        self.assertEqual(m.poignees(), [("x", 0.5, 0.25), ("x", 0.5, 0.75), ("y", 0.75, 0.5)])
        m.placer("d")
        self.assertEqual(m.rectangles(), [("a", (0, 0, 0.5, 0.5)), ("b", (0.5, 0, 1, 0.5)),
                                          ("c", (0.5, 0.5, 1, 1)), ("d", (0, 0.5, 0.5, 1))])
        self.assertEqual(m.point(), (0.5, 0.5))
        self.assertEqual(m.poignees(), [("x", 0.5, 0.25), ("x", 0.5, 0.75), ("y", 0.75, 0.5),
                                        ("y", 0.25, 0.5)])

    def test_un_terminal_ajoute_ne_deplace_pas_les_autres(self):
        m = Mosaique()
        coins = {}
        for nom in "abcd":
            m.placer(nom)
            for present, (x0, y0, _x1, _y1) in m.rectangles():
                self.assertEqual(coins.setdefault(present, (x0, y0)), (x0, y0), (nom, present))
        self.assertEqual(coins, {"a": (0, 0), "b": (0.5, 0), "c": (0.5, 0.5), "d": (0, 0.5)})

    def test_les_rectangles_couvrent_la_page_sans_se_chevaucher(self):
        for nombre in range(1, CAPACITE + 1):
            m = mosaique(*"abcd"[:nombre])
            m.deplacer(0.6, 0.3)
            surface = sum((x1 - x0) * (y1 - y0) for _nom, (x0, y0, x1, y1) in m.rectangles())
            self.assertAlmostEqual(surface, 1.0, msg=nombre)


class LesOperations(unittest.TestCase):
    def test_le_dernier_place_est_actif(self):
        m = mosaique("a", "b")
        self.assertEqual((m.page, m.reserve, m.actif), (["a", "b"], [], "b"))
        m.activer("a")
        self.assertEqual(m.actif, "a")
        # Placer un terminal déjà sur la page le rend seulement actif.
        m.placer("b")
        self.assertEqual((m.page, m.actif), (["a", "b"], "b"))
        with self.assertRaises(ValueError):
            m.activer("inconnu")

    def test_page_pleine_l_arrivant_prend_la_place_de_l_actif(self):
        self.assertEqual(CAPACITE, 4)
        m = mosaique("a", "b", "c", "d")
        m.activer("b")
        m.placer("e")
        self.assertEqual((m.page, m.reserve, m.actif), (["a", "e", "c", "d"], ["b"], "e"))

    def test_ranger_et_rouvrir(self):
        m = mosaique("a", "b", "c", "d")
        m.activer("b")
        m.placer("e")
        m.ranger("e")
        # Le voisin d'avant devient actif ; le rangé rejoint la fin de l'arrière-plan.
        self.assertEqual((m.page, m.reserve, m.actif), (["a", "c", "d"], ["b", "e"], "a"))
        m.placer("b")
        self.assertEqual((m.page, m.reserve, m.actif), (["a", "c", "d", "b"], ["e"], "b"))
        with self.assertRaises(ValueError):
            m.ranger("e")
        # Ranger un terminal qui n'est pas l'actif ne change pas l'actif.
        m.ranger("c")
        self.assertEqual((m.page, m.actif), (["a", "d", "b"], "b"))

    def test_retirer(self):
        m = mosaique("a", "b", "c")
        m.ranger("a")
        m.retirer("a")
        self.assertEqual((m.page, m.reserve, m.actif), (["b", "c"], [], "c"))
        m.retirer("c")
        self.assertEqual((m.page, m.actif), (["b"], "b"))
        m.retirer("b")
        self.assertEqual((m.page, m.reserve, m.actif, m.rectangles()), ([], [], None, []))
        with self.assertRaises(ValueError):
            m.retirer("b")

    def test_chaque_terminal_est_a_une_seule_place(self):
        """INV-TERM-04 : sur la page ou en arrière-plan, jamais les deux ni aucun ; quatre au plus
        sur la page ; l'actif est sur la page."""
        hasard = random.Random(8501)
        m, ouverts, numero = Mosaique(), [], 0
        for _ in range(400):
            geste = hasard.choice(["placer", "placer", "ranger", "retirer", "activer", "agrandir"])
            if geste == "placer":
                if ouverts and hasard.random() < 0.5:
                    m.placer(hasard.choice(ouverts))
                else:
                    numero += 1
                    ouverts.append(f"t{numero}")
                    m.placer(ouverts[-1])
            elif geste == "ranger" and m.page:
                m.ranger(hasard.choice(m.page))
            elif geste == "retirer" and ouverts:
                nom = ouverts.pop(hasard.randrange(len(ouverts)))
                m.retirer(nom)
            elif geste == "activer" and m.page:
                m.activer(hasard.choice(m.page))
            elif geste == "agrandir" and m.page:
                m.agrandir(hasard.choice(m.page))
            self.assertEqual(sorted(m.page + m.reserve), sorted(ouverts))
            self.assertEqual(len(set(m.page + m.reserve)), len(ouverts))
            self.assertLessEqual(len(m.page), CAPACITE)
            self.assertEqual(m.actif in m.page, bool(m.page))
            for capacite_ in (1, 2, 4):
                if m.page:
                    self.assertIn(m.actif, m.montres(capacite_))
            self.assertTrue(PART_MIN <= m.x <= PART_MAX and PART_MIN <= m.y <= PART_MAX)


class LePointMilieu(unittest.TestCase):
    def test_deplacer_redimensionne_tous_les_terminaux(self):
        m = mosaique("a", "b", "c", "d")
        m.deplacer(0.6, 0.3)
        self.assertEqual(m.rectangles(), [("a", (0, 0, 0.6, 0.3)), ("b", (0.6, 0, 1, 0.3)),
                                          ("c", (0.6, 0.3, 1, 1)), ("d", (0, 0.3, 0.6, 1))])
        self.assertEqual(m.point(), (0.6, 0.3))
        # Un seul axe : l'autre ne bouge pas.
        m.deplacer(x=0.4)
        self.assertEqual((m.x, m.y), (0.4, 0.3))
        m.deplacer(y=0.7)
        self.assertEqual((m.x, m.y), (0.4, 0.7))

    def test_aucune_part_sous_un_quart(self):
        self.assertEqual((PART_MIN, PART_MAX), (0.25, 0.75))
        m = mosaique("a", "b", "c", "d")
        m.deplacer(0.01, 0.99)
        self.assertEqual((m.x, m.y), (0.25, 0.75))
        m.deplacer(2, -1)
        self.assertEqual((m.x, m.y), (0.75, 0.25))

    def test_agrandir_puis_parts_egales(self):
        self.assertAlmostEqual(PART_AGRANDIE, 2 / 3)
        g = PART_AGRANDIE
        m = mosaique("a", "b", "c", "d")
        attendus = {"a": (g, g), "b": (1 - g, g), "c": (1 - g, 1 - g), "d": (g, 1 - g)}
        for nom, (x, y) in attendus.items():
            m.egaliser()
            m.agrandir(nom)
            self.assertAlmostEqual(m.x, x, msg=nom)
            self.assertAlmostEqual(m.y, y, msg=nom)
            self.assertEqual((m.agrandi, m.actif), (nom, nom))
            x0, y0, x1, y1 = dict(m.rectangles())[nom]
            self.assertAlmostEqual(x1 - x0, g)
            self.assertAlmostEqual(y1 - y0, g)
        # Un second clic sur le même terminal rend des parts égales.
        m.agrandir("d")
        self.assertEqual((m.x, m.y, m.agrandi), (0.5, 0.5, None))
        # Tirer le point après avoir agrandi : le terminal n'est plus « agrandi ».
        m.agrandir("a")
        m.deplacer(0.5, 0.4)
        self.assertIsNone(m.agrandi)
        m.egaliser()
        self.assertEqual((m.x, m.y), (0.5, 0.5))

    def test_agrandir_selon_la_disposition(self):
        g = PART_AGRANDIE
        trois = mosaique("a", "b", "c")
        trois.deplacer(y=0.4)
        trois.agrandir("a")
        # Le grand terminal de gauche n'a pas de séparation horizontale : y ne bouge pas.
        self.assertAlmostEqual(trois.x, g)
        self.assertEqual(trois.y, 0.4)
        trois.agrandir("c")
        self.assertAlmostEqual(trois.x, 1 - g)
        self.assertAlmostEqual(trois.y, 1 - g)
        deux = mosaique("a", "b")
        deux.agrandir("b")
        self.assertAlmostEqual(deux.x, 1 - g)
        self.assertEqual(deux.y, 0.5)
        un = mosaique("a")
        un.agrandir("a")
        self.assertEqual((un.x, un.y, un.agrandi), (0.5, 0.5, None))
        # Un terminal qui n'est pas montré ne s'agrandit pas.
        cote = mosaique("a", "b", "c")
        cote.agrandir("a", capacite=1)
        self.assertEqual((cote.x, cote.y, cote.agrandi), (0.5, 0.5, None))


class LaPlace(unittest.TestCase):
    def test_la_capacite_suit_la_place(self):
        self.assertEqual(capacite(320, 600), 1)
        self.assertEqual(capacite(659, 900), 1)
        self.assertEqual(capacite(660, 339), 2)
        self.assertEqual(capacite(660, 340), 4)
        self.assertEqual(capacite(1054, 584), 4)

    def test_l_actif_est_toujours_montre(self):
        m = mosaique("a", "b", "c")
        m.ranger("a")
        m.placer("d")
        self.assertEqual((m.page, m.reserve, m.actif), (["b", "c", "d"], ["a"], "d"))
        self.assertEqual((m.montres(1), m.en_attente(1)), (["d"], ["b", "c", "a"]))
        self.assertEqual((m.montres(2), m.en_attente(2)), (["c", "d"], ["b", "a"]))
        self.assertEqual((m.montres(4), m.en_attente(4)), (["b", "c", "d"], ["a"]))
        m.activer("b")
        self.assertEqual((m.montres(1), m.montres(2)), (["b"], ["b", "c"]))
        # Sur le côté : un seul terminal, sur toute la place, ni poignée ni point.
        self.assertEqual(m.rectangles(1), [("b", (0, 0, 1, 1))])
        self.assertEqual((m.point(1), m.poignees(1)), (None, []))
        self.assertEqual((m.point(2), m.poignees(2)), (None, [("x", 0.5, 0.5)]))

    def test_geometrie_en_pixels(self):
        self.assertEqual(ECART, 12)
        m = mosaique("a", "b", "c", "d")
        self.assertEqual(en_pixels(m.rectangles(), 1000, 600),
                         [("a", (0, 0, 494, 294)), ("b", (506, 0, 494, 294)),
                          ("c", (506, 306, 494, 294)), ("d", (0, 306, 494, 294))])
        hasard = random.Random(8502)
        for _ in range(200):
            largeur, hauteur = hasard.randrange(660, 2400), hasard.randrange(340, 1400)
            m.deplacer(hasard.random(), hasard.random())
            r = dict(en_pixels(m.rectangles(), largeur, hauteur))
            xs, ys = round(m.x * largeur), round(m.y * hauteur)
            # Bords de la page exacts, écart constant, centré sur le point.
            self.assertEqual(r["a"][:2], (0, 0))
            self.assertEqual((r["b"][0] + r["b"][2], r["c"][1] + r["c"][3]), (largeur, hauteur))
            self.assertEqual(r["b"][0] - (r["a"][0] + r["a"][2]), ECART)
            self.assertEqual(r["d"][1] - (r["a"][1] + r["a"][3]), ECART)
            self.assertEqual((r["a"][2] + ECART // 2, r["a"][3] + ECART // 2), (xs, ys))
            self.assertEqual((r["c"][0], r["c"][1]), (r["b"][0], r["d"][1]))

    def test_prise_d_une_separation_ou_du_point(self):
        quatre = mosaique("a", "b", "c", "d")
        self.assertEqual(quatre.prise(500, 300, 1000, 600), "xy")
        self.assertEqual(quatre.prise(500, 100, 1000, 600), "x")
        self.assertEqual(quatre.prise(200, 300, 1000, 600), "y")
        self.assertIsNone(quatre.prise(200, 100, 1000, 600))
        trois = mosaique("a", "b", "c")
        # À gauche, un seul terminal sur toute la hauteur : pas de séparation horizontale.
        self.assertIsNone(trois.prise(200, 300, 1000, 600))
        self.assertEqual(trois.prise(800, 300, 1000, 600), "y")
        self.assertEqual(trois.prise(500, 300, 1000, 600), "xy")
        deux = mosaique("a", "b")
        self.assertEqual(deux.prise(500, 300, 1000, 600), "x")
        self.assertIsNone(deux.prise(800, 300, 1000, 600))
        self.assertIsNone(mosaique("a").prise(500, 300, 1000, 600))
        self.assertIsNone(quatre.prise(500, 300, 1000, 600, capacite=1))


class LaConservation(unittest.TestCase):
    def test_aller_retour_json(self):
        m = mosaique("a", "b", "c", "d", "e")
        m.deplacer(0.6, 0.3)
        m.activer("c")
        relue, avertissement = Mosaique.de_json(json.loads(json.dumps(m.vers_json())),
                                                ["a", "b", "c", "d", "e"])
        self.assertIsNone(avertissement)
        self.assertEqual(relue, m)
        m.agrandir("c")
        relue, _ = Mosaique.de_json(json.loads(json.dumps(m.vers_json())), list("abcde"))
        self.assertEqual(relue, m)

    def test_par_defaut(self):
        m, avertissement = Mosaique.de_json(None, list("abcdef"))
        self.assertIsNone(avertissement)
        self.assertEqual((m.page, m.reserve, m.actif, m.x, m.y, m.agrandi),
                         (list("abcd"), list("ef"), "a", 0.5, 0.5, None))
        self.assertEqual(Mosaique.par_defaut(list("abcdef")), m)
        vide, _ = Mosaique.de_json(None, [])
        self.assertEqual((vide.page, vide.reserve, vide.actif), ([], [], None))

    def test_contenu_invalide_donne_la_mosaique_par_defaut(self):
        bon = mosaique("a", "b").vers_json()
        invalides = [
            [],
            "mosaïque",
            {**bon, "page": "a"},
            {**bon, "page": ["a", "a"]},
            {**bon, "page": ["a", "b"], "reserve": ["b"]},
            {**bon, "page": ["a", "b", "c", "d", "e"]},
            {**bon, "page": ["a", 3]},
            {**bon, "x": "large"},
            {**bon, "x": 0.9},
            {**bon, "y": True},
            {**bon, "actif": "c"},
            {k: v for k, v in bon.items() if k != "reserve"},
        ]
        for contenu in invalides:
            relue, avertissement = Mosaique.de_json(contenu, list("abcde"))
            self.assertEqual(relue, Mosaique.par_defaut(list("abcde")), contenu)
            self.assertIsNotNone(avertissement, contenu)

    def test_accord_avec_le_registre(self):
        """Un nom inconnu est ignoré ; un terminal oublié va en arrière-plan."""
        m = mosaique("a", "zz", "b")
        m.ranger("b")
        self.assertEqual((m.page, m.reserve, m.actif), (["a", "zz"], ["b"], "zz"))
        relue, avertissement = Mosaique.de_json(m.vers_json(), ["a", "b", "c"])
        self.assertIsNone(avertissement)
        self.assertEqual((relue.page, relue.reserve, relue.actif), (["a"], ["b", "c"], "a"))
        # Plus aucun terminal de la page : le premier de l'arrière-plan y revient.
        relue, _ = Mosaique.de_json(mosaique("zz").vers_json(), ["a", "b"])
        self.assertEqual((relue.page, relue.reserve, relue.actif), (["a"], ["b"], "a"))


if __name__ == "__main__":
    unittest.main()
