// use crate::command;

// #[derive(PartialEq, Eq, Debug)]
// enum BatteryState {
//     FullyCharged,
//     Charging,
//     Discharging,
// }

// const BAT_STATES: [&str; 3] = ["Fully charged", "Charging", "Discharging"];

// pub struct Battery {}

// impl Battery {
//     fn get() -> Option<String> {
//         match command::run("acpi", &["-b"]) {
//             Ok(output) => Some(output),
//             Err(e) => {
//                 eprintln!("{e}");
//                 None
//             }
//         }
//     }

//     fn get_percent(battery_command: &str) -> Option<u32> {
//         match battery_command.split_whitespace().nth(3) {
//             Some(percentage) => match percentage.trim().trim_end_matches("%,").parse() {
//                 Ok(integer) => Some(integer),
//                 Err(e) => {
//                     eprintln!("Could not parse battery into integer: {e}");
//                     None
//                 }
//             },
//             None => {
//                 eprintln!("Couldn't parse battery from battery command");
//                 None
//             }
//         }
//     }

//     fn get_time(battery_command: &str) -> Option<String> {
//         match battery_command.split_whitespace().nth(4) {
//             Some(time) => Some(time.trim().replace(':', " ")),
//             None => {
//                 let state = if let Some(s) = Self::get_state(battery_command) {
//                     s
//                 } else {
//                     return None;
//                 };

//                 if state == BatteryState::FullyCharged {
//                     Some(String::from(
//                         BAT_STATES[BatteryState::FullyCharged as usize],
//                     ))
//                 } else {
//                     eprintln!("Could not parse battery time");
//                     None
//                 }
//             }
//         }
//     }

//     fn get_state(battery_command: &str) -> Option<BatteryState> {
//         match battery_command.split_whitespace().nth(2) {
//             Some(state) => match state.trim_end_matches(',') {
//                 "Full" => Some(BatteryState::FullyCharged),
//                 "Charging" => Some(BatteryState::Charging),
//                 "Discharging" => Some(BatteryState::Discharging),
//                 incorrect => {
//                     eprintln!("Battery state '{incorrect}' unknown");
//                     None
//                 }
//             },
//             None => {
//                 eprintln!("Could not parse battery state");
//                 None
//             }
//         }
//     }

//     pub fn get_json() -> Option<String> {
//         let battery_command = match Self::get() {
//             Some(output) => output,
//             None => return None,
//         };

//         let percent = match Self::get_percent(&battery_command) {
//             Some(percent) => percent,
//             None => return None,
//         };

//         let time = match Self::get_time(&battery_command) {
//             Some(time) => time,
//             None => return None,
//         };

//         let state = match Self::get_state(&battery_command) {
//             Some(state) => state,
//             None => return None,
//         };

//         Some(format!(
//             "{{\"percent\": {}, \"time\": \"{}\", \"state\": \"{}\"}}",
//             percent, time, BAT_STATES[state as usize]
//         ))
//     }

//     pub fn parse_args(args: &[&str]) -> Option<String> {
//         let battery_command = match Self::get() {
//             Some(output) => output,
//             None => return None,
//         };

//         match args[0] {
//             "percent" | "per" | "p" => match Self::get_percent(&battery_command) {
//                 Some(percent) => Some(percent.to_string()),
//                 None => None,
//             },
//             "time" | "t" => Self::get_time(&battery_command),
//             "state" | "s" => match Self::get_state(&battery_command) {
//                 Some(state) => Some(BAT_STATES[state as usize].to_string()),
//                 None => None,
//             },
//             incorrect => Some(format!("'{incorrect}' is not a valid argument")),
//         }
//     }
// }
