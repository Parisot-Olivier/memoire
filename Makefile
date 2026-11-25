# memoire — lancement en une commande (INTERFACE.md, section 4).
#   make            construit l'exécutable puis ouvre l'interface
#   make lancer BASE=/chemin/projet/.memoire/depot.db
#   make aide       liste les cibles

CARGO  ?= cargo
PYTHON ?= python3
BIN    := target/release/memoire
BASE   ?=

# Essais de l'interface : sous un écran virtuel (xvfb-run) s'il est installé, donc sans afficher
# aucune fenêtre ; sinon sur l'écran courant. VIRTUEL=0 force l'écran courant.
VIRTUEL ?= 1
XVFB    := $(shell command -v xvfb-run 2>/dev/null)
ifneq ($(and $(filter 1,$(VIRTUEL)),$(XVFB)),)
ECRAN_VIRTUEL := env -u WAYLAND_DISPLAY xvfb-run -a
endif

ifneq ($(strip $(BASE)),)
BASE_OPTION := --base "$(BASE)"
endif

.DEFAULT_GOAL := lancer
.PHONY: lancer construire prerequis tests verifier nettoyer aide

lancer: construire ## Construit l'exécutable puis ouvre l'interface graphique
	$(PYTHON) gui/memoire_gui.py --memoire "$(CURDIR)/$(BIN)" $(BASE_OPTION)

construire: prerequis ## Construit l'exécutable memoire (versions figées par Cargo.lock, D14)
	$(CARGO) build --release --locked -p memoire-cli

prerequis: ## Vérifie la présence de cargo, de Python 3 et de Tkinter
	@command -v $(CARGO) >/dev/null || { echo "cargo introuvable : installer Rust (https://rustup.rs)"; exit 1; }
	@command -v $(PYTHON) >/dev/null || { echo "$(PYTHON) introuvable : installer Python 3"; exit 1; }
	@$(PYTHON) -c "import tkinter" 2>/dev/null || { echo "Tkinter absent : installer le paquet python3-tk (Debian, Ubuntu) ou l'équivalent"; exit 1; }

tests: construire ## Tests du noyau, de la ligne de commande et de l'interface (sans fenêtre visible si xvfb-run est installé)
	$(CARGO) test --workspace --locked
	@$(if $(ECRAN_VIRTUEL),echo "Essais de l'interface sous écran virtuel (xvfb-run).",echo "xvfb-run absent ou VIRTUEL=0 : les essais de l'interface utilisent l'écran courant.")
	cd gui && MEMOIRE="$(CURDIR)/$(BIN)" $(ECRAN_VIRTUEL) $(PYTHON) -m unittest discover -v

verifier: ## Contrôle complet (formatage, dépendances, audit, lints, tests, mutation)
	./tools/check.sh

nettoyer: ## Supprime les produits de compilation
	$(CARGO) clean

aide: ## Liste les cibles
	@grep -E '^[a-z]+:.*## ' $(MAKEFILE_LIST) | sed -E 's/:.*## /\t/'
