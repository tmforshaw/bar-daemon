use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    ICON_END, ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
};

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum FanState {
    Performance = 0,
    Balanced = 1,
    Quiet = 2,
}

const FAN_STATE_STRINGS: &[&str] = &["Performance", "Balanced", "Quiet"];

#[derive(Subcommand)]
pub enum FanProfileGetCommands {
    #[command(alias = "prof", alias = "p")]
    Profile,
    #[command(alias = "i")]
    Icon,
}

#[derive(Subcommand)]
pub enum FanProfileSetCommands {
    #[command(alias = "prof", alias = "p")]
    Profile {
        #[arg()]
        value: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FanProfileItem {
    Profile,
    Icon,
}

pub struct FanProfile;

impl FanProfile {
    /// # Errors
    /// Returns an error if the command can't be ran
    /// Returns an error if the correct line can't be found
    /// Returns an error if the correct part of the line can't be found
    /// Returns an error if the profile string can't be converted to ``FanState``
    pub fn get_profile() -> Result<FanState, DaemonError> {
        // Find the correct line where the fan profile is
        let output = command::run("asusctl", &["profile", "-p"])?;
        let output_line = output.lines().nth(1).ok_or_else(|| DaemonError::ParseError(output.clone()))?;

        // Match the profile string
        Ok(
            match output_line
                .split_whitespace()
                .nth(3)
                .ok_or_else(|| DaemonError::ParseError(output_line.to_string()))?
            {
                "Performance" => FanState::Performance,
                "Balanced" => FanState::Balanced,
                "Quiet" => FanState::Quiet,
                incorrect => Err(DaemonError::ParseError(incorrect.to_string()))?,
            },
        )
    }

    /// # Errors
    /// Returns an error if the given value is not a valid profile
    /// Returns an error if the set command can't be ran
    pub fn set_profile(profile_string: &str) -> Result<(), DaemonError> {
        let new_profile = if FAN_STATE_STRINGS.contains(&profile_string.trim()) {
            // A new profile has been set
            profile_string.trim()
        } else {
            // Profile is set via cyclic function
            let current_profile = Self::get_profile()?;

            match profile_string {
                "next" => FAN_STATE_STRINGS[(current_profile as usize + 1) % FAN_STATE_STRINGS.len()],
                "prev" => {
                    let new_profile_index = (current_profile as usize)
                        .checked_sub(1)
                        .unwrap_or(FAN_STATE_STRINGS.len() - 1);

                    FAN_STATE_STRINGS[new_profile_index]
                }
                incorrect => Err(DaemonError::ParseError(incorrect.to_string()))?,
            }
        };

        command::run("asusctl", &["profile", "-P", new_profile])?;

        Ok(())
    }

    #[must_use]
    pub fn get_icon() -> String {
        format!("sensors-fan{ICON_END}")
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let profile = Self::get_profile()?;
        let icon = Self::get_icon();

        Ok(vec![
            ("profile".to_string(), FAN_STATE_STRINGS[profile as usize].to_string()),
            ("icon".to_string(), format!("{icon}{ICON_EXT}")),
        ])
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn parse_item(
        item: DaemonItem,
        fan_profile_item: &FanProfileItem,
        value: Option<String>,
    ) -> Result<DaemonReply, DaemonError> {
        Ok(if let Some(value) = value {
            let prev_profile = Self::get_profile()?;

            // Set value
            if matches!(fan_profile_item, FanProfileItem::Profile) {
                Self::set_profile(value.as_str())?;
            }

            let new_profile = Self::get_profile()?;

            if prev_profile != new_profile {
                // Do a notification
                Self::notify()?;
            }

            DaemonReply::Value { item, value }
        } else {
            // Get value
            match fan_profile_item {
                FanProfileItem::Profile => DaemonReply::Value {
                    item,
                    value: FAN_STATE_STRINGS[Self::get_profile()? as usize].to_string(),
                },
                FanProfileItem::Icon => DaemonReply::Value {
                    item,
                    value: Self::get_icon(),
                },
            }
        })
    }

    #[must_use]
    pub const fn match_get_commands(commands: &FanProfileGetCommands) -> DaemonMessage {
        DaemonMessage::Get {
            item: match commands {
                FanProfileGetCommands::Profile => DaemonItem::FanProfile(FanProfileItem::Profile),
                FanProfileGetCommands::Icon => DaemonItem::FanProfile(FanProfileItem::Icon),
            },
        }
    }

    #[must_use]
    pub fn match_set_commands(commands: FanProfileSetCommands) -> DaemonMessage {
        match commands {
            FanProfileSetCommands::Profile { value } => DaemonMessage::Set {
                item: DaemonItem::FanProfile(FanProfileItem::Profile),
                value,
            },
        }
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn notify() -> Result<(), DaemonError> {
        let profile = Self::get_profile()?;
        let icon = Self::get_icon();

        command::run(
            "dunstify",
            &[
                "-u",
                "-normal",
                "-t",
                NOTIFICATION_TIMEOUT.to_string().as_str(),
                "-i",
                icon.as_str(),
                "-r",
                NOTIFICATION_ID.to_string().as_str(),
                format!("Fan Profile: {}", FAN_STATE_STRINGS[profile as usize]).as_str(),
            ],
        )?;

        Ok(())
    }
}
