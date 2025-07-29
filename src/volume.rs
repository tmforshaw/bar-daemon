use crate::{
    cli::parse_bool,
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    ICON_END, ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
};

use clap::{ArgAction, Subcommand};
use log_to_linear::{linear_to_logarithmic, logarithmic_to_linear};
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
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    #[command(alias = "m")]
    Mute {
        #[arg(action = ArgAction::Set, value_parser = parse_bool)]
        value: Option<bool>,
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
        // Get the volume and mute status as a string
        let output = command::run("wpctl", &["get-volume", "@DEFAULT_SINK@"])?;
        let mut output_split = output.trim_start_matches("Volume: ").split_whitespace(); // Left with only volume number, and muted status

        // Take the first part of the split (The numerical part) then convert to linear percentage
        let percent = if let Some(volume_str) = output_split.next() {
            logarithmic_to_linear(volume_str.parse::<f64>()? * 100.) as u32
        } else {
            return Err(DaemonError::ParseError(output));
        };

        // Get the mute state from the second part of the split
        let mute = output_split.next().is_some();

        Ok((percent, mute))
    }

    #[must_use]
    pub fn get_icon(percent: u32, muted: bool) -> String {
        format!(
            "audio-volume-{}",
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
            }
        )
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_percent() -> Result<u32, DaemonError> {
        let (percent, _) = Self::get()?;

        Ok(percent)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_mute() -> Result<bool, DaemonError> {
        let (_, mute) = Self::get()?;

        Ok(mute)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn set_percent(percent_string: &str) -> Result<(), DaemonError> {
        // If the percentage is a change, figure out the true percentage
        let linear_percent = if percent_string.starts_with('+') || percent_string.starts_with('-') {
            // Get the value of the percentage
            let delta_percent = percent_string
                .trim_start_matches('+')
                .trim_start_matches('-')
                .to_string()
                .parse::<f64>()?;

            let current_percent = f64::from(Self::get_percent()?);

            // Depending on the first char, add or subtract the percentage
            (current_percent
                + match percent_string.chars().next() {
                    Some('+') => delta_percent,
                    Some('-') => -delta_percent,
                    _ => 0.0,
                })
            .clamp(0.0, 100.0)
        } else {
            percent_string.parse::<f64>()?
        };

        let logarithmic_percent = linear_to_logarithmic(linear_percent);

        // Set the volume
        let _ = command::run(
            "wpctl",
            &["set-volume", "@DEFAULT_SINK@", format!("{logarithmic_percent}%").as_str()],
        )?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn set_mute(mute_string: &str) -> Result<(), DaemonError> {
        let mute = if mute_string == "toggle" {
            mute_string.to_string()
        } else {
            u8::from(mute_string.parse::<bool>()?).to_string()
        };

        // Set the mute state
        let _ = command::run("wpctl", &["set-mute", "@DEFAULT_SINK@", mute.as_str()])?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let (percent, mute_state) = Self::get()?;
        let icon = Self::get_icon(percent, mute_state);

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("mute_state".to_string(), mute_state.to_string()),
            ("icon".to_string(), format!("{icon}{ICON_EXT}")),
        ])
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn parse_item(item: DaemonItem, volume_item: &VolumeItem, value: Option<String>) -> Result<DaemonReply, DaemonError> {
        Ok(if let Some(value) = value {
            let prev_percent_and_mute = Self::get()?;

            println!("{prev_percent_and_mute:?}");

            // Set value
            match volume_item {
                VolumeItem::Percent => Self::set_percent(value.as_str())?,
                VolumeItem::Mute => Self::set_mute(value.as_str())?,
                _ => {}
            }

            let new_percent_and_mute = Self::get()?;
            println!("{new_percent_and_mute:?}");

            if prev_percent_and_mute != new_percent_and_mute {
                // Do a notification
                Self::notify()?;
            }

            DaemonReply::Value { item, value }
        } else {
            // Get value
            match volume_item {
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
            }
        })
    }

    #[must_use]
    pub const fn match_get_commands(commands: &Option<VolumeGetCommands>) -> DaemonMessage {
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

    #[must_use]
    pub fn match_set_commands(commands: VolumeSetCommands) -> DaemonMessage {
        match commands {
            VolumeSetCommands::Percent { value } => DaemonMessage::Set {
                item: DaemonItem::Volume(VolumeItem::Percent),
                value,
            },
            VolumeSetCommands::Mute { value } => DaemonMessage::Set {
                item: DaemonItem::Volume(VolumeItem::Mute),
                value: value.map_or("toggle".to_string(), |value| value.to_string()),
            },
        }
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn notify() -> Result<(), DaemonError> {
        let (percent, muted) = Self::get()?;

        let icon = Self::get_icon(percent, muted);

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
                "-h",
                format!("int:value:{percent}").as_str(),
                "Volume: ",
            ],
        )?;

        Ok(())
    }
}
