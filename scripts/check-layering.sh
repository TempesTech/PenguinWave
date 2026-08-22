#!/usr/bin/env bash
# Enforce the dependency rules the architecture depends on.
#
# These are one-line greps standing in for boundaries that are otherwise
# maintained by discipline alone. Each has a specific failure it prevents.
set -euo pipefail

cd "$(dirname "$0")/.."
fail=0

# Match code only. Comments naturally mention the things being banned — this
# file's own rules are documented in the crates they police — and a check that
# fires on prose is a check people turn off.
check() {
    local desc=$1 dir=$2 pattern=$3
    [ -d "$dir" ] || return 0
    local hits
    hits=$(grep -rnE "$pattern" "$dir" --include='*.rs' \
           | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*)' || true)
    if [ -n "$hits" ]; then
        echo "error: $desc"
        echo "$hits" | head -5
        fail=1
    fi
}

# penguinwave-proto must stay pure: it is compiled for wasm by future web
# clients, and anything that spawns or reads files breaks that.
check "penguinwave-proto must not perform I/O" \
      crates/penguinwave-proto/src \
      'std::process|std::fs|std::net|std::os::unix::net'

# Every subprocess call lives behind PipeWireBackend. A stray Command::new
# elsewhere is a backend the trait cannot mock, test, or later replace.
for c in penguinwave-core penguinwave-hid penguinwave-daemon penguinwave-cli; do
    check "$c must not spawn processes (use PipeWireBackend)" \
          "crates/$c/src" 'Command::new'
done

# hidapi is confined to penguinwave-hid so the daemon package can be installed
# and tested without it, and so core stays mockable. The invariant is the
# dependency, not the spelling: penguinwave-hid re-exports a backend whose type
# name contains "HidApi", and matching on that would fire on legitimate use.
for c in penguinwave-proto penguinwave-core penguinwave-pipewire penguinwave-daemon penguinwave-cli; do
    check "$c must not name the hidapi crate" "crates/$c/src" '(^|[^a-zA-Z_])hidapi::|use hidapi'
    if [ -f "crates/$c/Cargo.toml" ] && grep -qE '^hidapi[[:space:]]*=' "crates/$c/Cargo.toml"; then
        echo "error: $c must not depend on hidapi (use penguinwave-hid)"
        fail=1
    fi
done

# JSON numbers are IEEE-754 doubles in JavaScript, exact only to 2^53-1.
# ts-rs maps u64/i64 to `bigint` (which JSON.stringify throws on) and usize to a
# silently-lossy `number`. Wire fields stay <= 32 bits; anything genuinely
# larger must be carried as a string, deliberately.
check "penguinwave-proto wire fields must not use u64/i64/usize/isize" \
      crates/penguinwave-proto/src '^[[:space:]]*(pub )?[a-z_]+: (u64|i64|usize|isize)[,<]'

# The socket is the daemon's concern. Core owning transport would make it
# untestable without one.
check "penguinwave-core must not know about sockets" \
      crates/penguinwave-core/src 'UnixListener|UnixStream'

[ $fail -eq 0 ] && echo "layering ok"
exit $fail
