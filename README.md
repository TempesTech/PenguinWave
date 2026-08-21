# penguinwave-next

Working tree for the Penguin Wave 0.3.0 daemon/UI split. Lands back in the main
repo at P6.

Design: `PenguinWave/claudedocs/{REQUIREMENTS,DESIGN}-daemon-split.md`
Plan: `PenguinWave/claudedocs/workflow_daemon_split.md`

## Layout

| Crate | Role | Phase |
|---|---|---|
| `penguinwave-proto` | wire types, no I/O | **P0 done** |
| `penguinwave-pipewire` | `PipeWireBackend` trait + `pactl` impl | P1 |
| `penguinwave-hid` | headsets, sole owner of `hidapi` | P2 |
| `penguinwave-core` | state, EQ, ChatMix, config | P3 |
| `penguinwave-daemon` | socket server (`penguinwave-daemon` binary) | P4 |
| `penguinwave-cli` | socket client (`penguinwave-cli` binary) | P4 |

Dependency arrows never reverse: `penguinwave-pipewire` does not know the
daemon exists, `penguinwave-core` does not know sockets exist, and
`penguinwave-proto` depends on nothing but `serde`. `scripts/check-layering.sh`
enforces this in CI.

No installed binary may use the `pw-` prefix — `/usr/bin/pw-cli` belongs to the
`pipewire` package, and shadowing it would break the very tool the backend
shells out to.

## Development

```sh
cargo test --workspace          # includes TS binding export
./scripts/check-layering.sh     # dependency + I/O boundaries
./scripts/check-bindings.sh     # TS bindings match the Rust types
```

TypeScript definitions in `crates/penguinwave-proto/bindings/` are **generated**.
Edit the Rust types and re-run the export; never edit the `.ts` files.
