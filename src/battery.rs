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
        match command::run("acpi", &["-b"]) {
            Ok(output) => Ok(output),
            Err(e) => Err(Box::from(e)),
        }
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
                let state = match Self::get_state(battery_command) {
                    Ok(s) => s,
                    Err(e) => return Err(e),
                };

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

    pub fn get_json() -> Result<String, Box<ServerError>> {
        let battery_command = match Self::get() {
            Ok(output) => output,
            Err(e) => return Err(e),
        };

        let percent = match Self::get_percent(&battery_command) {
            Ok(percent) => percent,
            Err(e) => return Err(e),
        };

        let time = match Self::get_time(&battery_command) {
            Ok(time) => time,
            Err(e) => return Err(e),
        };

        let state = match Self::get_state(&battery_command) {
            Ok(state) => state,
            Err(e) => return Err(e),
        };

        Ok(format!(
            "{{\"percent\": {}, \"time\": \"{}\", \"state\": \"{}\"}}",
            percent, time, BAT_STATES[state as usize]
        ))
    }

    pub fn parse_args(args: &[String]) -> Result<String, Box<ServerError>> {
        let battery_command = match Self::get() {
            Ok(output) => output,
            Err(e) => return Err(e),
        };

        let percent = match Self::get_percent(&battery_command) {
            Ok(percent) => percent,
            Err(e) => return Err(e),
        };

        let time = match Self::get_time(&battery_command) {
            Ok(time) => time,
            Err(e) => return Err(e),
        };

        let state = match Self::get_state(&battery_command) {
            Ok(state) => state,
            Err(e) => return Err(e),
        };

        match args[0].as_str() {
            "percent" | "per" | "p" => Ok(percent.to_string()),
            "time" | "t" => Ok(time),
            "state" | "s" => Ok(BAT_STATES[state as usize].to_string()),
            incorrect => Err(Box::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: ["percent", "time", "state"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        }
    }
}
