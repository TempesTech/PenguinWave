# Changelog

## 0.3.0

Penguin Wave is now a daemon with clients. Audio management runs as a user
service; the desktop app and the new CLI are both just clients of it.

### You need to do one thing

The daemon runs as a systemd **user** service, and a package cannot enable it
for you — user units are per-user and package scripts run as root. Enable it
once:

```sh
systemctl --user enable --now penguinwave.service
```

The desktop app offers a button for this when it finds nothing listening.

### Changed behaviour

- **Closing the window no longer stops ChatMix.** The daemon owns the wheel, so
  the balance keeps tracking with the UI closed or never launched. The tray is
  a convenience now rather than a requirement, and its "Quit" says so.
- **Stopping the daemon leaves your virtual sinks and the EQ chain running.**
  Audio keeps flowing through `game_sink` and `chat_sink` across a restart or a
  package upgrade; only control stops. `penguinwave-cli teardown --confirm`
  removes them when you actually want them gone.
- **ChatMix can be set by hand.** Dragging the slider pins the balance and the
  wheel stops overriding it; "Follow wheel" hands control back.

### New

- `penguinwave-cli` covers every operation the UI has, plus `watch` for a live
  event stream. Exit codes are stable and scriptable.
- Multiple clients at once. Every change is broadcast, so the UI, the CLI and
  anything else stay in step without polling.
- Shell completions for bash, zsh and fish.
- `penguinwave-daemon` installs and runs with no webview or GTK stack, for
  headless and remote-audio machines.

### Fixed

- Routing survives the output device disappearing. Powering a wireless headset
  on after the app — or plugging the dongle back in — no longer leaves the sink
  silent until you pick the device again.
- Setting an output device no longer bypasses the equalizer. It silently
  disconnected the EQ from the signal path, so band edits did nothing while
  audio kept playing normally.
- The EQ's own streams no longer appear in the application list, where they
  invited you to move Penguin Wave's plumbing around inside Penguin Wave.
- An unrecognised battery reading no longer crashes the polling loop.
- The Port Link panel lists ports again. It had been silently empty:
  `pw-cli dump` does not exist and exits 0 anyway, so its error text was parsed
  as data.
- Port operations match by id rather than name. Several nodes of one
  application share a name, so unlinking could tear down the wrong link.
- Device selection is keyed by vendor and product id, not a list position that
  shifted when the list changed.

### Upgrading from 0.2.0

Your configuration is read as-is: same directory, same files, same format.
Devices and EQ presets carry over untouched, and there is no migration step.

`penguinwave` now depends on `penguinwave-daemon`; `pacman -Syu` pulls it in.
The binary is still `/usr/bin/penguinwave` and the desktop entry is unchanged.
