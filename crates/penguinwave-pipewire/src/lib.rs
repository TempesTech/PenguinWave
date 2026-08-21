//! PipeWire backend.
//!
//! Sole owner of every `pactl` / `pw-link` / `pw-cli` invocation in the
//! project. Everything above this crate talks to `PipeWireBackend`, never to a
//! subprocess, so replacing the shell-out implementation with native
//! `pipewire-rs` bindings later is a crate swap rather than a rewrite.
//!
//! Knows nothing about EQ semantics, ChatMix, or configuration: it moves
//! streams, sets volumes, creates sinks, links ports and writes node params.
//!
//! Filled in at P1. See `claudedocs/workflow_daemon_split.md`.
