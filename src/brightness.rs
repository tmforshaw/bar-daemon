use crate::command;
use crate::error;

pub struct Brightness {}

impl Brightness {
    fn get() -> Vec<String> {
        match command::run("brightnessctl", &["i"]).split('\n').nth(1) {
            Some(line) => line
                .trim()
                .split(' ')
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>(),
            None => error!("Could not find current brightness in output"),
        }
    }

    fn get_percent(percentage_unfiltered: &str) -> u32 {
        match percentage_unfiltered
            .trim()
            .trim_start_matches('(')
            .trim_end_matches("%)")
            .parse()
        {
            Ok(integer) => integer,
            Err(e) => error!("Could not parse brightness percent to integer: {e}"),
        }
    }

    fn get_value(value_unfiltered: &str) -> u32 {
        match value_unfiltered.trim().parse() {
            Ok(integer) => integer,
            Err(e) => error!("Could not parse brightness value to integer: {e}"),
        }
    }

    pub fn get_json() -> String {
        let current_brightness_info = Self::get();

        format!(
            "{{\"percent\": {}, \"value\": \"{}\"}}",
            Self::get_percent(&current_brightness_info[3]),
            Self::get_value(&current_brightness_info[2])
        )
    }

    pub fn parse_args(args: &[&str]) -> String {
        let current_brightness_info = Self::get();

        match args[0] {
            "percent" | "per" | "p" => Self::get_percent(&current_brightness_info[3]).to_string(),
            "value" | "val" | "v" => Self::get_value(&current_brightness_info[2]).to_string(),
            incorrect => format!("'{incorrect}' is not a valid argument"),
        }
    }
}
