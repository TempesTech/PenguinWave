//! Parsers for `pactl`, `pw-link` and `pw-dump` output.
//!
//! Pure functions over captured text so they can be tested without an audio
//! server. Callers must run the commands under `LC_ALL=C`: `pactl` localises
//! decimal separators, and fixtures show `-18,06 dB` on a non-English locale.

use std::collections::HashMap;

use penguinwave_proto::{
    LinkInfo, OutputDevice, PortDirection, PortInfo, PortRef, SinkInfo, StreamRef,
};

/// Factory behind every virtual sink Penguin Wave creates.
const NULL_SINK_FACTORY: &str = "support.null-audio-sink";

/// One `Sink #N` or `Sink Input #N` block.
#[derive(Debug, Default, Clone)]
pub struct Block {
    pub id: u32,
    pub fields: HashMap<String, String>,
    pub props: HashMap<String, String>,
}

impl Block {
    pub fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props.get(key).map(String::as_str)
    }
}

/// Split `pactl list <thing>` output into blocks introduced by `prefix`.
pub fn parse_blocks(input: &str, prefix: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current: Option<Block> = None;
    let mut in_props = false;

    for line in input.lines() {
        if let Some(rest) = line.strip_prefix(prefix) {
            if let Some(b) = current.take() {
                blocks.push(b);
            }
            in_props = false;
            let id = rest.trim().trim_start_matches('#').parse().unwrap_or(0);
            current = Some(Block {
                id,
                ..Default::default()
            });
            continue;
        }

        let Some(block) = current.as_mut() else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "Properties:" {
            in_props = true;
            continue;
        }

        // Properties are indented one level deeper than fields; a line that is
        // not indented past the field level ends the property section.
        let depth = line.len() - line.trim_start().len();
        if in_props && depth < 2 {
            in_props = false;
        }

        if in_props {
            if let Some((k, v)) = trimmed.split_once(" = ") {
                block
                    .props
                    .insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
            }
        } else if let Some((k, v)) = trimmed.split_once(": ") {
            block
                .fields
                .insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    blocks.extend(current);
    blocks
}

/// First percentage in a `Volume:` line, clamped to 0..=100.
///
/// The line repeats per channel and pads irregularly:
/// `front-left: 32768 /  50% / -18,06 dB,   front-right: ...`
pub fn parse_volume_pct(line: &str) -> Option<u8> {
    let idx = line.find('%')?;
    let digits: String = line[..idx]
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    let pct: u32 = digits.chars().rev().collect::<String>().parse().ok()?;
    Some(pct.min(100) as u8)
}

fn yes(v: Option<&str>) -> bool {
    matches!(v, Some("yes"))
}

/// Parse `pactl list sinks`.
pub fn parse_sinks(input: &str) -> Vec<SinkInfo> {
    parse_blocks(input, "Sink #")
        .into_iter()
        .filter_map(|b| {
            Some(SinkInfo {
                id: b.id,
                name: b.field("Name")?.to_string(),
                description: b.field("Description").unwrap_or_default().to_string(),
                volume: b.field("Volume").and_then(parse_volume_pct).unwrap_or(100),
                is_muted: yes(b.field("Mute")),
                managed: b.prop("factory.name") == Some(NULL_SINK_FACTORY),
            })
        })
        .collect()
}

/// Hardware outputs: every sink we did not create.
pub fn parse_output_devices(input: &str) -> Vec<OutputDevice> {
    parse_sinks(input)
        .into_iter()
        .filter(|s| !s.managed)
        .map(|s| OutputDevice {
            id: s.id,
            name: s.name,
            description: s.description,
        })
        .collect()
}

/// A stream, before its display name is resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct RawStream {
    pub stream: StreamRef,
    /// PipeWire `node.name`, which identifies our own streams.
    pub node_name: Option<String>,
    pub sink_id: u32,
    pub volume: u8,
    pub is_muted: bool,
    pub binary: Option<String>,
    pub media_role: Option<String>,
    pub media_name: Option<String>,
    pub icon_name: Option<String>,
}

/// Parse `pactl list sink-inputs`.
pub fn parse_sink_inputs(input: &str) -> Vec<RawStream> {
    parse_blocks(input, "Sink Input #")
        .into_iter()
        .map(|b| {
            let pid = b
                .prop("application.process.id")
                .and_then(|s| s.parse().ok());
            RawStream {
                stream: StreamRef {
                    index: b.id,
                    app_name: b.prop("application.name").unwrap_or_default().to_string(),
                    pid,
                },
                node_name: b.prop("node.name").map(str::to_string),
                sink_id: b.field("Sink").and_then(|s| s.parse().ok()).unwrap_or(0),
                volume: b.field("Volume").and_then(parse_volume_pct).unwrap_or(100),
                is_muted: yes(b.field("Mute")),
                binary: b.prop("application.process.binary").map(str::to_string),
                media_role: b.prop("media.role").map(str::to_string),
                media_name: b.prop("media.name").map(str::to_string),
                icon_name: b.prop("application.icon_name").map(str::to_string),
            }
        })
        .collect()
}

/// Parse `pactl list sinks short` into `(id, name)`.
pub fn parse_sinks_short(input: &str) -> Vec<(u32, String)> {
    input
        .lines()
        .filter_map(|line| {
            let mut cols = line.split('\t');
            let id = cols.next()?.trim().parse().ok()?;
            Some((id, cols.next()?.trim().to_string()))
        })
        .collect()
}

fn split_port(spec: &str, id: Option<u32>) -> Option<PortRef> {
    let (node, port) = spec.trim().rsplit_once(':')?;
    Some(PortRef {
        node_name: node.to_string(),
        port_name: port.to_string(),
        id,
    })
}

/// Split a leading numeric id off a `pw-link -I` line.
fn split_id(s: &str) -> (Option<u32>, &str) {
    let s = s.trim_start();
    match s.split_once(char::is_whitespace) {
        Some((head, rest)) => match head.parse() {
            Ok(id) => (Some(id), rest.trim_start()),
            Err(_) => (None, s),
        },
        None => (None, s),
    }
}

/// Parse `pw-link -I -l`.
///
/// Ports sit at the left margin, their links indented and prefixed `|->`
/// (outbound) or `|<-` (inbound). Only outbound links are collected, so a link
/// is reported once rather than from both ends.
///
/// Ids are required rather than optional: several nodes share a name, so
/// `node:port` alone does not identify a port.
pub fn parse_links(input: &str) -> Vec<LinkInfo> {
    let mut links = Vec::new();
    let mut current: Option<PortRef> = None;

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (id, rest) = split_id(line);

        if let Some(peer) = rest
            .strip_prefix("|-> ")
            .or_else(|| rest.strip_prefix("|<- "))
        {
            let outbound = rest.starts_with("|-> ");
            let (peer_id, peer_spec) = split_id(peer);
            let Some(source) = current.clone() else {
                continue;
            };
            if outbound {
                if let Some(target) = split_port(peer_spec, peer_id) {
                    links.push(LinkInfo { source, target });
                }
            }
            let _ = id; // link id, not needed to describe the connection
        } else {
            current = split_port(rest, id);
        }
    }

    links
}

/// Parse `pw-dump` JSON into ports.
///
/// Replaces the old `pw-cli dump <node>` scrape: that subcommand does not exist
/// on PipeWire 1.6+, and `pw-cli` exits 0 on an unknown command, so the error
/// text was being parsed as data.
pub fn parse_ports_from_dump(json: &str) -> Result<Vec<PortInfo>, serde_json::Error> {
    let objects: Vec<serde_json::Value> = serde_json::from_str(json)?;

    // Ports carry `node.id`, not `node.name`; the name lives on the node.
    let node_names: HashMap<u64, String> = objects
        .iter()
        .filter(|o| o["type"].as_str() == Some("PipeWire:Interface:Node"))
        .filter_map(|o| {
            let name = o["info"]["props"]["node.name"].as_str()?;
            Some((o["id"].as_u64()?, name.to_string()))
        })
        .collect();

    Ok(objects
        .iter()
        .filter(|o| o["type"].as_str() == Some("PipeWire:Interface:Port"))
        .filter_map(|o| {
            let props = &o["info"]["props"];
            Some(PortInfo {
                id: o["id"].as_u64()? as u32,
                node_name: node_names.get(&props["node.id"].as_u64()?)?.clone(),
                port_name: props["port.name"].as_str()?.to_string(),
                direction: match props["port.direction"].as_str()? {
                    "in" => PortDirection::Input,
                    "out" => PortDirection::Output,
                    _ => return None,
                },
            })
        })
        .collect())
}

/// `pw-cli` exits 0 on an unknown command, so the exit status alone cannot be
/// trusted; its error output starts with `Error:`.
pub fn is_tool_error(output: &str) -> bool {
    output.trim_start().starts_with("Error:")
}

/// Node id for a node name, from `pw-dump` JSON.
pub fn node_id_for(json: &str, node_name: &str) -> Option<u32> {
    let objects: Vec<serde_json::Value> = serde_json::from_str(json).ok()?;
    objects
        .iter()
        .find(|o| {
            o["type"].as_str() == Some("PipeWire:Interface:Node")
                && o["info"]["props"]["node.name"].as_str() == Some(node_name)
        })
        .and_then(|o| o["id"].as_u64())
        .map(|id| id as u32)
}
