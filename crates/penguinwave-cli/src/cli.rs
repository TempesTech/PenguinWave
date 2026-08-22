//! Command surface. One subcommand per protocol method.

use clap::{Parser, Subcommand, ValueEnum};
use penguinwave_proto::{EqChainId, FilterType};

#[derive(Parser, Debug)]
#[command(
    name = "penguinwave-cli",
    version,
    about = "Control the Penguin Wave audio daemon"
)]
pub struct Cli {
    /// Emit raw JSON instead of formatted output.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Full daemon state.
    Status,
    /// Stream playback applications.
    #[command(subcommand)]
    Stream(StreamCmd),
    /// Virtual and hardware sinks.
    #[command(subcommand)]
    Sink(SinkCmd),
    /// Ports and links.
    #[command(subcommand)]
    Graph(GraphCmd),
    /// Headsets.
    #[command(subcommand)]
    Device(DeviceCmd),
    /// Game/chat balance.
    #[command(subcommand)]
    Chatmix(ChatmixCmd),
    /// Parametric equalizer.
    #[command(subcommand)]
    Eq(EqCmd),
    /// Dependency and udev checks.
    #[command(subcommand)]
    System(SystemCmd),
    /// Stream events until interrupted.
    Watch {
        /// Event names to receive; defaults to all.
        #[arg(long, value_delimiter = ',')]
        events: Option<Vec<String>>,
    },
    /// Remove every sink the daemon manages.
    ///
    /// Manual only: stopping the daemon deliberately leaves them running.
    Teardown {
        /// Required, because this drops every stream routed to those sinks.
        #[arg(long)]
        confirm: bool,
    },
    /// Print a shell completion script.
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum StreamCmd {
    List,
    Move {
        index: u32,
        sink: String,
    },
    Unassign {
        index: u32,
    },
    SetVolume {
        index: u32,
        /// 0..=100.
        pct: u8,
    },
    SetMute {
        index: u32,
        /// `true` or `false`.
        // A positional bool needs an explicit Set action; clap's default flag
        // action panics when the command tree is built.
        #[arg(action = clap::ArgAction::Set)]
        mute: bool,
    },
    Icon {
        key: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SinkCmd {
    List,
    ListCustom,
    Create {
        name: String,
        #[arg(long)]
        display_name: Option<String>,
    },
    Delete {
        name: String,
    },
    SetVolume {
        sink: String,
        pct: u8,
    },
    Default,
    Route {
        sink: String,
        device: String,
    },
    CurrentRoute {
        sink: String,
    },
    Devices,
}

#[derive(Subcommand, Debug)]
pub enum GraphCmd {
    Ports {
        node: String,
    },
    Link {
        /// `node:port`.
        source: String,
        target: String,
    },
    Unlink {
        source: String,
        target: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum DeviceCmd {
    List,
    Selected,
    Select {
        /// `vendor:product` in hex, or `none` to clear.
        id: String,
    },
    ListUser,
    AddUser {
        name: String,
        #[arg(long)]
        vendor_id: Option<String>,
        #[arg(long)]
        product_id: Option<String>,
        #[arg(long)]
        sink: Option<String>,
    },
    RemoveUser {
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ChatmixCmd {
    Get,
    /// Pin the balance: 0 all chat, 100 all game.
    Set {
        value: u8,
    },
    /// Hand control back to the headset wheel.
    Auto,
}

#[derive(Subcommand, Debug)]
pub enum EqCmd {
    Get,
    Enable {
        chain: Chain,
    },
    Disable {
        chain: Chain,
    },
    SetBand {
        chain: Chain,
        index: u8,
        #[arg(long)]
        freq: f32,
        #[arg(long, allow_negative_numbers = true)]
        gain: f32,
        #[arg(long, default_value = "1.1")]
        q: f32,
        #[arg(long, value_enum, default_value = "peaking")]
        filter: Filter,
    },
    AddBand {
        chain: Chain,
        #[arg(long)]
        freq: f32,
        #[arg(long, allow_negative_numbers = true)]
        gain: f32,
        #[arg(long, default_value = "1.1")]
        q: f32,
        #[arg(long, value_enum, default_value = "peaking")]
        filter: Filter,
    },
    RemoveBand {
        chain: Chain,
        index: u8,
    },
    Preamp {
        chain: Chain,
        #[arg(allow_negative_numbers = true)]
        gain_db: f32,
    },
    Presets,
    SavePreset {
        chain: Chain,
        name: String,
    },
    ApplyPreset {
        chain: Chain,
        name: String,
    },
    DeletePreset {
        name: String,
    },
    /// Clear safe mode and retry the filter chain.
    ResetSafeMode,
}

#[derive(Subcommand, Debug)]
pub enum SystemCmd {
    Deps,
    Udev,
    /// Print the command to install the udev rule; never runs it.
    InstallUdev,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Chain {
    Game,
    Chat,
}

impl From<Chain> for EqChainId {
    fn from(c: Chain) -> Self {
        match c {
            Chain::Game => EqChainId::Game,
            Chain::Chat => EqChainId::Chat,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Filter {
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
}

impl From<Filter> for FilterType {
    fn from(f: Filter) -> Self {
        match f {
            Filter::Peaking => FilterType::Peaking,
            Filter::LowShelf => FilterType::LowShelf,
            Filter::HighShelf => FilterType::HighShelf,
            Filter::LowPass => FilterType::LowPass,
            Filter::HighPass => FilterType::HighPass,
            Filter::Notch => FilterType::Notch,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl From<Shell> for clap_complete::Shell {
    fn from(s: Shell) -> Self {
        match s {
            Shell::Bash => clap_complete::Shell::Bash,
            Shell::Zsh => clap_complete::Shell::Zsh,
            Shell::Fish => clap_complete::Shell::Fish,
        }
    }
}
