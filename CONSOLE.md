# La console en mosaïque — conception de l’incrément 8.5

Version 0.1 — 16 novembre 2026 — **validée par l’utilisateur le 16 novembre 2026** sur maquette interactive (« je valide le design, et tes choix ») ; une décision nouvelle (D32, section 9), qui remplace D29 pour la place des terminaux.

- **Demande de l’utilisateur** du 16 novembre 2026 : « améliorer la console : plusieurs fenêtres de terminal sur une même page, le choix d’ajouter une instance (en créer une ou en ouvrir une déjà existante), les agrandir facilement (un point milieu pour plus de deux consoles : si j’en agrandis une, les autres se redimensionnent bien) », dans l’esprit de « la technicité douce et légère » : « une action doit toujours être simple et efficace, pas une combinaison savante de plusieurs menus ».
- **Précisions de l’utilisateur** à la validation : les deux modes sont voulus, « sur le côté quand j’ai besoin de taper rapidement quelque chose », « le panneau complet quand je veux travailler de manière plus poussée » ; la fenêtre flottante porte la console entière, mosaïque comprise.
- **Prolonge** TERMINAL.md (D27), TERMINAUX.md (nommage et conservation de D29, qui restent), PLAN-DE-TRAVAIL.md (D28), APPARENCE.md (D31).
- **Constat de départ** : deux terminaux ancrés à droite donnaient des onglets sur trois rangées et une seule ligne de sortie visible.

---

## 1. Ce que l’utilisateur obtient

**La console est un seul bloc**, qui contient tous les terminaux. Elle a quatre places :

| Place | Usage | Ce qu’on voit |
|---|---|---|
| sur le côté (ancrée, à droite par défaut) | taper vite une commande sans quitter l’action ou la lecture | un terminal à la fois, les autres en pastilles |
| pleine page | travailler dans la console | la mosaïque, sur toute la carte |
| détachée (fenêtre flottante) | second écran | la mosaïque, selon la taille de la fenêtre |
| masquée | — | rien ; les shells continuent, la barre d’état nomme la commande en cours |

**Une action, un clic :**

| Geste | Effet |
|---|---|
| bouton rond `>_` de l’en-tête | montre la console là où elle était (sur le côté la première fois), curseur dans la saisie ; la masque si elle est visible ; en pleine page, la réduit |
| « Pleine page » (dans la console) | la console prend toute la carte |
| « Réduire » (en pleine page) | elle revient où elle était avant, sur le côté ou en fenêtre |
| « Détacher » / « Ancrer » | fenêtre flottante / retour sur le côté |
| « + Nouveau terminal » | crée un terminal, sur la page |
| pastille d’un terminal en arrière-plan | le remet sur la page |
| « ⋯ » | le menu ordinaire d’un panneau : ordre de la fenêtre, zone, masquer |

Ouvrir une action, un sujet du rail ou une note pendant la pleine page réduit la console : ce qu’on a demandé s’affiche, les terminaux restent à portée sur le côté.

---

## 2. La mosaïque

**De un à quatre terminaux sur la page**, disposés d’après leur nombre, sans réglage :

```
 1 terminal        2 terminaux       3 terminaux       4 terminaux
┌────────────┐    ┌─────┬──────┐    ┌─────┬──────┐    ┌─────┬──────┐
│            │    │     │      │    │     │  2   │    │  1  │  2   │
│     1      │    │  1  ┃  2   │    │  1  ●──────┤    ├─────●──────┤
│            │    │     │      │    │     │  3   │    │  4  │  3   │
└────────────┘    └─────┴──────┘    └─────┴──────┘    └─────┴──────┘
                   une poignée       le point milieu   le point milieu
```

- Un terminal ajouté ne déplace pas les autres : le deuxième prend la droite, le troisième le bas de la droite, le quatrième le bas de la gauche.
- **Le point milieu** (à partir de trois terminaux) : une pastille blanche à ombre courte, posée au croisement des séparations. La tirer redimensionne tous les terminaux ensemble ; un double-clic rend des parts égales. Les séparations se tirent aussi, sur un seul axe ; avec deux terminaux, il n’y a qu’une séparation et sa poignée.
- **Aucune part ne descend sous un quart** de la page : un terminal resserré garde son nom, quelques lignes et sa saisie.
- **Agrandir** : le bouton d’une tuile, ou un double-clic sur son nom, lui donne les deux tiers de la page dans chaque direction ; les autres se resserrent. Un second clic rend des parts égales.
- **Le terminal actif** (celui où l’on tape) porte un liseré lavande.

**La place commande le nombre de terminaux montrés** : moins de 660 pixels de large, un seul (c’est le cas sur le côté) ; au moins 660 de large mais moins de 340 de haut, deux côte à côte ; sinon jusqu’à quatre. Les terminaux de la page qui ne tiennent pas attendent en pastilles, avec ceux de l’arrière-plan ; le terminal actif est toujours montré.

---

## 3. Créer, rouvrir, ranger, fermer

- **« + Nouveau terminal »** : un clic, le terminal apparaît sur la page et reçoit le curseur.
- **En arrière-plan** : les terminaux ouverts qui ne sont pas sur la page sont des pastilles dans la rangée de la console, avec leur état (point vert qui pulse : commande en cours ; anneau : inactif ; point vermillon : dernière commande en échec). Un clic remet le terminal sur la page.
- **Page pleine** (quatre) : l’arrivant, nouveau ou rouvert, prend la place du terminal actif, qui passe en arrière-plan **sans s’arrêter**.
- **Ranger** (« — » d’une tuile) : le terminal passe en arrière-plan, son shell continue.
- **Fermer** (« ✕ ») : arrête le shell ; confirmation si une commande tourne (TERMINAUX.md, section 5).

Il n’y a plus ni onglets, ni panneau « Terminaux », ni menu « Terminaux ▾ » : la console montre tout.

---

## 4. Une tuile

```
┌──────────────────────────────────────────────────────────┐
│ ● make flash                    depuis 12 s    —  ⤢  ✕   │   état, nom (D29), ranger, agrandir, fermer
│ $ make flash                                             │
│ arm-none-eabi-gcc -mcpu=cortex-m3 -Os -c src/sched.c     │   sortie
│ $ [                                      ] (Interrompre) │   saisie ; « Interrompre » pendant une
└──────────────────────────────────────────────────────────┘   commande, « Effacer » sinon
```

- Le nom est celui de D29 : la commande en cours, sinon le shell et son dossier, doublons numérotés.
- L’état, à droite du nom : « depuis 12 s », « code 2 » après un échec, « terminé » si le shell est sorti ; « ● terminé, code N » quand une commande longue a fini hors de la vue, jusqu’à ce qu’on revienne au terminal (TERMINAL.md, section 4).
- Saisie, historique, complétion par Tab, saisie masquée, taille transmise au shell : inchangés.
- **« Notifier après N s »** devient un seul réglage pour toute la console, dans sa rangée quand la place le permet ; il est conservé.

---

## 5. Conservation

`~/.config/memoire/terminaux.json` (TERMINAUX.md, section 2) reçoit une clé `console` : le seuil de notification et la mosaïque (terminaux de la page dans l’ordre, terminaux en arrière-plan, terminal actif, position du point). La place de la console (côté, pleine page, fenêtre, masquée) est dans `plan-de-travail.json`, comme pour tout panneau.

- À la réouverture : mêmes terminaux aux mêmes places, shells neufs dans leur dernier dossier ; un terminal en arrière-plan ne lance son shell que lorsqu’on le remet sur la page.
- Lecture tolérante : une clé `console` absente ou invalide donne la mosaïque par défaut (les quatre premiers terminaux sur la page, parts égales) ; un nom inconnu est ignoré, un terminal oublié va en arrière-plan.
- Une disposition enregistrée avant 8.5 nommait un panneau par terminal : ces entrées sont ignorées, la console prend une fois sa place par défaut (masquée, à droite).

---

## 6. Architecture

```
gui/mosaique.py            modèle, sans Tkinter : page, arrière-plan, actif, point (x, y) ;
                           placer, ranger, retirer, activer, agrandir, égaliser, déplacer ;
                           rectangles, poignées, point, capacité, géométrie en pixels ; JSON
gui/console.py             réalisation Tkinter : rangée de la console, tuiles placées au pixel,
                           poignées, point milieu, glisser
gui/terminal.py            une tuile : en-tête, sortie, saisie (le moteur ne change pas)
gui/gestion_des_terminaux.py   registre, mosaïque, noms, actions, conservation
gui/plan_de_travail.py     état « page » d’un panneau (pleine page), un seul à la fois
gui/panneaux.py            la console est le panneau « console », sans bandeau
```

- **Une seule source de vérité** : le modèle de la mosaïque pour la place des terminaux, la disposition pour la place de la console. Chaque geste modifie le modèle, puis l’écran suit.
- **Aucun terminal n’est recréé** : une tuile est un cadre enfant de la console, placé ou retiré de l’écran ; son shell survit à tout changement de place (INV-TERM-01).
- **Pleine page** : le cadre de la console est posé par-dessus le plan de travail, dans la carte ; rien n’est défait dessous, « Réduire » retrouve tout.
- **Le point milieu en Tkinter** : un widget est un rectangle opaque. Son image est donc composée une fois par cas : le papier, les coins des tuiles voisines recopiés de l’image du thème, puis la pastille et son ombre. Les tuiles sont placées au pixel pour que tout s’aligne.
- Pas d’animation (APPARENCE.md, section 5) : une tuile agrandie change de taille d’un coup.

---

## 7. Ce qui reste comme avant

Le moteur du terminal (D27), le nommage et la confirmation de fermeture (D29), la complétion (8.4), la notification de fin, l’arrêt de tous les shells à la fermeture, l’ordre d’une fenêtre flottante (PLAN-DE-TRAVAIL.md, section 10), désormais pour la console entière.

**Ce qui disparaît, sans fonction perdue :** un panneau par terminal, les onglets (8.4), le panneau « Terminaux » et le menu « Terminaux ▾ » (6.9). Une seule différence d’usage : on ne fait plus flotter un terminal seul, mais la console, avec sa mosaïque (accord de l’utilisateur).

---

## 8. Tests

- **Sans affichage** (`gui/test_mosaique.py`) : chaque disposition, ajout sans déplacement, page pleine, ranger, retirer, actif toujours montré, agrandir et égaliser, bornes du point, capacité selon la place, géométrie en pixels (écart constant, bords exacts), prise d’une séparation ou du point, aller-retour JSON, contenu invalide, accord avec le registre. `gui/test_plan_de_travail.py` : état « page » (un seul, retour, masquer puis montrer, JSON). `gui/test_terminaux.py` : clé `console` conservée.
- **Graphiques** (`gui/test_gui.py`) : la console à ses quatre places sans relancer un shell ; un terminal à la fois sur le côté ; mosaïque de deux à quatre ; page pleine ; ranger et rouvrir ; agrandir ; tirer le point ; fermer avec confirmation ; réouverture ; réglage unique de notification.
- **Contrat modifié** : `EssaiDuPlanDeTravail`, `EssaiDePlusieursTerminaux`, `EssaiDesOngletsDuTerminal` et `test_le_bouton_du_bandeau_ouvre_son_menu` visaient un panneau par terminal ; ils visent la console. Les propriétés vérifiées restent (INV-TERM-01, INV-TERM-03, INV-PLAN-01).

---

## 9. Décision D32

Voir DECISIONS.md. Invariants : INV-TERM-04 (nouveau), INV-TERM-01, INV-TERM-03 et INV-PLAN-01 (prolongés).

---

## 10. Hors de 8.5

- Glisser une tuile pour l’échanger avec une autre ; plus de quatre terminaux sur la page.
- Raccourci clavier pour la console ; renommer un terminal à la main.
- Diffuser une commande à plusieurs terminaux ; reprendre une sortie après réouverture.

---

## 11. Retouches (8.5b, retours de l’utilisateur du 16 novembre 2026)

Retour : « quand je redimensionne les terminaux, ça fait des glitchs beiges du fond sur les icônes (la croix, le bouton réduire…), et l’animation n’est pas très fluide, elle lague un peu ; il y a plein de petits détails qui ne sont pas parfaits ».

**Constats, mesurés avant de corriger :**

- **Éclairs beiges.** Le serveur peint le fond de fenêtre d’un widget dès qu’il est découvert, avant que Tk le redessine. Un widget ttk n’a pas de fond propre : il montre celui de son premier ancêtre classique (expérience : un cadre ttk bleu déplacé dans un cadre classique rouge laisse voir du rouge). La tuile et son en-tête étaient des cadres ttk dans une zone au fond du papier.
- **Saccades.** Un pas du glisser coûtait 100 ms sous Xvfb et 150 ms sous KWin virtuel, entièrement dans le redessin : ttk remplit un élément image en répétant le milieu de son image, et celui du fond de tuile faisait 6 pixels, soit des milliers de copies par tuile et par pas. Avec un milieu large : 22 ms. Le texte des terminaux et le code Python n’y sont pour rien (moins de 1 ms).

**Corrections :**

1. **La tuile est un cadre classique au fond du terminal**, comme son en-tête et sa ligne de saisie ; ses quatre coins arrondis sont de petites images, son liseré (lavande quand elle est active) est sa bordure. Plus aucun fond de papier sous un widget d’une tuile, et plus aucune image à étirer.
2. **Les grandes surfaces du thème** (cadres des zones de texte, cartes, champs) ont une image au milieu large : un redessin en quelques copies.
3. **Pendant le glisser**, seules les tuiles, les poignées et le point bougent : la rangée de la console n’est pas relue, rien n’est écrit avant le lâcher.

**Mesure après correction** (quatre terminaux de 400 lignes, 60 pas, fenêtre de 1400 × 860) : médiane de 18 ms par pas sous Xvfb (103 avant) ; sous KWin virtuel, dont le rendu logiciel rend la mesure bruitée, 55 à 85 ms selon le passage (149 avant). Ce qui reste se partage entre le texte des quatre sorties, les en-têtes et la saisie, sans poste dominant.

**Détails :**

- page sans terminal : « Aucun terminal sur la page. » et un bouton « Nouveau terminal » ;
- le bouton rond `>_` de l’en-tête devient une pastille de papier quand la console est visible ;
- « Notifier après » : la valeur et ses boutons − et +, à la hauteur des autres boutons (plus de champ de saisie) ;
- les pastilles et la rangée se mettent à jour sur place, sans être reconstruites, quand seul un nom ou un état change ;
- le point d’état d’une commande en cours pulse au rythme de celui de la barre d’état (700 ms), et celui de la barre d’état s’atténue au lieu de disparaître ;
- pictogrammes des tuiles un peu plus grands ; Ctrl+L efface la sortie ;
- l’indication du journal ne déborde plus (« tirer la poignée »).
