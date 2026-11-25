"""Vue locale d'une note (VOISINAGE.md, sections 1 et 4 ; INV-VIEW-02 ; R7).

La note au centre, ses voisins autour, chaque lien coloré selon son origine (note entière, citation,
catégorie) et doublé d'un style de trait. Aucune règle ici : les liens et leur origine viennent de
`memoire voisinage --tsv`. La partie pure (lecture, disposition, libellés) se teste sans Tkinter
(gui/test_voisinage.py) ; la fenêtre, sous xvfb-run (gui/test_gui.py).
"""
from dataclasses import dataclass
import math
import re

ENTETE_TSV = "relation\tsens\ttype\torigine\tvoisine\trole\tnom\tsource\tetat"
SENS = {"sortant": "→", "entrant": "←"}
# Couleur et trait de chaque origine : palette lisible par les daltoniens, doublée du trait.
STYLES = {
    "note": ("#0072B2", ()),
    "citation": ("#D55E00", (2, 4)),
    "categorie": ("#009E73", (8, 4)),
}
NOMS_D_ORIGINE = {"note": "note entière", "citation": "citation", "categorie": "catégorie"}
ETATS = {"courante": "", "ecartee": "écartée", "a-reexaminer": "à réexaminer", "orpheline": "orpheline"}
LONGUEUR_DE_LIBELLE = 28
MARGE = 40
RAYON_DU_CENTRE = 10


@dataclass(frozen=True)
class Lien:
    """Une ligne de `voisinage --tsv` ; `source` : source d'une citation, sinon None."""

    relation: str
    sens: str
    type: str
    origine: str
    voisine: str
    role: str
    nom: str
    source: str
    etat: str


def lire_voisinage(sortie):
    """Liens d'une sortie `voisinage --tsv` ; ValueError si elle n'a pas la forme attendue."""
    lignes = sortie.splitlines()
    if not lignes or lignes[0] != ENTETE_TSV:
        raise ValueError("sortie de « voisinage --tsv » inattendue")
    liens = []
    for ligne in lignes[1:]:
        champs = ligne.split("\t")
        if len(champs) != 9:
            raise ValueError(f"ligne de voisinage invalide : {ligne}")
        lien = Lien(*champs[:7], None if champs[7] == "-" else champs[7], champs[8])
        if lien.sens not in SENS or lien.origine not in STYLES or lien.etat not in ETATS:
            raise ValueError(f"ligne de voisinage invalide : {ligne}")
        liens.append(lien)
    return liens


def entete(sortie_lisible):
    """Désignation de la note au centre, lue dans la première ligne de `voisinage`."""
    premiere = sortie_lisible.split("\n", 1)[0]
    if not premiere.startswith("Voisinage de "):
        raise ValueError("sortie de « voisinage » inattendue")
    return premiere[len("Voisinage de "):]


def titre_et_role(designation):
    """(titre, rôle) d'une désignation « Titre (identifiant) — rôle » ; sinon (désignation, "")."""
    trouve = re.match(r"^(.*) \(([0-9a-f]{8}-[0-9a-f-]{27})\) — (\S+)$", designation)
    if trouve is None:
        return designation, ""
    return trouve.group(1), trouve.group(3)


def disposer(nombre, largeur, hauteur, marge=MARGE):
    """Positions entières des voisins sur un cercle autour du centre : le premier en haut, puis
    dans le sens des aiguilles d'une montre."""
    cx, cy = largeur / 2, hauteur / 2
    rayon = min(largeur, hauteur) / 2 - marge
    positions = []
    for k in range(nombre):
        angle = -math.pi / 2 + 2 * math.pi * k / nombre
        positions.append((round(cx + rayon * math.cos(angle)), round(cy + rayon * math.sin(angle))))
    return positions


def disposer_en_ellipse(nombre, largeur, hauteur, demi_pastille=(78, 16)):
    """Comme `disposer`, sur une ellipse qui tient compte de la taille des pastilles (panneau
    étroit ou bas) ; un seul voisin va en haut."""
    cx, cy = largeur / 2, hauteur / 2
    rx = max(10, largeur / 2 - demi_pastille[0] - 6)
    ry = max(10, hauteur / 2 - demi_pastille[1] - 6)
    return [(round(cx + rx * math.cos(-math.pi / 2 + 2 * math.pi * k / nombre)),
             round(cy + ry * math.sin(-math.pi / 2 + 2 * math.pi * k / nombre)))
            for k in range(nombre)]


def disposer_en_coins(nombre, largeur, hauteur, demi_pastille=(82, 22)):
    """Jusqu'à quatre voisins aux quatre coins, dans l'ordre haut gauche, haut droite, bas gauche,
    bas droite (comme la maquette) ; au-delà, sur une ellipse."""
    if nombre > 4:
        return disposer_en_ellipse(nombre, largeur, hauteur)
    cx, cy = largeur / 2, hauteur / 2
    dx = max(10, largeur / 2 - demi_pastille[0] - 4)
    dy = max(10, hauteur / 2 - demi_pastille[1] - 6)
    coins = [(cx - dx, cy - dy), (cx + dx, cy - dy), (cx - dx, cy + dy), (cx + dx, cy + dy)]
    return [(round(x), round(y)) for x, y in coins[:nombre]]


def bord(x, y, vers_x, vers_y, demi_largeur, demi_hauteur):
    """Point où le segment de (x, y) vers (vers_x, vers_y) sort du rectangle centré en (x, y) de
    demi-dimensions données ; (x, y) si les deux points sont confondus."""
    dx, dy = vers_x - x, vers_y - y
    longueur = math.hypot(dx, dy)
    if longueur == 0:
        return (x, y)
    ux, uy = dx / longueur, dy / longueur
    limites = [demi_largeur / abs(ux)] if ux else []
    limites += [demi_hauteur / abs(uy)] if uy else []
    s = min(min(limites), longueur)
    return (round(x + ux * s), round(y + uy * s))


def tronquer(texte, longueur=LONGUEUR_DE_LIBELLE):
    """Libellé d'au plus `longueur` caractères, « … » final s'il est coupé."""
    return texte if len(texte) <= longueur else texte[:longueur - 1] + "…"


def nom_affiche(lien):
    """Nom d'un voisin : titre, ou texte cité entre guillemets."""
    return f"« {lien.nom} »" if lien.origine == "citation" else lien.nom


def ligne_de_liste(lien):
    """Ligne de la liste textuelle : sens, type, voisin, état, origine."""
    etat = ETATS[lien.etat]
    suffixe = f" ({etat})" if etat else ""
    return f"{SENS[lien.sens]} {lien.type} {nom_affiche(lien)}{suffixe} — {NOMS_D_ORIGINE[lien.origine]}"


def peindre(dessin_, liens, largeur, hauteur, titre_du_centre, fond, images, compact=False):
    """Dessine le voisinage (APPARENCE.md, section 9.3) : la note au centre en pastille d'encre,
    chaque voisin en pastille blanche au liseré de la couleur de son origine (écartée : grise,
    en italique), traits colorés et fléchés avec leur type. Rend {voisine: étiquette}. Les textes
    des voisins portent l'étiquette « voisin » ; les traits « lien » et « origine:… »."""
    import dessin
    from dessin import rgba
    from theme import photo, police
    dessin_.delete("all")
    cx, cy = largeur / 2, hauteur / 2
    # Compact (panneau) : libellés sur deux lignes au plus, sans couper avant 60 caractères.
    longueur = 60 if compact else LONGUEUR_DE_LIBELLE
    # Deux pastilles doivent tenir côte à côte aux coins : leur texte suit la largeur du panneau.
    largeur_de_texte = max(70, min(136, largeur / 2 - 46)) if compact else 0
    if compact:
        # Deux lignes au plus : environ cinq pixels et demi par caractère en moyenne.
        longueur = max(14, int(largeur_de_texte / 5.5) * 2)
    marge = 70 if compact else MARGE + 20

    def pastille(cle, fabrique):
        if cle not in images:
            images[cle] = photo(dessin_, fabrique(), fond)
        return images[cle]

    centre = dessin_.create_text(cx, cy, text=tronquer(titre_du_centre, longueur),
                                 fill="#FFEDE2", font=police(9 if compact else 10, "bold"),
                                 width=largeur_de_texte + 20 if compact else 0,
                                 justify="center", tags=("centre",))
    g, h_, d, b = dessin_.bbox(centre)
    cl, ch = d - g + 28, b - h_ + 14
    image = pastille(("centre", cl, ch), lambda: dessin.forme(
        cl, ch, min(ch / 2, 15), rgba("#241F2E"), ombre=(3, 4, 3, rgba("#241F2E", .25))))
    fond_du_centre = dessin_.create_image(cx - cl / 2, cy - ch / 2, image=image, anchor="nw",
                                          tags=("centre",))
    etiquettes = {}
    positions = (disposer_en_coins(len(liens), largeur, hauteur,
                                   (largeur_de_texte / 2 + 14, 24)) if compact
                 else disposer(len(liens), largeur, hauteur, marge))
    for lien, (x, y) in zip(liens, positions):
        couleur, tirets = STYLES[lien.origine]
        ecartee = lien.etat == "ecartee"
        etiquette = f"voisin:{lien.voisine}"
        texte = dessin_.create_text(
            x, y, text=tronquer(nom_affiche(lien), longueur), tags=("voisin", etiquette),
            fill="#6E6371" if ecartee else "#241F2E", width=largeur_de_texte,
            justify="center", font=police(9, "italic" if ecartee else "bold"))
        gauche, haut, droite, bas = dessin_.bbox(texte)
        pl, ph = droite - gauche + 24, bas - haut + 12
        # Une pastille haute (trois lignes) reste entière dans le dessin.
        y_voulu = min(max(y, ph / 2 + 4), hauteur - ph / 2 - 8)
        if y_voulu != y:
            dessin_.move(texte, 0, y_voulu - y)
            y = y_voulu
        image = pastille(("voisin", pl, ph, couleur, ecartee), lambda: dessin.forme(
            pl, ph, min(ph / 2, 14), rgba("#F1ECEE" if ecartee else "#FFFFFF"),
            bord=rgba(couleur, .35 if ecartee else .95), epaisseur=1.5,
            ombre=(3, 4, 3, rgba("#241F2E", .12))))
        dessin_.create_image(x - pl / 2, y - ph / 2, image=image, anchor="nw", tags=(etiquette,))
        # Le trait s'arrête au bord de la pastille du voisin et de celle du centre.
        bout = bord(x, y, cx, cy, pl / 2 + 3, ph / 2 + 3)
        depart_centre = bord(cx, cy, x, y, cl / 2 + 3, ch / 2 + 3)
        depart, arrivee = (depart_centre, bout) if lien.sens == "sortant" else (bout, depart_centre)
        dessin_.create_line(*depart, *arrivee, fill=couleur, width=2, dash=tirets,
                            arrow="last", tags=("lien", f"origine:{lien.origine}"))
        # Le type du lien contre la pastille de son voisin, du côté du centre (aux coins, les
        # milieux des traits se confondraient).
        if compact:
            tx, ty = x, (y + ph / 2 + 9) if y < cy else (y - ph / 2 - 9)
        else:
            tx, ty = (cx + x) / 2, (cy + y) / 2 - 9
        dessin_.create_text(tx, ty, text=lien.type, fill=couleur, font=police(8), tags=("type",))
        etiquettes[lien.voisine] = etiquette
    for etiquette in etiquettes.values():
        dessin_.tag_raise(etiquette)
    for item in dessin_.find_withtag("voisin"):
        dessin_.tag_raise(item)
    dessin_.tag_raise("type")
    dessin_.tag_raise(fond_du_centre)
    dessin_.tag_raise(centre)
    return etiquettes


class PanneauVoisinage:
    """Panneau « Voisinage » (APPARENCE.md, section 9.3 ; 8.2c) : le voisinage de la note lue dans
    la carte ; un clic sur un voisin le lit ; « Agrandir » ouvre la vue locale complète."""

    def __init__(self, cadre, app):
        import tkinter as tk
        from tkinter import ttk
        from apparence import COULEURS
        from theme import Flux
        self.app = app
        self.note = None
        self.liens = []
        self._images = {}
        self._attente = None
        cadre.configure(padding=(10, 4, 10, 8))
        # Une carte claire, comme la maquette : titre, graphe et légende dedans.
        fond = COULEURS["champ"]
        self.carte = ttk.Frame(cadre, style="Encadre.TFrame", padding=(16, 14, 16, 12))
        self.carte.pack(fill="both", expand=True)
        haut = ttk.Frame(self.carte, style="EncadreInterieur.TFrame")
        haut.pack(fill="x")
        ttk.Label(haut, text="VOISINAGE", style="EncadreEtiquette.TLabel").pack(side="left")
        ttk.Button(haut, text="Agrandir", style="EncadreBandeau.TButton",
                   command=self.agrandir).pack(side="right")
        self.titre = ttk.Label(self.carte, text="Ouvrir une note pour voir son voisinage.",
                               style="EncadreSecond.TLabel")
        self.titre.pack(fill="x", pady=(4, 0))
        self.dessin = tk.Canvas(self.carte, height=230, highlightthickness=0, borderwidth=0,
                                background=fond)
        self.dessin.pack(fill="both", expand=True, pady=(4, 4))
        legende = Flux(self.carte, ecart=14, interligne=2, style="EncadreInterieur.TFrame")
        legende.pack(fill="x")
        for origine, (couleur, tirets) in STYLES.items():
            ligne = ttk.Frame(legende, style="EncadreInterieur.TFrame")
            echantillon = tk.Canvas(ligne, width=20, height=10, highlightthickness=0,
                                    background=fond)
            echantillon.create_line(1, 5, 19, 5, fill=couleur, width=2, dash=tirets)
            echantillon.pack(side="left")
            ttk.Label(ligne, text=NOMS_D_ORIGINE[origine], style="EncadreSecond.TLabel").pack(
                side="left", padx=(4, 0))
            legende.ajouter(ligne)
        self.dessin.bind("<Configure>", lambda _e: self._redessiner_bientot(), add="+")

    def suivre(self, note):
        """Montre le voisinage de `note` (la note lue dans la carte)."""
        self.note = note
        self.actualiser()

    def _redessiner_bientot(self):
        if self._attente is not None:
            self.dessin.after_cancel(self._attente)
        self._attente = self.dessin.after(80, self.dessiner)

    def actualiser(self):
        from visionneur import interroger
        self.liens = []
        self._titre_du_centre = ""
        if self.note is None:
            return
        code, lisible, _ = interroger(self.app.memoire, self.app.base.get(), ["voisinage", self.note])
        code_tsv, tsv, _ = interroger(self.app.memoire, self.app.base.get(),
                                      ["voisinage", self.note, "--tsv"])
        if code != 0 or code_tsv != 0:
            self.titre.configure(text="Voisinage indisponible pour cette note.")
            self.dessin.delete("all")
            return
        titre, _role = titre_et_role(entete(lisible))
        self._titre_du_centre = titre
        self.liens = lire_voisinage(tsv)
        nombre = len(self.liens)
        self.titre.configure(text=f"{nombre} lien{'s' if nombre > 1 else ''}" if nombre
                             else "Aucun lien")
        self.dessiner()

    def dessiner(self):
        self._attente = None
        if self.note is None:
            return
        from apparence import COULEURS
        largeur = max(200, self.dessin.winfo_width())
        hauteur = max(180, self.dessin.winfo_height())
        etiquettes = peindre(self.dessin, self.liens, largeur, hauteur, self._titre_du_centre,
                             COULEURS["champ"], self._images, compact=True)
        for voisine, etiquette in etiquettes.items():
            self.dessin.tag_bind(etiquette, "<Button-1>", lambda _e, n=voisine: self.ouvrir(n))
            self.dessin.tag_bind(etiquette, "<Enter>",
                                 lambda _e: self.dessin.configure(cursor="hand2"))
            self.dessin.tag_bind(etiquette, "<Leave>", lambda _e: self.dessin.configure(cursor=""))

    def ouvrir(self, note):
        """Lit une voisine dans la carte (le panneau la suit)."""
        return self.app.voir(note)

    def agrandir(self):
        """La vue locale complète, dans sa fenêtre."""
        if self.note is None:
            return None
        return Voisinage(self.app, self.note)


class Voisinage:
    """Fenêtre de la vue locale, liée à la fenêtre principale `app`."""

    LARGEUR, HAUTEUR = 640, 520
    # Un clic recentre après ce délai, sauf s'il fait partie d'un double-clic.
    DELAI_DU_CLIC = 300

    def __init__(self, app, note):
        import tkinter as tk
        from tkinter import ttk
        self.app = app
        self.note = note
        self.pile = []
        self.clic_en_attente = None
        self.fenetre = tk.Toplevel(app.racine)
        self.fenetre.title(f"memoire — voisinage de {note}")
        haut = ttk.Frame(self.fenetre, padding=(14, 12, 14, 6))
        haut.pack(fill="x")
        self.titre = ttk.Label(haut, text="", style="SousTitre.TLabel")
        self.titre.pack(side="left")
        self.bouton_precedent = ttk.Button(haut, text="Précédent", command=self.precedent,
                                           state="disabled")
        self.bouton_precedent.pack(side="left", padx=6)
        ttk.Button(haut, text="Actualiser", command=self.actualiser).pack(side="left")
        ttk.Button(haut, text="Fermer", command=self.fenetre.destroy).pack(side="right")
        corps = ttk.Frame(self.fenetre, padding=6)
        corps.pack(fill="both", expand=True)
        from apparence import COULEURS
        from theme import ZoneDeTexte, police
        self.police = police
        self.dessin = tk.Canvas(corps, width=self.LARGEUR, height=self.HAUTEUR,
                                background=COULEURS["champ"], highlightthickness=0)
        self.dessin.pack(side="left")
        droite = ttk.Frame(corps, padding=(8, 0))
        droite.pack(side="left", fill="both", expand=True)
        legende = ttk.LabelFrame(droite, text="Légende", padding=4)
        legende.pack(fill="x")
        for origine, (couleur, tirets) in STYLES.items():
            ligne = ttk.Frame(legende)
            ligne.pack(anchor="w")
            echantillon = tk.Canvas(ligne, width=48, height=12, highlightthickness=0,
                                    background=COULEURS["papier"])
            echantillon.create_line(2, 6, 46, 6, fill=couleur, width=2, dash=tirets)
            echantillon.pack(side="left")
            ttk.Label(ligne, text=NOMS_D_ORIGINE[origine], foreground=couleur).pack(side="left", padx=4)
        self.liste = ZoneDeTexte(droite, width=48, height=24, wrap="word", font=police(10))
        self.liste.pack(fill="both", expand=True, pady=(6, 0))
        for origine, (couleur, _) in STYLES.items():
            self.liste.tag_configure(origine, foreground=couleur)
        self.etat = ttk.Label(self.fenetre, text="", padding=(14, 6, 14, 12), style="Second.TLabel")
        self.etat.pack(fill="x")
        self.actualiser()

    def interroger(self, args):
        from visionneur import interroger
        return interroger(self.app.memoire, self.app.base.get(), args)

    def actualiser(self):
        """Relit et redessine le voisinage de la note courante."""
        code, lisible, erreurs = self.interroger(["voisinage", self.note])
        code_tsv, tsv, _ = self.interroger(["voisinage", self.note, "--tsv"])
        self.dessin.delete("all")
        self.liste.delete("1.0", "end")
        if code != 0 or code_tsv != 0:
            self.titre.configure(text=self.note)
            self.etat.configure(text=erreurs.strip())
            return
        self.titre.configure(text=entete(lisible))
        self.etat.configure(text="Clic : recentrer — double-clic : ouvrir dans le visionneur")
        self.dessiner(lire_voisinage(tsv))

    def dessiner(self, liens):
        from apparence import COULEURS
        titre, _role = titre_et_role(self.titre.cget("text"))
        if not hasattr(self, "_images"):
            self._images = {}
        etiquettes = peindre(self.dessin, liens, self.LARGEUR, self.HAUTEUR, titre,
                             COULEURS["champ"], self._images)
        for lien in liens:
            etiquette = etiquettes[lien.voisine]
            self.dessin.tag_bind(etiquette, "<Button-1>",
                                 lambda _e, n=lien.voisine: self.clic(n))
            self.dessin.tag_bind(etiquette, "<Double-Button-1>",
                                 lambda _e, n=lien.voisine: self.double_clic(n))
            self.liste.insert("end", ligne_de_liste(lien) + "\n", lien.origine)
        if not liens:
            self.liste.insert("end", "(aucun lien)\n")

    def clic(self, note):
        self.annuler_clic()
        self.clic_en_attente = self.fenetre.after(self.DELAI_DU_CLIC, lambda: self.recentrer(note))

    def double_clic(self, note):
        self.annuler_clic()
        self.ouvrir(note)

    def annuler_clic(self):
        if self.clic_en_attente is not None:
            self.fenetre.after_cancel(self.clic_en_attente)
            self.clic_en_attente = None

    def recentrer(self, note):
        """Centre la vue sur une voisine ; la note quittée s'empile pour « Précédent »."""
        self.clic_en_attente = None
        self.pile.append(self.note)
        self.note = note
        self.bouton_precedent.configure(state="normal")
        self.actualiser()

    def precedent(self):
        if not self.pile:
            return
        self.note = self.pile.pop()
        if not self.pile:
            self.bouton_precedent.configure(state="disabled")
        self.actualiser()

    def ouvrir(self, note):
        """Ouvre une note dans le visionneur."""
        return self.app.voir(note)
