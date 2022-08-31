use crate::command::run_command;
use crate::error;

fn get_percent(volume_command: &String) -> String {
    match volume_command.split('/').nth(1) {
        Some(percentage) => percentage.trim().trim_end_matches('%').to_string(),
        None => error!("Couldn't parse volume from volume command"),
    }
}

fn get_decibel(volume_command: &String) -> String {
    match volume_command.split('/').nth(2) {
        Some(decibel_section) => match decibel_section.trim().split(',').next() {
            Some(decibel) => decibel.trim_end_matches(" dB").trim().to_string(),
            None => error!("Could not find decibel in output"),
        },
        None => error!("Could not find decibel section in output"),
    }
}

pub fn get_json() -> String {
    let volume_command = run_command("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]);

    format!(
        "{{\"percent\": {}, \"decibel\": \"{}\"}}",
        get_percent(&volume_command),
        get_decibel(&volume_command)
    )
}
