use clap::{Parser, Subcommand};

use crate::{
    battery::{Battery, BatteryGetCommands},
    bluetooth::{Bluetooth, BluetoothGetCommands, BluetoothSetCommands, BluetoothUpdateCommands},
    brightness::{Brightness, BrightnessGetCommands, BrightnessSetCommands, BrightnessUpdateCommands},
    daemon::{do_daemon, send_daemon_messaage, DaemonItem, DaemonMessage},
    error::DaemonError,
    fan_profile::{FanProfile, FanProfileGetCommands, FanProfileSetCommands, FanProfileUpdateCommands},
    listener::listen,
    ram::{Ram, RamGetCommands},
    volume::{Volume, VolumeGetCommands, VolumeSetCommands, VolumeUpdateCommands},
};

#[derive(Parser)]
#[command(name = "bar_daemon", about = "A daemon which can be ran, and seperate instances can listen for changes, or get/set values", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: CliCommands,
}

#[derive(Subcommand)]
pub enum CliCommands {
    #[command(alias = "g")]
    Get {
        #[command(subcommand)]
        commands: Option<GetCommands>,
    },
    #[command(alias = "s")]
    Set {
        #[command(subcommand)]
        commands: SetCommands,
    },
    #[command(alias = "u", alias = "up")]
    Update {
        #[command(subcommand)]
        commands: UpdateCommands,
    },
    #[command(alias = "lis", alias = "l")]
    Listen,
    #[command(alias = "dae", alias = "d")]
    Daemon,
}

#[derive(Subcommand)]
pub enum SetCommands {
    #[command(alias = "vol", alias = "v")]
    Volume {
        #[command(subcommand)]
        commands: VolumeSetCommands,
    },
    #[command(alias = "bri")]
    Brightness {
        #[command(subcommand)]
        commands: BrightnessSetCommands,
    },
    #[command(alias = "blue", alias = "blu", alias = "bt")]
    Bluetooth {
        #[command(subcommand)]
        commands: BluetoothSetCommands,
    },
    #[command(
        alias = "fan",
        alias = "profile",
        alias = "f",
        alias = "fp",
        alias = "prof",
        alias = "fanprof"
    )]
    FanProfile {
        #[command(subcommand)]
        commands: FanProfileSetCommands,
    },
}

// Should be the same as SetCommands, but the subcommands shouldn't take values
#[derive(Subcommand)]
pub enum UpdateCommands {
    #[command(alias = "vol", alias = "v")]
    Volume {
        #[command(subcommand)]
        commands: VolumeUpdateCommands,
    },
    #[command(alias = "bri")]
    Brightness {
        #[command(subcommand)]
        commands: BrightnessUpdateCommands,
    },
    #[command(alias = "blue", alias = "blu", alias = "bt")]
    Bluetooth {
        #[command(subcommand)]
        commands: BluetoothUpdateCommands,
    },
    #[command(
        alias = "fan",
        alias = "profile",
        alias = "f",
        alias = "fp",
        alias = "prof",
        alias = "fanprof"
    )]
    FanProfile {
        #[command(subcommand)]
        commands: FanProfileUpdateCommands,
    },
}

#[derive(Subcommand)]
pub enum GetCommands {
    #[command(alias = "vol", alias = "v")]
    Volume {
        #[command(subcommand)]
        commands: Option<VolumeGetCommands>,
    },
    #[command(alias = "bri")]
    Brightness {
        #[command(subcommand)]
        commands: Option<BrightnessGetCommands>,
    },
    #[command(alias = "blue", alias = "blu", alias = "bt")]
    Bluetooth {
        #[command(subcommand)]
        commands: Option<BluetoothGetCommands>,
    },
    #[command(alias = "bat")]
    Battery {
        #[command(subcommand)]
        commands: Option<BatteryGetCommands>,
    },
    #[command(alias = "r")]
    Ram {
        #[command(subcommand)]
        commands: Option<RamGetCommands>,
    },
    #[command(
        alias = "fan",
        alias = "profile",
        alias = "f",
        alias = "fp",
        alias = "prof",
        alias = "fanprof"
    )]
    FanProfile {
        #[command(subcommand)]
        commands: FanProfileGetCommands,
    },
    #[command(alias = "a")]
    All,
}

/// # Errors
/// Returns an error if the command for requested value cannot be spawned
/// Returns an error if values in the output of the command cannot be parsed
/// Returns an error if daemon or listener have received an error
pub async fn match_cli() -> Result<(), DaemonError> {
    let cli = Cli::parse();

    let message_to_send = match cli.commands {
        CliCommands::Get { commands } => {
            if let Some(commands) = commands {
                match commands {
                    GetCommands::Volume { commands } => Volume::match_get_commands(&commands),
                    GetCommands::Brightness { commands } => Brightness::match_get_commands(&commands),
                    GetCommands::Bluetooth { commands } => Bluetooth::match_get_commands(&commands),
                    GetCommands::Battery { commands } => Battery::match_get_commands(&commands),
                    GetCommands::Ram { commands } => Ram::match_get_commands(&commands),
                    GetCommands::FanProfile { commands } => FanProfile::match_get_commands(&commands),
                    GetCommands::All => DaemonMessage::Get { item: DaemonItem::All },
                }
            } else {
                DaemonMessage::Get { item: DaemonItem::All }
            }
        }
        CliCommands::Set { commands } => match commands {
            SetCommands::Volume { commands } => Volume::match_set_commands(commands),
            SetCommands::Brightness { commands } => Brightness::match_set_commands(commands),
            SetCommands::Bluetooth { commands } => Bluetooth::match_set_commands(&commands),
            SetCommands::FanProfile { commands } => FanProfile::match_set_commands(commands),
        },
        CliCommands::Update { commands } => match commands {
            UpdateCommands::Volume { commands } => Volume::match_update_commands(&commands),
            UpdateCommands::Brightness { commands } => Brightness::match_update_commands(&commands),
            UpdateCommands::Bluetooth { commands } => Bluetooth::match_update_commands(&commands),
            UpdateCommands::FanProfile { commands } => FanProfile::match_update_commands(&commands),
        },
        CliCommands::Listen => {
            listen().await?;

            return Ok(());
        }
        CliCommands::Daemon => {
            do_daemon().await?;

            // After the daemon has shutdown
            return Ok(());
        }
    };

    let reply = send_daemon_messaage(message_to_send).await?;
    println!("{reply:?}");

    Ok(())
}

/// # Errors
/// Returns an error if the bool was not in the correct format
pub fn parse_bool(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        other => Err(format!("Invalid value '{other}' for boolean. Use true/false or 1/0.")),
    }
}
