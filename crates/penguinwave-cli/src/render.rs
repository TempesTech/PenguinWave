//! Human-readable output.

use penguinwave_proto::{PortRef, Response};

pub fn render(response: &Response) -> String {
    match response {
        Response::Empty => "ok".into(),
        Response::Hello(h) => format!("daemon {} proto {}..={}", h.daemon, h.proto.0, h.proto.1),

        Response::Snapshot(s) => {
            let mut out = vec![
                format!("default sink: {}", s.default_sink),
                format!(
                    "chatmix: {} ({})",
                    s.chatmix.value,
                    if s.chatmix.manual { "manual" } else { "wheel" }
                ),
                format!(
                    "eq: {}",
                    if s.eq.safe_mode {
                        "safe mode"
                    } else {
                        "active"
                    }
                ),
                String::new(),
                format!("sinks ({})", s.sinks.len()),
            ];
            for sink in &s.sinks {
                out.push(format!(
                    "  {:<28} {:>3}%{}{}",
                    sink.name,
                    sink.volume,
                    if sink.is_muted { " muted" } else { "" },
                    if sink.managed { " [managed]" } else { "" }
                ));
            }
            out.push(String::new());
            out.push(format!("streams ({})", s.streams.len()));
            for stream in &s.streams {
                out.push(format!(
                    "  {:<5} {:<24} -> {:<20} {:>3}%{}",
                    stream.stream.index,
                    truncate(&stream.name, 24),
                    truncate(&stream.sink, 20),
                    stream.volume,
                    if stream.is_muted { " muted" } else { "" }
                ));
            }
            out.push(String::new());
            out.push(format!("devices ({})", s.devices.len()));
            for d in &s.devices {
                out.push(format!(
                    "  {:04x}:{:04x} {:<28} {:?}{}",
                    d.id.vendor_id,
                    d.id.product_id,
                    d.name,
                    d.presence,
                    if Some(d.id) == s.selected_device {
                        " [selected]"
                    } else {
                        ""
                    }
                ));
            }
            out.join("\n")
        }

        Response::Streams(streams) => lines(streams.iter().map(|s| {
            format!(
                "{:<5} {:<28} -> {:<20} {:>3}%{}",
                s.stream.index,
                truncate(&s.name, 28),
                truncate(&s.sink, 20),
                s.volume,
                if s.is_muted { " muted" } else { "" }
            )
        })),

        Response::Sinks(sinks) => lines(sinks.iter().map(|s| {
            format!(
                "{:<28} {:>3}%{}{}",
                s.name,
                s.volume,
                if s.is_muted { " muted" } else { "" },
                if s.managed { " [managed]" } else { "" }
            )
        })),

        Response::SinkName(name) => name.clone(),

        Response::Route(route) => match route {
            Some(r) => format!("{} -> {} ({})", r.sink, r.device, r.description),
            None => "no route".into(),
        },

        Response::OutputDevices(devices) => lines(
            devices
                .iter()
                .map(|d| format!("{:<40} {}", d.name, d.description)),
        ),

        Response::Ports(ports) => lines(ports.iter().map(|p| {
            format!(
                "{:<6} {:<32} {:<12} {:?}",
                p.id, p.node_name, p.port_name, p.direction
            )
        })),

        Response::Links(links) => lines(
            links
                .iter()
                .map(|l| format!("{} -> {}", port(&l.source), port(&l.target))),
        ),

        Response::Devices(devices) => lines(devices.iter().map(|d| {
            format!(
                "{:04x}:{:04x} {:<28} {:?}",
                d.id.vendor_id, d.id.product_id, d.name, d.presence
            )
        })),

        Response::SelectedDevice(id) => match id {
            Some(id) => format!("{:04x}:{:04x}", id.vendor_id, id.product_id),
            None => "none".into(),
        },

        Response::UserDevices(devices) => lines(devices.iter().map(|d| {
            format!(
                "{:<24} {}:{}",
                d.name,
                d.vendor_id.as_deref().unwrap_or("-"),
                d.product_id.as_deref().unwrap_or("-")
            )
        })),

        Response::ChatMix(mix) => format!(
            "{} ({})",
            mix.value,
            if mix.manual { "manual" } else { "wheel" }
        ),

        Response::EqState(state) => {
            let mut out = Vec::new();
            if state.safe_mode {
                out.push("safe mode: the filter chain is bypassed".to_string());
            }
            let mut chains: Vec<_> = state.chains.iter().collect();
            chains.sort_by_key(|(id, _)| format!("{id:?}"));
            for (id, chain) in chains {
                out.push(format!(
                    "{:?}: {} preamp {:+.1} dB, {} bands",
                    id,
                    if chain.enabled { "on " } else { "off" },
                    chain.preamp_db,
                    chain.bands.len()
                ));
                for (i, band) in chain.bands.iter().enumerate() {
                    out.push(format!(
                        "  [{i:>2}] {:>8.1} Hz {:+6.1} dB  Q {:.2}  {:?}{}",
                        band.freq,
                        band.gain_db,
                        band.q,
                        band.filter_type,
                        if band.enabled { "" } else { " (off)" }
                    ));
                }
            }
            out.join("\n")
        }

        Response::EqPresets(presets) => lines(presets.iter().map(|p| {
            format!(
                "{:<20}{}",
                p.name,
                if p.builtin { " [builtin]" } else { "" }
            )
        })),

        Response::SystemDeps(deps) => [
            format!("pipewire   {}", mark(deps.pipewire)),
            format!("pactl      {}", mark(deps.pactl)),
            format!("pw-link    {}", mark(deps.pw_link)),
            format!("libhidapi  {}", mark(deps.libhidapi)),
        ]
        .join("\n"),

        Response::UdevStatus(status) => {
            if status.installed {
                "udev rule installed".into()
            } else {
                let mut out = vec![
                    "udev rule missing or outdated; run:".to_string(),
                    String::new(),
                ];
                if let Some(cmd) = &status.install_command {
                    out.push(format!("    {}", shell_join(cmd)));
                }
                out.join("\n")
            }
        }

        Response::Icon(icon) => match icon {
            Some(data) => format!("data:image/png;base64,{data}"),
            None => "no icon".into(),
        },
    }
}

fn port(p: &PortRef) -> String {
    match p.id {
        Some(id) => format!("{}:{} (#{id})", p.node_name, p.port_name),
        None => format!("{}:{}", p.node_name, p.port_name),
    }
}

fn lines(items: impl Iterator<Item = String>) -> String {
    let collected: Vec<String> = items.collect();
    if collected.is_empty() {
        "(none)".into()
    } else {
        collected.join("\n")
    }
}

fn mark(present: bool) -> &'static str {
    if present {
        "found"
    } else {
        "MISSING"
    }
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else {
        s.chars().take(width.saturating_sub(1)).collect::<String>() + "…"
    }
}

/// Quote a command for a human to paste, matching what the daemon generated.
fn shell_join(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_/.=:".contains(c))
            {
                a.clone()
            } else {
                format!("'{}'", a.replace('\'', r"'\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
