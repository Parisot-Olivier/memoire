# Plan de travail modulaire — conception du système de panneaux (incrément 6.7)

Version 0.1 — 10 novembre 2026 — **validée par l’utilisateur le 10 novembre 2026** ; une décision nouvelle (D28, section 6) ; **implémentée** (6.7b modèle, 6.7c réalisation ; `v0.6.0-alpha.7`) ; précisions de mise en œuvre : section 9.
- Répond à la vision donnée par l’utilisateur le 10 novembre 2026 : « avoir le plan de travail le plus modulaire possible (par exemple Vivado : on peut créer des panneaux, redimensionner les fenêtres, etc.) ».
- **Périmètre : le système**, c’est-à-dire le modèle des panneaux, l’ancrage, le flottement, la disposition et sa conservation. **L’apparence** (direction visuelle, glisser-déposer, animations) sera conçue avec un designer d’interface, dans un second temps.
- **Premier panneau** : le terminal (TERMINAL.md).

La section 8 regroupe les points validés.

---

## 1. Ce que l’utilisateur obtient

La fenêtre principale garde sa **zone centrale** (liste des commandes, formulaire, sortie). Autour d’elle, quatre **zones d’ancrage** (haut, bas, gauche, droite) reçoivent des **panneaux** :

```
┌──────────────────────────────────────────────────────────────┐
│ barre du haut : Base … [Statut] [Terminal] [Panneaux ▾]       │
├──────────────┬───────────────────────────────┬───────────────┤
│              │                               │ ┌ Terminal ─┐ │
│  commandes   │   formulaire                  │ │ $ make    │ │  ← zone droite,
│              │                               │ │ …         │ │    en haut
│              │   sortie                      │ └───────────┘ │
│              │                               │ (autre panneau)│
├──────────────┴───────────────────────────────┴───────────────┤
│ zone bas (vide : n’apparaît pas)                              │
└──────────────────────────────────────────────────────────────┘
```

Chaque panneau peut être, au choix :
- **ancré** dans une zone, à un rang (en haut, en dessous…). La zone n’existe à l’écran que si elle contient un panneau. Les séparations entre panneaux, et entre zones et zone centrale, se **redimensionnent** à la souris ;
- **flottant** : une fenêtre à part, déplaçable et redimensionnable librement, avec l’option « Au premier plan » ;
- **masqué** : retiré de l’écran, son contenu et son activité conservés (un shell continue).

Chaque panneau a un **bandeau** avec son titre et ses actions :
- « Détacher » ou « Ancrer » ;
- « Plan ▾ », pour un panneau flottant (ajouté en 6.8, section 10) : « Toujours au premier plan », « Mettre devant », « Mettre derrière » ;
- « Zone ▸ haut, bas, gauche, droite » ;
- « Monter » et « Descendre » dans sa zone ;
- « Masquer ».

Le menu **« Panneaux ▾ »** de la barre du haut liste les panneaux et réaffiche un panneau masqué.

**La disposition se conserve** : à la fermeture, l’état de chaque panneau (zone, rang, tailles, position flottante, premier plan) est écrit dans la configuration de l’utilisateur, puis restauré à l’ouverture. « Panneaux ▸ Disposition par défaut » y revient.

---

## 2. Architecture : un modèle sans affichage, une réalisation Tkinter

```
gui/plan_de_travail.py   modèle : aucune importation de tkinter
  Disposition    état de chaque panneau : ancré (zone, rang, part de la zone), flottant
                 (géométrie, premier plan), masqué ; opérations : ancrer, détacher, déplacer,
                 monter, descendre, masquer, montrer, redimensionner ; lecture et écriture JSON,
                 validation, disposition par défaut
gui/panneaux.py          réalisation Tkinter : zones (PanedWindow), bandeaux, menus,
                         passage ancré ↔ flottant
```

- **Une seule source de vérité** : le modèle. Toute action de l’utilisateur devient une opération sur la `Disposition`, puis la réalisation se met à jour. Le modèle se teste sans affichage, et la future interface dessinée avec le designer pourra réutiliser le modèle tel quel, même sur une autre bibliothèque graphique.
- **Passage ancré ↔ flottant sans recréer le panneau** : Tk 9 sait faire d’un cadre une fenêtre de premier niveau (`wm manage`) et l’inverse (`wm forget`). Le contenu, le shell du terminal par exemple, n’est ni détruit ni relancé. Essai réalisé le 10 novembre 2026 sous Tk 9.0.2.
- **Un panneau** est décrit par :
  - un identifiant stable (`terminal`) ;
  - un titre ;
  - une fonction qui construit son contenu dans un cadre donné ;
  - des actions propres, facultatives.

  Ajouter un panneau, c’est l’inscrire dans une table, comme une commande dans `COMMANDES`.

---

## 3. Conservation de la disposition

- **Fichier** : `$XDG_CONFIG_HOME/memoire/plan-de-travail.json`, sinon `~/.config/memoire/plan-de-travail.json`. Il est propre à l’utilisateur, **hors du dépôt** et hors de la base : la disposition n’est pas une donnée du projet.
- **Contenu** : une version, puis, par panneau, son état et ses paramètres. Exemple :

```json
{"version": 1,
 "panneaux": {"terminal": {"etat": "ancre", "zone": "droite", "rang": 0, "part": 0.45,
                           "flottant": {"geometrie": "720x420+1200+80", "premier_plan": true}}},
 "zones": {"droite": 0.30, "bas": 0.25, "gauche": 0.20, "haut": 0.25}}
```

- **Écriture atomique** (fichier temporaire puis renommage), à la fermeture et après chaque changement de disposition.
- **Lecture tolérante** : un fichier absent, illisible, d’une version inconnue ou incohérent donne la disposition par défaut, avec un message dans la sortie. Un panneau inconnu est ignoré ; un panneau nouveau prend sa place par défaut. Une géométrie flottante hors des écrans actuels est ramenée dans l’écran principal.

---

## 4. Ce qui reste hors de 6.7

- **Les autres vues comme panneaux** (visionneur, vue locale, sortie, liste des commandes) : elles restent des fenêtres en 6.7. Les convertir est un incrément suivant, une fois le système éprouvé par le terminal et l’apparence décidée avec le designer.
- **Glisser-déposer** d’un panneau vers une zone (comme Vivado), zones imbriquées et onglets : apparence et interaction, avec le designer.
- **Plusieurs dispositions nommées** (« compilation », « lecture »).

---

## 5. Tests

- **Modèle, sans affichage** (`gui/test_plan_de_travail.py`, lancé par `check.sh`) :
  - chaque opération ;
  - rangs recalculés ;
  - zone vide ;
  - aller-retour JSON ;
  - fichier absent, illisible, de version inconnue, incohérent ;
  - panneau inconnu ou nouveau ;
  - géométrie ramenée dans l’écran ;
  - écriture atomique.
- **Réalisation sous `xvfb-run`** :
  - un panneau ancré à droite, détaché, rattaché en bas, masqué puis réaffiché, sans que son contenu soit recréé (même widget, même shell) ;
  - séparations redimensionnées ;
  - disposition conservée entre deux ouvertures de l’application d’essai.

---

## 6. Décision D28

**D28 — Plan de travail modulaire : panneaux ancrables ou flottants, modèle séparé de la réalisation, disposition propre à l’utilisateur**

- **Contexte** : l’utilisateur veut un plan de travail aussi modulaire que possible, à la manière de Vivado. L’apparence sera conçue plus tard avec un designer d’interface.
- **Décision** :
  - autour d’une zone centrale fixe, quatre zones d’ancrage reçoivent des panneaux qui peuvent aussi flotter ou être masqués ;
  - un modèle sans affichage (`Disposition`) est la seule source de vérité ; la réalisation Tkinter passe un panneau d’ancré à flottant sans le recréer (`wm manage`, `wm forget`) ;
  - la disposition est conservée dans la configuration de l’utilisateur (XDG), hors du dépôt.
- **Options écartées** :
  - **fenêtres séparées seulement**, sans ancrage : ni modulaire ni rangé ;
  - **bibliothèque d’ancrage existante** (Qt Advanced Docking System, PySide6) : changement de bibliothèque graphique et dépendance lourde au titre de D14, avant d’avoir le besoin visuel précis ; le modèle séparé garde cette voie ouverte ;
  - **disposition stockée dans la base** : elle n’est pas une donnée du projet et changerait l’historique pour une question d’affichage.
- **Conséquences** :
  - le terminal est le premier panneau ;
  - les autres vues suivront ;
  - un fichier de configuration apparaît dans `~/.config/memoire/`.

---

## 7. Découpage commun avec le terminal

Voir TERMINAL.md, section 6.

---

## 8. Points validés le 10 novembre 2026

1. **Zone centrale fixe et quatre zones d’ancrage** (haut, bas, gauche, droite), chacune empilant des panneaux redimensionnables ; un panneau ancré, flottant ou masqué.
2. **Actions par le bandeau et le menu « Panneaux »** en 6.7 ; le glisser-déposer viendra avec le designer.
3. **Modèle séparé de la réalisation**, seule source de vérité, réutilisable par une future interface.
4. **Disposition conservée** dans `~/.config/memoire/plan-de-travail.json`, hors du dépôt, avec retour à la disposition par défaut.
5. **Le terminal seul en panneau en 6.7** ; les autres vues ensuite.
6. **Décision D28.**

---

## 9. Précisions apportées à la mise en œuvre

1. **Format du fichier** : une entrée par panneau, à plat (`etat`, `zone`, `rang`, `part`, `geometrie`, `premier_plan`, `retour`), plutôt que l’objet `flottant` imbriqué de l’exemple de la section 3 ; `retour` est l’état rendu par « montrer » à un panneau masqué. Un panneau neuf est masqué à sa place par défaut.
2. **Placement d’un panneau ancré** : chaque panneau est un cadre enfant de la fenêtre principale ; ancré, il est placé dans une case de sa zone par `pack -in`, que Tk permet puisque la case descend de son parent. Il passe ainsi d’une zone à l’autre, et de flottant à ancré, sans jamais être recréé. Un `PanedWindow` n’accepte, lui, que ses enfants ou ceux de son parent.
3. **Zones haut et bas** : leurs panneaux se rangent côte à côte ; à gauche et à droite, les uns sous les autres.
4. **Menu « Panneaux »** : une entrée par panneau, qui l’affiche ou le masque, puis « Disposition par défaut ». Un bouton « Terminal » dans la barre du haut fait de même pour le terminal.
5. **Enregistrement** : après chaque action, à la fin d’un déplacement d’une séparation, une demi-seconde après le dernier mouvement d’une fenêtre flottante, et à la fermeture ; un enregistrement impossible est signalé dans la zone de sortie, sans erreur bloquante.
6. **Fenêtre principale** agrandie à 1400 × 800 par défaut, champ de la base raccourci : la barre du haut et un panneau ancré à droite tiennent sans être coupés.

---

## 10. Ordre des fenêtres flottantes (incrément 6.8)

Demande de l’utilisateur du 10 novembre 2026 : « que le terminal reste au premier plan, même si je touche à des boutons du logiciel ; je voudrais que ce soit moi qui décide (mettre devant, mettre derrière) ». Constat : détaché, le terminal passait derrière la fenêtre principale dès qu’on cliquait sur celle-ci. L’option « Au premier plan », prévue à la section 1 et tenue par le modèle (`premier_plan`), n’avait pas de commande dans le bandeau.

Dans le bandeau d’un panneau flottant, un menu **« Plan ▾ »**, inactif quand le panneau est ancré, propose :
- **« Toujours au premier plan »** (case à cocher) : la fenêtre flottante reste devant les autres fenêtres, la fenêtre principale du logiciel comprise, même quand on clique dans celle-ci. Le choix fait partie de la disposition, conservé d’une session à l’autre. Il est désactivé par défaut.
- **« Mettre devant »** : amène une fois la fenêtre devant les autres.
- **« Mettre derrière »** : la place une fois derrière la fenêtre principale du logiciel, et décoche « Toujours au premier plan ».

C’est toujours l’utilisateur qui décide : aucune de ces actions n’est automatique.

**Moyens** (implémentés en 6.8, `v0.6.0-alpha.8`) :
- `wm attributes -topmost` pour « toujours au premier plan » ;
- `lower` sous la fenêtre principale pour « mettre derrière » ;
- pour « mettre devant », retirer puis réafficher la fenêtre (`wm withdraw`, `wm deiconify`, à la même géométrie). KWin ignore la demande `raise` d’une application, par prévention du vol de focus ; constaté sous KWin virtuel le 10 novembre 2026. Cela produit un bref clignotement.

**Limite** : sous Wayland, Tk passe par XWayland et c’est le gestionnaire de fenêtres (KWin, dans l’environnement de développement) qui applique ces demandes. KWin respecte « toujours au premier plan » et l’ordre demandé par l’application qui a le focus, mais un autre gestionnaire pourrait les ignorer.

**Tests** :
- **Partout** : menu inactif pour un panneau ancré ; choix tenu par la disposition et conservé ; case décochée par « mettre derrière ».
- **Avec un gestionnaire de fenêtres** : `check.sh` lance les essais graphiques sous **KWin virtuel avec XWayland** (`tools/essais-sous-kwin.sh`) quand KWin est installé, sinon sous Xvfb, où cet essai est sauté. L’essai, qui attend que chaque demande soit appliquée, vérifie :
  - le cas signalé, un clic dans la fenêtre principale : le terminal passe derrière sans l’option, et reste devant avec ;
  - l’attribut `-topmost` ;
  - « mettre derrière » et « mettre devant ».

  Sous Xvfb seul, Tk ne relit que l’état appliqué par un gestionnaire : il n’y a rien à vérifier.

