# Contributing

## Module layout

Files live flat at `src/` root. A folder appears only when a *feature*
outgrows one file — roughly 3+ files or ~600 LOC — and is named for the
feature (`eq/`, `models/`), never for an architectural layer. Layering is
expressed by crate boundaries: `-proto` is wire types, `-hid`/`-pipewire`
are transport, `-core` is domain, `-daemon` is session and transport. Do
not restate that axis inside a crate.
