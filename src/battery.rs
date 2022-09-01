use crate::command;
use crate::error;

#[derive(PartialEq, Eq, Debug)]
enum BatteryState {
    FullyCharged,
    Charging,
    Discharging,
}

const BAT_STATES: [&str; 3] = ["Fully charged", "Charging", "Discharging"];

pub struct Battery {}

impl Battery {
    fn get() -> String {
        command::run("acpi", &["-b"])
    }

    fn get_percent(battery_command: &str) -> u32 {
        match battery_command.split_whitespace().nth(3) {
            Some(percentage) => match percentage.trim().trim_end_matches("%,").parse() {
                Ok(integer) => integer,
                Err(e) => error!("Could not parse battery into integer: {e}"),
            },
            None => error!("Couldn't parse battery from battery command"),
        }
    }

    fn get_time(battery_command: &str) -> String {
        match battery_command.split_whitespace().nth(4) {
            Some(time) => time.trim().replace(':', " "),
            None => {
                let state = Self::get_state(battery_command);

                if state == BatteryState::FullyCharged {
                    String::from(BAT_STATES[BatteryState::FullyCharged as usize])
                } else {
                    error!("Could not parse battery time");
                }
            }
        }
    }

    fn get_state(battery_command: &str) -> BatteryState {
        match battery_command.split_whitespace().nth(2) {
            Some(state) => match state.trim_end_matches(',') {
                "Full" => BatteryState::FullyCharged,
                "Charging" => BatteryState::Charging,
                "Discharging" => BatteryState::Discharging,
                _ => error!("Battery state '{state}' unknown"),
            },
            None => error!("Could not parse battery state"),
        }
    }

    pub fn get_json() -> String {
        let battery_command = Self::get();

        format!(
            "{{\"percent\": {}, \"time\": \"{}\", \"state\": \"{}\"}}",
            Self::get_percent(&battery_command),
            Self::get_time(&battery_command),
            BAT_STATES[Self::get_state(&battery_command) as usize]
        )
    }

    pub fn parse_args(args: &[&str]) -> String {
        let battery_command = Self::get();

        match args[0] {
            "percent" | "per" | "p" => Self::get_percent(&battery_command).to_string(),
            "time" | "t" => Self::get_time(&battery_command),
            "state" | "s" => BAT_STATES[Self::get_state(&battery_command) as usize].to_string(),
            incorrect => format!("'{incorrect}' is not a valid argument"),
        }
    }
}
