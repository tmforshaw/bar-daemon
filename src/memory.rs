use crate::command::run_command;
use crate::error;

fn get_percent(memory_command: &String) -> String {
    match memory_command.split('/').nth(1) {
        Some(percentage) => percentage.trim().trim_end_matches('%').to_string(),
        None => error!("Couldn't parse memory from memory command"),
    }
}

fn get_decibel(memory_command: &String) -> String {
    match memory_command.split('/').nth(2) {
        Some(decibel_section) => match decibel_section.trim().split(',').next() {
            Some(decibel) => decibel.trim_end_matches(" dB").trim().to_string(),
            None => error!("Could not find decibel in output"),
        },
        None => error!("Could not find decibel section in output"),
    }
}

pub fn get_json() -> String {
    let memory_command = run_command("free", &["-b"]);

    format!(
        "{{\"percent\": {}, \"decibel\": \"{}\"}}",
        get_percent(&memory_command),
        get_decibel(&memory_command)
    )
}
