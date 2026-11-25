# Plusieurs terminaux — conception de l’incrément 6.9

Version 0.1 — 10 novembre 2026 — **validée par l’utilisateur le 10 novembre 2026** ; une décision nouvelle (D29, section 7) ; **implémentée** (6.9a moteur, nommage, registre ; 6.9b interface ; `v0.6.0-alpha.9`) ; précisions de mise en œuvre : section 10.
- **Demande de l’utilisateur** du 10 novembre 2026 : « gérer le multi-terminal, avec un menu que je peux sélectionner et aussi placer sur l’interface principale, qui me donne l’ensemble des terminaux ouverts, qui les nomme (pas par leur identifiant, mais par la commande qu’ils sont en train d’exécuter), et qui me permet de les mettre devant ou derrière facilement ».
- **Prolonge** TERMINAL.md (D27) et PLAN-DE-TRAVAIL.md (D28, section 10 pour devant et derrière).
- **Depuis l’incrément 8.5 (CONSOLE.md, D32)** : les terminaux ne sont plus des panneaux séparés mais les tuiles d’une console unique ; le panneau « Terminaux », le menu « Terminaux ▾ » et les actions Devant et Derrière par terminal n’existent plus. Le nommage (section 1), la conservation des dossiers (section 2), le moteur (section 3) et la sécurité (section 5) restent.
- **Périmètre : le système.** L’apparence reste pour la phase avec le designer.

La section 9 regroupe les points validés.

---

## 1. Ce que l’utilisateur obtient

**Plusieurs terminaux**, chacun dans son propre panneau du plan de travail, avec son propre shell :
- chaque terminal peut être ancré, flottant ou masqué, indépendamment des autres ;
- **« Nouveau terminal »** en ouvre un de plus, ancré par défaut sous les terminaux de la zone droite ;
- **« Fermer »** arrête son shell et retire son panneau. Si une commande tourne encore, une confirmation est demandée.

**Chaque terminal est nommé par ce qu’il fait**, jamais par un identifiant :

| Situation | Nom affiché |
|---|---|
| une commande tourne | la commande, tronquée à 40 caractères : `make && ./run-tests` |
| le shell attend | le shell et son dossier : `bash — ~/projets/rtos` |
| la dernière commande a échoué, le shell attend | `bash — ~/projets/rtos` et un marqueur « ✗ code 2 » |
| le shell est terminé (`exit`) | `terminé (code 0)` |
| deux noms identiques | le second suivi de « (2) », le troisième de « (3) » |

Le nom se met à jour en direct :
- dans le bandeau du panneau ;
- dans le titre de sa fenêtre quand il flotte ;
- dans la liste et le menu ci-dessous.

**Le panneau « Terminaux »**, un panneau comme un autre (ancré où l’on veut, flottant ou masqué), donne l’ensemble des terminaux ouverts :

```
┌ Terminaux ──────────────────────────────────────────── [Nouveau terminal] ┐
│ ● make && ./run-tests      en cours depuis 2 min 13 s   flottant   [Afficher] [Devant] [Derrière] [Masquer] [Fermer] │
│ ○ bash — ~/projets/rtos    inactif                      droite     [Afficher]                     [Masquer] [Fermer] │
│ ✗ bash — ~/projets/rtos (2) dernière commande : code 2  masqué     [Afficher]                               [Fermer] │
└───────────────────────────────────────────────────────────────────────────┘
```

- **Une ligne par terminal**, avec :
  - son état : en cours (depuis combien de temps), inactif, dernière commande en échec, shell terminé ;
  - son nom ;
  - sa place : zone, flottant ou masqué.
- **« Afficher »** (ou double-clic sur la ligne) : montre le terminal s’il est masqué, le met devant s’il flotte, et place le curseur dans sa ligne de saisie.
- **« Devant »** et **« Derrière »** : seulement pour un terminal flottant (PLAN-DE-TRAVAIL.md, section 10).
- **« Masquer »** et **« Fermer »**.
- La liste se met à jour d’elle-même, au plus toutes les demi-secondes.

**Le menu « Terminaux ▾ »** de la barre du haut propose la même chose en bref :
- « Nouveau terminal » ;
- une entrée par terminal (nom et état), qui l’affiche et le met devant ;
- « Panneau des terminaux », qui montre ou masque ce panneau.

Le bouton « Terminal » actuel est remplacé par ce menu.

**Notification** (TERMINAL.md, section 4) : le texte indique la commande, comme aujourd’hui. Le terminal concerné garde son marqueur, et sa ligne dans le panneau « Terminaux » aussi.

---

## 2. Conservation entre deux sessions

À la fermeture du logiciel, la liste des terminaux ouverts est conservée avec la disposition : pour chacun, son numéro interne, le dernier dossier connu de son shell et sa place. À la réouverture :
- les terminaux sont **recréés à leur place** ;
- chacun démarre un **shell neuf dans son dernier dossier**, quand il devient visible ;
- un terminal masqué ne lance rien tant qu’on ne l’affiche pas.

Les commandes qui tournaient ne sont pas reprises, c’est impossible ; la sortie affichée non plus. Fichier : `~/.config/memoire/terminaux.json`, à côté de `plan-de-travail.json`.

---

## 3. Moteur (sans affichage)

**Dossier courant du shell** : la marque de fin de chaque commande (TERMINAL.md, section 4) porte désormais aussi le dossier, `ESC ] 7771 ; fin ; <code> ; <dossier> BEL`.
- Le tampon rend le dernier dossier connu en plus du code.
- L’ancienne forme, sans dossier, reste comprise.
- C’est ainsi que le nom suit un `cd`, et que le dossier est conservé.

**Suivi** : `SuiviDesCommandes` dit aussi quelle commande est **en cours** et depuis quand.

**Nommage**, dans un module pur `gui/terminaux.py`, testé sans affichage :
- `nom(etat_du_terminal)` applique le tableau de la section 1 :
  - dossier abrégé par `~` sous `$HOME` ;
  - commande tronquée à 40 caractères ;
- `noms(terminaux)` ajoute « (2) », « (3) » aux doublons, dans l’ordre de création ;
- `etat_lisible(...)` donne « en cours depuis 2 min 13 s », « inactif », « dernière commande : code 2 » ou « terminé (code 0) ».

**Registre des terminaux** (`Terminaux`, même module) :
- numéros internes jamais réemployés pendant une session ;
- création et fermeture ;
- lecture et écriture de `terminaux.json`, avec les mêmes règles de tolérance que la disposition (PLAN-DE-TRAVAIL.md, section 3) : un fichier invalide donne un seul terminal, masqué.

**Plan de travail** : les panneaux deviennent **dynamiques**.
- `Disposition.ajouter(nom, defaut)` et `Disposition.retirer(nom)`, avec renumérotation de la zone.
- Côté réalisation :
  - `PlanDeTravail.ajouter_panneau(...)` et `retirer_panneau(...)` en cours de session ;
  - `renommer(nom, titre)` met à jour bandeau, titre de fenêtre et menus.
- Retirer un panneau détruit son cadre, et l’arrêt du shell suit (INV-TERM-01).

---

## 4. Ce qui reste comme aujourd’hui

- **Chaque terminal** a tout ce que le terminal unique a aujourd’hui : historique, interruption, seuil de notification, saisie masquée, taille transmise.
- **Pas d’émulateur complet** (D27).
- **Fin de commande suivie pour bash seulement.** Pour un autre shell, le nom affiche la dernière commande envoyée, sans savoir si elle est finie.

---

## 5. Sécurité

- **Fermer un terminal dont une commande tourne** demande confirmation.
- **À la fermeture du logiciel**, tous les shells sont arrêtés, comme aujourd’hui.
- **`terminaux.json`** ne contient que des numéros, des dossiers et des places : aucune commande, aucune sortie.

---

## 6. Découpage et tests

| Étape | Contenu |
|---|---|
| 6.9a | moteur : dossier dans la marque de fin, commande en cours ; `gui/terminaux.py` (nommage, état lisible, registre, conservation) ; panneaux dynamiques dans le modèle du plan de travail ; tests sans affichage |
| 6.9b | réalisation : panneaux de terminaux créés et fermés, renommage en direct, panneau « Terminaux », menu « Terminaux ▾ » ; essais sous KWin virtuel ; documents ; contrôle complet ; livraison `v0.6.0-alpha.9` |

**Tests prévus :**
- **Sans affichage** :
  - marque avec dossier, coupée, ancienne forme ;
  - commande en cours ;
  - noms : chaque ligne du tableau, `~`, troncature, doublons ;
  - états lisibles ;
  - registre : numéros jamais réemployés, conservation et fichier invalide ;
  - panneaux ajoutés et retirés dans la disposition.
- **Sous KWin virtuel** :
  - trois terminaux, une commande longue dans l’un : la liste et le bandeau montrent la commande puis, après sa fin, `bash — <dossier>` ;
  - un `cd` change le nom ;
  - « Devant » et « Derrière » depuis la liste sur un terminal flottant ;
  - « Afficher » d’un terminal masqué ;
  - « Fermer » arrête le shell, avec confirmation si une commande tourne ;
  - réouverture : les trois terminaux à leur place, shells neufs dans leurs dossiers.

---

## 7. Décision D29

**D29 — Plusieurs terminaux : un panneau par terminal, créé à la demande, nommé par ce qu’il fait, géré depuis un panneau « Terminaux »**

- **Contexte** : demande de l’utilisateur (section 1) ; le plan de travail (D28) ne connaissait que des panneaux fixes, déclarés au démarrage.
- **Décision** :
  - chaque terminal est un panneau dynamique du plan de travail, avec son shell ;
  - son nom est calculé par une fonction pure, d’après la commande en cours, le shell et son dossier (transmis par la marque de fin) ;
  - un panneau « Terminaux », placé comme tout panneau, et un menu « Terminaux ▾ » listent les terminaux et permettent de les afficher, mettre devant ou derrière, masquer et fermer ;
  - la liste des terminaux et leurs dossiers sont conservés, et non leurs commandes ni leurs sorties.
- **Options écartées** :
  - **onglets dans un seul panneau** : on ne pourrait pas voir deux terminaux côte à côte, ni en faire flotter un seul ;
  - **nom choisi par l’utilisateur** : la demande est un nom qui dit ce que fait le terminal, sans effort ; un renommage manuel pourra venir plus tard ;
  - **commande en cours lue dans `/proc`** (processus au premier plan du terminal) : propre à Linux, et moins fidèle que la commande saisie, qui est connue.
- **Conséquences** :
  - panneaux dynamiques dans le modèle du plan de travail ;
  - un second fichier de configuration (`terminaux.json`) ;
  - la marque de fin porte le dossier.
- **Invariants** : INV-TERM-03 (le nom d’un terminal est celui de la commande en cours, sinon celui du shell et de son dossier, sans doublon dans la liste) ; INV-TERM-01 étendu à chaque terminal.

---

## 8. Hors de 6.9

- Renommer un terminal à la main.
- Disposer automatiquement plusieurs terminaux en grille.
- Diffuser une commande à plusieurs terminaux.
- Reprendre une sortie ou une commande après réouverture.

---

## 9. Points validés le 10 novembre 2026

1. **Un panneau par terminal**, ancré, flottant ou masqué indépendamment ; « Nouveau terminal » ancré par défaut sous les autres terminaux de la zone droite ; « Fermer » avec confirmation si une commande tourne.
2. **Le nommage** du tableau de la section 1 : la commande en cours, sinon `bash — dossier`, `terminé (code N)` pour un shell sorti, doublons numérotés.
3. **Le panneau « Terminaux »** (placé comme tout panneau) et **le menu « Terminaux ▾ »** (qui remplace le bouton « Terminal ») : Afficher, Devant, Derrière (terminaux flottants), Masquer, Fermer, Nouveau terminal.
4. **La conservation** : terminaux recréés à leur place, shells neufs dans leur dernier dossier, ni commande ni sortie reprises.
5. **Décision D29.**

---

## 10. Précisions apportées à la mise en œuvre

1. **Actions du panneau « Terminaux »** : elles portent sur la ligne choisie (Afficher, Devant, Derrière, Masquer, Fermer), plutôt que des boutons sur chaque ligne, qu’une liste Tkinter ne sait pas porter. Le double-clic sur une ligne fait « Afficher ». « Devant » et « Derrière » sont inactifs pour un terminal ancré ou masqué. « Nouveau » ouvre un terminal.
2. **Bandeau des panneaux sur deux rangées** : le titre, c’est-à-dire le nom d’un terminal qui change, et son marqueur sur toute la largeur, puis les actions. Sur une seule rangée, les noms étaient coupés par les boutons (capture d’écran).
3. **Noms internes** : le premier terminal est le panneau `terminal-1`, et non plus `terminal` (6.7, 6.8). Une disposition enregistrée par ces versions ne retrouve donc pas la place de l’ancien terminal unique : il reprend sa place par défaut, une fois.
4. **Dossier affiché sous `~`** même quand le dossier du shell est écrit par son chemin réel et `HOME` par un lien (par exemple un lien symbolique).
5. **Fenêtre principale détruite sans « fermer »** : la mise à jour périodique est annulée et les shells arrêtés quand même.
6. **Filet de sécurité de sortie** (TERMINAL.md, précision 8) : les sessions sont tenues par une référence forte jusqu’à leur arrêt, pour qu’aucune n’échappe à l’arrêt de sortie. Précaution seulement : le dossier de démarrage observé dans `/tmp` pendant les essais appartenait à une session du logiciel que l’utilisateur avait ouverte au même moment (`make`, terminal en service), et non aux essais ; 3 passages instrumentés n’ont montré aucune session restante après un essai.
7. **Mise à jour** : noms et liste toutes les 300 ms ; le menu « Terminaux ▾ » est reconstruit à chaque ouverture.

