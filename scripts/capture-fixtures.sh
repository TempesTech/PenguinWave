#!/usr/bin/env bash
# Capture parser fixtures from a live PipeWire session, then scrub identifying
# details before they can be committed.
#
# These fixtures are the only regression net for the backend's text parsers,
# and they cannot be reconstructed without the same hardware attached. Re-run
# this when adding support for a device whose output shape differs.
#
# Usage: ./scripts/capture-fixtures.sh [output-dir]
set -euo pipefail

cd "$(dirname "$0")/.."
DEST=${1:-crates/penguinwave-pipewire/tests/fixtures}
mkdir -p "$DEST"

command -v pactl   >/dev/null || { echo "pactl not found"   >&2; exit 1; }
command -v pw-link >/dev/null || { echo "pw-link not found" >&2; exit 1; }
command -v pw-dump >/dev/null || { echo "pw-dump not found" >&2; exit 1; }

cap() {
    local name=$1; shift
    "$@" > "$DEST/$name" 2>&1 || true
    printf '  %-28s %8s bytes\n' "$name" "$(wc -c < "$DEST/$name")"
}

echo "capturing:"
cap sinks_full.txt           pactl list sinks
cap sinks_short.txt          pactl list sinks short
cap sink_inputs_full.txt     pactl list sink-inputs
cap sink_inputs_short.txt    pactl list short sink-inputs
cap default_sink.txt         pactl get-default-sink
cap cards_full.txt           pactl list cards
cap modules_short.txt        pactl list modules short
cap sources_short.txt        pactl list sources short
cap pw_link_list.txt         pw-link -l
cap pw_link_ids.txt          pw-link -I -l
cap pw_link_ports_in.txt     pw-link -i
cap pw_link_ports_out.txt    pw-link -o
cap pw_dump_all.json         pw-dump

# Failure shapes matter as much as success shapes: the parsers must not treat
# an error as data. `pw-cli` in particular exits 0 on an unknown command.
cap pactl_error_missing_sink.txt pactl set-sink-volume nonexistent_sink_xyz 50%
cap pw_cli_unknown_cmd.txt       pw-cli dump game_sink
: > "$DEST/sinks_short_empty.txt"

echo "scrubbing:"
scrub() {
    local pattern=$1 replacement=$2 label=$3
    if grep -rlE "$pattern" "$DEST" >/dev/null 2>&1; then
        grep -rlE "$pattern" "$DEST" | while read -r f; do
            sed -i -E "s/$pattern/$replacement/g" "$f"
        done
        echo "  $label"
    fi
}

scrub "$(id -un)"   "testuser"           "username"
scrub "$(hostname)" "testhost"           "hostname"
scrub '([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}' 'AA:BB:CC:DD:EE:FF' "MAC (colon form)"
scrub '([0-9A-Fa-f]{2}_){5}[0-9A-Fa-f]{2}' 'AA_BB_CC_DD_EE_FF' "MAC (underscore form)"
# Anchored to the underscore that precedes a serial in an ALSA device name
# (`..._DuoCast_202011110001-00`). An unanchored digit run also matches inside
# legitimate numbers: it rewrote i64::MIN in pw-dump into invalid JSON.
scrub '_[0-9]{10,}' "_000000000000"      "device serials"

# Fail loudly rather than committing a leak.
leaks=0
for pat in "$(id -un)" "$(hostname)"; do
    if grep -rq "$pat" "$DEST" 2>/dev/null; then
        echo "error: '$pat' still present in fixtures" >&2
        leaks=1
    fi
done
if grep -rqE '([0-9A-Fa-f]{2}[:_]){5}[0-9A-Fa-f]{2}' "$DEST" \
   && ! grep -rqE 'AA[:_]BB' "$DEST"; then
    echo "error: unscrubbed MAC address in fixtures" >&2
    leaks=1
fi
[ $leaks -eq 0 ] || exit 1

# Scrubbing rewrites bytes inside files that still have to parse.
for f in "$DEST"/*.json; do
    [ -e "$f" ] || continue
    python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$f" \
        || { echo "error: $f is not valid JSON after scrubbing" >&2; exit 1; }
done

echo "fixtures clean"
