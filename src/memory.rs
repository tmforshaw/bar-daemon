use crate::command;
use crate::error;

pub struct Memory {}

impl Memory {
    fn get() -> String {
        command::run("free", &["-b"])
    }

    fn get_used_bytes(memory_command: &str) -> f32 {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(2) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => float_val,
                    Err(e) => error!("Error while parsing memory bytes to float '{string}': {e}"),
                },
                None => error!("Could not parse used memory bytes"),
            },
            None => error!("Could not process memory command lines"),
        }
    }

    fn get_available_bytes(memory_command: &str) -> f32 {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(1) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => float_val,
                    Err(e) => error!("Error while parsing memory bytes to float '{string}': {e}"),
                },
                None => error!("Could not parse used memory bytes"),
            },
            None => error!("Could not process memory command lines"),
        }
    }

    fn get_used_percent(memory_command: &str) -> f32 {
        (Self::get_used_bytes(memory_command) / Self::get_available_bytes(memory_command)) * 100f32
    }

    pub fn get_json() -> String {
        let memory_command = Self::get();

        format!(
            "{{\"used_bytes\": {}, \"used_percent\": \"{}\"}}",
            Self::get_used_bytes(&memory_command),
            Self::get_used_percent(&memory_command)
        )
    }

    pub fn parse_args(args: &[&str]) -> String {
        let memory_command = Self::get();

        match args[0] {
            "used_bytes" | "used_b" | "ub" => Self::get_used_bytes(&memory_command).to_string(),
            "used_percent" | "used_p" | "up" => Self::get_used_percent(&memory_command).to_string(),
            incorrect => format!("'{incorrect}' is not a valid argument"),
        }
    }
}
