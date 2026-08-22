# PenguinWave

PipeWire virtual-sink manager for Linux gamers. Route per-app audio between virtual sinks and drive your headset's ChatMix wheel from one place.

> v0.1 — desktop-only, Linux-only. Tested on Arch (CachyOS).

## What it does

- **Virtual sinks**: creates `game_sink` + `chat_sink` on first launch; lets you create / delete more.
- **Per-app routing**: drag-and-drop applications between sinks, or pick from a dropdown.
- **Per-stream volume + mute**: real `pactl set-sink-input-volume` / `set-sink-input-mute`, debounced sliders.
- **ChatMix wheel integration**: turn your headset wheel, watch the slider move and the game/chat sink volumes flip live.
- **Port linking**: a power-user panel to wire any source output port to any target input port (`pw-link`).
- **Friendlier stream names**: falls back from `application.name` to `application.process.binary` to `/proc/<pid>/cmdline` to a `media.role`-based hybrid label. Rename anything inline (persisted to `localStorage`).

## Supported headsets (v0.1)

- SteelSeries Arctis Nova 7

Adding more is a matter of implementing `DeviceTrait` in `src-tauri/src/headsets/` and registering it in `AppStateManager::initialize_supported_devices`.

## Requirements

- **PipeWire** userland: `pactl`, `pw-cli`, `pw-link` on PATH (Debian/Ubuntu: `pipewire pipewire-pulse pipewire-cli`; Arch: `pipewire pipewire-pulse wireplumber`).
- **Linux** only — uses `/proc` and HID via `hidapi`.
- HID device permissions for headset access (udev rules typically grant via `plugdev` / `input` group).

## Install (from source)

```bash
git clone <repo> penguinwave
cd penguinwave
npm install
npm run tauri:build
# .deb / AppImage ends up in src-tauri/target/release/bundle/
```

## Run in dev

```bash
npm run tauri:debug   # full app, RUST_BACKTRACE=full RUST_LOG=debug
npm run dev           # frontend only on http://localhost:1420 (no Tauri commands)
```

## Architecture

```
React (Vite)  ──invoke──▶  Tauri 2 commands  ──shell out──▶  pactl / pw-cli / pw-link
     ▲                            │                                    │
     │                            ▼                                    ▼
   listen()  ◀──emit─────  chatmix poll thread (100ms)  ◀──HID──  hidapi
```

- Frontend: React 18, TanStack Query 5, wouter, Radix + shadcn/ui (style "new-york"), Tailwind 3.
- Backend: Rust edition 2021, Tauri 2, `hidapi`, `anyhow`/`thiserror`.

## Project layout

- `src/` — React frontend (pages, components, hooks, queries, types).
- `src-tauri/` — Rust crate (`controllers/`, `system/pipewire.rs`, `event/chatmix_listener.rs`, `headsets/`, `utils/`).
- `shared/` — vestigial Drizzle schema (not wired up).

## License

TBD.
