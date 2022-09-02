use crate::command;
use crate::command::ServerError;

pub struct Brightness {}

impl Brightness {
    fn get() -> Result<Vec<String>, Box<ServerError>> {
        match command::run("brightnessctl", &["i"]) {
            Ok(output) => match output.split('\n').nth(1) {
                Some(line) => Ok(line
                    .trim()
                    .split(' ')
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>()),
                None => Err(Box::from(ServerError::NotInOutput {
                    looking_for: "brightness".to_string(),
                    output,
                })),
            },
            Err(e) => Err(Box::from(e)),
        }
    }

    fn get_percent(percentage_unfiltered: &str) -> Result<u32, Box<ServerError>> {
        match percentage_unfiltered
            .trim()
            .trim_start_matches('(')
            .trim_end_matches("%)")
            .parse()
        {
            Ok(integer) => Ok(integer),
            Err(e) => Err(Box::from(ServerError::StringParse {
                debug_string: percentage_unfiltered.to_string(),
                ty: "integer".to_string(),
                e: Box::from(e),
            })),
        }
    }

    fn get_value(value_unfiltered: &str) -> Result<u32, Box<ServerError>> {
        match value_unfiltered.trim().parse() {
            Ok(integer) => Ok(integer),
            Err(e) => Err(Box::from(ServerError::StringParse {
                debug_string: value_unfiltered.to_string(),
                ty: "integer".to_string(),
                e: Box::from(e),
            })),
        }
    }

    pub fn get_json() -> Result<String, Box<ServerError>> {
        let current_brightness_info = match Self::get() {
            Ok(info) => info,
            Err(e) => {
                return Err(e);
            }
        };

        let percent = match Self::get_percent(&current_brightness_info[3]) {
            Ok(p) => p,
            Err(e) => {
                return Err(e);
            }
        };

        let value = match Self::get_value(&current_brightness_info[2]) {
            Ok(v) => v,
            Err(e) => {
                return Err(e);
            }
        };

        Ok(format!(
            "{{\"percent\": {}, \"value\": \"{}\"}}",
            percent, value
        ))
    }

    pub fn parse_args(args: &[String]) -> Result<String, Box<ServerError>> {
        let current_brightness_info = match Self::get() {
            Ok(info) => info,
            Err(e) => {
                return Err(e);
            }
        };

        let percent = match Self::get_percent(&current_brightness_info[3]) {
            Ok(p) => p,
            Err(e) => {
                return Err(e);
            }
        };

        let value = match Self::get_value(&current_brightness_info[2]) {
            Ok(v) => v,
            Err(e) => {
                return Err(e);
            }
        };

        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "percent" | "per" | "p" => Ok(percent.to_string()),
                "value" | "val" | "v" => Ok(value.to_string()),
                incorrect => Err(Box::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: vec!["percent", "value"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Box::from(ServerError::EmptyArguments)),
        }
    }
}
