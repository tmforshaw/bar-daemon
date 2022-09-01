use crate::command;
use crate::error;

fn get_percent(percentage_unfiltered: &str) -> String {
    percentage_unfiltered
        .trim()
        .trim_start_matches('(')
        .trim_end_matches("%)")
        .to_string()
}

fn get_value(value_unfiltered: &str) -> String {
    value_unfiltered.trim().to_string()
}

fn get_current_brightness_line(brightness_command: &str) -> Vec<String> {
    match brightness_command.split('\n').nth(1) {
        Some(line) => line
            .trim()
            .split(' ')
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>(),
        None => error!("Could not find current brightness in output"),
    }
}

pub fn get_json() -> String {
    let brightness_command = command::run("brightnessctl", &["i"]);

    let current_brightness_info = get_current_brightness_line(&brightness_command);

    format!(
        "{{\"percent\": {}, \"value\": \"{}\"}}",
        get_percent(&current_brightness_info[3]),
        get_value(&current_brightness_info[2])
    )
}
