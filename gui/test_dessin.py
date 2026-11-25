"""Contrat du dessin de l'interface (APPARENCE.md, section 5 ; D31 ; INV-LOOK-01).

Sans Tkinter : les images sont des PNG calculés en Python pur ; on les relit ici octet par octet.
"""
import struct
import unittest
import zlib

import dessin
from dessin import rgba


def morceaux(donnees):
    """Morceaux d'un PNG : (type, contenu), après vérification de chaque CRC."""
    if donnees[:8] != b"\x89PNG\r\n\x1a\n":
        raise AssertionError("signature PNG absente")
    rang, trouves = 8, []
    while rang < len(donnees):
        (longueur,) = struct.unpack(">I", donnees[rang:rang + 4])
        genre = donnees[rang + 4:rang + 8]
        contenu = donnees[rang + 8:rang + 8 + longueur]
        (crc,) = struct.unpack(">I", donnees[rang + 8 + longueur:rang + 12 + longueur])
        if zlib.crc32(genre + contenu) & 0xFFFFFFFF != crc:
            raise AssertionError(f"CRC faux pour {genre!r}")
        trouves.append((genre, contenu))
        rang += 12 + longueur
    return trouves


def pixel(image, x, y):
    largeur, _hauteur, lignes = dessin.lire_png(image.png)
    return tuple(lignes[y][4 * x:4 * x + 4])


class LeFormatPng(unittest.TestCase):
    def test_un_png_rgba_bien_forme_aux_dimensions_annoncees(self):
        image = dessin.forme(40, 24, 12, rgba("#F89C39"), ombre=(3, 4, 3, rgba("#241F2E", .2)))
        trouves = morceaux(image.png)
        self.assertEqual([g for g, _ in trouves][0], b"IHDR")
        self.assertEqual([g for g, _ in trouves][-1], b"IEND")
        largeur, hauteur, profondeur, genre = struct.unpack(">IIBB", trouves[0][1][:10])
        self.assertEqual((largeur, hauteur), (image.largeur, image.hauteur))
        self.assertEqual((profondeur, genre), (8, 6))
        brut = zlib.decompress(b"".join(c for g, c in trouves if g == b"IDAT"))
        self.assertEqual(len(brut), hauteur * (1 + 4 * largeur))
        self.assertTrue(all(brut[y * (1 + 4 * largeur)] == 0 for y in range(hauteur)))

    def test_l_image_contient_la_forme_et_sa_marge(self):
        image = dessin.forme(40, 24, 12, rgba("#FFFFFF"), ombre=(5, 7, 4, rgba("#241F2E", .22)))
        self.assertGreaterEqual(image.x, 0)
        self.assertGreaterEqual(image.largeur, image.x + 40 + 5)
        self.assertGreaterEqual(image.hauteur, image.y + 24 + 7)

    def test_png_ecrit_les_lignes_donnees(self):
        lignes = [bytes([255, 0, 0, 255, 0, 0, 255, 128]), bytes([0, 255, 0, 0, 1, 2, 3, 4])]
        largeur, hauteur, relues = dessin.lire_png(dessin.png(2, 2, lignes))
        self.assertEqual((largeur, hauteur), (2, 2))
        self.assertEqual([bytes(l) for l in relues], lignes)

    def test_une_ligne_de_mauvaise_longueur_est_refusee(self):
        with self.assertRaises(ValueError):
            dessin.png(2, 1, [bytes(7)])


class LeDeterminisme(unittest.TestCase):
    def test_memes_parametres_memes_octets(self):
        parametres = dict(largeur=120, hauteur=50, rayon=25, fond=rgba("#F89C39"),
                          bord=rgba("#FFFFFF", .5), ombre=(4, 6, 3, rgba("#241F2E", .25)),
                          grain=(.12, 6328))
        self.assertEqual(dessin.forme(**parametres).png, dessin.forme(**parametres).png)
        self.assertEqual(dessin.degrade(64, 40, dessin.DEGRADE_DE_FOND, 128).png,
                         dessin.degrade(64, 40, dessin.DEGRADE_DE_FOND, 128).png)

    def test_le_grain_depend_de_sa_graine(self):
        a = dessin.forme(60, 30, 15, rgba("#F89C39"), grain=(.12, 1))
        b = dessin.forme(60, 30, 15, rgba("#F89C39"), grain=(.12, 2))
        self.assertNotEqual(a.png, b.png)


class LaForme(unittest.TestCase):
    def test_le_centre_a_la_couleur_du_fond_et_les_coins_sont_transparents(self):
        image = dessin.forme(60, 40, 16, rgba("#FFEDE2"))
        self.assertEqual(pixel(image, image.x + 30, image.y + 20), (0xFF, 0xED, 0xE2, 255))
        self.assertEqual(pixel(image, image.x, image.y)[3], 0)
        self.assertEqual(pixel(image, image.x + 59, image.y + 39)[3], 0)

    def test_le_bord_est_adouci(self):
        image = dessin.forme(60, 40, 16, rgba("#241F2E"))
        # Sur l'arc du coin, à 45° : couverture partielle.
        alphas = [pixel(image, image.x + k, image.y + k)[3] for k in range(0, 8)]
        self.assertTrue(any(0 < a < 255 for a in alphas), alphas)

    def test_un_fond_translucide_garde_son_opacite(self):
        image = dessin.forme(60, 40, 16, rgba("#FFF6F0", .52))
        r, g, b, a = pixel(image, image.x + 30, image.y + 20)
        self.assertEqual((r, g, b), (0xFF, 0xF6, 0xF0))
        self.assertAlmostEqual(a, round(.52 * 255), delta=1)

    def test_le_liseré_est_au_bord(self):
        image = dessin.forme(60, 40, 16, rgba("#FFEDE2"), bord=rgba("#342FCD"), epaisseur=2)
        self.assertEqual(pixel(image, image.x + 30, image.y)[:3], (0x34, 0x2F, 0xCD))
        self.assertEqual(pixel(image, image.x + 30, image.y + 20)[:3], (0xFF, 0xED, 0xE2))

    def test_l_ombre_est_decalee_et_ne_passe_pas_sous_le_verre(self):
        image = dessin.forme(60, 40, 16, rgba("#FFF6F0", .52), ombre=(5, 7, 4, rgba("#241F2E", .22)))
        dessous = pixel(image, image.x + 30, image.y + 40 + 3)
        dessus = pixel(image, image.x + 30, image.y - 3)
        self.assertGreater(dessous[3], 0)
        self.assertEqual(dessus[3], 0)
        # Sous la forme translucide, la couleur reste celle du verre, sans assombrissement.
        self.assertEqual(pixel(image, image.x + 30, image.y + 20)[:3], (0xFF, 0xF6, 0xF0))

    def test_sans_ombre_la_forme_est_symetrique(self):
        image = dessin.forme(50, 30, 12, rgba("#342FCD"), bord=rgba("#FFFFFF", .6))
        _l, _h, lignes = dessin.lire_png(image.png)
        for ligne in lignes:
            pixels = [bytes(ligne[i:i + 4]) for i in range(0, len(ligne), 4)]
            self.assertEqual(pixels, pixels[::-1])

    def test_le_calcul_rapide_donne_les_memes_octets_que_le_calcul_pixel_par_pixel(self):
        cas = [
            dict(largeur=90, hauteur=44, rayon=22, fond=rgba("#FFF6F0", .52),
                 bord=rgba("#FFFFFF", .7), ombre=(5, 7, 4, rgba("#241F2E", .22))),
            dict(largeur=70, hauteur=70, rayon=30, fond=rgba("#FFEDE2"),
                 ombre=(0, 18, 24, rgba("#3C1E6E", .3))),
            dict(largeur=120, hauteur=40, rayon=20, fond=rgba("#342FCD"),
                 degrade=dessin.DEGRADE_SIGNATURE, bord=rgba("#FFFFFF", .45),
                 halo=(12, rgba("#FFFFFF", .3)), ombre=(5, 7, 4, rgba("#241F2E", .22))),
            dict(largeur=33, hauteur=17, rayon=0, fond=rgba("#FFFFFF"), bord=rgba("#000000", .1)),
        ]
        for parametres in cas:
            with self.subTest(parametres=parametres):
                self.assertEqual(dessin.forme(**parametres).png,
                                 dessin.forme(**parametres, rapide=False).png)

    def test_un_degrade_horizontal_va_d_un_bout_a_l_autre(self):
        image = dessin.forme(200, 40, 20, rgba("#000000"), degrade=dessin.DEGRADE_SIGNATURE)
        gauche = pixel(image, image.x + 22, image.y + 20)
        milieu = pixel(image, image.x + 104, image.y + 20)
        self.assertLess(gauche[0], 80)  # indigo
        self.assertGreater(milieu[0], 230)  # corail


class LeDegradeDeFond(unittest.TestCase):
    def test_de_gauche_a_droite_a_90_degres(self):
        arrets = ((0, rgba("#000000")), (1, rgba("#FFFFFF")))
        image = dessin.degrade(101, 3, arrets, 90)
        self.assertEqual((image.largeur, image.hauteur), (101, 3))
        rouges = [pixel(image, x, 1)[0] for x in range(101)]
        self.assertLessEqual(rouges[0], 3)
        self.assertGreaterEqual(rouges[-1], 252)
        self.assertEqual(rouges, sorted(rouges))
        self.assertTrue(all(pixel(image, x, 1)[3] == 255 for x in range(101)))

    def test_le_fond_de_l_interface_va_de_l_indigo_au_violet(self):
        image = dessin.degrade(128, 80, dessin.DEGRADE_DE_FOND, 128)
        haut_gauche, bas_droite = pixel(image, 0, 0), pixel(image, 127, 79)
        self.assertGreater(haut_gauche[2], haut_gauche[0])  # bleu dominant
        self.assertGreater(bas_droite[0], 120)
        self.assertGreater(bas_droite[2], 200)


class LesIcones(unittest.TestCase):
    def test_chaque_icone_se_dessine(self):
        self.assertTrue({"ecrire", "relier", "juger", "lire", "code", "pistes", "reprise",
                         "projet", "expert", "chercher", "terminaux", "panneaux", "reglages",
                         "chevron", "coche", "plus"} <= set(dessin.ICONES))
        for nom in dessin.ICONES:
            with self.subTest(icone=nom):
                image = dessin.icone(nom, 18, rgba("#241F2E"))
                self.assertEqual((image.largeur, image.hauteur), (18, 18))
                _l, _h, lignes = dessin.lire_png(image.png)
                alphas = [ligne[i] for ligne in lignes for i in range(3, len(ligne), 4)]
                self.assertTrue(any(a > 200 for a in alphas))
                self.assertTrue(any(a == 0 for a in alphas))

    def test_une_icone_inconnue_est_refusee(self):
        with self.assertRaises(KeyError):
            dessin.icone("inconnue", 18, rgba("#241F2E"))


if __name__ == "__main__":
    unittest.main()
