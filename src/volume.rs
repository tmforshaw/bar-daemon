use crate::command;
use crate::error;

pub struct Volume {}

impl Volume {
    fn get() -> String {
        command::run("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])
    }

    fn get_percent(volume_command: &str) -> u32 {
        match volume_command.split('/').nth(1) {
            Some(percentage) => match percentage.trim().trim_end_matches('%').parse() {
                Ok(integer) => integer,
                Err(e) => error!("Could not parse volume percentage into integer: {e}"),
            },
            None => error!("Couldn't parse volume from volume command"),
        }
    }

    fn get_decibel(volume_command: &str) -> f32 {
        match volume_command.split('/').nth(2) {
            Some(decibel_section) => match decibel_section.trim().split(',').next() {
                Some(decibel) => match decibel.trim_end_matches(" dB").trim().parse() {
                    Ok(float) => float,
                    Err(e) => error!("Could not parse decibel into float: {e}"),
                },
                None => error!("Could not find decibel in output"),
            },
            None => error!("Could not find decibel section in output"),
        }
    }

    pub fn get_json() -> String {
        let volume_command = Self::get();

        format!(
            "{{\"percent\": {}, \"decibel\": \"{}\"}}",
            Self::get_percent(&volume_command),
            Self::get_decibel(&volume_command)
        )
    }

    pub fn parse_args(args: &[&str]) -> String {
        let volume_command = Self::get();

        match args[0] {
            "percent" | "per" | "p" => Self::get_percent(&volume_command).to_string(),
            "decibel" | "dec" | "d" => Self::get_decibel(&volume_command).to_string(),
            incorrect => format!("'{incorrect}' is not a valid argument"),
        }
    }
}
