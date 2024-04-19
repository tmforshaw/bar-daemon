use crate::command;
use crate::command::ServerError;

use std::sync::Arc;

pub struct Brightness {}

impl Brightness {
    fn get() -> Result<Vec<String>, Arc<ServerError>> {
        let bri = command::run("brightnessctl", &["i"])?;

        bri.split('\n').nth(1).map_or_else(
            || {
                Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "brightness".to_string(),
                    output: bri.clone(),
                }))
            },
            |line| {
                Ok(line
                    .trim()
                    .split(' ')
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>())
            },
        )
    }

    fn get_percent(percentage_unfiltered: &str) -> Result<u32, Arc<ServerError>> {
        match percentage_unfiltered
            .trim()
            .trim_start_matches('(')
            .trim_end_matches("%)")
            .parse()
        {
            Ok(integer) => Ok(integer),
            Err(e) => Err(Arc::from(ServerError::StringParse {
                debug_string: percentage_unfiltered.to_string(),
                ty: "integer".to_string(),
                e: Arc::from(e),
            })),
        }
    }

    fn get_value(value_unfiltered: &str) -> Result<u32, Arc<ServerError>> {
        match value_unfiltered.trim().parse() {
            Ok(integer) => Ok(integer),
            Err(e) => Err(Arc::from(ServerError::StringParse {
                debug_string: value_unfiltered.to_string(),
                ty: "integer".to_string(),
                e: Arc::from(e),
            })),
        }
    }

    fn get_icon(percent: u32) -> String {
        format!(
            "display-brightness-{}{}",
            match percent {
                0 => "off",
                1..=33 => "low",
                34..=67 => "medium",
                68.. => "high",
            },
            crate::ICON_EXT
        )
    }

    pub fn update() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        Self::get_json_tuple()
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        let current_brightness_info = Self::get()?;
        let percent = Self::get_percent(&current_brightness_info[3])?;
        let value = Self::get_value(&current_brightness_info[2])?;
        let icon = Self::get_icon(percent);

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("value".to_string(), value.to_string()),
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
                "value" | "val" | "v" => Ok(vec_tup[1].1.clone()),
                "icon" | "i" => Ok(vec_tup[2].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["percent", "value", "icon"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
        )
    }
}
