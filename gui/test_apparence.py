"""Contrat de l'apparence sans affichage (APPARENCE.md ; D31 ; INV-LOOK-02, INV-UI-01).

Couleurs et contrastes du thème, choix des polices, nom du projet, lecture de l'état pour la
barre d'état, recherche des actions. Aucune importation de tkinter.
"""
import unittest

import apparence
from actions import ACTIONS, GROUPES


class LesContrastes(unittest.TestCase):
    def test_le_calcul_du_contraste(self):
        self.assertAlmostEqual(apparence.contraste("#FFFFFF", "#000000"), 21, places=3)
        self.assertAlmostEqual(apparence.contraste("#123456", "#123456"), 1, places=6)
        self.assertAlmostEqual(apparence.contraste("#FFEDE2", "#241F2E"),
                               apparence.contraste("#241F2E", "#FFEDE2"), places=9)

    def test_le_melange_d_une_couleur_translucide(self):
        self.assertEqual(apparence.melange(("#FFFFFF", 0), "#102030"), (0x10, 0x20, 0x30))
        self.assertEqual(apparence.melange(("#FFFFFF", 1), "#102030"), (255, 255, 255))
        self.assertEqual(apparence.melange(("#000000", .5), "#FFFFFF"), (128, 128, 128))

    def test_chaque_couple_texte_surface_du_theme_est_lisible(self):
        self.assertGreaterEqual(len(apparence.COUPLES), 12)
        for nom, texte, surface, grand in apparence.COUPLES:
            with self.subTest(couple=nom):
                seuil = 3 if grand else 4.5
                self.assertGreaterEqual(apparence.contraste(texte, surface), seuil)

    def test_l_encre_reste_lisible_sur_le_verre_givre_partout_sur_le_degrade(self):
        for rang in range(101):
            dessous = apparence.couleur_du_degrade(apparence.DEGRADE_DE_FOND, rang / 100)
            surface = apparence.melange(apparence.VERRE_GIVRE, dessous)
            for texte in (apparence.COULEURS["encre"], apparence.COULEURS["texte_second"]):
                with self.subTest(position=rang, texte=texte):
                    self.assertGreaterEqual(apparence.contraste(texte, surface), 4.5)

    def test_le_blanc_reste_lisible_sur_le_verre_teinte_de_la_signature(self):
        for rang in range(101):
            dessous = apparence.couleur_du_degrade(apparence.DEGRADE_SIGNATURE, rang / 100)
            surface = apparence.melange(apparence.VERRE_TEINTE, dessous)
            with self.subTest(position=rang):
                self.assertGreaterEqual(apparence.contraste("#FFFFFF", surface), 4.5)

    def test_le_blanc_est_lisible_aux_extremites_de_la_signature(self):
        for rang in list(range(0, 21)) + list(range(80, 101)):
            dessous = apparence.couleur_du_degrade(apparence.DEGRADE_SIGNATURE, rang / 100)
            with self.subTest(position=rang):
                self.assertGreaterEqual(apparence.contraste("#FFFFFF", dessous), 4.5)

    def test_les_couleurs_de_sens_restent_celles_de_la_vue_locale(self):
        self.assertEqual(apparence.COULEURS["bleu"], "#0072B2")
        self.assertEqual(apparence.COULEURS["vermillon"], "#D55E00")
        self.assertEqual(apparence.COULEURS["vert"], "#009E73")


class LesPolices(unittest.TestCase):
    def test_la_premiere_police_presente_est_choisie(self):
        presentes = ["DejaVu Sans", "adwaita sans", "Cantarell"]
        self.assertEqual(apparence.choisir_police(presentes, apparence.POLICES_SANS), "adwaita sans")
        self.assertEqual(apparence.choisir_police(["Geist", "Inter"], apparence.POLICES_SANS), "Geist")

    def test_sans_police_preferee_rien_n_est_impose(self):
        self.assertIsNone(apparence.choisir_police(["Comic"], apparence.POLICES_MONO))

    def test_les_preferences_suivent_la_maquette(self):
        self.assertEqual(apparence.POLICES_SANS[:3], ("Geist", "Inter", "Adwaita Sans"))
        self.assertEqual(apparence.POLICES_MONO[0], "Geist Mono")


class LeNomDuProjet(unittest.TestCase):
    def test_le_dossier_qui_contient_la_base(self):
        self.assertEqual(apparence.nom_du_projet("/home/a/rtos/.memoire/depot.db"), "rtos")
        self.assertEqual(apparence.nom_du_projet("/tmp/essai/depot.db"), "essai")
        self.assertEqual(apparence.nom_du_projet("/tmp/essai/evalbot.db"), "evalbot")
        self.assertEqual(apparence.nom_du_projet(""), "sans projet")


STATUT = """Branche active : 58a99889-7b83-4b73-96e9-b024d40f15b0
Ressources actuelles :
  (aucune)
À réexaminer :
  - Conséquence (c74f9aa8-f507-4159-a550-830d9686e055) — cause : Hypothèse ISR (0bd84761-dac3-4371-b984-f94fdb1fcf08)
  - Réveil (d74f9aa8-f507-4159-a550-830d9686e055) — cause : Hypothèse ISR (0bd84761-dac3-4371-b984-f94fdb1fcf08)
Orphelines :
  - Primitive (e74f9aa8-f507-4159-a550-830d9686e055)
"""
BRANCHES = ("piste\tactive\tmise-de-cote\tnom\tmotif\n"
            "00000000-0000-0000-0000-000000000000\tnon\tnon\tpiste principale\t\n"
            "58a99889-7b83-4b73-96e9-b024d40f15b0\toui\tnon\tRéfutation de « Hypothèse ISR »\tx\n")


class LEtatDuProjet(unittest.TestCase):
    def test_la_piste_active_et_les_notes_a_revoir(self):
        etat = apparence.lire_etat(STATUT, BRANCHES)
        self.assertEqual(etat.piste, "Réfutation de « Hypothèse ISR »")
        self.assertEqual(etat.a_revoir, 2)
        self.assertEqual(etat.sans_appui, 1)

    def test_un_projet_sans_rien_a_revoir(self):
        statut = ("Branche active : 0000 (main)\nRessources actuelles :\n  (aucune)\n"
                  "À réexaminer :\n  (aucune)\nOrphelines :\n  (aucune)\n")
        branches = ("piste\tactive\tmise-de-cote\tnom\tmotif\n"
                    "00000000-0000-0000-0000-000000000000\toui\tnon\tpiste principale\t\n")
        self.assertEqual(apparence.lire_etat(statut, branches),
                         apparence.Etat("piste principale", 0, 0))

    def test_sans_base_lisible_l_etat_est_vide(self):
        self.assertEqual(apparence.lire_etat("", ""), apparence.Etat("", 0, 0))

    def test_les_libelles(self):
        self.assertEqual(apparence.libelle_a_revoir(0), "rien à revoir")
        self.assertEqual(apparence.libelle_a_revoir(1), "1 note à revoir")
        self.assertEqual(apparence.libelle_a_revoir(3), "3 notes à revoir")
        self.assertEqual(apparence.libelle_sans_appui(1), "1 note sans appui")
        self.assertEqual(apparence.libelle_sans_appui(2), "2 notes sans appui")


class LaRecherche(unittest.TestCase):
    def test_sans_majuscules_ni_accents_dans_l_ordre_du_catalogue(self):
        trouvees = apparence.actions_trouvees("ECHANGE", ACTIONS)
        self.assertTrue(trouvees)
        self.assertIn("Nouvel échange avec une IA", [a.titre for a in trouvees])
        rangs = [ACTIONS.index(a) for a in trouvees]
        self.assertEqual(rangs, sorted(rangs))
        for action in trouvees:
            self.assertIn("echange", apparence.sans_accents(action.titre))

    def test_une_recherche_vide_ne_propose_rien(self):
        self.assertEqual(apparence.actions_trouvees("  ", ACTIONS), [])

    def test_sans_accents(self):
        self.assertEqual(apparence.sans_accents("Réveil Où Çà"), "reveil ou ca")


class LeRail(unittest.TestCase):
    """INV-UI-01 prolongé : chaque sujet a sa place et son pictogramme dans le rail."""

    def test_chaque_sujet_a_un_pictogramme_dessinable(self):
        import dessin
        self.assertEqual(tuple(apparence.ICONE_DU_SUJET), GROUPES)
        for sujet, icone in apparence.ICONE_DU_SUJET.items():
            with self.subTest(sujet=sujet):
                self.assertIn(icone, dessin.ICONES)

    def test_chaque_action_est_dans_un_sujet_du_rail(self):
        for action in ACTIONS:
            self.assertIn(action.groupe, apparence.ICONE_DU_SUJET)


if __name__ == "__main__":
    unittest.main()
