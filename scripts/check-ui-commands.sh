#!/usr/bin/env bash
# Every invoke() in the frontend must name a command the Tauri side registers.
#
# tsc cannot catch this: invoke() is generic over the string, not the command
# name, so a call to a deleted command type-checks and fails only at runtime.
# This shipped once already -- the Maintenance page called three commands
# removed in the daemon split, and every check short of grep missed it.
set -euo pipefail

cd "$(dirname "$0")/.."

registered=$(grep -oE 'fn [a-z_]+' ui/src-tauri/src/lib.rs \
             | awk '{print $2}' | sort -u)

fail=0
while IFS=: read -r file line rest; do
    cmd=$(echo "$rest" | grep -oE "invoke<[^>]*>\('[a-zA-Z_]+'\)|invoke\('[a-zA-Z_]+'\)" \
          | grep -oE "'[a-zA-Z_]+'" | tr -d "'")
    [ -z "$cmd" ] && continue
    if ! echo "$registered" | grep -qx "$cmd"; then
        echo "error: $file:$line calls unregistered Tauri command '$cmd'"
        fail=1
    fi
done < <(grep -rnE "invoke(<[^>]*>)?\('[a-zA-Z_]+'\)" ui/src --include='*.ts' --include='*.tsx')

[ $fail -eq 0 ] && echo "ui commands ok"
exit $fail
