use crate::command;
use crate::command::ServerError;

use std::sync::Arc;

pub struct Volume {}

impl Volume {
    fn get() -> Result<String, Arc<ServerError>> {
        Ok(command::run(
            "pactl",
            &["get-sink-volume", "@DEFAULT_SINK@"],
        )?)
    }

    fn get_percent(volume_command: &str) -> Result<u32, Arc<ServerError>> {
        match volume_command.split('/').nth(1) {
            Some(percentage) => match percentage.trim().trim_end_matches('%').parse() {
                Ok(integer) => Ok(integer),
                Err(e) => Err(Arc::from(ServerError::StringParse {
                    debug_string: percentage.to_string(),
                    ty: "integer".to_string(),
                    e: Arc::from(e),
                })),
            },
            None => Err(Arc::from(ServerError::NotInOutput {
                looking_for: "volume percentage".to_string(),
                output: volume_command.to_string(),
            })),
        }
    }

    fn get_decibel(volume_command: &str) -> Result<f32, Arc<ServerError>> {
        match volume_command.split('/').nth(2) {
            Some(decibel_section) => match decibel_section.trim().split(',').next() {
                Some(decibel) => match decibel.trim_end_matches(" dB").trim().parse() {
                    Ok(float) => Ok(float),
                    Err(e) => Err(Arc::from(ServerError::StringParse {
                        debug_string: decibel.to_string(),
                        ty: "float".to_string(),
                        e: Arc::from(e),
                    })),
                },
                None => Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "decibel".to_string(),
                    output: decibel_section.to_string(),
                })),
            },
            None => Err(Arc::from(ServerError::NotInOutput {
                looking_for: "decibel".to_string(),
                output: volume_command.to_string(),
            })),
        }
    }

    fn get_state() -> Result<bool, Arc<ServerError>> {
        let mute_command = command::run("pactl", &["get-sink-mute", "@DEFAULT_SINK@"])?;

        match mute_command.split_whitespace().nth(1) {
            Some(mute_val) => Ok(mute_val == "yes"),
            None => Err(Arc::from(ServerError::NotInOutput {
                looking_for: "mute state".to_string(),
                output: mute_command,
            })),
        }
    }

    fn get_icon(percent: u32, mute_state: bool) -> String {
        format!(
            "status/audio-volume-{}{}",
            if mute_state {
                "muted"
            } else {
                match percent {
                    0 => "muted",
                    1..=33 => "low",
                    34..=67 => "medium",
                    68..=100 => "high",
                    101.. => "overamplified",
                }
            },
            crate::ICON_EXT
        )
    }

    pub async fn update() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        Self::get_json_tuple()
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        let volume_command = Self::get()?;
        let percent = Self::get_percent(&volume_command)?;
        let decibel = Self::get_decibel(&volume_command)?;
        let state = Self::get_state()?;
        let icon = Self::get_icon(percent, state);

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("decibel".to_string(), decibel.to_string()),
            ("state".to_string(), state.to_string()),
            ("icon".to_string(), icon),
        ])
    }

    pub async fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Arc<ServerError>> {
        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "percent" | "per" | "p" => Ok(vec_tup[0].1.clone()),
                "decibel" | "dec" | "d" => Ok(vec_tup[1].1.clone()),
                "state" | "s" => Ok(vec_tup[2].1.clone()),
                "icon" | "i" => Ok(vec_tup[3].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: vec!["percent", "decibel", "state", "icon"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Arc::from(ServerError::EmptyArguments)),
        }
    }
}
