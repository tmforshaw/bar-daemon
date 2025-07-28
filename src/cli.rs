use clap::{Parser, Subcommand};

use crate::{
    daemon::{do_daemon, send_daemon_messaage, DaemonItem, DaemonMessage},
    error::DaemonError,
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
        #[arg()]
        value: String,
    },
}

#[derive(Subcommand)]
pub enum GetCommands {
    #[command(alias = "vol", alias = "v")]
    Volume,
}

pub async fn match_cli() -> Result<(), DaemonError> {
    let cli = Cli::parse();

    let message_to_send = match cli.commands {
        CliCommands::Get { commands } => match commands {
            GetCommands::Volume => DaemonMessage::Get {
                item: DaemonItem::Volume,
            },
        },
        CliCommands::Set { commands } => match commands {
            SetCommands::Volume { value } => DaemonMessage::Set {
                item: DaemonItem::Volume,
                value,
            },
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
