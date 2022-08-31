use crate::command::run_command;
use crate::error;

fn get_used_bytes(memory_command: &String) -> f32 {
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

fn get_available_bytes(memory_command: &String) -> f32 {
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

fn get_used_percent(memory_command: &String) -> f32 {
    (get_used_bytes(memory_command) / get_available_bytes(memory_command)) * 100f32
}

pub fn get_json() -> String {
    let memory_command = run_command("free", &["-b"]);

    format!(
        "{{\"used_bytes\": {}, \"used_percent\": \"{}\"}}",
        get_used_bytes(&memory_command),
        get_used_percent(&memory_command)
    )
}
