use crate::command;
use crate::command::ServerError;

pub struct Brightness {}

impl Brightness {
    fn get() -> Result<Vec<String>, Box<ServerError>> {
        let bri = command::run("brightnessctl", &["i"])?;

        match bri.split('\n').nth(1) {
            Some(line) => Ok(line
                .trim()
                .split(' ')
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>()),
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "brightness".to_string(),
                output: bri,
            })),
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

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Box<ServerError>> {
        let current_brightness_info = Self::get()?;
        let percent = Self::get_percent(&current_brightness_info[3])?;
        let value = Self::get_value(&current_brightness_info[2])?;

        Ok(vec![
            ("percent".to_string(), percent.to_string()),
            ("value".to_string(), value.to_string()),
        ])
    }

    pub fn parse_args(
        vec_tup: &Vec<(String, String)>,
        args: &[String],
    ) -> Result<String, Box<ServerError>> {
        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "percent" | "per" | "p" => Ok(vec_tup[0].1.clone()),
                "value" | "val" | "v" => Ok(vec_tup[1].1.clone()),
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
