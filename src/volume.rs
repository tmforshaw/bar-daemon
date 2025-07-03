use crate::command;
use crate::command::ServerError;

use std::sync::Arc;

pub struct Volume {}

impl Volume {
    fn get() -> Result<String, Arc<ServerError>> {
        Ok(command::run("wpctl", &["get-volume", "@DEFAULT_SINK@"])?)
    }

    fn get_percent_and_mute(volume_command: &str) -> Result<(u32, bool), Arc<ServerError>> {
        let output = volume_command.split(':').nth(1).map_or(
            {
                Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "volume percentage".to_string(),
                    output: volume_command.to_string(),
                }))
            },
            |text| Ok(text.trim()),
        );

        let mut output_split = output?.split(' ');

        let percent_output = output_split.next().map_or_else(
            || {
                Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "volume percentage".to_string(),
                    output: volume_command.to_string(),
                }))
            },
            |text| Ok(text.trim()),
        );

        let text = percent_output?;

        let percent = match text.parse::<f32>() {
            Ok(float) => Ok((float * 100.) as u32),
            Err(e) => Err(Arc::from(ServerError::StringParse {
                debug_string: text.to_string(),
                ty: "float".to_string(),
                e: Arc::from(e),
            })),
        };

        let mute = output_split.next().is_some();

        Ok((percent?, mute))
    }

    fn get_percent(volume_command: &str) -> Result<u32, Arc<ServerError>> {
        Ok(Self::get_percent_and_mute(volume_command)?.0)
    }

    fn get_mute_state(volume_command: &str) -> Result<bool, Arc<ServerError>> {
        Ok(Self::get_percent_and_mute(volume_command)?.1)
    }

    fn get_icon(percent: u32, mute_state: bool) -> String {
        format!(
            "audio-volume-{}{}",
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

    pub fn update() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        Self::get_json_tuple()
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        let volume_command = Self::get()?;
        let percent = Self::get_percent(&volume_command)?;
        // let decibel = Self::get_decibel(&volume_command)?;
        let mute_state = Self::get_mute_state(&volume_command)?;
        let icon = Self::get_icon(percent, mute_state);

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            // ("decibel".to_string(), decibel.to_string()),
            ("mute_state".to_string(), mute_state.to_string()),
            ("icon".to_string(), icon),
        ])
    }

    pub fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Arc<ServerError>> {
        args.first().map_or_else(
            || Err(Arc::from(ServerError::EmptyArguments)),
            |argument| match argument.as_str() {
                "percent" | "per" | "p" => Ok(vec_tup[0].1.clone()),
                // "decibel" | "dec" | "d" => Ok(vec_tup[1].1.clone()),
                "mute" | "m" => Ok(vec_tup[1].1.clone()),
                "icon" | "i" => Ok(vec_tup[2].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["percent", "mute", "icon"] // , "decibel"
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
        )
    }
}
