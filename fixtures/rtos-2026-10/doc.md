* rôle du driver GPIO : réveiller les tâches selon la topologie de ses entrées-sorties

** question

voici pour l'instant mon code. je ne veux pas de correction, juste une évaluation conceptuelle du driver GPIO de ma carte : l'organisation qui transparaît de mes commentaires, les rôles que j'ai donnés à chaque fonction, et surtout si le partage des responsabilités entre le driver et le noyau tient la route pour un petit RTOS monocœur. le driver doit surtout réveiller les tâches qui attendent un événement sur une broche.

#include "common.h"
#include <stdint.h>
#include <stdbool.h>

//structures de contrôle

//événements reçus des handlers : un drapeau par broche, un port par ligne
static volatile bool gpio_irq_pending[16][8] = { false };

//broches pour lesquelles au moins une tâche attend
static volatile bool gpio_waiting_irq[16][8] = { false };

//ports effectivement présents sur la carte
static bool gpio_port_ready[16] = { false };

//adresse de base de chaque port (renseignée par init_driver)
static uint32_t gpio_port_base[16];

//attentes : une file de 8 tâches au plus par broche (puissance de 2, masque plutôt que modulo)
typedef struct {
    task_t *tasks[8];
    uint8_t head;
    uint8_t count;
} waiting_file_t;

static waiting_file_t waiting_files[16][8];

//arbre de réduction : un bit par port, levé si une broche du port a une attente
static uint16_t waiting_ports = 0;

//section critique : on masque les interruptions pendant la manipulation des files
static inline uint32_t enter_critical(void) {
    uint32_t primask = read_primask();
    disable_irq();
    return primask;
}

static inline void leave_critical(uint32_t primask) {
    write_primask(primask);
}

//préparation des périphériques : horloges, directions, masques d'interruption
void init_driver(void) {
    for (int port = 0; port < 16; port++) {
        gpio_port_ready[port] = port_exists(port);
        gpio_port_base[port] = port_base_address(port);
    }
}

//appelé par les handlers : mémorise l'événement, rien d'autre
void trigger_event(int port, int pin) {
    gpio_irq_pending[port][pin] = true;
}

//handler commun : lit le statut, acquitte exactement les bits traités, mémorise
static void gpio_port_handler(int port) {
    uint32_t status = reg_read(gpio_port_base[port] + GPIOMIS);
    reg_write(gpio_port_base[port] + GPIOICR, status);   //écrire 1 acquitte le bit
    for (int pin = 0; pin < 8; pin++) {
        if (status & (1u << pin)) {
            trigger_event(port, pin);
        }
    }
}

void gpio_port_a_handler(void) { gpio_port_handler(0); }
void gpio_port_b_handler(void) { gpio_port_handler(1); }
void gpio_port_c_handler(void) { gpio_port_handler(2); }

//enregistre l'attente d'une tâche sur une broche ; refuse si la file est pleine
bool set_wait_event(task_t *task, int port, int pin) {
    uint32_t saved = enter_critical();
    waiting_file_t *file = &waiting_files[port][pin];
    bool accepted = gpio_port_ready[port] && file->count < 8;
    if (accepted) {
        file->tasks[(file->head + file->count) & 7] = task;
        file->count++;
        gpio_waiting_irq[port][pin] = true;
        waiting_ports |= (uint16_t)(1u << port);
    }
    leave_critical(saved);
    return accepted;
}

//retire une tâche de la file d'une broche (délai dépassé, tâche supprimée)
bool cancel_wait_event(task_t *task, int port, int pin) {
    uint32_t saved = enter_critical();
    waiting_file_t *file = &waiting_files[port][pin];
    bool found = false;
    for (int k = 0; k < file->count; k++) {
        int index = (file->head + k) & 7;
        if (file->tasks[index] == task) {
            found = true;
        }
        if (found && k + 1 < file->count) {
            file->tasks[index] = file->tasks[(index + 1) & 7];
        }
    }
    if (found) {
        file->count--;
    }
    if (file->count == 0) {
        gpio_waiting_irq[port][pin] = false;
    }
    leave_critical(saved);
    return found;
}

//voir dans doc.md pourquoi cette decision
void wake_up(int port, int pin) {
    waiting_file_t *file = &waiting_files[port][pin];
    while (file->count > 0) {
        task_t *task = file->tasks[file->head];
        file->head = (file->head + 1) & 7;
        file->count--;
        scheduler_make_ready(task);
    }
    gpio_waiting_irq[port][pin] = false;
}

//dans tick() de engine.c, on attend une fonction nommée refresh()
void refresh() {
    for (int port = 0; port < 16; port++) {
        if (!(waiting_ports & (1u << port))) {
            continue;
        }
        for (int pin = 0; pin < 8; pin++) {
            if (gpio_irq_pending[port][pin]) {
                gpio_irq_pending[port][pin] = false;
                wake_up(port, pin);
            }
        }
    }
}

//à revoir si la période du tick change

** réponse

L’organisation qui se dessine est cohérente pour un petit RTOS : les interruptions collectent les événements, puis le noyau les prend en compte à un moment déterminé. Ton « driver » remplit cependant deux rôles à la fois : il décrit le matériel, et il décide de l’ordonnancement.

Je lis ton modèle de cette manière :

Élément	Rôle qui apparaît	Contenu	Remarque
table_pins	Décrire le matériel	broches valides par port	figé à la compilation
handlers	Identifier et acquitter	un handler par port	courts, sans blocage
trigger_event()	Mémoriser un événement	un drapeau par broche	écrit en interruption
set_wait_event()	Enregistrer une attente	tâche et broche	lu par le noyau
refresh()	Consommer les événements	parcourt les drapeaux	appelé à chaque tick

Quatre choses me paraissent solides :

* La séparation entre la réception d’une interruption et le traitement de ses conséquences : tu évites de faire tout le travail de réveil dans chaque handler.
* Un drapeau par broche, écrit en interruption et lu en dehors, qui garde le handler très court.
* Une table des broches valides, qui permet de refuser une attente sur une broche qui n’existe pas.
* Des handlers de moins de 5 µs, ce qui laisse de la marge sur un système à 16 ports.

Pour clarifier les responsabilités, je distinguerais trois couches. La première est la description du matériel (adresses, broches valides, masques) : elle ne change que lorsque la carte change. La deuxième est la gestion des événements (drapeaux, acquittement) : elle dépend du matériel, mais pas du noyau. La troisième est la politique de réveil (quelle tâche, dans quel ordre, avec quelle priorité) : elle appartient au noyau. Dans ton code, les deux premières couches sont bien séparées ; la troisième est répartie entre wake_up() et refresh(), ce qui rend la lecture moins claire.

Un mot sur les interruptions elles-mêmes : donne à tous les handlers GPIO la même priorité, et une priorité inférieure à celle de l’interruption du tick et de la liaison série. Ainsi, un événement de bouton ne retarde jamais le tick, et le noyau garde la maîtrise du temps. Si deux ports peuvent déclencher en même temps, l’ordre dans lequel ils sont servis n’a pas d’importance tant que chaque handler ne touche qu’à ses propres drapeaux, ce qui est le cas chez toi.

Il y a en revanche un coût de latence. Si refresh() n’est appelée qu’au tick, une tâche peut attendre presque une période de tick après l’interruption avant d’être rendue prête. Avec un tick de 1 ms, c’est acceptable pour un bouton ; ça l’est moins pour un capteur à 10 kHz.

Le point le plus délicat est la fenêtre entre la vérification d’un événement et le blocage de la tâche : si l’interruption arrive entre les deux, le drapeau est écrit, mais personne ne l’a consommé. C’est là que peut se produire un réveil perdu. Le remède classique consiste à rendre la vérification et le blocage indivisibles, soit en masquant les interruptions pendant cet intervalle, soit en faisant relire le drapeau par le noyau juste après l’enregistrement de l’attente.

Un mot sur l’acquittement. Il faut lire le registre de statut, puis acquitter exactement les bits traités : sur ce composant, GPIOICR acquitte les bits auxquels on écrit 1, et ignore ceux auxquels on écrit 0. Écrire un masque complet acquitterait donc aussi des événements arrivés entre-temps. Ton handler commun fait déjà ce qu’il faut en écrivant la valeur du statut lu, et non une constante.

Sur le nommage, je trouverais plus clair de rapprocher les fonctions qui font partie du même contrat : trigger_event() et set_wait_event() décrivent les deux côtés d’un rendez-vous (l’événement arrive, la tâche attend), alors que wake_up() et refresh() décrivent la façon dont le rendez-vous est honoré. Un préfixe commun, ou deux groupes de commentaires distincts, aiderait un lecteur qui découvre le fichier.

Pour les détails des registres, voir 01-LM3S9B92-DataSheet.pdf⁠￼ (section consacrée aux interruptions des ports).

** question

merci. une idée : ranger les tâches en attente par port puis par broche, avec une file de 8 tâches au plus par broche (une puissance de 2), plutôt qu'une liste unique. et un petit arbre de réduction au-dessus pour savoir vite quelles broches ont des tâches en attente. qu'en penses-tu ?

** reponse

Ton idée d’organiser les attentes par port, puis par broche, est pertinente. Elle suit la structure du matériel : un handler traite un port, donc il n’a à parcourir que les attentes de ce port.

Ce que cela apporte :

* Le coût d’un réveil ne dépend plus du nombre total de tâches en attente, mais du nombre de broches du port concerné.
* Une file de 8 places (puissance de 2) permet d’utiliser un masque plutôt qu’un modulo, ce qui est plus simple sur un cœur sans division matérielle.
* L’arbre de réduction répond vite à la question « ce port a-t-il quelqu’un en attente ? », sans parcourir les 8 broches.

Deux précautions. D’abord, la taille maximale par broche devient une limite du système : il faut décider du comportement quand la file est pleine. Refuser l’attente est plus sûr que d’écraser une tâche déjà enregistrée, parce que l’appelant peut alors choisir de réessayer, d’attendre autrement ou de signaler une erreur de configuration. Ensuite, l’arbre de réduction doit être mis à jour dans la même section critique que la file, sinon il peut annoncer une attente qui n’existe plus, et refresh() parcourrait des broches pour rien.

Sur la représentation, une file circulaire de pointeurs vers les tâches suffit : une tête, un compteur, et un tableau de huit cases. Le test « la file est-elle pleine ? » devient une simple comparaison du compteur, et l’insertion se fait à l’indice (tête + compteur) masqué par 7. Tu n’as besoin ni d’allocation dynamique ni de liste chaînée, ce qui compte dans un système où l’on veut borner la mémoire à la compilation.

Sur l’arbre de réduction, un simple entier de 16 bits suffit : un bit par port. Le test « y a-t-il une attente quelque part ? » devient une comparaison à zéro, et la recherche du premier port concerné peut utiliser l’instruction de comptage des zéros de tête, que ton processeur fournit. Tu pourrais ajouter un deuxième niveau (un octet par port pour les broches), mais avec huit broches par port le gain serait négligeable : la boucle sur les broches est déjà courte.

Pour tester cette structure, écris d’abord un scénario sur le papier : deux tâches attendent la même broche, une troisième attend une broche voisine, puis un événement arrive sur chacune. Vérifie à la main l’ordre des réveils et l’état de l’arbre après chaque étape. Si ce scénario est simple à écrire, l’implémentation le sera aussi.

Enfin, pense à la symétrie : tout ce qui est ajouté à l’enregistrement d’une attente doit être retiré lors du réveil, y compris le bit du port dans l’arbre de réduction quand plus aucune broche du port n’est en attente. Dans ton code actuel, ce dernier retrait manque ; ce n’est pas grave tant que refresh() ignore les ports sans événement, mais cela fait travailler la boucle inutilement.

** question

et pour le réveil lui-même : le driver peut-il mettre directement la tâche dans l'état prêt quand l'événement arrive, ou est-ce qu'il vaut mieux passer par le noyau ? j'aimerais éviter une copie de l'information d'attente.

** reponse

Les deux approches existent, et elles ne s’excluent pas.

Option	Ce que fait le driver	Ce que fait le noyau
Réveil direct	change l’état de la tâche	le découvre au prochain tick
Passage par le noyau	signale la broche	identifie et réveille la tâche
Hybride	marque l’événement	consomme la marque
Réveil différé	pose un drapeau	décide au prochain tick
Rappel (callback)	appelle une fonction	fournit cette fonction

Une tâche bloquée peut être référencée directement dans les structures d’attente du périphérique, puis rendue à l’ordonnanceur lors du réveil. Cela évite toute copie : l’attente enregistre un pointeur vers la tâche (4 octets), et le réveil n’a qu’à la retrouver.

La difficulté est de savoir qui a le droit de modifier l’état d’une tâche. Si deux endroits le font, il faut une règle unique, sinon deux réveils concurrents peuvent se contredire : l’un place la tâche dans l’état prêt pendant que l’autre la considère encore bloquée. Je te conseille plutôt :

* Le driver ne connaît que des événements : « broche 3 du port B est active ».
* le gestionnaire d’attente identifie la tâche à réveiller, puis une primitive commune réalise le transfert vers PENDING.
* Cette primitive est la seule à écrire l’état, ce qui rend le comportement facile à vérifier.
* Le réveil direct reste possible pour les cas urgents, mais en passant par cette même primitive.

En multicœur, cependant, une écriture atomique de l’état ne suffit pas à garantir ce transfert. Il faut aussi s’assurer que la tâche n’est pas déjà en cours d’exécution ailleurs, ce qui demande un verrou ou un protocole plus riche.

Pour que le comportement reste lisible, fais en sorte que la primitive commune rende un résultat : « tâche réveillée », « tâche déjà prête » ou « tâche inconnue ». Le premier cas est le cas normal. Le deuxième indique un réveil en double, souvent sans gravité, mais qu’il vaut mieux compter. Le troisième est une erreur de structure, par exemple une attente qui pointe vers une tâche supprimée, et doit être signalé tout de suite plutôt que découvert plus tard sous la forme d’un blocage inexplicable.

Pour la gestion des priorités, pense à l’endroit où elles apparaissent. Si le driver réveille les tâches dans l’ordre d’arrivée, une tâche urgente peut attendre derrière une tâche peu importante. Deux solutions : trier la file à l’insertion, ce qui coûte au moment de l’attente (hors interruption, donc sans contrainte forte), ou laisser l’ordonnanceur choisir parmi les tâches devenues prêtes, ce qui est plus simple et plus classique. Je choisirais la seconde : le driver n’a alors aucune notion de priorité.

Dernier point : la durée de la section critique. Tant qu’elle se limite à vider une file de huit cases au plus, elle reste de l’ordre de quelques microsecondes, et tu peux la masquer sans craindre de perdre une interruption. Si un jour tu ajoutes des priorités ou des politiques de réveil plus riches, mesure-la de nouveau : c’est ce chiffre qui décidera si le tri doit rester dans la section critique ou en sortir.

Enfin, sur le nom de la primitive : évite « wake » seul, qui laisse croire qu’elle exécute la tâche. Elle ne fait que la rendre candidate à l’exécution ; c’est l’ordonnanceur qui décide quand elle s’exécute vraiment. Un nom comme « marquer prête » est plus honnête, et évite des erreurs de raisonnement plus tard.

** question

attends : mon noyau est monocœur pour l'instant, un seul ordonnanceur et une seule pile d'interruptions. est-ce que ça change ta réponse ?

** reponse

Oui, sur un point précis. Ma réponse précédente supposait plusieurs cœurs capables d’ordonnancer simultanément. Ce n’est pas ton cas, et cela simplifie la situation.

Dans un système monocœur :

* Une section critique courte (masquer les interruptions le temps d’écrire l’état) suffit à rendre le transfert atomique.
* Il n’y a pas de course entre deux ordonnanceurs : le seul concurrent du noyau est un handler d’interruption.

Concrètement, cela te permet de retirer toute idée de verrou et de compteur de références sur l’état des tâches. Le masquage des interruptions autour de l’écriture suffit, pourvu qu’il soit court et toujours restauré à sa valeur précédente plutôt que forcé : une section critique appelée depuis une autre section critique ne doit pas réactiver les interruptions en sortant.

Un détail qui change aussi : tu peux supprimer le drapeau « attente en cours » de chaque broche si tu acceptes de parcourir la file vide à chaque tick. Sur huit broches par port, le coût est négligeable, et tu retires une cause de désynchronisation entre le drapeau et la file. Cette simplification est typique du monocœur : on échange un peu de temps de calcul contre moins d’états à garder cohérents.

Une précaution subsiste : le handler d’interruption peut survenir à n’importe quel moment du code du noyau, y compris pendant la manipulation d’une file. C’est pourquoi le masquage reste indispensable, même sans second cœur. Seule sa justification change : il ne protège plus contre un autre cœur, mais contre toi-même.

Je garderais tout de même la primitive commune. Elle ne coûte presque rien, et elle t’évitera de réécrire le driver si tu passes un jour à plusieurs cœurs. Le seul changement à ce moment-là sera l’intérieur de la primitive : le reste du driver continuera de ne parler que d’événements.

** question

et si plus tard je passe en multicœur, qu'est-ce qu'il faudrait prévoir dès maintenant dans le driver, pour ne pas tout refaire ?

** reponse

Peu de choses, si les frontières sont bien posées dès maintenant.

* Isoler l’écriture de l’état d’une tâche dans une seule fonction, pour pouvoir la remplacer par une version avec verrou.
* Ne jamais supposer que le code qui réveille une tâche s’exécute sur le cœur qui l’a bloquée.

Le reste est une question de protocole plus que de code : décider quel cœur reçoit quelle interruption, et où vit la file d’attente de chaque broche. Deux familles de solutions existent. Dans la première, chaque broche est liée à un cœur, et seules les tâches de ce cœur peuvent l’attendre : le driver reste presque inchangé, mais le placement des tâches devient une contrainte. Dans la seconde, n’importe quelle tâche peut attendre n’importe quelle broche, et le réveil doit alors retrouver le cœur de la tâche, puis lui envoyer une interruption inter-cœurs pour que son ordonnanceur la voie.

La première famille est plus simple à raisonner et plus rapide dans le cas courant. La seconde est plus souple, mais elle ajoute une latence de communication et un risque de réveil perdu entre cœurs, qu’il faudra traiter avec le même soin que dans le cas monocœur. Dans les deux cas, garde l’habitude de noter dans le code pourquoi un choix a été fait : c’est ce qui te fera gagner du temps le jour où tu devras le remettre en question.

Enfin, prépare les mesures. Avant de passer en multicœur, note la latence moyenne et la latence maximale entre l’interruption et le moment où la tâche s’exécute, dans plusieurs charges différentes. Ce sont tes valeurs de référence : sans elles, tu ne pourras pas dire si la version multicœur est meilleure, ni dans quels cas elle l’est moins. Mieux vaut le noter dans le cahier des charges. Ce choix se teste avec une carte à deux cœurs. Il faudra aussi mesurer le coût des verrous. Je te conseille de garder une trace de cette décision. En pratique, le plus simple est d’écrire un test qui force la course. Un commentaire dans le code suffit pour ne pas l’oublier. Rien ne presse tant que la carte reste monocœur. L’essentiel est de ne pas figer ce choix trop tôt.
