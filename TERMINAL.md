# Terminal intégré — conception de l’incrément 6.7 (moteur et système)

Version 0.3 — 10 novembre 2026 — **validée par l’utilisateur le 10 novembre 2026** ; une décision nouvelle (D27, section 8), avec D28 (PLAN-DE-TRAVAIL.md) ; **implémentée** (6.7a moteur et notifications, 6.7c panneau ; `v0.6.0-alpha.7`) ; précisions de mise en œuvre : section 11.
- Répond à R5, précisé par l’utilisateur le 10 novembre 2026 :
  - « une simple intégration du shell système dans le logiciel, pour éviter d’avoir un shell ouvert à côté » ;
  - un shell redimensionnable, « assez libre, et flottant », qu’on peut aussi **incruster dans l’interface, dans un coin en haut** ;
  - qui **notifie la fin d’une compilation**.
- **Premier panneau du plan de travail modulaire** (PLAN-DE-TRAVAIL.md, D28) : ancré, flottant ou masqué.
- **Périmètre : le moteur et le système.** La partie visuelle (direction à la manière de Liquid Glass) viendra dans un second temps, avec le designer d’interface (section 9).

Historique du document :
- version 0.1 : panneau fixe ;
- version 0.2 : fenêtre flottante seule ;
- version 0.3 : panneau ancrable ou flottant, fin de commande et notifications.

La section 10 regroupe les points validés.

---

## 1. Ce que l’utilisateur obtient

**Un panneau « Terminal »** du plan de travail :
- **Par défaut**, il est ancré **en haut de la zone droite**, le coin en haut à droite de la fenêtre principale.
- **Il peut changer de place** à tout moment, par son bandeau (PLAN-DE-TRAVAIL.md, section 1) :
  - être déplacé dans une autre zone (haut, bas, gauche) ;
  - être **détaché en fenêtre flottante**, librement déplaçable et redimensionnable, éventuellement au premier plan ;
  - être masqué.

  Il garde sa place d’une session à l’autre.
- **Changer de place ne coupe pas le shell** : ni l’ancrer, ni le détacher, ni le masquer. Une compilation en cours continue, et la sortie est conservée.
- **Taille transmise au shell** : à chaque redimensionnement (séparation déplacée, fenêtre flottante agrandie), la taille du pseudo-terminal suit (colonnes et lignes d’après la police). La sortie de `ls`, `git` ou du compilateur s’adapte à la largeur.

**Le shell** :
- **C’est votre shell** (`$SHELL`, sinon `/bin/bash`), lancé à la première ouverture du panneau et conservé pendant la session : `cd`, variables, alias et historique persistent.
- **Dossier de départ** : le répertoire de lancement du logiciel.
- **Saisie** : une ligne de saisie, Entrée pour envoyer, ↑ et ↓ pour l’historique de la session.
- **Interruption** : Ctrl+C ou « Interrompre » arrête la commande en cours, pas le shell.
- **`memoire` vise la même base que la fenêtre** : `PATH` et `MEMOIRE_BASE`.

**Pas un émulateur complet** :
- les programmes plein écran (`vim`, `less`, `top`) ne s’affichent pas correctement ;
- `TERM=dumb`, `PAGER=cat` et `GIT_PAGER=cat` sont posés ;
- les couleurs ANSI sont retirées ;
- les programmes ligne à ligne fonctionnent.

**Fin de commande et notification** (section 4) : quand une commande longue se termine (10 secondes ou plus par défaut) et que vous regardez ailleurs, une **notification du bureau** l’annonce, avec sa réussite ou son échec et sa durée :

```
memoire — commande terminée
make && ./run-tests — échec (code 2) en 2 min 13 s
```

Le bandeau du panneau le signale aussi (« ● terminé, code 2 ») jusqu’à ce que vous reveniez au terminal.

---

## 2. Architecture

```
gui/terminal_moteur.py   moteur, aucune importation de tkinter
  Session      shell sous pseudo-terminal : démarrer, écrire, interrompre, redimensionner,
               lire ce qui est disponible, écho actif ?, en vie ?, arrêter
  Tampon       texte à afficher : UTF-8 incrémental, ANSI retiré, retours chariot, limite ;
               repère et retire les marques de fin de commande (section 4) ; rend les
               changements à appliquer et les événements « commande terminée »
  Historique   commandes saisies, ↑ ↓
  environnement(…), geometrie(…)
gui/notification.py      message de fin de commande (pur) et envoi au bureau (D-Bus, sinon
                         notify-send, sinon rien)
gui/terminal.py          contenu du panneau Terminal (Tkinter) : affichage, saisie, bandeau
gui/plan_de_travail.py, gui/panneaux.py   système de panneaux (PLAN-DE-TRAVAIL.md)
```

Le moteur ne connaît ni l’affichage ni le système de panneaux. Une future vue, dessinée avec le designer, peut le reprendre tel quel.

---

## 3. Le moteur

**Session :**
- `pty.fork` lance le shell dans son propre groupe de processus, dans le dossier de départ, avec l’environnement de la section 1 ; la vue lit la sortie sans bloquer (`createfilehandler`, sinon toutes les 30 ms).
- `ecrire(texte)` envoie une ligne et note l’heure de l’envoi.
- `interrompre()` écrit le caractère d’interruption (Ctrl+C) : `SIGINT` à la commande au premier plan, le shell reste en vie.
- `redimensionner(colonnes, lignes)` règle la taille du pseudo-terminal (`TIOCSWINSZ`, donc `SIGWINCH`).
- `echo_actif()` lit l’attribut d’écho du pseudo-terminal : sans écho (mot de passe), la saisie est masquée et n’entre pas dans l’historique.
- `arreter()` envoie `SIGHUP` au groupe du shell, puis `SIGKILL` après une seconde ; aucun processus ne reste.
- `en_vie()` est faux après `exit` : le panneau affiche « [shell terminé, code N] » et propose de relancer.

**Tampon :**
- décodage UTF-8 incrémental (un caractère coupé entre deux lectures n’est pas corrompu, un octet invalide devient `�`) ;
- séquences ANSI retirées, même coupées entre deux lectures ;
- `\r\n` donne une fin de ligne ; `\r` seul réécrit la ligne en cours ; le retour arrière efface ;
- au plus 5 000 lignes ;
- changements rendus à la vue.

**Historique** : sans doublon consécutif ni ligne vide ; saisie en cours conservée pendant la navigation ; saisie sans écho exclue.

**Géométrie** : colonnes et lignes d’après la taille de la zone en pixels et celle d’un caractère de la police à chasse fixe ; au moins 40 colonnes et 6 lignes.

---

## 4. Fin de commande et notification

**Détection, pour bash :**
- Le shell est lancé avec un fichier de démarrage propre au logiciel (`bash --rcfile`). Ce fichier :
  - charge d’abord votre `~/.bashrc` s’il existe : vos alias, votre invite et votre configuration restent les vôtres ;
  - ajoute à `PROMPT_COMMAND` une fonction qui, à chaque retour de l’invite, écrit une marque invisible : la séquence `ESC ] 7771 ; fin ; <code> BEL`, où `<code>` est le code de retour de la dernière commande.
- Le tampon repère cette marque, la retire de l’affichage, et rend l’événement « commande terminée (code) ». Avec l’heure d’envoi notée par la session, cela donne la durée.
- La première invite, qui ne termine aucune commande, n’émet pas d’événement.
- **Fichier de démarrage** : écrit dans un dossier temporaire privé (droits 0700, fichier 0600), il ne contient aucune donnée de l’utilisateur, seulement la fonction de la marque, et il est supprimé à l’arrêt du shell.
- **Autres shells** (zsh, fish) : le terminal fonctionne, mais sans détection de fin de commande. Le bandeau indique « fin de commande non suivie pour ce shell ».

**Quand notifier :**
- la commande a duré au moins **10 secondes** (seuil réglable dans le panneau) ;
- **et** vous ne regardez pas le terminal : le panneau est masqué, ou la fenêtre qui le contient n’a pas le focus.

**Comment notifier :**
- **notification du bureau** par le service standard `org.freedesktop.Notifications` (D-Bus, via `gdbus`), sinon `notify-send`, sinon aucune ; jamais d’erreur pour l’utilisateur si aucune n’est disponible ;
- **signal dans le panneau** : le bandeau garde le marqueur « ● terminé, code N » jusqu’à ce que le terminal soit regardé ;
- le texte (titre, réussite ou échec, code, durée lisible, commande tronquée à 80 caractères) est une **fonction pure**, testée.

**Ce que l’on ne fait pas** : analyser la sortie pour deviner une fin de compilation. Toute commande longue est annoncée de la même façon, qu’elle compile, teste ou copie.

---

## 5. Sécurité

- Le terminal a les droits de votre shell, ni plus ni moins qu’un terminal ouvert à côté.
- Saisie masquée sans écho, jamais dans l’historique.
- Rien d’autre n’est écrit sur le disque :
  - le fichier de démarrage temporaire de la section 4, supprimé à l’arrêt ;
  - la disposition des panneaux (PLAN-DE-TRAVAIL.md, section 3), qui ne contient ni commande ni sortie.
- Une notification montre la commande, tronquée, sur le bureau : éviter d’y saisir un secret en clair. Ce risque est le même que dans un terminal ordinaire.
- Le shell n’est lancé qu’à la première ouverture du panneau. Le panneau masqué par défaut ne lance rien tant qu’il n’a pas été ouvert.

---

## 6. Découpage (terminal et plan de travail)

| Étape | Contenu |
|---|---|
| 6.7a | `gui/terminal_moteur.py` (session, tampon avec marques de fin, historique, environnement, géométrie) et `gui/notification.py` ; tests sans affichage, dont une session réelle ; `check.sh` |
| 6.7b | `gui/plan_de_travail.py` : modèle de la disposition et sa conservation ; tests sans affichage |
| 6.7c | `gui/panneaux.py` et `gui/terminal.py` : zones, bandeaux, menu « Panneaux », ancré ↔ flottant, panneau Terminal (ancré en haut à droite par défaut), notification ; essais sous `xvfb-run` ; documents ; contrôle complet ; livraison `v0.6.0-alpha.7` |

Aucun changement du noyau, du stockage ni de la ligne de commande : l’aide figée reste à 47 commandes.

---

## 7. Tests

- **Moteur, sans affichage** :
  - tampon : ANSI, UTF-8 coupés, `\r`, retour arrière, limite ; marque de fin repérée, retirée, même coupée entre deux lectures ; première invite sans événement ;
  - historique ; environnement ; géométrie ;
  - session réelle avec bash :
    - `cd` persistant ;
    - `memoire statut` sur la base donnée ;
    - `stty size` après `redimensionner` ;
    - interruption d’un `sleep 30` ;
    - `false` puis `true` donnent les codes 1 et 0 ;
    - durée mesurée ;
    - votre `~/.bashrc` chargé (essai avec un `HOME` d’essai) ;
    - fichier de démarrage supprimé à l’arrêt ;
    - aucun processus restant.
- **Notification, sans affichage** :
  - texte (réussite, échec, durées en secondes, minutes, heures, commande tronquée) ;
  - règle « durée ≥ seuil et terminal non regardé » ;
  - envoi par une commande de remplacement, vérifiée sans toucher au vrai bureau ;
  - aucune erreur sans service de notification.
- **Fenêtre sous `xvfb-run`** :
  - le panneau s’ouvre ancré en haut à droite ;
  - une commande s’exécute ;
  - détacher puis rattacher sans relancer le shell (même processus) ;
  - redimensionner change `stty size` ;
  - une commande longue terminée panneau masqué envoie une notification (envoi remplacé) et marque le bandeau.

---

## 8. Décision D27

**D27 — Terminal intégré : le shell de l’utilisateur sous pseudo-terminal, premier panneau du plan de travail, fin de commande signalée par une marque d’invite**

- **Contexte** : R5 et ses précisions (shell du système dans le logiciel, flottant ou incrusté, notification de fin de compilation). Environnement de l’utilisateur : Linux, session Wayland, sans `xterm` ; `notify-send` et le service D-Bus des notifications présents.
- **Décision** :
  - le shell de l’utilisateur tourne dans un pseudo-terminal (bibliothèque standard de Python) ;
  - le moteur est séparé de la vue ;
  - le terminal est un panneau du plan de travail (D28), ancré en haut à droite par défaut ;
  - la fin de chaque commande est repérée par une marque invisible ajoutée à `PROMPT_COMMAND` (bash), après le chargement de `~/.bashrc` ;
  - une commande longue terminée hors de la vue est notifiée par le service de notifications du bureau ;
  - pas d’émulateur complet.
- **Options écartées** :
  - **`xterm -into`** : `xterm` absent, installation par `rpm-ostree` et redémarrage, dépendance à X11 ;
  - **émulateur VT100 complet** : hors de proportion ;
  - **mini-shell propre** : moins utile que le vrai shell ;
  - **fin de commande devinée** d’après la sortie ou le retour de l’invite : peu fiable, et sans code de retour ;
  - **commande enveloppée** (`cmd; echo $?`) : visible dans l’historique du shell et dans l’affichage ;
  - **bibliothèque Python de notifications** : dépendance inutile, D-Bus et `notify-send` suffisent.
- **Conséquences** :
  - aucune dépendance nouvelle ;
  - Linux et macOS seulement ;
  - fin de commande suivie pour bash seulement ;
  - un fichier de démarrage temporaire par session de shell.

---

## 9. Phase visuelle, plus tard

- Direction à la manière de Liquid Glass (translucidité, flou, coins arrondis, ombre, animations) et interactions du plan de travail (glisser-déposer des panneaux), avec le designer d’interface.
- Tkinter sait rendre une fenêtre uniformément translucide et sans décoration, mais ni flouter ni arrondir. Si ce rendu est voulu, la phase visuelle choisira peut-être une autre bibliothèque (PySide6, alternative notée dans D25, dépendance à justifier au titre de D14). Le moteur du terminal et le modèle du plan de travail sont séparés de leur réalisation pour cette raison.
- Couleurs ANSI, onglets, complétion par Tab, programmes plein écran : à rediscuter alors.

---

## 10. Points validés le 10 novembre 2026

1. **Le terminal est un panneau** du plan de travail : ancré en haut à droite par défaut, ou ailleurs, ou flottant et redimensionnable, ou masqué, sans jamais couper le shell ; taille transmise au shell.
2. **Le vrai shell** sous pseudo-terminal, moteur séparé de la vue, **pas d’émulateur complet** en 6.7.
3. **Fin de commande** par une marque d’invite ajoutée après votre `~/.bashrc` (bash seulement) ; **notification du bureau** pour une commande d’au moins 10 secondes terminée quand vous ne regardez pas le terminal, et marqueur dans le bandeau.
4. **`memoire` vise la base de la fenêtre** ; saisie masquée sans écho ; un fichier de démarrage temporaire, supprimé à l’arrêt.
5. **Décision D27**, avec D28 (PLAN-DE-TRAVAIL.md) ; phase visuelle et choix éventuel d’une autre bibliothèque renvoyés au designer.

---

## 11. Précisions apportées à la mise en œuvre

1. **bash sans readline** (`--noediting`) : la saisie est celle du panneau ; readline coupe l’écho du terminal à chaque invite, ce qui empêcherait de reconnaître une saisie de mot de passe. La complétion par Tab de bash n’existe donc pas (déjà hors de 6.7).
2. **Invite initiale** : la première marque de fin, celle de l’invite initiale, n’est jamais attribuée à une commande, même envoyée avant qu’elle ne soit lue (course constatée par les essais : une commande saisie dès l’ouverture du panneau était annoncée terminée en 0 s).
3. **Lecture du shell toutes les 30 ms** (`after`) : `createfilehandler` n’existe pas avec un Tcl à fils d’exécution, ce qui est le cas courant.
4. **Mise en page du panneau** : la saisie occupe une rangée sur toute la largeur, les réglages (Interrompre, Effacer, « Notifier après N s ») une seconde rangée ; le bandeau place ses actions avant le titre, pour qu’aucune ne soit coupée quand le panneau est étroit.
5. **Marqueur du bandeau** : posé quand une notification part ; effacé quand le panneau est réaffiché ou que sa saisie prend le focus.
6. **Une disposition conservée avec le terminal visible** le rouvre à l’ouverture du logiciel, et lance donc le shell : c’est le choix laissé par l’utilisateur à la fermeture précédente.
7. **Shell terminé** (`exit`) : le panneau l’indique ; Entrée relance un shell neuf.
8. **Filet de sécurité** (6.8) : à la sortie de Python, toute session encore ouverte est arrêtée et son fichier de démarrage supprimé (`atexit`), même si `arreter` n’a pas été appelé. Ajouté par précaution. **Correction du 10 novembre 2026** : le dossier de démarrage d’abord pris pour une fuite des essais était, selon toute vraisemblance, celui d’une session du logiciel ouverte au même moment par l’utilisateur (`make`), dont le terminal était en service ; aucune fuite des essais n’a été établie.
