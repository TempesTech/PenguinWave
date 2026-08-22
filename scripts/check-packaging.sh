#!/usr/bin/env bash
# Packaging invariants that only fail at install time, where they are expensive.
set -euo pipefail

cd "$(dirname "$0")/.."
fail=0

# /usr/bin/pw-cli belongs to the pipewire package, and PactlBackend shells out
# to it. Shipping a pw- prefixed binary would be a pacman file conflict and
# would shadow the tool the backend depends on.
bins=$(grep -rhoE '\$pkgdir"?/usr/bin/[a-zA-Z0-9_-]+' packaging/aur/*/PKGBUILD \
       | sed 's|.*/usr/bin/||' | sort -u)
for bin in $bins; do
    case "$bin" in
        pw-*)
            echo "error: $bin collides with PipeWire's binary namespace"
            fail=1
            ;;
    esac
done

# The UI's path is what existing users and .desktop entries point at.
if ! grep -q 'usr/bin/penguinwave"' packaging/aur/penguinwave/PKGBUILD; then
    echo "error: penguinwave must install /usr/bin/penguinwave (unchanged since 0.2.0)"
    fail=1
fi

# A user unit cannot be enabled by a root-run package script. Heredoc bodies
# are stripped first: these scripts print the enable command as advice, and a
# check that cannot tell a message from a command is one people switch off.
for f in packaging/aur/*/*.install; do
    [ -e "$f" ] || continue
    if awk '
        /<<-?'"'"'?[A-Za-z_]+'"'"'?/ { inheredoc = 1; next }
        inheredoc && /^[A-Za-z_]+$/  { inheredoc = 0; next }
        !inheredoc                   { print }
    ' "$f" | grep -qE '^[[:space:]]*systemctl[[:space:]]+(--user[[:space:]]+)?enable'; then
        echo "error: $f enables a systemd unit; user units are per-user"
        fail=1
    fi
done

# Versions move together: a split package set that disagrees on version is
# unsatisfiable once one of them lands.
versions=$(grep -h '^pkgver=' packaging/aur/*/PKGBUILD | sort -u | wc -l)
if [ "$versions" -ne 1 ]; then
    echo "error: PKGBUILDs disagree on pkgver"
    grep -H '^pkgver=' packaging/aur/*/PKGBUILD
    fail=1
fi

# The rules file grants HID access, which is the daemon's concern. Installing
# it from two packages is a file conflict.
owners=$(grep -lE 'udev/rules\.d' packaging/aur/*/PKGBUILD | wc -l)
if [ "$owners" -ne 1 ]; then
    echo "error: $owners packages install the udev rule; exactly one may"
    fail=1
fi

[ $fail -eq 0 ] && echo "packaging ok"
exit $fail
