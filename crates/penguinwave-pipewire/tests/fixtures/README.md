# Backend parser fixtures

Captured from a live PipeWire session by `scripts/capture-fixtures.sh`, which
also scrubs username, hostname, MAC addresses and device serials before the
files can be committed.

These are the **only** regression net for the backend's text parsers, and they
cannot be reconstructed without the same hardware attached. Do not edit by
hand; re-run the capture script.

## Session captured

PipeWire 1.6.8 on Arch. Deliberately varied, so the parsers meet more than one
shape of input:

| Sink | Why it is here |
|---|---|
| `alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo` | USB headset, the primary supported device |
| `game_sink`, `chat_sink` | the managed virtual sinks, `float32le` |
| `alsa_output.pci-...hdmi-stereo` | PCI/HDMI, `s32le`, suspended |
| `alsa_output.usb-HP__Inc_HyperX_DuoCast_...` | a second USB device — double-underscore in the name |
| `bluez_output.AA_BB_CC_DD_EE_FF.1` | Bluetooth, `s16le`, underscore-separated MAC |

Sink states cover `RUNNING`, `IDLE` and `SUSPENDED`; sample formats cover
`s16le`, `s24le`, `s32le` and `float32le`.

## Files

| File | Source | Consumed by |
|---|---|---|
| `sinks_full.txt` | `pactl list sinks` | output devices, volumes, ports |
| `sinks_short.txt` | `pactl list sinks short` | sink enumeration, custom-sink filtering |
| `sink_inputs_full.txt` | `pactl list sink-inputs` | application streams, name resolution |
| `sink_inputs_short.txt` | `pactl list short sink-inputs` | stream→sink mapping |
| `default_sink.txt` | `pactl get-default-sink` | default sink |
| `cards_full.txt` | `pactl list cards` | card profiles and routes |
| `modules_short.txt` | `pactl list modules short` | identifying sinks we created |
| `sources_short.txt` | `pactl list sources short` | monitor sources |
| `pw_link_list.txt` | `pw-link -l` | link graph, sink routing |
| `pw_link_ids.txt` | `pw-link -I -l` | link graph with ids |
| `pw_link_ports_in.txt` | `pw-link -i` | input port enumeration |
| `pw_link_ports_out.txt` | `pw-link -o` | output port enumeration |
| `pw_dump_all.json` | `pw-dump` | structured graph: nodes, ports, links |

## Failure shapes

Parsers must not mistake an error for data, so the failure cases are fixtures
too.

| File | What it captures |
|---|---|
| `sinks_short_empty.txt` | empty listing — zero sinks |
| `pactl_error_missing_sink.txt` | `pactl` failing on a missing sink |
| `pw_cli_unknown_cmd.txt` | see below |

### `pw-cli dump` does not exist

The pre-split code enumerated ports with `pw-cli dump <node-name>`
(`system/pipewire.rs:466`). On PipeWire 1.6.8 that subcommand does not exist:

```
$ pw-cli dump game_sink
Error: "Command "dump" does not exist. Type 'help' for usage."
$ echo $?
0
```

It **exits 0**. The old code checks `output.status.success()`, sees success,
and parses the error string as port data — so `list_node_ports` returns an
empty list rather than an error, and the Port Link panel silently shows no
ports. A hard failure was being reported as "this node has no ports".

The replacement is `pw-dump` (structured JSON, ids and `port.direction`
included) or `pw-link -i` / `pw-link -o`. `pw_cli_unknown_cmd.txt` exists so a
test can pin the rule: **a zero exit code is not proof of success when the
output is an error.**
