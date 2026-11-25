"""Contrat du catalogue des actions de l'interface (ACTIONS.md ; D30 ; INV-UI-01). Écrit avant le
code. Sans Tkinter.

Exécution : python3 -m unittest discover -s gui
"""
from pathlib import Path
import unittest

from actions import (ACTIONS, EXPERT_SEULEMENT, GROUPES, action, commandes_de, exemples, mot,
                     traduire_apercu)
from schema import analyser, lire_aide

AIDE = (Path(__file__).resolve().parent.parent / "crates/memoire-cli/tests/aide.txt").read_text(
    encoding="utf-8")
COMMANDES = lire_aide(AIDE)


class LeCatalogue(unittest.TestCase):
    def test_groupes_et_cles(self):
        self.assertEqual(GROUPES, ("Écrire", "Relier", "Juger", "Lire", "Code", "Histoire et pistes",
                                   "Où j’en suis", "Projet"))
        cles = [a.cle for a in ACTIONS]
        self.assertEqual(len(cles), len(set(cles)))
        titres = [a.titre for a in ACTIONS]
        self.assertEqual(len(titres), len(set(titres)))
        for a in ACTIONS:
            self.assertIn(a.groupe, GROUPES, a.cle)
            self.assertIs(action(a.cle), a)

    def test_libelles_courts_et_aides(self):
        """Libellés courts et simples (ACTIONS.md, principe 2 et 6)."""
        for a in ACTIONS:
            self.assertLessEqual(len(a.titre), 45, a.titre)
            self.assertTrue(a.aide, a.cle)
            self.assertLessEqual(len(a.aide), 200, a.cle)
            for q in a.toutes_les_questions():
                self.assertLessEqual(len(q.libelle), 50, q.libelle)
                self.assertNotIn("--", q.libelle, q.libelle)
            for s in a.sections:
                self.assertLessEqual(len(s.libelle), 50, s.libelle)

    def test_les_actions_demandees_existent(self):
        for cle in ("nouvelle_note", "nouvel_echange", "relier", "refuter", "verifier",
                    "adopter_piste", "revenir_a_une_etape", "lier_au_code", "noter_ou_j_en_suis"):
            action(cle)
        self.assertEqual(action("adopter_piste").titre, "Adopter une piste")
        self.assertEqual(action("nouvel_echange").titre, "Nouvel échange avec une IA")
        with self.assertRaises(KeyError):
            action("inconnue")


class LaCoherenceAvecLaLigneDeCommande(unittest.TestCase):
    """INV-UI-01 : chaque commande produite est acceptée par l'aide figée ; chaque commande de
    l'aide est employée par une action ou réservée au mode expert."""

    def test_chaque_commande_produite_est_acceptee(self):
        for a in ACTIONS:
            reponses, sections = exemples(a)
            for argv in commandes_de(a, reponses, sections):
                try:
                    analyser(COMMANDES, argv)
                except ValueError as erreur:
                    self.fail(f"{a.cle} : {argv} refusé : {erreur}")
            # Sans aucune section cochée aussi.
            for argv in commandes_de(a, reponses, set()):
                analyser(COMMANDES, argv)

    def test_chaque_commande_est_atteignable(self):
        employees = set(EXPERT_SEULEMENT)
        for a in ACTIONS:
            employees |= set(a.emploie)
            reponses, sections = exemples(a)
            for argv in commandes_de(a, reponses, sections):
                employees.add(analyser(COMMANDES, argv)[0].nom)
        self.assertEqual({c.nom for c in COMMANDES} - employees, set())
        self.assertEqual(employees - {c.nom for c in COMMANDES}, set())

    def test_analyser_refuse_ce_que_l_aide_refuse(self):
        self.assertEqual(analyser(COMMANDES, ["note", "creer", "--role", "note", "--titre", "T"])[0].nom,
                         "note creer")
        for argv in (["inconnue"], ["note", "creer", "--titre", "T"],
                     ["note", "creer", "--role", "pas-un-role", "--titre", "T"],
                     ["note", "creer", "--role", "note", "--titre", "T", "--option-inconnue", "x"],
                     ["lier", "a"], ["verdict", "n", "refutee", "--motif"]):
            with self.assertRaises(ValueError, msg=argv):
                analyser(COMMANDES, argv)


class LesCommandesProduites(unittest.TestCase):
    def test_nouvelle_note_minimum_puis_sections(self):
        a = action("nouvelle_note")
        self.assertEqual([q.cle for q in a.questions], ["titre", "genre"])
        reponses = {"titre": "Réveil perdu", "genre": "raisonnement"}
        self.assertEqual(commandes_de(a, reponses, set()),
                         [["note", "creer", "--role", "raisonnement", "--titre", "Réveil perdu"]])
        reponses.update(texte="le tick…", appuis="aaaaaaaa,bbbbbbbb", theme="cccccccc")
        self.assertEqual(
            commandes_de(a, reponses, {"texte", "appuis", "theme"}),
            [["note", "creer", "--role", "raisonnement", "--titre", "Réveil perdu",
              "--texte", "le tick…", "--depend-de", "aaaaaaaa,bbbbbbbb", "--classe-dans", "cccccccc"]])
        # Une section non cochée ne compte pas, même remplie.
        self.assertEqual(commandes_de(a, reponses, {"texte"})[0][-2:], ["--texte", "le tick…"])

    def test_nouvel_echange_par_fichiers_ecrits_par_l_interface(self):
        a = action("nouvel_echange")
        self.assertEqual([q.cle for q in a.questions], ["titre", "reponse"])
        self.assertTrue(action("nouvel_echange").question("reponse").vers_fichier)
        reponses = {"titre": "Réveil", "reponse": "/tmp/r.txt", "question": "/tmp/q.txt",
                    "precedent": "dddddddd"}
        self.assertEqual(commandes_de(a, reponses, {"question", "precedent"}),
                         [["echange", "coller", "--titre", "Réveil", "--reponse-fichier", "/tmp/r.txt",
                           "--question-fichier", "/tmp/q.txt", "--precedent", "dddddddd"]])

    def test_relier_en_mots_simples(self):
        """Une phrase à compléter ; une question n'apparaît que pour le choix qui la demande."""
        a = action("relier")
        self.assertEqual(a.question("condition").si, ("lien", "ne-vaut-que-si"))
        self.assertEqual(a.question("raison").si, ("lien", "contredit"))
        self.assertEqual([q.cle for q in a.visibles({"lien": "depend-de"})],
                         ["origine", "lien", "destination"])
        self.assertEqual([q.cle for q in a.visibles({"lien": "contredit"})],
                         ["origine", "lien", "destination", "raison"])
        choix = dict(a.question("lien").choix)
        self.assertEqual(choix["s’appuie sur"], "depend-de")
        self.assertEqual(choix["vient de"], "developpe-a-partir-de")
        base = {"origine": "aaaaaaaa", "destination": "bbbbbbbb", "motif": ""}
        self.assertEqual(commandes_de(a, {**base, "lien": "depend-de"}, set()),
                         [["lier", "aaaaaaaa", "depend-de", "bbbbbbbb"]])
        self.assertEqual(
            commandes_de(a, {**base, "lien": "ne-vaut-que-si", "condition": "si ISR"}, set()),
            [["restreindre", "aaaaaaaa", "bbbbbbbb", "--condition", "si ISR"]])
        self.assertEqual(
            commandes_de(a, {**base, "lien": "contredit", "raison": "parce que"}, set()),
            [["contredire", "aaaaaaaa", "bbbbbbbb", "--justification", "parce que"]])

    def test_refuter_et_les_actions_importantes(self):
        a = action("refuter")
        self.assertTrue(a.importante)
        self.assertEqual([q.libelle for q in a.questions], ["Quelle note ?", "Pourquoi est-elle fausse ?"])
        self.assertEqual(commandes_de(a, {"note": "aaaaaaaa", "motif": "contre-exemple"}, set()),
                         [["verdict", "aaaaaaaa", "refutee", "--motif", "contre-exemple"]])
        self.assertFalse(action("nouvelle_note").importante)

    def test_suites_valides(self):
        """« Et ensuite ? » : chaque suite désigne une action et des questions existantes."""
        for a in ACTIONS:
            for suite in a.suites:
                cible = action(suite.action)
                cles = {q.cle for q in cible.toutes_les_questions()}
                for cle in suite.remplir:
                    self.assertIn(cle, cles, f"{a.cle} → {suite.action}.{cle}")
        suites = {s.action for s in action("nouvel_echange").suites}
        self.assertTrue({"citer", "nouvelle_note"} <= suites)


class LeVocabulaire(unittest.TestCase):
    def test_chaque_valeur_a_son_mot(self):
        for categorie, valeurs in (
                ("relation", ("classe-dans", "depend-de", "developpe-a-partir-de", "lien",
                              "remplace", "restreint", "contredit", "concerne", "traite")),
                ("verdict", ("non-verifiee", "verifiee", "refutee")),
                ("decision", ("proposee", "retenue", "plus-retenue", "remplacee")),
                ("implementation", ("non-commencee", "en-cours", "implementee")),
                ("role", ("categorie", "raisonnement", "implementation", "note", "echange",
                          "citation", "reserve"))):
            for valeur in valeurs:
                self.assertTrue(mot(categorie, valeur), (categorie, valeur))
        self.assertEqual(mot("relation", "depend-de"), "s’appuie sur")
        self.assertEqual(mot("decision", "plus-retenue"), "mise de côté")
        self.assertEqual(mot("role", "categorie"), "thème")
        self.assertEqual(mot("role", "inconnu"), "inconnu")

    def test_apercu_traduit(self):
        apercu = ("Opération critique :\n  - une branche sera créée depuis main\n"
                  "  - à réexaminer : Réveil perdu (abc)\n  - n'est plus à réexaminer : File (def)\n")
        self.assertEqual(
            traduire_apercu(apercu),
            "Ce que ça va changer :\n"
            "  - ce sera fait sur une nouvelle piste, pour ne pas toucher à la piste principale\n"
            "  - à revoir : Réveil perdu (abc)\n"
            "  - n’est plus à revoir : File (def)\n")
        self.assertEqual(traduire_apercu("autre ligne\n"), "autre ligne\n")


if __name__ == "__main__":
    unittest.main()


class LApercuPrevu(unittest.TestCase):
    """APPARENCE.md, section 9.3 (8.2b) : « Ce que ça va changer » traduit les commandes que le
    formulaire va lancer ; « Dernières étapes » lit `historique`. Sans Tkinter."""

    def test_chaque_action_se_decrit_sans_options_ni_valeurs_vides(self):
        from actions import ACTIONS, commandes_de, decrire, exemples
        for action in ACTIONS:
            reponses, sections = exemples(action)
            for argv in commandes_de(action, reponses, sections):
                with self.subTest(action=action.cle, argv=argv):
                    lignes = decrire(argv)
                    self.assertTrue(lignes)
                    for signe, quoi, objet in lignes:
                        self.assertTrue(signe and quoi)
                        self.assertNotIn("--", quoi + objet)
                        self.assertNotIn("None", quoi + objet)

    def test_une_note_avec_ses_liens(self):
        from actions import action, commandes_de, decrire
        argv = commandes_de(action("nouvelle_note"),
                            {"titre": "Réveil", "genre": "raisonnement",
                             "appuis": "Hypothèse ISR · 0bd84761", "theme": "Driver · aaaaaaaa"},
                            {"appuis", "theme"})[0]
        self.assertEqual(decrire(argv), [("+", "idée créée", "Réveil"),
                                         ("→", "s’appuie sur", "Hypothèse ISR"),
                                         ("◇", "rangée dans le thème", "Driver")])

    def test_une_refutation_et_son_motif(self):
        from actions import decrire
        self.assertEqual(decrire(["verdict", "Hypothèse ISR · 0bd84761", "refutee", "--motif",
                                  "contre-exemple"]),
                         [("✕", "réfutée (c’est faux)", "Hypothèse ISR"),
                          ("“", "pourquoi", "contre-exemple")])

    def test_une_reponse_manquante_et_une_lecture(self):
        from actions import action, commandes_de, decrire
        argv = commandes_de(action("nouvelle_note"), {"genre": "categorie"}, set())[0]
        self.assertEqual(decrire(argv), [("+", "thème créé", "…")])
        self.assertEqual(decrire(["statut"]), [("·", "rien ne change", "lecture seule")])

    def test_les_noms_affiches_remplacent_les_identifiants(self):
        from actions import decrire
        self.assertEqual(decrire(["basculer", "58a99889-7b83"],
                                 {"58a99889-7b83": "Réfutation de « H »"}),
                         [("⇄", "aller sur la piste", "Réfutation de « H »")])

    def test_les_dernieres_etapes(self):
        from actions import libelle_d_etape, lire_historique
        sortie = ("58a99889-7b83-4b73-96e9-b024d40f15b0 — verdict — contre-exemple\n"
                  "96b4c1b1-bac0-4853-ab6f-4ba24836e19f — lot — (sans motif)\n"
                  "00000000-0000-0000-0000-000000000000 — racine\n")
        etapes = lire_historique(sortie)
        self.assertEqual([e.commande for e in etapes], ["verdict", "lot", "racine"])
        self.assertEqual(etapes[0].motif, "contre-exemple")
        self.assertEqual(etapes[1].motif, "")
        self.assertEqual(etapes[0].identifiant, "58a99889-7b83-4b73-96e9-b024d40f15b0")
        for commande in ("annuler", "retablir", "basculer", "abandonner", "reactiver", "fusionner",
                         "lot", "reprise", "creer-note", "reviser", "inclure", "copier-contexte",
                         "lier", "verdict", "decision", "implementation", "retirer", "reattacher",
                         "remplacer", "restreindre", "contredire", "creer-echange",
                         "completer-question", "citer", "creer-reserve", "traiter", "etat-reserve",
                         "acquitter", "detacher", "ancrer", "retirer-ancrage", "restaurer",
                         "restaurer-notes", "racine"):
            with self.subTest(commande=commande):
                libelle = libelle_d_etape(commande)
                self.assertTrue(libelle)
                self.assertNotIn("-", libelle)
