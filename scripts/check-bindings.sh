#!/usr/bin/env bash
# Fail if the committed TypeScript bindings differ from what the Rust types
# currently generate.
#
# Hand-maintained types on both sides of a socket drift, and the drift shows up
# as a runtime `undefined` in the UI rather than a compile error. Regenerating
# in CI and diffing is what keeps `penguinwave-proto` the single source of truth.
set -euo pipefail

cd "$(dirname "$0")/.."
BINDINGS="crates/penguinwave-proto/bindings"

before=$(mktemp -d)
trap 'rm -rf "$before"' EXIT
[ -d "$BINDINGS" ] && cp -r "$BINDINGS/." "$before/"

rm -rf "$BINDINGS"
cargo test -p penguinwave-proto export_bindings --quiet

if ! diff -ru "$before" "$BINDINGS"; then
    echo
    echo "error: TypeScript bindings are stale."
    echo "Run: cargo test -p penguinwave-proto export_bindings"
    echo "Then commit the regenerated files under $BINDINGS."
    exit 1
fi
echo "bindings up to date"
