use crate::command;
use crate::command::ServerError;

use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(PartialEq, Eq, Debug)]
enum BatteryState {
    FullyCharged,
    Charging,
    Discharging,
}

const BAT_STATES: &[&str] = &["Fully charged", "Charging", "Discharging"];

const BAT_NOTIFY_VALUES: &[u32] = &[5, 15, 20, 30];
const BAT_NOTIFY_ID: u32 = 42069;
const BAT_NOTIFY_TIMEOUT: u32 = 5000;
const BAT_NOTIFY_MESSAGE: &str = "Battery: ";

pub struct Battery {}

impl Battery {
    fn get() -> Result<String, Arc<ServerError>> {
        Ok(command::run("acpi", &["-b"])?)
    }

    fn get_percent(battery_command: &str) -> Result<u32, Arc<ServerError>> {
        match battery_command.split_whitespace().nth(3) {
            Some(percentage) => match percentage.trim().trim_end_matches("%,").parse() {
                Ok(integer) => Ok(integer),
                Err(e) => Err(Arc::from(ServerError::StringParse {
                    debug_string: percentage.to_string(),
                    ty: "integer".to_string(),
                    e: Arc::from(e),
                })),
            },
            None => Err(Arc::from(ServerError::NotInOutput {
                looking_for: "battery".to_string(),
                output: battery_command.to_string(),
            })),
        }
    }

    fn get_time(battery_command: &str) -> Result<String, Arc<ServerError>> {
        match battery_command.split_whitespace().nth(4) {
            Some(time) => Ok(time.trim().replace(':', " ")),
            None => {
                let state = Self::get_state(battery_command)?;

                if state == BatteryState::FullyCharged {
                    Ok(String::from(
                        BAT_STATES[BatteryState::FullyCharged as usize],
                    ))
                } else {
                    Err(Arc::from(ServerError::NotInOutput {
                        looking_for: "battery time".to_string(),
                        output: battery_command.to_string(),
                    }))
                }
            }
        }
    }

    fn get_state(battery_command: &str) -> Result<BatteryState, Arc<ServerError>> {
        match battery_command.split_whitespace().nth(2) {
            Some(state) => match state.trim_end_matches(',') {
                "Full" => Ok(BatteryState::FullyCharged),
                "Charging" => Ok(BatteryState::Charging),
                "Discharging" => Ok(BatteryState::Discharging),
                incorrect => Err(Arc::from(ServerError::UnknownValue {
                    incorrect: incorrect.to_string(),
                    object: "battery".to_string(),
                })),
            },
            None => Err(Arc::from(ServerError::NotInOutput {
                looking_for: "battery state".to_string(),
                output: battery_command.to_string(),
            })),
        }
    }

    pub fn notify(
        prev_percentage: String,
        current_percentage: String,
    ) -> Result<(), Arc<ServerError>> {
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

        let curr_u32 = match current_percentage.parse::<u32>() {
            Ok(curr) => curr,
            Err(e) => {
                return Err(Arc::from(ServerError::StringParse {
                    debug_string: current_percentage,
                    ty: "integer".to_string(),
                    e: Arc::from(e),
                }))
            }
        };

        if curr_u32 < prev_u32 {
            for value in BAT_NOTIFY_VALUES.iter().rev() {
                if curr_u32 == *value {
                    command::run(
                        "dunstify",
                        &[
                            "-u",
                            "normal",
                            "-t",
                            BAT_NOTIFY_TIMEOUT.to_string().as_str(),
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

    pub async fn update(mutex: &Arc<Mutex<Vec<(String, String)>>>) -> Result<(), Arc<ServerError>> {
        let mut lock = mutex.lock().await;

        let prev_vec_tup = lock.clone();
        let vec_tup = Self::get_json_tuple()?;

        *lock = vec_tup.clone();

        Self::notify(prev_vec_tup[0].1.clone(), vec_tup[0].1.clone())
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Arc<ServerError>> {
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

    pub async fn parse_args(
        mutex: &Arc<Mutex<Vec<(String, String)>>>,
        args: &[String],
    ) -> Result<String, Arc<ServerError>> {
        let lock = mutex.lock().await;
        let vec_tup = lock.clone();

        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "percent" | "per" | "p" => Ok(vec_tup[0].1.clone()),
                "time" | "t" => Ok(vec_tup[1].1.clone()),
                "state" | "s" => Ok(vec_tup[2].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["percent", "time", "state"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Arc::from(ServerError::EmptyArguments)),
        }
    }
}
