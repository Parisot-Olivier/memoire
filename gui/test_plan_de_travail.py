"""Contrat du modèle du plan de travail (PLAN-DE-TRAVAIL.md, sections 1 à 3 ; D28 ; INV-PLAN-01).
Écrit avant le code. Sans Tkinter.

Exécution : python3 -m unittest discover -s gui
"""
import json
from pathlib import Path
import tempfile
import unittest

from plan_de_travail import (ZONES, Defaut, Disposition, chemin_de_configuration, ecrire, lire,
                             ramener_dans_l_ecran)

DEFINITIONS = {
    "terminal": Defaut(zone="droite", rang=0),
    "essai": Defaut(zone="droite", rang=1),
}


def disposition():
    return Disposition.par_defaut(DEFINITIONS)


class LesOperations(unittest.TestCase):
    def test_par_defaut_tout_est_masque_avec_sa_place(self):
        d = disposition()
        self.assertEqual(ZONES, ("haut", "bas", "gauche", "droite"))
        self.assertEqual(d.etat("terminal").etat, "masque")
        self.assertEqual(d.zones_visibles(), [])
        d.montrer("terminal")
        self.assertEqual(d.etat("terminal").etat, "ancre")
        self.assertEqual((d.etat("terminal").zone, d.etat("terminal").rang), ("droite", 0))
        self.assertEqual(d.zones_visibles(), ["droite"])
        self.assertEqual(d.panneaux_de("droite"), ["terminal"])

    def test_rangs_dans_une_zone(self):
        d = disposition()
        d.montrer("terminal")
        d.montrer("essai")
        self.assertEqual(d.panneaux_de("droite"), ["terminal", "essai"])
        d.monter("essai")
        self.assertEqual(d.panneaux_de("droite"), ["essai", "terminal"])
        d.monter("essai")
        self.assertEqual(d.panneaux_de("droite"), ["essai", "terminal"])
        d.descendre("essai")
        d.descendre("essai")
        self.assertEqual(d.panneaux_de("droite"), ["terminal", "essai"])
        self.assertEqual([d.etat(p).rang for p in ("terminal", "essai")], [0, 1])
        d.ancrer("terminal", "bas")
        self.assertEqual(d.panneaux_de("droite"), ["essai"])
        self.assertEqual(d.etat("essai").rang, 0)
        self.assertEqual(d.zones_visibles(), ["bas", "droite"])
        d.ancrer("terminal", "droite", rang=0)
        self.assertEqual(d.panneaux_de("droite"), ["terminal", "essai"])

    def test_flottant_masque_puis_montre_a_sa_place(self):
        d = disposition()
        d.montrer("terminal")
        d.detacher("terminal", "720x420+100+80")
        e = d.etat("terminal")
        self.assertEqual((e.etat, e.geometrie), ("flottant", "720x420+100+80"))
        self.assertEqual(d.zones_visibles(), [])
        d.premier_plan("terminal", True)
        d.masquer("terminal")
        self.assertEqual(d.etat("terminal").etat, "masque")
        d.montrer("terminal")
        e = d.etat("terminal")
        self.assertEqual((e.etat, e.geometrie, e.premier_plan), ("flottant", "720x420+100+80", True))
        # Détacher sans géométrie garde la dernière connue ; ancrer revient à la dernière zone.
        d.ancrer("terminal", "gauche")
        d.detacher("terminal")
        self.assertEqual(d.etat("terminal").geometrie, "720x420+100+80")
        d.deplacer("terminal", "300x200+5+6")
        self.assertEqual(d.etat("terminal").geometrie, "300x200+5+6")
        d.ancrer("terminal")
        self.assertEqual(d.etat("terminal").zone, "gauche")
        # Montrer un panneau déjà visible ne change rien.
        avant = d.vers_json()
        d.montrer("terminal")
        self.assertEqual(d.vers_json(), avant)

    def test_parts_et_refus(self):
        d = disposition()
        d.redimensionner_zone("droite", 0.4)
        self.assertEqual(d.part_de_zone("droite"), 0.4)
        d.redimensionner_zone("droite", 0.99)
        self.assertEqual(d.part_de_zone("droite"), 0.9)
        d.redimensionner_zone("bas", 0.0)
        self.assertEqual(d.part_de_zone("bas"), 0.05)
        d.redimensionner_panneau("terminal", 0.6)
        self.assertEqual(d.etat("terminal").part, 0.6)
        with self.assertRaises(ValueError):
            d.montrer("inconnu")
        with self.assertRaises(ValueError):
            d.ancrer("terminal", "milieu")
        with self.assertRaises(ValueError):
            d.detacher("terminal", "pas une géométrie")


class LesPanneauxDynamiques(unittest.TestCase):
    """6.9 (TERMINAUX.md, section 3) : panneaux ajoutés et retirés en cours de session."""

    def test_ajouter_puis_retirer(self):
        d = disposition()
        d.montrer("terminal")
        d.ajouter("terminal-2", Defaut(zone="droite", rang=9))
        self.assertEqual(d.etat("terminal-2").etat, "masque")
        d.ancrer("terminal-2", "droite")
        self.assertEqual(d.panneaux_de("droite"), ["terminal", "terminal-2"])
        d.monter("terminal-2")
        d.retirer("terminal-2")
        self.assertEqual(d.panneaux_de("droite"), ["terminal"])
        self.assertEqual(d.etat("terminal").rang, 0)
        with self.assertRaises(ValueError):
            d.etat("terminal-2")
        with self.assertRaises(ValueError):
            d.retirer("terminal-2")
        with self.assertRaises(ValueError):
            d.ajouter("terminal", Defaut(zone="droite", rang=0))
        with self.assertRaises(ValueError):
            d.ajouter("autre", Defaut(zone="milieu", rang=0))


class LaPleinePage(unittest.TestCase):
    """8.5 (CONSOLE.md, section 1 ; D32 ; INV-PLAN-01) : un panneau peut prendre toute la carte
    (état « page »), un seul à la fois, et revient où il était."""

    def test_un_panneau_en_pleine_page_revient_ou_il_etait(self):
        d = disposition()
        d.montrer("terminal")
        d.montrer("essai")
        self.assertIsNone(d.en_page())
        d.mettre_en_page("terminal")
        e = d.etat("terminal")
        self.assertEqual((e.etat, e.retour), ("page", "ancre"))
        self.assertEqual(d.en_page(), "terminal")
        # Il a quitté sa zone, qui est renumérotée.
        self.assertEqual(d.panneaux_de("droite"), ["essai"])
        self.assertEqual(d.etat("essai").rang, 0)
        d.quitter_la_page("terminal")
        self.assertEqual(d.panneaux_de("droite"), ["terminal", "essai"])
        self.assertIsNone(d.en_page())
        # Depuis une fenêtre flottante, il redevient cette fenêtre.
        d.detacher("terminal", "720x420+100+80")
        d.mettre_en_page("terminal")
        self.assertEqual(d.etat("terminal").retour, "flottant")
        d.quitter_la_page("terminal")
        e = d.etat("terminal")
        self.assertEqual((e.etat, e.geometrie), ("flottant", "720x420+100+80"))
        # Quitter la page sans y être, ou y entrer deux fois : sans effet.
        avant = d.vers_json()
        d.quitter_la_page("terminal")
        self.assertEqual(d.vers_json(), avant)
        d.mettre_en_page("terminal")
        avant = d.vers_json()
        d.mettre_en_page("terminal")
        self.assertEqual(d.vers_json(), avant)

    def test_masquer_puis_montrer_rend_la_place_d_avant_la_page(self):
        d = disposition()
        d.montrer("terminal")
        d.mettre_en_page("terminal")
        d.masquer("terminal")
        self.assertEqual(d.etat("terminal").etat, "masque")
        self.assertIsNone(d.en_page())
        d.montrer("terminal")
        e = d.etat("terminal")
        self.assertEqual((e.etat, e.zone, e.rang), ("ancre", "droite", 0))
        # Depuis l'état masqué : la page, puis la place par défaut.
        d.masquer("terminal")
        d.mettre_en_page("terminal")
        self.assertEqual(d.etat("terminal").etat, "page")
        d.quitter_la_page("terminal")
        self.assertEqual(d.etat("terminal").etat, "ancre")

    def test_un_seul_panneau_en_pleine_page(self):
        d = disposition()
        d.montrer("terminal")
        d.montrer("essai")
        d.mettre_en_page("terminal")
        d.mettre_en_page("essai")
        self.assertEqual(d.en_page(), "essai")
        self.assertEqual(d.etat("terminal").etat, "ancre")
        self.assertEqual(d.zones_visibles(), ["droite"])

    def test_ancrer_ou_detacher_quitte_la_page(self):
        d = disposition()
        d.mettre_en_page("terminal")
        d.ancrer("terminal", "bas")
        e = d.etat("terminal")
        self.assertEqual((e.etat, e.zone, e.retour), ("ancre", "bas", "ancre"))
        d.mettre_en_page("terminal")
        d.detacher("terminal")
        self.assertEqual((d.etat("terminal").etat, d.en_page()), ("flottant", None))
        # Monter, descendre : sans effet sur un panneau en pleine page.
        d.mettre_en_page("terminal")
        d.monter("terminal")
        d.descendre("terminal")
        self.assertEqual(d.etat("terminal").etat, "page")

    def test_la_pleine_page_est_conservee(self):
        d = disposition()
        d.montrer("terminal")
        d.mettre_en_page("terminal")
        relue, avertissement = Disposition.de_json(json.loads(json.dumps(d.vers_json())), DEFINITIONS)
        self.assertIsNone(avertissement)
        self.assertEqual(relue, d)
        self.assertEqual(relue.en_page(), "terminal")
        # Deux panneaux en pleine page, ou un retour qui n'est pas une place : contenu invalide.
        modele = d.vers_json()
        deux = {**modele, "panneaux": {nom: {**modele["panneaux"]["terminal"]}
                                       for nom in ("terminal", "essai")}}
        retour = {**modele, "panneaux": {"terminal": {**modele["panneaux"]["terminal"],
                                                      "retour": "page"}}}
        for contenu in (deux, retour):
            relue, avertissement = Disposition.de_json(contenu, DEFINITIONS)
            self.assertEqual(relue, disposition(), contenu)
            self.assertIsNotNone(avertissement, contenu)


class LaConservation(unittest.TestCase):
    def test_aller_retour_json(self):
        d = disposition()
        d.montrer("terminal")
        d.montrer("essai")
        d.detacher("essai", "400x300+10+20")
        d.premier_plan("essai", True)
        d.redimensionner_zone("droite", 0.35)
        texte = json.dumps(d.vers_json())
        relue, avertissement = Disposition.de_json(json.loads(texte), DEFINITIONS)
        self.assertIsNone(avertissement)
        self.assertEqual(relue, d)
        self.assertEqual(relue.vers_json()["version"], 1)

    def test_contenu_invalide_donne_la_disposition_par_defaut(self):
        bon = disposition()
        bon.montrer("terminal")
        modele = bon.vers_json()
        invalides = [
            None,
            [],
            {"version": 2, "panneaux": {}, "zones": {}},
            {**modele, "panneaux": {"terminal": {**modele["panneaux"]["terminal"], "zone": "milieu"}}},
            {**modele, "panneaux": {"terminal": {**modele["panneaux"]["terminal"], "etat": "?"}}},
            {**modele, "panneaux": {"terminal": {**modele["panneaux"]["terminal"], "rang": -1}}},
            {**modele, "panneaux": {"terminal": {**modele["panneaux"]["terminal"], "part": 2}}},
            {**modele, "panneaux": {"terminal": {**modele["panneaux"]["terminal"],
                                                 "geometrie": "grand"}}},
            {**modele, "zones": {"droite": "large"}},
        ]
        for contenu in invalides:
            relue, avertissement = Disposition.de_json(contenu, DEFINITIONS)
            self.assertEqual(relue, disposition(), contenu)
            self.assertIsNotNone(avertissement, contenu)

    def test_panneau_inconnu_ignore_et_nouveau_a_sa_place(self):
        d = disposition()
        d.montrer("terminal")
        contenu = d.vers_json()
        contenu["panneaux"]["ancien"] = contenu["panneaux"]["terminal"]
        del contenu["panneaux"]["essai"]
        relue, avertissement = Disposition.de_json(contenu, DEFINITIONS)
        self.assertIsNone(avertissement)
        self.assertEqual(relue.etat("terminal").etat, "ancre")
        self.assertEqual(relue.etat("essai"), disposition().etat("essai"))
        with self.assertRaises(ValueError):
            relue.etat("ancien")

    def test_rangs_en_double_reordonnes(self):
        d = disposition()
        d.montrer("terminal")
        d.montrer("essai")
        contenu = d.vers_json()
        contenu["panneaux"]["terminal"]["rang"] = 5
        contenu["panneaux"]["essai"]["rang"] = 5
        relue, _ = Disposition.de_json(contenu, DEFINITIONS)
        self.assertEqual(relue.panneaux_de("droite"), ["essai", "terminal"])
        self.assertEqual([relue.etat(p).rang for p in ("essai", "terminal")], [0, 1])


class LeFichier(unittest.TestCase):
    def setUp(self):
        self.dossier = tempfile.TemporaryDirectory()
        self.chemin = Path(self.dossier.name) / "config" / "memoire" / "plan-de-travail.json"

    def tearDown(self):
        self.dossier.cleanup()

    def test_ecriture_atomique_et_relecture(self):
        d = disposition()
        d.montrer("terminal")
        ecrire(self.chemin, d)
        self.assertEqual(json.loads(self.chemin.read_text(encoding="utf-8")), d.vers_json())
        self.assertEqual(sorted(p.name for p in self.chemin.parent.iterdir()), ["plan-de-travail.json"])
        relue, avertissement = lire(self.chemin, DEFINITIONS)
        self.assertEqual((relue, avertissement), (d, None))

    def test_fichier_absent_ou_illisible(self):
        relue, avertissement = lire(self.chemin, DEFINITIONS)
        self.assertEqual((relue, avertissement), (disposition(), None))
        self.chemin.parent.mkdir(parents=True)
        self.chemin.write_text("{pas du json", encoding="utf-8")
        relue, avertissement = lire(self.chemin, DEFINITIONS)
        self.assertEqual(relue, disposition())
        self.assertIn("disposition par défaut", avertissement)

    def test_geometrie_ramenee_dans_l_ecran(self):
        self.assertEqual(ramener_dans_l_ecran("720x420+100+80", 1920, 1080), "720x420+100+80")
        self.assertEqual(ramener_dans_l_ecran("720x420+3000+2000", 1920, 1080), "720x420+1200+660")
        self.assertEqual(ramener_dans_l_ecran("4000x3000+-50+-10", 1920, 1080), "1920x1080+0+0")
        d = disposition()
        d.montrer("terminal")
        d.detacher("terminal", "720x420+3000+2000")
        ecrire(self.chemin, d)
        relue, _ = lire(self.chemin, DEFINITIONS, ecran=(1920, 1080))
        self.assertEqual(relue.etat("terminal").geometrie, "720x420+1200+660")

    def test_chemin_de_configuration(self):
        self.assertEqual(chemin_de_configuration({"XDG_CONFIG_HOME": "/c", "HOME": "/h"}),
                         Path("/c/memoire/plan-de-travail.json"))
        self.assertEqual(chemin_de_configuration({"HOME": "/h"}),
                         Path("/h/.config/memoire/plan-de-travail.json"))
        self.assertEqual(chemin_de_configuration({"XDG_CONFIG_HOME": "", "HOME": "/h"}),
                         Path("/h/.config/memoire/plan-de-travail.json"))


if __name__ == "__main__":
    unittest.main()
