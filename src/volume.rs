use crate::command;
use crate::command::ServerError;

pub struct Volume {}

impl Volume {
    fn get() -> Result<String, Box<ServerError>> {
        match command::run("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]) {
            Ok(output) => Ok(output),
            Err(e) => Err(Box::from(e)),
        }
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

    pub fn get_json() -> Result<String, Box<ServerError>> {
        let volume_command = match Self::get() {
            Ok(output) => output,
            Err(e) => return Err(e),
        };

        let percent = match Self::get_percent(&volume_command) {
            Ok(per) => per,
            Err(e) => return Err(e),
        };

        let decibel = match Self::get_decibel(&volume_command) {
            Ok(db) => db,
            Err(e) => return Err(e),
        };

        Ok(format!(
            "{{\"percent\": {}, \"decibel\": \"{}\"}}",
            percent, decibel
        ))
    }

    pub fn parse_args(args: &[String]) -> Result<String, Box<ServerError>> {
        let volume_command = match Self::get() {
            Ok(output) => output,
            Err(e) => return Err(e),
        };

        let percent = match Self::get_percent(&volume_command) {
            Ok(per) => per,
            Err(e) => return Err(e),
        };

        let decibel = match Self::get_decibel(&volume_command) {
            Ok(db) => db,
            Err(e) => return Err(e),
        };

        match args[0].as_str() {
            "percent" | "per" | "p" => Ok(percent.to_string()),
            "decibel" | "dec" | "d" => Ok(decibel.to_string()),
            incorrect => Err(Box::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: vec!["percent", "decibel"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        }
    }
}
