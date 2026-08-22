//! Penguin Wave command line client.

mod cli;
mod client;
mod render;

use clap::{CommandFactory, Parser};
use cli::{ChatmixCmd, Cli, Command, DeviceCmd, EqCmd, GraphCmd, SinkCmd, StreamCmd, SystemCmd};
use client::Client;
use penguinwave_proto::{
    DeviceId, EqBand, ErrorKind, PortRef, PwError, Request, Response, SinkConfig, StreamRef,
    UserDevice,
};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Cli::parse();

    // Completions need no daemon; generating them must work on a cold install.
    if let Command::Completions { shell } = args.command {
        clap_complete::generate(
            clap_complete::Shell::from(shell),
            &mut Cli::command(),
            "penguinwave-cli",
            &mut std::io::stdout(),
        );
        return ExitCode::SUCCESS;
    }

    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("penguinwave-cli: {}", e.msg);
            ExitCode::from(e.kind.exit_code() as u8)
        }
    }
}

fn run(args: Cli) -> Result<(), PwError> {
    let mut client = Client::connect()?;

    if let Command::Watch { events } = &args.command {
        return watch(&mut client, events.clone(), args.json);
    }
    if let Command::Teardown { confirm } = args.command {
        return teardown(&mut client, confirm, args.json);
    }

    let response = client.call(request(&args.command)?)?;
    // `chatmix get` has no method of its own; it reads the snapshot.
    let response = match (&args.command, response) {
        (Command::Chatmix(ChatmixCmd::Get), Response::Snapshot(s)) => Response::ChatMix(s.chatmix),
        (_, other) => other,
    };
    print(&response, args.json);
    Ok(())
}

fn print(response: &Response, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(response).unwrap_or_default()
        );
    } else {
        println!("{}", render::render(response));
    }
}

fn watch(client: &mut Client, events: Option<Vec<String>>, json: bool) -> Result<(), PwError> {
    client.subscribe(events.unwrap_or_else(|| vec!["*".into()]))?;
    loop {
        let frame = client.next_event()?;
        if json {
            println!("{}", serde_json::to_string(&frame).unwrap_or_default());
        } else {
            println!("{}", frame.event.name());
        }
    }
}

/// Remove every managed sink.
///
/// Not wired to daemon lifecycle: stopping the daemon deliberately leaves the
/// sinks running so audio survives a restart.
fn teardown(client: &mut Client, confirm: bool, json: bool) -> Result<(), PwError> {
    if !confirm {
        return Err(PwError::new(
            ErrorKind::BadRequest,
            "teardown drops every stream routed to the managed sinks; pass --confirm",
        ));
    }
    let Response::Sinks(sinks) = client.call(Request::SinkListCustom)? else {
        return Err(PwError::new(
            ErrorKind::Internal,
            "unexpected response kind",
        ));
    };
    for sink in &sinks {
        client.call(Request::SinkDelete {
            name: sink.name.clone(),
        })?;
    }
    print(&Response::Sinks(sinks), json);
    Ok(())
}

fn stream_ref(index: u32) -> StreamRef {
    // The daemon revalidates before mutating, so an index alone is enough to
    // address a stream from a shell.
    StreamRef {
        index,
        app_name: String::new(),
        pid: None,
    }
}

fn parse_port(spec: &str) -> Result<PortRef, PwError> {
    let (node, port) = spec.split_once(':').ok_or_else(|| {
        PwError::new(
            ErrorKind::BadRequest,
            format!("expected node:port, got {spec:?}"),
        )
    })?;
    Ok(PortRef {
        node_name: node.to_string(),
        port_name: port.to_string(),
        id: None,
    })
}

fn parse_device_id(spec: &str) -> Result<Option<DeviceId>, PwError> {
    if spec.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let bad = || {
        PwError::new(
            ErrorKind::BadRequest,
            format!("expected vendor:product in hex, got {spec:?}"),
        )
    };
    let (v, p) = spec.split_once(':').ok_or_else(bad)?;
    Ok(Some(DeviceId::new(
        u16::from_str_radix(v.trim_start_matches("0x"), 16).map_err(|_| bad())?,
        u16::from_str_radix(p.trim_start_matches("0x"), 16).map_err(|_| bad())?,
    )))
}

fn band(freq: f32, gain: f32, q: f32, filter: cli::Filter) -> EqBand {
    EqBand {
        freq,
        gain_db: gain,
        q,
        filter_type: filter.into(),
        enabled: true,
    }
}

fn request(command: &Command) -> Result<Request, PwError> {
    Ok(match command {
        Command::Completions { .. } | Command::Watch { .. } | Command::Teardown { .. } => {
            unreachable!("handled before dispatch")
        }
        Command::Status => Request::SessionSnapshot,

        Command::Stream(cmd) => match cmd {
            StreamCmd::List => Request::StreamList,
            StreamCmd::Move { index, sink } => Request::StreamMove {
                stream: stream_ref(*index),
                sink: sink.clone(),
            },
            StreamCmd::Unassign { index } => Request::StreamUnassign {
                stream: stream_ref(*index),
            },
            StreamCmd::SetVolume { index, pct } => Request::StreamSetVolume {
                stream: stream_ref(*index),
                pct: *pct,
            },
            StreamCmd::SetMute { index, mute } => Request::StreamSetMute {
                stream: stream_ref(*index),
                mute: *mute,
            },
            StreamCmd::Icon { key } => Request::StreamIcon { key: key.clone() },
        },

        Command::Sink(cmd) => match cmd {
            SinkCmd::List => Request::SinkListCustom,
            SinkCmd::ListCustom => Request::SinkListCustom,
            SinkCmd::Create { name, display_name } => Request::SinkCreate {
                config: SinkConfig {
                    name: name.clone(),
                    display_name: display_name.clone().unwrap_or_else(|| name.clone()),
                },
            },
            SinkCmd::Delete { name } => Request::SinkDelete { name: name.clone() },
            SinkCmd::SetVolume { sink, pct } => Request::SinkSetVolume {
                sink: sink.clone(),
                pct: *pct,
            },
            SinkCmd::Default => Request::SinkDefault,
            SinkCmd::Route { sink, device } => Request::SinkRouteToDevice {
                sink: sink.clone(),
                device: device.clone(),
            },
            SinkCmd::CurrentRoute { sink } => Request::SinkCurrentRoute { sink: sink.clone() },
            SinkCmd::Devices => Request::SinkListOutputDevices,
        },

        Command::Graph(cmd) => match cmd {
            GraphCmd::Ports { node } => Request::GraphListNodePorts { node: node.clone() },
            GraphCmd::Link { source, target } => Request::GraphLink {
                source: parse_port(source)?,
                target: parse_port(target)?,
            },
            GraphCmd::Unlink { source, target } => Request::GraphUnlink {
                source: parse_port(source)?,
                target: parse_port(target)?,
            },
        },

        Command::Device(cmd) => match cmd {
            DeviceCmd::List => Request::DeviceListSupported,
            DeviceCmd::Selected => Request::DeviceGetSelected,
            DeviceCmd::Select { id } => Request::DeviceSetSelected {
                device: parse_device_id(id)?,
            },
            DeviceCmd::ListUser => Request::DeviceListUser,
            DeviceCmd::AddUser {
                name,
                vendor_id,
                product_id,
                sink,
            } => Request::DeviceAddUser {
                device: UserDevice {
                    name: name.clone(),
                    vendor_id: vendor_id.clone(),
                    product_id: product_id.clone(),
                    pipewire_sink: sink.clone(),
                },
            },
            DeviceCmd::RemoveUser { name } => Request::DeviceRemoveUser { name: name.clone() },
        },

        Command::Chatmix(cmd) => match cmd {
            ChatmixCmd::Get => Request::SessionSnapshot,

            ChatmixCmd::Set { value } => Request::ChatmixSetManual {
                value: Some(*value),
            },
            ChatmixCmd::Auto => Request::ChatmixSetManual { value: None },
        },

        Command::Eq(cmd) => match cmd {
            EqCmd::Get => Request::EqGetState,
            EqCmd::Enable { chain } => Request::EqSetChainEnabled {
                chain: (*chain).into(),
                enabled: true,
            },
            EqCmd::Disable { chain } => Request::EqSetChainEnabled {
                chain: (*chain).into(),
                enabled: false,
            },
            EqCmd::SetBand {
                chain,
                index,
                freq,
                gain,
                q,
                filter,
            } => Request::EqSetBand {
                chain: (*chain).into(),
                index: *index,
                band: band(*freq, *gain, *q, *filter),
            },
            EqCmd::AddBand {
                chain,
                freq,
                gain,
                q,
                filter,
            } => Request::EqAddBand {
                chain: (*chain).into(),
                band: band(*freq, *gain, *q, *filter),
            },
            EqCmd::RemoveBand { chain, index } => Request::EqRemoveBand {
                chain: (*chain).into(),
                index: *index,
            },
            EqCmd::Preamp { chain, gain_db } => Request::EqSetPreamp {
                chain: (*chain).into(),
                preamp_db: *gain_db,
            },
            EqCmd::Presets => Request::EqListPresets,
            EqCmd::SavePreset { chain, name } => Request::EqSavePreset {
                name: name.clone(),
                chain: (*chain).into(),
            },
            EqCmd::ApplyPreset { chain, name } => Request::EqApplyPreset {
                name: name.clone(),
                chain: (*chain).into(),
            },
            EqCmd::DeletePreset { name } => Request::EqDeletePreset { name: name.clone() },
            EqCmd::ResetSafeMode => Request::EqResetSafeMode,
        },

        Command::System(cmd) => match cmd {
            SystemCmd::Deps => Request::SystemCheckDeps,
            SystemCmd::Udev => Request::SystemCheckUdev,
            SystemCmd::InstallUdev => Request::SystemInstallUdev,
        },
    })
}
