use crate::command;
use crate::command::ServerError;

use std::sync::Arc;

#[derive(PartialEq, Eq, Debug)]
enum BatteryState {
    FullyCharged,
    Charging,
    Discharging,
    NotCharging,
}

const BAT_STATES: &[&str] = &["Fully charged", "Charging", "Discharging", "Not Charging"];

const BAT_NOTIFY_VALUES: &[u32] = &[5, 15, 20, 30];
const BAT_NOTIFY_ID: u32 = 42069;
const BAT_NOTIFY_TIMEOUT: u32 = 10000;
const BAT_NOTIFY_MESSAGE: &str = "Battery: ";

pub struct Battery {}

impl Battery {
    fn get() -> Result<String, Arc<ServerError>> {
        Ok(command::run("acpi", &["-b"])?)
    }

    fn get_percent(battery_command: &str) -> Result<u32, Arc<ServerError>> {
        battery_command.split_whitespace().nth(3).map_or(
            {
                Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "battery".to_string(),
                    output: battery_command.to_string(),
                }))
            },
            |percentage| {
                percentage
                    .trim()
                    .trim_end_matches(',')
                    .trim_end_matches('%')
                    .parse()
                    .map_or_else(
                        |_| {
                            battery_command.split(',').nth(1).map_or(
                                {
                                    Err(Arc::from(ServerError::NotInOutput {
                                        looking_for: "battery".to_string(),
                                        output: battery_command.to_string(),
                                    }))
                                },
                                |percentage| {
                                    percentage.trim().trim_end_matches('%').parse().map_or_else(
                                        |_| {
                                            Err(Arc::from(ServerError::NotInOutput {
                                                looking_for: "battery".to_string(),
                                                output: battery_command.to_string(),
                                            }))
                                        },
                                        Ok,
                                    )
                                },
                            )
                        },
                        Ok,
                    )
            },
        )
    }

    fn get_time(battery_command: &str) -> Result<String, Arc<ServerError>> {
        let state = Self::get_state(battery_command)?;

        match state {
            BatteryState::NotCharging => {
                Ok(String::from(BAT_STATES[BatteryState::NotCharging as usize]))
            }
            BatteryState::FullyCharged => Ok(String::from(
                BAT_STATES[BatteryState::FullyCharged as usize],
            )),
            _ => battery_command.split_whitespace().nth(4).map_or_else(
                || Ok(String::new()),
                |time| Ok(time.trim().replace(':', " ")),
            ),
        }
    }

    fn get_state(battery_command: &str) -> Result<BatteryState, Arc<ServerError>> {
        battery_command.split_whitespace().nth(2).map_or_else(
            || {
                Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "battery state".to_string(),
                    output: battery_command.to_string(),
                }))
            },
            |state| match state.trim_end_matches(',') {
                "Full" => Ok(BatteryState::FullyCharged),
                "Charging" => Ok(BatteryState::Charging),
                "Discharging" => Ok(BatteryState::Discharging),
                "Not" => Ok(BatteryState::NotCharging),
                incorrect => Err(Arc::from(ServerError::UnknownValue {
                    incorrect: incorrect.to_string(),
                    object: "battery".to_string(),
                })),
            },
        )
    }

    fn get_icon(percent: u32, state: &BatteryState) -> String {
        if state == &BatteryState::NotCharging {
            format!("battery-missing{}", crate::ICON_EXT)
        } else {
            format!(
                "battery-{:0>3}{}{}",
                percent / 10 * 10,
                match state {
                    BatteryState::Charging => "-charging",
                    // BatteryState::FullyCharged => "-charged",
                    _ => "",
                },
                crate::ICON_EXT
            )
        }
    }

    pub fn notify(prev_percentage: String, icon: &str) -> Result<(), Arc<ServerError>> {
        // Parse the previous percentage
        let prev_u32 = match prev_percentage.parse::<u32>() {
            Ok(prev) => prev,
            Err(e) => {
                return Err(Arc::from(ServerError::StringParse {
                    debug_string: prev_percentage,
                    ty: "integer".to_string(),
                    e: Arc::from(e),
                }))
            }
        };

        let battery_command = Self::get()?;
        let curr_u32 = Self::get_percent(&battery_command)?;
        let charging_state = Self::get_state(&battery_command)?;

        if curr_u32 < prev_u32 && charging_state == BatteryState::Discharging {
            for &value in BAT_NOTIFY_VALUES.iter().rev() {
                if curr_u32 == value {
                    command::run(
                        "dunstify",
                        &[
                            "-u",
                            "normal",
                            "-t",
                            BAT_NOTIFY_TIMEOUT.to_string().as_str(),
                            "-i",
                            icon,
                            "-r",
                            BAT_NOTIFY_ID.to_string().as_str(),
                            "-h",
                            format!("int:value:{curr_u32}").as_str(),
                            BAT_NOTIFY_MESSAGE,
                        ],
                    )?;

                    break;
                }
            }
        }

        Ok(())
    }

    pub fn update(vec_tup: &[(String, String)]) -> Result<Vec<(String, String)>, Arc<ServerError>> {
        let prev_vec_tup = vec_tup.to_owned();

        Self::notify(prev_vec_tup[0].1.clone(), &vec_tup[3].1.clone())?;

        Self::get_json_tuple()
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        let battery_command = Self::get()?;

        let percent = Self::get_percent(&battery_command)?;
        let time = Self::get_time(&battery_command)?;
        let state = Self::get_state(&battery_command)?;
        let icon = Self::get_icon(percent, &state);

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("time".to_string(), time),
            ("state".to_string(), BAT_STATES[state as usize].to_string()),
            ("icon".to_string(), icon),
        ])
    }

    pub fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Arc<ServerError>> {
        args.first().map_or_else(
            || Err(Arc::from(ServerError::EmptyArguments)),
            |argument| match argument.as_str() {
                "percent" | "per" | "p" => Ok(vec_tup[0].1.clone()),
                "time" | "t" => Ok(vec_tup[1].1.clone()),
                "state" | "s" => Ok(vec_tup[2].1.clone()),
                "icon" | "i" => Ok(vec_tup[3].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["percent", "time", "state", "icon"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
        )
    }
}
