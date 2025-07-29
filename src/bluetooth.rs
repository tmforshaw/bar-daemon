use clap::{ArgAction, Subcommand};
use serde::{Deserialize, Serialize};

use crate::{
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    ICON_END, ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
};

#[derive(Subcommand)]
pub enum BluetoothGetCommands {
    #[command(alias = "s")]
    State,
    #[command(alias = "i")]
    Icon,
}

#[derive(Subcommand)]
pub enum BluetoothSetCommands {
    #[command(alias = "s")]
    State {
        #[arg(required = true, action = ArgAction::Set, value_parser = parse_bool)]
        value: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum BluetoothItem {
    State,
    Icon,
    All,
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        other => Err(format!("Invalid value '{other}' for boolean. Use true/false or 1/0.")),
    }
}

pub struct Bluetooth;

impl Bluetooth {
    pub fn get_state() -> Result<bool, DaemonError> {
        let output = command::run("bluetooth", &[])?;

        // Split the output and check if it is on or off
        output
            .clone()
            .split_whitespace()
            .nth(2)
            .map_or(Err(DaemonError::ParseError(output)), |state| Ok(state == "on"))
    }

    pub fn set_state(state: bool) -> Result<(), DaemonError> {
        command::run("bluetooth", &[(if state { "on" } else { "off" }).to_string().as_str()])?;

        Ok(())
    }

    pub fn get_icon(state: bool) -> String {
        format!("bluetooth-{}", if state { "active" } else { "disabled" })
    }

    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let state = Self::get_state()?;
        let icon = Self::get_icon(state);

        Ok(vec![
            ("state".to_string(), state.to_string()),
            ("icon".to_string(), format!("{icon}{ICON_EXT}")),
        ])
    }

    pub fn parse_item(
        item: DaemonItem,
        bluetooth_item: BluetoothItem,
        value: Option<String>,
    ) -> Result<DaemonReply, DaemonError> {
        Ok(if let Some(value) = value {
            let prev_state = Self::get_state()?;
            let new_state = value.parse::<bool>()?;

            // Set value
            if bluetooth_item == BluetoothItem::State {
                Self::set_state(new_state)?
            };

            if prev_state != new_state {
                // Do a notification
                Self::notify()?;
            }

            DaemonReply::Value {
                item,
                value: value.to_string(),
            }
        } else {
            // Get value
            match bluetooth_item {
                BluetoothItem::State => DaemonReply::Value {
                    item,
                    value: Self::get_state()?.to_string(),
                },
                BluetoothItem::Icon => {
                    let state = Self::get_state()?;

                    DaemonReply::Value {
                        item,
                        value: Self::get_icon(state).to_string(),
                    }
                }
                BluetoothItem::All => DaemonReply::Tuples {
                    item,
                    tuples: Self::get_tuples()?,
                },
            }
        })
    }

    pub fn match_get_commands(commands: Option<BluetoothGetCommands>) -> DaemonMessage {
        DaemonMessage::Get {
            item: match commands {
                Some(commands) => match commands {
                    BluetoothGetCommands::State => DaemonItem::Bluetooth(BluetoothItem::State),
                    BluetoothGetCommands::Icon => DaemonItem::Bluetooth(BluetoothItem::Icon),
                },
                None => DaemonItem::Bluetooth(BluetoothItem::All),
            },
        }
    }

    pub fn match_set_commands(commands: BluetoothSetCommands) -> DaemonMessage {
        match commands {
            BluetoothSetCommands::State { value } => DaemonMessage::Set {
                item: DaemonItem::Bluetooth(BluetoothItem::State),
                value: value.to_string(),
            },
        }
    }

    pub fn notify() -> Result<(), DaemonError> {
        let state = Self::get_state()?;

        let icon = Self::get_icon(state);

        command::run(
            "dunstify",
            &[
                "-u",
                "normal",
                "-r",
                format!("{NOTIFICATION_ID}").as_str(),
                "-i",
                format!("{}{ICON_END}", icon.trim()).as_str(),
                "-t",
                format!("{NOTIFICATION_TIMEOUT}").as_str(),
                format!("Bluetooth: {}", if state { "on" } else { "off" }).as_str(),
            ],
        )?;

        Ok(())
    }
}
