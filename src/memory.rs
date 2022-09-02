// use crate::command;

// pub struct Memory {}

// impl Memory {
//     fn get() -> Option<String> {
//         match command::run("free", &["-b"]) {
//             Ok(output) => Some(output),
//             Err(e) => {
//                 eprintln!("{e}");
//                 None
//             }
//         }
//     }

//     fn get_used_bytes(memory_command: &str) -> Option<f32> {
//         match memory_command.split_terminator('\n').nth(1) {
//             Some(line) => match line.split_ascii_whitespace().nth(2) {
//                 Some(string) => match string.trim().parse::<f32>() {
//                     Ok(float_val) => Some(float_val),
//                     Err(e) => {
//                         eprintln!("Error while parsing memory bytes to float '{string}': {e}");
//                         None
//                     }
//                 },
//                 None => {
//                     eprintln!("Could not parse used memory bytes");
//                     None
//                 }
//             },
//             None => {
//                 eprintln!("Could not process memory command lines");
//                 None
//             }
//         }
//     }

//     fn get_available_bytes(memory_command: &str) -> Option<f32> {
//         match memory_command.split_terminator('\n').nth(1) {
//             Some(line) => match line.split_ascii_whitespace().nth(1) {
//                 Some(string) => match string.trim().parse::<f32>() {
//                     Ok(float_val) => Some(float_val),
//                     Err(e) => {
//                         eprintln!("Error while parsing memory bytes to float '{string}': {e}");
//                         None
//                     }
//                 },
//                 None => {
//                     eprintln!("Could not parse used memory bytes");
//                     None
//                 }
//             },
//             None => {
//                 eprintln!("Could not process memory command lines");
//                 None
//             }
//         }
//     }

//     fn get_used_percent(memory_command: &str) -> Option<f32> {
//         let used_bytes = if let Some(ub) = Self::get_used_bytes(memory_command) {
//             ub
//         } else {
//             return None;
//         };

//         let available_bytes = if let Some(ab) = Self::get_available_bytes(memory_command) {
//             ab
//         } else {
//             return None;
//         };

//         Some((used_bytes / available_bytes) * 100f32)
//     }

//     pub fn get_json() -> Option<String> {
//         let memory_command = if let Some(output) = Self::get() {
//             output.as_str()
//         } else {
//             return None;
//         };

//         let used_bytes = if let Some(ub) = Self::get_used_bytes(memory_command) {
//             ub
//         } else {
//             return None;
//         };

//         let used_percent = if let Some(up) = Self::get_used_percent(memory_command) {
//             up
//         } else {
//             return None;
//         };

//         Some(format!(
//             "{{\"used_bytes\": {}, \"used_percent\": \"{}\"}}",
//             used_bytes, used_percent,
//         ))
//     }

//     pub fn parse_args(args: &[&str]) -> Option<String> {
//         let memory_command = if let Some(output) = Self::get() {
//             output.as_str()
//         } else {
//             return None;
//         };

//         let used_bytes = if let Some(ub) = Self::get_used_bytes(memory_command) {
//             ub
//         } else {
//             return None;
//         };

//         let used_percent = if let Some(up) = Self::get_used_percent(memory_command) {
//             up
//         } else {
//             return None;
//         };

//         match args[0] {
//             "used_bytes" | "used_b" | "ub" => Some(used_bytes.to_string()),
//             "used_percent" | "used_p" | "up" => Some(used_percent.to_string()),
//             incorrect => Some(format!("'{incorrect}' is not a valid argument")),
//         }
//     }
// }
