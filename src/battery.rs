use crate::command;
use crate::error;

fn get_percent(battery_command: &str) -> String {
    match battery_command.split_whitespace().nth(3) {
        Some(percentage) => percentage.trim().trim_end_matches("%,").to_string(),
        None => error!("Couldn't parse battery from battery command"),
    }
}

#[derive(PartialEq, Eq, Debug)]
enum BatteryState {
    FullyCharged,
    Charging,
    Discharging,
}

const BAT_STATES: [&str; 3] = ["Fully charged", "Charging", "Discharging"];

fn get_time(battery_command: &str) -> String {
    match battery_command.split_whitespace().nth(4) {
        Some(time) => time.trim().replace(':', " "),
        None => {
            let state = get_state(battery_command);

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
    let battery_command = command::run("acpi", &["-b"]);

    format!(
        "{{\"percent\": {}, \"time\": \"{}\", \"state\": \"{}\"}}",
        get_percent(&battery_command),
        get_time(&battery_command),
        BAT_STATES[get_state(&battery_command) as usize]
    )
}
