use crate::command;
use crate::command::ServerError;

pub struct Volume {}

impl Volume {
    fn get() -> Result<String, Box<ServerError>> {
        Ok(command::run(
            "pactl",
            &["get-sink-volume", "@DEFAULT_SINK@"],
        )?)
    }

    fn get_percent(volume_command: &str) -> Result<u32, Box<ServerError>> {
        match volume_command.split('/').nth(1) {
            Some(percentage) => match percentage.trim().trim_end_matches('%').parse() {
                Ok(integer) => Ok(integer),
                Err(e) => Err(Box::from(ServerError::StringParse {
                    debug_string: percentage.to_string(),
                    ty: "integer".to_string(),
                    e: Box::from(e),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "volume percentage".to_string(),
                output: volume_command.to_string(),
            })),
        }
    }

    fn get_decibel(volume_command: &str) -> Result<f32, Box<ServerError>> {
        match volume_command.split('/').nth(2) {
            Some(decibel_section) => match decibel_section.trim().split(',').next() {
                Some(decibel) => match decibel.trim_end_matches(" dB").trim().parse() {
                    Ok(float) => Ok(float),
                    Err(e) => Err(Box::from(ServerError::StringParse {
                        debug_string: decibel.to_string(),
                        ty: "float".to_string(),
                        e: Box::from(e),
                    })),
                },
                None => Err(Box::from(ServerError::NotInOutput {
                    looking_for: "decibel".to_string(),
                    output: decibel_section.to_string(),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "decibel".to_string(),
                output: volume_command.to_string(),
            })),
        }
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Box<ServerError>> {
        let volume_command = Self::get()?;
        let percent = Self::get_percent(&volume_command)?;
        let decibel = Self::get_decibel(&volume_command)?;

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("decibel".to_string(), decibel.to_string()),
        ])
    }

    pub fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Box<ServerError>> {
        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "percent" | "per" | "p" => Ok(vec_tup[0].1.clone()),
                "decibel" | "dec" | "d" => Ok(vec_tup[1].1.clone()),
                incorrect => Err(Box::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: vec!["percent", "decibel"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Box::from(ServerError::EmptyArguments)),
        }
    }
}
