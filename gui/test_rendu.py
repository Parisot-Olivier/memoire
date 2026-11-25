"""Contrat du rendu du visionneur (VISIONNEUR.md, section 4 ; D25 ; INV-VIEW-01, INV-SOURCE-02).
Écrit avant le code.

Exécution : cd gui && python3 -m unittest -v test_rendu (sans Tkinter).
"""
from pathlib import Path
import random
import unittest

from rendu import brut, rendre

RACINE = Path(__file__).resolve().parent.parent
DOC = (RACINE / "fixtures" / "rtos-2026-10" / "doc.md").read_text(encoding="utf-8")


def octets(texte, debut, fin):
    """Texte de la plage [debut, fin) en octets UTF-8."""
    return texte.encode("utf-8")[debut:fin].decode("utf-8")


def styles(rendu, style):
    """Plages affichées portant un style."""
    return [(d, f) for d, f, s in rendu.styles if s == style]


class Correspondance(unittest.TestCase):
    """INV-VIEW-01 : chaque caractère affiché provient d'un caractère de la source."""

    def verifier(self, texte):
        rendu = rendre(texte)
        self.assertEqual(len(rendu.spans), len(rendu.affiche))
        substitues = {i for d, f, s in rendu.styles if s in ("puce", "separateur") for i in range(d, f)}
        precedent = -1
        for i, (car, (debut, fin)) in enumerate(zip(rendu.affiche, rendu.spans)):
            source = octets(texte, debut, fin)
            self.assertEqual(len(source), 1, f"caractère {i} : {source!r}")
            if i not in substitues:
                self.assertEqual(car, source, f"caractère {i}")
            # L'ordre de la source est conservé.
            self.assertGreater(debut, precedent)
            precedent = debut
        return rendu

    def test_chaque_caractere_affiche_vient_de_la_source(self):
        for texte in [
            "",
            "texte simple\n",
            "# Titre\n## Sous-titre\ncorps",
            "a **b** c, *i* et _j_, `code` et [lien](https://exemple.org)",
            "- un\n* deux\n1. trois\n> cité\n---\nfin",
            "```rust\nlet *a* = `1`;\n```\naprès",
            "é🚀 **x** \r\n— fin",
            "**pas fermé, `ouvert, [lien sans fin, *seul",
            DOC,
        ]:
            with self.subTest(texte=texte[:30]):
                self.verifier(texte)

    def test_le_rendu_brut_est_l_identite(self):
        for texte in ["", "a **b**\n", "é🚀\r\n", DOC]:
            rendu = brut(texte)
            self.assertEqual(rendu.affiche, texte)
            self.assertEqual(rendu.styles, [])
            if texte:
                self.assertEqual(
                    rendu.plage_source(0, len(texte)), (0, len(texte.encode("utf-8")))
                )


class SousEnsemble(unittest.TestCase):
    """VISIONNEUR.md, section 4 : éléments reconnus ; tout le reste tel quel."""

    def test_titres(self):
        rendu = rendre("# Titre\n### Trois\n####### sept\n")
        self.assertEqual(rendu.affiche, "Titre\nTrois\n####### sept\n")
        self.assertEqual(styles(rendu, "titre1"), [(0, 5)])
        self.assertEqual(styles(rendu, "titre3"), [(6, 11)])
        self.assertEqual(rendu.spans[0], (2, 3))

    def test_mises_en_valeur_et_code_en_ligne(self):
        rendu = rendre("a **b** c *i* _j_ `k`")
        self.assertEqual(rendu.affiche, "a b c i j k")
        self.assertEqual(styles(rendu, "gras"), [(2, 3)])
        self.assertEqual(styles(rendu, "italique"), [(6, 7), (8, 9)])
        self.assertEqual(styles(rendu, "code"), [(10, 11)])

    def test_liens_montres_comme_texte(self):
        rendu = rendre("voir [la doc](https://exemple.org/a_b) ici")
        self.assertEqual(rendu.affiche, "voir la doc ici")
        self.assertEqual(styles(rendu, "lien"), [(5, 11)])

    def test_listes_citations_et_separateurs(self):
        rendu = rendre("- un\n* deux\n1. trois\n> cité\n---\n")
        self.assertEqual(rendu.affiche, "• un\n• deux\n1. trois\ncité\n———\n")
        self.assertEqual(styles(rendu, "puce"), [(0, 1), (5, 6)])
        self.assertEqual(styles(rendu, "citation"), [(21, 25)])
        self.assertEqual(styles(rendu, "separateur"), [(26, 29)])

    def test_blocs_de_code_sans_rendu_interne(self):
        texte = "avant\n```rust\nlet *a* = `1`;\n```\naprès"
        rendu = rendre(texte)
        self.assertEqual(rendu.affiche, "avant\nlet *a* = `1`;\naprès")
        self.assertEqual(styles(rendu, "bloc"), [(6, 21)])
        self.assertEqual(styles(rendu, "italique"), [])
        # Bloc non fermé : jusqu'à la fin, toujours sans rendu interne.
        rendu = rendre("```\n**x**")
        self.assertEqual(rendu.affiche, "**x**")

    def test_marques_mal_formees_restent_telles_quelles(self):
        for texte in ["**pas fermé", "`ouvert", "[lien sans fin", "*seul", "a ** b", "[t](sans fin"]:
            with self.subTest(texte=texte):
                self.assertEqual(rendre(texte).affiche, texte)


class Plages(unittest.TestCase):
    """Règles 2 et 3 : sélection vers la source, citation vers l'affichage."""

    def test_une_selection_devient_la_plus_petite_plage_qui_la_contient(self):
        texte = "a **b** c"
        rendu = rendre(texte)
        self.assertEqual(rendu.plage_source(0, 5), (0, 9))
        self.assertEqual(octets(texte, *rendu.plage_source(0, 5)), "a **b** c")
        self.assertEqual(rendu.plage_source(2, 3), (4, 5))
        self.assertEqual(rendu.plage_source(2, 2), None)

    def test_octets_d_un_texte_accentue(self):
        texte = "é🚀 **où**"
        rendu = rendre(texte)
        self.assertEqual(rendu.affiche, "é🚀 où")
        debut, fin = rendu.plage_source(3, 5)
        self.assertEqual(octets(texte, debut, fin), "où")
        self.assertEqual((debut, fin), (9, 12))

    def test_une_citation_se_surligne_sur_les_caracteres_issus_de_sa_plage(self):
        texte = "a **b** c"
        rendu = rendre(texte)
        self.assertEqual(rendu.plages_affichees(4, 5), [(2, 3)])
        self.assertEqual(rendu.plages_affichees(2, 7), [(2, 3)])
        self.assertEqual(rendu.plages_affichees(0, 9), [(0, 5)])
        # Une plage qui ne couvre que des marques ne se voit pas.
        self.assertEqual(rendu.plages_affichees(2, 4), [])
        # Une citation coupée par un bloc caché se surligne en deux morceaux.
        texte = "x\n```\ny\n```\nz"
        rendu = rendre(texte)
        self.assertEqual(rendu.affiche, "x\ny\nz")
        self.assertEqual(rendu.plages_affichees(0, len(texte)), [(0, 5)])

    def test_aller_retour_sur_une_source_reelle(self):
        rendu = rendre(DOC)
        hasard = random.Random(7)
        for _ in range(300):
            a = hasard.randrange(len(rendu.affiche))
            b = hasard.randrange(a + 1, len(rendu.affiche) + 1)
            debut, fin = rendu.plage_source(a, b)
            couvert = rendu.plages_affichees(debut, fin)
            # La sélection est couverte par le surlignage de la citation qu'elle crée.
            self.assertTrue(any(d <= a and b <= f for d, f in couvert), (a, b, couvert))
            # INV-SOURCE-02 : le texte cité est la source exacte, quel que soit le rendu.
            self.assertEqual(octets(DOC, debut, fin), DOC.encode("utf-8")[debut:fin].decode("utf-8"))


if __name__ == "__main__":
    unittest.main()
