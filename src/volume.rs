use crate::{
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
};

use clap::Subcommand;
use log_to_linear::logarithmic_to_linear;
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
pub enum VolumeGetCommands {
    #[command(alias = "per", alias = "p")]
    Percent,
    #[command(alias = "m")]
    Mute,
    #[command(alias = "i")]
    Icon,
}

#[derive(Subcommand)]
pub enum VolumeSetCommands {
    #[command(alias = "per", alias = "p")]
    Percent {
        #[arg()]
        value: u32,
    },
    #[command(alias = "m")]
    Mute {
        #[arg()]
        value: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum VolumeItem {
    Percent,
    Mute,
    Icon,
    All,
}

pub struct Volume;

impl Volume {
    fn get() -> Result<(u32, bool), DaemonError> {
        let output = command::run("wpctl", &["get-volume", "@DEFAULT_SINK@"])?;
        let mut output_split = output.trim_start_matches("Volume: ").split(' '); // Left with only volume number, and muted status

        let percent = if let Some(volume_str) = output_split.next() {
            volume_str.parse::<f64>()? * 100.
        } else {
            return Err(DaemonError::ParseError(output));
        };

        let linear_percent = logarithmic_to_linear(percent) as u32;

        let mute = output_split.next().is_some();

        Ok((linear_percent, mute))
    }

    pub fn get_icon(percent: u32, muted: bool) -> String {
        format!(
            "audio-volume-{}{}",
            if muted {
                "muted"
            } else {
                match percent {
                    0 => "muted",
                    1..=33 => "low",
                    34..=67 => "medium",
                    68..=100 => "high",
                    101.. => "overamplified",
                }
            },
            crate::ICON_EXT
        )
    }

    pub fn get_percent() -> Result<u32, DaemonError> {
        let (percent, _) = Self::get()?;

        Ok(percent)
    }

    pub fn get_mute() -> Result<bool, DaemonError> {
        let (_, mute) = Self::get()?;

        Ok(mute)
    }

    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let (percent, mute_state) = Self::get()?;
        let icon = Self::get_icon(percent, mute_state);

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("mute_state".to_string(), mute_state.to_string()),
            ("icon".to_string(), icon.to_string()),
        ])
    }

    pub fn parse_item(item: DaemonItem, value: Option<String>) -> Result<DaemonReply, DaemonError> {
        // TODO set options
        Ok(if let Some(_value) = value {
            // Set value
            todo!()
        } else {
            // Get value
            match item.clone() {
                DaemonItem::Volume(volume_item) => match volume_item {
                    VolumeItem::Percent => DaemonReply::Value {
                        item,
                        value: Self::get_percent()?.to_string(),
                    },
                    VolumeItem::Mute => DaemonReply::Value {
                        item,
                        value: Self::get_mute()?.to_string(),
                    },
                    VolumeItem::Icon => {
                        let (percent, muted) = Self::get()?;
                        DaemonReply::Value {
                            item,
                            value: Self::get_icon(percent, muted),
                        }
                    }
                    VolumeItem::All => DaemonReply::Tuples {
                        item,
                        tuples: Self::get_tuples()?,
                    },
                },
            }
        })
    }

    pub fn match_get_commands(commands: Option<VolumeGetCommands>) -> DaemonMessage {
        DaemonMessage::Get {
            item: match commands {
                Some(commands) => match commands {
                    VolumeGetCommands::Percent => DaemonItem::Volume(VolumeItem::Percent),
                    VolumeGetCommands::Mute => DaemonItem::Volume(VolumeItem::Mute),
                    VolumeGetCommands::Icon => DaemonItem::Volume(VolumeItem::Icon),
                },
                None => DaemonItem::Volume(VolumeItem::All),
            },
        }
    }

    pub fn match_set_commands(commands: VolumeSetCommands) -> DaemonMessage {
        match commands {
            VolumeSetCommands::Percent { value } => DaemonMessage::Set {
                item: DaemonItem::Volume(VolumeItem::Percent),
                value: value.to_string(),
            },
            VolumeSetCommands::Mute { value } => DaemonMessage::Set {
                item: DaemonItem::Volume(VolumeItem::Mute),
                value: value.to_string(),
            },
        }
    }
}
