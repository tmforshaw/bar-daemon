use crate::command;
use crate::command::ServerError;

#[derive(PartialEq, Eq, Debug)]
enum BatteryState {
    FullyCharged,
    Charging,
    Discharging,
}

const BAT_STATES: [&str; 3] = ["Fully charged", "Charging", "Discharging"];

pub struct Battery {}

impl Battery {
    fn get() -> Result<String, Box<ServerError>> {
        Ok(command::run("acpi", &["-b"])?)
    }

    fn get_percent(battery_command: &str) -> Result<u32, Box<ServerError>> {
        match battery_command.split_whitespace().nth(3) {
            Some(percentage) => match percentage.trim().trim_end_matches("%,").parse() {
                Ok(integer) => Ok(integer),
                Err(e) => Err(Box::from(ServerError::StringParse {
                    debug_string: percentage.to_string(),
                    ty: "integer".to_string(),
                    e: Box::from(e),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "battery".to_string(),
                output: battery_command.to_string(),
            })),
        }
    }

    fn get_time(battery_command: &str) -> Result<String, Box<ServerError>> {
        match battery_command.split_whitespace().nth(4) {
            Some(time) => Ok(time.trim().replace(':', " ")),
            None => {
                let state = Self::get_state(battery_command)?;

                if state == BatteryState::FullyCharged {
                    Ok(String::from(
                        BAT_STATES[BatteryState::FullyCharged as usize],
                    ))
                } else {
                    Err(Box::from(ServerError::NotInOutput {
                        looking_for: "battery time".to_string(),
                        output: battery_command.to_string(),
                    }))
                }
            }
        }
    }

    fn get_state(battery_command: &str) -> Result<BatteryState, Box<ServerError>> {
        match battery_command.split_whitespace().nth(2) {
            Some(state) => match state.trim_end_matches(',') {
                "Full" => Ok(BatteryState::FullyCharged),
                "Charging" => Ok(BatteryState::Charging),
                "Discharging" => Ok(BatteryState::Discharging),
                incorrect => Err(Box::from(ServerError::UnknownValue {
                    incorrect: incorrect.to_string(),
                    object: "battery".to_string(),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "battery state".to_string(),
                output: battery_command.to_string(),
            })),
        }
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Box<ServerError>> {
        let battery_command = Self::get()?;

        let percent = Self::get_percent(&battery_command)?;
        let time = Self::get_time(&battery_command)?;
        let state = Self::get_state(&battery_command)?;

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("time".to_string(), time),
            ("state".to_string(), BAT_STATES[state as usize].to_string()),
        ])
    }

    pub fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Box<ServerError>> {
        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "percent" | "per" | "p" => Ok(vec_tup[0].1.clone()),
                "time" | "t" => Ok(vec_tup[1].1.clone()),
                "state" | "s" => Ok(vec_tup[2].1.clone()),
                incorrect => Err(Box::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["percent", "time", "state"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Box::from(ServerError::EmptyArguments)),
        }
    }
}
