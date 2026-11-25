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
