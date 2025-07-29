use clap::{Parser, Subcommand};

use crate::{
    battery::{Battery, BatteryGetCommands},
    bluetooth::{Bluetooth, BluetoothGetCommands, BluetoothSetCommands},
    brightness::{Brightness, BrightnessGetCommands, BrightnessSetCommands},
    daemon::{do_daemon, send_daemon_messaage},
    error::DaemonError,
    volume::{Volume, VolumeGetCommands, VolumeSetCommands},
};

#[derive(Parser)]
#[command(name = "bar_daemon", about = "A daemon which can be ran, and seperate instances can listen for changes, or get/set values", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: CliCommands,
}

#[derive(Subcommand)]
pub enum CliCommands {
    Get {
        #[command(subcommand)]
        commands: GetCommands,
    },
    Set {
        #[command(subcommand)]
        commands: SetCommands,
    },
    Listen,
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
}

pub async fn match_cli() -> Result<(), DaemonError> {
    let cli = Cli::parse();

    let message_to_send = match cli.commands {
        CliCommands::Get { commands } => match commands {
            GetCommands::Volume { commands } => Volume::match_get_commands(commands),
            GetCommands::Brightness { commands } => Brightness::match_get_commands(commands),
            GetCommands::Bluetooth { commands } => Bluetooth::match_get_commands(commands),
            GetCommands::Battery { commands } => Battery::match_get_commands(commands),
        },
        CliCommands::Set { commands } => match commands {
            SetCommands::Volume { commands } => Volume::match_set_commands(commands),
            SetCommands::Brightness { commands } => Brightness::match_set_commands(commands),
            SetCommands::Bluetooth { commands } => Bluetooth::match_set_commands(commands),
        },
        CliCommands::Listen => {
            println!("Listen");

            unreachable!()
        }
        CliCommands::Daemon => {
            do_daemon().await?;

            unreachable!()
        }
    };

    let reply = send_daemon_messaage(message_to_send).await?;
    println!("{reply:?}");

    Ok(())
}
