use crate::command;
use crate::command::ServerError;

pub struct Memory {}

impl Memory {
    fn get() -> Result<String, Box<ServerError>> {
        Ok(command::run("free", &["-b"])?)
    }

    fn get_used_bytes(memory_command: &str) -> Result<f32, Box<ServerError>> {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(2) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => Ok(float_val),
                    Err(e) => Err(Box::from(ServerError::StringParse {
                        debug_string: string.to_string(),
                        ty: "float".to_string(),
                        e: Box::from(e),
                    })),
                },
                None => Err(Box::from(ServerError::NotInOutput {
                    looking_for: "used bytes".to_string(),
                    output: line.to_string(),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "memory".to_string(),
                output: memory_command.to_string(),
            })),
        }
    }

    fn get_available_bytes(memory_command: &str) -> Result<f32, Box<ServerError>> {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(1) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => Ok(float_val),
                    Err(e) => Err(Box::from(ServerError::StringParse {
                        debug_string: string.to_string(),
                        ty: "float".to_string(),
                        e: Box::from(e),
                    })),
                },
                None => Err(Box::from(ServerError::NotInOutput {
                    looking_for: "available bytes".to_string(),
                    output: line.to_string(),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "memory".to_string(),
                output: memory_command.to_string(),
            })),
        }
    }

    fn get_used_percent(memory_command: &str) -> Result<f32, Box<ServerError>> {
        let used_bytes = Self::get_used_bytes(memory_command)?;
        let available_bytes = Self::get_available_bytes(memory_command)?;

        Ok((used_bytes / available_bytes) * 100f32)
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Box<ServerError>> {
        let memory_command = Self::get()?;
        let used_bytes = Self::get_used_bytes(&memory_command)?;
        let used_percent = Self::get_used_percent(&memory_command)?;

        Ok(vec![
            ("used_bytes".to_string(), used_bytes.to_string()),
            ("used_percent".to_string(), used_percent.to_string()),
        ])
    }

    pub fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Box<ServerError>> {
        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "used_bytes" | "used_b" | "ub" => Ok(vec_tup[0].1.clone()),
                "used_percent" | "used_p" | "up" => Ok(vec_tup[1].1.clone()),
                incorrect => Err(Box::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["used_bytes", "used_percent"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Box::from(ServerError::EmptyArguments)),
        }
    }
}
