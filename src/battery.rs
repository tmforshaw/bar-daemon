use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    ICON_END, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
};

#[derive(PartialEq, Eq, Debug)]
pub enum BatteryState {
    FullyCharged = 0,
    Charging = 1,
    Discharging = 2,
    NotCharging = 3,
}

#[derive(Subcommand)]
pub enum BatteryGetCommands {
    #[command(alias = "s")]
    State,
    #[command(alias = "per", alias = "p")]
    Percent,
    #[command(alias = "t")]
    Time,
    #[command(alias = "i")]
    Icon,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BatteryItem {
    State,
    Percent,
    Time,
    Icon,
    All,
}

const BAT_STATE_STRINGS: &[&str] = &["Fully Charged", "Charging", "Discharging", "Not Charging"];
const BAT_NOTIFY_VALUES: &[u32] = &[5, 15, 20, 30];

pub struct Battery;

impl Battery {
    // TODO remove pub
    pub fn get() -> Result<(BatteryState, u32, String), DaemonError> {
        let output = command::run("acpi", &["-b"])?;
        let output_split = output.split(",");

        let state = Self::get_state_from_split(output_split.clone())?;
        let percent = Self::get_percent_from_split(output_split.clone())?;
        let time_remaining = Self::get_time_from_split(output_split)?;

        Ok((state, percent, time_remaining))
    }

    fn get_state_from_split(mut output_split: std::str::Split<&str>) -> Result<BatteryState, DaemonError> {
        match output_split
            .next()
            .ok_or(DaemonError::ParseError(output_split.collect::<String>()))?
            .trim_start_matches("Battery 0: ")
        {
            "Fully charged" => Ok(BatteryState::FullyCharged),
            "Charging" => Ok(BatteryState::Charging),
            "Discharging" => Ok(BatteryState::Discharging),
            "Not charging" => Ok(BatteryState::NotCharging),
            state_string => Err(DaemonError::ParseError(state_string.to_string())),
        }
    }

    pub fn get_state() -> Result<BatteryState, DaemonError> {
        let output = command::run("acpi", &["-b"])?;
        let output_split = output.split(",");

        Self::get_state_from_split(output_split)
    }

    fn get_percent_from_split(mut output_split: std::str::Split<&str>) -> Result<u32, DaemonError> {
        Ok(output_split
            .nth(1)
            .ok_or(DaemonError::ParseError(output_split.collect::<String>()))?
            .trim()
            .trim_end_matches("%")
            .parse::<u32>()?)
    }

    pub fn get_percent() -> Result<u32, DaemonError> {
        let output = command::run("acpi", &["-b"])?;
        let output_split = output.split(",");

        Self::get_percent_from_split(output_split)
    }

    fn get_time_from_split(mut output_split: std::str::Split<&str>) -> Result<String, DaemonError> {
        // Return empty string if the time part of the output_split is not present
        let Some(time_string_unsplit) = output_split.nth(2) else {
            return Ok(String::new());
        };

        Ok(time_string_unsplit
            .split_whitespace()
            .nth(0)
            .ok_or(DaemonError::ParseError(output_split.collect::<String>()))?
            .to_string())
    }

    pub fn get_time() -> Result<String, DaemonError> {
        let output = command::run("acpi", &["-b"])?;
        let output_split = output.split(",");

        Self::get_time_from_split(output_split)
    }

    pub fn get_icon(state: &BatteryState, percent: u32) -> String {
        if state == &BatteryState::NotCharging {
            String::from("battery-missing")
        } else {
            format!(
                "battery-level-{}{}",
                percent / 10 * 10,
                match state {
                    BatteryState::Charging => "-charging",
                    BatteryState::FullyCharged => "-charged",
                    _ => "",
                }
            )
        }
    }

    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let (state, percent, time) = Self::get()?;
        let icon = Self::get_icon(&state, percent);

        Ok(vec![
            ("state".to_string(), BAT_STATE_STRINGS[state as usize].to_string()),
            ("percent".to_string(), percent.to_string()),
            ("time".to_string(), time),
            ("icon".to_string(), format!("{icon}{ICON_END}")),
        ])
    }

    pub fn parse_item(item: DaemonItem, battery_item: BatteryItem) -> Result<DaemonReply, DaemonError> {
        Ok(
            // Get value
            match battery_item {
                BatteryItem::State => DaemonReply::Value {
                    item,
                    value: BAT_STATE_STRINGS[Self::get_state()? as usize].to_string(),
                },
                BatteryItem::Percent => DaemonReply::Value {
                    item,
                    value: Self::get_percent()?.to_string(),
                },
                BatteryItem::Time => DaemonReply::Value {
                    item,
                    value: Self::get_time()?.to_string(),
                },
                BatteryItem::Icon => {
                    let (state, percent, _) = Self::get()?;

                    DaemonReply::Value {
                        item,
                        value: Self::get_icon(&state, percent),
                    }
                }
                BatteryItem::All => DaemonReply::Tuples {
                    item,
                    tuples: Self::get_tuples()?,
                },
            },
        )
    }

    pub fn match_get_commands(commands: Option<BatteryGetCommands>) -> DaemonMessage {
        DaemonMessage::Get {
            item: match commands {
                Some(commands) => match commands {
                    BatteryGetCommands::State => DaemonItem::Battery(BatteryItem::State),
                    BatteryGetCommands::Percent => DaemonItem::Battery(BatteryItem::Percent),
                    BatteryGetCommands::Time => DaemonItem::Battery(BatteryItem::Time),
                    BatteryGetCommands::Icon => DaemonItem::Battery(BatteryItem::Icon),
                },
                None => DaemonItem::Battery(BatteryItem::All),
            },
        }
    }

    pub fn notify(prev_percent: u32) -> Result<(), DaemonError> {
        let (state, current_percent, _) = Self::get()?;
        let icon = Self::get_icon(&state, current_percent);

        if current_percent < prev_percent && state == BatteryState::Discharging {
            for &value in BAT_NOTIFY_VALUES.iter().rev() {
                if current_percent == value {
                    command::run(
                        "dunstify",
                        &[
                            "-u",
                            "-normal",
                            "-t",
                            NOTIFICATION_TIMEOUT.to_string().as_str(),
                            "-i",
                            format!("{icon}{ICON_END}").as_str(),
                            "-r",
                            NOTIFICATION_ID.to_string().as_str(),
                            "-h",
                            format!("int:value:{current_percent}").as_str(),
                            "Battery: ",
                        ],
                    )?;
                }
            }
        }

        Ok(())
    }
}
