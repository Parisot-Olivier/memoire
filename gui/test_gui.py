"""Essai de l'interface graphique (INTERFACE.md, section 3) : formulaire rempli, commande lancée,
confirmation d'une opération critique. Ignoré sans Tkinter, sans affichage ou sans exécutable.

Exécution : MEMOIRE=target/release/memoire xvfb-run -a python3 -m unittest discover -s gui
"""
import os
from pathlib import Path
import shutil
import tempfile
import time
import unittest

try:
    import tkinter as tk
except ImportError:  # pragma: no cover
    tk = None

EXECUTABLE = os.environ.get("MEMOIRE", "")
# Contrat modifié (6.7c, D28) : la disposition des panneaux se conserve dans la configuration de
# l'utilisateur ; les essais l'écrivent dans un dossier temporaire, jamais dans ~/.config.
_CONFIGURATION = tempfile.TemporaryDirectory()
os.environ["XDG_CONFIG_HOME"] = _CONFIGURATION.name
# Base de schéma 1 écrite par v0.5.0-alpha.1 (fixtures/schema1/SHA256SUMS).
SCHEMA_1 = Path(__file__).resolve().parent.parent / "fixtures" / "schema1" / "sref01-schema1.db"


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDeLInterface(unittest.TestCase):
    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))

    def tearDown(self):
        self.racine.destroy()
        self.dossier.cleanup()

    def commande(self, nom):
        return next(c for c in self.app.commandes if c.nom == nom)

    def remplir(self, nom, valeurs):
        self.app.choisir(self.commande(nom))
        for champ, valeur in valeurs.items():
            self.app.valeurs[champ].set(valeur)
        self.app.executer()

    def sortie(self):
        return self.app.sortie.get("1.0", "end")

    def test_parcours_avec_confirmation(self):
        # Contrat modifié (5.2c, D21) : 30 commandes avec « verifier » ; (5.2d) : 31 avec « migrer » ;
        # (5.4a, D23) : 33 avec « abandonner » et « reactiver » ; (5.4c) : 35 avec « fusion plan »
        # et « fusionner » ; (5.5b, D24) : 36, « passage » devenant « citer » et « retirer » s'ajoutant ;
        # (6.1a, D25) : 38 avec « source » et « citations » ; (6.3b) : 39 avec « indexer » ;
        # (6.4b) : 42 avec « historique », « restauration plan » et « restaurer » ; (6.5b) : 46
        # avec « reprise noter », « reprise voir », « reprise liste » et « reprise code » ; (6.6a) : 47
        # avec « voisinage » ; (7.2a) : 48 avec « code notes ».
        self.assertEqual(len(self.app.commandes), 48)
        self.app.choisir(self.commande("init"))
        self.assertIn("Dépôt créé", self.sortie())
        self.remplir("note creer", {"--role": "raisonnement", "--titre": "hypothèse"})
        note = self.sortie().split("Note créée : ")[1].split("\n")[0]
        self.remplir("verdict", {"NOTE": note, "VALEUR": "refutee", "--motif": "contre-exemple"})
        self.assertIn("[code 4]", self.sortie())
        self.assertEqual(str(self.app.bouton_confirmer["state"]), "normal")
        self.app.confirmer()
        self.assertIn("Branche créée", self.sortie())
        self.assertEqual(str(self.app.bouton_confirmer["state"]), "disabled")
        self.app.executer_args(["statut"])
        self.assertIn("Branche active", self.sortie())

    def test_la_fiche_de_reprise_s_affiche_a_l_ouverture(self):
        """6.5 (FICHE-REPRISE.md, section 6) : à l'ouverture, la dernière fiche de la branche
        active s'affiche dans la zone de sortie ; sans fiche, rien ne s'affiche."""
        self.assertNotIn("Point de reprise", self.sortie())
        self.app.executer_args(["init"])
        self.app.executer_args(["reprise", "noter", "--objectif", "isoler le réveil perdu",
                                "--action", "instrumenter wake_task", "--sans-code"])
        self.racine.destroy()
        from memoire_gui import Application
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        sortie = self.sortie()
        self.assertIn("Point de reprise ", sortie)
        self.assertIn("Objectif : isoler le réveil perdu", sortie)
        self.assertIn("Prochaine action : instrumenter wake_task", sortie)

    def test_autocompletion_par_nom(self):
        """R3 : un nom tapé dans un champ de note propose les notes, le choix donne l'identifiant.
        Contrat modifié (7.2, RETROUVER.md, section 1) : la proposition se lit « nom — rôle » en mots
        simples, et la note choisie s'affiche « nom · id8 » ; la commande reçoit l'identifiant."""
        self.app.choisir(self.commande("init"))
        self.remplir("note creer", {"--role": "raisonnement", "--titre": "Réveil perdu"})
        note = self.sortie().split("Note créée : ")[1].split("\n")[0]
        self.app.choisir(self.commande("verdict"))
        liste = next(w for w in self.app.formulaire.winfo_children()
                     if w.winfo_class() == "TCombobox" and w.cget("textvariable") == str(self.app.valeurs["NOTE"]))
        self.app.valeurs["NOTE"].set("reveil")
        self.racine.tk.call(liste.cget("postcommand"))
        valeurs = list(liste.cget("values"))
        self.assertEqual(valeurs, ["Réveil perdu — idée"])
        self.app.valeurs["NOTE"].set(valeurs[0])
        liste.event_generate("<<ComboboxSelected>>")
        self.assertEqual(self.app.valeurs["NOTE"].get(), f"Réveil perdu · {note[:8]}")
        self.app.valeurs["--motif"].set("vérifiée")
        self.app.executer()
        self.assertIn(f"$ memoire verdict {note} ", self.sortie())

    def test_autocompletion_a_plusieurs_notes(self):
        """5.3 : dans un champ NOTES, la liste porte sur la dernière note tapée, et le choix
        ne remplace qu'elle ; la note créée est reliée aux deux notes en une seule opération.
        Contrat modifié (7.2) : proposition « nom — rôle », note choisie affichée « nom · id8 »."""
        self.app.choisir(self.commande("init"))
        self.remplir("note creer", {"--role": "raisonnement", "--titre": "Réveil perdu"})
        premiere = self.sortie().split("Note créée : ")[1].split("\n")[0]
        self.remplir("note creer", {"--role": "raisonnement", "--titre": "Primitive commune"})
        seconde = self.sortie().split("Note créée : ")[-1].split("\n")[0]
        self.app.choisir(self.commande("note creer"))
        variable = self.app.valeurs["--depend-de"]
        liste = next(w for w in self.app.formulaire.winfo_children()
                     if w.winfo_class() == "TCombobox" and w.cget("textvariable") == str(variable))
        variable.set(f"{premiere},primitive")
        self.racine.tk.call(liste.cget("postcommand"))
        valeurs = list(liste.cget("values"))
        self.assertEqual(valeurs, ["Primitive commune — idée"])
        variable.set(valeurs[0])
        liste.event_generate("<<ComboboxSelected>>")
        self.assertEqual(variable.get(), f"{premiere}, Primitive commune · {seconde[:8]}")
        self.app.valeurs["--role"].set("implementation")
        self.app.valeurs["--titre"].set("Gestionnaire d'attente")
        self.app.executer()
        sortie = self.sortie()
        self.assertEqual(sortie.count("Relation créée : "), 2)
        self.assertIn(f"(depend-de {premiere})", sortie)
        self.assertIn(f"(depend-de {seconde})", sortie)

    def base_schema_1(self):
        """Remplace la base de l'essai par une copie de la base figée au schéma 1."""
        shutil.copy(SCHEMA_1, self.app.base.get())

    def test_une_base_de_schema_1_propose_la_migration(self):
        """5.2d : une commande refusée pour le schéma 1 fait poser la question ; « oui » migre."""
        self.base_schema_1()
        questions = []
        self.app.demander = lambda titre, texte: questions.append((titre, texte)) or True
        self.app.executer_args(["statut"])
        self.assertEqual(len(questions), 1)
        self.assertIn("sauvegarde", questions[0][1])
        self.assertIn("Sauvegarde écrite : ", self.sortie())
        self.assertIn("Base migrée au schéma 2 : 47 commits, 2 branches, vérifiée.", self.sortie())
        self.assertEqual(self.app.executer_args(["statut"]), 0)
        self.assertEqual(len(questions), 1)

    def test_la_migration_n_est_jamais_automatique(self):
        """5.2d : « non » ne lance rien ; la base reste au schéma 1."""
        self.base_schema_1()
        avant = Path(self.app.base.get()).read_bytes()
        questions = []
        self.app.demander = lambda titre, texte: questions.append((titre, texte)) and False
        self.assertEqual(self.app.executer_args(["statut"]), 3)
        self.assertEqual(len(questions), 1)
        self.assertNotIn("Base migrée", self.sortie())
        self.assertEqual(Path(self.app.base.get()).read_bytes(), avant)
        self.assertFalse((Path(self.dossier.name) / "sauvegardes").exists())


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDuVisionneur(unittest.TestCase):
    """VISIONNEUR.md, sections 1 et 5 (D25 ; INV-VIEW-01, INV-SOURCE-02) : lecture rendue ou
    brute, surlignage des citations, citation d'une sélection, versions d'une note."""

    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.executer_args(["init"])

    def tearDown(self):
        self.racine.destroy()
        self.dossier.cleanup()

    def creer(self, args, debut):
        self.app.executer_args(args)
        sortie = self.app.sortie.get("1.0", "end")
        return sortie.split(debut)[-1].split("\n")[0].split(" ")[0]

    def ouvrir(self, note):
        from visionneur import Visionneur
        return Visionneur(self.app, note)

    def selectionner(self, zone, texte):
        """Sélectionne la première occurrence affichée de `texte` dans une zone."""
        affiche = zone.texte.get("1.0", "end-1c")
        debut = affiche.index(texte)
        zone.texte.tag_remove("sel", "1.0", "end")
        zone.texte.tag_add("sel", f"1.0 + {debut} chars", f"1.0 + {debut + len(texte)} chars")

    def surlignes(self, zone):
        """Textes affichés sous l'étiquette des citations."""
        plages = zone.texte.tag_ranges("citation")
        return [zone.texte.get(a, b) for a, b in zip(plages[::2], plages[1::2])]

    def test_une_note_se_lit_rendue_ou_brute_et_se_cite_par_selection(self):
        source = "# Titre\nla **priorité** est conservée"
        note = self.creer(["note", "creer", "--role", "raisonnement", "--titre", "Invariant",
                           "--texte", source], "Note créée : ")
        vue = self.ouvrir(note)
        (zone,) = vue.zones
        self.assertEqual(zone.texte.get("1.0", "end-1c"), "Titre\nla priorité est conservée")
        vue.basculer_brut()
        self.assertEqual(zone.texte.get("1.0", "end-1c"), source)
        vue.basculer_brut()
        self.selectionner(zone, "priorité")
        self.assertEqual(vue.citer_selection(), 0)
        # La citation porte sur le texte exact de la source, sans les marques qui l'entourent.
        resultat = self.app.executer_args(["citations", note])
        self.assertEqual(resultat, 0)
        self.assertIn("\t13\t22\tnon-verifiee", self.app.sortie.get("1.0", "end"))
        self.assertEqual(self.surlignes(zone), ["priorité"])
        # En brut, le même surlignage porte sur les mêmes caractères de la source (INV-SOURCE-02).
        vue.basculer_brut()
        self.assertEqual(self.surlignes(zone), ["priorité"])
        # Un clic sur un surlignage montre la citation.
        vue.montrer_citation(zone, "1.0 + 15 chars")
        self.assertIn("Citation ", vue.etat.cget("text"))
        # Une sélection vide ne cite rien.
        zone.texte.tag_remove("sel", "1.0", "end")
        self.assertIsNone(vue.citer_selection())

    def test_un_echange_montre_question_et_reponse(self):
        dossier = Path(self.dossier.name)
        (dossier / "q.txt").write_text("Pourquoi `wake` ?", encoding="utf-8")
        (dossier / "r.txt").write_text("Le **réveil** peut être perdu.", encoding="utf-8")
        echange = self.creer(["echange", "coller", "--titre", "Réveil", "--question-fichier",
                              str(dossier / "q.txt"), "--reponse-fichier", str(dossier / "r.txt")],
                             "Échange créé : ")
        self.app.executer_args(["citer", echange, "--texte", "perdu"])
        vue = self.ouvrir(echange)
        question, reponse = vue.zones
        self.assertEqual(question.texte.get("1.0", "end-1c"), "Pourquoi wake ?")
        self.assertEqual(reponse.texte.get("1.0", "end-1c"), "Le réveil peut être perdu.")
        self.assertEqual(self.surlignes(reponse), ["perdu"])
        self.assertEqual(self.surlignes(question), [])
        self.selectionner(question, "wake")
        self.assertEqual(vue.citer_selection(), 0)
        self.assertEqual(self.surlignes(question), ["wake"])

    def test_une_citation_critique_demande_confirmation(self):
        note = self.creer(["note", "creer", "--role", "raisonnement", "--titre", "Hypothèse",
                           "--texte", "un appui fragile"], "Note créée : ")
        self.app.executer_args(["verdict", note, "refutee", "--motif", "contre-exemple", "--oui"])
        vue = self.ouvrir(note)
        (zone,) = vue.zones
        self.selectionner(zone, "fragile")
        self.assertEqual(vue.citer_selection(), 4)
        self.assertEqual(self.surlignes(zone), [])
        self.assertEqual(str(vue.bouton_confirmer["state"]), "normal")
        self.assertEqual(vue.confirmer(), 0)
        self.assertEqual(self.surlignes(zone), ["fragile"])
        self.assertEqual(str(vue.bouton_confirmer["state"]), "disabled")

    def test_le_bouton_voir_ouvre_le_visionneur_de_la_note_du_champ(self):
        """VISIONNEUR.md, section 5 : « Voir » à côté d'un champ de note (ajouté après le code)."""
        note = self.creer(["note", "creer", "--role", "raisonnement", "--titre", "Invariant",
                           "--texte", "texte vu"], "Note créée : ")
        self.app.choisir(next(c for c in self.app.commandes if c.nom == "verdict"))
        self.app.valeurs["NOTE"].set(note)
        bouton = next(w for w in self.app.formulaire.winfo_children()
                      if w.winfo_class() == "TButton" and w.cget("text") == "Voir")
        bouton.invoke()
        # Contrat modifié (8.2c, APPARENCE.md, section 9.3) : la note s'ouvre dans la carte, plus
        # dans une fenêtre ; « Ouvrir dans une fenêtre » la détache.
        fenetres = [w for w in self.racine.winfo_children() if isinstance(w, tk.Toplevel)]
        self.assertEqual(len(fenetres), 0)
        self.assertEqual(self.app.lecture.note, note)
        self.app.lecture.ouvrir_dans_une_fenetre()
        fenetres = [w for w in self.racine.winfo_children() if isinstance(w, tk.Toplevel)]
        self.assertEqual(len(fenetres), 1)
        self.assertIsNone(self.app.voir(""))

    def test_relier_la_selection_en_un_geste(self):
        """CENTRE.md, sections 1 et 4 (INV-WORK-01) : la sélection est citée, reliée à une note
        choisie par son nom, d'où naît une note nouvelle et une réserve, en une opération ; le
        panneau se vide après réussite."""
        soutenue = self.creer(["note", "creer", "--role", "raisonnement", "--titre", "Hypothèse",
                               "--texte", "un appui"], "Note créée : ")
        dossier = Path(self.dossier.name)
        (dossier / "r.txt").write_text("le réveil peut être perdu", encoding="utf-8")
        echange = self.creer(["echange", "coller", "--titre", "Conversation", "--reponse-fichier",
                              str(dossier / "r.txt")], "Échange créé : ")
        vue = self.ouvrir(echange)
        _, reponse = vue.zones
        # Autocomplétion par nom dans le champ « Soutient ».
        variable = vue.liens["--soutient"]
        liste = vue.listes["--soutient"]
        variable.set("hypo")
        self.racine.tk.call(liste.cget("postcommand"))
        valeurs = list(liste.cget("values"))
        # Contrat modifié (7.2, RETROUVER.md, section 1) : proposition « nom — rôle » en mots
        # simples, note choisie affichée « nom · id8 » ; citer reçoit l'identifiant (plus bas).
        self.assertEqual(valeurs, ["Hypothèse — idée"])
        variable.set(valeurs[0])
        liste.event_generate("<<ComboboxSelected>>")
        self.assertEqual(variable.get(), f"Hypothèse · {soutenue[:8]}")
        vue.nouvelle_note.set("Idée née du réveil")
        vue.reserve.set("Vérifier le réveil")
        self.selectionner(reponse, "réveil")
        self.assertEqual(vue.citer_selection(), 0)
        sortie = self.app.sortie.get("1.0", "end")
        citation = sortie.split("Citation créée : ")[-1].split(" ")[0]
        self.assertIn(f"({soutenue} depend-de {citation})", sortie)
        self.assertIn("Note créée : ", sortie)
        self.assertIn("Réserve créée : ", sortie)
        self.assertEqual(self.surlignes(reponse), ["réveil"])
        self.assertEqual((variable.get(), vue.nouvelle_note.get(), vue.reserve.get()), ("", "", ""))

    def test_la_reserve_n_est_offerte_que_pour_un_echange(self):
        note = self.creer(["note", "creer", "--role", "raisonnement", "--titre", "Invariant",
                           "--texte", "texte"], "Note créée : ")
        vue = self.ouvrir(note)
        self.assertEqual(str(vue.champ_reserve["state"]), "disabled")

    def test_les_versions_d_une_note_se_lisent_avec_leurs_citations(self):
        note = self.creer(["note", "creer", "--role", "raisonnement", "--titre", "Invariant",
                           "--texte", "première version"], "Note créée : ")
        self.app.executer_args(["citer", note, "--texte", "première"])
        self.app.executer_args(["note", "reviser", note, "--texte", "seconde version",
                                "--nature", "mineure"])
        vue = self.ouvrir(note)
        (zone,) = vue.zones
        self.assertEqual(vue.versions(), ["0", "1"])
        self.assertEqual(zone.texte.get("1.0", "end-1c"), "seconde version")
        self.assertEqual(self.surlignes(zone), [])
        vue.choisir_version("0")
        self.assertEqual(zone.texte.get("1.0", "end-1c"), "première version")
        self.assertEqual(self.surlignes(zone), ["première"])
        vue.montrer_citation(zone, "1.0 + 2 chars")
        self.assertIn("source révisée", vue.etat.cget("text"))
        # Une sélection dans la version affichée cite cette version.
        self.selectionner(zone, "version")
        self.assertEqual(vue.citer_selection(), 0)
        self.assertEqual(self.surlignes(zone), ["première", "version"])


if __name__ == "__main__":
    unittest.main()


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDeLaVueLocale(unittest.TestCase):
    """VOISINAGE.md, sections 1 et 4 (INV-VIEW-02 ; R7) : un lien de chaque origine dessiné avec
    sa couleur et son trait ; clic pour recentrer ; « Précédent » ; ouverture dans le visionneur."""

    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.executer_args(["init"])

    def tearDown(self):
        self.racine.destroy()
        self.dossier.cleanup()

    def creer(self, args, debut):
        self.app.executer_args(args)
        sortie = self.app.sortie.get("1.0", "end")
        return sortie.split(debut)[-1].split("\n")[0].split(" ")[0]

    def note(self, role, titre):
        return self.creer(["note", "creer", "--role", role, "--titre", titre, "--texte", titre],
                          "Note créée : ")

    def reseau(self):
        centre = self.note("raisonnement", "Réveil perdu")
        appui = self.note("implementation", "Primitive commune")
        categorie = self.note("categorie", "Scheduler")
        self.app.executer_args(["lier", centre, "depend-de", appui])
        self.app.executer_args(["lier", centre, "classe-dans", categorie])
        citation = self.creer(["citer", appui, "--texte", "Primitive", "--soutient", centre],
                              "Citation créée : ")
        return centre, appui, categorie, citation

    def test_chaque_origine_est_dessinee_avec_son_style(self):
        from voisinage import STYLES, Voisinage
        centre, appui, categorie, citation = self.reseau()
        vue = Voisinage(self.app, centre)
        self.assertEqual(vue.note, centre)
        traits = {}
        for item in vue.dessin.find_withtag("lien"):
            origine = next(t.split(":", 1)[1] for t in vue.dessin.gettags(item)
                           if t.startswith("origine:"))
            traits[origine] = (vue.dessin.itemcget(item, "fill"), vue.dessin.itemcget(item, "dash"))
        self.assertEqual(set(traits), {"note", "citation", "categorie"})
        for origine, (couleur, tirets) in STYLES.items():
            self.assertEqual(traits[origine][0].lower(), couleur.lower())
            self.assertEqual(bool(traits[origine][1]), bool(tirets))
        self.assertEqual(len(vue.dessin.find_withtag("voisin")), 3)
        self.assertIn("Réveil perdu", vue.titre.cget("text"))
        liste = vue.liste.get("1.0", "end")
        self.assertIn("→ depend-de Primitive commune — note entière", liste)
        self.assertIn("→ classe-dans Scheduler — catégorie", liste)
        self.assertIn("— citation", liste)
        # Chaque voisin répond au clic et au double-clic.
        item = vue.dessin.find_withtag(f"voisin:{appui}")[0]
        self.assertTrue(vue.dessin.tag_bind(item, "<Button-1>") or vue.dessin.tag_bind(f"voisin:{appui}", "<Button-1>"))
        self.assertTrue(vue.dessin.tag_bind(f"voisin:{appui}", "<Double-Button-1>"))

    def test_recentrer_puis_revenir(self):
        from voisinage import Voisinage
        centre, appui, categorie, _ = self.reseau()
        vue = Voisinage(self.app, centre)
        self.assertEqual(str(vue.bouton_precedent["state"]), "disabled")
        vue.recentrer(categorie)
        self.assertEqual(vue.note, categorie)
        self.assertIn("Scheduler", vue.titre.cget("text"))
        self.assertEqual(len(vue.dessin.find_withtag("voisin")), 1)
        self.assertEqual(str(vue.bouton_precedent["state"]), "normal")
        vue.precedent()
        self.assertEqual(vue.note, centre)
        self.assertEqual(str(vue.bouton_precedent["state"]), "disabled")
        # Une modification faite ailleurs apparaît après « Actualiser ».
        autre = self.note("note", "Nouvelle voisine")
        self.app.executer_args(["lier", autre, "lien", centre])
        vue.actualiser()
        self.assertEqual(len(vue.dessin.find_withtag("voisin")), 4)
        # Double-clic : le visionneur de la note.
        visionneur = vue.ouvrir(appui)
        self.assertEqual(visionneur.note, appui)

    def test_la_vue_s_ouvre_depuis_la_fenetre_principale(self):
        centre, _, _, _ = self.reseau()
        vue = self.app.voisinage(centre)
        self.assertEqual(vue.note, centre)
        self.assertIsNone(self.app.voisinage(""))


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file()
                     and shutil.which("bash"), "Tkinter, affichage, exécutable memoire ou bash absent")
class EssaiDuPlanDeTravail(unittest.TestCase):
    """Contrat modifié (8.5, D32, CONSOLE.md) : les terminaux ne sont plus des panneaux ; le panneau
    du plan de travail est la console, qui les contient tous. Ce que ces essais vérifiaient sur le
    panneau « terminal-1 » (6.7, 6.9) l'est sur le panneau « console » ; la marque de fin d'une
    commande hors de la vue est portée par le terminal. `app.terminal` reste le premier terminal
    et `app.basculer_terminal()` montre ou masque la console.
    PLAN-DE-TRAVAIL.md et TERMINAL.md (D27, D28 ; INV-TERM-01, INV-PLAN-01) : la console, ancrée
    en haut à droite, détachée, rattachée, en pleine page, masquée, sans relancer un shell ;
    taille transmise ; notification de fin ; disposition conservée ; arrêt à la fermeture."""

    def setUp(self):
        self.dossier = tempfile.TemporaryDirectory()
        maison = Path(self.dossier.name) / "maison"
        maison.mkdir()
        (maison / ".bashrc").write_text("PS1='$ '\n", encoding="utf-8")
        self.anciens = {k: os.environ.get(k) for k in ("HOME", "SHELL", "XDG_CONFIG_HOME")}
        os.environ.update(HOME=str(maison), SHELL=shutil.which("bash"),
                          XDG_CONFIG_HOME=str(Path(self.dossier.name) / "config"))
        self.notifications = []
        self.ouvrir()

    def ouvrir(self):
        from memoire_gui import Application
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.notifier = lambda titre, corps: self.notifications.append((titre, corps)) or True

    def tearDown(self):
        self.app.fermer()
        for cle, valeur in self.anciens.items():
            if valeur is None:
                os.environ.pop(cle, None)
            else:
                os.environ[cle] = valeur
        self.dossier.cleanup()

    def attendre(self, condition, delai=10.0):
        limite = time.monotonic() + delai
        while not condition():
            if time.monotonic() > limite:
                self.fail(f"délai dépassé ; terminal : {self.app.terminal.texte()!r}")
            self.racine.update()
            time.sleep(0.02)

    def commande(self, texte):
        avant = len(self.app.terminal.fins)
        self.app.terminal.executer(texte)
        self.attendre(lambda: len(self.app.terminal.fins) > avant)
        return self.app.terminal.fins[-1]

    def test_la_console_s_ouvre_ancree_en_haut_a_droite(self):
        plan = self.app.plan
        self.assertEqual(plan.disposition.etat("console").etat, "masque")
        self.assertIsNone(self.app.terminal.session)
        self.app.basculer_terminal()
        etat = plan.disposition.etat("console")
        self.assertEqual((etat.etat, etat.zone, etat.rang), ("ancre", "droite", 0))
        self.racine.update()
        self.assertTrue(plan.cadre("console").winfo_ismapped())
        self.assertEqual(str(plan.cadre("console").winfo_toplevel()), str(self.racine))
        self.attendre(lambda: self.app.terminal.session is not None)
        self.assertEqual(self.commande("echo bonjour").code, 0)
        self.assertIn("bonjour", self.app.terminal.texte())
        # Le menu « Panneaux » liste la console et la disposition par défaut ; plus de panneau
        # « Terminaux » ni de panneau par terminal.
        menu = plan.menu
        libelles = [menu.entrycget(i, "label") for i in range(menu.index("end") + 1)
                    if menu.type(i) != "separator"]
        self.assertIn("Console", libelles)
        self.assertNotIn("Terminaux", libelles)
        self.assertIn("Disposition par défaut", libelles)
        # Masquer puis réafficher par le bouton.
        self.app.basculer_terminal()
        self.assertEqual(plan.disposition.etat("console").etat, "masque")
        self.racine.update()
        self.assertFalse(plan.cadre("console").winfo_ismapped())
        self.app.basculer_terminal()
        self.assertEqual(plan.disposition.etat("console").etat, "ancre")

    def test_detacher_et_rattacher_sans_relancer_le_shell(self):
        plan = self.app.plan
        self.app.basculer_terminal()
        self.attendre(lambda: self.app.terminal.session is not None)
        self.commande("echo avant")
        pid = self.app.terminal.session.pid
        plan.detacher("console", "900x400+20+20")
        self.racine.update()
        cadre = plan.cadre("console")
        self.assertEqual(str(cadre.winfo_toplevel()), str(cadre))
        self.assertEqual(plan.texte_du_bouton("console"), "Ancrer")
        plan.ancrer("console", "bas")
        self.racine.update()
        self.assertEqual(str(cadre.winfo_toplevel()), str(self.racine))
        self.assertTrue(cadre.winfo_ismapped())
        self.assertEqual(plan.texte_du_bouton("console"), "Détacher")
        # Pleine page (8.5) : la console couvre le plan de travail, puis revient en bas.
        plan.mettre_en_page("console")
        self.racine.update()
        self.assertTrue(cadre.winfo_ismapped())
        self.assertLessEqual(abs(cadre.winfo_width() - plan.vertical.winfo_width()), 2)
        self.assertLessEqual(abs(cadre.winfo_height() - plan.vertical.winfo_height()), 2)
        plan.quitter_la_page("console")
        self.racine.update()
        etat = plan.disposition.etat("console")
        self.assertEqual((etat.etat, etat.zone), ("ancre", "bas"))
        plan.masquer("console")
        plan.montrer("console")
        self.assertIs(plan.cadre("console"), cadre)
        self.assertEqual(self.app.terminal.session.pid, pid)
        self.assertTrue(self.app.terminal.session.en_vie())
        self.assertIn("avant", self.app.terminal.texte())
        self.assertEqual(self.commande("echo apres").code, 0)

    def test_la_taille_du_panneau_est_transmise_au_shell(self):
        self.app.basculer_terminal()
        self.app.plan.detacher("console", "1000x500+0+0")
        self.attendre(lambda: self.app.terminal.session is not None
                      and self.app.terminal.session.colonnes > 60)
        self.commande("stty size")
        session = self.app.terminal.session
        self.assertIn(f"{session.lignes} {session.colonnes}", self.app.terminal.texte())

    def test_fin_d_une_commande_longue_hors_de_la_vue(self):
        self.app.basculer_terminal()
        self.attendre(lambda: self.app.terminal.session is not None)
        self.app.terminal.seuil = 0.0
        avant = len(self.app.terminal.fins)
        self.app.terminal.executer("sleep 0.3; false")
        self.app.plan.masquer("console")
        self.attendre(lambda: len(self.app.terminal.fins) > avant)
        self.attendre(lambda: self.notifications)
        titre, corps = self.notifications[-1]
        self.assertEqual(titre, "memoire — commande terminée")
        self.assertTrue(corps.startswith("sleep 0.3; false — échec (code 1) en "), corps)
        self.assertEqual(self.app.terminal.marque, "● terminé, code 1")
        self.app.terminaux.afficher("terminal-1")
        self.assertTrue(self.app.plan.visible("console"))
        self.assertEqual(self.app.terminal.marque, "")

    def test_l_utilisateur_decide_de_l_ordre_d_une_fenetre_flottante(self):
        """6.8 (PLAN-DE-TRAVAIL.md, section 10) : menu inactif pour un panneau ancré ; « toujours au
        premier plan » tenu par la disposition et conservé ; « mettre derrière » le décoche.
        Contrat modifié (6.8) : la partie qui dépend du gestionnaire de fenêtres est dans l'essai
        suivant ; sans gestionnaire (Xvfb seul), Tk relit l'état appliqué par lui, donc rien."""
        plan = self.app.plan
        self.app.basculer_terminal()
        self.assertEqual(plan.etat_du_menu_plan("console"), "disabled")
        plan.detacher("console", "600x300+40+40")
        self.racine.update()
        self.assertEqual(plan.etat_du_menu_plan("console"), "normal")
        plan.premier_plan("console", True)
        self.assertTrue(plan.disposition.etat("console").premier_plan)
        self.assertTrue(plan.case_premier_plan("console"))
        chemin = Path(os.environ["XDG_CONFIG_HOME"]) / "memoire" / "plan-de-travail.json"
        self.assertIn('"premier_plan": true', chemin.read_text(encoding="utf-8"))
        plan.mettre_derriere("console")
        self.assertFalse(plan.disposition.etat("console").premier_plan)
        self.assertFalse(plan.case_premier_plan("console"))
        plan.mettre_devant("console")
        self.racine.update()
        self.assertTrue(plan.cadre("console").winfo_ismapped())
        self.assertEqual(plan.disposition.etat("console").etat, "flottant")
        plan.ancrer("console")
        self.assertEqual(plan.etat_du_menu_plan("console"), "disabled")

    @unittest.skipUnless(shutil.which("xprop") and "_NET_SUPPORTING_WM_CHECK(WINDOW)" in __import__(
        "subprocess").run(["xprop", "-root", "_NET_SUPPORTING_WM_CHECK"], capture_output=True,
                          text=True).stdout, "aucun gestionnaire de fenêtres (Xvfb seul)")
    def test_sous_un_gestionnaire_de_fenetres_l_ordre_est_celui_demande(self):
        """6.8, avec un gestionnaire de fenêtres (check.sh lance KWin virtuel s'il est installé) :
        le cas signalé par l'utilisateur, un clic dans la fenêtre principale, laisse la console
        devant quand « toujours au premier plan » est coché, et la fait passer derrière sinon ;
        « mettre devant » et « mettre derrière » font ce qu'ils disent. Chaque demande est appliquée
        par le gestionnaire après un court délai : on l'attend."""
        plan = self.app.plan
        self.app.basculer_terminal()
        plan.detacher("console", "600x300+40+40")
        cadre = plan.cadre("console")
        tk_ = self.racine.tk
        devant = lambda: bool(tk_.call("wm", "stackorder", cadre, "isabove", self.racine))
        derriere = lambda: bool(tk_.call("wm", "stackorder", cadre, "isbelow", self.racine))

        def cliquer_dans_la_fenetre_principale():
            self.racine.focus_force()
            tk_.call("raise", self.racine)
            fin = time.monotonic() + 0.6
            while time.monotonic() < fin:
                self.racine.update()
                time.sleep(0.02)

        self.attendre(lambda: cadre.winfo_ismapped() and (devant() or derriere()))
        cliquer_dans_la_fenetre_principale()
        self.assertTrue(derriere(), "sans « toujours au premier plan », la console passe derrière")
        plan.premier_plan("console", True)
        self.attendre(lambda: int(tk_.call("wm", "attributes", cadre, "-topmost")) == 1)
        self.attendre(devant)
        cliquer_dans_la_fenetre_principale()
        self.assertTrue(devant(), "toujours au premier plan : elle reste devant")
        plan.mettre_derriere("console")
        self.attendre(lambda: int(tk_.call("wm", "attributes", cadre, "-topmost")) == 0)
        self.attendre(derriere)
        plan.mettre_devant("console")
        self.attendre(devant)

    def test_la_disposition_est_conservee_et_le_shell_arrete_a_la_fermeture(self):
        self.app.basculer_terminal()
        self.app.plan.ancrer("console", "gauche")
        self.attendre(lambda: self.app.terminal.session is not None)
        self.commande("true")
        pid = self.app.terminal.session.pid
        self.app.fermer()
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)
        chemin = Path(os.environ["XDG_CONFIG_HOME"]) / "memoire" / "plan-de-travail.json"
        self.assertTrue(chemin.is_file())
        self.ouvrir()
        etat = self.app.plan.disposition.etat("console")
        self.assertEqual((etat.etat, etat.zone), ("ancre", "gauche"))
        self.racine.update()
        self.assertTrue(self.app.plan.cadre("console").winfo_ismapped())
        # Disposition par défaut : la console masquée à sa place.
        self.app.plan.disposition_par_defaut()
        self.assertEqual(self.app.plan.disposition.etat("console").etat, "masque")


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file()
                     and shutil.which("bash"), "Tkinter, affichage, exécutable memoire ou bash absent")
class EssaiDePlusieursTerminaux(unittest.TestCase):
    """Contrat modifié (8.5, D32, CONSOLE.md) : un nouveau terminal va sur la page de la console
    au lieu d'un panneau ancré ; « masquer » un terminal devient le ranger en arrière-plan ; mettre
    devant et derrière portent sur la console (EssaiDuPlanDeTravail) ; le panneau « Terminaux » et
    le menu « Terminaux ▾ » n'existent plus (EssaiDeLaConsole les remplace).
    TERMINAUX.md (D29 ; INV-TERM-03, INV-TERM-01) : plusieurs terminaux nommés par ce qu'ils
    font, actions, conservation."""

    def setUp(self):
        self.dossier = tempfile.TemporaryDirectory()
        self.maison = Path(self.dossier.name) / "maison"
        self.maison.mkdir()
        (self.maison / ".bashrc").write_text("PS1='$ '\n", encoding="utf-8")
        self.anciens = {k: os.environ.get(k) for k in ("HOME", "SHELL", "XDG_CONFIG_HOME")}
        os.environ.update(HOME=str(self.maison), SHELL=shutil.which("bash"),
                          XDG_CONFIG_HOME=str(Path(self.dossier.name) / "config"))
        self.ancien_dossier = os.getcwd()
        os.chdir(self.maison)
        self.ouvrir()

    def ouvrir(self):
        from memoire_gui import Application
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.notifier = lambda titre, corps: True
        self.gestion = self.app.terminaux

    def tearDown(self):
        self.app.fermer()
        os.chdir(self.ancien_dossier)
        for cle, valeur in self.anciens.items():
            if valeur is None:
                os.environ.pop(cle, None)
            else:
                os.environ[cle] = valeur
        self.dossier.cleanup()

    def attendre(self, condition, delai=10.0):
        limite = time.monotonic() + delai
        while not condition():
            if time.monotonic() > limite:
                self.fail(f"délai dépassé ; lignes : {self.gestion.lignes()!r}")
            self.racine.update()
            time.sleep(0.02)

    def nom(self, interne):
        return next(l[1] for l in self.gestion.lignes() if l[0] == interne)

    def lance(self, interne):
        terminal = self.gestion.terminal(interne)
        return terminal.session is not None and terminal.tampon.dossier is not None

    def commande(self, interne, texte):
        terminal = self.gestion.terminal(interne)
        avant = len(terminal.fins)
        terminal.executer(texte)
        self.attendre(lambda: len(terminal.fins) > avant)

    def test_plusieurs_terminaux_nommes_par_ce_qu_ils_font(self):
        self.assertEqual(self.gestion.noms_internes(), ["terminal-1"])
        deux = self.gestion.nouveau()
        trois = self.gestion.nouveau()
        self.assertEqual((deux, trois), ("terminal-2", "terminal-3"))
        # Un nouveau terminal va sur la page de la console, qui se montre.
        self.assertEqual(self.gestion.mosaique.page, ["terminal-1", deux, trois])
        self.assertTrue(self.app.plan.visible("console"))
        self.gestion.pleine_page()
        for interne in self.gestion.noms_internes():
            self.attendre(lambda i=interne: self.lance(i))
        self.attendre(lambda: self.nom("terminal-1") == "bash — ~")
        # Mêmes shell et dossier : doublons numérotés dans l'ordre.
        self.attendre(lambda: [l[1] for l in self.gestion.lignes()]
                      == ["bash — ~", "bash — ~ (2)", "bash — ~ (3)"])
        # Une commande en cours donne son nom au terminal, à la liste et à l'en-tête de sa tuile.
        self.gestion.terminal(deux).executer("sleep 2")
        self.attendre(lambda: self.nom(deux) == "sleep 2")
        ligne = next(l for l in self.gestion.lignes() if l[0] == deux)
        self.assertTrue(ligne[2].startswith("en cours depuis"), ligne)
        self.attendre(lambda: self.gestion.terminal(deux).nom_affiche == "sleep 2")
        self.attendre(lambda: self.nom(deux) == "bash — ~ (2)")
        # Un cd change le nom ; un échec est signalé dans l'état.
        self.commande(trois, "mkdir -p projet && cd projet && false")
        self.attendre(lambda: self.nom(trois) == "bash — ~/projet")
        ligne = next(l for l in self.gestion.lignes() if l[0] == trois)
        self.assertEqual(ligne[2], "dernière commande : code 1")
        self.attendre(lambda: self.gestion.terminal(trois).etat_affiche == "code 1")
        # La place figure dans la liste.
        self.assertEqual(ligne[3], "page")

    def test_afficher_ranger_fermer(self):
        deux = self.gestion.nouveau()
        self.gestion.pleine_page()
        self.attendre(lambda: self.lance(deux))
        pid = self.gestion.terminal(deux).session.pid
        # Ranger : en arrière-plan, le shell continue.
        self.gestion.ranger(deux)
        self.assertEqual((self.gestion.mosaique.page, self.gestion.mosaique.reserve),
                         (["terminal-1"], [deux]))
        self.assertEqual(next(l for l in self.gestion.lignes() if l[0] == deux)[3], "arrière-plan")
        self.racine.update()
        self.assertFalse(self.gestion.console.tuile(deux).winfo_ismapped())
        self.assertTrue(_vivant(pid))
        # Afficher : de retour sur la page, la console montrée si elle était masquée.
        self.app.plan.masquer("console")
        self.gestion.afficher(deux)
        self.assertTrue(self.app.plan.visible("console"))
        self.assertEqual((self.gestion.mosaique.page, self.gestion.mosaique.actif),
                         (["terminal-1", deux], deux))
        self.assertEqual(self.gestion.terminal(deux).session.pid, pid)
        # Fermer un terminal inactif : pas de question ; tuile retirée, shell arrêté.
        questions = []
        self.app.demander = lambda titre, texte: questions.append(texte) or True
        self.gestion.fermer(deux)
        self.assertEqual(questions, [])
        self.assertNotIn(deux, self.gestion.noms_internes())
        self.assertNotIn(deux, self.gestion.mosaique.page + self.gestion.mosaique.reserve)
        self.attendre(lambda: not _vivant(pid))
        # Fermer un terminal dont une commande tourne : confirmation demandée.
        trois = self.gestion.nouveau()
        self.attendre(lambda: self.lance(trois))
        self.gestion.terminal(trois).executer("sleep 30")
        self.attendre(lambda: self.nom(trois) == "sleep 30")
        self.app.demander = lambda titre, texte: questions.append(texte) or False
        self.gestion.fermer(trois)
        self.assertEqual(len(questions), 1)
        self.assertIn("sleep 30", questions[0])
        self.assertIn(trois, self.gestion.noms_internes())
        self.app.demander = lambda titre, texte: True
        self.gestion.fermer(trois)
        self.assertNotIn(trois, self.gestion.noms_internes())

    def test_les_terminaux_sont_retrouves_a_la_reouverture(self):
        deux = self.gestion.nouveau()
        self.gestion.pleine_page()
        self.attendre(lambda: self.lance(deux))
        self.commande(deux, "mkdir -p ici && cd ici")
        self.gestion.ranger("terminal-1")
        self.app.fermer()
        self.ouvrir()
        self.assertEqual(self.gestion.noms_internes(), ["terminal-1", deux])
        self.assertEqual((self.gestion.mosaique.page, self.gestion.mosaique.reserve),
                         ([deux], ["terminal-1"]))
        self.attendre(lambda: self.gestion.terminal(deux).session is not None)
        # En arrière-plan : aucun shell n'est lancé tant qu'on ne le remet pas sur la page.
        self.assertIsNone(self.gestion.terminal("terminal-1").session)
        self.commande(deux, "pwd")
        self.assertIn(str(self.maison.resolve() / "ici"), self.gestion.terminal(deux).texte())


def _vivant(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDuPanneauDesActions(unittest.TestCase):
    """ACTIONS.md (D30 ; INV-UI-01) : menu par sujet, questions simples, sections à cocher, « Ce
    que ça va changer », « Et ensuite ? », réfuter et adopter une piste sans taper une option."""

    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.executer_args(["init"])

    def tearDown(self):
        self.app.fermer()
        self.dossier.cleanup()

    def cli(self, *args):
        from visionneur import interroger
        code, sortie, erreurs = interroger(EXECUTABLE, self.app.base.get(), list(args))
        self.assertEqual(code, 0, erreurs)
        return sortie

    def note(self, titre, genre="raisonnement"):
        f = self.app.ouvrir_action("nouvelle_note")
        f.remplir({"titre": titre, "genre": genre})
        self.assertEqual(f.faire(), 0)
        return f.objet

    def test_le_menu_par_sujet_et_le_mode_expert(self):
        # Contrat modifié (8.1b, APPARENCE.md, D31) : le menu n'est plus un arbre, mais le rail des
        # sujets et la liste des actions du sujet choisi ; la vérification reste la même : tous les
        # sujets, puis le mode expert, les actions d'un sujet, les 48 commandes du mode expert.
        from actions import GROUPES
        self.assertEqual(self.app.sujets(), list(GROUPES) + ["Toutes les commandes (mode expert)"])
        self.app.choisir_sujet("Écrire")
        titres = self.app.actions_affichees()
        self.assertIn("Nouvel échange avec une IA", titres)
        self.app.choisir_sujet("Toutes les commandes (mode expert)")
        # Contrat modifié (7.2a) : 48 commandes avec « code notes ».
        self.assertEqual(len(self.app.actions_affichees()), 48)
        # Choisir une action montre son titre, son aide et ses seules questions minimales.
        f = self.app.ouvrir_action("nouvel_echange")
        self.assertEqual(self.app.titre.cget("text"), "Nouvel échange avec une IA")
        self.assertEqual(f.libelles_visibles(), ["Titre de l’échange ?", "La réponse de l’IA (collez-la)"])
        f.cocher("question")
        self.assertIn("La question (collez-la)", f.libelles_visibles())

    def test_un_echange_en_collant_puis_et_ensuite(self):
        f = self.app.ouvrir_action("nouvel_echange")
        f.remplir({"titre": "Réveil", "reponse": "Le tick réveille la tâche.\nDeux lignes."})
        f.cocher("question")
        f.remplir({"question": "Pourquoi la tâche dort-elle ?"})
        self.assertEqual(f.faire(), 0)
        echange = f.objet
        self.assertEqual(self.cli("source", echange), "Le tick réveille la tâche.\nDeux lignes.")
        self.assertEqual(self.cli("source", echange, "--partie", "question"),
                         "Pourquoi la tâche dort-elle ?")
        libelles = [libelle for libelle, _ in f.suites()]
        self.assertEqual(libelles[0], "Citer un passage de cet échange")
        suite = f.suivre(0)
        self.assertEqual(suite.action.cle, "citer")
        self.assertEqual(suite.valeur("note"), echange)
        suite.remplir({"texte": "Le tick"})
        self.assertEqual(suite.faire(), 0)
        self.assertIn("\t0\t7\t", self.cli("citations", echange))

    def test_une_note_construite_en_cochant(self):
        theme = self.note("Scheduler", "categorie")
        appui = self.note("Primitive", "implementation")
        f = self.app.ouvrir_action("nouvelle_note")
        f.remplir({"titre": "Réveil perdu", "genre": "raisonnement"})
        f.cocher("theme")
        f.cocher("appuis")
        f.remplir({"theme": theme, "appuis": appui})
        # Une section remplie puis décochée ne compte pas.
        f.cocher("texte")
        f.remplir({"texte": "ignoré"})
        f.cocher("texte", False)
        self.assertEqual(f.faire(), 0)
        voisins = self.cli("voisinage", f.objet, "--tsv")
        self.assertIn(f"\tsortant\tdepend-de\tnote\t{appui}\t", voisins)
        self.assertIn(f"\tsortant\tclasse-dans\tcategorie\t{theme}\t", voisins)
        self.assertEqual(self.cli("source", f.objet), "")

    def test_refuter_montre_ce_que_ca_va_changer(self):
        idee = self.note("Hypothèse")
        dependante = self.note("Conséquence")
        r = self.app.ouvrir_action("relier")
        r.remplir({"origine": dependante, "lien": "depend-de", "destination": idee})
        self.assertEqual(r.faire(), 0)
        avant = self.cli("historique", "--nombre", "1")
        f = self.app.ouvrir_action("refuter")
        self.assertEqual(f.libelles_visibles(), ["Quelle note ?", "Pourquoi est-elle fausse ?"])
        f.remplir({"note": idee, "motif": "contre-exemple"})
        self.assertEqual(f.faire(), 4)
        self.assertIn("Ce que ça va changer", f.apercu)
        self.assertIn("nouvelle piste", f.apercu)
        self.assertIn("à revoir : Conséquence", f.apercu)
        self.assertEqual(self.cli("historique", "--nombre", "1"), avant)
        self.assertEqual(f.confirmer(), 0)
        self.assertIn("Réfutation de « Hypothèse »", self.cli("branches", "--tsv"))
        libelles = [libelle for libelle, _ in f.suites()]
        self.assertIn("Garder une note quand même", libelles)
        garder = f.suivre(libelles.index("Garder une note quand même"))
        self.assertEqual(garder.valeur("cause"), idee)

    def test_voir_une_note_ouvre_la_recherche_par_nom(self):
        """7.2 (RETROUVER.md, section 1) : les boutons du haut ouvrent les actions du panneau, pas
        une fenêtre qui demande un identifiant."""
        self.app.voir_une_note()
        self.assertEqual(self.app.action_courante.action.cle, "lire")
        self.app.voisinage_d_une_note()
        self.assertEqual(self.app.action_courante.action.cle, "voisinage")

    def test_suggestions_pendant_la_frappe_et_note_affichee_par_son_nom(self):
        idee = self.note("Réveil perdu")
        self.note("File d’attente")
        f = self.app.ouvrir_action("lire")
        champ = f.rangees["note"][1]
        f.variables["note"].set("rév")
        champ.proposer_maintenant()
        self.assertEqual(len(champ.suggestions), 1)
        self.assertTrue(champ.suggestions[0].startswith("Réveil perdu"))
        champ.choisir_suggestion(0)
        self.assertEqual(f.valeur("note"), f"Réveil perdu · {idee[:8]}")
        self.assertEqual(f.reponse("note"), idee)
        self.assertEqual(f.faire(), 0)
        # Champ à plusieurs notes : la suggestion porte sur la dernière.
        n = self.app.ouvrir_action("nouvelle_note")
        n.cocher("appuis")
        champ = n.rangees["appuis"][1]
        n.variables["appuis"].set(f"Réveil perdu · {idee[:8]}, fil")
        champ.proposer_maintenant()
        champ.choisir_suggestion(0)
        self.assertTrue(n.valeur("appuis").startswith(f"Réveil perdu · {idee[:8]}, File d’attente · "))
        self.assertEqual(n.reponse("appuis").split(",")[0], idee)
        # Peu de lettres : pas de suggestion.
        f = self.app.ouvrir_action("lire")
        f.variables["note"].set("r")
        f.rangees["note"][1].proposer_maintenant()
        self.assertEqual(f.rangees["note"][1].suggestions, [])

    def test_quelles_notes_concernent_ce_code(self):
        idee = self.note("Réveil perdu")
        fichier = Path(self.dossier.name) / "src" / "sched.c"
        fichier.parent.mkdir()
        fichier.write_text(f"// @design: {idee}\nvoid wake(void) {{}}\n", encoding="utf-8")
        self.cli("ancrer", idee, "--fichier", str(fichier), "--lignes", "2-2")
        f = self.app.ouvrir_action("notes_du_code")
        f.remplir({"fichier": str(fichier)})
        f.cocher("ligne")
        f.remplir({"ligne": "2"})
        self.assertEqual(f.faire(), 0)
        self.assertEqual(f.notes_trouvees, [(idee, "Réveil perdu")])
        vue = f.lire_note(0)
        self.assertEqual(vue.note, idee)

    def test_une_piste_choisie_s_affiche_par_son_nom(self):
        """Ajouté après la relecture d'une capture d'écran (7.1c) : une piste remplie par
        programme (suite « Et ensuite ? ») s'affiche par son nom, et non par un champ vide."""
        idee = self.note("File d’attente")
        d = self.app.ouvrir_action("decider")
        d.remplir({"note": idee, "decision": "plus-retenue", "motif": "trop lente"})
        d.faire()
        d.confirmer()
        piste = next(l.split("\t")[0] for l in self.cli("branches", "--tsv").splitlines()
                     if "Mise de côté de" in l)
        f = self.app.ouvrir_action("adopter_piste", {"piste": piste})
        self.assertEqual(f.rangees["piste"][1].get(), "★ Mise de côté de « File d’attente »")
        self.assertEqual(f.valeur("piste"), piste)

    def test_adopter_une_piste_en_tranchant_un_desaccord(self):
        idee = self.note("File d’attente")
        d = self.app.ouvrir_action("decider")
        d.remplir({"note": idee, "decision": "plus-retenue", "motif": "trop lente"})
        self.assertEqual(d.faire(), 4)
        self.assertEqual(d.confirmer(), 0)
        piste = next(l.split("\t")[0] for l in self.cli("branches", "--tsv").splitlines()
                     if "Mise de côté de" in l)
        self.cli("basculer", "00000000")
        self.cli("decision", idee, "retenue", "--motif", "gardée")
        f = self.app.ouvrir_action("adopter_piste")
        noms = [nom for nom, _ in f.choix_de("piste")]
        self.assertIn("Mise de côté de « File d’attente »", noms)
        f.remplir({"piste": piste})
        f.voir_le_plan()
        self.assertIn("deviendront la version retenue de la piste principale", f.resume_du_plan)
        self.assertEqual(len(f.desaccords), 1)
        self.assertIn("décision", f.desaccords[0])
        self.assertIn("retenue", f.desaccords[0])
        self.assertIn("mise de côté", f.desaccords[0])
        f.trancher(1, "piste")
        code = f.faire()
        if code == 4:
            code = f.confirmer()
        self.assertEqual(code, 0)
        self.assertEqual(self.cli("fusion", "plan", piste, "--tsv"), "a-jour\toui\n")
        # L'idée mise de côté dans la piste adoptée n'est plus en vigueur sur la principale.
        en_vigueur = self.cli("statut").split("À réexaminer")[0]
        self.assertNotIn("File d’attente", en_vigueur)


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDeLHabillage(unittest.TestCase):
    """APPARENCE.md (8.1b, D31 ; INV-UI-01, INV-LOOK-01) : le thème, le rail et la liste des
    actions, la barre d'état relue après chaque commande, la recherche des actions et des notes."""

    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.base = str(Path(self.dossier.name) / "rtos" / ".memoire" / "depot.db")
        Path(self.base).parent.mkdir(parents=True)
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, self.base)
        self.app.executer_args(["init"])

    def tearDown(self):
        self.app.fermer()
        self.dossier.cleanup()

    def cli(self, *args):
        from visionneur import interroger
        code, sortie, erreurs = interroger(EXECUTABLE, self.base, list(args))
        self.assertEqual(code, 0, erreurs)
        return sortie

    def note(self, titre):
        sortie = self.cli("note", "creer", "--titre", titre, "--role", "raisonnement")
        return sortie.split("Note créée : ")[1].split()[0]

    def test_le_theme_est_applique(self):
        from tkinter import ttk
        from theme import ZoneDeTexte
        self.assertEqual(ttk.Style(self.racine).theme_use(), "memoire")
        self.assertIsInstance(self.app.sortie, ZoneDeTexte)
        f = self.app.ouvrir_action("nouvelle_note")
        self.assertEqual(str(f.bouton_faire.cget("style")), "Action.TButton")
        self.assertEqual(self.app.nom_du_projet(), "rtos")

    def test_chaque_action_s_ouvre_par_le_rail_et_la_liste(self):
        from actions import ACTIONS, GROUPES
        for sujet in GROUPES:
            self.app.choisir_sujet(sujet)
            self.assertEqual(self.app.sujet_courant, sujet)
            attendues = [a.titre for a in ACTIONS if a.groupe == sujet]
            self.assertEqual(self.app.actions_affichees(), attendues)
        self.app.choisir_sujet("Relier")
        self.app.choisir_dans_la_liste(0)
        self.assertEqual(self.app.titre.cget("text"), "Relier deux notes")
        self.assertEqual(self.app.action_courante.action.cle, "relier")
        self.app.choisir_sujet("Toutes les commandes (mode expert)")
        rang = self.app.actions_affichees().index("statut")
        self.app.choisir_dans_la_liste(rang)
        self.assertEqual(self.app.courante.nom, "statut")
        self.assertIn("Branche active", self.app.sortie.get("1.0", "end"))

    def test_la_barre_d_etat_suit_le_projet(self):
        self.app.rafraichir_etat()
        self.assertEqual(self.app.etat.piste, "piste principale")
        self.assertIn("piste principale", self.app.textes_de_la_barre())
        self.assertIn("rien à revoir", self.app.textes_de_la_barre())
        idee = self.note("Hypothèse ISR")
        self.cli("note", "creer", "--titre", "Conséquence", "--role", "raisonnement",
                 "--depend-de", idee)
        f = self.app.ouvrir_action("refuter")
        f.remplir({"note": idee, "motif": "contre-exemple"})
        self.assertEqual(f.faire(), 4)
        self.assertEqual(f.confirmer(), 0)
        # Relue après la commande, sans autre geste.
        self.racine.update_idletasks()
        self.assertEqual(self.app.etat.piste, "Réfutation de « Hypothèse ISR »")
        self.assertEqual(self.app.etat.a_revoir, 1)
        self.assertIn("1 note à revoir", self.app.textes_de_la_barre())

    def test_la_recherche_trouve_les_actions_et_les_notes(self):
        idee = self.note("Réveil perdu")
        recherche = self.app.recherche
        recherche.ouvrir()
        self.assertTrue(recherche.visible())
        recherche.chercher("echange")
        libelles = [libelle for _genre, libelle, _cible in recherche.resultats]
        self.assertIn("Nouvel échange avec une IA", libelles)
        recherche.choisir(libelles.index("Nouvel échange avec une IA"))
        self.assertFalse(recherche.visible())
        self.assertEqual(self.app.action_courante.action.cle, "nouvel_echange")
        recherche.ouvrir()
        recherche.chercher("rév")
        notes = [(libelle, cible) for genre, libelle, cible in recherche.resultats if genre == "note"]
        self.assertEqual(len(notes), 1)
        self.assertTrue(notes[0][0].startswith("Réveil perdu"))
        vue = recherche.choisir([g for g, _l, _c in recherche.resultats].index("note"))
        self.assertEqual(vue.note, idee)
        recherche.ouvrir()
        recherche.chercher("")
        self.assertEqual(recherche.resultats, [])
        recherche.fermer()
        self.assertFalse(recherche.visible())

    def test_le_survol_montre_son_fond_et_ne_s_accumule_pas(self):
        """Ajouté à la relecture de 8.1b : au survol d'un sujet du rail ou d'une pastille de la
        barre d'état, son fond de survol apparaît et le pointeur devient une main ; chaque
        commande redessine le fond, et ces liaisons sont remplacées, jamais ajoutées."""
        canevas = self.app.habillage.canevas
        self.app.habillage.dessiner()
        self.racine.update()
        avant = {tag: canevas.tag_bind(tag, "<Enter>").count("\n") for tag in ("sujet-1", "segment-0")}
        for _ in range(3):
            self.app.rafraichir_etat()
        self.racine.update()
        for tag, lignes in avant.items():
            with self.subTest(tag=tag):
                self.assertEqual(canevas.tag_bind(tag, "<Enter>").count("\n"), lignes)
                fond = canevas.find_withtag(tag)[0]
                self.assertEqual(canevas.itemcget(fond, "state"), "hidden")
                x0, y0, x1, y1 = canevas.bbox(tag)
                canevas.event_generate("<Motion>", x=(x0 + x1) // 2, y=(y0 + y1) // 2)
                self.racine.update()
                self.assertEqual(canevas.itemcget(fond, "state"), "normal")
                self.assertEqual(str(canevas.cget("cursor")), "hand2")
                canevas.event_generate("<Motion>", x=2, y=2)
                self.racine.update()
                self.assertEqual(canevas.itemcget(fond, "state"), "hidden")

    def test_le_journal_se_redimensionne_et_s_agrandit(self):
        """8.1c (APPARENCE.md, section 3) : la séparation entre l'action et le journal se tire ;
        « Agrandir » donne au journal la plus grande part, « Réduire » rend la taille d'avant."""
        self.racine.geometry("1400x860")
        self.racine.update()
        colonne = self.app.colonne
        hauteur = colonne.winfo_height()
        colonne.sashpos(0, hauteur - 250)
        self.racine.update()
        self.assertAlmostEqual(colonne.sashpos(0), hauteur - 250, delta=2)
        self.assertEqual(str(self.app.bouton_journal.cget("text")), "Agrandir")
        self.app.basculer_journal()
        self.racine.update()
        self.assertGreater(hauteur - colonne.sashpos(0), hauteur * 0.6)
        self.assertEqual(str(self.app.bouton_journal.cget("text")), "Réduire")
        self.app.basculer_journal()
        self.racine.update()
        self.assertAlmostEqual(colonne.sashpos(0), hauteur - 250, delta=2)
        self.assertEqual(str(self.app.bouton_journal.cget("text")), "Agrandir")

    def test_les_grandes_surfaces_ne_sont_pas_pavees(self):
        """8.5b (retour de l'utilisateur : interface saccadée). ttk remplit un élément image en
        répétant le milieu de son image : un milieu de 6 pixels demandait des milliers de copies à
        chaque redessin d'un grand cadre (100 ms par pas d'un redimensionnement, mesuré). Les
        cadres et les champs ont un milieu large."""
        formes = self.app.theme._formes_des_elements
        minimum = {"Champ.cadre": (100, 100), "Champ.actif": (100, 100), "Sombre.cadre": (100, 100),
                   "Carte.bordure": (100, 100), "Champ.champ": (100, 2), "Sombre.champ": (100, 2)}
        for nom, (large, haut_) in minimum.items():
            formes_, rayon, _marge, _sticky = formes[nom]
            image = formes_[0][1].image
            gauche, haut, droite, bas = formes_[0][1].marges
            coin = int(rayon) + 1
            self.assertGreaterEqual(image.largeur - gauche - droite - 2 * coin, large, nom)
            self.assertGreaterEqual(image.hauteur - haut - bas - 2 * coin, haut_, nom)

    def test_les_images_dessinees_sont_opaques(self):
        """Optimisation (8.1c, APPARENCE.md, section 5) : Tk mélange une image à transparence à
        chaque dessin ; les éléments du thème et le fond de la fenêtre sont donc aplatis sur leur
        surface. Les coins, partie la plus transparente d'une forme arrondie, sont opaques."""
        self.racine.update()
        images = list(self.app.theme.images_d_elements) + [self.app.habillage.travail]
        self.assertGreater(len(images), 30)
        for image in images:
            l, h = image.width(), image.height()
            for x, y in ((0, 0), (l - 1, 0), (0, h - 1), (l - 1, h - 1), (l // 2, h // 2)):
                with self.subTest(image=str(image), x=x, y=y):
                    self.assertFalse(image.transparency_get(x, y))

    def test_une_grande_forme_etiree_est_celle_du_calcul_complet(self):
        """Optimisation (8.1c) : au redimensionnement, la carte, la plaque et le rail ne sont plus
        calculés pixel par pixel ; une petite forme est étirée par Tk (coins gardés, bords et
        milieu répétés). Le résultat est celui du calcul complet, à une unité près par canal."""
        import dessin
        from dessin import rgba
        from habillage import forme_etiree
        from theme import photo
        cas = [dict(rayon=28, fond=rgba("#FFEDE2"), bord=rgba("#FFFFFF", .9),
                    ombre=(0, 18, 26, rgba("#3C1E6E", .30))),
               dict(rayon=26, fond=rgba("#FFF6F0", .6), bord=rgba("#FFFFFF", .7),
                    ombre=(5, 7, 4, rgba("#241F2E", .22)))]
        for options in cas:
            with self.subTest(options=options):
                rayon = options.pop("rayon")
                etiree, image = forme_etiree(self.racine, 300, 180, rayon, **options)
                complete = dessin.forme(300, 180, rayon, **options)
                attendue = photo(self.racine, complete)
                self.assertEqual((etiree.width(), etiree.height()),
                                 (complete.largeur, complete.hauteur))
                self.assertEqual((image.x, image.y), (complete.x, complete.y))
                for x in range(0, complete.largeur, 7):
                    for y in range(0, complete.hauteur, 5):
                        a, b = etiree.get(x, y), attendue.get(x, y)
                        self.assertTrue(all(abs(u - v) <= 1 for u, v in zip(a, b)), (x, y, a, b))
                        self.assertEqual(etiree.transparency_get(x, y),
                                         attendue.transparency_get(x, y))


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDesMenus(unittest.TestCase):
    """APPARENCE.md, section 9.3 (8.2a) : menus, listes déroulantes et suggestions dans une même
    carte arrondie ; même interface que tk.Menu pour le code existant ; clavier ; image opaque."""

    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.executer_args(["init"])
        self.racine.update()

    def tearDown(self):
        self.app.fermer()
        self.dossier.cleanup()

    def test_un_menu_offre_l_interface_de_tk_menu(self):
        import menus
        appels = []
        case = tk.BooleanVar(value=False)
        menu = menus.Menu(self.racine, tearoff=False)
        self.assertIsNone(menu.index("end"))
        menu.add_command(label="Changer de projet…", command=lambda: appels.append("projet"))
        menu.add_separator()
        menu.add_checkbutton(label="Toujours au premier plan", variable=case,
                             command=lambda: appels.append("case"))
        menu.add_command(label="Inactif", state="disabled", command=lambda: appels.append("non"))
        self.assertEqual(menu.index("end"), 3)
        self.assertEqual([menu.type(i) for i in range(4)],
                         ["command", "separator", "checkbutton", "command"])
        self.assertEqual(menu.entrycget(0, "label"), "Changer de projet…")
        menu.tk_popup(self.racine.winfo_rootx() + 300, self.racine.winfo_rooty() + 200)
        self.racine.update()
        self.assertTrue(menu.liste.visible())
        self.assertEqual(menu.liste.libelles(), ["Changer de projet…", "Toujours au premier plan",
                                                 "Inactif"])
        # Clavier : ↓ saute le séparateur, Entrée choisit et ferme.
        menu.liste.deplacer(1)
        menu.liste.deplacer(1)
        menu.liste.valider()
        self.racine.update()
        self.assertFalse(menu.liste.visible())
        self.assertTrue(case.get())
        self.assertEqual(appels, ["case"])
        # Une entrée inactive ne se choisit pas.
        menu.tk_popup(self.racine.winfo_rootx() + 300, self.racine.winfo_rooty() + 200)
        menu.liste.choisir(2)
        self.assertEqual(appels, ["case"])
        menu.liste.fermer()
        menu.delete(0, "end")
        self.assertIsNone(menu.index("end"))

    def test_un_seul_menu_ouvert_a_la_fois_et_un_clic_ailleurs_le_ferme(self):
        """8.4b : ouvrir un menu ferme celui qui était ouvert ; un clic ailleurs dans la fenêtre
        ferme le menu ouvert, et ce clic agit (il n'est pas avalé)."""
        import menus
        un, deux = menus.Menu(self.racine), menus.Menu(self.racine)
        un.add_command(label="Un")
        deux.add_command(label="Deux")
        x, y = self.racine.winfo_rootx() + 300, self.racine.winfo_rooty() + 200
        un.tk_popup(x, y)
        self.racine.update()
        deux.tk_popup(x + 200, y)
        self.racine.update()
        self.assertFalse(un.liste.visible())
        self.assertTrue(deux.liste.visible())
        self.assertEqual(self.racine.grab_current(), None)
        canevas = self.app.habillage.canevas
        canevas.event_generate("<ButtonPress-1>", x=5, y=5)
        self.racine.update()
        self.assertFalse(deux.liste.visible())

    def test_la_carte_reste_dans_la_fenetre_et_son_image_est_opaque(self):
        import menus
        menu = menus.Menu(self.racine)
        for rang in range(6):
            menu.add_command(label=f"Entrée {rang}")
        # Ouverte tout en bas à droite : elle se décale pour rester visible.
        menu.tk_popup(self.racine.winfo_rootx() + self.racine.winfo_width() - 5,
                      self.racine.winfo_rooty() + self.racine.winfo_height() - 5)
        self.racine.update()
        carte = menu.liste.canevas
        self.assertLessEqual(carte.winfo_x() + carte.winfo_width(), self.racine.winfo_width())
        self.assertLessEqual(carte.winfo_y() + carte.winfo_height(), self.racine.winfo_height())
        image = menu.liste.image_de_fond
        l, h = image.width(), image.height()
        for x, y in ((0, 0), (l - 1, 0), (0, h - 1), (l - 1, h - 1), (l // 2, h // 2)):
            self.assertFalse(image.transparency_get(x, y))
        menu.liste.fermer()

    def test_une_liste_deroulante_ouvre_la_carte(self):
        from tkinter import ttk
        import menus
        choisis = []
        liste = ttk.Combobox(self.app.formulaire, values=["un", "deux", "trois"], state="readonly")
        liste.set("un")
        liste.pack()
        liste.bind("<<ComboboxSelected>>", lambda _e: choisis.append(liste.get()))
        self.racine.update()
        # Le même chemin que le clic sur la flèche ou ↓ : ttk::combobox::Post.
        self.racine.tk.call("ttk::combobox::Post", liste)
        self.racine.update()
        carte = menus.liste_de(liste)
        self.assertTrue(carte.visible())
        self.assertEqual(carte.libelles(), ["un", "deux", "trois"])
        self.assertFalse(self.racine.tk.call("winfo", "exists", f"{liste}.popdown"))
        carte.choisir(1)
        self.racine.update()
        self.assertEqual(liste.get(), "deux")
        self.assertEqual(choisis, ["deux"])
        self.assertFalse(carte.visible())

    def test_les_suggestions_de_notes_sont_dans_la_carte(self):
        sortie = self.app.executer_args(["note", "creer", "--titre", "Réveil perdu", "--role",
                                         "raisonnement"])
        self.assertEqual(sortie, 0)
        f = self.app.ouvrir_action("lire")
        champ = f.rangees["note"][1]
        self.racine.update()
        f.variables["note"].set("rév")
        champ.proposer_maintenant()
        self.racine.update()
        self.assertTrue(champ.carte.visible())
        self.assertTrue(champ.carte.libelles()[0].startswith("Réveil perdu"))
        champ.carte.choisir(0)
        self.assertTrue(f.valeur("note").startswith("Réveil perdu · "))
        self.assertFalse(champ.carte.visible())

    def test_le_bouton_du_bandeau_ouvre_son_menu(self):
        # Contrat modifié (8.2b) : le bandeau d'un panneau tient sur une rangée ; « Plan », « Zone »,
        # monter, descendre et masquer sont dans le menu « ⋯ ». Les entrées de l'ordre d'une
        # fenêtre ne sont actives que pour un panneau flottant (PLAN-DE-TRAVAIL.md, section 10).
        # Contrat modifié (8.5, D32) : il n'y a plus de panneau par terminal, et la console n'a pas
        # de bandeau (son « ⋯ » est dans sa rangée, EssaiDeLaConsole) ; l'essai porte sur le
        # panneau « Aperçu ».
        plan = self.app.plan
        plan.montrer("apercu")
        self.racine.update()
        panneau = plan.panneaux["apercu"]
        panneau.bouton_menu.invoke()
        self.racine.update()
        self.assertTrue(panneau.menu.liste.visible())
        libelles = panneau.menu.liste.libelles()
        for libelle in ("Mettre devant", "Ancrer à gauche", "Monter", "Masquer"):
            self.assertIn(libelle, libelles)
        panneau.menu.liste.fermer()
        self.assertEqual(plan.etat_du_menu_plan("apercu"), "disabled")
        plan.detacher("apercu")
        self.racine.update()
        self.assertEqual(plan.etat_du_menu_plan("apercu"), "normal")
        plan.ancrer("apercu")


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDeLApercu(unittest.TestCase):
    """APPARENCE.md, section 9.3 (8.2b) : panneau « Aperçu » : ce que ça va changer, suivi pendant
    la saisie, puis l'aperçu du moteur pour une opération critique ; les dernières étapes."""

    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.executer_args(["init"])
        self.racine.update()

    def tearDown(self):
        self.app.fermer()
        self.dossier.cleanup()

    def cli(self, *args):
        from visionneur import interroger
        code, sortie, erreurs = interroger(EXECUTABLE, self.app.base.get(), list(args))
        self.assertEqual(code, 0, erreurs)
        return sortie

    def test_le_panneau_est_visible_au_premier_lancement(self):
        self.assertTrue(self.app.plan.visible("apercu"))
        self.assertEqual(self.app.plan.disposition.etat("apercu").zone, "droite")

    def test_ce_que_ca_va_changer_est_dans_un_encadre_clair(self):
        """Comme la maquette : un encadré clair (#FFF8F4) à liseré fin, posé sur le papier ;
        son contenu (textes, pastilles des signes) est sur ce même fond."""
        from apparence import COULEURS
        apercu = self.app.apercu
        self.assertEqual(str(apercu.carte.cget("style")), "Encadre.TFrame")
        self.assertEqual(apercu.changements.cget("background").upper(), COULEURS["champ"])
        from tkinter import ttk
        style = ttk.Style(self.racine)
        for nom in ("Encadre.TLabel", "EncadreEtiquette.TLabel", "EncadreSecond.TLabel"):
            self.assertEqual(style.lookup(nom, "background").upper(), COULEURS["champ"])

    def test_ce_que_ca_va_changer_suit_la_saisie(self):
        appui = self.cli("note", "creer", "--titre", "Hypothèse ISR", "--role",
                         "raisonnement").split("Note créée : ")[1].split()[0]
        f = self.app.ouvrir_action("nouvelle_note")
        f.remplir({"titre": "Réveil", "genre": "raisonnement"})
        f.cocher("appuis")
        f.remplir({"appuis": appui})
        self.racine.update_idletasks()
        lignes = self.app.apercu.lignes
        self.assertEqual(lignes[0], ("+", "idée créée", "Réveil"))
        self.assertEqual(lignes[1][:2], ("→", "s’appuie sur"))
        self.assertIn("prévu", self.app.apercu.etat_du_texte())
        self.assertEqual(f.faire(), 0)
        self.racine.update_idletasks()
        self.assertIn("fait", self.app.apercu.etat_du_texte())

    def test_l_apercu_du_moteur_pour_une_operation_critique(self):
        idee = self.cli("note", "creer", "--titre", "Hypothèse", "--role",
                        "raisonnement").split("Note créée : ")[1].split()[0]
        self.cli("note", "creer", "--titre", "Conséquence", "--role", "raisonnement",
                 "--depend-de", idee)
        f = self.app.ouvrir_action("refuter")
        f.remplir({"note": idee, "motif": "contre-exemple"})
        self.assertEqual(f.faire(), 4)
        self.racine.update_idletasks()
        self.assertIn("à revoir : Conséquence", self.app.apercu.texte_du_moteur)
        self.assertEqual(f.confirmer(), 0)

    def test_les_dernieres_etapes_et_revenir_a_l_une(self):
        self.cli("note", "creer", "--titre", "Hypothèse", "--role", "raisonnement")
        self.app.executer_args(["statut"])
        self.racine.update_idletasks()
        etapes = self.app.apercu.etapes
        self.assertEqual(etapes[0].commande, "creer-note")
        self.assertEqual(self.app.apercu.libelles_des_etapes()[0], "Nouvelle note")
        f = self.app.apercu.choisir_etape(1)
        self.assertEqual(f.action.cle, "revenir_a_une_etape")
        self.assertEqual(f.valeur("etape"), etapes[1].identifiant)


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file(),
                     "Tkinter, affichage ou exécutable memoire absent")
class EssaiDeLaLecture(unittest.TestCase):
    """APPARENCE.md, section 9.3 (8.2c) : la lecture dans la carte, son en-tête, la barre de
    sélection, la carte de citation, le panneau Voisinage qui suit la note lue."""

    def setUp(self):
        from memoire_gui import Application
        self.dossier = tempfile.TemporaryDirectory()
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.executer_args(["init"])
        self.racine.update()

    def tearDown(self):
        self.app.fermer()
        self.dossier.cleanup()

    def cli(self, *args):
        from visionneur import interroger
        code, sortie, erreurs = interroger(EXECUTABLE, self.app.base.get(), list(args))
        self.assertEqual(code, 0, erreurs)
        return sortie

    def echange(self):
        dossier = Path(self.dossier.name)
        (dossier / "q.txt").write_text("Que fait le driver ?", encoding="utf-8")
        (dossier / "r.txt").write_text("Le driver connaît la source ; le noyau connaît les "
                                       "destinataires.", encoding="utf-8")
        return self.cli("echange", "coller", "--titre", "Rôle du driver", "--question-fichier",
                        str(dossier / "q.txt"), "--reponse-fichier",
                        str(dossier / "r.txt")).split("Échange créé : ")[1].split()[0]

    def selectionner(self, zone, texte):
        affiche = zone.texte.get("1.0", "end-1c")
        debut = affiche.index(texte)
        zone.texte.tag_remove("sel", "1.0", "end")
        zone.texte.tag_add("sel", f"1.0 + {debut} chars", f"1.0 + {debut + len(texte)} chars")
        zone.texte.event_generate("<<Selection>>")
        self.racine.update()

    def test_la_lecture_s_ouvre_dans_la_carte_et_l_action_revient(self):
        echange = self.echange()
        f = self.app.ouvrir_action("nouvelle_note")
        vue = self.app.voir(echange)
        self.racine.update()
        self.assertIs(vue, self.app.lecture)
        self.assertEqual(self.app.vue_de_la_carte, "lecture")
        self.assertEqual(self.app.sujet_courant, "Lire")
        self.assertEqual(vue.genre.cget("text"), "Échange avec une IA")
        self.assertEqual(vue.titre_de_la_note.cget("text"), "Rôle du driver")
        self.assertEqual([z.partie for z in vue.zones], ["question", "reponse"])
        self.app.retour_a_l_action()
        self.racine.update()
        self.assertEqual(self.app.vue_de_la_carte, "action")
        self.assertIs(self.app.action_courante, f)
        # Ouvrir une autre action ramène aussi à la vue des actions.
        self.app.voir(echange)
        self.app.ouvrir_action("refuter")
        self.assertEqual(self.app.vue_de_la_carte, "action")

    def test_la_barre_de_selection_relie_et_cite_en_un_geste(self):
        appui = self.cli("note", "creer", "--titre", "Hypothèse", "--role",
                         "raisonnement").split("Note créée : ")[1].split()[0]
        echange = self.echange()
        vue = self.app.voir(echange)
        _, reponse = vue.zones
        self.assertFalse(vue.barre.visible())
        self.selectionner(reponse, "connaît la source")
        self.assertTrue(vue.barre.visible())
        # Contrat modifié (8.3, APPARENCE.md, section 10) : la barre tient en une pastille au bas
        # de l'article ; « Classe dans… », « Lien… » et « Doute à suivre… » sont sous « ⋯ ».
        self.assertEqual(vue.barre.libelles(), ["Citer la sélection", "Soutient…", "Inspire…",
                                                "Nouvelle note…", "⋯"])
        self.assertEqual(vue.barre.menu_autres.liste.libelles() if vue.barre.menu_autres.liste
                         .visible() else [e["label"] for e in vue.barre.menu_autres.entrees],
                         ["Classe dans…", "Lien…", "Doute à suivre…"])
        self.assertEqual(vue.barre.cadre.winfo_manager(), "pack")
        vue.barre.ouvrir("--soutient")
        self.assertTrue(vue.relier_visible())
        vue.liens["--soutient"].set(f"Hypothèse · {appui[:8]}")
        self.assertEqual(vue.citer_selection(), 0)
        self.assertIn(f"({appui} depend-de ", self.app.sortie.get("1.0", "end"))
        self.assertFalse(vue.relier_visible())

    def test_la_carte_de_citation(self):
        echange = self.echange()
        self.cli("citer", echange, "--texte", "le noyau")
        vue = self.app.voir(echange)
        _, reponse = vue.zones
        affiche = reponse.texte.get("1.0", "end-1c")
        vue.montrer_citation(reponse, f"1.0 + {affiche.index('le noyau') + 2} chars")
        self.racine.update()
        self.assertTrue(vue.carte_citation.visible())
        textes = " ".join(vue.carte_citation.textes())
        self.assertIn("Passage cité", textes)
        self.assertIn("à jour", textes)
        self.assertIn("Citation ", vue.etat.cget("text"))
        vue.carte_citation.fermer()
        self.assertFalse(vue.carte_citation.visible())

    def test_le_panneau_voisinage_suit_la_note_lue(self):
        appui = self.cli("note", "creer", "--titre", "Hypothèse", "--role",
                         "raisonnement").split("Note créée : ")[1].split()[0]
        note = self.cli("note", "creer", "--titre", "Conséquence", "--role", "raisonnement",
                        "--depend-de", appui).split("Note créée : ")[1].split()[0]
        self.app.voir(note)
        self.racine.update()
        plan = self.app.plan
        self.assertTrue(plan.visible("voisinage"))
        panneau = plan.contenu("voisinage")
        self.assertEqual(panneau.note, note)
        self.assertEqual(len(panneau.dessin.find_withtag("voisin")), 1)
        panneau.ouvrir(appui)
        self.racine.update()
        self.assertEqual(self.app.lecture.note, appui)
        self.assertEqual(panneau.note, appui)
        fenetre = panneau.agrandir()
        self.assertEqual(fenetre.note, appui)

    def test_les_deux_vues_et_leurs_panneaux(self):
        """8.3 (APPARENCE.md, section 10) : la lecture retire la liste des actions et montre le
        Voisinage à la place de l'Aperçu ; le retour à l'action rétablit l'inverse ; la
        disposition enregistrée est celle de la vue des actions."""
        echange = self.echange()
        plan = self.app.plan
        self.assertTrue(plan.visible("apercu"))
        self.app.voir(echange)
        self.racine.update()
        self.assertFalse(self.app.liste.canevas.winfo_ismapped())
        self.assertFalse(plan.visible("apercu"))
        self.assertTrue(plan.visible("voisinage"))
        self.app.retour_a_l_action()
        self.racine.update()
        self.assertTrue(self.app.liste.canevas.winfo_ismapped())
        self.assertTrue(plan.visible("apercu"))
        self.assertFalse(plan.visible("voisinage"))
        # Masqué par l'utilisateur pendant la lecture : il reste masqué à la lecture suivante.
        self.app.voir(echange)
        plan.masquer("voisinage")
        self.app.retour_a_l_action()
        self.app.voir(echange)
        self.assertFalse(plan.visible("voisinage"))
        # Fermer pendant la lecture enregistre la disposition de la vue des actions.
        import json
        self.app.fermer()
        enregistree = json.loads(Path(plan.chemin).read_text(encoding="utf-8"))
        self.assertEqual(enregistree["panneaux"]["apercu"]["etat"], "ancre")

    def test_la_carte_du_voisinage_et_la_signature(self):
        appui = self.cli("note", "creer", "--titre", "Traitement différé des interruptions",
                         "--role", "raisonnement").split("Note créée : ")[1].split()[0]
        note = self.cli("note", "creer", "--titre", "Le driver s’arrête à la source", "--role",
                        "raisonnement", "--depend-de", appui).split("Note créée : ")[1].split()[0]
        self.app.voir(note)
        self.racine.update()
        panneau = self.app.plan.contenu("voisinage")
        self.assertEqual(str(panneau.carte.cget("style")), "Encadre.TFrame")
        # Les libellés ne sont plus tronqués : ils passent sur deux lignes.
        textes = [panneau.dessin.itemcget(i, "text") for i in panneau.dessin.find_withtag("voisin")]
        self.assertEqual(textes, ["Traitement différé des interruptions"])
        self.assertIn("Lire › Le driver s’arrête à la source",
                      self.app.habillage.texte_de_la_recherche())
        self.app.retour_a_l_action()
        self.racine.update()
        self.assertIn("Chercher", self.app.habillage.texte_de_la_recherche())


class _AvecUnDossierPersonnel(unittest.TestCase):
    """Application ouverte dans un dossier personnel d'essai (bash, configuration à part)."""

    def setUp(self):
        self.dossier = tempfile.TemporaryDirectory()
        self.maison = Path(self.dossier.name) / "maison"
        (self.maison / "Documents").mkdir(parents=True)
        (self.maison / "Docker").mkdir()
        (self.maison / ".bashrc").write_text("PS1='$ '\n", encoding="utf-8")
        self.anciens = {k: os.environ.get(k) for k in ("HOME", "SHELL", "XDG_CONFIG_HOME")}
        os.environ.update(HOME=str(self.maison), SHELL=shutil.which("bash"),
                          XDG_CONFIG_HOME=str(Path(self.dossier.name) / "config"))
        self.ancien_dossier = os.getcwd()
        os.chdir(self.maison)
        self.ouvrir()

    def ouvrir(self):
        from memoire_gui import Application
        self.racine = tk.Tk()
        self.app = Application(self.racine, EXECUTABLE, str(Path(self.dossier.name) / "depot.db"))
        self.app.notifier = lambda titre, corps: True
        self.gestion = self.app.terminaux
        self.console = self.gestion.console
        self.plan = self.app.plan

    def tearDown(self):
        self.app.fermer()
        os.chdir(self.ancien_dossier)
        for cle, valeur in self.anciens.items():
            if valeur is None:
                os.environ.pop(cle, None)
            else:
                os.environ[cle] = valeur
        self.dossier.cleanup()

    def attendre(self, condition, delai=10.0):
        limite = time.monotonic() + delai
        while not condition():
            if time.monotonic() > limite:
                self.fail(f"délai dépassé ; lignes : {self.gestion.lignes()!r}")
            self.racine.update()
            time.sleep(0.02)

    def place(self):
        return self.plan.disposition.etat("console").etat

    def lance(self, interne):
        terminal = self.gestion.terminal(interne)
        return terminal.session is not None and terminal.tampon.dossier is not None

    def pid(self, interne):
        self.attendre(lambda: self.lance(interne))
        return self.gestion.terminal(interne).session.pid

    def en_pleine_page(self, nombre):
        """La console en pleine page avec `nombre` terminaux ; rend leurs noms internes."""
        self.app.basculer_terminal()
        self.gestion.pleine_page()
        noms = ["terminal-1"] + [self.gestion.nouveau() for _ in range(nombre - 1)]
        self.racine.update()
        return noms

    def geometrie(self, interne):
        tuile = self.console.tuile(interne)
        return (tuile.winfo_x(), tuile.winfo_y(), tuile.winfo_width(), tuile.winfo_height())


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file()
                     and shutil.which("bash"), "Tkinter, affichage, exécutable memoire ou bash absent")
class EssaiDeLaConsole(_AvecUnDossierPersonnel):
    """CONSOLE.md (8.5, D32 ; INV-TERM-04, INV-TERM-01) : la console sur le côté, en pleine page,
    en fenêtre ; la mosaïque, le point milieu, l'arrière-plan ; un seul réglage de notification."""

    def test_sur_le_cote_un_terminal_a_la_fois(self):
        self.app.basculer_terminal()
        self.racine.update()
        self.assertEqual(self.place(), "ancre")
        self.assertEqual(self.gestion.montres(), ["terminal-1"])
        un = self.pid("terminal-1")
        deux = self.gestion.nouveau()
        self.racine.update()
        self.assertEqual(self.gestion.mosaique.page, ["terminal-1", deux])
        # Le nouveau terminal est actif : c'est lui qu'on voit ; l'autre attend en pastille.
        self.assertEqual((self.gestion.montres(), self.gestion.en_attente()),
                         ([deux], ["terminal-1"]))
        self.assertTrue(self.console.tuile(deux).winfo_ismapped())
        self.assertFalse(self.console.tuile("terminal-1").winfo_ismapped())
        self.assertFalse(self.console.point.winfo_ismapped())
        textes = self.console.textes_de_la_rangee()
        for texte in ("+ Nouveau", "Pleine page", "⋯"):
            self.assertIn(texte, textes)
        # Sur le côté, la place est comptée : « Détacher » est dans le menu « ⋯ ».
        self.assertNotIn("Détacher", textes)
        # Le terminal garde une hauteur utile, quel que soit le nombre de pastilles.
        self.assertGreaterEqual(self.console.tuile(deux).winfo_height(), 180)
        # Un clic sur la pastille remet l'autre terminal devant ; son shell n'a pas été relancé.
        self.console.pastille("terminal-1").invoke()
        self.racine.update()
        self.assertEqual((self.gestion.montres(), self.gestion.en_attente()),
                         (["terminal-1"], [deux]))
        self.assertTrue(self.console.tuile("terminal-1").winfo_ismapped())
        self.assertEqual(self.gestion.terminal("terminal-1").session.pid, un)
        # Le bouton de l'en-tête masque la console, puis la remontre à la même place.
        self.app.basculer_terminal()
        self.assertEqual(self.place(), "masque")
        self.app.basculer_terminal()
        self.assertEqual(self.place(), "ancre")

    def test_pleine_page_reduire_et_fenetre(self):
        self.app.basculer_terminal()
        self.racine.update()
        pid = self.pid("terminal-1")
        self.console.bouton("Pleine page").invoke()
        self.racine.update()
        self.assertEqual(self.place(), "page")
        cadre = self.plan.cadre("console")
        self.assertTrue(cadre.winfo_ismapped())
        self.assertLessEqual(abs(cadre.winfo_width() - self.plan.vertical.winfo_width()), 2)
        self.assertLessEqual(abs(cadre.winfo_height() - self.plan.vertical.winfo_height()), 2)
        textes = self.console.textes_de_la_rangee()
        for texte in ("+ Nouveau terminal", "Réduire", "Détacher", "Notifier après"):
            self.assertIn(texte, textes)
        self.assertNotIn("Pleine page", textes)
        self.console.bouton("Réduire").invoke()
        self.assertEqual(self.place(), "ancre")
        # En pleine page, le bouton de l'en-tête réduit la console ; il ne la masque pas.
        self.gestion.pleine_page()
        self.app.basculer_terminal()
        self.assertEqual(self.place(), "ancre")
        # Ouvrir une action ou un sujet réduit la console : ce qu'on a demandé s'affiche.
        self.gestion.pleine_page()
        self.app.ouvrir_action("ou_en_est_le_projet")
        self.assertEqual(self.place(), "ancre")
        self.gestion.pleine_page()
        self.app.choisir_sujet("Lire")
        self.assertEqual(self.place(), "ancre")
        # En fenêtre : la console entière ; la pleine page puis « Réduire » rendent la fenêtre.
        self.gestion.pleine_page()
        self.racine.update()
        self.console.bouton("Détacher").invoke()
        self.racine.update()
        self.assertEqual(self.place(), "flottant")
        self.assertIn("Ancrer", self.console.textes_de_la_rangee())
        self.console.bouton("Pleine page").invoke()
        self.racine.update()
        self.assertEqual(self.place(), "page")
        self.console.bouton("Réduire").invoke()
        self.racine.update()
        self.assertEqual(self.place(), "flottant")
        self.console.bouton("Ancrer").invoke()
        self.assertEqual(self.place(), "ancre")
        # Aucun de ces changements de place n'a relancé le shell (INV-TERM-01).
        self.assertEqual(self.gestion.terminal("terminal-1").session.pid, pid)
        self.assertTrue(self.gestion.terminal("terminal-1").session.en_vie())

    def test_la_mosaique_de_un_a_quatre_terminaux(self):
        from mosaique import en_pixels
        un, deux, trois, quatre = self.en_pleine_page(4)
        self.assertEqual(self.gestion.montres(), [un, deux, trois, quatre])
        self.assertEqual(self.gestion.en_attente(), [])
        zone = self.console.zone
        largeur, hauteur = zone.winfo_width(), zone.winfo_height()
        attendu = dict(en_pixels(self.gestion.mosaique.rectangles(), largeur, hauteur))
        for interne in (un, deux, trois, quatre):
            self.assertTrue(self.console.tuile(interne).winfo_ismapped(), interne)
            self.assertEqual(self.geometrie(interne), attendu[interne], interne)
        # Le point milieu est au croisement des séparations.
        point = self.console.point
        self.assertTrue(point.winfo_ismapped())
        self.assertEqual((point.winfo_x() + point.winfo_width() // 2,
                          point.winfo_y() + point.winfo_height() // 2),
                         (round(0.5 * largeur), round(0.5 * hauteur)))
        self.assertEqual(len([p for p in self.console.poignees if p.winfo_ismapped()]), 4)
        # La tuile active porte le liseré ; les autres non.
        # Contrat modifié (8.5b) : la tuile n'est plus un cadre ttk à image (éclairs de papier au
        # redimensionnement, fond pavé lent) mais un cadre classique au fond du terminal ; le
        # liseré lavande est sa bordure, et non plus un style.
        from apparence import COULEURS
        lisere = lambda nom: str(self.console.tuile(nom).cget("highlightbackground")).upper()
        self.assertEqual(lisere(quatre), COULEURS["lavande"])
        self.assertNotEqual(lisere(un), COULEURS["lavande"])
        self.gestion.terminal(un).saisie.focus_force()
        self.attendre(lambda: self.gestion.mosaique.actif == un)
        self.assertEqual(lisere(un), COULEURS["lavande"])
        self.assertNotEqual(lisere(quatre), COULEURS["lavande"])
        # Chaque terminal de la page a son propre shell.
        pids = {self.pid(interne) for interne in (un, deux, trois, quatre)}
        self.assertEqual(len(pids), 4)
        # À trois : un grand à gauche, sur toute la hauteur.
        self.gestion.ranger(quatre)
        self.racine.update()
        self.assertEqual(self.geometrie(un)[3], hauteur)
        self.assertTrue(point.winfo_ismapped())
        # À deux : une séparation et sa poignée, plus de point.
        self.gestion.ranger(trois)
        self.racine.update()
        self.assertFalse(point.winfo_ismapped())
        self.assertEqual(len([p for p in self.console.poignees if p.winfo_ismapped()]), 1)
        self.gestion.ranger(deux)
        self.racine.update()
        # Seul sur la page : toute la place (la rangée, avec trois pastilles, a pu grandir).
        self.assertEqual(self.geometrie(un), (0, 0, zone.winfo_width(), zone.winfo_height()))

    def test_page_pleine_ranger_et_rouvrir(self):
        un, deux, trois, quatre = self.en_pleine_page(4)
        pid_deux = self.pid(deux)
        self.gestion.activer(deux)
        # Page pleine : « + Nouveau terminal » met l'arrivant à la place du terminal actif.
        self.console.bouton("+ Nouveau terminal").invoke()
        self.racine.update()
        cinq = self.gestion.noms_internes()[-1]
        self.assertEqual((self.gestion.mosaique.page, self.gestion.mosaique.reserve),
                         ([un, cinq, trois, quatre], [deux]))
        self.assertFalse(self.console.tuile(deux).winfo_ismapped())
        # Le terminal remplacé continue en arrière-plan, et sa pastille le nomme.
        self.assertTrue(_vivant(pid_deux))
        self.assertIn("bash", str(self.console.pastille(deux).cget("text")))
        self.assertIn("EN ARRIÈRE-PLAN", self.console.textes_de_la_rangee())
        # « Ranger » d'une tuile, puis la pastille pour rouvrir.
        self.gestion.terminal(cinq).boutons["ranger"].invoke()
        self.racine.update()
        self.assertEqual((self.gestion.mosaique.page, self.gestion.mosaique.reserve),
                         ([un, trois, quatre], [deux, cinq]))
        self.console.pastille(deux).invoke()
        self.racine.update()
        self.assertEqual((self.gestion.mosaique.page, self.gestion.mosaique.actif),
                         ([un, trois, quatre, deux], deux))
        self.assertTrue(self.console.tuile(deux).winfo_ismapped())
        self.assertEqual(self.gestion.terminal(deux).session.pid, pid_deux)
        # « Fermer » d'une tuile arrête son shell.
        self.gestion.terminal(deux).boutons["fermer"].invoke()
        self.assertNotIn(deux, self.gestion.noms_internes())
        self.attendre(lambda: not _vivant(pid_deux))

    def test_agrandir_et_tirer_le_point_milieu(self):
        from mosaique import ECART, PART_AGRANDIE, PART_MIN
        un, deux, trois, quatre = self.en_pleine_page(4)
        pids = [self.pid(interne) for interne in (un, deux, trois, quatre)]
        m = self.gestion.mosaique
        zone = self.console.zone
        largeur, hauteur = zone.winfo_width(), zone.winfo_height()
        # Agrandir : les deux tiers pour ce terminal, les autres se resserrent sans disparaître.
        self.gestion.terminal(trois).boutons["agrandir"].invoke()
        self.racine.update()
        self.assertEqual(m.actif, trois)
        self.assertAlmostEqual(m.x, 1 - PART_AGRANDIE)
        self.assertAlmostEqual(m.y, 1 - PART_AGRANDIE)
        self.assertGreater(self.geometrie(trois)[2], 1.8 * self.geometrie(un)[2])
        self.assertGreater(self.geometrie(trois)[3], 1.8 * self.geometrie(un)[3])
        for interne in (un, deux, quatre):
            self.assertTrue(self.console.tuile(interne).winfo_ismapped())
        # Second clic : parts égales.
        self.gestion.terminal(trois).boutons["agrandir"].invoke()
        self.assertEqual((m.x, m.y), (0.5, 0.5))
        # Tirer le point milieu redimensionne les quatre terminaux.
        point = self.console.point
        racine_x, racine_y = zone.winfo_rootx(), zone.winfo_rooty()
        point.event_generate("<ButtonPress-1>", x=5, y=5)
        point.event_generate("<B1-Motion>", rootx=racine_x + round(0.6 * largeur),
                             rooty=racine_y + round(0.4 * hauteur))
        point.event_generate("<ButtonRelease-1>")
        self.racine.update()
        self.assertAlmostEqual(m.x, 0.6, delta=0.005)
        self.assertAlmostEqual(m.y, 0.4, delta=0.005)
        self.assertEqual(self.geometrie(un)[2], round(m.x * largeur) - ECART // 2)
        self.assertEqual(self.geometrie(trois)[3], hauteur - round(m.y * hauteur) - ECART // 2)
        # Tiré hors de la page : aucune part sous un quart.
        point.event_generate("<ButtonPress-1>", x=5, y=5)
        point.event_generate("<B1-Motion>", rootx=racine_x - 400, rooty=racine_y - 400)
        point.event_generate("<ButtonRelease-1>")
        self.assertEqual((m.x, m.y), (PART_MIN, PART_MIN))
        # Une séparation ne se tire que sur son axe.
        verticale = next(p for p in self.console.poignees if p.winfo_ismapped() and p.axe == "x")
        verticale.event_generate("<ButtonPress-1>", x=1, y=1)
        verticale.event_generate("<B1-Motion>", rootx=racine_x + round(0.5 * largeur),
                                 rooty=racine_y + round(0.7 * hauteur))
        verticale.event_generate("<ButtonRelease-1>")
        self.assertAlmostEqual(m.x, 0.5, delta=0.005)
        self.assertEqual(m.y, PART_MIN)
        # Double-clic sur le point : parts égales (Tk refuse le modificateur « Double » d'un
        # événement fabriqué : deux pressions rapprochées font le double-clic).
        for _ in range(2):
            point.event_generate("<ButtonPress-1>", x=5, y=5)
            point.event_generate("<ButtonRelease-1>", x=5, y=5)
        self.assertEqual((m.x, m.y), (0.5, 0.5))
        # Rien de tout cela n'a touché aux shells (INV-TERM-04).
        self.assertEqual([self.gestion.terminal(i).session.pid for i in (un, deux, trois, quatre)],
                         pids)

    def test_aucun_fond_de_papier_sous_les_widgets_d_une_tuile(self):
        """8.5b (retour de l'utilisateur : éclairs beiges sur la croix et les boutons d'une tuile
        qu'on redimensionne). Le serveur peint le fond de fenêtre d'un widget avant que Tk le
        redessine ; celui d'un widget ttk est le fond de son premier ancêtre classique. Dans une
        tuile, ce fond est partout celui du terminal."""
        from tkinter import ttk
        from apparence import COULEURS
        un, deux = self.en_pleine_page(2)
        nuit = COULEURS["terminal"].lower()

        def fond_de_fenetre(widget):
            while isinstance(widget, ttk.Widget):
                widget = widget.master
            return str(widget.cget("background")).lower()

        def descendants(widget):
            for enfant in widget.winfo_children():
                yield enfant
                yield from descendants(enfant)

        for interne in (un, deux):
            tuile = self.console.tuile(interne)
            self.assertNotIsInstance(tuile, ttk.Widget)
            self.assertEqual(fond_de_fenetre(tuile), nuit)
            montres = [w for w in descendants(tuile) if w.winfo_ismapped()]
            # En-tête (point, nom, trois boutons), sortie, saisie, bouton d'action : au moins.
            self.assertGreaterEqual(len(montres), 9)
            for widget in montres:
                self.assertEqual(fond_de_fenetre(widget), nuit, str(widget))

    def test_tirer_le_point_ne_refait_que_la_mosaique(self):
        """8.5b (retour de l'utilisateur : redimensionnement saccadé). Pendant qu'on tire le point
        milieu, seules les tuiles, les poignées et le point changent de place : la rangée de la
        console n'est ni relue ni reconstruite, rien n'est écrit sur le disque avant le lâcher."""
        self.en_pleine_page(4)
        zone, point = self.console.zone, self.console.point
        largeur, hauteur = zone.winfo_width(), zone.winfo_height()
        chemin = Path(os.environ["XDG_CONFIG_HOME"]) / "memoire" / "terminaux.json"
        avant = (self.console.lectures_de_la_rangee, chemin.read_text(encoding="utf-8"))
        point.event_generate("<ButtonPress-1>", x=5, y=5)
        for pas in range(10):
            point.event_generate("<B1-Motion>", rootx=zone.winfo_rootx() + largeur * (30 + 4 * pas) // 100,
                                 rooty=zone.winfo_rooty() + hauteur * (70 - 4 * pas) // 100)
            self.racine.update_idletasks()
        self.assertEqual((self.console.lectures_de_la_rangee, chemin.read_text(encoding="utf-8")),
                         avant)
        self.assertAlmostEqual(self.gestion.mosaique.x, 0.66, delta=0.01)
        point.event_generate("<ButtonRelease-1>")
        self.assertIn(f'"x": {self.gestion.mosaique.x}', chemin.read_text(encoding="utf-8"))

    def test_la_page_vide_propose_un_terminal(self):
        """8.5b : sans terminal sur la page, la console le dit et offre d'en ouvrir un."""
        self.app.basculer_terminal()
        self.gestion.pleine_page()
        self.racine.update()
        self.assertFalse(self.console.vide.winfo_ismapped())
        self.gestion.fermer("terminal-1")
        self.racine.update()
        self.assertEqual(self.gestion.noms_internes(), [])
        self.assertTrue(self.console.vide.winfo_ismapped())
        self.console.bouton_de_la_page_vide.invoke()
        self.racine.update()
        self.assertEqual(len(self.gestion.mosaique.page), 1)
        self.assertFalse(self.console.vide.winfo_ismapped())

    def test_le_bouton_de_l_en_tete_dit_si_la_console_est_montree(self):
        """8.5b : le bouton rond de la console est une pastille de papier quand elle est visible
        (APPARENCE.md, section 2 : l'élément choisi ressort du verre)."""
        habillage = self.app.habillage
        self.assertEqual(habillage.outils_choisis(), [])
        self.app.basculer_terminal()
        self.racine.update()
        self.assertEqual(habillage.outils_choisis(), ["terminaux"])
        self.gestion.pleine_page()
        self.assertEqual(habillage.outils_choisis(), ["terminaux"])
        self.plan.masquer("console")
        self.racine.update()
        self.assertEqual(habillage.outils_choisis(), [])

    def test_un_seul_reglage_de_notification(self):
        self.assertEqual(self.gestion.seuil, 10)
        self.gestion.seuil = 25
        deux = self.gestion.nouveau()
        self.assertEqual((self.app.terminal.seuil, self.gestion.terminal(deux).seuil), (25, 25))
        self.gestion.pleine_page()
        self.racine.update()
        self.assertEqual(self.console.seuil_saisi.get(), "25")
        self.console.bouton("+").invoke()
        self.assertEqual((self.gestion.seuil, self.console.seuil_saisi.get()), (30, "30"))
        self.console.bouton("−").invoke()
        self.console.bouton("−").invoke()
        self.assertEqual(self.gestion.terminal(deux).seuil, 20)
        # Le réglage est conservé avec la console.
        self.app.fermer()
        self.ouvrir()
        self.assertEqual(self.gestion.seuil, 20)

    def test_la_console_est_retrouvee_a_la_reouverture(self):
        un, deux, trois = self.en_pleine_page(3)
        quatre = self.gestion.nouveau()
        self.gestion.ranger(deux)
        self.gestion.deplacer_le_point(0.6, 0.4)
        self.gestion.activer(trois)
        avant = self.gestion.mosaique.vers_json()
        self.assertEqual((avant["page"], avant["reserve"], avant["actif"]),
                         ([un, trois, quatre], [deux], trois))
        self.app.fermer()
        self.ouvrir()
        self.assertEqual(self.place(), "page")
        self.assertEqual(self.gestion.mosaique.vers_json(), avant)
        self.racine.update()
        self.attendre(lambda: self.gestion.montres() == [un, trois, quatre])
        self.attendre(lambda: all(self.lance(i) for i in (un, trois, quatre)))
        self.assertIsNone(self.gestion.terminal(deux).session)

    def test_le_menu_de_la_console_est_celui_d_un_panneau(self):
        self.app.basculer_terminal()
        self.racine.update()
        self.console.bouton("⋯").invoke()
        self.racine.update()
        menu = self.plan.panneaux["console"].menu
        self.assertTrue(menu.liste.visible())
        libelles = menu.liste.libelles()
        for libelle in ("Mettre devant", "Ancrer à gauche", "Masquer", "Détacher"):
            self.assertIn(libelle, libelles)
        menu.liste.fermer()

    def test_la_barre_d_etat_ouvre_le_terminal_qui_travaille(self):
        self.app.basculer_terminal()
        deux = self.gestion.nouveau()
        self.racine.update()
        self.attendre(lambda: self.lance(deux))
        self.gestion.terminal(deux).executer("sleep 30")
        self.attendre(lambda: self.gestion.en_cours() == [(deux, "sleep 30")])
        self.gestion.afficher("terminal-1")
        self.plan.masquer("console")
        segment = next(s for s in self.app.habillage.segments_de_la_barre()
                       if s[0] == "terminal · sleep 30")
        segment[2]()
        self.assertEqual(self.place(), "ancre")
        self.assertEqual(self.gestion.montres(), [deux])
        self.app.demander = lambda titre, texte: True


@unittest.skipUnless(tk and os.environ.get("DISPLAY") and Path(EXECUTABLE).is_file()
                     and shutil.which("bash"), "Tkinter, affichage, exécutable memoire ou bash absent")
class EssaiDeLaCompletion(_AvecUnDossierPersonnel):
    """Contrat modifié (8.5, D32) : les onglets de 8.4 n'existent plus (la console en mosaïque
    les remplace, EssaiDeLaConsole) ; leurs deux essais sont retirés, ceux de la complétion
    restent tels quels. APPARENCE.md, section 11 : complétion par Tab et double Tab."""

    def setUp(self):
        super().setUp()
        self.app.basculer_terminal()
        self.racine.update()

    def test_les_possibilites_ont_le_fond_du_terminal(self):
        terminal = self.app.terminal
        terminal.ligne.set("cd Doc")
        terminal.completer_la_saisie()
        terminal.completer_la_saisie()
        self.racine.update()
        self.assertTrue(terminal.completions.sombre)
        image = terminal.completions.image_de_fond
        r, v, b = image.get(image.width() // 2, image.height() // 2)
        self.assertLess(r + v + b, 3 * 90)
        terminal.completions.fermer()

    def test_tab_complete_et_double_tab_montre_les_possibilites(self):
        terminal = self.app.terminal
        terminal.ligne.set("cd Docu")
        terminal.completer_la_saisie()
        self.assertEqual(terminal.ligne.get(), "cd Documents/")
        terminal.ligne.set("cd Doc")
        terminal.completer_la_saisie()
        self.assertEqual(terminal.ligne.get(), "cd Doc")
        self.assertFalse(terminal.completions.visible())
        terminal.completer_la_saisie()
        self.racine.update()
        self.assertTrue(terminal.completions.visible())
        self.assertEqual(terminal.completions.libelles(), ["Docker/", "Documents/"])
        terminal.completions.choisir(1)
        self.assertEqual(terminal.ligne.get(), "cd Documents/")
        self.assertFalse(terminal.completions.visible())
