#!/bin/sh
# Essais graphiques sous KWin virtuel avec XWayland : un vrai gestionnaire de fenêtres, celui du
# bureau de l'utilisateur, sans écran. Il faut un gestionnaire pour vérifier l'ordre des fenêtres
# flottantes (PLAN-DE-TRAVAIL.md, section 10) : sous Xvfb seul, Tk ne relit que ce qu'un
# gestionnaire a appliqué. Usage : MEMOIRE=… tools/essais-sous-kwin.sh ; code : celui des essais.
set -eu
racine="$(cd "$(dirname "$0")/.." && pwd)"
travail="$(mktemp -d)"
trap 'rm -rf "$travail"' EXIT
cat > "$travail/session.sh" <<FIN
#!/bin/sh
cd "$racine/gui"
python3 -m unittest -v test_gui > "$travail/sortie" 2>&1
echo \$? > "$travail/code"
FIN
chmod +x "$travail/session.sh"
env -u WAYLAND_DISPLAY -u DISPLAY kwin_wayland --virtual --xwayland --no-lockscreen \
    --width 1600 --height 1000 --socket "memoire-essais-$$" \
    --exit-with-session "$travail/session.sh" > "$travail/kwin" 2>&1 || true
if [ ! -f "$travail/code" ]; then
    echo "KWin virtuel : les essais n'ont pas pu se lancer." >&2
    cat "$travail/kwin" >&2
    exit 1
fi
cat "$travail/sortie"
exit "$(cat "$travail/code")"
