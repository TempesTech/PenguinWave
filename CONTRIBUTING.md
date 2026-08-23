# Contributing

## Module layout

Every crate's `src/` is split by architectural layer, not left flat:

- `models/` — wire/data types with no behavior (`penguinwave-proto`, `penguinwave-hid`)
- `domain/` — business logic: policy, state, parsing, backend traits
- `transport/` — I/O boundaries: sockets, subprocess shell-outs, D-Bus, HID transport
- `api/` — the public request/response surface (`penguinwave-core`)
- `session/` — per-connection session handling (`penguinwave-daemon`)
- `commands/` — CLI argument/command definitions (`penguinwave-cli`)

A crate-root file (`error.rs`, `lib.rs`, `main.rs`, `state.rs`, `icons.rs`, `mock.rs`,
`notify.rs`) stays at `src/` root when it doesn't belong to one of the above layers,
or is the crate's own aggregate root.

See each crate's `src/*/mod.rs` doc comment for what that layer holds there
specifically — the same folder name can mean a different thing in a different
crate (e.g. `transport/` in `-pipewire` is shell-outs; in `-hid` it's `hidapi`).
